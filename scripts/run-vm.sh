#!/bin/sh
# Boot the kiosk VM built by scripts/build-vm.sh under QEMU (aarch64, Hypervisor.framework on Apple Silicon), with the display server on host port 18788, the panel's
# VNC on 15900, a virtio GPU for the cage+Chromium kiosk, the serial console (a socket, $OUT/serial.sock, for scripts/vm-sh.py, and a log, $OUT/serial.log) and a monitor socket in $OUT/mon.sock (screendump).
set -eu
OUT=${OUT:-/tmp/jean-vm}
rm -f "$OUT/mon.sock" "$OUT/serial.log" "$OUT/serial.sock"
exec qemu-system-aarch64 -M virt -accel hvf -cpu host -m "${MEM:-4096}" -smp 4 \
  -kernel "$OUT/Image" -initrd "$OUT/initrd" -append "$(cat "$OUT/cmdline")" \
  -device virtio-net-pci,netdev=n -netdev user,id=n,hostfwd=tcp::18788-:8788,hostfwd=tcp::15900-:5900 \
  -device virtio-gpu-pci -device virtio-keyboard-pci -device virtio-mouse-pci -display none \
  -chardev "socket,id=ser,path=$OUT/serial.sock,server=on,wait=off,logfile=$OUT/serial.log" -serial chardev:ser -monitor "unix:$OUT/mon.sock,server,nowait"
