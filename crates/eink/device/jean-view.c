/* jean-view: an AppLoad (QTFB) app for the reMarkable 2 that shows Jean's display.
 *
 * It is a minimal VNC (RFB 3.8) client for `eink-serve --panel rm2-portrait`: Raw encoding only, 32-bit BGRX pixels (the gray in each), no security.
 * Each update rectangle is written into the QTFB shared-memory framebuffer (RGB565, 1404x1872) and announced as a partial update, so
 * the panel refreshes only what changed. If the server goes away it waits and reconnects.
 *
 *   JEAN_HOST (default 10.11.99.8, the Mac on the USB link)   JEAN_PORT (default 5911)   QTFB_KEY (set by AppLoad)
 */
#include <errno.h>
#include <fcntl.h>
#include <netdb.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <poll.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <time.h>
#include <unistd.h>

#define SOCKET_PATH "/tmp/qtfb.sock"
#define MESSAGE_INITIALIZE 0
#define MESSAGE_UPDATE 1
#define MESSAGE_USERINPUT 4
#define MESSAGE_SET_REFRESH_MODE 5
#define REFRESH_MODE_UFAST 0
#define REFRESH_MODE_FAST 1
#define REFRESH_MODE_ANIMATE 2
#define REFRESH_MODE_CONTENT 3
#define REFRESH_MODE_UI 4
#define INPUT_TOUCH_PRESS 0x10
#define INPUT_TOUCH_RELEASE 0x11
#define INPUT_TOUCH_UPDATE 0x12
#define FBFMT_RM2FB 0
#define UPDATE_ALL 0
#define UPDATE_PARTIAL 1
#define WIDTH 1404
#define HEIGHT 1872

/* the QTFB wire structs (asivery/qtfb common.h), laid out as the C++ compiler lays them out on 32-bit ARM */
struct init_msg { unsigned int key; uint8_t type; };
struct update_msg { int type; int x, y, w, h; };
struct client_msg {
    uint8_t type;
    union { struct init_msg init; struct update_msg update; int refresh_mode; } u;
};
struct init_reply { int shm_key; size_t shm_size; };
struct server_msg { uint8_t type; struct init_reply init; };

/* what the QTFB server sends back: touches, in the framebuffer's own coordinates (the union starts at offset 4 on 32-bit ARM) */
struct input_msg { uint8_t type; uint8_t pad[3]; int input_type, dev_id, x, y, d; };

static uint16_t *fb;
static int qtfb_sock;

static void die(const char *what) {
    fprintf(stderr, "jean-view: %s (errno %d)\n", what, errno);
    exit(1);
}

static void qtfb_connect(unsigned int key) {
    qtfb_sock = socket(AF_UNIX, SOCK_SEQPACKET, 0);
    if (qtfb_sock < 0) die("socket");
    struct sockaddr_un addr;
    memset(&addr, 0, sizeof addr);
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, SOCKET_PATH, sizeof addr.sun_path - 1);
    if (connect(qtfb_sock, (struct sockaddr *)&addr, sizeof addr) != 0) die("connect to qtfb");
    struct client_msg m;
    memset(&m, 0, sizeof m);
    m.type = MESSAGE_INITIALIZE;
    m.u.init.key = key;
    m.u.init.type = FBFMT_RM2FB;
    if (send(qtfb_sock, &m, sizeof m, 0) < 0) die("send init");
    struct server_msg r;
    if (recv(qtfb_sock, &r, sizeof r, 0) < 1) die("recv init");
    char name[32];
    snprintf(name, sizeof name, "/qtfb_%d", r.init.shm_key);
    int fd = shm_open(name, O_RDWR, 0);
    if (fd < 0) die("shm_open");
    void *mem = mmap(NULL, r.init.shm_size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (mem == MAP_FAILED) die("mmap");
    if (r.init.shm_size < (size_t)WIDTH * HEIGHT * 2) {
        fprintf(stderr, "jean-view: framebuffer is only %zu bytes\n", r.init.shm_size);
        exit(1);
    }
    fb = mem;
}

