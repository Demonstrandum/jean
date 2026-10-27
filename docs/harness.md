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
- The reMarkable itself: the VNC stream is tested with a hand-written client, not with the tablet's viewer. Remote-desktop latency and the tablet's refresh behaviour are unknown.
- Home Assistant: the translation is tested; no instance was reachable.
- The builder promoting a change automatically (`JEAN_APPROVAL=auto`) has only run against fakes; the live test stops at "needs approval".
- Sensors beyond Home Assistant (GPIO, I²C) are not written.
