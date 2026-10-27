// Generated from crates/ui/ts/ui.ts and schema.ts by `cargo run -p ui --bin gen-docs`: do not edit.
// The globals available to an eval on the display page.

type Child = Node | string | number | false | null | undefined | Child[];
type Props = { [k: string]: any };
type Arg = Child | Props;
type Replacement = Child;
declare function isNode(x: unknown): x is Node;
declare function isPlain(x: unknown): x is Props;
declare function setProps(el: Element, props: Props): void;
declare function applyArg(el: Element, arg: Arg): void;
declare function h<K extends keyof HTMLElementTagNameMap>(tag: K, ...args: Arg[]): HTMLElementTagNameMap[K];
declare function h(tag: string, ...args: Arg[]): HTMLElement;
declare function frag(...vals: Child[]): DocumentFragment;
declare function def(name: string, desc: PropertyDescriptor): any;
declare function row(...a: Arg[]): any;
declare function col(...a: Arg[]): any;
declare function grid(cols: number, ...a: Arg[]): any;
declare function box(...a: Arg[]): any;
declare function card(title: string, ...a: Arg[]): any;
declare function big(...a: Arg[]): any;
declare function heading(text: Child, level?: 1 | 2 | 3 | 4): any;
declare function divider(): any;
declare function badge(text: Child): any;
declare function stat(label: string, value: Child, o?: { unit?: string; delta?: Child }): any;
declare function list(items: Child[], o?: { ordered?: boolean }): any;
declare function table(rows: Child[][], o?: { head?: Child[] }): any;
declare function kv(obj: Record<string, Child>): any;
declare function progress(value: number, label?: string): any;
declare function bars(data: (number | [string, number])[], o?: { height?: number; values?: boolean }): any;
declare function sparkline(values: number[], o?: { w?: number; h?: number }): any;
declare function agenda(events: { time: string; title: string; note?: string }[]): any;
declare function weather(o: { temp: string; cond: string; icon?: string; hi?: string; lo?: string }): any;
/** Tiny markdown: #/##/### headings, - lists, **bold**, *italic*, `code`. */
declare function md(src: string): any;
declare function circ(cx: number, cy: number, r: number): any;
/** The built-in icon names. */
type IconName = "sun" | "moon" | "cloud" | "rain" | "snow" | "bolt" | "wind" | "fog" | "drop" | "thermo" | "umbrella" | "home" | "door" | "lock" | "tv" | "bulb" | "plug" | "wifi" | "battery" | "cpu" | "disk" | "memory" | "phone" | "car" | "bus" | "bag" | "cup" | "utensils" | "book" | "mail" | "calendar" | "clock" | "bell" | "trash" | "flag" | "pin" | "user" | "pulse" | "wave" | "leaf" | "star" | "check" | "cross" | "warn" | "info" | "sparkle" | "hub" | "nodes" | "peers" | "arrow" | "chevron" | "sunrise" | "sunset" | "partly" | "cloudsnow" | "storm" | "sigil-h" | "sigil-v" | "sigil-c" | "plusbox";
declare function svgIcon(inner: string, size: number | string, stroke?: any): any;
declare function iconBox(name: string): [number, number, number, number];
declare function icon(name: IconName, o?: { size?: number; fill?: boolean; stroke?: number }): any;
declare function rays(cx: number, cy: number, r1: number, r2: number, n?: any): any;
/** Vertical hatching clipped to a disc: the "grey" of a drawing. x runs from..to across a circle at (cx, cy). */
declare function hatchDisc(cx: number, cy: number, r: number, from: number, to: number, step?: any): any;
/** Weather symbols from the icon set: clear, night, partly, cloudy, rain, snow, storm, fog, wind (hairline at large sizes). */
declare function weatherIcon(kind: string, size?: any): any;
/** Cap dynamic text to N lines so unexpected data can't push the scene off-screen. */
declare function clamp(lines: number, ...a: Arg[]): any;
declare function fill(...a: Arg[]): any;
declare function spacer(): any;
declare function center(...a: Arg[]): any;
declare function chips(items: Child[]): any;
declare function topbar(title: Child, right?: Child): any;
declare function banner(text: Child, o?: { icon?: string }): any;
declare function quote(text: Child, by?: Child): any;
declare function trend(n: number, unit?: any): any;
declare function todo(items: (string | { text: string; done?: boolean })[]): any;
declare function escHtml(s: string): any;
declare function gauge(value: number, label?: string, o?: { size?: number; text?: string }): any;
declare function month(date?: Date, o?: { marks?: number[]; weekStart?: 0 | 1 }): any;
declare function lineChart(values: number[], o?: { w?: number; h?: number; labels?: string[] }): any;
declare function image(src: string, o?: { fit?: "cover" | "contain"; w?: number | string; h?: number | string }): any;
/** Claude speaking: a speech bubble with a round sparkle avatar. */
declare function say(text: Child, o?: { icon?: string }): any;
declare function minutes(hm: string): any;
/** Minutes after local midnight of an instant (epoch ms): the older components also take an "HH:MM" text. */
declare function toMinutes(t: number | string): number;
/** Minutes after midnight as "HH:MM". */
declare function hhmm(m: number): string;
/** Sunrise to sunset arc with the sun where it is now. Times are "HH:MM". */
declare function sunArc(o: { rise: number | string; set: number | string; now?: number | string; w?: number }): any;
/** A strip of hours: time, weather picture, temperature, chance of rain (0..1). */
declare function hourly(items: { t: string; temp: string; icon?: string; pop?: number }[]): any;
/** Contribution-style dot grid; levels 0..3, filled column by column. */
declare function habitGrid(levels: number[], o?: { rows?: number }): any;
declare function mmss(s: number): any;
/** A countdown dial: a ring of 60 ticks, the time still to run drawn long and heavy, the elapsed ones thin. */
declare function timerRing(remaining: number, total: number, label?: string): any;
/** A tear-off calendar page. */
declare function tearoff(date?: Date): any;
/** A handwritten sticky note, slightly tilted. */
declare function note(text: Child, o?: { tilt?: number }): any;
/** Who is home: solid circle with an initial when home, dashed when away. */
declare function presence(people: { name: string; home: boolean }[]): any;
/** Moon phase, 0 = new, 0.25 = first quarter, 0.5 = full, 0.75 = last quarter. The shadow is hatched, the lit side open. */
declare function moon(phase: number, size?: any): any;
/** A launcher-style module: label top-left, something in the middle, a count bottom-left. */
declare function tile(label: string, body?: Child, o?: { count?: Child; inverted?: boolean; bleed?: boolean; tone?: string; glyph?: IconName; waves?: boolean | Parameters<typeof waves>[0]; back?: Node }): any;
/** Compact ruled rows [left, text, right?]. The text takes the space and is cut with an ellipsis; rows share the height. */
declare function lines(rows: [Child, Child, Child?][]): any;
/** A shopping-list style checklist: [text, qty?, done?]. Open items are an empty square, done ones a solid one. */
declare function checklist(items: [Child, Child?, boolean?][]): any;
/** A box that is its own measuring frame (container units inside it refer to it, not to the tile). */
declare function pane(...a: Arg[]): any;
declare function motionName(p: string): any;
/** A blinking block cursor. */
declare function cursor(): any;
/** A spinner of 12 dots; the bright one steps round once a second. */
declare function spinner(size?: any): any;
/** A countdown ring that ticks once a second and stops at zero (then calls onDone). */
declare function liveTimer(seconds: number, o?: { label?: string; total?: number; onDone?: () => void }): any;
/** Text that types itself in, `step` words per second. The full text is laid out (hidden) first, so the size never changes. */
declare function typewriter(text: string, o?: { step?: number; cursor?: boolean }): any;
/** A big numeral that counts up to `to` in `ticks` one-second steps (eased). Sized for the final text, so it never reflows. */
declare function countUp(to: number, o?: { from?: number; ticks?: number; dec?: number; unit?: string }): any;
/** A graph that scrolls: each tick `next()` gives one new value per series, the oldest drops off, `draw` rebuilds the graph. */
declare function stream(series: number[][], next: () => number[], draw: (s: number[][]) => HTMLElement, ms?: any): any;
/** A tile that fills like a tank (level 0..1) with a wavy surface. */
declare function levelTile(label: string, level: number, text?: Child): any;
/** The silhouette of a usage history, held between samples (capacity = full height): a `back` for a tile with tone "usage". */
declare function usageFill(values: readonly number[], cap: number): HTMLElement;
/** A number written over a fill (inverted against what is behind it): the `count` of a usage tile. */
declare function chip(text: Child): HTMLElement;
/** A minimal box filled from the bottom up to a history curve (held between samples): the capacity is the
 * full height, so the silhouette IS the usage over time. A number sits inside. Pair it with a chart. */
