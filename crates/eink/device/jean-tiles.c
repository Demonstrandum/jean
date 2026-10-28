/* jean-tiles: the backend of the "Jean (tiles)" AppLoad app for the reMarkable 2.
 *
 * xochitl damages the panel per Qt item. jean-view put the whole screen in one framebuffer item, so even a 3x18 px change (the seconds) made xochitl swap
 * and re-drive the entire 1404x1872 screen: everything flashed on every tick. Here the screen is a grid of 12x16 small framebuffer items (main.qml); this
 * program owns one QTFB framebuffer per item, writes a frame only into the tiles it changes and announces only those, so a tick dirties one small item
 * and the rest of the panel is left alone.
 *
 * It is a minimal VNC (RFB 3.8) client for `eink-serve --panel rm2-turned-ccw` (Raw encoding, 32-bit BGRX pixels, the gray in each), and it forwards the
 * touches that each tile reports (in the tile's own coordinates) back as VNC pointer events in screen coordinates.
 *
 *   argv[1]    AppLoad's backend socket (we connect to it so AppLoad knows we are up, and exit when it closes)
 *   JEAN_HOST (default 10.11.99.8)   JEAN_PORT (default 5911)
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
#define MESSAGE_UPDATE 1
#define MESSAGE_CUSTOM_INITIALIZE 2
#define MESSAGE_USERINPUT 4
#define FBFMT_RMPP_RGB565 3 /* 16 bits a pixel, any size (the rm2 format fixes the size) */
#define UPDATE_PARTIAL 1
#define INPUT_TOUCH_PRESS 0x10
#define INPUT_TOUCH_RELEASE 0x11
#define INPUT_TOUCH_UPDATE 0x12

#define WIDTH 1404
#define HEIGHT 1872
#define COLS 12
#define ROWS 16
#define TW 117 /* WIDTH / COLS */
#define TH 117 /* HEIGHT / ROWS */
#define NTILES (COLS * ROWS)
#define KEY_BASE 5000000 /* tile i is framebuffer KEY_BASE + i: main.qml has the same numbers */

/* the QTFB wire structs, as the C++ compiler lays them out on 32-bit ARM */
struct cmsg_custom { uint8_t type; uint8_t pad[3]; int key; uint8_t fbtype; uint8_t pad2; uint16_t w, h; };
struct cmsg_update { uint8_t type; uint8_t pad[3]; int utype, x, y, w, h; };
struct smsg_init { uint8_t type; uint8_t pad[3]; int shm_key; uint32_t shm_size; };
struct smsg_input { uint8_t type; uint8_t pad[3]; int input_type, dev_id, x, y, d; };

struct tile {
    int sock;
    uint16_t *px;
    int dx0, dy0, dx1, dy1; /* what this frame changed, in tile coordinates; dx0 > dx1 when nothing */
};
static struct tile tiles[NTILES];

/* stage timing, for finding where a touch's latency goes: lines in the journal carry microsecond stamps */
static long now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000L + ts.tv_nsec / 1000000L;
}

static void die(const char *what) {
    fprintf(stderr, "jean-tiles: %s (errno %d)\n", what, errno);
    exit(1);
}

static void tile_connect(int i) {
    struct tile *t = &tiles[i];
    t->sock = socket(AF_UNIX, SOCK_SEQPACKET, 0);
    if (t->sock < 0) die("socket");
    struct sockaddr_un addr;
    memset(&addr, 0, sizeof addr);
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, SOCKET_PATH, sizeof addr.sun_path - 1);
    if (connect(t->sock, (struct sockaddr *)&addr, sizeof addr) != 0) die("connect to qtfb");
    struct cmsg_custom m;
    memset(&m, 0, sizeof m);
    m.type = MESSAGE_CUSTOM_INITIALIZE;
    m.key = KEY_BASE + i;
    m.fbtype = FBFMT_RMPP_RGB565;
    m.w = TW;
    m.h = TH;
    if (send(t->sock, &m, sizeof m, 0) < 0) die("send init");
    struct smsg_init r;
    if (recv(t->sock, &r, sizeof r, 0) < 1) die("recv init");
    char name[32];
    snprintf(name, sizeof name, "/qtfb_%d", r.shm_key);
    int fd = shm_open(name, O_RDWR, 0);
    if (fd < 0) die("shm_open");
    void *mem = mmap(NULL, (size_t)TW * TH * 2, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (mem == MAP_FAILED) die("mmap");
    close(fd);
    t->px = mem;
    t->dx0 = t->dy0 = 1 << 20;
    t->dx1 = t->dy1 = -1;
}

