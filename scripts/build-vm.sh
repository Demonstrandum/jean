#!/bin/sh
# Build the NixOS kiosk VM (flake: packages.aarch64-linux.vm-boot: a kernel, an initrd carrying the whole system, the kernel command line) in a Linux container,
# since this machine has no Linux builder. Needs Docker; the Nix store is kept in a volume (jean-nix-store) so a second build is quick. Output: $OUT (/tmp/jean-vm).
set -eu
ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT=${OUT:-/tmp/jean-vm}; SRC=${SRC:-/tmp/jean-src}
mkdir -p "$OUT"; rm -rf "$SRC"
# the working tree without build output (the fonts are Git LFS objects: build from a checkout that has them)
rsync -a --exclude target --exclude display-state --exclude harness-state --exclude .git "$ROOT/" "$SRC/"
docker run --rm --name jean-nix -v jean-nix-store:/nix -v "$SRC":/src:ro -v "$OUT":/out nixos/nix sh -c '
  set -e
  mkdir -p /etc/nix
  printf "experimental-features = nix-command flakes\nsandbox = false\nfilter-syscalls = false\nmax-jobs = auto\ncores = 0\n" >> /etc/nix/nix.conf
  cp -r /src /tmp/jean && cd /tmp/jean
  nix build "path:/tmp/jean#packages.aarch64-linux.vm-boot" -L -o /tmp/result
  rm -f /out/Image /out/initrd /out/cmdline; cp -L /tmp/result/Image /tmp/result/initrd /tmp/result/cmdline /out/; ls -l /out'
