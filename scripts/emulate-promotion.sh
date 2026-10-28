#!/bin/sh
# The builder and the promotion pipeline for real, on a scratch copy of the project: a queued task, a real model as the builder (the logged-in `claude`) in a
# sandboxed worktree, then the gate, `cargo test --offline`, the canary self-test, the fast-forward swap and the watchdog probes, with approval set to automatic.
# The live repository here is a copy, so nothing real is touched. Takes several minutes (the checks build the project in the worktree).
#   scripts/emulate-promotion.sh [seconds to wait, default 1500]
set -eu
ROOT=$(cd "$(dirname "$0")/.." && pwd)
WORK=${WORK:-/tmp/jean-promo}
WAIT=${1:-1500}
CHROME=${CHROME:-"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"}
PORT=$(python3 -c "import socket;s=socket.socket();s.bind(('127.0.0.1',0));print(s.getsockname()[1])")
rm -rf "$WORK"; mkdir -p "$WORK/state/tasks"
# a copy of the project with its own git history: the "live" repository
rsync -a --exclude target --exclude display-state --exclude harness-state "$ROOT/" "$WORK/live/"
git -C "$WORK/live" config user.email jean@example.invalid; git -C "$WORK/live" config user.name jean
git -C "$WORK/live" add -A >/dev/null; git -C "$WORK/live" commit -qm "scratch copy" 2>/dev/null || true
cargo build -q -p preview -p harness --manifest-path "$ROOT/Cargo.toml"
"$ROOT/target/debug/preview" 127.0.0.1:$PORT "$WORK/store" "$WORK/live" >/dev/null 2>&1 & DISPLAY_PID=$!
trap 'kill $DISPLAY_PID $CHROME_PID $MON_PID $JEAN_PID 2>/dev/null || true' EXIT
until curl -sf "http://127.0.0.1:$PORT/status" >/dev/null && ! curl -s "http://127.0.0.1:$PORT/" | grep -q "Soft alerts"; do sleep 0.5; done
"$CHROME" --headless=new --disable-gpu --no-sandbox --window-size=1000,800 --user-data-dir="$WORK/chrome" "http://127.0.0.1:$PORT/live?panel&size=800x600" >/dev/null 2>&1 & CHROME_PID=$!
until curl -s "http://127.0.0.1:$PORT/status" | grep -q '"connected":true'; do sleep 0.5; done
sleep 4
NOW=$(python3 -c "import time;print(int(time.time()*1000))")
cat > "$WORK/state/tasks/000001.json" <<JSON
{"id": 1, "title": "Retitle the Claude tile", "details": "On the home screen (crates/ui/seed/10-home.ts) the tile with id \"claude\" has the title \"Claude\". Change its title to \"Jean\" and change nothing else.", "status": "queued", "created_ms": $NOW, "result": ""}
JSON
# what the Claude tile is told while the builder works
python3 - "$PORT" > "$WORK/live.log" <<'PY' &
import json, sys, time, urllib.request
port=sys.argv[1]; last=None; t0=time.time()
while True:
    try:
        d=json.load(urllib.request.urlopen(f"http://127.0.0.1:{port}/data",timeout=3))
        v=lambda k:(d.get(k) or {}).get("value")
        steps=" ".join(f"{s['tool']}{'✓' if s['ok'] else '…' if s['ok'] is None else '✗'}" for s in (v("claude.steps") or []))
        line=f"state={v('claude.state')} task={v('claude.task')!r} steps=[{steps}]"
        if line!=last: print(f"+{time.time()-t0:6.1f}s {line}",flush=True); last=line
    except Exception: pass
    time.sleep(1)
PY
MON_PID=$!
( sleep $((WAIT + 60)) | JEAN_LLM=cli JEAN_CLI_MODEL=${JEAN_CLI_MODEL:-claude-sonnet-5-5} JEAN_BUILDER_MODEL= JEAN_REPO="$WORK/live" JEAN_APPROVAL=auto JEAN_DATA="$WORK/state" JEAN_DISPLAY="http://127.0.0.1:$PORT" "$ROOT/target/debug/jean" > "$WORK/jean.log" 2>&1 ) & JEAN_PID=$!
echo "the builder is working (this takes minutes); log: $WORK/jean.log"
i=0
while [ $i -lt $WAIT ]; do
  st=$(python3 -c "import json,glob;print(json.load(open(glob.glob('$WORK/state/tasks/*.json')[0]))['status'])")
  case "$st" in done|failed|needs_approval) break;; esac
  sleep 10; i=$((i + 10))
done
echo "--- the Claude tile was told:"; cat "$WORK/live.log"
echo "--- the task:"; cat "$WORK"/state/tasks/*.json
echo "--- the audit log:"; cat "$WORK/state/audit.jsonl" 2>/dev/null | python3 -c "import sys,json;[print(' ',json.loads(l).get('action'),json.loads(l).get('detail','')[:100]) for l in sys.stdin]"
echo "--- the live repository's history:"; git -C "$WORK/live" log --oneline | head -n 4
echo "--- the promoted change:"; git -C "$WORK/live" show --stat --format=%s HEAD | head -n 8
echo "--- jean's log:"; tail -n 15 "$WORK/jean.log"
