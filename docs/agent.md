# For the agent: how to look things up, and how to change the display

The display is a browser page. You drive it from its REPL (`POST /eval` for a change, `POST /query` for a read). Every change is
typed, verified, rolled back if it is wrong, and committed (the journal in `display-state/` is a git repo). Nothing is addressed by
a string: data are typed keys, categories are enums, instants are epoch milliseconds, places on the sheet are zones.

## Look things up (the documentation tool)

The reference is extracted from the kit's own TypeScript (signatures and doc comments), so it cannot go stale.

| call | returns |
|------|---------|
| `GET /docs?q=tile` | the items whose name, comment or signature mention `tile`, in full (name first) |
| `GET /docs/tile` | that one item: `{name, kind, signature, doc, params[{name,type,optional}], returns, members, global, line}` |
| `GET /docs?list=1` | every item as `{name, kind, summary, global}`: browse the library |
| `GET /docs?q=K.` | the typed live-data keys, with what each one is |
| `GET /docs.md` | the whole reference as Markdown (also `docs/reference.md`) |
| `await ui.docs("zone")` | the same, from inside the REPL |

`kind` is `DocKind`: 0 function, 1 value or namespace, 2 interface, 3 type alias, 4 enum, 5 key. Enum members come with their wire
integers (`Sky.Rain = 3`); interfaces list their fields.

For the harness, a tool needs one parameter: `docs(query?: string, name?: string)` → `GET /docs?q=<query>` or `GET /docs/<name>`.

## Change the display

A screen is data: a list of tiles, each `{ id, at: zone(...), title, glyph, tone, body, count, back }`. `body`, `count` and `back`
are a child or a function making one; `bind(keys, render)` makes a live one.

```ts
// change one tile of a screen
ui.screenOf("weather").patchTile("uv", { title: "UV" })
// add one (zones may not overlap; tile ids are unique)
ui.screenOf("home").setTile({ id: "co2", at: zone(Z.F3), title: "CO₂", body: bind(K.air.no2, (v) => bignum(fmt.num(v))) })
// a whole new screen: it gets a tab
ui.defineScreen({ id: "garden", tab: "GARDEN", tiles: [ /* ... */ ] })
```

Each eval is checked before it is kept: it must run, no two tiles may overlap, and every screen it touched must be built without a tile
overflowing its own cell (`ui.fits().crowded`). If not, the change is rolled back and the error says which tile.

`GET /selftest` shows every screen with sample data (`?data=empty` shows how they look before the first feed arrives) and reports how each
fits. Run it after a change that could matter.

The shipped screens are in `crates/ui/seed/`: applied as ordinary journaled changes the first time the display starts. After that the journal
is the truth; `POST /reseed?name=30-weather.ts` applies a shipped screen again (it replaces that screen's definition).

## Data

Live values are typed keys (`K.wx.temp: Key<number>`). Feeds in `feeds/` write them; a screen reads them with `feed` / `bind`. To add a
value: declare the key in `crates/ui/ts/schema.ts` (`ui.key<T>(id, doc, sample)`), add any enum it needs there (`feeds/_schema.py` is generated
from the schema's enums at server start, so a feed writes the same integers), then have a feed write `{"id": value}`. The sample value is what the
self-test shows. Types of incoming values are checked against the sample's type; a mismatch is logged in the page console.

## What the library gives you instead of strings

* `Sky`, `UvLevel`, `AirBand`, `CarbonIndex`, `Pollutant`, `Fuel`, `Presence`, `RateKind`, `MoonPhase`: categories (`label.sky(Sky.Rain)` for words).
* `fmt.*`: numbers, durations, clock times, weekdays, compass points. `calendar.*`: today, upcoming, next, focus. `series.*`: last n, scaling.
* `sig.state(State.Ok)`, `sig.trend(Trend.Fall)`, ...: sigils as typed values. `Z.A1 ... Z.F3`, `zone(a, b)`: places.
* `Tier.Beat`, `Minute`: refresh periods.