declare function usageTile(label: string, values: number[], cap: number, text: Child, o?: { glyph?: string }): any;
/** A 24-hour donut: noon at the top, midnight at the bottom. The night stretch is solid, the
 * day stretch open, and a marker sits where "now" is. Times are "HH:MM". */
declare function dayDial(o: { rise: number | string; set: number | string; now?: number | string; size?: number; sub?: string; callouts?: boolean; side?: number }): any;
/** A scale with ticks, a marker at the current value and an optional target tick. */
declare function ruler(o: { min: number; max: number; value: number; target?: number; step?: number; w?: number }): any;
/** Hairline-divided readouts: a tiny label, a big value, a note underneath. */
declare function statStrip(cells: { label: string; value: Child; sub?: Child }[]): any;
/** Scene chips; the active one is inverted. Items are strings or {label, icon}. */
declare function pills(items: (string | { label: string; icon?: string })[], o?: { active?: number }): any;
/** A level bar with its label and value written on it (text flips colour over the fill). */
declare function slider(value: number, label: Child, o?: { text?: Child }): any;
/** Settings-style rows: label with a status dot (ok / warn / off), then a value or an arrow. */
declare function menu(rows: { label: Child; value?: Child; dot?: "ok" | "warn" | "off" }[]): any;
/** A device row: round icon, name and state, and a switch (display only). */
declare function device(o: { name: Child; sub?: Child; on?: boolean; icon?: string }): any;
/** One huge number with a small unit and a status line. */
declare function hero(value: Child, o?: { unit?: string; label?: Child; status?: Child; sub?: Child; size?: number }): any;
/** A polar plot: rings, spokes, the values as a hatched polygon. Good for anything cyclic (hours, compass). */
declare function polar(values: number[], o?: { max?: number; rings?: number; labels?: [string, number][]; marks?: boolean }): any;
/** Rows stacked as ridgelines, back to front, each hiding the ones behind it: a surface made of days. */
declare function ridgeline(rows: number[][], o?: { w?: number; h?: number; labels?: string[]; hours?: [string, number][] }): any;
/** A spaced-caps word with a caption: the quick-glance status screen (e.g. FLX, NUC). */
declare function word(text: string, caption?: Child): any;
/** One continuous gradient whose flow is twisted by a domain-warped sine field, so it swirls like marbling rather than
 * bumping. Drawn once to a small grey canvas and used as an image (so it survives in the DOM). At 1-bit the dither
 * turns it into grain. Put one behind a tile's content with tile {waves}. */
declare function waves(o?: { from?: number; seed?: number; scale?: number; tone?: "sky" | "water" }): any;
/** Two dot graphs mirrored on a centre line: `up` above, `down` below (traffic in/out). Values 0..1. */
declare function dotMirror(up: number[], down: number[], o?: { rows?: number }): any;
/** Rows of  LABEL  [small dot graph]  value, one graph per row (cores, disks, ...). */
declare function sparkRows(rows: { label: string; values: number[]; text: Child }[], o?: { rows?: number }): any;
type SigKind = "state" | "level" | "trend" | "age" | "count" | "dir" | "bars" | "glyph";
/** The marks are drawn in a 12x12 box, 1px line. What they say:
 * state  "ok" solid square | "idle" open square | "warn" half-filled square | "err" crossed square | anything else: dashed square (unknown)
 * level  [value, max] (max 8 by default): a square tank filled from the bottom (empty..full)
 * trend  "up" | "down" | "flat" | "rise" | "fall"  (slow and steep arrows)
 * age    minutes since the data was fresh: the hand of a one-hand clock in a square (60 or more: dashed, stale)
 * count  1..6 as dice faces, more as a plus
 * dir    degrees, 0 = up: a rotated arrow (wind, bearing)
 * bars   0..4 signal bars
 * glyph  any icon name, scaled down */