static void qtfb_update(int type, int x, int y, int w, int h) {
    struct client_msg m;
    memset(&m, 0, sizeof m);
    m.type = MESSAGE_UPDATE;
    m.u.update.type = type;
    m.u.update.x = x;
    m.u.update.y = y;
    m.u.update.w = w;
    m.u.update.h = h;
    send(qtfb_sock, &m, sizeof m, 0);
}

/* How the panel refreshes what we draw: JEAN_REFRESH = ufast | fast | animate | content | ui (default content: the most faithful grays, no
 * darkening of dithered areas, at the cost of slower updates). */
static void set_refresh_mode(const char *name) {
    int mode = REFRESH_MODE_CONTENT;
    if (name) {
        if (!strcmp(name, "ufast")) mode = REFRESH_MODE_UFAST;
        else if (!strcmp(name, "fast")) mode = REFRESH_MODE_FAST;
        else if (!strcmp(name, "animate")) mode = REFRESH_MODE_ANIMATE;
        else if (!strcmp(name, "ui")) mode = REFRESH_MODE_UI;
    }
    struct client_msg m;
    memset(&m, 0, sizeof m);
    m.type = MESSAGE_SET_REFRESH_MODE;
    m.u.refresh_mode = mode;
    send(qtfb_sock, &m, sizeof m, 0);
}

static int read_all(int fd, void *buf, size_t n) {
    uint8_t *p = buf;
    while (n) {
        ssize_t k = read(fd, p, n);
        if (k <= 0) return -1;
        p += k;
        n -= (size_t)k;
    }
    return 0;
}

static int write_all(int fd, const void *buf, size_t n) {
    const uint8_t *p = buf;
    while (n) {
        ssize_t k = write(fd, p, n);
        if (k <= 0) return -1;
        p += k;
        n -= (size_t)k;
    }
    return 0;
}

static int dial(const char *host, const char *port) {
    struct addrinfo hints, *res;
    memset(&hints, 0, sizeof hints);
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    if (getaddrinfo(host, port, &hints, &res) != 0) return -1;
    int fd = socket(res->ai_family, res->ai_socktype, 0);
    if (fd >= 0 && connect(fd, res->ai_addr, res->ai_addrlen) != 0) {
        close(fd);
        fd = -1;
    }
    freeaddrinfo(res);
    if (fd >= 0) {
        int one = 1;
        setsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &one, sizeof one);
    }
    return fd;
}

static uint16_t be16(const uint8_t *p) { return (uint16_t)(p[0] << 8 | p[1]); }

static int request(int fd, int incremental) {
    uint8_t m[10] = {3, (uint8_t)incremental, 0, 0, 0, 0, WIDTH >> 8, WIDTH & 255, HEIGHT >> 8, HEIGHT & 255};
    return write_all(fd, m, sizeof m);
}

/* Touches from AppLoad become VNC pointer events for the server (button down while the finger is down); eink-serve works out what a tap means. */
static int pump_touch(int rfb) {
    struct input_msg m;
    for (;;) {
        ssize_t n = recv(qtfb_sock, &m, sizeof m, MSG_DONTWAIT);
        if (n == 0) exit(0); /* AppLoad closed the window */
        if (n < 0) return errno == EAGAIN || errno == EWOULDBLOCK ? 0 : -1;
        if (m.type != MESSAGE_USERINPUT) continue;
        static int last_x, last_y;
        int down;
        if (m.input_type == INPUT_TOUCH_PRESS) { down = 1; fprintf(stderr, "jean-view: touch %d,%d\n", m.x, m.y); }
        else if (m.input_type == INPUT_TOUCH_UPDATE) down = 1;
        else if (m.input_type == INPUT_TOUCH_RELEASE) { down = 0; fprintf(stderr, "jean-view: release %d,%d (last %d,%d)\n", m.x, m.y, last_x, last_y); }
        else continue;
        if (down) { if (m.x < 0 || m.y < 0) continue; last_x = m.x; last_y = m.y; }
        int ex = down ? m.x : last_x, ey = down ? m.y : last_y; /* a release carries where the finger last was, whatever the event says */
        uint8_t ev[6] = {5, (uint8_t)down, (uint8_t)(ex >> 8), (uint8_t)ex, (uint8_t)(ey >> 8), (uint8_t)ey};
        if (write_all(rfb, ev, sizeof ev)) return -1;
    }
}

