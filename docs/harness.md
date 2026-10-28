# The harness: what is built, how it is tested, how to run it

`crates/harness` is the assistant. Everything below exists and has tests; what has not been run on real hardware is listed at the end.

| Module | What it does | Tested with |
|---|---|---|
| `claude`, `agent` | Messages API client (`Anthropic`), the tool loop, and `ClaudeCli` (the logged-in `claude` program as the model, for machines with no API key) | scripted model; the real model (`tests/live_agent.rs`) |
| `session`, `prompts` | one request: show "thinking", act, show and return the answer; history; timers | scripted model, mock display |
| `voice_tools` | show_screen, timers, shopping, notes, calendar, look_up, display_state, start_task | mock display |
| `audio`, `stt`, `tts`, `wake` | microphone and voice-activity detection (sox), whisper.cpp (CLI or server), Piper/`say`/espeak, the wake phrase | fake binaries |
| `triggers`, `run` | the first motion of a morning greets (and nudges about a jacket if rain is near or it is cold); the motion sensor (`room.motion`) feeds it | injected clock |
| `sandbox` | commands run under `sandbox-exec` (macOS) or `bwrap` (Linux): writes only inside the worktree, no network, a timeout | the OS enforcing it: writes outside and connections out fail |
| `builder`, `work` | a task gets a git worktree on `builder/<id>`; file tools are confined; protected paths (`crates/harness`, `crates/store`, `nix`, `flake.*`, `secrets`, `scripts`, `.git`, `config.json`) are refused, and the finished diff is checked again, so a protected file changed through `run` is still caught | temp repos; a real model |
| `promote` | gate, checks, canary (the self-test, from the candidate's worktree), fast-forward, watchdog probes, rollback | git repos, fake canary and live |
| `audit` | every consequential step is a line that commits to the one before; `verify` says where a chain breaks | tamper tests |
| `scraper` | a manifest declares typed keys and fixtures; a page that stops parsing becomes a new fixture and a repair task; a repair must give the old fixtures their recorded answers exactly and may not edit the manifest, old fixtures or answers (`fixtures` binary is the gate) | real parser run in the sandbox |
| `crystal` | the journal is replaced by one change written by the agent, accepted only if two scratch displays (headless Chrome, frozen clock) render every screen byte for byte the same | fake renderer; the real display (`tests/crystal_real.rs`) |

`crates/eink`: Atkinson dithering, reMarkable 2 geometry (1872×1404 landscape, or portrait), damage rectangles, an RFB (VNC) server, and `eink-serve`, which holds the display page in headless Chrome over CDP and publishes each change. Tested end to end against the real display page and a hand-written VNC client.

## Running it

```sh
cargo build --release
target/release/preview 127.0.0.1:8788 display-state .          # the display
JEAN_LLM=cli JEAN_MODE=text target/release/jean                  # the assistant, typed (or ANTHROPIC_API_KEY=... without JEAN_LLM)
JEAN_MODE=voice JEAN_WHISPER_URL=http://127.0.0.1:8790 JEAN_TTS=piper target/release/jean
JEAN_REPO=$PWD JEAN_APPROVAL=person target/release/jean          # also runs the builder; `jean tasks`, `jean approve 3`
target/release/eink-serve --panel rm2 --listen 0.0.0.0:5900     # the display on a reMarkable 2 (vnc://host:5900)
```

Home Assistant: `config.json` `homeassistant` section and `secrets/secrets.json` `homeassistant_token` (see `feeds/homeassistant.py`); the ROOM screen reads `K.room.*`.

NixOS: `services.display = { enable = true; agent = {...}; panel.enable = true; }` in `nix/module.nix` runs the display, whisper, the assistant, `eink-serve` and a watchdog timer. `nix run .#vm` is a VM to poke at.

## Not yet done or not verified

- Voice with a real microphone: the rest of the voice path (voice-activity detection, wake phrase, whisper.cpp, the agent, Piper) runs for real under `scripts/emulate-voice.sh`; only the microphone and speaker hardware (sox's `rec` and a player) are stood in for, so those two adapters are the untested part.
- The NixOS configuration evaluates (with the assistant and panel on), but it has not been built or booted: there is no Linux builder here, so the VM and the `checks.vm` test have not run.
- The reMarkable: it shows the display and the touch relay is tested end to end up to the tablet's log (see below). Whether the panel still flashes the gradients on each tick, whether touches feel immediate on the glass, and the effect of `JEAN_REFRESH` have been judged by eye only by the owner, not measured.
- Home Assistant: run against an emulator (`scripts/ha-emulator.py`, driven end to end by `scripts/emulate-home.sh`); not against a real instance, so its real-world quirks (long-lived token scopes, entity renames) are untested.
- The builder promoting a change automatically (`JEAN_APPROVAL=auto`) has only run against fakes; the live test stops at "needs approval".
- Sensors beyond Home Assistant (GPIO, I²C) are not written.

## The voice path on emulators

`scripts/emulate-voice.sh` runs the whole path without a microphone or a speaker. Piper speaks three utterances into a track (a request, a remark not meant for the assistant, a request); a fake `rec` (`scripts/fake-rec.py`, first in `PATH`) streams the track to the harness at real-time pace as raw PCM, so the harness's own voice-activity detector, wake phrase, whisper.cpp, the agent (the logged-in `claude`) and Piper all run unmodified; a fake player keeps the replies as WAV files, which are transcribed back. The display is a scratch one on a free port with a headless Chrome page, and a monitor prints what the Claude tile is told as it happens. Needs: `brew install whisper-cpp`, the `ggml-base.en.bin` model, `pip install piper-tts` in a venv, a Piper voice (all under `~/.local/share/jean`).

A real run: the request "Set a timer for 10 minutes for the tea" went `Listening, heard, hear✓ set_timer✓, Speaking "Ten minute timer for the tea is set.", Idle`, the remark with no wake phrase was ignored, and "What is the weather like?" answered from the display's live weather; Piper's replies transcribe back word for word.

It found a real weakness: whisper's base model sometimes hears the wake phrase badly ("Eclord, what is the weather like?"), and a request then went unanswered. Two fixes: whisper gets the wake phrases as its initial prompt, and the wake matcher accepts a name with the same consonant skeleton ("Eclord", "cloud", "clawed", "clod" for "Claude") when something vouches for it (the greeting before it matched exactly, or it is a longish word that begins the utterance), so that "The cold wind" and "Hey John" wake nothing.

## The house on emulators

`scripts/ha-emulator.py` is a Home Assistant stand-in: the REST API the feed uses (`GET /api/states/<entity>`, a bearer token, HA's state objects with `last_changed`, a 401 without the token, a 404 for an unknown entity) and a control endpoint to act as the house (`POST /_emulate {"binary_sensor.living_room_motion": "on"}`). `scripts/emulate-home.sh` runs the real `feeds/homeassistant.py` against it in a scratch project root with its own config and token, a scratch display, and the assistant listening to a silent microphone: the room values reach the display, someone "walks in" (motion and the front door turn on), and the assistant says aloud "Good morning. It is cold out, so take a jacket." (the script puts the household in a timezone where it is 08:00 now).

It found two real bugs: while the sensor stays on the feed keeps republishing "now", and the watcher took every poll for new motion (the greeting logic hid it, any other rule would have been flooded), so motion is now one event until the room has been still for a minute (`MOTION_QUIET_MS`); and an entity Home Assistant does not know (a typo, a removed device) was shown as an off light or a closed door, which is a guess, so it is left out.

## The assistant driving the display (the live REPL)

The voice agent has four tools besides the household ones, so a spoken request can change the display itself, live, with no builder and no restart:

| tool | does |
|---|---|
| `ui_describe` | the current screen as data: each tile's id, zone, title, whether it is inverted, the text it shows, the free cells, the screens there are, how it fits |
| `ui_eval` | TypeScript run in the display's page: type-checked, applied, verified for fit, journaled (so it persists and replays), or refused with the reason and nothing changed |
| `ui_query` | read-only evaluation in the page (`ui.screenOf("home").tiles()`, `ui.get(K.wx.temp)`) |
| `ui_undo` | undo the last change |

`show_screen` shows any screen that exists now, including ones the agent made. The prompt (`prompts.rs`, `REPL_GUIDE`) gives the grid, the recipes (change, add, remove a tile; a new screen; live `bind`ings; `invertWhen`; `svg` drawing) and the rule: use `ui_eval` for what is wanted now, `start_task` for a new data source or anything that touches files.

**Is it intuitive enough for a model?** `crates/harness/tests/ui_trials.rs` is the test of that. Each trial gives a real Claude (the logged-in `claude` program, the same JSON-in-prompt tool loop as in production) a request on a fresh scratch display, then checks the display's actual state afterwards. If a trial fails the API is at fault, not the model: the fix goes into the library, the errors or the prompt.

```
cargo test -p harness --test ui_trials -- --ignored --nocapture
JEAN_TRIALS=retitle,undo cargo test ...          # some of them
JEAN_TRIAL_MODEL=claude-sonnet-5-5 cargo test ... # a model
```

The eleven trials: retitle a tile; put up a message; make a screen and show it; replace a tile with a drawn line chart; turn a tile black while live carbon intensity is above 100 (the check pushes values and watches the tile); a tile title that tracks the live shopping list; undo in a second turn; a chart of the live forecast on a full screen; a three-tile screen; take a tile out and use the space; a long text that must fit. All eleven pass, in 2 to 7 tool calls and 16 to 43 seconds each.

What the first runs taught, all fixed because the agents' guesses were reasonable and the API did not meet them: a fixed list of screen names made `show_screen` refuse a screen the agent had just made; `look_up` returned nothing for a question in words (it now ranks words over names, members, signatures and comments); a tile `title` could not be a `bind`; `removeTile(...).setTile(...)` threw because methods returned nothing (they chain now); `ui.screens()` did not exist; `ui.screenOf` of an unknown id was an opaque TypeError (it names the screens); and a refused layout said only "overlap at A1" and the agent spent 17 steps guessing zones (it now lists who is where and which cells are free, and `ui.describe()` and `screenOf(id).free()` give the free cells up front).

## The Claude tile shows the work

While the assistant is busy the home Claude tile turns black (`inverted: invertWhen(K.claude.state, ...)`) and narrates: the state with a blinking cursor, what it heard, the steps as they run (a solid square when done, open while running, crossed if it failed), and what it says. The harness publishes the typed keys `claude.state` (`Presence`: Idle, Listening, Thinking, Speaking, Working), `claude.heard`, `claude.say`, `claude.steps` and `claude.task` as it goes: the runner shows transcription as the first step, the agent loop reports each tool start and end (`agent::Watch`, `activity::Feed`), and the builder worker shows its task title with `Working`. The trials print the live feed of a real run, for example `ui_describe✓ ui_eval✗ look_up✓ ui_eval… ui_eval✓ show_screen✓`. Not run live yet: the builder's `Working` display and a real microphone transcription step.

## reMarkable 2 as the screen

The tablet runs the **"Jean (tiles)"** AppLoad app (`crates/eink/device/tiles` + `jean-tiles.c`): a QML front end that lays the screen out as 12x16 small framebuffer items, and a backend that is a minimal VNC client for `eink-serve`, writing each frame only into the tiles it changes and sending touches back as VNC pointer events. (`jean-view.c` is the older one-big-framebuffer version, kept for reference: it works, but see below.)

**Why tiles.** xochitl damages the panel per Qt item. With one framebuffer item for the whole screen, `rm.framebuffer.updates` logs `swapBuffers: QRegion(0,0 1404x1872)` for every update, however small, so a seconds tick re-drove (flashed) the entire panel and dithered gradients visibly changed. With tiles it logs `QRegion(18,539 17x16)`: only the changed patch. (Turn that log on with `QT_LOGGING_RULES=rm.framebuffer.updates.debug=true` in `xovi/services/xochitl.service/qt-resource-rebuilder.conf`, then read `journalctl`.)

**Why the QML counter-scales.** AppLoad v0.4.2 gives an app a fixed 1620x2160 canvas and scales it by 0.8667 onto the rM2's 1404x1872, which resamples a dither into mush and does not fit a 1404x1872 layout. `main.qml` measures the canvas on the screen and puts the tile grid in a container with the inverse scale, at the screen's top-left, so one tile pixel is one panel pixel. A tile app must be *maximized* (a plain tap on its icon in AppLoad; press-and-hold makes a floating window).

**Mac side** (the tablet is at 10.11.99.1 over USB, the Mac at 10.11.99.8; build `eink-serve` with `--release`: the debug build takes a second to dither and diff a frame):

```
cargo run -p preview -- 127.0.0.1:8788 display-state
cargo run --release -p eink --bin eink-serve -- --panel rm2-turned-ccw --listen 0.0.0.0:5911
```

`--panel rm2-turned-ccw` is the tablet held in landscape (`rm2-turned-cw` for the other way). The scene is 800x600 there (4:3, edge to edge) and is sent 1-bit, dithered in independent 64x64 blocks so a change never re-rolls dots elsewhere, as one small rectangle per changed block. `EINK_DEBUG=1` logs how much changes per frame; `--dump out.png` writes the first frame.

**Tablet side** (firmware 3.25.0.119; the versions matter, newer AppLoad builds need Qt 6.10 or hooks this firmware lacks):

- xovi + qt-resource-rebuilder from `asivery/rm-xovi-extensions` tag `v19-23052026`, unpacked in `/home/root/xovi`; `appload.so` **v0.4.2** from `asivery/rm-appload` in `xovi/extensions.d/`.
- Build and install the app: `zig cc -target arm-linux-musleabihf -O2 -static -s -o entry crates/eink/device/jean-tiles.c`, then copy `crates/eink/device/tiles/{manifest.json,icon.png,resources.rcc}` and `entry` (as `backend/entry`) to `/home/root/xovi/exthome/appload/jean-tiles/`. `resources.rcc` is made by `python3 crates/eink/device/mkrcc.py tiles/resources.rcc /ui/main.qml=tiles/ui/main.qml` (a small Qt resource writer: there is no Qt toolchain here; note that file entries must carry the C locale, language 1).
- xovi is tethered: after a reboot run `/home/root/xovi/start` over ssh (it restarts the UI); AppLoad scans its apps and registers their resources then, so restart it after changing an app. Open AppLoad from the left menu and tap Jean (tiles).
- `IdleSuspendDelay=86400000` in `xochitl.conf` keeps the tablet from sleeping (backup: `/home/root/xochitl.conf.bak`).

**Touch** is relayed, the page stays display-only: a tap on a tab shows its screen; on home a tap on a tile shows the screen it `opens`; on any other screen a tap goes home; a swipe across the screen steps to the next (left) or previous (right) tab. `eink-serve` maps the touch to the scene, asks the page (`ui.tap`, `ui.step`), switches it with `ui.go` and sends the new frame at once. `crates/eink/device/touch.c` puts a finger on the tablet's touchscreen from the shell (`touch tap X Y`, `touch swipe X0 Y0 X1 Y1`, screen pixels), which is how the whole path was driven and checked on the real panel; `crates/eink/tools/touch_probe.py` plays the tablet's part against a running `eink-serve` and times each response; `crates/eink/tools/latency.py` injects touches on the tablet and reads xochitl's own timestamps (microsecond `journalctl -o short-precise`, with `jean-tiles` logging when a release is sent and when a frame arrives) to measure touch-to-panel latency on the tablet's clock. Measured: 460 to 520 ms for a tap, 650 to 720 ms for a swipe (which includes the drag itself), from 1.7 to 3.5 s before two fixes: a release build of the server (a frame was ready after 0.2 s, not 0.85 s) and a server that reads a client's touches while waiting for the screen to change (with one thread per client a touch was not read until the next change of the screen).

Weather comes from Open-Meteo, with MET Norway as the fallback when its daily quota is spent, the last good reading kept on disk, and sunrise/sunset computed locally so the day dial never depends on the network.
