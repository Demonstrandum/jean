# Screens and the tab bar

The display has a fixed set of **screens**, shown as tabs along the bottom of the 800×480 panel (the current tab inverted, the time at the
right). The panel takes no input: you ask by voice ("show energy") and the harness switches it. The sample layouts in
`crates/preview/static/index.html` are the gallery of ideas; the real screens are `crates/ui/ts/screens.ts`.

| tab | what it shows | data |
|-----|---------------|------|
| 1 HOME | day dial, weather, next events, shopping, grid carbon, machine, link, Claude's last words | everything |
| 2 DAY | today as a route diagram, busy hours, the next 7 days, sun and moon | `feeds/gcal.py` |
| 3 WEATHER | now, next 24 h, rain chance, wind, pressure, sun and moon | `feeds/weather.py` |
| 4 ENERGY | 24 h carbon forecast against the 100 g line, unit rate, last full day's usage, the week | `feeds/carbon.py`, `feeds/power.py` |
| 5 LISTS | shopping and notes | `data/groceries.json`, `data/notes.json` via `feeds/lists.py` |
| 6 CLAUDE | what it heard, what it said, the running timer | `claude.*` and `timer.*` keys pushed by the harness |
| 7 SYSTEM | cpu, memory, network, disk, processes, host | `feeds/machine.py`, `net.py`, `wifi.sh` |

## Switching

```sh
curl -s -X POST '127.0.0.1:8788/screen?name=energy'                # now; returns home after 2 min
curl -s -X POST '127.0.0.1:8788/screen?name=energy&hold=600000'    # stay 10 min (hold=0: stay)
```

or `ui.go("energy")` from the REPL. Navigation is a data key (`ui.screen`), not a journaled change, so it is immediate, never fills the
journal, and survives a page reload. A non-home screen returns home after `ui.hold` ms (default 2 minutes) so the wall display never stays on
a visitor's request.

## The CLAUDE screen's keys

The harness writes them with `POST /push`:

```sh
curl -s -d '{"claude.heard":"How long do I boil an egg?","claude.say":"About six and a half minutes.","claude.at":"19:41","claude.state":"idle"}' 127.0.0.1:8788/push
curl -s -d '{"timer.total":390,"timer.label":"egg","timer.end":'$((($(date +%s)+390)*1000))'}' 127.0.0.1:8788/push   # a ticking ring
```

`claude.state` is `idle`, `listening`, `thinking` or `speaking`.

## Adding a screen

```ts
ui.defineScreen("garden", () => sheet(place(tile("Soil", feed("garden.moisture", (v) => bignum(v)), {}), "1 / 1 / 2 / 3")), { tab: "GARDEN" });
```

The tab bar draws itself from the registered screens (first one defined is home; `{hidden: true}` keeps one off the bar). A screen's timers stop
when you leave it. Keep the bar to about eight tabs: each cell is `752 / n` px wide.
