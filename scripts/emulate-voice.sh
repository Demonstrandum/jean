#!/bin/sh
# The whole voice path without a microphone or a speaker: Piper speaks the utterances into a track, a fake `rec` plays the track to the harness at real-time
# pace (its voice-activity detector, wake phrase, whisper.cpp, the agent and Piper all run unmodified), replies are kept as WAV files and transcribed back.
#   scripts/emulate-voice.sh [seconds]       needs: whisper-cli, piper (a venv), ffmpeg, sox, the logged-in `claude`; models under ~/.local/share/jean
set -eu
ROOT=$(cd "$(dirname "$0")/.." && pwd)
HOME_J=${JEAN_HOME:-$HOME/.local/share/jean}
WORK=${WORK:-/tmp/jean-voice}
PIPER="$HOME_J/piper-venv/bin/piper"; VOICE="$HOME_J/voices/en_GB-alba-medium.onnx"; WHISPER_MODEL="$HOME_J/models/ggml-base.en.bin"
DURATION=${1:-110}
PORT=${PORT:-$(python3 -c "import socket;s=socket.socket();s.bind(('127.0.0.1',0));print(s.getsockname()[1])")}
CHROME=${CHROME:-"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"}
rm -rf "$WORK"; mkdir -p "$WORK/spoken" "$WORK/bin"
# the track: silence, a request, a remark that is not for the assistant, another request
say() { echo "$1" | "$PIPER" --model "$VOICE" --output_file "$WORK/$2.wav" 2>/dev/null; ffmpeg -y -loglevel error -i "$WORK/$2.wav" -ar 16000 -ac 1 -f s16le "$WORK/$2.raw"; }
say "Hey Claude, set a timer for ten minutes for the tea." u1
say "I think we should get oat milk this week." u2
say "Hey Claude, what is the weather like?" u3
silence() { head -c $((32000 * $1)) /dev/zero; }
{ silence 3; cat "$WORK/u1.raw"; silence 28; cat "$WORK/u2.raw"; silence 4; cat "$WORK/u3.raw"; silence 30; } > "$WORK/mic.raw"
cp "$ROOT/scripts/fake-rec.py" "$WORK/bin/rec"; chmod +x "$WORK/bin/rec"
# a display of its own, so the real one is left alone
cargo build -q -p preview -p harness --manifest-path "$ROOT/Cargo.toml"
STORE="$WORK/store"
"$ROOT/target/debug/preview" 127.0.0.1:$PORT "$STORE" "$ROOT" >/dev/null 2>&1 &
DISPLAY_PID=$!
trap 'kill $DISPLAY_PID $CHROME_PID $JEAN_PID $MON_PID 2>/dev/null || true' EXIT
until curl -sf "http://127.0.0.1:$PORT/status" >/dev/null && ! curl -s "http://127.0.0.1:$PORT/" | grep -q "Soft alerts"; do sleep 0.5; done
# headless Chrome is the display's page: evals, screens and live data need one
"$CHROME" --headless=new --disable-gpu --no-sandbox --window-size=1000,800 --user-data-dir="$WORK/chrome" "http://127.0.0.1:$PORT/live?panel&size=800x600" >/dev/null 2>&1 &
CHROME_PID=$!
until curl -s "http://127.0.0.1:$PORT/status" | grep -q '"connected":true'; do sleep 0.5; done
sleep 4
# what the display is told, as it happens
python3 - "$PORT" > "$WORK/live.log" <<'PY' &
import json, sys, time, urllib.request
port=sys.argv[1]; last=None; t0=time.time()
while True:
    try:
        d=json.load(urllib.request.urlopen(f"http://127.0.0.1:{port}/data",timeout=3))
        v=lambda k:(d.get(k) or {}).get("value")
        steps=" ".join(f"{s['tool']}{'✓' if s['ok'] else '…' if s['ok'] is None else '✗'}" for s in (v("claude.steps") or []))
        line=f"state={v('claude.state')} heard={v('claude.heard')!r} steps=[{steps}] say={v('claude.say')!r} timer={v('timer.label')}"
        if line!=last: print(f"+{time.time()-t0:5.1f}s {line}",flush=True); last=line
    except Exception: pass
    time.sleep(0.4)
PY
MON_PID=$!
FAKE_MIC="$WORK/mic.raw" SPOKEN_DIR="$WORK/spoken" PATH="$WORK/bin:$PATH" \
JEAN_MODE=voice JEAN_LLM=cli JEAN_CLI_MODEL=${JEAN_CLI_MODEL:-claude-haiku-5-5} JEAN_DISPLAY="http://127.0.0.1:$PORT" JEAN_DATA="$WORK/state" \
JEAN_WHISPER_BIN=whisper-cli JEAN_WHISPER_MODEL="$WHISPER_MODEL" JEAN_TTS=piper JEAN_PIPER_BIN="$PIPER" JEAN_PIPER_MODEL="$VOICE" JEAN_PLAYER="$ROOT/scripts/fake-player.sh" \
"$ROOT/target/debug/jean" > "$WORK/jean.log" 2>&1 &
JEAN_PID=$!
sleep "$DURATION"
kill $JEAN_PID 2>/dev/null || true
echo "--- the display was told (claude.* keys, as they changed):"; cat "$WORK/live.log"
echo "--- what the assistant said, transcribed back from its own speech:"
for f in "$WORK"/spoken/reply-*.wav; do [ -f "$f" ] || continue; ffmpeg -y -loglevel error -i "$f" -ar 16000 -ac 1 "$WORK/r.wav"; printf "%s: " "$(basename "$f")"; whisper-cli -m "$WHISPER_MODEL" -f "$WORK/r.wav" -nt -np 2>/dev/null | tr '\n' ' '; echo; done
echo "--- jean's log:"; tail -n 12 "$WORK/jean.log"