declare function sigPaths(kind: SigKind, v: any): string;
/** One sigil as an inline element (use it next to text, in a table cell, in a module label). */
declare function sigil(kindOrMark: SigKind | SigMark, v?: any, size?: any): any;
declare function sigils(items: [SigKind, any?][], size?: any): any;
/** A ruled bar of labelled segments (the first is a solid mark), then a gap, then right-aligned segments. */
declare function statusStrip(left: Child[], right?: Child[]): any;
/** A bar: the filled part is solid, the rest a halftone. value of max. */
declare function segbar(value: number, max?: any): any;
/** Rows of  LABEL  [segmented bar]  value unit. Dense and exact. */
declare function readouts(rows: { label: string; value: number; max?: number; unit?: string; text?: Child }[]): any;
/** Key/value pairs in a grid of columns (default 2), each pair a ruled row. */
declare function kvgrid(obj: Record<string, Child>, cols?: any): any;
/** A small framed square: a weather icon over a dithered sky, with a value written in the dark part. */
declare function skybox(name: string, text: Child, size?: any): any;
/** The sheet header: small stacked kicker lines, a large code, right-aligned lines. */
declare function header(o: { kicker: string[]; code: string; right?: Child[] }): any;
/** A ruled table of cells; each cell is {title, rows: [label, value][]}. Like a drawing's header block. */
declare function ruled(...cells: { title?: string; rows: [string, Child][] }[]): any;
/** A calibration strip of grey steps (the panel dithers them), the way a print sheet carries one. */
declare function toneStrip(steps?: any): any;
type Stop = { at: number | string; to?: number | string; name: string };
/** A route diagram of the day: lanes of stops/spans along a time axis. `from`, `to`, `now` and each stop's `at`/`to` are instants (epoch ms), or "HH:MM" text. */
declare function routes(o: { from: number | string; to: number | string; now?: number | string; w?: number; lanes: { label: string; look?: "solid" | "dash" | "dot" | "double"; stops: Stop[] }[]; links?: [number, number | string, number][] }): any;
type Tone = "plain" | "inv" | "sky" | "dots";
/** A big region: plain, inverted (white on black), a dithered sky gradient, or a dot field. */
declare function panel(tone: Tone, ...a: Arg[]): any;
/** Tiny all-caps line across the top: left text, right text. */
declare function meta(left: Child, right?: Child): any;
/** Claude's line of the moment: avatar, one sentence, a timestamp. */
declare function claudeLine(text: Child, time?: Child): any;
/** A 6-column launcher grid; place children with style gridArea ("row / col / row-end / col-end"). */
declare function tilegrid(...a: Arg[]): any;
declare function HATCH(id: string): any;
declare function dotMatrixSvg(text: string, pitch: number): any;
/** A dot-matrix clock. Pass `time` ("22:26") for a fixed one, otherwise it follows the time. */
declare function dotClock(o?: { time?: string; pitch?: number }): any;
/** The 24 hours as a ruler: night hatched, day open, a marker where "now" is. Times "HH:MM". */
declare function dayStrip(o: { rise: number | string; set: number | string; now?: number | string; w?: number }): any;
type Block = { from: number | string; to: number | string; label?: string; look?: "solid" | "hatch" | "outline" };
/** Lanes of blocks over a time axis (a Gantt strip), with a "now" line. Times "HH:MM". */
declare function timeline(o: { w?: number; from: string; to: string; now?: string; lanes: { label: string; blocks: Block[] }[] }): any;
/** Hourly values as a stepped area (hatched), an optional solid highlight window, and a "now" marker. */
declare function areaChart(values: number[], o?: { w?: number; h?: number; labels?: [string, number][]; now?: number; window?: [number, number]; marks?: boolean; fmt?: (i: number, v: number) => string; floor?: number; ceil?: number; limit?: [number, string] }): any;
/** Bars against a limit line: values at or under `limit` are solid, over it hatched (carbon intensity: green under 100 g).
 * `now` outlines one bar and flags it; `window` brackets a stretch (the cleanest hours); `labels` are [text, bar index]. */
declare function limitBars(values: number[], o: { limit: number; w?: number; h?: number; now?: number; window?: [number, number]; labels?: [string, number][]; limitLabel?: string; windowLabel?: string }): any;
/** A numeral sized to fill its module (container query units). Keep tiles inside the tilegrid. */
declare function bignum(text: Child, o?: { unit?: string; chars?: number }): any;
/** Put a module in a zone of the sheet: place(node, zone(Z.A3, Z.B3)). The tile's corner shows the zone's top-left address.
 * (A CSS grid-area string is still accepted, for the gallery's old samples; use zones.) */
declare function place<T extends HTMLElement>(node: T, area: Zone | string): T;
/** A value over the day as a smooth curve (UV index, sun height), hatched underneath, with a line and a dot where "now" is.
 * `values` are the 24 hours from 00:00; `now` is the hour as a fraction (20.5 = 20:30); `guides` are dotted [value, label] lines
 * (UV 3 = moderate, 6 = high); the scale runs to at least `max` so a weak winter curve reads as weak, not as full height. */
declare function dayCurve(values: number[], o?: { now?: number; w?: number; h?: number; max?: number; padT?: number; padB?: number; baseline?: boolean; guides?: [number, string][]; labels?: [string, number][] }): any;
/** LED-style histogram: columns of dots lit from the bottom. Values 0..1. */
declare function dotBars(values: number[], o?: { rows?: number; labels?: string[] }): any;
/** A quantity as a grid of cells: `value` of `total` filled, a fraction shows as a partial cell. */
declare function waffle(value: number, total: number, o?: { cols?: number; hatch?: number }): any;
/** One bar split into labelled parts (solid / hatched / outlined), widths in proportion. */
declare function stackBar(parts: { label: string; value: number; look?: "solid" | "hatch" | "outline" }[]): any;
/** A half-circle dial with ticks and the value in the middle. */
declare function arcGauge(value: number, o?: { min?: number; max?: number }): any;
/** A full-bleed black band, edge to edge. "top"/"bottom" also reach the screen edge. White on black inside. */
declare function band(edge: "top" | "bottom" | "mid", ...a: Arg[]): any;
/** An oversized numeral (default 240px). Put it in a tile with {bleed: true}: it sits on the module's
 * floor and the module's edge crops it, on purpose. Keep the real value in the label or count. */
