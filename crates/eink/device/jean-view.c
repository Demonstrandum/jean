/* jean-view: an AppLoad (QTFB) app for the reMarkable 2 that shows Jean's display.
 *
 * It is a minimal VNC (RFB 3.8) client for `eink-serve --panel rm2-portrait`: Raw encoding only, 32-bit BGRX pixels, no security.
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
    union { struct init_msg init; struct update_msg update; } u;
};
struct init_reply { int shm_key; size_t shm_size; };
struct server_msg { uint8_t type; struct init_reply init; };

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
        if (read_all(fd, buf, 4) || buf[0] != 0) break;
        int rects = be16(buf + 2);
        int minx = WIDTH, miny = HEIGHT, maxx = 0, maxy = 0;
        for (int i = 0; i < rects; i++) {
            uint8_t h[12];
            if (read_all(fd, h, 12)) goto out;
            int x = be16(h), y = be16(h + 2), w = be16(h + 4), ht = be16(h + 6);
            if (h[8] | h[9] | h[10] | h[11]) goto out; /* only Raw */
            if (x + w > WIDTH || y + ht > HEIGHT) goto out;
            for (int yy = 0; yy < ht; yy++) {
                if (read_all(fd, row, (size_t)w * 4)) goto out;
                uint16_t *dst = fb + (size_t)(y + yy) * WIDTH + x;
                for (int xx = 0; xx < w; xx++) dst[xx] = row[xx * 4] >= 128 ? 0xFFFF : 0x0000;
            }
            if (x < minx) minx = x;
            if (y < miny) miny = y;
            if (x + w > maxx) maxx = x + w;
            if (y + ht > maxy) maxy = y + ht;
        }
        if (rects) qtfb_update(UPDATE_PARTIAL, minx, miny, maxx - minx, maxy - miny);
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
