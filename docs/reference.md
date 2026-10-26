# Reference

Generated from the doc comments and signatures in `crates/ui/ts/ui.ts` and `crates/ui/ts/schema.ts` by `cargo run -p ui --bin gen-docs`. Do not edit by hand.

## Functions and components

### `isNode`

```ts
isNode(x: unknown): x is Node
```

### `isPlain`

```ts
isPlain(x: unknown): x is Props
```

### `setProps`

```ts
function setProps(el: Element, props: Props): void
```

### `applyArg`

```ts
function applyArg(el: Element, arg: Arg): void
```

### `h`

```ts
function h<K extends keyof HTMLElementTagNameMap>(tag: K, ...args: Arg[]): HTMLElementTagNameMap[K]
function h(tag: string, ...args: Arg[]): HTMLElement
function h(tag: string, ...args: Arg[]): HTMLElement
```

### `frag`

```ts
frag(...vals: Child[]): DocumentFragment
```

### `def`

```ts
def(name: string, desc: PropertyDescriptor)
```

### `row`

```ts
row(...a: Arg[])
```

### `col`

```ts
col(...a: Arg[])
```

### `grid`

```ts
grid(cols: number, ...a: Arg[])
```

### `box`

```ts
box(...a: Arg[])
```

### `card`

```ts
card(title: string, ...a: Arg[])
```

### `big`

```ts
big(...a: Arg[])
```

### `heading`

```ts
heading(text: Child, level: 1 | 2 | 3 | 4 = 1)
```

### `divider`

```ts
divider()
```

### `badge`

```ts
badge(text: Child)
```

### `stat`

```ts
stat(label: string, value: Child, o: { unit?: string; delta?: Child } = {})
```

### `list`

```ts
list(items: Child[], o: { ordered?: boolean } = {})
```

### `table`

```ts
table(rows: Child[][], o: { head?: Child[] } = {})
```

### `kv`

```ts
kv(obj: Record<string, Child>)
```

### `progress`

```ts
progress(value: number, label?: string)
```

### `bars`

```ts
bars(data: (number | [string, number])[], o: { height?: number; values?: boolean } = {})
```

### `sparkline`

```ts
sparkline(values: number[], o: { w?: number; h?: number } = {})
```

### `agenda`

```ts
agenda(events: { time: string; title: string; note?: string }[])
```

### `weather`

```ts
weather(o: { temp: string; cond: string; icon?: string; hi?: string; lo?: string })
```

### `md`

```ts
md(src: string)
```

Tiny markdown: #/##/### headings, - lists, **bold**, *italic*, `code`.

### `circ`

```ts
circ(cx: number, cy: number, r: number)
```

### `svgIcon`

```ts
svgIcon(inner: string, size: number | string, stroke = 1.5)
```

### `iconBox`

```ts
iconBox(name: string): [number, number, number, number]
```

### `icon`

```ts
icon(name: IconName, o: { size?: number; fill?: boolean; stroke?: number } = {})
```

### `rays`

```ts
rays(cx: number, cy: number, r1: number, r2: number, n = 8)
```

### `hatchDisc`

```ts
hatchDisc(cx: number, cy: number, r: number, from: number, to: number, step = 1.7)
```

Vertical hatching clipped to a disc: the "grey" of a drawing. x runs from..to across a circle at (cx, cy).

### `weatherIcon`

```ts
weatherIcon(kind: string, size = 84)
```

Weather symbols from the icon set: clear, night, partly, cloudy, rain, snow, storm, fog, wind (hairline at large sizes).

### `clamp`

```ts
clamp(lines: number, ...a: Arg[])
```

Cap dynamic text to N lines so unexpected data can't push the scene off-screen.

### `fill`

```ts
fill(...a: Arg[])
```

### `spacer`

```ts
spacer()
```

### `center`

```ts
center(...a: Arg[])
```

### `chips`

```ts
chips(items: Child[])
```

### `topbar`

```ts
topbar(title: Child, right?: Child)
```

### `banner`

```ts
banner(text: Child, o: { icon?: string } = {})
```

### `quote`

```ts
quote(text: Child, by?: Child)
```

### `trend`

```ts
trend(n: number, unit = "%")
```

### `todo`

```ts
todo(items: (string | { text: string; done?: boolean })[])
```

### `escHtml`

```ts
escHtml(s: string)
```

### `gauge`

```ts
gauge(value: number, label?: string, o: { size?: number; text?: string } = {})
```

### `month`

```ts
month(date: Date = new Date(), o: { marks?: number[]; weekStart?: 0 | 1 } = {})
```

### `lineChart`

```ts
lineChart(values: number[], o: { w?: number; h?: number; labels?: string[] } = {})
```

### `image`

```ts
image(src: string, o: { fit?: "cover" | "contain"; w?: number | string; h?: number | string } = {})
```

### `say`

```ts
say(text: Child, o: { icon?: string } = {})
```

Claude speaking: a speech bubble with a round sparkle avatar.

### `minutes`

```ts
minutes(hm: string)
```

### `toMinutes`

```ts
toMinutes(t: number | string): number
```

Minutes after local midnight of an instant (epoch ms): the older components also take an "HH:MM" text.

### `hhmm`

```ts
hhmm(m: number): string
```

Minutes after midnight as "HH:MM".

### `sunArc`

```ts
sunArc(o: { rise: number | string; set: number | string; now?: number | string; w?: number })
```

Sunrise to sunset arc with the sun where it is now. Times are "HH:MM".

### `hourly`

```ts
hourly(items: { t: string; temp: string; icon?: string; pop?: number }[])
```

A strip of hours: time, weather picture, temperature, chance of rain (0..1).

### `habitGrid`

```ts
habitGrid(levels: number[], o: { rows?: number } = {})
```

Contribution-style dot grid; levels 0..3, filled column by column.

### `mmss`

```ts
mmss(s: number)
```

### `timerRing`

```ts
timerRing(remaining: number, total: number, label?: string)
```

A countdown dial: a ring of 60 ticks, the time still to run drawn long and heavy, the elapsed ones thin.

### `tearoff`

```ts
tearoff(date: Date = new Date())
```

A tear-off calendar page.

### `note`

```ts
note(text: Child, o: { tilt?: number } = {})
```

A handwritten sticky note, slightly tilted.

### `presence`

```ts
presence(people: { name: string; home: boolean }[])
```

Who is home: solid circle with an initial when home, dashed when away.

### `moon`

```ts
moon(phase: number, size = 64)
```

Moon phase, 0 = new, 0.25 = first quarter, 0.5 = full, 0.75 = last quarter. The shadow is hatched, the lit side open.

### `tile`

```ts
tile(label: string, body?: Child, o: { count?: Child; inverted?: boolean; bleed?: boolean; tone?: string; glyph?: IconName; waves?: boolean | Parameters<typeof waves>[0]; back?: Node } = {})
```

A launcher-style module: label top-left, something in the middle, a count bottom-left.

### `lines`

```ts
lines(rows: [Child, Child, Child?][])
```

Compact ruled rows [left, text, right?]. The text takes the space and is cut with an ellipsis; rows share the height.

### `checklist`

```ts
checklist(items: [Child, Child?, boolean?][])
```

A shopping-list style checklist: [text, qty?, done?]. Open items are an empty square, done ones a solid one.

### `pane`

```ts
pane(...a: Arg[])
```

A box that is its own measuring frame (container units inside it refer to it, not to the tile).

### `motionName`

```ts
motionName(p: string)
```

### `cursor`

```ts
cursor()
```

A blinking block cursor.

### `spinner`

```ts
spinner(size = 14)
```

A spinner of 12 dots; the bright one steps round once a second.

### `liveTimer`

```ts
liveTimer(seconds: number, o: { label?: string; total?: number; onDone?: () => void } = {})
```

A countdown ring that ticks once a second and stops at zero (then calls onDone).

### `typewriter`

