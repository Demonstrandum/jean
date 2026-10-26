# jean

A voice-driven living-room assistant's **display**: a TypeScript UI kit for a 800×480 e-ink panel, run directly in a browser page, and a
small Rust server that gives an agent (or you) a REPL into that page, journals every change in git, and feeds it live data from the
machine it runs on.

* `crates/ui`: the UI kit (`ts/ui.ts`): components, a guaranteed-fit layout (nothing can scroll or clip), a master refresh clock for
  e-ink, and a 1-bit dither preview. Types are stripped by oxc at load time.
* `crates/store`: the event-sourced journal (a git repo of evals and snapshots), replayed on restart.
* `crates/preview`: the server: serves the pages, bridges `/eval` and `/query` to the page, follows live data (`/sub`, `/push`), runs
  `feeds/` scripts and named `commands/`.
* `crates/ui/ts/screens.ts`: the real screens (HOME DAY WEATHER ENERGY LISTS CLAUDE SYSTEM) and the tab bar; `crates/preview/static/index.html` is the gallery of sample layouts.
* `feeds/`, `commands/`, `data/`: the scripts and files the agent edits (machine, network, Wi-Fi, weather, Octopus, carbon, Google Calendar).
* `nix/`, `flake.nix`: a NixOS module (server + optional kiosk browser). Not yet built or booted.

## Run it

```sh
cp config.example.json config.json           # your location and postcode
mkdir secrets && $EDITOR secrets/secrets.json # optional: octopus {account, api_key}, google {service_account_file, calendar_id}
cargo run -p preview                          # http://127.0.0.1:8788 (scenes) and /live (the REPL-driven display)
```

Docs: [screens and the tab bar](docs/screens.md), [live data and the refresh clock](docs/live-data.md), [persistence](docs/persistence.md), [autonomy](docs/autonomy.md).

## Fonts

All font files are stored with Git LFS (`git lfs install` before cloning, or `git lfs pull` after; the build stops with that hint if it sees a
pointer file). Inter and Archivo are included (OFL). The default monospace, Berkeley Mono, is licensed: only an age-encrypted copy
(`crates/ui/fonts/berkeley.woff2.age`) is in the repo, and the server decrypts it at startup with `secrets/font.key` (in memory; the plaintext is
never written). Without the key everything works and the UI falls back to the system monospace.

## Licence

GPL-3.0-or-later. See [LICENSE](LICENSE) and [THIRD_PARTY.md](THIRD_PARTY.md).
