#!/usr/bin/env python3
"""The computer itself, every 5 s: cpu, load, memory, swap, disk (with a daily history), top processes, uptime.
Linux and macOS, standard library only. Histories are kept here so a page reload doesn't lose the graphs."""
import datetime, json, os, platform, re, shutil, subprocess, time
from _lib import ROOT, emit, put, forever, log

LINUX = platform.system() == "Linux"
HIST = 60
cpu_hist, load_hist = [], {"1": [], "5": [], "15": []}
core_hist = {}
prev_stat = None


def sh(cmd, timeout=10):
    return subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=timeout).stdout


def push(h, v, n=HIST):
    h.append(round(v, 3))
    del h[:-n]


def cpu():
    global prev_stat
    out = {}
    if LINUX:
        rows = {}
        for line in open("/proc/stat"):
            if line.startswith("cpu"):
                f = line.split()
                vals = list(map(int, f[1:9]))
                rows[f[0]] = (sum(vals), vals[3] + vals[4])
        if prev_stat:
            def pct(k):
                dt = rows[k][0] - prev_stat[k][0]
                return 0 if dt <= 0 else 1 - (rows[k][1] - prev_stat[k][1]) / dt
            total = pct("cpu")
            push(cpu_hist, total)
            cores = sorted(k for k in rows if k != "cpu" and k[3:].isdigit())
            for k in cores:
                push(core_hist.setdefault(k, []), pct(k))
            out["cpu.total"] = round(total * 100, 1)
            out["cpu.cores"] = {k: core_hist[k] for k in cores}
        prev_stat = rows
    else:
        m = re.findall(r"CPU usage: ([\d.]+)% user, ([\d.]+)% sys, ([\d.]+)% idle", sh("top -l 2 -s 1 -n 0"))
        if m:
            total = 100 - float(m[-1][2])
            push(cpu_hist, total / 100)
            out["cpu.total"] = round(total, 1)
    out["cpu.hist"] = cpu_hist
    out["cpu.count"] = os.cpu_count()
    return out


def load():
    l = os.getloadavg()
    for k, v in zip(("1", "5", "15"), l):
        push(load_hist[k], v)
    return {"sys.load": round(l[0], 2), "sys.load5": round(l[1], 2), "sys.load15": round(l[2], 2), "sys.load_hist": {"m1": load_hist["1"], "m5": load_hist["5"], "m15": load_hist["15"]}}


def memory():
    GB = 1024 ** 3
    if LINUX:
        mi = {k: int(v.split()[0]) * 1024 for k, v in (l.split(":") for l in open("/proc/meminfo"))}
        total, free = mi["MemTotal"], mi["MemFree"]
        cache = mi.get("Cached", 0) + mi.get("Buffers", 0) + mi.get("SReclaimable", 0)
        apps = max(0, total - free - cache)
        swap_t, swap_u = mi.get("SwapTotal", 0), mi.get("SwapTotal", 0) - mi.get("SwapFree", 0)
    else:
        total = int(sh("sysctl -n hw.memsize"))
        vs = sh("vm_stat")
        page = int(re.search(r"page size of (\d+)", vs).group(1))
        pg = lambda name: int(re.search(name + r":\s+(\d+)", vs).group(1)) * page
        free = pg("Pages free") + pg("Pages speculative")
        cache = pg("Pages inactive") + pg("Pages purgeable")
        apps = max(0, total - free - cache)
        m = re.search(r"total = ([\d.]+)M\s+used = ([\d.]+)M", sh("sysctl -n vm.swapusage"))
        swap_t, swap_u = (float(m.group(1)) * 1024 ** 2, float(m.group(2)) * 1024 ** 2) if m else (0, 0)
    r = lambda b: round(b / GB, 2)
    return {"mem.total": r(total), "mem.apps": r(apps), "mem.cache": r(cache), "mem.free": r(free), "mem.pct": round(100 * (total - free - cache) / total), "swap.used": r(swap_u), "swap.total": r(swap_t)}


def disk():
    TB = 1000 ** 4
    path = "/System/Volumes/Data" if not LINUX and os.path.exists("/System/Volumes/Data") else "/"
    u = shutil.disk_usage(path)
    f = os.path.join(ROOT, "data", "disk-history.json")
    try:
        hist = json.load(open(f))
    except Exception:
        hist = []
    today = datetime.date.today().isoformat()
    if not hist or hist[-1][0] != today:
        hist.append([today, u.used])
        hist = hist[-60:]
        json.dump(hist, open(f, "w"))
    else:
        hist[-1][1] = u.used
    series = [round(b / TB, 4) for _, b in hist]
    out = {"disk.used": round(u.used / TB, 3), "disk.total": round(u.total / TB, 3), "disk.pct": round(100 * u.used / u.total), "disk.hist": series}
    return out


def procs():
    cmd = "ps -eo pcpu,comm --sort=-pcpu" if LINUX else "ps -Ao pcpu,comm -r"
    rows = []
    for line in sh(cmd).splitlines()[1:]:
        p = line.strip().split(None, 1)
        if len(p) == 2 and p[1].split("/")[-1] not in ("ps", "python3", "Python"):
            rows.append({"name": p[1].split("/")[-1][:18], "cpu": float(p[0])})
    return {"proc.top": rows[:4], "proc.count": len(rows)}


def system():
    out = {"sys.host": platform.node().split(".")[0], "sys.kernel": platform.release()}
    if LINUX:
        out["sys.uptime"] = int(float(open("/proc/uptime").read().split()[0]))
        temps = []
        for z in os.listdir("/sys/class/thermal") if os.path.isdir("/sys/class/thermal") else []:
            try:
                temps.append(int(open(f"/sys/class/thermal/{z}/temp").read()) / 1000)
            except Exception:
                pass
        if temps:
            out["sys.temp"] = round(max(temps))
    else:
        m = re.search(r"sec = (\d+)", sh("sysctl -n kern.boottime"))
        if m:
            out["sys.uptime"] = int(time.time()) - int(m.group(1))
    return out


def step():
    d = {}
    for part in (cpu, load, memory, disk, procs, system):
        try:
            d.update(part())
        except Exception as e:
            log(part.__name__, "failed:", repr(e))
    put(d)


if __name__ == "__main__":
    forever(2 if LINUX else 3, step, retry=10)
