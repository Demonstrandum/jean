#!/bin/sh
# The house on emulators: a Home Assistant emulator, the real feed (feeds/homeassistant.py) in a scratch project root with its own config and token, a scratch
# display with a headless page, and the assistant listening to a silent microphone: someone "walks in" (the motion sensor turns on) on a cold morning and the
# assistant greets them, aloud, with the jacket advice. Needs the same tools as scripts/emulate-voice.sh.
set -eu
ROOT=$(cd "$(dirname "$0")/.." && pwd)
HOME_J=${JEAN_HOME:-$HOME/.local/share/jean}
WORK=${WORK:-/tmp/jean-home}
free_port() { python3 -c "import socket;s=socket.socket();s.bind(('127.0.0.1',0));print(s.getsockname()[1])"; }
HA=$(free_port); PORT=$(free_port)
# the greeting is for mornings (05:00 to 11:00 local): live in a timezone where it is 08:00 now, whatever the hour here
TZ_NAME=$(python3 -c "import datetime;h=datetime.datetime.now(datetime.timezone.utc).hour;n=(8-h)%24;n=n if n<=14 else n-24;print('Etc/GMT%+d' % (-n) if n else 'Etc/UTC')")
echo "the household is in $TZ_NAME"
CHROME=${CHROME:-"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"}
rm -rf "$WORK"; mkdir -p "$WORK/root/feeds" "$WORK/root/secrets" "$WORK/spoken" "$WORK/bin"
cp "$ROOT/feeds/homeassistant.py" "$ROOT/feeds/_lib.py" "$WORK/root/feeds/"
cat > "$WORK/root/config.json" <<JSON
{"tz": "$TZ_NAME", "homeassistant": {"url": "http://127.0.0.1:$HA", "temp": "sensor.living_room_temperature", "humidity": "sensor.living_room_humidity",
 "lux": "sensor.living_room_illuminance", "motion": "binary_sensor.living_room_motion", "lights": ["light.ceiling", "light.lamp"], "doors": ["binary_sensor.front_door", "binary_sensor.back_door", "binary_sensor.garage_door"]}}
JSON
echo '{"homeassistant_token": "emu-token"}' > "$WORK/root/secrets/secrets.json"
cargo build -q -p preview -p harness --manifest-path "$ROOT/Cargo.toml"
python3 "$ROOT/scripts/ha-emulator.py" "$HA" emu-token & HA_PID=$!
"$ROOT/target/debug/preview" 127.0.0.1:$PORT "$WORK/store" "$WORK/root" >/dev/null 2>&1 & DISPLAY_PID=$!
trap 'kill $HA_PID $DISPLAY_PID $CHROME_PID $JEAN_PID 2>/dev/null || true' EXIT
until curl -sf "http://127.0.0.1:$PORT/status" >/dev/null && ! curl -s "http://127.0.0.1:$PORT/" | grep -q "Soft alerts"; do sleep 0.5; done
"$CHROME" --headless=new --disable-gpu --no-sandbox --window-size=1000,800 --user-data-dir="$WORK/chrome" "http://127.0.0.1:$PORT/live?panel&size=800x600" >/dev/null 2>&1 & CHROME_PID=$!
until curl -s "http://127.0.0.1:$PORT/status" | grep -q '"connected":true'; do sleep 0.5; done
sleep 6
echo "--- the feed's room.* values on the display:"
curl -s "http://127.0.0.1:$PORT/data" | python3 -c "import sys,json;d=json.load(sys.stdin);[print(' ',k,'=',json.dumps(d[k]['value'])) for k in sorted(d) if k.startswith('room.')]"
curl -s -X POST "http://127.0.0.1:$PORT/push" -d '{"wx.temp": 3, "wx.rain_from": null}' >/dev/null
# a silent microphone, and the assistant listening
head -c $((32000 * 90)) /dev/zero > "$WORK/mic.raw"; cp "$ROOT/scripts/fake-rec.py" "$WORK/bin/rec"; chmod +x "$WORK/bin/rec"
FAKE_MIC="$WORK/mic.raw" SPOKEN_DIR="$WORK/spoken" PATH="$WORK/bin:$PATH" JEAN_MODE=voice JEAN_LLM=cli JEAN_DISPLAY="http://127.0.0.1:$PORT" JEAN_DATA="$WORK/state" JEAN_TZ=$TZ_NAME \
JEAN_WHISPER_BIN=whisper-cli JEAN_WHISPER_MODEL="$HOME_J/models/ggml-base.en.bin" JEAN_TTS=piper JEAN_PIPER_BIN="$HOME_J/piper-venv/bin/piper" JEAN_PIPER_MODEL="$HOME_J/voices/en_GB-alba-medium.onnx" JEAN_PLAYER="$ROOT/scripts/fake-player.sh" \
"$ROOT/target/debug/jean" > "$WORK/jean.log" 2>&1 & JEAN_PID=$!
sleep 12
echo "--- someone walks in: the motion sensor turns on, and the front door opens"
curl -s -X POST "http://127.0.0.1:$(echo $HA)/_emulate" -d '{"binary_sensor.living_room_motion": "on", "binary_sensor.front_door": "on"}' >/dev/null
sleep 25
echo "--- the room.* values now:"
curl -s "http://127.0.0.1:$PORT/data" | python3 -c "import sys,json;d=json.load(sys.stdin);[print(' ',k,'=',json.dumps(d[k]['value'])) for k in ('room.motion','room.doors') if k in d]"
echo "--- the ROOM screen, as text:"
curl -s -X POST "http://127.0.0.1:$PORT/query" -d 'ui.go("room")' >/dev/null; sleep 3
curl -s -X POST "http://127.0.0.1:$PORT/query" -d 'ui.describe().tiles.map((t) => t.zone + " " + t.title + ": " + t.text)' | python3 -c "import sys,json;[print(' ',x) for x in json.load(sys.stdin)['value']]"
echo "--- jean's log:"; cat "$WORK/jean.log"
echo "--- what the assistant said aloud, transcribed back:"
for f in "$WORK"/spoken/reply-*.wav; do [ -f "$f" ] || continue; ffmpeg -y -loglevel error -i "$f" -ar 16000 -ac 1 "$WORK/r.wav"; printf "%s: " "$(basename "$f")"; whisper-cli -m "$HOME_J/models/ggml-base.en.bin" -f "$WORK/r.wav" -nt -np 2>/dev/null | tr '\n' ' '; echo; done
