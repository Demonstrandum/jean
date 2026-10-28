import re, subprocess, time
def ssh(c): return subprocess.run(["/usr/bin/ssh","-o","BatchMode=yes","root@10.11.99.1",c],capture_output=True,text=True,timeout=60).stdout
def ms(t):
    h,m,s=t.split(":"); return (int(h)*3600+int(m)*60+float(s))*1000
def run(label,args):
    out=ssh(f"/home/root/touch {args}"); t0=ms(re.search(r"touch at (\d+:\d+:\d+\.\d+)",out).group(1)); time.sleep(4)
    log=ssh("journalctl -b -o short-precise --no-pager --since '-8 sec' | grep -E 'jean-tiles: frame|swapBuffers'")
    arrive=None; bigswap=None
    for l in log.splitlines():
        m=re.search(r"(\d\d:\d\d:\d\d\.\d+) ",l)
        if not m: continue
        t=ms(m.group(1))
        if t<t0: continue
        if "jean-tiles: frame" in l and arrive is None: arrive=t-t0
        sz=re.search(r"size=(\d+)",l)
        if "swapBuffers" in l and sz and int(sz.group(1))>=30 and bigswap is None: bigswap=t-t0
    print(f"{label:28s} frame at the tablet +{arrive:.0f} ms, first big panel swap +{bigswap:.0f} ms" if arrive and bigswap else f"{label:28s} arrive={arrive} swap={bigswap}")
run("tab day","tap 19 1075")
run("tap sub-screen -> home","tap 700 900")
run("home tile Wx","tap 351 982")
run("swipe left (next)","swipe 900 1300 900 700")
run("swipe right (previous)","swipe 900 700 900 1300")
run("tab home","tap 19 1133")
run("home tile Wx (again)","tap 351 982")
run("tap -> home (again)","tap 700 900")
