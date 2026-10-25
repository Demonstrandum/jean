#!/usr/bin/env python3
"""Network throughput of the default interface, every 5 s: net.down / net.up in MB/s, with 60-sample histories, plus net.iface and net.ip."""
import platform, re, socket, subprocess, time
from _lib import put, log, forever

LINUX = platform.system() == "Linux"
down_h, up_h = [], []
prev = None


def sh(cmd):
    return subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=10).stdout


def iface():
    if LINUX:
        m = re.search(r"dev (\S+)", sh("ip route show default"))
    else:
        m = re.search(r"interface: (\S+)", sh("route -n get default"))
    return m.group(1) if m else None


def counters(i):
    if LINUX:
        for line in open("/proc/net/dev"):
            if line.strip().startswith(i + ":"):
                f = line.split(":")[1].split()
                return int(f[0]), int(f[8])
    else:
        for line in sh("netstat -ibn").splitlines():
            f = line.split()
            if f and f[0] == i and len(f) > 3 and f[2].startswith("<Link"):
                return int(f[-5]), int(f[-2])
    return None


def local_ip():
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("10.255.255.255", 1))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except Exception:
        return None


def step():
    global prev
    i = iface()
    if not i:
        put({"net.iface": None})
        return
    c, now = counters(i), time.time()
    if c and prev and prev[0] == i:
        dt = max(1, now - prev[2])
        d, u = max(0, (c[0] - prev[1][0]) / dt / 1048576), max(0, (c[1] - prev[1][1]) / dt / 1048576)
        down_h.append(round(d, 3)); up_h.append(round(u, 3))
        del down_h[:-60], up_h[:-60]
        put({"net.iface": i, "net.ip": local_ip(), "net.down": round(d, 2), "net.up": round(u, 2), "net.down_hist": down_h, "net.up_hist": up_h})
    prev = (i, c, now) if c else None


if __name__ == "__main__":
    forever(5, step, retry=10)