```ts
typewriter(text: string, o: { step?: number; cursor?: boolean } = {})
```

Text that types itself in, `step` words per second. The full text is laid out (hidden) first, so the size never changes.

### `countUp`

```ts
countUp(to: number, o: { from?: number; ticks?: number; dec?: number; unit?: string } = {})
```

A big numeral that counts up to `to` in `ticks` one-second steps (eased). Sized for the final text, so it never reflows.

### `stream`

```ts
stream(series: number[][], next: () => number[], draw: (s: number[][]) => HTMLElement, ms = 2000)
```

A graph that scrolls: each tick `next()` gives one new value per series, the oldest drops off, `draw` rebuilds the graph.

### `levelTile`

```ts
levelTile(label: string, level: number, text?: Child)
```

A tile that fills like a tank (level 0..1) with a wavy surface.

### `usageFill`

```ts
usageFill(values: readonly number[], cap: number): HTMLElement
```

The silhouette of a usage history, held between samples (capacity = full height): a `back` for a tile with tone "usage".

### `chip`

```ts
chip(text: Child): HTMLElement
```

A number written over a fill (inverted against what is behind it): the `count` of a usage tile.

### `usageTile`

```ts
usageTile(label: string, values: number[], cap: number, text: Child, o: { glyph?: string } = {})
```

A minimal box filled from the bottom up to a history curve (held between samples): the capacity is the
full height, so the silhouette IS the usage over time. A number sits inside. Pair it with a chart.

### `dayDial`

```ts
dayDial(o: { rise: number | string; set: number | string; now?: number | string; size?: number; sub?: string; callouts?: boolean; side?: number })
```

A 24-hour donut: noon at the top, midnight at the bottom. The night stretch is solid, the
day stretch open, and a marker sits where "now" is. Times are "HH:MM".

### `ruler`

```ts
ruler(o: { min: number; max: number; value: number; target?: number; step?: number; w?: number })
```

A scale with ticks, a marker at the current value and an optional target tick.

### `statStrip`

```ts
statStrip(cells: { label: string; value: Child; sub?: Child }[])
```

Hairline-divided readouts: a tiny label, a big value, a note underneath.

### `pills`

```ts
pills(items: (string | { label: string; icon?: string })[], o: { active?: number } = {})
```

Scene chips; the active one is inverted. Items are strings or {label, icon}.

### `slider`

```ts
slider(value: number, label: Child, o: { text?: Child } = {})
```

A level bar with its label and value written on it (text flips colour over the fill).

### `menu`

```ts
menu(rows: { label: Child; value?: Child; dot?: "ok" | "warn" | "off" }[])
```

Settings-style rows: label with a status dot (ok / warn / off), then a value or an arrow.

### `device`

```ts
device(o: { name: Child; sub?: Child; on?: boolean; icon?: string })
```

A device row: round icon, name and state, and a switch (display only).

### `hero`

```ts
hero(value: Child, o: { unit?: string; label?: Child; status?: Child; sub?: Child; size?: number } = {})
```

One huge number with a small unit and a status line.

### `polar`

```ts
polar(values: number[], o: { max?: number; rings?: number; labels?: [string, number][]; marks?: boolean } = {})
```

A polar plot: rings, spokes, the values as a hatched polygon. Good for anything cyclic (hours, compass).

### `ridgeline`

```ts
ridgeline(rows: number[][], o: { w?: number; h?: number; labels?: string[]; hours?: [string, number][] } = {})
```

Rows stacked as ridgelines, back to front, each hiding the ones behind it: a surface made of days.

### `word`

```ts
word(text: string, caption?: Child)
```

A spaced-caps word with a caption: the quick-glance status screen (e.g. FLX, NUC).

### `waves`

```ts
waves(o: { from?: number; seed?: number; scale?: number; tone?: "sky" | "water" } = {})
```

One continuous gradient whose flow is twisted by a domain-warped sine field, so it swirls like marbling rather than
bumping. Drawn once to a small grey canvas and used as an image (so it survives in the DOM). At 1-bit the dither
turns it into grain. Put one behind a tile's content with tile {waves}.

### `dotMirror`

```ts
dotMirror(up: number[], down: number[], o: { rows?: number } = {})
```

Two dot graphs mirrored on a centre line: `up` above, `down` below (traffic in/out). Values 0..1.

### `sparkRows`

```ts
sparkRows(rows: { label: string; values: number[]; text: Child }[], o: { rows?: number } = {})
```

Rows of  LABEL  [small dot graph]  value, one graph per row (cores, disks, ...).

### `sigPaths`

```ts
sigPaths(kind: SigKind, v: any): string
```

The marks are drawn in a 12x12 box, 1px line. What they say:
state  "ok" solid square | "idle" open square | "warn" half-filled square | "err" crossed square | anything else: dashed square (unknown)
level  [value, max] (max 8 by default): a square tank filled from the bottom (empty..full)
trend  "up" | "down" | "flat" | "rise" | "fall"  (slow and steep arrows)
age    minutes since the data was fresh: the hand of a one-hand clock in a square (60 or more: dashed, stale)
count  1..6 as dice faces, more as a plus
dir    degrees, 0 = up: a rotated arrow (wind, bearing)
bars   0..4 signal bars
glyph  any icon name, scaled down

### `sigil`

```ts
sigil(kindOrMark: SigKind | SigMark, v?: any, size = 12)
```

One sigil as an inline element (use it next to text, in a table cell, in a module label).

### `sigils`

```ts
sigils(items: [SigKind, any?][], size = 12)
```

### `statusStrip`

```ts
statusStrip(left: Child[], right: Child[] = [])
```

A ruled bar of labelled segments (the first is a solid mark), then a gap, then right-aligned segments.

### `segbar`

```ts
segbar(value: number, max = 1)
```

A bar: the filled part is solid, the rest a halftone. value of max.

### `readouts`

```ts
readouts(rows: { label: string; value: number; max?: number; unit?: string; text?: Child }[])
```

Rows of  LABEL  [segmented bar]  value unit. Dense and exact.

### `kvgrid`

```ts
kvgrid(obj: Record<string, Child>, cols = 2)
```

Key/value pairs in a grid of columns (default 2), each pair a ruled row.

### `skybox`

```ts
skybox(name: string, text: Child, size = 84)
```

A small framed square: a weather icon over a dithered sky, with a value written in the dark part.

### `header`

```ts
header(o: { kicker: string[]; code: string; right?: Child[] })
```

The sheet header: small stacked kicker lines, a large code, right-aligned lines.

### `ruled`

```ts
ruled(...cells: { title?: string; rows: [string, Child][] }[])
```

A ruled table of cells; each cell is {title, rows: [label, value][]}. Like a drawing's header block.

### `toneStrip`

```ts
toneStrip(steps = 16)
```

A calibration strip of grey steps (the panel dithers them), the way a print sheet carries one.

### `routes`

```ts
routes(o: { from: number | string; to: number | string; now?: number | string; w?: number; lanes: { label: string; look?: "solid" | "dash" | "dot" | "double"; stops: Stop[] }[]; links?: [number, number | string, number][] })
```

A route diagram of the day: lanes of stops/spans along a time axis. `from`, `to`, `now` and each stop's `at`/`to` are instants (epoch ms), or "HH:MM" text.

### `panel`

```ts
panel(tone: Tone, ...a: Arg[])
```

A big region: plain, inverted (white on black), a dithered sky gradient, or a dot field.

### `meta`

```ts
meta(left: Child, right?: Child)
```

Tiny all-caps line across the top: left text, right text.

### `claudeLine`

```ts
claudeLine(text: Child, time?: Child)
```

Claude's line of the moment: avatar, one sentence, a timestamp.

### `tilegrid`

```ts
tilegrid(...a: Arg[])
```

A 6-column launcher grid; place children with style gridArea ("row / col / row-end / col-end").

### `HATCH`

```ts
HATCH(id: string)
```

