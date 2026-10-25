#!/bin/sh
# Wi-Fi link every 15 s: wifi.ssid, wifi.rssi (dBm), wifi.bars (0-4). Linux: nmcli, else iw. macOS: system_profiler (about 4 s a run).
bars() { awk -v r="$1" 'BEGIN{b=0; if(r>=-55)b=4; else if(r>=-65)b=3; else if(r>=-75)b=2; else if(r>=-85)b=1; print b}'; }
while true; do
  ssid=""; rssi=""
  if command -v nmcli >/dev/null 2>&1; then
    row=$(nmcli -t -f IN-USE,SIGNAL,SSID dev wifi 2>/dev/null | grep '^\*' | head -n1)
    if [ -n "$row" ]; then
      sig=$(echo "$row" | cut -d: -f2); ssid=$(echo "$row" | cut -d: -f3-)
      rssi=$(awk -v s="$sig" 'BEGIN{printf "%d", s/2-100}')      # nmcli gives a percentage; this is the usual approximation
    fi
  elif command -v iw >/dev/null 2>&1; then
    dev=$(iw dev 2>/dev/null | awk '$1=="Interface"{print $2; exit}')
    link=$(iw dev "$dev" link 2>/dev/null)
    rssi=$(echo "$link" | awk '/signal:/{print $2}'); ssid=$(echo "$link" | sed -n 's/^[[:space:]]*SSID: //p')
  elif command -v system_profiler >/dev/null 2>&1; then
    line=$(system_profiler SPAirPortDataType 2>/dev/null | awk '/Current Network Information:/{f=1;next} f&&!ssid{gsub(/^ +| *:$/,"");ssid=$0;next} f&&/Signal \/ Noise/{print ssid "|" $4; exit}')
    ssid=${line%|*}; rssi=${line#*|}; [ "$line" = "" ] && rssi=""
  fi
  if [ -n "$rssi" ]; then
    echo "wifi.rssi=$rssi"; echo "wifi.bars=$(bars "$rssi")"; case "$ssid" in ""|"<redacted>") ;; *) echo "wifi.ssid=$ssid";; esac   # macOS hides the SSID from scripts
  else
    echo "wifi.bars=0"
  fi
  sleep 15
done
