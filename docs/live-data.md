# Live data and the beat

> Values are typed keys now (`K.wx.temp`) with typed enums and epoch-ms instants, declared in `crates/ui/ts/schema.ts`; the examples below that use `"wx.temp"`-style names show the wire names feeds write. See `docs/agent.md`.

E-ink refreshes are slow and visible, so nothing on the page updates on its own schedule.

## The beat (page, `ui.ts`)

One master clock ticks on the wall clock. Jobs (`ui.every`) and live bindings (`feed`) are applied together, in one
synchronous batch, when their period comes due, so one beat is one repaint.

| tier | period | for |
|------|--------|-----|
| `tick` | 1 s | only what must count (timer ring, typewriter) |
| `beat` | 30 s | live data (default) |
| `slow` | 5 min | weather, prices, anything that drifts |

`ui.cadence({beat: 60000})` changes periods; `ui.hold()` / `ui.resume()` freeze all redraws (night mode); `ui.beats()` reports.
Writing data (`ui.set`, or a server push) never touches the DOM: bindings go stale and redraw at their next beat. The first
value for an empty binding is drawn at the next second, not after a whole period. Live updates are never part of a change
(a rollback does not undo them).

```ts
tile("Wi-Fi", feed(["wifi.bars", "wifi.ssid"], ([b, ssid]) => [sigil("bars", b ?? 0), " ", ssid]), { count: "feeds/wifi.sh" })
feed("sys.load", (v) => bignum(Number(v).toFixed(2)), { tier: "slow" })
```

## The machine (server, `crates/preview`)

* `feeds/<name>.{sh,py,js}` or any executable: started by the server, restarted when it exits (5 s back-off) or when edited,
  stopped when deleted. stdout lines become data: `key=value`, `{"key":"k","value":v}` or `{"k1":v1,"k2":v2}`.
  stderr goes to `feeds/.logs/<name>.log`. This is where the agent writes its custom loops; when one breaks, it fixes the script.
* `POST /push` takes the same JSON shapes (any script, `curl -d '{"a.b":1}' :8788/push`).
* Pages follow `/sub?since=<seq>` (long-poll, woken by every write); `GET /data` is a snapshot; `GET /feeds` lists feeds.
* `POST /run?cmd=name&arg=..` runs `commands/<name>` (10 s limit, by name only: no arbitrary shell). From the page: `ui.run("wifi")`.

Raw shell from the browser is deliberately not exposed: the agent already has the computer through the harness, and a
named command is something the journal can show, review and restore.

## What is wired (feeds/)

| feed | source | keys |
|------|--------|------|
| `machine.py` | the computer (Linux/macOS, stdlib) | `cpu.*` `sys.load*` `mem.*` `swap.*` `disk.*` (with a daily history in `data/disk-history.json`) `proc.*` `sys.*` |
| `net.py` | default interface counters | `net.down/up` (MB/s) with 60-sample histories, `net.iface`, `net.ip` |
| `wifi.sh` | nmcli / iw / system_profiler | `wifi.rssi` `wifi.bars` `wifi.ssid` (macOS hides the SSID) |
| `weather.py` | Open-Meteo, lat/lon from `config.json` | `wx.*` `sun.rise/set` `moon.*` |
| `power.py` | your Octopus account (secrets/): tariff, unit rate, standing charge, smart-meter usage | `power.*` (flat or half-hourly tariffs; usage appears ~1 day after the meter's first readings) |
| `carbon.py` | National Grid ESO regional carbon forecast (free) | `carbon.*` (48 half-hours, the 100 g green line, cleanest 2 h) |
| `gcal.py` | Google Calendar through a service account (download) | `cal.today` `cal.week` `cal.next` `cal.left` |
| `lists.py` | `data/groceries.json` (Claude edits it) | `shop.*` |

The page also owns `time.minute` (rewritten on the minute) and `link.server`.

The harness gives Claude a voice on the screen the same way any script writes data:

```sh
curl -s -d '{"claude.say": "Rain from 15:00. Take a jacket.", "claude.at": "07:42"}' http://127.0.0.1:8788/push
```

## Calendar, both ways

Download is `feeds/gcal.py`. Upload is `commands/cal.py` (the harness calls `POST /run?cmd=cal&arg=add&arg=<title>&arg=<YYYY-MM-DD>&arg=<HH:MM>&arg=<min>`):
`list [days]`, `add <title> <date> <HH:MM|all> [min]`, `move <id> <date> <HH:MM>`, `delete <id>`. The feed shows a change within 30 s.
The service account in `secrets/` must be shared on the calendar with "Make changes to events".

## Secrets

`secrets/secrets.json` (Octopus account id and API key, Google calendar id) and `secrets/google-service-account.json` are mode 600 and
git-ignored. Feeds read them with `_lib.secrets()`; nothing else does, and the page never sees them. On NixOS they are not seeded from
the repo: copy `secrets/` (including `font.key`) into the service's root once (`/var/lib/display/secrets`, owner `display`).

## Deploying (NixOS)

`flake.nix` exports `packages.default` (the server, with pages, UI kit and fonts compiled in) and `nixosModules.default`:

```nix
services.display = { enable = true; kiosk = true; };   # nix/module.nix
```

It runs the server as `display` under systemd with `root = /var/lib/display` (feeds/, commands/, data/, config.json and the journal in
state/ are seeded from the repo on first start and then belong to the agent), and optionally a cage + Chromium kiosk on the panel.
The fonts are Git LFS objects: a flake input from GitHub needs `?lfs=1` (or a local checkout with `git lfs pull`). Put `font.key` in
`/var/lib/display/secrets/` to unlock Berkeley Mono at startup. The module and package are parse-checked but have not been built or booted yet.
