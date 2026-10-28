/* touch: put a finger on the reMarkable 2's touchscreen from the shell, to drive the UI for testing.
 *   touch tap X Y            a tap at X,Y
 *   touch swipe X0 Y0 X1 Y1  a drag from one point to another
 * X,Y are screen pixels (0..1403 across, 0..1871 down, the portrait framebuffer's). The panel itself reports y upside down, so a raw event is written with
 * y flipped; set TOUCH_RAW=1 to give raw panel coordinates instead.
 * Events are written to the evdev node, which the kernel passes on to whoever reads it (Qt in xochitl) as if the panel had sent them. */
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

struct ev { uint32_t sec, usec; uint16_t type, code; int32_t value; }; /* the 32-bit ARM kernel's struct input_event */

#define EV_SYN 0
#define EV_KEY 1
#define EV_ABS 3
#define SYN_REPORT 0
#define BTN_TOUCH 0x14a
#define ABS_MT_SLOT 0x2f
#define ABS_MT_POSITION_X 0x35
#define ABS_MT_POSITION_Y 0x36
#define ABS_MT_TRACKING_ID 0x39

static int fd, raw;
#define FLIP(y) (raw ? (y) : 1871 - (y))

static void put(uint16_t type, uint16_t code, int32_t value) {
    struct ev e = {0, 0, type, code, value};
    if (write(fd, &e, sizeof e) != sizeof e) { perror("write"); exit(1); }
}

static void down(int x, int y) {
    put(EV_ABS, ABS_MT_SLOT, 0);
    put(EV_ABS, ABS_MT_TRACKING_ID, 1000 + (getpid() & 0xff));
    put(EV_ABS, ABS_MT_POSITION_X, x);
    put(EV_ABS, ABS_MT_POSITION_Y, FLIP(y));
    put(EV_KEY, BTN_TOUCH, 1);
    put(EV_SYN, SYN_REPORT, 0);
}

static void move(int x, int y) {
    put(EV_ABS, ABS_MT_POSITION_X, x);
    put(EV_ABS, ABS_MT_POSITION_Y, FLIP(y));
    put(EV_SYN, SYN_REPORT, 0);
}

static void up(void) {
    put(EV_ABS, ABS_MT_TRACKING_ID, -1);
    put(EV_KEY, BTN_TOUCH, 0);
    put(EV_SYN, SYN_REPORT, 0);
}

int main(int argc, char **argv) {
    fd = open(getenv("TOUCH_DEV") ? getenv("TOUCH_DEV") : "/dev/input/event2", O_WRONLY);
    if (fd < 0) { perror("open"); return 1; }
    raw = getenv("TOUCH_RAW") != NULL;
    if (argc == 4 && !strcmp(argv[1], "tap")) {
        int x = atoi(argv[2]), y = atoi(argv[3]);
        down(x, y); usleep(80000); up();
    } else if (argc == 6 && !strcmp(argv[1], "swipe")) {
        int x0 = atoi(argv[2]), y0 = atoi(argv[3]), x1 = atoi(argv[4]), y1 = atoi(argv[5]);
        down(x0, y0);
        for (int i = 1; i <= 12; i++) { usleep(20000); move(x0 + (x1 - x0) * i / 12, y0 + (y1 - y0) * i / 12); }
        usleep(40000); up();
    } else {
        fprintf(stderr, "usage: touch tap X Y | touch swipe X0 Y0 X1 Y1\n");
        return 2;
    }
    return 0;
}
