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

- Voice with a real microphone, whisper.cpp and Piper: the code and adapters are tested with fakes; no whisper or Piper is installed on this machine.
- The NixOS configuration evaluates (with the assistant and panel on), but it has not been built or booted: there is no Linux builder here, so the VM and the `checks.vm` test have not run.
- The reMarkable: it shows the display and the touch relay is tested end to end up to the tablet's log (see below). Whether the panel still flashes the gradients on each tick, whether touches feel immediate on the glass, and the effect of `JEAN_REFRESH` have been judged by eye only by the owner, not measured.
- Home Assistant: the translation is tested; no instance was reachable.
- The builder promoting a change automatically (`JEAN_APPROVAL=auto`) has only run against fakes; the live test stops at "needs approval".
- Sensors beyond Home Assistant (GPIO, I²C) are not written.

## reMarkable 2 as the screen

The tablet runs the **"Jean (tiles)"** AppLoad app (`crates/eink/device/tiles` + `jean-tiles.c`): a QML front end that lays the screen out as 12x16 small framebuffer items, and a backend that is a minimal VNC client for `eink-serve`, writing each frame only into the tiles it changes and sending touches back as VNC pointer events. (`jean-view.c` is the older one-big-framebuffer version, kept for reference: it works, but see below.)

**Why tiles.** xochitl damages the panel per Qt item. With one framebuffer item for the whole screen, `rm.framebuffer.updates` logs `swapBuffers: QRegion(0,0 1404x1872)` for every update, however small, so a seconds tick re-drove (flashed) the entire panel and dithered gradients visibly changed. With tiles it logs `QRegion(18,539 17x16)`: only the changed patch. (Turn that log on with `QT_LOGGING_RULES=rm.framebuffer.updates.debug=true` in `xovi/services/xochitl.service/qt-resource-rebuilder.conf`, then read `journalctl`.)

**Why the QML counter-scales.** AppLoad v0.4.2 gives an app a fixed 1620x2160 canvas and scales it by 0.8667 onto the rM2's 1404x1872, which resamples a dither into mush and does not fit a 1404x1872 layout. `main.qml` measures the canvas on the screen and puts the tile grid in a container with the inverse scale, at the screen's top-left, so one tile pixel is one panel pixel. A tile app must be *maximized* (a plain tap on its icon in AppLoad; press-and-hold makes a floating window).

**Mac side** (the tablet is at 10.11.99.1 over USB, the Mac at 10.11.99.8):

```
cargo run -p preview -- 127.0.0.1:8788 display-state
cargo run -p eink --bin eink-serve -- --panel rm2-turned-ccw --listen 0.0.0.0:5911
```

`--panel rm2-turned-ccw` is the tablet held in landscape (`rm2-turned-cw` for the other way). The scene is 800x600 there (4:3, edge to edge) and is sent 1-bit, dithered in independent 64x64 blocks so a change never re-rolls dots elsewhere, as one small rectangle per changed block. `EINK_DEBUG=1` logs how much changes per frame; `--dump out.png` writes the first frame.

**Tablet side** (firmware 3.25.0.119; the versions matter, newer AppLoad builds need Qt 6.10 or hooks this firmware lacks):

- xovi + qt-resource-rebuilder from `asivery/rm-xovi-extensions` tag `v19-23052026`, unpacked in `/home/root/xovi`; `appload.so` **v0.4.2** from `asivery/rm-appload` in `xovi/extensions.d/`.
- Build and install the app: `zig cc -target arm-linux-musleabihf -O2 -static -s -o entry crates/eink/device/jean-tiles.c`, then copy `crates/eink/device/tiles/{manifest.json,icon.png,resources.rcc}` and `entry` (as `backend/entry`) to `/home/root/xovi/exthome/appload/jean-tiles/`. `resources.rcc` is made by `python3 crates/eink/device/mkrcc.py tiles/resources.rcc /ui/main.qml=tiles/ui/main.qml` (a small Qt resource writer: there is no Qt toolchain here; note that file entries must carry the C locale, language 1).
- xovi is tethered: after a reboot run `/home/root/xovi/start` over ssh (it restarts the UI); AppLoad scans its apps and registers their resources then, so restart it after changing an app. Open AppLoad from the left menu and tap Jean (tiles).
- `IdleSuspendDelay=86400000` in `xochitl.conf` keeps the tablet from sleeping (backup: `/home/root/xochitl.conf.bak`).

**Touch** is relayed, the page stays display-only: a tap on a tab shows its screen; on home a tap on a tile shows the screen it `opens`; on any other screen a tap goes home; a swipe across the screen steps to the next (left) or previous (right) tab. `eink-serve` maps the touch to the scene, asks the page (`ui.tap`, `ui.step`), switches it with `ui.go` and sends the new frame at once. `crates/eink/device/touch.c` puts a finger on the tablet's touchscreen from the shell (`touch tap X Y`, `touch swipe X0 Y0 X1 Y1`, screen pixels), which is how the whole path was driven and checked on the real panel; `crates/eink/tools/touch_probe.py` plays the tablet's part against a running `eink-serve` and times each response.

Weather comes from Open-Meteo, with MET Norway as the fallback when its daily quota is spent, the last good reading kept on disk, and sunrise/sunset computed locally so the day dial never depends on the network.