declare function giant(value: Child, o?: { size?: number; unit?: string }): any;
/** Sigils for a corner cluster: sig.* values (or the legacy [kind, value] pairs), or a function returning them (re-read at every redraw). */
type SigList = readonly (SigMark | [SigKind, any?])[] | (() => readonly (SigMark | [SigKind, any?])[]);
/** The five operators. */
declare enum BoxOp { Minus = 0, Plus = 1, Divide = 2, Times = 3, Equals = 4 }
/** Which way a number's zigzag runs: across the page (default) or down it. */
declare enum BoxLayout { Horizontal = 0, Vertical = 1 }
type BoxSegment = readonly [number, number, number, number];
/** Where cell `i` of a number's zigzag sits, in cells: across, i down every other one; or the transpose. */
declare function boxCell(i: number, layout?: BoxLayout): { x: number; y: number };
/** The decimal digits of a non-negative whole number, most significant first (zero-padded to `width` digits). */
declare function digitsOf(n: number, width?: any): number[];
declare function strokesPath(segs: readonly BoxSegment[], cell: number, ox: number, oy: number): string;
/** One digit 0..9 as path data in a cell of `cell` units at (x, y). */
declare function boxDigitPath(digit: number, cell?: any, x?: any, y?: any): string;
/** A number's glyph: path data for its digits on the zigzag, and the box it fills (in the same units, `cell` per cell). */
declare function boxNumberPath(digits: readonly number[], o?: { cell?: number; layout?: BoxLayout; x?: number; y?: number }): { d: string; w: number; h: number };
/** A whole number as one glyph, `cell` px per cell (default 8). */
declare function boxNumber(n: number, o?: { cell?: number; layout?: BoxLayout; width?: number }): HTMLElement;
/** An expression as a straight row: numbers (each its own zigzag glyph) and operators, e.g. boxExpression([82, BoxOp.Plus, 85, BoxOp.Equals, 167]). */
declare function boxExpression(items: readonly (number | BoxOp)[], o?: { cell?: number; gap?: number }): HTMLElement;
/** A time: the hours and the minutes, each a two-digit glyph (a two-cell diagonal), with a small space between; `at` is epoch ms (default now). */
declare function boxClock(at?: number, o?: { cell?: number }): HTMLElement;
/** A sigil's meaning for a state mark. */
declare enum State { Ok = 0, Idle = 1, Warn = 2, Err = 3, Unknown = 4 }
/** A sigil's meaning for a trend mark: slow arrows (Up/Down/Flat) and steep ones (Rise/Fall). */
declare enum Trend { Up = 0, Down = 1, Flat = 2, Rise = 3, Fall = 4 }
/** One sigil as data (what to draw), made by the `sig` constructors and drawn by `sigil(mark, size)` or by the frame's corner clusters. */
interface SigMark { readonly kind: SigKind; readonly v: unknown }
/** The sheet's columns, left to right. */
declare enum Col { A = 0, B = 1, C = 2, D = 3, E = 4, F = 5 }
/** The sheet's rows, top to bottom. */
type Row = 1 | 2 | 3;
/** One cell of the sheet. */
interface Cell { readonly col: Col; readonly row: Row }
/** A rectangle of cells, both corners inclusive. */
interface Zone { readonly from: Cell; readonly to: Cell }
/** The names Z.A1 ... Z.F3 (a type for the typed constants below). */
type CellName = `${keyof typeof Col}${Row}`;
/** The block of cells between two corners, in any order: zone(Z.A3, Z.B3). One argument is a single cell. */
declare function zone(a: Cell, b?: Cell): Zone;
declare function zoneArea(z: Zone): string;
/** The address printed in a tile's corner: its top-left cell, e.g. "B2". */
declare function zoneName(z: Zone): string;
declare function zonesOverlap(a: Zone, b: Zone): boolean;
/** What a tile region holds: any child, or a function making it when the screen is built. */
type Slot<T extends Child = Child> = T | (() => T);
/** How a tile is dressed: fill = its body stretches, list = ruled rows, legend = title on the border, iconic = centred icon, usage = filled to a history curve. */
type TileTone = "fill" | "list" | "legend" | "iconic" | "usage";
interface TileSpec { readonly id: string; readonly at: Zone; readonly title: string; readonly glyph?: IconName; readonly tone?: TileTone | readonly TileTone[]; readonly inverted?: boolean; readonly body?: Slot; readonly count?: Slot; readonly back?: Slot<Node>; readonly waves?: boolean | Parameters<typeof waves>[0]; readonly frameless?: boolean; }
interface ScreenSpec { readonly id: string; readonly tab?: string; readonly tiles: readonly TileSpec[]; }
/** The handle `defineScreen` returns: read and change a screen's tiles one at a time. */
interface ScreenHandle { readonly id: string; tiles(): readonly TileSpec[]; tile(id: string): TileSpec | undefined; setTile(spec: TileSpec): void; patchTile(id: string, patch: Partial<Omit<TileSpec, "id">>): void; removeTile(id: string): void; }
interface ScreenState { id: string; tab: string; tiles: readonly TileSpec[] }
declare function toneClass(t: TileSpec["tone"]): string | undefined;
declare function resolveBody<T extends Child>(b: Slot<T> | undefined): T | undefined;
/** Make one tile from its spec. */
declare function tileFrom(s: TileSpec): HTMLElement;
declare function checkTiles(id: string, tiles: readonly TileSpec[]): void;
/** Build a screen's DOM into the scene (its timers replace the previous screen's). */
declare function renderScreen(id: string): void;
/** Register a screen, or replace the one with this id. The first screen defined is home. A screen defined while nothing is showing
 * is shown; redefining the one on show rebuilds it. Overlapping zones or repeated tile ids throw (so the change is rolled back). */