/* one connection's lifetime; returns when the server goes away */
static void session(int fd) {
    uint8_t buf[64];
    if (read_all(fd, buf, 12) || write_all(fd, "RFB 003.008\n", 12)) return;
    if (read_all(fd, buf, 2) || buf[0] != 1) return; /* one security type */
    buf[0] = 1;                                       /* None */
    if (write_all(fd, buf, 1) || read_all(fd, buf, 4)) return;
    buf[0] = 1;                                       /* shared */
    if (write_all(fd, buf, 1) || read_all(fd, buf, 24)) return;
    int sw = be16(buf), sh = be16(buf + 2);
    uint32_t name_len = (uint32_t)buf[20] << 24 | buf[21] << 16 | buf[22] << 8 | buf[23];
    for (uint32_t i = 0; i < name_len; i++) {
        if (read_all(fd, buf, 1)) return;
    }
    if (sw != WIDTH || sh != HEIGHT) {
        fprintf(stderr, "jean-view: server is %dx%d, expected %dx%d (run eink-serve --panel rm2-portrait)\n", sw, sh, WIDTH, HEIGHT);
        sleep(5);
        return;
    }
    fprintf(stderr, "jean-view: connected\n");
    if (request(fd, 0)) return;
    uint8_t *row = malloc((size_t)WIDTH * 4);
    for (;;) {
        for (;;) { /* wait for an update from the server or a touch from AppLoad, whichever comes first */
            struct pollfd p[2] = {{fd, POLLIN, 0}, {qtfb_sock, POLLIN, 0}};
            if (poll(p, 2, -1) < 0) goto out;
            if ((p[1].revents & POLLIN) && pump_touch(fd)) goto out;
            if (p[0].revents & (POLLIN | POLLHUP | POLLERR)) break;
        }
        if (read_all(fd, buf, 4) || buf[0] != 0) break;
        int rects = be16(buf + 2);
        for (int i = 0; i < rects; i++) {
            uint8_t h[12];
            if (read_all(fd, h, 12)) goto out;
            int x = be16(h), y = be16(h + 2), w = be16(h + 4), ht = be16(h + 6);
            if (h[8] | h[9] | h[10] | h[11]) goto out; /* only Raw */
            if (x + w > WIDTH || y + ht > HEIGHT) goto out;
            for (int yy = 0; yy < ht; yy++) {
                if (read_all(fd, row, (size_t)w * 4)) goto out;
                uint16_t *dst = fb + (size_t)(y + yy) * WIDTH + x;
                for (int xx = 0; xx < w; xx++) { uint16_t g = row[xx * 4]; dst[xx] = (uint16_t)(((g >> 3) << 11) | ((g >> 2) << 5) | (g >> 3)); } /* gray to RGB565 */
            }
            qtfb_update(UPDATE_PARTIAL, x, y, w, ht); /* each rectangle on its own: two distant changes are not one big refresh */
        }
        if (request(fd, 1)) break;
    }
out:
    free(row);
}

int main(void) {
    const char *key = getenv("QTFB_KEY");
    const char *host = getenv("JEAN_HOST");
    const char *port = getenv("JEAN_PORT");
    if (!key) {
        fprintf(stderr, "jean-view: QTFB_KEY is not set; start me from AppLoad\n");
        return 1;
    }
    qtfb_connect((unsigned int)strtoul(key, NULL, 10));
    set_refresh_mode(getenv("JEAN_REFRESH"));
    for (size_t i = 0; i < (size_t)WIDTH * HEIGHT; i++) fb[i] = 0xFFFF;
    qtfb_update(UPDATE_ALL, 0, 0, 0, 0);
    for (;;) {
        int fd = dial(host ? host : "10.11.99.8", port ? port : "5911");
        if (fd >= 0) {
            session(fd);
            close(fd);
        }
        sleep(2);
    }
}
