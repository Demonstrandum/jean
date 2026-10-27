"""A stand-in for the tablet: a VNC client that taps and swipes on eink-serve and times how long the resulting frame takes."""
import socket, struct, sys, time

W, H = 1404, 1872
s = socket.create_connection(("127.0.0.1", 5911))
s.settimeout(10)


def rd(n):
    b = bytearray()
    while len(b) < n:
        c = s.recv(min(65536, n - len(b)))
        if not c:
            raise EOFError
        b += c
    return bytes(b)


rd(12); s.sendall(b"RFB 003.008\n"); rd(2); s.sendall(b"\x01"); rd(4); s.sendall(b"\x01")
head = rd(24); rd(struct.unpack(">I", head[20:24])[0])


def request(inc):
    s.sendall(struct.pack(">BBHHHH", 3, inc, 0, 0, W, H))


def update():
    """Read one FramebufferUpdate: (rect count, changed pixels)."""
    h = rd(4)
    n = struct.unpack(">H", h[2:4])[0]
    area = 0
    for _ in range(n):
        x, y, w, hh, enc = struct.unpack(">HHHHi", rd(12))
        rd(w * hh * 4)
        area += w * hh
    return n, area


def pointer(mask, x, y):
    s.sendall(struct.pack(">BBHH", 5, mask, x, y))


def big_change(timeout=6.0):
    """Ask for updates until one changes a lot of the screen (a new screen, not the seconds ticking); seconds elapsed, or None."""
    t0 = time.time()
    while time.time() - t0 < timeout:
        request(1)
        try:
            n, area = update()
        except socket.timeout:
            return None
        if area > 150_000:
            return time.time() - t0
    return None


request(0); update()            # the first full frame
def tap(label, fx, fy):
    pointer(1, fx, fy); time.sleep(0.05); pointer(0, fx, fy)
    t = big_change()
    print(f"{label}: new frame after {t:.2f}s" if t is not None else f"{label}: NO big change within 6s", flush=True)


def swipe(label, x0, y0, x1, y1):
    pointer(1, x0, y0)
    for i in range(1, 6):
        pointer(1, x0 + (x1 - x0) * i // 5, y0 + (y1 - y0) * i // 5); time.sleep(0.02)
    pointer(0, x1, y1)
    t = big_change()
    print(f"{label}: new frame after {t:.2f}s" if t is not None else f"{label}: NO big change within 6s", flush=True)


tap("tab 日 (day)", 19, 1075)
tap("any tap on a sub-screen (home)", 700, 900)
tap("home tile Wx (weather)", 351, 982)
swipe("swipe right (previous)", 900, 1300, 900, 700)   # scene x grows as frame y falls: right = previous
swipe("swipe left (next)", 900, 700, 900, 1300)
tap("tab 首 (home)", 19, 1871 - int(316 * 2.34))