declare function defineScreen(spec: ScreenSpec): ScreenHandle;
declare function screenHandle(id: string): ScreenHandle;
/** Show a screen now. A screen other than home returns home after `hold` ms (default: the key `K.ui.hold`, else 2 minutes; 0 stays). */
declare function go(id: string, o?: { hold?: number }): boolean;
/** Draw the drawing-sheet frame: corner targets, sigil clusters in the top corners, zone letters over the grid columns and numbers beside its rows, a terse title block. */
declare function drawFrame(): void;
/** The drawing-sheet frame, in the screen margin: registration targets, two corner clusters of sigils (tl, tr),
 * column letters (A-F) and row numbers (1-3) matching the tilegrid so every module has an address like "B2",
 * and a terse title block. ui.frame({title, sheet, tl, tr}) turns it on, ui.frame(false) off. */
declare function frame(opt?: boolean | { title?: string; sheet?: string; tl?: SigList; tr?: SigList; tabs?: boolean; clock?: boolean }): void;
/** Run fn now and every `ms` while `el` stays mounted (stops once removed). */
declare function live<T extends Element>(el: T, fn: (el: T) => void, ms: number): T;
declare function setText(el: Element, t: string): any;
declare function clock(o?: { seconds?: boolean; date?: boolean; hour12?: boolean }): any;
declare function countdown(to: Date | string | number, label?: string): any;
declare function fontFormat(file: string): any;
declare function fontFaceCss(): any;
declare function get(name: string): any;
declare function mount<T extends Node>(node: T, parent?: Element): T;
declare function describe(c: Element): any;
type FitReport = { /** Always true: the screen is guaranteed to contain the scene. */ fits: true; /** 1 = natural size. Less means the scene was too big and was shrunk. */ scale: number; /** What was hidden because even the smallest readable size did not fit. */ dropped: string[]; /** True only if nothing was left to hide and the edge had to clip. */ clipped: boolean; /** Spare vertical pixels after fitting. */ freeY: number; /** How far over the scene was at natural size. Positive numbers mean "trim me". */ natural: { overflowX: number; overflowY: number }; /** Modules whose content is bigger than their own cell (it is clipped inside the module). */ crowded: string[]; };
declare function adapt(): FitReport;
/** What a documented item is. */
declare enum DocKind { Function = 0, Value = 1, Interface = 2, Type = 3, Enum = 4, Key = 5 }
/** One parameter of a documented function. */
interface DocParam { readonly name: string; readonly type: string; readonly optional: boolean }
/** An entry of the generated reference: its signature and doc comment, extracted from the kit's own source. */
interface DocItem { readonly name: string; readonly kind: DocKind; readonly signature: string; readonly doc: string; readonly params: readonly DocParam[]; readonly returns: string; readonly members: readonly string[]; readonly global: boolean; readonly line: number; }
/** Look the library up: docs("tile") finds items whose name or comment mentions it; docs() lists everything. Same data as GET /docs. */
declare function docs(query?: string): Promise<DocItem[]>;
/** The fit report for the scene as it is now (re-fits it). */
declare function fits(): FitReport;
declare function begin(): void;
/** How often a live binding or job may repaint: Tick = 1 s (only for what must count), Beat = 30 s (live data), Slow = 5 min. */
declare enum Tier { Tick = "tick", Beat = "beat", Slow = "slow" }
/** A period: a tier, or milliseconds (minimum 1 s, aligned to the clock). */
type Period = Tier | number;
declare function periodOf(r: Period): number;
declare function slotOf(r: Period, now?: any): number;
interface Job { name?: string; every: Period; fn: () => void; slot: number }
declare function stop(which: string | number): void;
/** A named job on the beat: ui.every("clock", Tier.Beat, fn), or with a period in ms. Calling again with the same name replaces it.
 * It first runs at the next boundary of its period, together with everything else due then. */
declare function every(name: string, period: Period, fn: () => void): number;
declare function timers(): any;
/** A typed handle on one live value. `id` is the wire name feeds write; T is what the value is; `sample` is a fixture the self-test shows. */
interface Key<T> { readonly id: string; readonly doc: string; readonly sample: T }
/** Declare a key (the catalogue is in the schema, as K.*). Values are JSON on the wire: numbers, booleans, strings, arrays, objects, null. */
declare function defineKey<T>(id: string, doc: string, sample: T): Key<T>;
/** Whether the link to the server is up. */
declare enum Link { Connecting = 0, Up = 1, Down = 2 }
interface Datum { v: unknown; ts: number }
interface Watcher { keys: string[]; el: Element; draw: () => void; tier: Period; slot: number; stale: boolean; always: boolean; seen: boolean; empty: boolean }
/** Write a value by its wire name (the server's updates arrive this way). Use `setData(key, value)` in your own code. */
declare function setById(id: string, v: unknown, ts?: number): void;
/** Write a value. This never touches the DOM: bindings that show the key go stale and redraw at their next beat. */
declare function setData<T>(key: Key<T>, v: T, ts?: number): void;
/** The latest value of a key, or undefined if it has never been written. */
declare function getData<T>(key: Key<T>): T | undefined;
/** Minutes since a key was last written (Infinity if never): feed sig.age(...) with it. */
declare function dataAge(key: Key<unknown>): number;
/** What a binding's render also receives. */
interface FeedMeta { readonly age: number }
interface FeedOptions { readonly tier?: Period; readonly fallback?: Child; readonly always?: boolean }
/** The values a render gets for a list of keys: one per key, in order (undefined until written). */
type Values<Ks extends readonly Key<any>[]> = { [I in keyof Ks]: Ks[I] extends Key<infer T> ? T | undefined : never };
/** A live binding: feed(K.wifi.bars, (n) => bars(n)) draws now and redraws only at its tier's next beat after the value changes.
 * With a list of keys the render gets a tuple of their values. The first value for an empty binding is drawn at the next second. */
declare function feed<T>(key: Key<T>, render: (v: T | undefined, meta: FeedMeta) => Child, o?: FeedOptions): HTMLElement;
/** A live binding: feed(K.wifi.bars, (n) => bars(n)) draws now and redraws only at its tier's next beat after the value changes.
 * With a list of keys the render gets a tuple of their values. The first value for an empty binding is drawn at the next second. */