### `dotMatrixSvg`

```ts
dotMatrixSvg(text: string, pitch: number)
```

### `dotClock`

```ts
dotClock(o: { time?: string; pitch?: number } = {})
```

A dot-matrix clock. Pass `time` ("22:26") for a fixed one, otherwise it follows the time.

### `dayStrip`

```ts
dayStrip(o: { rise: number | string; set: number | string; now?: number | string; w?: number })
```

The 24 hours as a ruler: night hatched, day open, a marker where "now" is. Times "HH:MM".

### `timeline`

```ts
timeline(o: { w?: number; from: string; to: string; now?: string; lanes: { label: string; blocks: Block[] }[] })
```

Lanes of blocks over a time axis (a Gantt strip), with a "now" line. Times "HH:MM".

### `areaChart`

```ts
areaChart(values: number[], o: { w?: number; h?: number; labels?: [string, number][]; now?: number; window?: [number, number]; marks?: boolean; fmt?: (i: number, v: number) => string; floor?: number; ceil?: number; limit?: [number, string] } = {})
```

Hourly values as a stepped area (hatched), an optional solid highlight window, and a "now" marker.

### `limitBars`

```ts
limitBars(values: number[], o: { limit: number; w?: number; h?: number; now?: number; window?: [number, number]; labels?: [string, number][]; limitLabel?: string; windowLabel?: string })
```

Bars against a limit line: values at or under `limit` are solid, over it hatched (carbon intensity: green under 100 g).
`now` outlines one bar and flags it; `window` brackets a stretch (the cleanest hours); `labels` are [text, bar index].

### `bignum`

```ts
bignum(text: Child, o: { unit?: string; chars?: number } = {})
```

A numeral sized to fill its module (container query units). Keep tiles inside the tilegrid.

### `place`

```ts
place<T extends HTMLElement>(node: T, area: Zone | string): T
```

Put a module in a zone of the sheet: place(node, zone(Z.A3, Z.B3)). The tile's corner shows the zone's top-left address.
(A CSS grid-area string is still accepted, for the gallery's old samples; use zones.)

### `dayCurve`

```ts
dayCurve(values: number[], o: { now?: number; w?: number; h?: number; max?: number; padT?: number; padB?: number; baseline?: boolean; guides?: [number, string][]; labels?: [string, number][] } = {})
```

A value over the day as a smooth curve (UV index, sun height), hatched underneath, with a line and a dot where "now" is.
`values` are the 24 hours from 00:00; `now` is the hour as a fraction (20.5 = 20:30); `guides` are dotted [value, label] lines
(UV 3 = moderate, 6 = high); the scale runs to at least `max` so a weak winter curve reads as weak, not as full height.

### `dotBars`

```ts
dotBars(values: number[], o: { rows?: number; labels?: string[] } = {})
```

LED-style histogram: columns of dots lit from the bottom. Values 0..1.

### `waffle`

```ts
waffle(value: number, total: number, o: { cols?: number; hatch?: number } = {})
```

A quantity as a grid of cells: `value` of `total` filled, a fraction shows as a partial cell.

### `stackBar`

```ts
stackBar(parts: { label: string; value: number; look?: "solid" | "hatch" | "outline" }[])
```

One bar split into labelled parts (solid / hatched / outlined), widths in proportion.

### `arcGauge`

```ts
arcGauge(value: number, o: { min?: number; max?: number } = {})
```

A half-circle dial with ticks and the value in the middle.

### `band`

```ts
band(edge: "top" | "bottom" | "mid", ...a: Arg[])
```

A full-bleed black band, edge to edge. "top"/"bottom" also reach the screen edge. White on black inside.

### `giant`

```ts
giant(value: Child, o: { size?: number; unit?: string } = {})
```

An oversized numeral (default 240px). Put it in a tile with {bleed: true}: it sits on the module's
floor and the module's edge crops it, on purpose. Keep the real value in the label or count.

### `sig.state`

```ts
sig.state(s: State): SigMark
```

A solid, open, half-filled or crossed square.

### `sig.level`

```ts
sig.level(value: number, max = 8): SigMark
```

A square tank filled from the bottom: value of max.

### `sig.trend`

```ts
sig.trend(t: Trend): SigMark
```

### `sig.age`

```ts
sig.age(minutes: number): SigMark
```

A one-hand clock in a square: minutes since the data was fresh (60 or more draws dashed, stale).

### `sig.count`

```ts
sig.count(n: number): SigMark
```

Dice faces 1..6, a plus beyond.

### `sig.dir`

```ts
sig.dir(degrees: number): SigMark
```

A rotated arrow: degrees, 0 = up.

### `sig.bars`

```ts
sig.bars(n: number): SigMark
```

Signal bars, 0..4.

### `sig.glyph`

```ts
sig.glyph(name: IconName): SigMark
```

### `zone`

```ts
zone(a: Cell, b: Cell = a): Zone
```

The block of cells between two corners, in any order: zone(Z.A3, Z.B3). One argument is a single cell.

### `zoneArea`

```ts
zoneArea(z: Zone): string
```

### `zoneName`

```ts
zoneName(z: Zone): string
```

The address printed in a tile's corner: its top-left cell, e.g. "B2".

### `zonesOverlap`

```ts
zonesOverlap(a: Zone, b: Zone): boolean
```

### `toneClass`

```ts
toneClass(t: TileSpec["tone"]): string | undefined
```

### `resolveBody`

```ts
resolveBody<T extends Child>(b: Body<T> | undefined): T | undefined
```

### `tileFrom`

```ts
tileFrom(s: TileSpec): HTMLElement
```

Make one tile from its spec.

### `checkTiles`

```ts
function checkTiles(id: string, tiles: readonly TileSpec[]): void
```

### `renderScreen`

```ts
function renderScreen(id: string): void
```

Build a screen's DOM into the scene (its timers replace the previous screen's).

### `defineScreen`

```ts
function defineScreen(spec: ScreenSpec): ScreenHandle
```

Register a screen, or replace the one with this id. The first screen defined is home. A screen defined while nothing is showing
is shown; redefining the one on show rebuilds it. Overlapping zones or repeated tile ids throw (so the change is rolled back).

### `screenHandle`

```ts
function screenHandle(id: string): ScreenHandle
```

### `go`

```ts
function go(id: string, o: { hold?: number } = {}): boolean
```

Show a screen now. A screen other than home returns home after `hold` ms (default: the key `K.ui.hold`, else 2 minutes; 0 stays).

### `drawFrame`

```ts
function drawFrame(): void
```

Draw the drawing-sheet frame: corner targets, sigil clusters in the top corners, zone letters over the grid columns and numbers beside its rows, a terse title block.

### `frame`

```ts
function frame(opt: boolean | { title?: string; sheet?: string; tl?: SigList; tr?: SigList; tabs?: boolean } = true): void
```

The drawing-sheet frame, in the screen margin: registration targets, two corner clusters of sigils (tl, tr),
column letters (A-F) and row numbers (1-3) matching the tilegrid so every module has an address like "B2",
and a terse title block. ui.frame({title, sheet, tl, tr}) turns it on, ui.frame(false) off.

### `live`

```ts
function live<T extends Element>(el: T, fn: (el: T) => void, ms: number): T
```

Run fn now and every `ms` while `el` stays mounted (stops once removed).

### `setText`

```ts
setText(el: Element, t: string)
```

### `clock`

```ts
clock(o: { seconds?: boolean; date?: boolean; hour12?: boolean } = {})
```

### `countdown`

```ts
countdown(to: Date | string | number, label?: string)
```

### `fontFormat`

```ts
fontFormat(file: string)
```

### `fontFaceCss`

```ts
fontFaceCss()
```

### `get`

```ts
get(name: string)
```

### `mount`

```ts
function mount<T extends Node>(node: T, parent: Element = root): T
```

### `describe`

```ts
describe(c: Element)
```

### `adapt`

```ts
function adapt(): FitReport
```

