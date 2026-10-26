#!/bin/sh
# The fit test: start a display server on a scratch port with an empty journal, open its page in headless Chrome, and ask it to show every
# screen with sample data and with none (GET /selftest). Exits non-zero if any screen has a tile that does not fit its cell.
# usage: scripts/selftest.sh [port]      (a free port by default; set CHROME to a Chrome/Chromium binary if it is not found)
set -eu
cd "$(dirname "$0")/.."
PORT="${1:-$(python3 -c "import socket; s = socket.socket(); s.bind(('127.0.0.1', 0)); print(s.getsockname()[1])")}"
CHROME="${CHROME:-}"
for c in "$CHROME" "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" "/Applications/Chromium.app/Contents/MacOS/Chromium" google-chrome chromium chromium-browser; do
  [ -n "$c" ] && command -v "$c" >/dev/null 2>&1 && CHROME="$c" && break
  [ -n "$c" ] && [ -x "$c" ] && CHROME="$c" && break
done
[ -x "$CHROME" ] || command -v "$CHROME" >/dev/null 2>&1 || { echo "selftest: no Chrome/Chromium found (set CHROME=...)" >&2; exit 2; }
cargo build -q -p preview
STATE=$(mktemp -d); PROFILE=$(mktemp -d)
cleanup() { kill "$SERVER" "$BROWSER" 2>/dev/null || true; rm -rf "$STATE" "$PROFILE"; }
trap cleanup EXIT
target/debug/preview "127.0.0.1:$PORT" "$STATE" . >"$STATE/server.log" 2>&1 & SERVER=$!
for _ in $(seq 1 40); do curl -sf "http://127.0.0.1:$PORT/status" >/dev/null 2>&1 && break; sleep 0.5; done
"$CHROME" --headless=new --disable-gpu --no-sandbox --user-data-dir="$PROFILE" --window-size=1000,800 "http://127.0.0.1:$PORT/live" >/dev/null 2>&1 & BROWSER=$!
# wait for the page to connect and the seeds to be applied
for _ in $(seq 1 60); do
  n=$(curl -sf "http://127.0.0.1:$PORT/status" 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['tail'] if d['connected'] else -1)" 2>/dev/null || echo -1)
  [ "${n:-0}" -ge 7 ] && break
  sleep 1
done
fail=0
for mode in "" "?data=empty"; do
  curl -sf "http://127.0.0.1:$PORT/selftest$mode" | python3 -c "
import sys, json
mode = '$mode' or 'sample data'
d = json.load(sys.stdin)
if 'screens' not in d:
    print('selftest', mode, ':', d); sys.exit(1)
bad = [r for r in d['screens'] if not r['ok']]
print('selftest (%s): %d screens, %d not ok' % (mode, len(d['screens']), len(bad)))
for r in bad: print('  ', r['screen'], 'crowded:', r['crowded'], 'dropped:', r['dropped'], 'clipped:', r['clipped'])
sys.exit(1 if bad else 0)" || fail=1
done
exit $fail