declare function feed<Ks extends Key<any>[]>(keys: readonly [...Ks], render: (v: Values<Ks>, meta: FeedMeta) => Child, o?: FeedOptions): HTMLElement;
/** The same as feed, as a function that makes the binding when called: a tile's body, count or back is built fresh with each screen build. */
declare function bind<T>(key: Key<T>, render: (v: T | undefined, meta: FeedMeta) => Child, o?: FeedOptions): () => HTMLElement;
/** The same as feed, as a function that makes the binding when called: a tile's body, count or back is built fresh with each screen build. */
declare function bind<Ks extends Key<any>[]>(keys: readonly [...Ks], render: (v: Values<Ks>, meta: FeedMeta) => Child, o?: FeedOptions): () => HTMLElement;
declare function applyFeeds(now: number): number;
/** The master clock's step: everything due this second runs in this one synchronous batch. */
declare function beatStep(): void;
declare function beatLoop(): void;
/** Change the periods (ms), e.g. ui.cadence({ [Tier.Beat]: 60000 }). Jobs re-base so nothing fires in a burst. */
declare function setCadence(c?: Partial<Record<Tier, number>>): Record<Tier, number>;
/** Follow the server's data feed: long-polls /sub, writing every update into the store (and K.link.server). */
declare function connect(base?: any): Promise<void>;
/** Publish a value to the server's store (and so to every page and script). */
declare function push<T>(key: Key<T>, value: T): any;
/** Run a named command from the server's commands/ directory (never arbitrary shell): resolves {ok, code, stdout, stderr}. */
declare function run(cmd: string, ...args: string[]): Promise<{ ok: boolean; code: number | null; stdout: string; stderr: string; error?: string }>;
declare function flush(): any;
declare function undo(r: MutationRecord): void;
declare function rollback(): void;
/** Commit the changes since the last call as the new known-good state, and make the scene fit. */
declare function verify(): FitReport;
/** Build a screen that is not on show, measure it, and put the previous one back. */
declare function trialScreen(id: string): FitReport;
/** One screen's outcome in the self-test. */
interface ScreenReport { readonly screen: string; readonly ok: boolean; readonly scale: number; readonly dropped: readonly string[]; readonly clipped: boolean; readonly crowded: readonly string[] }
/** Show every screen with every key set to its sample value (or, with `empty`, with no data at all: how a screen looks before the first
 * feed arrives) and report how each fits (ok = nothing crowded, dropped or clipped). Your real data and the current screen are put back.
 * Call it inside a query or tx so the DOM is restored too. */
