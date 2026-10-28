#!/usr/bin/env python3
"""Run a shell command in the kiosk VM through its serial console (the netboot image logs in a `nixos` user with sudo automatically):
    vm-sh.py 'systemctl status display-server --no-pager'      prints the output; exit status is the command's."""
import os, socket, sys, time, uuid
SOCK = os.path.join(os.environ.get("OUT", "/tmp/jean-vm"), "serial.sock")
cmd = " ".join(sys.argv[1:])
mark = uuid.uuid4().hex[:8]
s = socket.socket(socket.AF_UNIX); s.settimeout(1); s.connect(SOCK)
def drain(t=0.5):
    out = b""; end = time.time() + t
    while time.time() < end:
        try: out += s.recv(65536)
        except socket.timeout: pass
    return out
drain(0.5); s.sendall(b"\r"); drain(0.5)
s.sendall(f"({cmd}) 2>&1; echo END{mark}:$?\r".encode())
buf = b""; t0 = time.time()
while time.time() - t0 < float(os.environ.get("TIMEOUT", "60")):
    try: buf += s.recv(65536)
    except socket.timeout: pass
    if f"END{mark}:".encode() in buf.split(b"echo END")[-1]: break
text = buf.decode(errors="replace")
body = text.split(f"echo END{mark}", 1)[-1] if False else text
import re
m = re.search(rf"END{mark}:(\d+)", text.split("\n", 1)[-1])
# the echoed command line is in the buffer once; take what comes after it
after = text.split(f"echo END{mark}:$?", 1)[-1]
print(re.sub(rf"END{mark}:\d+.*", "", after, flags=re.S).strip("\r\n"))
sys.exit(int(m.group(1)) if m else 1)