static void tile_update(int i, int x, int y, int w, int h) {
    struct cmsg_update m;
    memset(&m, 0, sizeof m);
    m.type = MESSAGE_UPDATE;
    m.utype = UPDATE_PARTIAL;
    m.x = x;
    m.y = y;
    m.w = w;
    m.h = h;
    send(tiles[i].sock, &m, sizeof m, 0);
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

static int appload_sock = -1;

/* Touches: each tile reports a finger in its own coordinates; add the tile's place to make screen coordinates and tell the server as a VNC pointer. A
 * release is sent where the finger last was, whatever the event says. */
static int pump_touch(int tile, int rfb) {
    struct smsg_input m;
    for (;;) {
        ssize_t n = recv(tiles[tile].sock, &m, sizeof m, MSG_DONTWAIT);
        if (n == 0) exit(0); /* AppLoad closed the framebuffer: the window is gone */
        if (n < 0) return errno == EAGAIN || errno == EWOULDBLOCK ? 0 : -1;
        if (m.type != MESSAGE_USERINPUT) continue;
        static int last_x, last_y;
        int down;
        int gx = (tile % COLS) * TW + m.x, gy = (tile / COLS) * TH + m.y;
        if (m.input_type == INPUT_TOUCH_PRESS) { down = 1; fprintf(stderr, "jean-tiles: touch %d,%d\n", gx, gy); }
        else if (m.input_type == INPUT_TOUCH_UPDATE) down = 1;
        else if (m.input_type == INPUT_TOUCH_RELEASE) down = 0;
        else continue;
        if (down) { last_x = gx; last_y = gy; }
        int ex = down ? gx : last_x, ey = down ? gy : last_y;
        if (ex < 0) ex = 0;
        if (ey < 0) ey = 0;
        uint8_t ev[6] = {5, (uint8_t)down, (uint8_t)(ex >> 8), (uint8_t)ex, (uint8_t)(ey >> 8), (uint8_t)ey};
        if (write_all(rfb, ev, sizeof ev)) return -1;
        if (!down) fprintf(stderr, "jean-tiles: release sent to the server at %ld ms\n", now_ms());
    }
}

static void note_dirty(struct tile *t, int x0, int y0, int x1, int y1) {
    if (x0 < t->dx0) t->dx0 = x0;
    if (y0 < t->dy0) t->dy0 = y0;
    if (x1 > t->dx1) t->dx1 = x1;
    if (y1 > t->dy1) t->dy1 = y1;
}

static void session(int fd) {
    uint8_t buf[64];
    if (read_all(fd, buf, 12) || write_all(fd, "RFB 003.008\n", 12)) return;
    if (read_all(fd, buf, 2) || buf[0] != 1) return;
    buf[0] = 1;
    if (write_all(fd, buf, 1) || read_all(fd, buf, 4)) return;
    buf[0] = 1;
    if (write_all(fd, buf, 1) || read_all(fd, buf, 24)) return;
    int sw = be16(buf), sh = be16(buf + 2);
    uint32_t name_len = (uint32_t)buf[20] << 24 | buf[21] << 16 | buf[22] << 8 | buf[23];
    for (uint32_t i = 0; i < name_len; i++) {
        if (read_all(fd, buf, 1)) return;
    }
    if (sw != WIDTH || sh != HEIGHT) {
        fprintf(stderr, "jean-tiles: server is %dx%d, expected %dx%d (run eink-serve --panel rm2-turned-ccw)\n", sw, sh, WIDTH, HEIGHT);
        sleep(5);
        return;
    }
    fprintf(stderr, "jean-tiles: connected\n");
    /* ask for the packed 1-bit encoding (a 32nd of the bytes of Raw), and Raw as the fallback */
    {
        uint8_t enc[12] = {2, 0, 0, 2, 0x6A, 0x65, 0x00, 0x01, 0, 0, 0, 0};
        if (write_all(fd, enc, sizeof enc)) return;
    }
    if (request(fd, 0)) return;
    uint8_t *row = malloc((size_t)WIDTH * 4);
    struct pollfd p[NTILES + 2];
    for (;;) {
        for (;;) { /* wait for an update from the server, a touch on any tile, or AppLoad hanging up */
            p[0].fd = fd; p[0].events = POLLIN; p[0].revents = 0;
            p[1].fd = appload_sock; p[1].events = POLLIN; p[1].revents = 0;
            for (int i = 0; i < NTILES; i++) { p[2 + i].fd = tiles[i].sock; p[2 + i].events = POLLIN; p[2 + i].revents = 0; }
            if (poll(p, NTILES + 2, -1) < 0) goto out;
            if (appload_sock >= 0 && (p[1].revents & (POLLHUP | POLLERR))) exit(0);
            if (appload_sock >= 0 && (p[1].revents & POLLIN)) { uint8_t junk[256]; if (recv(appload_sock, junk, sizeof junk, MSG_DONTWAIT) == 0) exit(0); }
            for (int i = 0; i < NTILES; i++)
                if ((p[2 + i].revents & POLLIN) && pump_touch(i, fd)) goto out;
            if (p[0].revents & (POLLIN | POLLHUP | POLLERR)) break;
        }
        if (read_all(fd, buf, 4) || buf[0] != 0) break;
        int rects = be16(buf + 2);
        long t_frame = now_ms();
        for (int i = 0; i < rects; i++) {
            uint8_t h[12];
            if (read_all(fd, h, 12)) goto out;
            int x = be16(h), y = be16(h + 2), w = be16(h + 4), ht = be16(h + 6);
            int32_t enc = (int32_t)((uint32_t)h[8] << 24 | h[9] << 16 | h[10] << 8 | h[11]);
            int is_packed = enc == 0x6A650001;
            if (enc != 0 && !is_packed) goto out; /* Raw or the packed bits only */
            if (x + w > WIDTH || y + ht > HEIGHT) goto out;
            for (int yy = 0; yy < ht; yy++) {
                size_t row_len = is_packed ? (size_t)(w + 7) / 8 : (size_t)w * 4;
                if (read_all(fd, row, row_len)) goto out;
                int gy = y + yy, ty = gy / TH, ly = gy % TH;
                for (int xx = 0; xx < w;) {
                    int gx = x + xx, tx = gx / TW, lx = gx % TW;
                    int run = TW - lx;
                    if (run > w - xx) run = w - xx;
                    struct tile *t = &tiles[ty * COLS + tx];
                    uint16_t *dst = t->px + (size_t)ly * TW + lx;
                    for (int k = 0; k < run; k++) {
                        uint16_t g;
                        if (is_packed) g = (row[(xx + k) / 8] >> (7 - (xx + k) % 8)) & 1 ? 255 : 0;
                        else g = row[(xx + k) * 4]; /* gray to RGB565 */
                        dst[k] = (uint16_t)(((g >> 3) << 11) | ((g >> 2) << 5) | (g >> 3));
                    }
                    note_dirty(t, lx, ly, lx + run - 1, ly);
                    xx += run;
                }
            }
        }
        int announced = 0;
        for (int i = 0; i < NTILES; i++) { /* announce only the tiles that changed, each with its own box */
            struct tile *t = &tiles[i];
            if (t->dx1 < 0) continue;
            tile_update(i, t->dx0, t->dy0, t->dx1 - t->dx0 + 1, t->dy1 - t->dy0 + 1);
            announced++;
            t->dx0 = t->dy0 = 1 << 20;
            t->dx1 = t->dy1 = -1;
        }
        if (rects > 40) fprintf(stderr, "jean-tiles: frame of %d rects arrived at %ld ms, %d tiles announced after %ld ms\n", rects, t_frame, announced, now_ms() - t_frame);
        if (request(fd, 1)) break;
    }
out:
    free(row);
}

int main(int argc, char **argv) {
    const char *host = getenv("JEAN_HOST");
    const char *port = getenv("JEAN_PORT");
    if (argc > 1) { /* AppLoad's backend socket: connect so it knows we are up */
        appload_sock = socket(AF_UNIX, SOCK_SEQPACKET, 0);
        struct sockaddr_un addr;
        memset(&addr, 0, sizeof addr);
        addr.sun_family = AF_UNIX;
        strncpy(addr.sun_path, argv[1], sizeof addr.sun_path - 1);
        if (appload_sock < 0 || connect(appload_sock, (struct sockaddr *)&addr, sizeof addr) != 0) {
            fprintf(stderr, "jean-tiles: cannot reach AppLoad's socket %s (errno %d); going on without\n", argv[1], errno);
            appload_sock = -1;
        }
    }
    for (int i = 0; i < NTILES; i++) tile_connect(i);
    fprintf(stderr, "jean-tiles: %d tiles of %dx%d\n", NTILES, TW, TH);
    for (;;) {
        int fd = dial(host ? host : "10.11.99.8", port ? port : "5911");
        if (fd >= 0) {
            session(fd);
            close(fd);
        }
        sleep(2);
    }
}