declare function selfTest(empty?: any): ScreenReport[];
declare function snapshot(): string;
/** Every screen as HTML, rendered from each key's sample data: what crystallizing the journal must leave unchanged. The live scene is put back afterwards. */
declare function frames(): Record<string, string>;
/** Run fn, then verify(). If fn throws, its changes are rolled back too. */
declare function tx<T>(fn: () => T): T;
declare function outline(el?: Element, depth?: any): string;
type DitherAlgo = "atkinson" | "floyd" | "threshold";
/** Quantise to pure black/white, in place. */
declare function dither(img: ImageData, algo?: DitherAlgo): ImageData;
declare function faceCss(): any;
declare function rasterize(el: HTMLElement): Promise<ImageData>;
declare function paintEink(): Promise<void>;
/** ui.eink() atkinson | ui.eink({algo:"floyd"|"threshold"}) | ui.eink(false) off */
declare function eink(opt?: boolean | { algo?: DitherAlgo }): void;
/** Epoch milliseconds (what `Date.now()` returns): every instant in the schema. */
type Ms = number;
/** The sky, as the weather symbols draw it. */
declare enum Sky { Clear = 0, Partly = 1, Cloudy = 2, Rain = 3, Snow = 4, Storm = 5, Fog = 6 }
/** UV index bands. */
declare enum UvLevel { Low = 0, Moderate = 1, High = 2, VeryHigh = 3, Extreme = 4 }
/** European air-quality bands (from the European AQI). */
declare enum AirBand { Good = 0, Fair = 1, Moderate = 2, Poor = 3, VeryPoor = 4, ExtremelyPoor = 5 }
/** The grid's carbon-intensity index. */
declare enum CarbonIndex { VeryLow = 0, Low = 1, Moderate = 2, High = 3, VeryHigh = 4 }
/** The pollutants the air-quality service reports. */
declare enum Pollutant { Pm25 = 0, Pm10 = 1, No2 = 2, O3 = 3 }
/** Generation fuels in the grid mix. */
declare enum Fuel { Biomass = 0, Coal = 1, Imports = 2, Gas = 3, Nuclear = 4, Other = 5, Hydro = 6, Solar = 7, Wind = 8 }
/** What the assistant is doing. */
declare enum Presence { Idle = 0, Listening = 1, Thinking = 2, Speaking = 3 }
/** The tariff's shape: one rate all day, or a rate per half hour. */
declare enum RateKind { Flat = 0, HalfHourly = 1 }
/** The eight named phases of the moon. */
declare enum MoonPhase { New = 0, WaxingCrescent = 1, FirstQuarter = 2, WaxingGibbous = 3, Full = 4, WaningGibbous = 5, LastQuarter = 6, WaningCrescent = 7 }
/** One hour of the forecast: temperature in °C, rain probability 0 to 100, UV index. */
interface Hour { readonly at: Ms; readonly temp: number; readonly rain: number; readonly uv: number }
/** A calendar event. `end` is exclusive; an all-day event starts at local midnight. */
interface CalEvent { readonly id: string; readonly title: string; readonly start: Ms; readonly end: Ms; readonly allDay: boolean }
/** A unit rate for the half hour starting at `at`, in pence per kWh including VAT. */
interface Rate { readonly at: Ms; readonly pence: number }
/** Grid carbon intensity for the half hour starting at `at`, in gCO₂ per kWh. */
interface Intensity { readonly at: Ms; readonly grams: number }
/** A stretch of time with its average value. */
interface Span { readonly from: Ms; readonly to: Ms; readonly avg: number }
/** A value at an instant (a daily low or high). */
interface Peak { readonly at: Ms; readonly value: number }
/** One fuel's share of generation, in percent. */
interface MixShare { readonly fuel: Fuel; readonly pct: number }
/** A process and the share of one core it uses. */
interface Proc { readonly name: string; readonly cpu: number }
/** A shopping-list line. */
interface Grocery { readonly text: string; readonly qty: string; readonly done: boolean }
/** A note to keep in mind; `due` is whatever the person wrote ("Fri"). */
interface Note { readonly text: string; readonly due: string }
/** Electricity used on one local day, in kWh. */
interface DayUsage { readonly day: Ms; readonly kwh: number }
/** Load averages over their history windows, as series. */
interface LoadHistory { readonly m1: readonly number[]; readonly m5: readonly number[]; readonly m15: readonly number[] }
declare function wave(n: number, base: number, amp: number, seed?: any): any;
/** The UV band for an index value. */
declare function uvLevel(uv: number): UvLevel;
/** The European air-quality band for an index value. */
declare function airBand(eaqi: number): AirBand;
/** The named moon phase for a lunar fraction (0 new, 0.5 full). */
declare function moonPhaseOf(fraction: number): MoonPhase;
/** Whether a carbon intensity counts as green, against a line (default 100 gCO₂/kWh). */
declare function isGreen(grams: number, line?: any): boolean;
/** The weather symbol the kit draws for a sky: pass it to weatherIcon(). */
declare function skyIcon(s: Sky): "clear" | "partly" | "cloudy" | "rain" | "snow" | "storm" | "fog";
declare function two(n: number): any;
/** The start of the local day containing `at`. */
declare function dayStart(at: Ms): Ms;
/** Set up the frame: registration marks, live sigils in the corners (left = the machine, right = the world) and the tab bar. */
declare function shell(): void;
declare function div(...a: Arg[]): HTMLElement;
declare function span(...a: Arg[]): HTMLElement;
declare function p(...a: Arg[]): HTMLElement;
declare function section(...a: Arg[]): HTMLElement;
declare function article(...a: Arg[]): HTMLElement;
declare function header(...a: Arg[]): HTMLElement;
declare function footer(...a: Arg[]): HTMLElement;
declare function main(...a: Arg[]): HTMLElement;
declare function aside(...a: Arg[]): HTMLElement;
declare function nav(...a: Arg[]): HTMLElement;
declare function h1(...a: Arg[]): HTMLElement;
declare function h2(...a: Arg[]): HTMLElement;
declare function h3(...a: Arg[]): HTMLElement;
declare function h4(...a: Arg[]): HTMLElement;
declare function ul(...a: Arg[]): HTMLElement;
declare function ol(...a: Arg[]): HTMLElement;
declare function li(...a: Arg[]): HTMLElement;
declare function table(...a: Arg[]): HTMLElement;
declare function thead(...a: Arg[]): HTMLElement;
declare function tbody(...a: Arg[]): HTMLElement;
declare function tr(...a: Arg[]): HTMLElement;
declare function td(...a: Arg[]): HTMLElement;
declare function th(...a: Arg[]): HTMLElement;
declare function em(...a: Arg[]): HTMLElement;
declare function strong(...a: Arg[]): HTMLElement;
declare function small(...a: Arg[]): HTMLElement;
declare function code(...a: Arg[]): HTMLElement;
declare function pre(...a: Arg[]): HTMLElement;
declare function mark(...a: Arg[]): HTMLElement;
declare function sup(...a: Arg[]): HTMLElement;
declare function sub(...a: Arg[]): HTMLElement;
declare function br(...a: Arg[]): HTMLElement;
declare function hr(...a: Arg[]): HTMLElement;
declare function img(...a: Arg[]): HTMLElement;
declare function blockquote(...a: Arg[]): HTMLElement;
declare function figure(...a: Arg[]): HTMLElement;
declare function figcaption(...a: Arg[]): HTMLElement;
declare function bold(...a: Arg[]): HTMLElement;
declare function italic(...a: Arg[]): HTMLElement;
/** Words for the enums. */
declare const label: {
sky(s: Sky): string;
uv(l: UvLevel): string;
air(b: AirBand): string;
carbon(c: CarbonIndex): string;
pollutant(p: Pollutant): string;
fuel(f: Fuel): string;
presence(p: Presence): string;
moon(m: MoonPhase): string;
};
/** Sigil states for the bands: ok for good ones. */
declare const stateOf: {
carbon(c: CarbonIndex): any;
air(b: AirBand): any;
presence(p: Presence): any;
};
/** Formatting: numbers, durations and instants as text. Every function takes typed values, never text. */
declare const fmt: {
/** A number with fixed decimals, or a dash for a missing value. */
num(v: number | null | undefined, digits?: any): string;
/** Minutes as "45m" or "1h30". */
duration(minutes: number): string;
/** Seconds as "13d 4h" or "5h 12m". */
uptime(seconds: number | null | undefined): string;
/** An instant as the local 24-hour clock, "09:30". */
clock(at: Ms | null | undefined): string;
/** An instant's local weekday, "Tue". */
weekday(at: Ms): string;
/** An instant's local date, "Tue 27 Oct". */
date(at: Ms): string;
/** Degrees as a compass point, "SW". */
compass(degrees: number): string;
/** A percentage rounded, with its sign. */
percent(v: number | null | undefined): string;
/** The hour of an instant as two digits, "07". */
hour(at: Ms): string;
};
/** Helpers for number series. */
declare const series: {
/** The last n values, padded with zeros on the left. */
last(values: readonly number[] | null | undefined, n: number): number[];
/** Scale to 0..1 against `top`, square-rooted so small values stay visible. */
sqrtScale(values: readonly number[], top: number): number[];
};
/** Calendar queries over `K.cal.events`. */
declare const calendar: {
/** Events that start on the local day of `at`. */
on(events: readonly CalEvent[], at: Ms): CalEvent[];
/** Whether an event is over at `at`. */
over(e: CalEvent, at: Ms): boolean;
/** Pack events into lanes so that none overlap within a lane (the first lane takes what it can, the next takes what is left): the rows
 * of a route diagram. Events that only touch (one ends as the next starts) share a lane. */
lanes(events: readonly CalEvent[]): CalEvent[][];
/** An event's label in a list: "09:30" today, "Tue 09:30" on another day, "all day" for an all-day event. */
when(e: CalEvent, at: Ms): string;
/** The next `n` events that are not over, from `at`. */
upcoming(events: readonly CalEvent[], at: Ms, n: number): CalEvent[];
/** The first timed event that has not started, and minutes until it. */
next(events: readonly CalEvent[], at: Ms): { event: CalEvent; inMinutes: number } | undefined;
/** The day to draw as "the day": today while something timed is still to come, else the next day with something. */
focus(events: readonly CalEvent[], at: Ms): { day: Ms; isToday: boolean; events: CalEvent[] };
};
/** Typed constructors for sigils: sig.state(State.Ok), sig.level(7, 8), sig.trend(Trend.Fall), sig.age(minutes), sig.count(n), sig.dir(degrees), sig.bars(0..4), sig.glyph(icon). */
declare const sig: {
/** A solid, open, half-filled or crossed square. */
state(s: State): SigMark;
/** A square tank filled from the bottom: value of max. */
level(value: number, max?: any): SigMark;
trend(t: Trend): SigMark;
/** A one-hand clock in a square: minutes since the data was fresh (60 or more draws dashed, stale). */
age(minutes: number): SigMark;
/** Dice faces 1..6, a plus beyond. */
count(n: number): SigMark;
/** A rotated arrow: degrees, 0 = up. */
dir(degrees: number): SigMark;
/** Signal bars, 0..4. */
bars(n: number): SigMark;
glyph(name: IconName): SigMark;
};
declare const prims: Record<string, (...a: Arg[]) => HTMLElement>;
declare const iconBoxes: Record<string, [number, number, number, number]>;
declare const BOX_OPS: Readonly<Record<BoxOp, readonly BoxSegment[]>>;
declare const STATE_WIRE: Record<State, string>;
declare const TREND_WIRE: Record<Trend, string>;
declare const Z: Record<CellName, Cell>;
declare const root: HTMLElement;
declare const fontsReady: Promise<void>;
declare const regSnap: { defs: Map<string, ScreenState>; current: string | null; home: string | null } | null;
declare const cadence: Record<Tier, number>;
declare const Minute: number;
declare const inlineFaces: Promise<string> | null;
declare const K: {
  air: { eaqi: Key<number>; main: Key<Pollutant>; no2: Key<number>; o3: Key<number>; pm10: Key<number>; pm25: Key<number> };
  cal: { events: Key<CalEvent[]> };
  carbon: { best: Key<Span | null>; green: Key<number>; index: Key<CarbonIndex>; mix: Key<MixShare[]>; now: Key<number>; region: Key<string>; slots: Key<Intensity[]> };
  claude: { at: Key<Ms>; heard: Key<string>; say: Key<string>; state: Key<Presence> };
  cpu: { cores: Key<Record<string, number[]>>; count: Key<number>; hist: Key<number[]>; total: Key<number> };
  disk: { hist: Key<number[]>; pct: Key<number>; total: Key<number>; used: Key<number> };
  link: { server: Key<number> };
  mem: { apps: Key<number>; cache: Key<number>; free: Key<number>; pct: Key<number>; swapTotal: Key<number>; swapUsed: Key<number>; total: Key<number> };
  moon: { phase: Key<number> };
  net: { down: Key<number>; downHist: Key<number[]>; iface: Key<string | null>; ip: Key<string | null>; up: Key<number>; upHist: Key<number[]> };
  notes: { items: Key<Note[]> };
  power: { avg: Key<number | null>; cheap: Key<Span | null>; kind: Key<RateKind>; latest: Key<Ms | null>; max: Key<Peak | null>; min: Key<Peak | null>; nextSlots: Key<Rate[]>; rate: Key<number>; slots: Key<Rate[]>; standing: Key<number>; tariff: Key<string>; updated: Key<Ms>; usageCost: Key<number>; usageDay: Key<Ms | null>; usageKwh: Key<number>; usageSlots: Key<number[]>; week: Key<DayUsage[]> };
  proc: { count: Key<number>; top: Key<Proc[]> };
  shop: { items: Key<Grocery[]> };
  sun: { rise: Key<Ms>; set: Key<Ms> };
  sys: { host: Key<string>; kernel: Key<string>; load: Key<number>; load15: Key<number>; load5: Key<number>; loadHist: Key<LoadHistory>; temp: Key<number>; uptime: Key<number> };
  time: { minute: Key<Ms> };
  timer: { end: Key<Ms | null>; label: Key<string>; total: Key<number> };
  ui: { hold: Key<number>; screen: Key<string> };
  wifi: { bars: Key<number>; rssi: Key<number | null>; ssid: Key<string | null> };
  wx: { cloud: Key<number>; feels: Key<number>; hours: Key<Hour[]>; pressure: Key<number>; rainFrom: Key<Ms | null>; sky: Key<Sky>; temp: Key<number>; uvDay: Key<number[]>; uvMax: Key<number>; uvNow: Key<number>; wind: Key<number>; windDir: Key<number> };
};
declare const ui: {
  root: any;
  screen: any;
  h: typeof h;
  rev(): any;
  mount: typeof mount;
  outline: typeof outline;
  eink: typeof eink;
  dither: typeof dither;
  rasterize: typeof rasterize;
  snapshot: typeof snapshot;
  show(...nodes: Child[]): any;
  clear(): any;
  settled(): any;
  ready(): any;
  accent(color: string | null): any;
  icons(): any;
  fits: typeof fits;
  frame: typeof frame;
  begin: typeof begin;
  verify: typeof verify;
  rollback: typeof rollback;
  tx: typeof tx;
  every: typeof every;
  stop: typeof stop;
  timers: typeof timers;
  beats(): any;
  cadence: any;
  hold(): any;
  resume(): any;
  set: any;
  get: typeof get;
  age: any;
  data(): any;
  key: any;
  keys: any;
  connect: typeof connect;
  push: typeof push;
  run: typeof run;
  link(): Link;
  navigation(on: boolean): any;
  defineScreen: typeof defineScreen;
  go: typeof go;
  screenOf(id: string): ScreenHandle | undefined;
  screenIds(): any;
  current(): any;
  selfTest: typeof selfTest;
  frames: typeof frames;
  docs: typeof docs;
  size(w: number, ht: number): any;
  css(text: string): any;
  help(): any;
};
