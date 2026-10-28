#!/bin/sh
# The fit test: start a display server on a scratch port with an empty journal, open its page in headless Chrome, and ask it to show every
# screen with sample data and with none (GET /selftest). Exits non-zero if any screen has a tile that does not fit its cell.
# Exit codes: 0 = every screen fits, 1 = something does not, 2 = no Chrome/Chromium (skipped). `cargo test` runs this.
# usage: scripts/selftest.sh [port]      (a free port by default; set CHROME to a Chrome/Chromium binary if it is not found)
set -eu
[ -z "${JEAN_NO_BROWSER:-}" ] || { echo "selftest: JEAN_NO_BROWSER is set; skipped" >&2; exit 2; }
cd "$(dirname "$0")/.."
PORT="${1:-$(python3 -c "import socket; s = socket.socket(); s.bind(('127.0.0.1', 0)); print(s.getsockname()[1])")}"
CHROME="${CHROME:-}"
for c in "$CHROME" "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" "/Applications/Chromium.app/Contents/MacOS/Chromium" google-chrome chromium chromium-browser; do
  [ -n "$c" ] && command -v "$c" >/dev/null 2>&1 && CHROME="$c" && break
  [ -n "$c" ] && [ -x "$c" ] && CHROME="$c" && break
done
[ -x "$CHROME" ] || command -v "$CHROME" >/dev/null 2>&1 || { echo "selftest: no Chrome/Chromium found (set CHROME=...)" >&2; exit 2; }
if [ -z "${SERVER_BIN:-}" ]; then cargo build -q -p preview; SERVER_BIN=target/debug/preview; fi   # cargo test passes the binary it built
STATE=$(mktemp -d); PROFILE=$(mktemp -d)
cleanup() { kill "$SERVER" "$BROWSER" 2>/dev/null || true; rm -rf "$STATE" "$PROFILE"; }
trap cleanup EXIT
"$SERVER_BIN" "127.0.0.1:$PORT" "$STATE" . >"$STATE/server.log" 2>&1 & SERVER=$!
for _ in $(seq 1 40); do curl -sf "http://127.0.0.1:$PORT/status" >/dev/null 2>&1 && break; sleep 0.5; done
"$CHROME" --headless=new --disable-gpu --no-sandbox --user-data-dir="$PROFILE" --window-size=1000,800 "http://127.0.0.1:$PORT/live" >/dev/null 2>&1 & BROWSER=$!
# wait for the page to connect and the seeds to be applied
for _ in $(seq 1 60); do
  n=$(curl -sf "http://127.0.0.1:$PORT/status" 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['tail'] if d['connected'] else -1)" 2>/dev/null || echo -1)
  [ "${n:-0}" -ge "$(ls crates/ui/seed/*.ts | wc -l | tr -d " ")" ] && break
  sleep 1
done
EXPECTED=$(ls crates/ui/seed/*.ts | wc -l | tr -d " ")
fail=0
for mode in "" "?data=empty"; do
  curl -sf "http://127.0.0.1:$PORT/selftest$mode" | python3 -c "
import sys, json
mode = '$mode' or 'sample data'
d = json.load(sys.stdin)
if 'screens' not in d:
    print('selftest', mode, ':', d); sys.exit(1)
expected = $EXPECTED
if len(d['screens']) != expected:
    print('selftest', mode, ': %d screens, expected %d: a shipped screen was rejected at start-up' % (len(d['screens']), expected))
    for l in open('$STATE/server.log'):
        if 'REJECTED' in l: print('  ', l.strip()[:600])
    sys.exit(1)
bad = [r for r in d['screens'] if not r['ok']]
print('selftest (%s): %d screens, %d not ok' % (mode, len(d['screens']), len(bad)))
for r in bad: print('  ', r['screen'], 'crowded:', r['crowded'], 'dropped:', r['dropped'], 'clipped:', r['clipped'])
sys.exit(1 if bad else 0)" || fail=1
done
exit $fail