### `docs`

```ts
docs(query?: string): Promise<DocItem[]>
```

Look the library up: docs("tile") finds items whose name or comment mentions it; docs() lists everything. Same data as GET /docs.

### `fits`

```ts
fits(): FitReport
```

The fit report for the scene as it is now (re-fits it).

### `begin`

```ts
function begin(): void
```

### `periodOf`

```ts
periodOf(r: Period): number
```

### `slotOf`

```ts
slotOf(r: Period, now = Date.now()): number
```

### `stop`

```ts
function stop(which: string | number): void
```

### `every`

```ts
function every(name: string, period: Period, fn: () => void): number
```

A named job on the beat: ui.every("clock", Tier.Beat, fn), or with a period in ms. Calling again with the same name replaces it.
It first runs at the next boundary of its period, together with everything else due then.

### `timers`

```ts
timers()
```

### `defineKey`

```ts
function defineKey<T>(id: string, doc: string, sample: T): Key<T>
```

Declare a key (the catalogue is in the schema, as K.*). Values are JSON on the wire: numbers, booleans, strings, arrays, objects, null.

### `setById`

```ts
function setById(id: string, v: unknown, ts: number = Date.now()): void
```

Write a value by its wire name (the server's updates arrive this way). Use `setData(key, value)` in your own code.

### `setData`

```ts
function setData<T>(key: Key<T>, v: T, ts?: number): void
```

Write a value. This never touches the DOM: bindings that show the key go stale and redraw at their next beat.

### `getData`

```ts
getData<T>(key: Key<T>): T | undefined
```

The latest value of a key, or undefined if it has never been written.

### `dataAge`

```ts
dataAge(key: Key<unknown>): number
```

Minutes since a key was last written (Infinity if never): feed sig.age(...) with it.

### `feed`

```ts
function feed<T>(key: Key<T>, render: (v: T | undefined, meta: FeedMeta) => Child, o?: FeedOptions): HTMLElement
function feed<Ks extends readonly Key<any>[]>(keys: Ks, render: (v: Values<Ks>, meta: FeedMeta) => Child, o?: FeedOptions): HTMLElement
function feed(keys: Key<any> | readonly Key<any>[], render: (v: any, meta: FeedMeta) => Child, o: FeedOptions = {}): HTMLElement
```

A live binding: feed(K.wifi.bars, (n) => bars(n)) draws now and redraws only at its tier's next beat after the value changes.
With a list of keys the render gets a tuple of their values. The first value for an empty binding is drawn at the next second.

### `bind`

```ts
function bind<T>(key: Key<T>, render: (v: T | undefined, meta: FeedMeta) => Child, o?: FeedOptions): () => HTMLElement
function bind<Ks extends readonly Key<any>[]>(keys: Ks, render: (v: Values<Ks>, meta: FeedMeta) => Child, o?: FeedOptions): () => HTMLElement
function bind(keys: any, render: any, o?: FeedOptions): () => HTMLElement
```

The same as feed, as a function that makes the binding when called: a tile's body, count or back is built fresh with each screen build.

### `applyFeeds`

```ts
function applyFeeds(now: number): number
```

### `beatStep`

```ts
function beatStep(): void
```

The master clock's step: everything due this second runs in this one synchronous batch.

### `beatLoop`

```ts
beatLoop(): void
```

### `setCadence`

```ts
function setCadence(c: Partial<Record<Tier, number>> = {}): Record<Tier, number>
```

Change the periods (ms), e.g. ui.cadence({ [Tier.Beat]: 60000 }). Jobs re-base so nothing fires in a burst.

### `connect`

```ts
async function connect(base = ""): Promise<void>
```

Follow the server's data feed: long-polls /sub, writing every update into the store (and K.link.server).

### `push`

```ts
push<T>(key: Key<T>, value: T)
```

Publish a value to the server's store (and so to every page and script).

### `run`

```ts
run(cmd: string, ...args: string[]): Promise<{ ok: boolean; code: number | null; stdout: string; stderr: string; error?: string }>
```

Run a named command from the server's commands/ directory (never arbitrary shell): resolves {ok, code, stdout, stderr}.

### `flush`

```ts
flush()
```

### `undo`

```ts
function undo(r: MutationRecord): void
```

### `rollback`

```ts
function rollback(): void
```

### `verify`

```ts
function verify(): FitReport
```

Commit the changes since the last call as the new known-good state, and make the scene fit.

### `trialScreen`

```ts
function trialScreen(id: string): FitReport
```

Build a screen that is not on show, measure it, and put the previous one back.

### `selfTest`

```ts
function selfTest(empty = false): ScreenReport[]
```

Show every screen with every key set to its sample value (or, with `empty`, with no data at all: how a screen looks before the first
feed arrives) and report how each fits (ok = nothing crowded, dropped or clipped). Your real data and the current screen are put back.
Call it inside a query or tx so the DOM is restored too.

### `tx`

```ts
function tx<T>(fn: () => T): T
```

Run fn, then verify(). If fn throws, its changes are rolled back too.

### `outline`

```ts
function outline(el: Element = root, depth = 0): string
```

### `dither`

```ts
function dither(img: ImageData, algo: DitherAlgo = "atkinson"): ImageData
```

Quantise to pure black/white, in place.

### `faceCss`

```ts
faceCss()
```

### `rasterize`

```ts
async function rasterize(el: HTMLElement): Promise<ImageData>
```

### `paintEink`

```ts
async function paintEink(): Promise<void>
```

### `eink`

```ts
function eink(opt: boolean | { algo?: DitherAlgo } = true): void
```

ui.eink() atkinson | ui.eink({algo:"floyd"|"threshold"}) | ui.eink(false) off

### `ui.rev`

```ts
ui.rev()
```

### `ui.snapshot`

```ts
ui.snapshot()
```

The scene as HTML: what the store keeps as its snapshot cache.

### `ui.show`

```ts
ui.show(...nodes: Child[])
```

Replace the whole scene. Returns the fit report: a scene that doesn't fit must not be shown.

### `ui.clear`

```ts
ui.clear()
```

### `ui.settled`

```ts
ui.settled()
```

### `ui.ready`

```ts
ui.ready()
```

Resolves when the fonts are loaded. Await it before the first scene: layout depends on them.

### `ui.accent`

```ts
ui.accent(color: string | null)
```

One accent colour for colour e-ink panels (e.g. red/yellow). Without it everything stays black.

### `ui.icons`

```ts
ui.icons()
```

Names of the built-in icons.

### `ui.beats`

```ts
ui.beats()
```

The master clock: periods, held state, what is scheduled, how many batches ran.

### `ui.hold`

```ts
ui.hold()
```

Freeze every beat (night mode): data keeps arriving, nothing redraws until resume().

### `ui.resume`

```ts
ui.resume()
```

### `ui.data`

```ts
ui.data()
```

### `ui.link`

```ts
ui.link(): Link
```

### `ui.screenOf`

```ts
ui.screenOf(id: string): ScreenHandle | undefined
```

### `ui.screenIds`

```ts
ui.screenIds()
```

### `ui.current`

```ts
ui.current()
```

### `ui.size`

```ts
ui.size(w: number, ht: number)
```

### `ui.css`

```ts
ui.css(text: string)
```

### `ui.help`

```ts
ui.help()
```

### `wave`

```ts
wave(n: number, base: number, amp: number, seed = 0)
```

### `uvLevel`

```ts
uvLevel(uv: number): UvLevel
```

The UV band for an index value.

### `airBand`

```ts
airBand(eaqi: number): AirBand
```

The European air-quality band for an index value.

### `moonPhaseOf`

```ts
moonPhaseOf(fraction: number): MoonPhase
```

The named moon phase for a lunar fraction (0 new, 0.5 full).

### `isGreen`

```ts
isGreen(grams: number, line = 100): boolean
```

Whether a carbon intensity counts as green, against a line (default 100 gCO₂/kWh).

### `label.sky`

```ts
label.sky(s: Sky): string
```

### `label.uv`

```ts
label.uv(l: UvLevel): string
```

### `label.air`

```ts
label.air(b: AirBand): string
```

### `label.carbon`

```ts
label.carbon(c: CarbonIndex): string
```

### `label.pollutant`

```ts
label.pollutant(p: Pollutant): string
```

### `label.fuel`

```ts
label.fuel(f: Fuel): string
```

### `label.presence`

```ts
label.presence(p: Presence): string
```

### `label.moon`

```ts
label.moon(m: MoonPhase): string
```

### `skyIcon`

```ts
skyIcon(s: Sky): "clear" | "partly" | "cloudy" | "rain" | "snow" | "storm" | "fog"
```

The weather symbol the kit draws for a sky: pass it to weatherIcon().

### `stateOf.carbon`

```ts
stateOf.carbon(c: CarbonIndex)
```

### `stateOf.air`

```ts
stateOf.air(b: AirBand)
```

### `stateOf.presence`

```ts
stateOf.presence(p: Presence)
```

### `two`

```ts
two(n: number)
```

### `fmt.num`

```ts
fmt.num(v: number | null | undefined, digits = 0): string
```

A number with fixed decimals, or a dash for a missing value.

### `fmt.duration`

```ts
fmt.duration(minutes: number): string
```

Minutes as "45m" or "1h30".

### `fmt.uptime`

```ts
fmt.uptime(seconds: number | null | undefined): string
```

Seconds as "13d 4h" or "5h 12m".

### `fmt.clock`

```ts
fmt.clock(at: Ms | null | undefined): string
```

An instant as the local 24-hour clock, "09:30".

### `fmt.weekday`

```ts
fmt.weekday(at: Ms): string
```

An instant's local weekday, "Tue".

### `fmt.date`

```ts
fmt.date(at: Ms): string
```

An instant's local date, "Tue 27 Oct".

### `fmt.compass`

```ts
fmt.compass(degrees: number): string
```

Degrees as a compass point, "SW".

### `fmt.percent`

```ts
fmt.percent(v: number | null | undefined): string
```

A percentage rounded, with its sign.

### `fmt.hour`

```ts
fmt.hour(at: Ms): string
```

The hour of an instant as two digits, "07".

### `series.last`

```ts
series.last(values: readonly number[] | null | undefined, n: number): number[]
```

The last n values, padded with zeros on the left.

### `series.sqrtScale`

```ts
series.sqrtScale(values: readonly number[], top: number): number[]
```

Scale to 0..1 against `top`, square-rooted so small values stay visible.

### `dayStart`

```ts
dayStart(at: Ms): Ms
```

The start of the local day containing `at`.

### `calendar.on`

```ts
calendar.on(events: readonly CalEvent[], at: Ms): CalEvent[]
```

Events that start on the local day of `at`.

### `calendar.over`

```ts
calendar.over(e: CalEvent, at: Ms): boolean
```

Whether an event is over at `at`.

### `calendar.when`

```ts
calendar.when(e: CalEvent, at: Ms): string
```

An event's label in a list: "09:30" today, "Tue 09:30" on another day, "all day" for an all-day event.

### `calendar.upcoming`

```ts
calendar.upcoming(events: readonly CalEvent[], at: Ms, n: number): CalEvent[]
```

The next `n` events that are not over, from `at`.

### `calendar.next`

```ts
calendar.next(events: readonly CalEvent[], at: Ms): { event: CalEvent; inMinutes: number } | undefined
```

The first timed event that has not started, and minutes until it.

### `calendar.focus`

```ts
calendar.focus(events: readonly CalEvent[], at: Ms): { day: Ms; isToday: boolean; events: CalEvent[] }
```

The day to draw as "the day": today while something timed is still to come, else the next day with something.

### `shell`

```ts
shell(): void
```

Set up the frame: registration marks, live sigils in the corners (left = the machine, right = the world) and the tab bar.

## Values and namespaces

### `prims`

```ts
prims: Record<string, (...a: Arg[]) => HTMLElement>
```

### `ICON_PATHS`

```ts
ICON_PATHS
```

### `iconBoxes`

```ts
iconBoxes: Record<string, [number, number, number, number]>
```

icon("sun") ... a named icon (size defaults to the surrounding font size). Unknown names render as text, so any glyph works.

### `GLYPHS`

```ts
GLYPHS: Record<string, string[]>
```

### `STATE_WIRE`

```ts
STATE_WIRE: Record<State, string>
```

### `TREND_WIRE`

```ts
TREND_WIRE: Record<Trend, string>
```

### `sig`

```ts
sig
```

Typed constructors for sigils: sig.state(State.Ok), sig.level(7, 8), sig.trend(Trend.Fall), sig.age(minutes), sig.count(n), sig.dir(degrees), sig.bars(0..4), sig.glyph(icon).

### `Z`

```ts
Z
```

Every cell as a constant: Z.A1 ... Z.F3.

### `root`

```ts
root: HTMLElement
```

The scene: everything built goes in here, and it is always made to fit the screen (see adapt()).

### `fontsReady`

```ts
fontsReady: Promise<void>
```

Resolves once the fonts are loaded. Layout depends on them, so measure (verify) only after this.

### `regSnap`

```ts
regSnap: { defs: Map<string, ScreenState>; current: string | null; home: string | null } | null
```

The orchestrator calls this right before evaluating a change (tx() does it for you).

### `cadence`

```ts
cadence: Record<Tier, number>
```

### `Minute`

```ts
Minute
```

One minute in ms: a period that redraws exactly when the minute changes.

### `KERNELS`

```ts
KERNELS: Record<"atkinson" | "floyd", [number, number, number][]>
```

### `inlineFaces`

```ts
inlineFaces: Promise<string> | null
```

Rasterise an element via an SVG foreignObject.

### `ui`

```ts
ui
```

### `ui.set`

```ts
set: setData
```

The data store: ui.set(key, v) (no redraw until the next beat), ui.get(key), ui.age(key) minutes, ui.data() everything.

### `ui.key`

```ts
key: defineKey
```

Declare a typed key (the catalogue is K.*).

### `label`

```ts
label
```

Words for the enums.

### `stateOf`

```ts
stateOf
```

Sigil states for the bands: ok for good ones.

### `fmt`

```ts
fmt
```

Formatting: numbers, durations and instants as text. Every function takes typed values, never text.

### `series`

```ts
series
```

Helpers for number series.

### `calendar`

```ts
calendar
```

Calendar queries over `K.cal.events`.

## Types, interfaces and enums

### `Child`

```ts
type Child = Node | string | number | false | null | undefined | Child[]
```

### `Props`

```ts
type Props = { [k: string]: any }
```

### `Arg`

```ts
type Arg = Child | Props
```

### `Replacement`

```ts
type Replacement = Child
```

### `IconName`

```ts
type IconName = keyof typeof ICON_PATHS
```

The built-in icon names.

### `SigKind`

```ts
type SigKind = "state" | "level" | "trend" | "age" | "count" | "dir" | "bars" | "glyph"
```

### `Stop`

```ts
type Stop = { at: number | string; to?: number | string; name: string }
```

### `Tone`

```ts
type Tone = "plain" | "inv" | "sky" | "dots"
```

### `Block`

```ts
type Block = { from: number | string; to: number | string; label?: string; look?: "solid" | "hatch" | "outline" }
```

### `SigList`

```ts
type SigList = readonly (SigMark | [SigKind, any?])[] | (() => readonly (SigMark | [SigKind, any?])[])
```

Sigils for a corner cluster: sig.* values (or the legacy [kind, value] pairs), or a function returning them (re-read at every redraw).

### `State`

```ts
enum State
```

A sigil's meaning for a state mark.

- `Ok = 0`
- `Idle = 1`
- `Warn = 2`
- `Err = 3`
- `Unknown = 4`

### `Trend`

```ts
enum Trend
```

A sigil's meaning for a trend mark: slow arrows (Up/Down/Flat) and steep ones (Rise/Fall).

- `Up = 0`
- `Down = 1`
- `Flat = 2`
- `Rise = 3`
- `Fall = 4`

### `SigMark`

```ts
interface SigMark
```

One sigil as data (what to draw), made by the `sig` constructors and drawn by `sigil(mark, size)` or by the frame's corner clusters.

- `readonly kind: SigKind;`
- `readonly v: unknown`

### `Col`

```ts
enum Col
```

The sheet's columns, left to right.

- `A = 0`
- `B = 1`
- `C = 2`
- `D = 3`
- `E = 4`
- `F = 5`

### `Row`

```ts
type Row = 1 | 2 | 3
```

The sheet's rows, top to bottom.

### `Cell`

```ts
interface Cell
```

One cell of the sheet.

- `readonly col: Col;`
- `readonly row: Row`

### `Zone`

```ts
interface Zone
```

A rectangle of cells, both corners inclusive.

- `readonly from: Cell;`
- `readonly to: Cell`

### `CellName`

```ts
type CellName = `${keyof typeof Col}${Row}`
```

The names Z.A1 ... Z.F3 (a type for the typed constants below).

### `Body`

```ts
type Body<T extends Child = Child> = T | (() => T)
```

What a tile region holds: any child, or a function making it when the screen is built.

### `TileTone`

```ts
type TileTone = "fill" | "list" | "legend" | "iconic" | "usage"
```

How a tile is dressed: fill = its body stretches, list = ruled rows, legend = title on the border, iconic = centred icon, usage = filled to a history curve.

### `TileSpec`

```ts
interface TileSpec
```

- `readonly id: string;`
- `readonly at: Zone;`
- `readonly title: string;`
- `readonly glyph?: IconName;`
- `readonly tone?: TileTone | readonly TileTone[];`
- `readonly inverted?: boolean;`
- `readonly body?: Body;`
- `readonly count?: Body;`
- `readonly back?: Body<Node>;`
- `readonly waves?: boolean | Parameters<typeof waves>[0];`
- `readonly frameless?: boolean;`

### `ScreenSpec`

```ts
interface ScreenSpec
```

- `readonly id: string;`
- `readonly tab?: string;`
- `readonly tiles: readonly TileSpec[];`

### `ScreenHandle`

```ts
interface ScreenHandle
```

The handle `defineScreen` returns: read and change a screen's tiles one at a time.

- `readonly id: string;`
- `tiles(): readonly TileSpec[];`
- `tile(id: string): TileSpec | undefined;`
- `setTile(spec: TileSpec): void;`
- `patchTile(id: string, patch: Partial<Omit<TileSpec, "id">>): void;`
- `removeTile(id: string): void;`

### `ScreenState`

```ts
interface ScreenState
```

- `id: string;`
- `tab: string;`
- `tiles: readonly TileSpec[]`

### `FitReport`

```ts
type FitReport = { /** Always true: the screen is guaranteed to contain the scene. */ fits: true; /** 1 = natural size. Less means the scene was too big and was shrunk. */ scale: number; /** What was hidden because even the smallest readable size did not fit. */ dropped: string[]; /** True only if nothing was left to hide and the edge had to clip. */ clipped: boolean; /** Spare vertical pixels after fitting. */ freeY: number; /** How far over the scene was at natural size. Positive numbers mean "trim me". */ natural: { overflowX: number; overflowY: number }; /** Modules whose content is bigger than their own cell (it is clipped inside the module). */ crowded: string[]; }
```

### `DocKind`

```ts
enum DocKind
```

What a documented item is.

- `Function = 0`
- `Value = 1`
- `Interface = 2`
- `Type = 3`
- `Enum = 4`
- `Key = 5`

### `DocParam`

```ts
interface DocParam
```

One parameter of a documented function.

- `readonly name: string;`
- `readonly type: string;`
- `readonly optional: boolean`

### `DocItem`

```ts
interface DocItem
```

An entry of the generated reference: its signature and doc comment, extracted from the kit's own source.

- `readonly name: string;`
- `readonly kind: DocKind;`
- `readonly signature: string;`
- `readonly doc: string;`
- `readonly params: readonly DocParam[];`
- `readonly returns: string;`
- `readonly members: readonly string[];`
- `readonly global: boolean;`
- `readonly line: number;`

### `Tier`

```ts
enum Tier
```

How often a live binding or job may repaint: Tick = 1 s (only for what must count), Beat = 30 s (live data), Slow = 5 min.

- `Tick = "tick"`
- `Beat = "beat"`
- `Slow = "slow"`

### `Period`

```ts
type Period = Tier | number
```

A period: a tier, or milliseconds (minimum 1 s, aligned to the clock).

### `Job`

```ts
interface Job
```

- `name?: string;`
- `every: Period;`
- `fn: () => void;`
- `slot: number`

### `Key`

```ts
interface Key
```

A typed handle on one live value. `id` is the wire name feeds write; T is what the value is; `sample` is a fixture the self-test shows.

- `readonly id: string;`
- `readonly doc: string;`
- `readonly sample: T`

### `Link`

```ts
enum Link
```

Whether the link to the server is up.

- `Connecting = 0`
- `Up = 1`
- `Down = 2`

### `Datum`

```ts
interface Datum
```

- `v: unknown;`
- `ts: number`

### `Watcher`

```ts
interface Watcher
```

- `keys: string[];`
- `el: Element;`
- `draw: () => void;`
- `tier: Period;`
- `slot: number;`
- `stale: boolean;`
- `always: boolean;`
- `seen: boolean;`
- `empty: boolean`

### `FeedMeta`

```ts
interface FeedMeta
```

What a binding's render also receives.

- `readonly age: number`

### `FeedOptions`

```ts
interface FeedOptions
```

- `readonly tier?: Period;`
- `readonly fallback?: Child;`
- `readonly always?: boolean`

### `Values`

```ts
type Values<Ks extends readonly Key<any>[]> = { [I in keyof Ks]: Ks[I] extends Key<infer T> ? T | undefined : never }
```

The values a render gets for a list of keys: one per key, in order (undefined until written).

### `ScreenReport`

```ts
interface ScreenReport
```

One screen's outcome in the self-test.

- `readonly screen: string;`
- `readonly ok: boolean;`
- `readonly scale: number;`
- `readonly dropped: readonly string[];`
- `readonly clipped: boolean;`
- `readonly crowded: readonly string[]`

### `DitherAlgo`

```ts
type DitherAlgo = "atkinson" | "floyd" | "threshold"
```

### `Ms`

```ts
type Ms = number
```

Epoch milliseconds (what `Date.now()` returns): every instant in the schema.

### `Sky`

```ts
enum Sky
```

The sky, as the weather symbols draw it.

- `Clear = 0`
- `Partly = 1`
- `Cloudy = 2`
- `Rain = 3`
- `Snow = 4`
- `Storm = 5`
- `Fog = 6`

### `UvLevel`

```ts
enum UvLevel
```

UV index bands.

- `Low = 0`
- `Moderate = 1`
- `High = 2`
- `VeryHigh = 3`
- `Extreme = 4`

### `AirBand`

```ts
enum AirBand
```

European air-quality bands (from the European AQI).

- `Good = 0`
- `Fair = 1`
- `Moderate = 2`
- `Poor = 3`
- `VeryPoor = 4`
- `ExtremelyPoor = 5`

### `CarbonIndex`

```ts
enum CarbonIndex
```

The grid's carbon-intensity index.

- `VeryLow = 0`
- `Low = 1`
- `Moderate = 2`
- `High = 3`
- `VeryHigh = 4`

### `Pollutant`

```ts
enum Pollutant
```

The pollutants the air-quality service reports.

- `Pm25 = 0`
- `Pm10 = 1`
- `No2 = 2`
- `O3 = 3`

### `Fuel`

```ts
enum Fuel
```

Generation fuels in the grid mix.

- `Biomass = 0`
- `Coal = 1`
- `Imports = 2`
- `Gas = 3`
- `Nuclear = 4`
- `Other = 5`
- `Hydro = 6`
- `Solar = 7`
- `Wind = 8`

### `Presence`

```ts
enum Presence
```

What the assistant is doing.

- `Idle = 0`
- `Listening = 1`
- `Thinking = 2`
- `Speaking = 3`

### `RateKind`

```ts
enum RateKind
```

The tariff's shape: one rate all day, or a rate per half hour.

- `Flat = 0`
- `HalfHourly = 1`

### `MoonPhase`

```ts
enum MoonPhase
```

The eight named phases of the moon.

- `New = 0`
- `WaxingCrescent = 1`
- `FirstQuarter = 2`
- `WaxingGibbous = 3`
- `Full = 4`
- `WaningGibbous = 5`
- `LastQuarter = 6`
- `WaningCrescent = 7`

### `Hour`

```ts
interface Hour
```

One hour of the forecast: temperature in °C, rain probability 0 to 100, UV index.

- `readonly at: Ms;`
- `readonly temp: number;`
- `readonly rain: number;`
- `readonly uv: number`

### `CalEvent`

```ts
interface CalEvent
```

A calendar event. `end` is exclusive; an all-day event starts at local midnight.

- `readonly id: string;`
- `readonly title: string;`
- `readonly start: Ms;`
- `readonly end: Ms;`
- `readonly allDay: boolean`

### `Rate`

```ts
interface Rate
```

A unit rate for the half hour starting at `at`, in pence per kWh including VAT.

- `readonly at: Ms;`
- `readonly pence: number`

### `Intensity`

```ts
interface Intensity
```

Grid carbon intensity for the half hour starting at `at`, in gCO₂ per kWh.

- `readonly at: Ms;`
- `readonly grams: number`

### `Span`

```ts
interface Span
```

A stretch of time with its average value.

- `readonly from: Ms;`
- `readonly to: Ms;`
- `readonly avg: number`

### `Peak`

```ts
interface Peak
```

A value at an instant (a daily low or high).

- `readonly at: Ms;`
- `readonly value: number`

### `MixShare`

```ts
interface MixShare
```

One fuel's share of generation, in percent.

- `readonly fuel: Fuel;`
- `readonly pct: number`

### `Proc`

```ts
interface Proc
```

A process and the share of one core it uses.

- `readonly name: string;`
- `readonly cpu: number`

### `Grocery`

```ts
interface Grocery
```

A shopping-list line.

- `readonly text: string;`
- `readonly qty: string;`
- `readonly done: boolean`

### `Note`

```ts
interface Note
```

A note to keep in mind; `due` is whatever the person wrote ("Fri").

- `readonly text: string;`
- `readonly due: string`

### `DayUsage`

```ts
interface DayUsage
```

Electricity used on one local day, in kWh.

- `readonly day: Ms;`
- `readonly kwh: number`

### `LoadHistory`

```ts
interface LoadHistory
```

Load averages over their history windows, as series.

- `readonly m1: readonly number[];`
- `readonly m5: readonly number[];`
- `readonly m15: readonly number[]`

## Live data keys

### `K.cpu.total`

```ts
K.cpu.total: Key<number>
```

Overall CPU use, percent.

- `id: cpu.total`

### `K.cpu.hist`

```ts
K.cpu.hist: Key<number[]>
```

Recent overall CPU use, 0..1 per sample, oldest first.

- `id: cpu.hist`

### `K.cpu.count`

```ts
K.cpu.count: Key<number>
```

Number of CPU cores.

- `id: cpu.count`

### `K.cpu.cores`

```ts
K.cpu.cores: Key<Record<string, number[]>
```

Per-core use histories (0..1), by core name; Linux only.

- `id: cpu.cores`

### `K.sys.load`

```ts
K.sys.load: Key<number>
```

One-minute load average.

- `id: sys.load`

### `K.sys.load5`

```ts
K.sys.load5: Key<number>
```

Five-minute load average.

- `id: sys.load5`

### `K.sys.load15`

```ts
K.sys.load15: Key<number>
```

Fifteen-minute load average.

- `id: sys.load15`

### `K.sys.loadHist`

```ts
K.sys.loadHist: Key<LoadHistory>
```

Load average histories.

- `id: sys.load_hist`

### `K.sys.host`

```ts
K.sys.host: Key<string>
```

Host name.

- `id: sys.host`

### `K.sys.kernel`

```ts
K.sys.kernel: Key<string>
```

Kernel release.

- `id: sys.kernel`

### `K.sys.uptime`

```ts
K.sys.uptime: Key<number>
```

Seconds since boot.

- `id: sys.uptime`

### `K.sys.temp`

```ts
K.sys.temp: Key<number>
```

Hottest sensor, °C (Linux).

- `id: sys.temp`

### `K.mem.total`

```ts
K.mem.total: Key<number>
```

Installed memory, GB.

- `id: mem.total`

### `K.mem.apps`

```ts
K.mem.apps: Key<number>
```

Memory used by applications, GB.

- `id: mem.apps`

### `K.mem.cache`

```ts
K.mem.cache: Key<number>
```

Reclaimable cache, GB.

- `id: mem.cache`

### `K.mem.free`

```ts
K.mem.free: Key<number>
```

Free memory, GB.

- `id: mem.free`

### `K.mem.pct`

```ts
K.mem.pct: Key<number>
```

Memory in use, percent.

- `id: mem.pct`

### `K.mem.swapUsed`

```ts
K.mem.swapUsed: Key<number>
```

Swap in use, GB.

- `id: swap.used`

### `K.mem.swapTotal`

```ts
K.mem.swapTotal: Key<number>
```

Swap size, GB.

- `id: swap.total`

### `K.disk.used`

```ts
K.disk.used: Key<number>
```

Disk used, TB.

- `id: disk.used`

### `K.disk.total`

```ts
K.disk.total: Key<number>
```

Disk size, TB.

- `id: disk.total`

### `K.disk.pct`

```ts
K.disk.pct: Key<number>
```

Disk used, percent.

- `id: disk.pct`

### `K.disk.hist`

```ts
K.disk.hist: Key<number[]>
```

Daily disk use, TB, oldest first.

- `id: disk.hist`

### `K.proc.top`

```ts
K.proc.top: Key<Proc[]>
```

The busiest processes.

- `id: proc.top`

### `K.proc.count`

```ts
K.proc.count: Key<number>
```

Number of processes.

- `id: proc.count`

### `K.net.iface`

```ts
K.net.iface: Key<string | null>
```

The default network interface.

- `id: net.iface`

### `K.net.ip`

```ts
K.net.ip: Key<string | null>
```

Local IP address.

- `id: net.ip`

### `K.net.down`

```ts
K.net.down: Key<number>
```

Download throughput, MB/s.

- `id: net.down`

### `K.net.up`

```ts
K.net.up: Key<number>
```

Upload throughput, MB/s.

- `id: net.up`

### `K.net.downHist`

```ts
K.net.downHist: Key<number[]>
```

Recent download, MB/s, oldest first.

- `id: net.down_hist`

### `K.net.upHist`

```ts
K.net.upHist: Key<number[]>
```

Recent upload, MB/s, oldest first.

- `id: net.up_hist`

### `K.wifi.rssi`

```ts
K.wifi.rssi: Key<number | null>
```

Signal strength, dBm.

- `id: wifi.rssi`

### `K.wifi.bars`

```ts
K.wifi.bars: Key<number>
```

Signal bars, 0..4.

- `id: wifi.bars`

### `K.wifi.ssid`

```ts
K.wifi.ssid: Key<string | null>
```

Network name, when the system lets scripts read it.

- `id: wifi.ssid`

### `K.wx.temp`

```ts
K.wx.temp: Key<number>
```

Air temperature, °C.

- `id: wx.temp`

### `K.wx.feels`

```ts
K.wx.feels: Key<number>
```

Apparent temperature, °C.

- `id: wx.feels`

### `K.wx.sky`

```ts
K.wx.sky: Key<Sky>
```

The sky now.

- `id: wx.sky`

### `K.wx.wind`

```ts
K.wx.wind: Key<number>
```

Wind speed, mph.

- `id: wx.wind`

### `K.wx.windDir`

```ts
K.wx.windDir: Key<number>
```

Where the wind comes from, degrees (0 = north).

- `id: wx.wind_dir`

### `K.wx.cloud`

```ts
K.wx.cloud: Key<number>
```

Cloud cover in oktas, 0..8.

- `id: wx.cloud`

### `K.wx.pressure`

```ts
K.wx.pressure: Key<number>
```

Sea-level pressure, hPa.

- `id: wx.pressure`

### `K.wx.hours`

```ts
K.wx.hours: Key<Hour[]>
```

The next 24 hours of forecast, starting with the current hour.

- `id: wx.hours`

### `K.wx.uvDay`

```ts
K.wx.uvDay: Key<number[]>
```

Today's UV index for each hour from 00:00 (24 values).

- `id: wx.uv_day`

### `K.wx.uvNow`

```ts
K.wx.uvNow: Key<number>
```

UV index now.

- `id: wx.uv`

### `K.wx.uvMax`

```ts
K.wx.uvMax: Key<number>
```

Today's peak UV index.

- `id: wx.uv_max`

### `K.wx.rainFrom`

```ts
K.wx.rainFrom: Key<Ms | null>
```

When rain is next likely (50% or more), or null for none in 24 h.

- `id: wx.rain_from`

### `K.sun.rise`

```ts
K.sun.rise: Key<Ms>
```

Sunrise today.

- `id: sun.rise`

### `K.sun.set`

```ts
K.sun.set: Key<Ms>
```

Sunset today.

- `id: sun.set`

### `K.moon.phase`

```ts
K.moon.phase: Key<number>
```

Fraction of the lunar month since new moon: 0 new, 0.5 full.

- `id: moon.phase`

### `K.air.eaqi`

```ts
K.air.eaqi: Key<number>
```

European air-quality index (0 best).

- `id: aq.eaqi`

### `K.air.pm25`

```ts
K.air.pm25: Key<number>
```

PM2.5, µg/m³.

- `id: aq.pm25`

### `K.air.pm10`

```ts
K.air.pm10: Key<number>
```

PM10, µg/m³.

- `id: aq.pm10`

### `K.air.no2`

```ts
K.air.no2: Key<number>
```

Nitrogen dioxide, µg/m³.

- `id: aq.no2`

### `K.air.o3`

```ts
K.air.o3: Key<number>
```

Ozone, µg/m³.

- `id: aq.o3`

### `K.air.main`

```ts
K.air.main: Key<Pollutant>
```

The pollutant furthest into its band.

- `id: aq.main`

### `K.carbon.slots`

```ts
K.carbon.slots: Key<Intensity[]>
```

Forecast carbon intensity for the next 24 hours, half-hourly.

- `id: carbon.slots`

### `K.carbon.now`

```ts
K.carbon.now: Key<number>
```

Carbon intensity now, gCO₂/kWh.

- `id: carbon.now`

### `K.carbon.index`

```ts
K.carbon.index: Key<CarbonIndex>
```

The grid's own rating of now.

- `id: carbon.index`

### `K.carbon.green`

```ts
K.carbon.green: Key<number>
```

The line under which energy counts as green, gCO₂/kWh.

- `id: carbon.green`

### `K.carbon.region`

```ts
K.carbon.region: Key<string>
```

The grid region the forecast is for.

- `id: carbon.region`

### `K.carbon.best`

```ts
K.carbon.best: Key<Span | null>
```

The cleanest two hours from now.

- `id: carbon.best`

### `K.carbon.mix`

```ts
K.carbon.mix: Key<MixShare[]>
```

Generation by fuel now.

- `id: carbon.mix`

### `K.power.tariff`

```ts
K.power.tariff: Key<string>
```

The tariff's name.

- `id: power.tariff`

### `K.power.kind`

```ts
K.power.kind: Key<RateKind>
```

Whether the rate is flat or half-hourly.

- `id: power.kind`

### `K.power.rate`

```ts
K.power.rate: Key<number>
```

Unit rate now, p/kWh inc. VAT.

- `id: power.rate`

### `K.power.standing`

```ts
K.power.standing: Key<number>
```

Standing charge, p/day.

- `id: power.standing`

### `K.power.slots`

```ts
K.power.slots: Key<Rate[]>
```

Today's half-hourly rates (empty for a flat tariff).

- `id: power.slots`

### `K.power.nextSlots`

```ts
K.power.nextSlots: Key<Rate[]>
```

Tomorrow's half-hourly rates, once published.

- `id: power.next_slots`

### `K.power.min`

```ts
K.power.min: Key<Peak | null>
```

Today's lowest rate (half-hourly tariffs).

- `id: power.min`

### `K.power.max`

```ts
K.power.max: Key<Peak | null>
```

Today's highest rate (half-hourly tariffs).

- `id: power.max`

### `K.power.avg`

```ts
K.power.avg: Key<number | null>
```

Today's average rate (half-hourly tariffs).

- `id: power.avg`

### `K.power.cheap`

```ts
K.power.cheap: Key<Span | null>
```

The cheapest two hours from now (half-hourly tariffs).

- `id: power.cheap`

### `K.power.usageDay`

```ts
K.power.usageDay: Key<Ms | null>
```

The newest local day with a full set of meter readings.

- `id: power.usage_day`

### `K.power.usageSlots`

```ts
K.power.usageSlots: Key<number[]>
```

That day's usage per half hour, kWh.

- `id: power.usage_slots`

### `K.power.usageKwh`

```ts
K.power.usageKwh: Key<number>
```

That day's total, kWh.

- `id: power.usage_kwh`

### `K.power.usageCost`

```ts
K.power.usageCost: Key<number>
```

That day's cost in pounds, standing charge included.

- `id: power.usage_cost`

### `K.power.week`

```ts
K.power.week: Key<DayUsage[]>
```

Daily usage over the last week.

- `id: power.week`

### `K.power.latest`

```ts
K.power.latest: Key<Ms | null>
```

When the newest meter reading ends.

- `id: power.usage_latest`

### `K.power.updated`

```ts
K.power.updated: Key<Ms>
```

When the account was last read.

- `id: power.updated`

### `K.cal.events`

```ts
K.cal.events: Key<CalEvent[]>
```

Events from today through the next seven days, by start.

- `id: cal.events`

### `K.shop.items`

```ts
K.shop.items: Key<Grocery[]>
```

The shopping list.

- `id: shop.items`

### `K.notes.items`

```ts
K.notes.items: Key<Note[]>
```

Notes to keep in mind.

- `id: notes.items`

### `K.claude.state`

```ts
K.claude.state: Key<Presence>
```

What the assistant is doing.

- `id: claude.state`

### `K.claude.heard`

```ts
K.claude.heard: Key<string>
```

The last thing it heard.

- `id: claude.heard`

### `K.claude.say`

```ts
K.claude.say: Key<string>
```

The last thing it said.

- `id: claude.say`

### `K.claude.at`

```ts
K.claude.at: Key<Ms>
```

When it last spoke.

- `id: claude.at`

### `K.timer.end`

```ts
K.timer.end: Key<Ms | null>
```

When the running timer rings, or null.

- `id: timer.end`

### `K.timer.total`

```ts
K.timer.total: Key<number>
```

The running timer's full length, seconds.

- `id: timer.total`

### `K.timer.label`

```ts
K.timer.label: Key<string>
```

What the timer is for.

- `id: timer.label`
