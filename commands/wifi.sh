#!/bin/sh
# On-demand: `ui.run("wifi")` prints the current link in one line.
if command -v nmcli >/dev/null 2>&1; then nmcli -t -f IN-USE,SIGNAL,SSID dev wifi | grep '^\*'
elif command -v networksetup >/dev/null 2>&1; then networksetup -getairportnetwork en0
else echo "no wifi tool"; fi
