// ui.ts — display-only UI kit. Real DOM nodes plus a few conveniences.
// Erasable TypeScript only (no enums/namespaces/param-properties) so any type stripper works.

type Child = Node | string | number | false | null | undefined | Child[];
type Props = { [k: string]: any };
type Arg = Child | Props;
type Replacement = Child;

declare global {
  interface Element {
    readonly words: Lens;
    readonly chars: Lens;
    readonly sentences: Lens;
    text: string | Child;
    readonly child: Element | null;
    readonly kids: Element[];
    readonly up: Element | null;
    readonly next: Element | null;
    readonly prev: Element | null;
    add(...args: Arg[]): this;
    set(props: Props): this;
    q(sel: string): Element | null;
    qa(sel: string): Element[];
    clear(): this;
  }
}

const isNode = (x: unknown): x is Node => x instanceof Node;
const isPlain = (x: unknown): x is Props =>
  !!x && typeof x === "object" && Object.getPrototypeOf(x) === Object.prototype;

// ---------------------------------------------------------------- elements

function setProps(el: Element, props: Props): void {
  for (const [k, v] of Object.entries(props)) {
    if (k === "name") (el as HTMLElement).dataset.name = v;
    else if (k === "priority") (el as HTMLElement).dataset.priority = String(v);
    else if (k === "class" || k === "cls")
      el.classList.add(...String(v).split(/\s+/).filter(Boolean));
    else if (k === "style") {
      const st = (el as HTMLElement).style;
      if (typeof v === "string") st.cssText += ";" + v;
      else
        for (const [sk, sv] of Object.entries(v))
          sk.startsWith("--") ? st.setProperty(sk, String(sv)) : ((st as any)[sk] = sv);
    } else if (k === "text") el.textContent = v;
    else if (k === "html") el.innerHTML = v;
    else if (typeof v === "function") (el as any)[k] = v;
    else if (v === false || v == null) el.removeAttribute(k);
    else el.setAttribute(k, v === true ? "" : String(v));
  }
}

function applyArg(el: Element, arg: Arg): void {
  if (arg == null || arg === false) return;
  if (Array.isArray(arg)) arg.forEach((a) => applyArg(el, a));
  else if (isNode(arg)) el.append(arg);
  else if (isPlain(arg)) setProps(el, arg);
  else el.append(String(arg));
}

function h<K extends keyof HTMLElementTagNameMap>(tag: K, ...args: Arg[]): HTMLElementTagNameMap[K];
function h(tag: string, ...args: Arg[]): HTMLElement;
function h(tag: string, ...args: Arg[]): HTMLElement {
  const el = document.createElement(tag);
  args.forEach((a) => applyArg(el, a));
  return el;
}

const frag = (...vals: Child[]): DocumentFragment => {
  const f = document.createDocumentFragment();
  for (const v of vals.flat(Infinity as 1) as Child[])
    if (v != null && v !== false) f.append(isNode(v) ? v : String(v));
  return f;
};

// ------------------------------------------------------------------ lenses
// A lens is a view over the *flattened text* of an element. Reading gives
// strings; writing splices the underlying DOM (splitting text nodes as needed).

class Lens {
  [i: number]: any;
  private root: Element;
  private re: RegExp;
  constructor(root: Element, re: RegExp) {
    this.root = root;
    this.re = re;
    return new Proxy(this, {
      get: (t, k, r) =>
        typeof k === "string" && /^-?\d+$/.test(k) ? t.at(+k) : Reflect.get(t, k, r),
      set: (t, k, v) => {
        if (typeof k === "string" && /^-?\d+$/.test(k)) {
          const i = t.norm(+k);
          t.replace(i, i + 1, v);
          return true;
        }
        return Reflect.set(t, k, v);
      },
    });
  }

  private scan() {
    const nodes: Text[] = [];
    const w = document.createTreeWalker(this.root, NodeFilter.SHOW_TEXT);
    for (let n = w.nextNode(); n; n = w.nextNode()) nodes.push(n as Text);
    const text = nodes.map((n) => n.data).join("");
    const toks = [...text.matchAll(this.re)].map((m) => ({
      s: m.index!,
      e: m.index! + m[0].length,
      t: m[0],
    }));
    return { nodes, toks };
  }

  private locate(nodes: Text[], off: number, isEnd: boolean): [Text, number] {
    for (const n of nodes) {
      if (isEnd ? off <= n.length : off < n.length) return [n, off];
      off -= n.length;
    }
    const last = nodes[nodes.length - 1];
    return [last, last.length];
  }

  private norm(i: number, len = this.length) {
    return i < 0 ? len + i : i;
  }

  private range(i: number, j: number) {
    const { nodes, toks } = this.scan();
    i = this.norm(i, toks.length);
    j = this.norm(j, toks.length);
    if (i < 0 || j > toks.length || i >= j)
      throw new RangeError(`no such range ${i}..${j} (length ${toks.length})`);
    const r = document.createRange();
    r.setStart(...this.locate(nodes, toks[i].s, false));
    r.setEnd(...this.locate(nodes, toks[j - 1].e, true));
    return r;
  }

  get length(): number {
    return this.scan().toks.length;
  }
  at(i: number): string | undefined {
    const { toks } = this.scan();
    return toks[this.norm(i, toks.length)]?.t;
  }
  slice(i = 0, j = this.length): string[] {
    return this.scan().toks.slice(i, j).map((t) => t.t);
  }
  toArray(): string[] {
    return this.slice();
  }
  toString(): string {
    return this.toArray().join(" ");
  }
  indexOf(x: string | RegExp): number {
    return this.scan().toks.findIndex((t) => (typeof x === "string" ? t.t === x : x.test(t.t)));
  }
  *[Symbol.iterator]() {
    yield* this.toArray();
  }

  /** Replace tokens i..j (exclusive) with a string / node / array of them. */
  replace(i: number, j: number, val: Replacement): this {
    const r = this.range(i, j);
    // elements the range empties out would otherwise linger as <b></b> husks
    const touched = [...this.root.querySelectorAll("*")].filter((e) => e.hasChildNodes() && r.intersectsNode(e));
    r.deleteContents();
    for (const e of touched.reverse())
      if (e.isConnected && !e.textContent && !e.firstElementChild) e.remove();
    r.insertNode(frag(val));
    this.root.normalize();
    return this;
  }
  /** Wrap tokens i..j in a tag name or a component fn: words.wrap(1, bold) */
  wrap(i: number, how: string | ((c: DocumentFragment) => Node), j = i + 1): Element {
    const r = this.range(i, j);
    const c = r.extractContents();
    const el = (typeof how === "string" ? h(how, c) : how(c)) as Element;
    r.insertNode(el);
    this.root.normalize();
    return el;
  }
  /** Insert before token i (i === length appends after the last token), space-separated. */
  insert(i: number, val: Replacement): this {
    const { nodes, toks } = this.scan();
    i = this.norm(i, toks.length);
    const r = document.createRange();
    const atEnd = i >= toks.length;
    if (atEnd) r.setStart(...this.locate(nodes, toks[toks.length - 1].e, true));
    else r.setStart(...this.locate(nodes, toks[i].s, false));
    r.collapse(true);
    r.insertNode(atEnd ? frag(" ", val) : frag(val, " "));
    this.root.normalize();
    return this;
  }
  remove(i: number, j = i + 1): this {
    return this.replace(i, j, "");
  }
}

// ------------------------------------------------- Element conveniences

const def = (name: string, desc: PropertyDescriptor) =>
  Object.defineProperty(Element.prototype, name, { configurable: true, ...desc });

def("words", { get(this: Element) { return new Lens(this, /\S+/g); } });
def("chars", { get(this: Element) { return new Lens(this, /\S/g); } });
def("sentences", { get(this: Element) { return new Lens(this, /[^.!?\s][^.!?]*[.!?]*/g); } });
def("text", {
  get(this: Element) { return this.textContent; },
  set(this: Element, v: Child) { this.replaceChildren(frag(v)); },
});
def("child", { get(this: Element) { return this.firstElementChild; } });
def("kids", { get(this: Element) { return [...this.children]; } });
def("up", { get(this: Element) { return this.parentElement; } });
def("next", { get(this: Element) { return this.nextElementSibling; } });
def("prev", { get(this: Element) { return this.previousElementSibling; } });
def("add", { value(this: Element, ...a: Arg[]) { a.forEach((x) => applyArg(this, x)); return this; } });
def("set", { value(this: Element, p: Props) { setProps(this, p); return this; } });
def("q", { value(this: Element, s: string) { return this.querySelector(s); } });
def("qa", { value(this: Element, s: string) { return [...this.querySelectorAll(s)]; } });
def("clear", { value(this: Element) { this.replaceChildren(); return this; } });

// ------------------------------------------------------------ primitives

const tags = [
  "div", "span", "p", "section", "article", "header", "footer", "main", "aside", "nav",
  "h1", "h2", "h3", "h4", "ul", "ol", "li", "table", "thead", "tbody", "tr", "td", "th",
  "em", "strong", "small", "code", "pre", "mark", "sup", "sub", "br", "hr", "img",
  "blockquote", "figure", "figcaption",
] as const;

const prims: Record<string, (...a: Arg[]) => HTMLElement> = {};
for (const t of tags) prims[t] = (...a) => h(t, ...a);
prims.bold = (...a) => h("b", ...a);
prims.italic = (...a) => h("i", ...a);

// ------------------------------------------------------------ components

const row = (...a: Arg[]) => h("div", { class: "row" }, ...a);
const col = (...a: Arg[]) => h("div", { class: "col" }, ...a);
const grid = (cols: number, ...a: Arg[]) => h("div", { class: "grid", style: { "--cols": cols } }, ...a);
const box = (...a: Arg[]) => h("section", { class: "card" }, ...a);
const card = (title: string, ...a: Arg[]) => box(h("div", { class: "card-title" }, title), ...a);
const big = (...a: Arg[]) => h("div", { class: "big" }, ...a);
const heading = (text: Child, level: 1 | 2 | 3 | 4 = 1) => h(`h${level}`, text);
const divider = () => h("hr");
const badge = (text: Child) => h("span", { class: "badge" }, text);

const stat = (label: string, value: Child, o: { unit?: string; delta?: Child } = {}) =>
  h("div", { class: "stat" },
    h("div", { class: "label" }, label),
    h("div", { class: "value" }, value, o.unit && h("small", o.unit)),
    o.delta && h("div", { class: "delta" }, o.delta));

const list = (items: Child[], o: { ordered?: boolean } = {}) =>
  h(o.ordered ? "ol" : "ul", items.map((i) => h("li", i)));

const table = (rows: Child[][], o: { head?: Child[] } = {}) =>
  h("table",
    o.head && h("thead", h("tr", o.head.map((c) => h("th", c)))),
    h("tbody", rows.map((r) => h("tr", r.map((c) => h("td", c))))));

const kv = (obj: Record<string, Child>) =>
  h("div", { class: "kvs" },
    Object.entries(obj).map(([k, v]) => h("div", { class: "kv" }, h("span", { class: "k" }, k), h("span", { class: "v" }, v))));

const progress = (value: number, label?: string) =>
  h("div", { class: "progress" },
    label && h("div", { class: "label" }, label),
    h("div", { class: "track" }, h("div", { class: "fill", style: { width: `${Math.round(Math.max(0, Math.min(1, value)) * 100)}%` } })));

const bars = (data: (number | [string, number])[], o: { height?: number; values?: boolean } = {}) => {
  const items = data.map((d) => (typeof d === "number" ? ["", d] as [string, number] : d));
  const max = Math.max(...items.map((i) => i[1]), 1e-9);
  return h("div", { class: "bars", style: { height: `${o.height ?? 120}px` } },
    items.map(([label, v]) =>
      h("div", { class: "bar" },
        h("div", { class: "fill", style: { height: `${(v / max) * 100}%` } }),
        (label || o.values) && h("div", { class: "bar-label" }, o.values && h("b", { style: "display:block" }, v), label))));
};

const sparkline = (values: number[], o: { w?: number; h?: number } = {}) => {
  const w = o.w ?? 160, ht = o.h ?? 40;
  const min = Math.min(...values), max = Math.max(...values), span = max - min || 1;
  const pts = values.map((v, i) => `${(i / Math.max(values.length - 1, 1)) * w},${ht - 2 - ((v - min) / span) * (ht - 4)}`).join(" ");
  const wrap = document.createElement("span");
  wrap.className = "spark";
  wrap.innerHTML = `<svg width="${w}" height="${ht}" viewBox="0 0 ${w} ${ht}"><polyline points="${pts}" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linejoin="round"/></svg>`;
  return wrap;
};

const agenda = (events: { time: string; title: string; note?: string }[]) =>
  h("div", { class: "agenda" },
    events.map((e) =>
      h("div", { class: "event" },
        h("span", { class: "time" }, e.time),
        h("span", { class: "what" }, e.title, e.note && h("small", e.note)))));

const weather = (o: { temp: string; cond: string; icon?: string; hi?: string; lo?: string }) =>
  h("div", { class: "weather" },
    weatherIcon(o.icon ?? "clear", 88),
    h("div", { class: "now" }, h("div", { class: "temp" }, o.temp), h("div", { class: "cond" }, o.cond)),
    (o.hi || o.lo) && h("div", { class: "range" }, o.hi && `H ${o.hi}`, o.hi && o.lo && " · ", o.lo && `L ${o.lo}`));

/** Tiny markdown: #/##/### headings, - lists, **bold**, *italic*, `code`. */
const md = (src: string) => {
  const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;");
  const inline = (s: string) =>
    esc(s).replace(/`([^`]+)`/g, "<code>$1</code>").replace(/\*\*([^*]+)\*\*/g, "<b>$1</b>").replace(/\*([^*]+)\*/g, "<i>$1</i>");
  const out: string[] = [];
  let inList = false;
  for (const line of src.split("\n")) {
    const li = /^\s*[-*]\s+(.*)/.exec(line);
    if (li) { if (!inList) out.push("<ul>"); inList = true; out.push(`<li>${inline(li[1])}</li>`); continue; }
    if (inList) { out.push("</ul>"); inList = false; }
    const hd = /^(#{1,3})\s+(.*)/.exec(line);
    if (hd) out.push(`<h${hd[1].length}>${inline(hd[2])}</h${hd[1].length}>`);
    else if (line.trim()) out.push(`<p>${inline(line)}</p>`);
  }
  if (inList) out.push("</ul>");
  return h("div", { class: "md", html: out.join("") });
};

// ------------------------------------------------------ more components

// Icons: 24x24 grid, rounded strokes, drawn heavy so they survive 1-bit dithering.
const circ = (cx: number, cy: number, r: number) => `M${cx - r} ${cy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0z`;
// Icons: Lucide v0.469.0 (https://lucide.dev, ISC licence), 24x24, stroked. Each value is the inner SVG markup.
const ICON_PATHS: Record<string, string> = {
  sun: "<circle cx=\"12\" cy=\"12\" r=\"4\"/> <path d=\"M12 2v2\"/> <path d=\"M12 20v2\"/> <path d=\"m4.93 4.93 1.41 1.41\"/> <path d=\"m17.66 17.66 1.41 1.41\"/> <path d=\"M2 12h2\"/> <path d=\"M20 12h2\"/> <path d=\"m6.34 17.66-1.41 1.41\"/> <path d=\"m19.07 4.93-1.41 1.41\"/>",
  moon: "<path d=\"M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z\"/>",
  cloud: "<path d=\"M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z\"/>",
  rain: "<path d=\"M4 14.899A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 2.5 8.242\"/> <path d=\"M16 14v6\"/> <path d=\"M8 14v6\"/> <path d=\"M12 16v6\"/>",
  snow: "<path d=\"m10 20-1.25-2.5L6 18\"/> <path d=\"M10 4 8.75 6.5 6 6\"/> <path d=\"m14 20 1.25-2.5L18 18\"/> <path d=\"m14 4 1.25 2.5L18 6\"/> <path d=\"m17 21-3-6h-4\"/> <path d=\"m17 3-3 6 1.5 3\"/> <path d=\"M2 12h6.5L10 9\"/> <path d=\"m20 10-1.5 2 1.5 2\"/> <path d=\"M22 12h-6.5L14 15\"/> <path d=\"m4 10 1.5 2L4 14\"/> <path d=\"m7 21 3-6-1.5-3\"/> <path d=\"m7 3 3 6h4\"/>",
  bolt: "<path d=\"M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z\"/>",
  wind: "<path d=\"M12.8 19.6A2 2 0 1 0 14 16H2\"/> <path d=\"M17.5 8a2.5 2.5 0 1 1 2 4H2\"/> <path d=\"M9.8 4.4A2 2 0 1 1 11 8H2\"/>",
  fog: "<path d=\"M4 14.899A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 2.5 8.242\"/> <path d=\"M16 17H7\"/> <path d=\"M17 21H9\"/>",
  drop: "<path d=\"M12 22a7 7 0 0 0 7-7c0-2-1-3.9-3-5.5s-3.5-4-4-6.5c-.5 2.5-2 4.9-4 6.5C6 11.1 5 13 5 15a7 7 0 0 0 7 7z\"/>",
  thermo: "<path d=\"M14 4v10.54a4 4 0 1 1-4 0V4a2 2 0 0 1 4 0Z\"/>",
  umbrella: "<path d=\"M22 12a10.06 10.06 1 0 0-20 0Z\"/> <path d=\"M12 12v8a2 2 0 0 0 4 0\"/> <path d=\"M12 2v1\"/>",
  home: "<path d=\"M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8\"/> <path d=\"M3 10a2 2 0 0 1 .709-1.528l7-5.999a2 2 0 0 1 2.582 0l7 5.999A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z\"/>",
  door: "<path d=\"M13 4h3a2 2 0 0 1 2 2v14\"/> <path d=\"M2 20h3\"/> <path d=\"M13 20h9\"/> <path d=\"M10 12v.01\"/> <path d=\"M13 4.562v16.157a1 1 0 0 1-1.242.97L5 20V5.562a2 2 0 0 1 1.515-1.94l4-1A2 2 0 0 1 13 4.561Z\"/>",
  lock: "<rect width=\"18\" height=\"11\" x=\"3\" y=\"11\" rx=\"2\" ry=\"2\"/> <path d=\"M7 11V7a5 5 0 0 1 10 0v4\"/>",
  tv: "<rect width=\"20\" height=\"15\" x=\"2\" y=\"7\" rx=\"2\" ry=\"2\"/> <polyline points=\"17 2 12 7 7 2\"/>",
  bulb: "<path d=\"M15 14c.2-1 .7-1.7 1.5-2.5 1-.9 1.5-2.2 1.5-3.5A6 6 0 0 0 6 8c0 1 .2 2.2 1.5 3.5.7.7 1.3 1.5 1.5 2.5\"/> <path d=\"M9 18h6\"/> <path d=\"M10 22h4\"/>",
  plug: "<path d=\"M12 22v-5\"/> <path d=\"M9 8V2\"/> <path d=\"M15 8V2\"/> <path d=\"M18 8v5a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V8Z\"/>",
  wifi: "<path d=\"M12 20h.01\"/> <path d=\"M2 8.82a15 15 0 0 1 20 0\"/> <path d=\"M5 12.859a10 10 0 0 1 14 0\"/> <path d=\"M8.5 16.429a5 5 0 0 1 7 0\"/>",
  battery: "<rect width=\"16\" height=\"10\" x=\"2\" y=\"7\" rx=\"2\" ry=\"2\"/> <line x1=\"22\" x2=\"22\" y1=\"11\" y2=\"13\"/> <line x1=\"6\" x2=\"6\" y1=\"11\" y2=\"13\"/> <line x1=\"10\" x2=\"10\" y1=\"11\" y2=\"13\"/>",
  cpu: "<rect width=\"16\" height=\"16\" x=\"4\" y=\"4\" rx=\"2\"/> <rect width=\"6\" height=\"6\" x=\"9\" y=\"9\" rx=\"1\"/> <path d=\"M15 2v2\"/> <path d=\"M15 20v2\"/> <path d=\"M2 15h2\"/> <path d=\"M2 9h2\"/> <path d=\"M20 15h2\"/> <path d=\"M20 9h2\"/> <path d=\"M9 2v2\"/> <path d=\"M9 20v2\"/>",
  disk: "<line x1=\"22\" x2=\"2\" y1=\"12\" y2=\"12\"/> <path d=\"M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z\"/> <line x1=\"6\" x2=\"6.01\" y1=\"16\" y2=\"16\"/> <line x1=\"10\" x2=\"10.01\" y1=\"16\" y2=\"16\"/>",
  memory: "<path d=\"M6 19v-3\"/> <path d=\"M10 19v-3\"/> <path d=\"M14 19v-3\"/> <path d=\"M18 19v-3\"/> <path d=\"M8 11V9\"/> <path d=\"M16 11V9\"/> <path d=\"M12 11V9\"/> <path d=\"M2 15h20\"/> <path d=\"M2 7a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v1.1a2 2 0 0 0 0 3.837V17a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-5.1a2 2 0 0 0 0-3.837Z\"/>",
  phone: "<rect width=\"14\" height=\"20\" x=\"5\" y=\"2\" rx=\"2\" ry=\"2\"/> <path d=\"M12 18h.01\"/>",
  car: "<path d=\"M19 17h2c.6 0 1-.4 1-1v-3c0-.9-.7-1.7-1.5-1.9C18.7 10.6 16 10 16 10s-1.3-1.4-2.2-2.3c-.5-.4-1.1-.7-1.8-.7H5c-.6 0-1.1.4-1.4.9l-1.4 2.9A3.7 3.7 0 0 0 2 12v4c0 .6.4 1 1 1h2\"/> <circle cx=\"7\" cy=\"17\" r=\"2\"/> <path d=\"M9 17h6\"/> <circle cx=\"17\" cy=\"17\" r=\"2\"/>",
  bus: "<path d=\"M8 6v6\"/> <path d=\"M15 6v6\"/> <path d=\"M2 12h19.6\"/> <path d=\"M18 18h3s.5-1.7.8-2.8c.1-.4.2-.8.2-1.2 0-.4-.1-.8-.2-1.2l-1.4-5C20.1 6.8 19.1 6 18 6H4a2 2 0 0 0-2 2v10h3\"/> <circle cx=\"7\" cy=\"18\" r=\"2\"/> <path d=\"M9 18h5\"/> <circle cx=\"16\" cy=\"18\" r=\"2\"/>",
  bag: "<path d=\"M6 2 3 6v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6l-3-4Z\"/> <path d=\"M3 6h18\"/> <path d=\"M16 10a4 4 0 0 1-8 0\"/>",
  cup: "<path d=\"M10 2v2\"/> <path d=\"M14 2v2\"/> <path d=\"M16 8a1 1 0 0 1 1 1v8a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4V9a1 1 0 0 1 1-1h14a4 4 0 1 1 0 8h-1\"/> <path d=\"M6 2v2\"/>",
  utensils: "<path d=\"M3 2v7c0 1.1.9 2 2 2h4a2 2 0 0 0 2-2V2\"/> <path d=\"M7 2v20\"/> <path d=\"M21 15V2a5 5 0 0 0-5 5v6c0 1.1.9 2 2 2h3Zm0 0v7\"/>",
  book: "<path d=\"M12 7v14\"/> <path d=\"M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z\"/>",
  mail: "<rect width=\"20\" height=\"16\" x=\"2\" y=\"4\" rx=\"2\"/> <path d=\"m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7\"/>",
  calendar: "<path d=\"M8 2v4\"/> <path d=\"M16 2v4\"/> <rect width=\"18\" height=\"18\" x=\"3\" y=\"4\" rx=\"2\"/> <path d=\"M3 10h18\"/>",
  clock: "<circle cx=\"12\" cy=\"12\" r=\"10\"/> <polyline points=\"12 6 12 12 16 14\"/>",
  bell: "<path d=\"M10.268 21a2 2 0 0 0 3.464 0\"/> <path d=\"M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326\"/>",
  trash: "<path d=\"M3 6h18\"/> <path d=\"M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6\"/> <path d=\"M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2\"/> <line x1=\"10\" x2=\"10\" y1=\"11\" y2=\"17\"/> <line x1=\"14\" x2=\"14\" y1=\"11\" y2=\"17\"/>",
  flag: "<path d=\"M4 15s1-1 4-1 5 2 8 2 4-1 4-1V3s-1 1-4 1-5-2-8-2-4 1-4 1z\"/> <line x1=\"4\" x2=\"4\" y1=\"22\" y2=\"15\"/>",
  pin: "<path d=\"M20 10c0 4.993-5.539 10.193-7.399 11.799a1 1 0 0 1-1.202 0C9.539 20.193 4 14.993 4 10a8 8 0 0 1 16 0\"/> <circle cx=\"12\" cy=\"10\" r=\"3\"/>",
  user: "<path d=\"M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2\"/> <circle cx=\"12\" cy=\"7\" r=\"4\"/>",
  pulse: "<path d=\"M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2\"/>",
  wave: "<path d=\"M2 10v3\"/> <path d=\"M6 6v11\"/> <path d=\"M10 3v18\"/> <path d=\"M14 8v7\"/> <path d=\"M18 5v13\"/> <path d=\"M22 10v3\"/>",
  leaf: "<path d=\"M11 20A7 7 0 0 1 9.8 6.1C15.5 5 17 4.48 19 2c1 2 2 4.18 2 8 0 5.5-4.78 10-10 10Z\"/> <path d=\"M2 21c0-3 1.85-5.36 5.08-6C9.5 14.52 12 13 13 12\"/>",
  star: "<path d=\"M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z\"/>",
  check: "<path d=\"M20 6 9 17l-5-5\"/>",
  cross: "<path d=\"M18 6 6 18\"/> <path d=\"m6 6 12 12\"/>",
  warn: "<path d=\"m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3\"/> <path d=\"M12 9v4\"/> <path d=\"M12 17h.01\"/>",
  info: "<circle cx=\"12\" cy=\"12\" r=\"10\"/> <path d=\"M12 16v-4\"/> <path d=\"M12 8h.01\"/>",
  sparkle: "<path d=\"M12 6v12\"/> <path d=\"M17.196 9 6.804 15\"/> <path d=\"m6.804 9 10.392 6\"/>",
  hub: "<circle cx=\"12\" cy=\"4.5\" r=\"2.5\"/> <path d=\"m10.2 6.3-3.9 3.9\"/> <circle cx=\"4.5\" cy=\"12\" r=\"2.5\"/> <path d=\"M7 12h10\"/> <circle cx=\"19.5\" cy=\"12\" r=\"2.5\"/> <path d=\"m13.8 17.7 3.9-3.9\"/> <circle cx=\"12\" cy=\"19.5\" r=\"2.5\"/>",
  nodes: "<circle cx=\"18\" cy=\"5\" r=\"3\"/> <circle cx=\"6\" cy=\"12\" r=\"3\"/> <circle cx=\"18\" cy=\"19\" r=\"3\"/> <line x1=\"8.59\" x2=\"15.42\" y1=\"13.51\" y2=\"17.49\"/> <line x1=\"15.41\" x2=\"8.59\" y1=\"6.51\" y2=\"10.49\"/>",
  peers: "<path d=\"M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2\"/> <circle cx=\"9\" cy=\"7\" r=\"4\"/> <path d=\"M22 21v-2a4 4 0 0 0-3-3.87\"/> <path d=\"M16 3.13a4 4 0 0 1 0 7.75\"/>",
  arrow: "<path d=\"M5 12h14\"/> <path d=\"m12 5 7 7-7 7\"/>",
  chevron: "<path d=\"m9 18 6-6-6-6\"/>",
  sunrise: "<path d=\"M12 2v8\"/> <path d=\"m4.93 10.93 1.41 1.41\"/> <path d=\"M2 18h2\"/> <path d=\"M20 18h2\"/> <path d=\"m19.07 10.93-1.41 1.41\"/> <path d=\"M22 22H2\"/> <path d=\"m8 6 4-4 4 4\"/> <path d=\"M16 18a4 4 0 0 0-8 0\"/>",
  sunset: "<path d=\"M12 10V2\"/> <path d=\"m4.93 10.93 1.41 1.41\"/> <path d=\"M2 18h2\"/> <path d=\"M20 18h2\"/> <path d=\"m19.07 10.93-1.41 1.41\"/> <path d=\"M22 22H2\"/> <path d=\"m16 6-4 4-4-4\"/> <path d=\"M16 18a4 4 0 0 0-8 0\"/>",
  partly: "<path d=\"M12 2v2\"/> <path d=\"m4.93 4.93 1.41 1.41\"/> <path d=\"M20 12h2\"/> <path d=\"m19.07 4.93-1.41 1.41\"/> <path d=\"M15.947 12.65a4 4 0 0 0-5.925-4.128\"/> <path d=\"M13 22H7a5 5 0 1 1 4.9-6H13a3 3 0 0 1 0 6Z\"/>",
  cloudsnow: "<path d=\"M4 14.899A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 2.5 8.242\"/> <path d=\"M8 15h.01\"/> <path d=\"M8 19h.01\"/> <path d=\"M12 17h.01\"/> <path d=\"M12 21h.01\"/> <path d=\"M16 15h.01\"/> <path d=\"M16 19h.01\"/>",
  storm: "<path d=\"M6 16.326A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 .5 8.973\"/> <path d=\"m13 12-3 5h4l-3 5\"/>",
  // custom marks (not from the set)
  "sigil-h": '<path d="M3 3h18v18H3zM3 12h18"/>',
  "sigil-v": '<path d="M3 3h18v18H3zM12 3v18"/>',
  "sigil-c": '<path d="M3 3h18v18H3zM8 8v8h8"/>',
  plusbox: '<path d="M4 4h16v16H4zM12 8v8M8 12h8"/>',
};
const ICON_FILLED = new Set<string>(); // line art only; pass {fill:true} for a solid one

const svgIcon = (inner: string, size: number | string, stroke = 1.5) => {
  const el = h("span", { class: "icon" });
  el.innerHTML = `<svg viewBox="0 0 24 24" width="${size}" height="${size}" fill="none" stroke="currentColor" stroke-width="${stroke}" stroke-linecap="round" stroke-linejoin="round">${inner}</svg>`;
  return el;
};

/** icon("sun") ... a named icon (size defaults to the surrounding font size). Unknown names render as text, so any glyph works. */
// Icons are drawn on different footprints (a battery is wide, a book tall). So each is measured once and
// scaled to the same live area, centred, with the stroke compensated: they all read at the same size and weight.
const iconBoxes: Record<string, [number, number, number, number]> = {};
let measurer: SVGSVGElement | null = null;
const iconBox = (name: string): [number, number, number, number] => {
  if (iconBoxes[name]) return iconBoxes[name];
  if (!measurer) {
    measurer = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    measurer.setAttribute("style", "position:absolute;left:-9999px;top:0;width:24px;height:24px");
    document.body.append(measurer);
  }
  const p = document.createElementNS("http://www.w3.org/2000/svg", "g");
  p.innerHTML = ICON_PATHS[name];
  measurer.append(p);
  const b = p.getBBox();
  p.remove();
  return (iconBoxes[name] = [b.x, b.y, b.width, b.height]);
};

const icon = (name: string, o: { size?: number; fill?: boolean; stroke?: number } = {}) => {
  const d = ICON_PATHS[name];
  if (!d) return h("span", { class: "icon glyph", style: o.size ? { fontSize: `${o.size}px` } : "" }, name);
  const filled = o.fill ?? ICON_FILLED.has(name);
  const [bx, by, bw, bh] = iconBox(name), live = 20, sc = Math.min(live / Math.max(bw, bh, 0.01), 1.5);
  const g = `<g transform="translate(12 12) scale(${sc.toFixed(3)}) translate(${(-(bx + bw / 2)).toFixed(2)} ${(-(by + bh / 2)).toFixed(2)})"${filled ? ' fill="currentColor"' : ""}>${d}</g>`;
  return svgIcon(g, o.size ?? "1em", (o.stroke ?? 1.5) / sc);
};

const rays = (cx: number, cy: number, r1: number, r2: number, n = 8) =>
  Array.from({ length: n }, (_, i) => {
    const a = (i * 2 * Math.PI) / n, c = Math.cos(a), s = Math.sin(a);
    return `M${(cx + r1 * c).toFixed(2)} ${(cy + r1 * s).toFixed(2)}L${(cx + r2 * c).toFixed(2)} ${(cy + r2 * s).toFixed(2)}`;
  }).join("");

/** Vertical hatching clipped to a disc: the "grey" of a drawing. x runs from..to across a circle at (cx, cy). */
const hatchDisc = (cx: number, cy: number, r: number, from: number, to: number, step = 1.7) => {
  let d = "";
  for (let x = from; x <= to + 1e-6; x += step) {
    const dx = x - cx, k = r * r - dx * dx;
    if (k <= 0) continue;
    const dy = Math.sqrt(k);
    d += `M${x.toFixed(2)} ${(cy - dy).toFixed(2)}V${(cy + dy).toFixed(2)}`;
  }
  return d;
};

/** Weather symbols from the icon set: clear, night, partly, cloudy, rain, snow, storm, fog, wind (hairline at large sizes). */
const weatherIcon = (kind: string, size = 84) => {
  const name: Record<string, string> = { clear: "sun", sun: "sun", night: "moon", moon: "moon", partly: "partly", cloudy: "cloud", cloud: "cloud", rain: "rain", snow: "cloudsnow", storm: "storm", bolt: "storm", fog: "fog", wind: "wind" };
  return icon(name[kind] ?? "cloud", { size, stroke: size > 40 ? 1.1 : 1.5 });
};

/** Cap dynamic text to N lines so unexpected data can't push the scene off-screen. */
const clamp = (lines: number, ...a: Arg[]) => h("div", { class: "clamp", style: { "--lines": lines } }, ...a);
const fill = (...a: Arg[]) => h("div", { class: "fill-region" }, ...a);
const spacer = () => h("div", { class: "spacer" });
const center = (...a: Arg[]) => h("div", { class: "center" }, ...a);
const chips = (items: Child[]) => h("div", { class: "chips" }, items.map((i) => badge(i)));
const topbar = (title: Child, right?: Child) =>
  h("div", { class: "topbar" }, h("div", { class: "t" }, title), right && h("div", { class: "r" }, right));
const banner = (text: Child, o: { icon?: string } = {}) =>
  h("div", { class: "banner" }, o.icon && icon(o.icon), h("span", text));
const quote = (text: Child, by?: Child) =>
  h("blockquote", { class: "quote" }, h("p", text), by && h("footer", by));
const trend = (n: number, unit = "%") =>
  h("span", { class: "trend" }, n >= 0 ? "▲" : "▼", ` ${Math.abs(n)}${unit}`);

const todo = (items: (string | { text: string; done?: boolean })[]) =>
  h("div", { class: "todo" },
    items.map((it) => {
      const o = typeof it === "string" ? { text: it, done: false } : it;
      return h("div", { class: o.done ? "item done" : "item" }, h("span", { class: "box" }, o.done && icon("check", { size: 16, stroke: 4.5 })), h("span", o.text));
    }));

const escHtml = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;");

const gauge = (value: number, label?: string, o: { size?: number; text?: string } = {}) => {
  const size = o.size ?? 120, r = size / 2 - 9, c = 2 * Math.PI * r, mid = size / 2;
  const v = Math.max(0, Math.min(1, value));
  const el = h("div", { class: "gauge" });
  el.innerHTML = `<svg width="${size}" height="${size}" viewBox="0 0 ${size} ${size}">
    <circle cx="${mid}" cy="${mid}" r="${r}" fill="none" stroke="currentColor" stroke-width="2"/>
    <circle cx="${mid}" cy="${mid}" r="${r}" fill="none" stroke="currentColor" stroke-width="14" stroke-dasharray="${v * c} ${c}" transform="rotate(-90 ${mid} ${mid})"/>
    <text x="50%" y="50%" text-anchor="middle" dominant-baseline="central" font-size="${size * ((o.text ?? "000%").length > 4 ? 0.2 : 0.26)}" font-weight="800" font-family="var(--display)" fill="currentColor">${escHtml(o.text ?? Math.round(v * 100) + "%")}</text></svg>`;
  if (label) el.append(h("div", { class: "label" }, label));
  return el;
};

const month = (date: Date = new Date(), o: { marks?: number[]; weekStart?: 0 | 1 } = {}) => {
  const y = date.getFullYear(), m = date.getMonth(), ws = o.weekStart ?? 1;
  const lead = (new Date(y, m, 1).getDay() - ws + 7) % 7;
  const days = new Date(y, m + 1, 0).getDate();
  const now = new Date();
  const isToday = (d: number) => now.getFullYear() === y && now.getMonth() === m && now.getDate() === d;
  // 2024-01-07 is a Sunday, so (i + ws) % 7 maps straight onto weekday numbers
  const dow = Array.from({ length: 7 }, (_, i) => new Date(2024, 0, 7 + ((i + ws) % 7)).toLocaleDateString([], { weekday: "narrow" }));
  return h("div", { class: "month" },
    h("div", { class: "month-title" }, date.toLocaleDateString([], { month: "long", year: "numeric" })),
    h("div", { class: "month-grid" },
      dow.map((d) => h("span", { class: "dow" }, d)),
      Array.from({ length: lead }, () => h("span")),
      Array.from({ length: days }, (_, i) =>
        h("span", { class: `day${isToday(i + 1) ? " today" : ""}${o.marks?.includes(i + 1) ? " mark" : ""}` }, i + 1))));
};

const lineChart = (values: number[], o: { w?: number; h?: number; labels?: string[] } = {}) => {
  const w = o.w ?? 360, ht = o.h ?? 140, padL = 44, padR = 10, padT = 8, padB = o.labels ? 24 : 6;
  const min = Math.min(...values), max = Math.max(...values), span = max - min || 1;
  const x = (i: number) => padL + (i / Math.max(values.length - 1, 1)) * (w - padL - padR);
  const y = (v: number) => padT + (1 - (v - min) / span) * (ht - padT - padB);
  const fmt = (n: number) => (Number.isInteger(n) ? String(n) : n.toFixed(1));
  const pts = values.map((v, i) => `${x(i)},${y(v)}`).join(" ");
  const lx = x(values.length - 1), ly = y(values[values.length - 1]);
  const el = h("div", { class: "chart" });
  el.innerHTML = `<svg width="${w}" height="${ht}" viewBox="0 0 ${w} ${ht}" font-family="inherit" font-size="14" font-weight="700" fill="currentColor">
    <path d="M${padL} ${padT}V${ht - padB}H${w - padR}" fill="none" stroke="currentColor" stroke-width="2"/>
    <text x="${padL - 6}" y="${y(max)}" text-anchor="end" dominant-baseline="hanging">${fmt(max)}</text>
    <text x="${padL - 6}" y="${y(min)}" text-anchor="end">${fmt(min)}</text>
    ${(o.labels ?? []).map((l, i) => `<text x="${x(i)}" y="${ht - 4}" text-anchor="middle">${escHtml(l)}</text>`).join("")}
    <polyline points="${pts}" fill="none" stroke="currentColor" stroke-width="3.5" stroke-linejoin="round" stroke-linecap="round"/>
    <circle cx="${lx}" cy="${ly}" r="5"/></svg>`;
  return el;
};

const image = (src: string, o: { fit?: "cover" | "contain"; w?: number | string; h?: number | string } = {}) => {
  const px = (v: number | string) => (typeof v === "number" ? `${v}px` : v);
  const style: Props = { objectFit: o.fit ?? "cover" };
  if (o.w != null) style.width = px(o.w);
  if (o.h != null) style.height = px(o.h);
  return h("img", { src, class: "image", style });
};

// ---------------------------------------------------- friendly elements

/** Claude speaking: a speech bubble with a round sparkle avatar. */
const say = (text: Child, o: { icon?: string } = {}) =>
  h("div", { class: "say" }, h("div", { class: "avatar" }, icon(o.icon ?? "sparkle", { size: 30, stroke: 2 })), h("div", { class: "bubble" }, text));

const minutes = (hm: string) => {
  const [a, b] = hm.split(":").map(Number);
  return a * 60 + b;
};

/** Sunrise to sunset arc with the sun where it is now. Times are "HH:MM". */
const sunArc = (o: { rise: string; set: string; now?: string; w?: number }) => {
  const w = o.w ?? 360, hgt = 112, pad = 22, base = hgt - 10, rx = (w - 2 * pad) / 2, ry = base - 16, cx = w / 2;
  const d = new Date();
  const now = o.now ?? `${d.getHours()}:${d.getMinutes()}`;
  const r0 = minutes(o.rise), r1 = minutes(o.set);
  const f = (minutes(now) - r0) / (r1 - r0);
  const day = f >= 0 && f <= 1;
  const pt = (t: number) => [cx - rx * Math.cos(Math.PI * t), base - ry * Math.sin(Math.PI * t)];
  const pts = (a: number, b: number) =>
    Array.from({ length: 41 }, (_, i) => pt(a + ((b - a) * i) / 40).map((v) => v.toFixed(1)).join(",")).join(" ");
  const [sx, sy] = pt(Math.max(0, Math.min(1, f)));
  const len = r1 - r0;
  const el = h("div", { class: "sunarc" });
  el.innerHTML = `<svg width="${w}" height="${hgt}" viewBox="0 0 ${w} ${hgt}" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round">
    <path d="M${pad - 8} ${base}H${w - pad + 8}" stroke-width="3"/>
    <polyline points="${pts(day ? f : 0, 1)}" stroke-width="3" stroke-dasharray="1 9"/>
    ${day ? `<polyline points="${pts(0, f)}" stroke-width="6"/>
    <circle cx="${sx.toFixed(1)}" cy="${sy.toFixed(1)}" r="13" fill="#fff" stroke="none"/>
    <circle cx="${sx.toFixed(1)}" cy="${sy.toFixed(1)}" r="9" fill="currentColor" stroke="none"/>
    <path d="${rays(+sx.toFixed(1), +sy.toFixed(1), 12, 16)}" stroke-width="3"/>` : ""}</svg>`;
  el.append(h("div", { class: "labels" },
    h("span", icon("sunrise", { size: 26 }), o.rise),
    h("small", `${Math.floor(len / 60)}h ${len % 60}m`),
    h("span", o.set, icon("sunset", { size: 26 }))));
  return el;
};

/** A strip of hours: time, weather picture, temperature, chance of rain (0..1). */
const hourly = (items: { t: string; temp: string; icon?: string; pop?: number }[]) =>
  h("div", { class: "hourly" },
    items.map((i) =>
      h("div", { class: "hr" },
        h("div", { class: "t" }, i.t),
        weatherIcon(i.icon ?? "clear", 44),
        h("div", { class: "temp" }, i.temp),
        i.pop != null && h("div", { class: "pop" }, h("i", { style: { width: `${Math.round(i.pop * 100)}%` } })))));

/** Contribution-style dot grid; levels 0..3, filled column by column. */
const habitGrid = (levels: number[], o: { rows?: number } = {}) =>
  h("div", { class: "habit", style: { "--rows": o.rows ?? 7 } },
    levels.map((l) => h("span", { class: `dot l${Math.max(0, Math.min(3, Math.round(l)))}` })));

const mmss = (s: number) => `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(Math.floor(s % 60)).padStart(2, "0")}`;
/** A countdown dial: a ring of 60 ticks, the time still to run drawn long and heavy, the elapsed ones thin. */
const timerRing = (remaining: number, total: number, label?: string) => {
  const n = 60, c = 75, frac = total ? Math.max(0, Math.min(1, remaining / total)) : 0, left = Math.round(frac * n);
  let ticks = "", elapsed = "";
  for (let i = 0; i < n; i++) {
    const a = (i / n) * 2 * Math.PI, sn = Math.sin(a), cs = -Math.cos(a), major = i % 5 === 0;
    const r0 = i < left ? (major ? 48 : 54) : 62, r1 = 68;
    const seg = `M${(c + r0 * sn).toFixed(1)} ${(c + r0 * cs).toFixed(1)}L${(c + r1 * sn).toFixed(1)} ${(c + r1 * cs).toFixed(1)}`;
    if (i < left) ticks += seg; else elapsed += seg;
  }
  const la = (left / n) * 2 * Math.PI, px = c + 74 * Math.sin(la), py = c - 74 * Math.cos(la);
  const el = h("div", { class: "timerring" });
  el.innerHTML = `<svg viewBox="0 0 150 150" fill="none" stroke="currentColor">
    <path d="${elapsed}" stroke-width="1.2"/><path d="${ticks}" stroke-width="3"/>
    <path d="M${px.toFixed(1)} ${py.toFixed(1)}l${(-5 * Math.cos(la) + 0).toFixed(1)} ${(-5 * Math.sin(la)).toFixed(1)}l${(10 * Math.cos(la)).toFixed(1)} ${(10 * Math.sin(la)).toFixed(1)}z" fill="currentColor" stroke="none"/>
    <text x="75" y="84" text-anchor="middle" fill="currentColor" stroke="none" style="font:700 38px var(--cond)">${mmss(remaining)}</text>
    <text x="75" y="101" text-anchor="middle" fill="currentColor" stroke="none" style="font:600 9px var(--mono);letter-spacing:.12em">${escHtml((label ?? "REMAINING").toUpperCase())}</text></svg>`;
  return el;
};

/** A tear-off calendar page. */
const tearoff = (date: Date = new Date()) =>
  h("div", { class: "tearoff" },
    h("div", { class: "rings" }, h("i"), h("i")),
    h("div", { class: "mon" }, date.toLocaleDateString([], { month: "short" }).toUpperCase()),
    h("div", { class: "day" }, date.getDate()),
    h("div", { class: "dow" }, date.toLocaleDateString([], { weekday: "long" })));

/** A handwritten sticky note, slightly tilted. */
const note = (text: Child, o: { tilt?: number } = {}) =>
  h("div", { class: "note", style: { transform: `rotate(${o.tilt ?? -1.5}deg)` } }, h("p", text));

/** Who is home: solid circle with an initial when home, dashed when away. */
const presence = (people: { name: string; home: boolean }[]) =>
  h("div", { class: "presence" },
    people.map((p) => h("div", { class: p.home ? "who home" : "who away" }, h("div", { class: "face" }, p.name.charAt(0).toUpperCase()), h("small", p.name))));

/** Moon phase, 0 = new, 0.25 = first quarter, 0.5 = full, 0.75 = last quarter. The shadow is hatched, the lit side open. */
const moon = (phase: number, size = 64) => {
  const p = ((phase % 1) + 1) % 1, k = Math.cos(2 * Math.PI * p), rx = (10 * Math.abs(k)).toFixed(2);
  const waxing = p < 0.5;
  const sweep = waxing ? (k > 0 ? 0 : 1) : k > 0 ? 1 : 0; // which way the terminator bulges
  const lit = `M12 2A10 10 0 0 ${waxing ? 1 : 0} 12 22A${rx} 10 0 0 ${sweep} 12 2z`;
  const id = `mn${++svgSeq}`;
  return svgIcon(`<defs><clipPath id="${id}"><path clip-rule="evenodd" d="${circ(12, 12, 10)}${lit}"/></clipPath></defs><path d="${circ(12, 12, 10)}"/><path d="${hatchDisc(12, 12, 10, 2, 22, 1.5)}" stroke-width="0.8" clip-path="url(#${id})"/><path d="${lit}" fill="none"/>`, size, 1.1);
};

// ------------------------------------------- modules: tiles, dials, rulers

/** A launcher-style module: label top-left, something in the middle, a count bottom-left. */
const tile = (label: string, body?: Child, o: { count?: Child; inverted?: boolean; bleed?: boolean; tone?: string; glyph?: string; waves?: boolean | Parameters<typeof waves>[0] } = {}) =>
  h("section", { class: `tile${o.inverted ? " inv" : ""}${o.bleed ? " bleed" : ""}${o.waves ? " has-waves" : ""}${o.tone ? " " + o.tone : ""}` },
    o.waves ? waves(o.waves === true ? {} : o.waves) : null,
    h("div", { class: "tile-head" }, h("span", { class: "tile-label" }, o.glyph && icon(o.glyph, { size: 12, stroke: 1.15 }), label), h("span", { class: "tile-id" })),
    h("div", { class: "tile-body" }, body),
    o.count != null && h("div", { class: "tile-count" }, o.count));

/** Compact ruled rows [left, text, right?]. The text takes the space and is cut with an ellipsis; rows share the height. */
const lines = (rows: [Child, Child, Child?][]) =>
  h("div", { class: "lines" },
    rows.map(([l, t, r]) => h("div", { class: "ln" }, l ? h("span", { class: "l" }, l) : null, h("span", { class: "t" }, t), r ? h("span", { class: "r" }, r) : null)));

/** A shopping-list style checklist: [text, qty?, done?]. Open items are an empty square, done ones a solid one. */
const checklist = (items: [Child, Child?, boolean?][]) =>
  lines(items.map(([t, q, done]) => [sigil("state", done ? "ok" : "idle", 11), t, q] as [Child, Child, Child]));

/** A box that is its own measuring frame (container units inside it refer to it, not to the tile). */
const pane = (...a: Arg[]) => h("div", { class: "pane" }, ...a);

// ------------------------------------------------ motion
// An e-ink panel redraws in partial refreshes, so motion is stepped: whole-second ticks, a few words at a time, a
// graph that shifts one column. Timers are named (ui.every, min 1 s): re-running replaces, a rollback cancels them.
// Sub-second motion (a spinner, a blinking cursor) is CSS steps(), which costs no JS.
let motionSeq = 0;
const motionName = (p: string) => `${p}${++motionSeq}`;

/** A blinking block cursor. */
const cursor = () => h("i", { class: "cursor" });

/** A spinner of 12 dots; the bright one steps round once a second. */
const spinner = (size = 14) => {
  let dots = "";
  for (let i = 0; i < 12; i++) {
    const a = (i / 12) * 2 * Math.PI;
    dots += `<circle cx="${(6 + 4.6 * Math.sin(a)).toFixed(2)}" cy="${(6 - 4.6 * Math.cos(a)).toFixed(2)}" r="${(0.35 + 0.65 * (i / 11)).toFixed(2)}"/>`;
  }
  const el = h("span", { class: "icon sig spinner" });
  el.innerHTML = `<svg viewBox="0 0 12 12" width="${size}" height="${size}" fill="currentColor">${dots}</svg>`;
  return el;
};

/** A countdown ring that ticks once a second and stops at zero (then calls onDone). */
const liveTimer = (seconds: number, o: { label?: string; total?: number; onDone?: () => void } = {}) => {
  let left = seconds;
  const name = motionName("timer"), el = h("div", { class: "timerlive" }, timerRing(left, o.total ?? seconds, o.label));
  let seen = false;
  every(name, 1000, () => {
    if (!el.isConnected) { if (seen) stop(name); return; }   // the screen changed: stop ticking
    seen = true;
    left = Math.max(0, left - 1);
    el.replaceChildren(timerRing(left, o.total ?? seconds, left ? o.label : "DONE"));
    if (!left) { stop(name); o.onDone?.(); }
  });
  return el;
};

/** Text that types itself in, `step` words per second. The full text is laid out (hidden) first, so the size never changes. */
const typewriter = (text: string, o: { step?: number; cursor?: boolean } = {}) => {
  const words = text.split(/\s+/), step = o.step ?? 2;
  let n = Math.min(step, words.length);
  const live = h("span", { class: "live" }), name = motionName("type");
  const draw = () => { live.textContent = words.slice(0, n).join(" ") + " "; if (o.cursor !== false) live.append(cursor()); };
  draw();
  const el = h("div", { class: "typewriter" }, h("span", { class: "ghost" }, text), live);
  let seen = false;
  every(name, 1000, () => { if (!el.isConnected) { if (seen) stop(name); return; } seen = true; if (n >= words.length) { stop(name); live.lastChild instanceof HTMLElement && live.lastChild.remove(); return; } n = Math.min(words.length, n + step); draw(); });
  return el;
};

/** A big numeral that counts up to `to` in `ticks` one-second steps (eased). Sized for the final text, so it never reflows. */
const countUp = (to: number, o: { from?: number; ticks?: number; dec?: number; unit?: string } = {}) => {
  const from = o.from ?? 0, ticks = o.ticks ?? 5, dec = o.dec ?? 0, fmt = (v: number) => v.toFixed(dec);
  const el = bignum(fmt(from), { unit: o.unit, chars: ([...fmt(to)].reduce((a, c) => a + (c === "." ? 0.45 : 1), 0)) + (o.unit ? 0.7 : 0) });
  let i = 0;
  const name = motionName("count");
  let seen = false;
  every(name, 1000, () => {
    if (!el.isConnected) { if (seen) stop(name); return; }
    seen = true;
    i++;
    const t = 1 - Math.pow(1 - Math.min(1, i / ticks), 3);
    el.firstChild!.textContent = fmt(from + (to - from) * t);
    if (i >= ticks) stop(name);
  });
  return el;
};

/** A graph that scrolls: each tick `next()` gives one new value per series, the oldest drops off, `draw` rebuilds the graph. */
const stream = (series: number[][], next: () => number[], draw: (s: number[][]) => HTMLElement, ms = 2000) => {
  const el = h("div", { class: "stream" }, draw(series)), name = motionName("stream");
  let seen = false;
  every(name, ms, () => {
    if (!el.isConnected) { if (seen) stop(name); return; }
    seen = true;
    const nx = next();
    series.forEach((s, i) => { s.shift(); s.push(nx[i]); });
    el.replaceChildren(draw(series));
  });
  return el;
};

/** A tile that fills like a tank (level 0..1) with a wavy surface. */
const levelTile = (label: string, level: number, text?: Child) => {
  const p = Math.max(0, Math.min(1, level)), top = (1 - p) * 100;
  const pts = Array.from({ length: 25 }, (_, i) => `${((i * 100) / 24).toFixed(1)}% ${(top + 2.2 * Math.sin(i * 0.9)).toFixed(1)}%`);
  return h("section", { class: "tile level" },
    h("div", { class: "water", style: { clipPath: `polygon(0% 100%, ${pts.join(", ")}, 100% 100%)` } }),
    h("div", { class: "tile-head" }, h("span", { class: "tile-label" }, label), h("span", { class: "tile-id" })),
    h("div", { class: "tile-count" }, h("span", { class: "chip" }, text ?? `${Math.round(p * 100)}%`)));
};

/**
 * A minimal box filled from the bottom up to a history curve (held between samples): the capacity is the
 * full height, so the silhouette IS the usage over time. A number sits inside. Pair it with a chart.
 */
const usageTile = (label: string, values: number[], cap: number, text: Child, o: { glyph?: string } = {}) => {
  const n = values.length, pts: string[] = [];
  values.forEach((v, i) => {
    const y = ((1 - Math.max(0, Math.min(1, v / cap))) * 100).toFixed(2);
    pts.push(`${((i / n) * 100).toFixed(2)}% ${y}%`, `${(((i + 1) / n) * 100).toFixed(2)}% ${y}%`);
  });
  return h("section", { class: "tile usage" },
    h("div", { class: "fillcurve", style: { clipPath: `polygon(0% 100%, ${pts.join(", ")}, 100% 100%)` } }),
    h("div", { class: "tile-head" }, h("span", { class: "tile-label" }, o.glyph && icon(o.glyph, { size: 12, stroke: 1.15 }), label), h("span", { class: "tile-id" })),
    h("div", { class: "tile-count" }, h("span", { class: "chip" }, text)));
};

/**
 * A 24-hour donut: noon at the top, midnight at the bottom. The night stretch is solid, the
 * day stretch open, and a marker sits where "now" is. Times are "HH:MM".
 */
const dayDial = (o: { rise: string; set: string; now?: string; size?: number; sub?: string; callouts?: boolean; side?: number }) => {
  const size = o.size ?? 190, T = Math.max(8, Math.round(size * 0.12)), ro = size / 2 - 3, ri = ro - T, R = (ro + ri) / 2, c = size / 2;
  const side = o.callouts ? o.side ?? 96 : 0, W = size + 2 * side;
  const d = new Date();
  const now = o.now ?? `${d.getHours()}:${d.getMinutes()}`;
  const phi = (m: number) => (2 * Math.PI * m) / 1440;
  const P = (m: number, r = R) => [side + c - r * Math.sin(phi(m)), c + r * Math.cos(phi(m))];
  const f = (n: number) => n.toFixed(1);
  const r0 = minutes(o.rise), r1 = minutes(o.set), nm = minutes(now), tw = 75;
  const deg = (m: number) => ((Math.max(0, Math.min(1440, m)) / 1440) * 360).toFixed(1);
  // midnight at the bottom, time running clockwise: dark at night, light by day, and the twilights blend smoothly
  const grad = `conic-gradient(from 180deg, #000 0deg, #000 ${deg(r0 - tw)}deg, #fff ${deg(r0 + tw)}deg, #fff ${deg(r1 - tw)}deg, #000 ${deg(r1 + tw)}deg, #000 360deg)`;
  const mask = `radial-gradient(circle at 50% 50%, transparent ${ri}px, #000 ${ri + 0.6}px, #000 ${ro}px, transparent ${ro + 0.6}px)`;
  const [nx, ny] = P(nm), mr = T * 0.46, isDay = nm >= r0 && nm <= r1;
  const marker = isDay
    ? `<circle cx="${f(nx)}" cy="${f(ny)}" r="${f(mr)}" fill="#fff" stroke-width="1.1"/><circle cx="${f(nx)}" cy="${f(ny)}" r="${f(mr * 0.38)}" fill="currentColor" stroke="none"/>`
    : `<circle cx="${f(nx)}" cy="${f(ny)}" r="${f(mr)}" fill="#fff" stroke-width="1.1"/><path d="M${f(nx + mr * 0.15)} ${f(ny - mr * 0.62)}A${f(mr * 0.62)} ${f(mr * 0.62)} 0 1 0 ${f(nx + mr * 0.15)} ${f(ny + mr * 0.62)}A${f(mr * 0.44)} ${f(mr * 0.62)} 0 1 1 ${f(nx + mr * 0.15)} ${f(ny - mr * 0.62)}Z" fill="currentColor" stroke="none"/>`;
  let notes = "";
  if (o.callouts) {
    const mid = (r0 + r1) / 2;
    const list: [string, string, number, "l" | "r"][] = [
      ["SUNRISE", o.rise, r0, "l"], ["SUNSET", o.set, r1, "r"], ["NOW", now, nm, "r"],
      ["SOLAR NOON", `${String(Math.floor(mid / 60)).padStart(2, "0")}:${String(Math.round(mid % 60)).padStart(2, "0")}`, mid, "l"],
    ];
    for (const [name, val, m, sd] of list) {
      const [tx, ty] = P(m), edge = sd === "l" ? 2 : W - 2;
      notes += `<path d="M${f(tx)} ${f(ty)}H${edge}" stroke-width="0.8"/><rect x="${f(tx - 2)}" y="${f(ty - 2)}" width="4" height="4" fill="currentColor" stroke="none"/>`;
      notes += `<text x="${edge}" y="${f(ty - 13)}" text-anchor="${sd === "l" ? "start" : "end"}" fill="currentColor" stroke="none" style="font:600 9px var(--mono);letter-spacing:.06em">${name}</text>`;
      notes += `<text x="${edge}" y="${f(ty - 3)}" text-anchor="${sd === "l" ? "start" : "end"}" fill="currentColor" stroke="none" style="font:600 11px var(--mono)">${val}</text>`;
    }
  }
  const el = h("div", { class: "dial", style: { width: `${W}px`, height: `${size}px` } });
  el.innerHTML = `<div class="ring" style="left:${side}px;width:${size}px;height:${size}px;background:${grad};-webkit-mask:${mask};mask:${mask}"></div>
    <svg width="${W}" height="${size}" viewBox="0 0 ${W} ${size}" fill="none" stroke="currentColor" style="position:absolute;inset:0">
    <circle cx="${side + c}" cy="${c}" r="${ro}" stroke-width="0.8"/><circle cx="${side + c}" cy="${c}" r="${ri}" stroke-width="0.8"/>${marker}${notes}</svg>`;
  const [hh, mm] = now.split(":").map(Number);
  el.append(h("div", { class: "mid", style: { left: `${side}px`, width: `${size}px`, right: "auto" } }, h("b", { style: { fontSize: `${Math.max(12, Math.round(size * 0.19))}px` } }, `${String(hh).padStart(2, "0")}:${String(mm).padStart(2, "0")}`), o.sub && h("small", o.sub)));
  return el;
};

/** A scale with ticks, a marker at the current value and an optional target tick. */
const ruler = (o: { min: number; max: number; value: number; target?: number; step?: number; w?: number }) => {
  const w = o.w ?? 360, hgt = 76, pad = 16;
  const step = o.step ?? Math.max(1, Math.round((o.max - o.min) / 7)), minor = step / 4;
  const x = (v: number) => pad + ((v - o.min) / (o.max - o.min)) * (w - 2 * pad);
  const n = Math.round((o.max - o.min) / minor);
  let ticks = "", labels = "";
  for (let i = 0; i <= n; i++) {
    const v = o.min + i * minor, major = i % 4 === 0;
    ticks += `M${x(v).toFixed(1)} ${major ? 36 : 44}V56`;
    if (major) labels += `<text x="${x(v).toFixed(1)}" y="70" text-anchor="middle" stroke="none" fill="currentColor">${Number.isInteger(v) ? v : v.toFixed(1)}</text>`;
  }
  const vx = x(Math.max(o.min, Math.min(o.max, o.value))).toFixed(1);
  const target = o.target == null ? "" : `<path d="M${x(o.target).toFixed(1)} 30V58" stroke-width="2.5"/>`;
  const el = h("div", { class: "ruler" });
  el.innerHTML = `<svg width="${w}" height="${hgt}" viewBox="0 0 ${w} ${hgt}" fill="none" stroke="currentColor" stroke-linecap="butt" style="font:600 11px var(--mono)">
    <path d="${ticks}" stroke-width="1.5"/>${labels}${target}
    <path d="M${vx} 12V58" style="stroke:var(--accent)" stroke-width="2.5"/>
    <path d="M${(+vx - 7).toFixed(1)} 0h14l-7 12z" style="fill:var(--accent)" stroke="none"/></svg>`;
  return el;
};

/** Hairline-divided readouts: a tiny label, a big value, a note underneath. */
const statStrip = (cells: { label: string; value: Child; sub?: Child }[]) =>
  h("div", { class: "stats" },
    cells.map((c) => h("div", { class: "cell" }, h("div", { class: "k" }, c.label), h("div", { class: "v" }, c.value), c.sub && h("small", c.sub))));

/** Scene chips; the active one is inverted. Items are strings or {label, icon}. */
const pills = (items: (string | { label: string; icon?: string })[], o: { active?: number } = {}) =>
  h("div", { class: "pills" },
    items.map((it, i) => {
      const p = typeof it === "string" ? { label: it, icon: undefined } : it;
      return h("span", { class: i === o.active ? "pill on" : "pill" }, p.icon && icon(p.icon, { size: 18 }), p.label);
    }));

/** A level bar with its label and value written on it (text flips colour over the fill). */
const slider = (value: number, label: Child, o: { text?: Child } = {}) => {
  const p = Math.round(Math.max(0, Math.min(1, value)) * 100);
  const layer = (cls: string) => h("div", { class: `sl-txt ${cls}` }, h("span", label), h("b", o.text ?? `${p}%`));
  return h("div", { class: "slider", style: { "--p": `${p}%` } }, h("div", { class: "sl-fill" }), layer("dark"), layer("light"));
};

/** Settings-style rows: label with a status dot (ok / warn / off), then a value or an arrow. */
const menu = (rows: { label: Child; value?: Child; dot?: "ok" | "warn" | "off" }[]) =>
  h("div", { class: "menu" },
    rows.map((r) =>
      h("div", { class: "mrow" },
        h("span", { class: "ml" }, r.dot ? sigil("state", r.dot === "off" ? "idle" : r.dot, 11) : h("i", { class: "sig-gap" }), r.label),
        h("span", { class: "mv" }, r.value ?? icon("arrow", { size: 22, stroke: 2.8 })))));

/** A device row: round icon, name and state, and a switch (display only). */
const device = (o: { name: Child; sub?: Child; on?: boolean; icon?: string }) =>
  h("div", { class: o.on ? "device on" : "device" },
    h("div", { class: "dv-icon" }, icon(o.icon ?? "plug", { size: 24 })),
    h("div", { class: "dv-txt" }, h("div", { class: "dv-name" }, o.name), o.sub && h("small", o.sub)),
    h("div", { class: "switch" }, h("i")));

/** One huge number with a small unit and a status line. */
const hero = (value: Child, o: { unit?: string; label?: Child; status?: Child; sub?: Child; size?: number } = {}) =>
  h("div", { class: "hero" },
    o.label && h("div", { class: "label" }, o.label),
    h("div", { class: "num", style: o.size ? { fontSize: `${o.size}px` } : "" }, value, o.unit && h("sup", o.unit)),
    o.status && h("div", { class: "hstatus" }, h("i", { class: "sd on" }), o.status, o.sub && h("small", o.sub)));

// ------------------------------------------------ vector screens: polar plot, ridgeline, word tiles

/** A polar plot: rings, spokes, the values as a hatched polygon. Good for anything cyclic (hours, compass). */
const polar = (values: number[], o: { max?: number; rings?: number; labels?: [string, number][]; marks?: boolean } = {}) => {
  const n = values.length, c = 100, R = 74, max = o.max ?? Math.max(...values), rings = o.rings ?? 4, id = `hz${++svgSeq}`;
  const pt = (i: number, r: number) => { const a = (i / n) * 2 * Math.PI; return [c + r * Math.sin(a), c - r * Math.cos(a)]; };
  let g = "";
  for (let k = 1; k <= rings; k++) g += `<circle cx="${c}" cy="${c}" r="${(R * k) / rings}" stroke-width="${k === rings ? 1.4 : 0.7}" ${k === rings ? "" : 'stroke-dasharray="1.5 3"'}/>`;
  const q = Math.max(1, Math.round(n / 4));
  for (let i = 0; i < n; i++) { const [x, y] = pt(i, R); g += `<path d="M${c} ${c}L${x.toFixed(1)} ${y.toFixed(1)}" stroke-width="${i % q === 0 ? 1 : 0.35}"/>`; }
  const pts = values.map((v, i) => pt(i, (R * v) / max));
  g += `<path d="${pts.map(([x, y], i) => `${i ? "L" : "M"}${x.toFixed(1)} ${y.toFixed(1)}`).join("")}Z" fill="url(#${id})" stroke-width="2"/>`;
  if (o.marks) for (const [x, y] of pts) g += `<rect x="${(x - 2).toFixed(1)}" y="${(y - 2).toFixed(1)}" width="4" height="4" fill="#fff" stroke-width="1"/>`;
  for (const [t, i] of o.labels ?? []) { const [x, y] = pt(i, R + 13); g += `<text x="${x.toFixed(1)}" y="${(y + 3).toFixed(1)}" text-anchor="middle" fill="currentColor" stroke="none" style="font:600 9px var(--mono)">${escHtml(t)}</text>`; }
  const el = h("div", { class: "polar" });
  el.innerHTML = `<svg viewBox="0 0 200 200" fill="none" stroke="currentColor" stroke-linejoin="miter"><defs>${HATCH(id)}</defs>${g}</svg>`;
  return el;
};

/** Rows stacked as ridgelines, back to front, each hiding the ones behind it: a surface made of days. */
const ridgeline = (rows: number[][], o: { w?: number; h?: number; labels?: string[]; hours?: [string, number][] } = {}) => {
  const W = o.w ?? 330, H = o.h ?? 130, n = rows.length, m = rows[0].length, LW = o.labels ? 26 : 2;
  const step = (H - 26) / (n + 0.5), amp = step * 3.2, max = Math.max(...rows.flat());
  const X = (j: number) => LW + (j / (m - 1)) * (W - LW - 2);
  let g = "";
  rows.forEach((r, i) => {
    const base = 18 + (i + 1) * step;
    let line = "";
    r.forEach((v, j) => { line += `${j ? "L" : "M"}${X(j).toFixed(1)} ${(base - (v / max) * amp).toFixed(1)}`; });
    g += `<path d="${line}L${X(m - 1).toFixed(1)} ${(base + 4).toFixed(1)}L${X(0).toFixed(1)} ${(base + 4).toFixed(1)}Z" fill="#fff" stroke="none"/>`;
    g += `<path d="${line}" stroke-width="${i === n - 1 ? 2 : 1}"/>`;
    if (o.labels) g += `<text x="0" y="${(base + 2).toFixed(1)}" fill="currentColor" stroke="none" style="font:600 8.5px var(--mono)">${escHtml(o.labels[i] ?? "")}</text>`;
  });
  for (const [t, j] of o.hours ?? []) g += `<text x="${X(j).toFixed(1)}" y="${H - 3}" text-anchor="middle" fill="currentColor" stroke="none" style="font:600 8.5px var(--mono)">${escHtml(t)}</text>`;
  const el = h("div", { class: "ridge" });
  el.innerHTML = `<svg viewBox="0 0 ${W} ${H}" fill="none" stroke="currentColor" stroke-linejoin="round">${g}</svg>`;
  return el;
};

/** A spaced-caps word with a caption: the quick-glance status screen (e.g. FLX, NUC). */
const word = (text: string, caption?: Child) =>
  h("div", { class: "word" }, h("div", { class: "w" }, text), caption != null && h("div", { class: "cap" }, caption));

// ------------------------------------------------ wavy gradients, border-title modules, dot graphs

/**
 * One continuous gradient whose flow is twisted by a domain-warped sine field, so it swirls like marbling rather than
 * bumping. Drawn once to a small grey canvas and used as an image (so it survives in the DOM). At 1-bit the dither
 * turns it into grain. Put one behind a tile's content with tile {waves}.
 */
const waves = (o: { from?: number; seed?: number; scale?: number; tone?: "sky" | "water" } = {}) => {
  const from = (o.from ?? 30) / 100, seed = o.seed ?? 3, tw = (o.scale ?? 52) / 52, W = 96, H = 96;
  const c = document.createElement("canvas");
  c.width = W; c.height = H;
  const ctx = c.getContext("2d")!, img = ctx.createImageData(W, H);
  const ss = (a: number, b: number, x: number) => { const t = Math.max(0, Math.min(1, (x - a) / (b - a))); return t * t * (3 - 2 * t); };
  for (let j = 0; j < H; j++) for (let i = 0; i < W; i++) {
    const x = i / W, y = j / H;
    let u = x * 3.6, v = y * 2.2;
    for (let k = 0; k < 3; k++) {            // warp the plane through itself: the twist
      const pu = u, pv = v;
      u = pu + 0.85 * tw * Math.sin(pv * 1.6 + seed * 1.3 + k * 2.1);
      v = pv + 0.85 * tw * Math.sin(pu * 1.9 + seed * 0.7 + k * 1.3);
    }
    const t = y + 0.2 * tw * Math.sin(u * 1.3 + v * 0.8) + 0.08 * Math.sin(v * 2.1 - u);
    let g = o.tone === "water" ? 1 - ss(0.05, 0.75, t) : ss(from, 0.92, t);        // 1 = black
    const bx = x - 0.8, by = y - 1.05 + 0.1 * Math.sin(u * 1.2);                   // a second, darker mass
    g = Math.max(g, 0.85 * Math.max(0, 1 - Math.hypot(bx, by) / 0.75) * (o.tone === "water" ? 0 : 1));
    const k = Math.round(255 * (1 - g)), n = (j * W + i) * 4;
    img.data[n] = img.data[n + 1] = img.data[n + 2] = k;
    img.data[n + 3] = 255;
  }
  ctx.putImageData(img, 0, 0);
  const el = h("div", { class: "waves" });
  el.innerHTML = `<img src="${c.toDataURL()}" alt="" style="width:100%;height:100%;display:block;image-rendering:auto">`;
  return el;
};

/** Two dot graphs mirrored on a centre line: `up` above, `down` below (traffic in/out). Values 0..1. */
const dotMirror = (up: number[], down: number[], o: { rows?: number } = {}) => {
  const rows = o.rows ?? 5, p = 10, n = Math.max(up.length, down.length);
  let dots = "";
  for (let x = 0; x < n; x++) {
    const u = Math.round(Math.max(0, Math.min(1, up[x] ?? 0)) * rows), dn = Math.round(Math.max(0, Math.min(1, down[x] ?? 0)) * rows);
    for (let y = 0; y < rows; y++) {
      dots += `<circle cx="${((x + 0.5) * p).toFixed(1)}" cy="${((rows - 1 - y + 0.5) * p).toFixed(1)}" r="${(p * (y < u ? 0.4 : 0.07)).toFixed(2)}"/>`;
      dots += `<circle cx="${((x + 0.5) * p).toFixed(1)}" cy="${((rows + 1 + y + 0.5) * p).toFixed(1)}" r="${(p * (y < dn ? 0.4 : 0.07)).toFixed(2)}"/>`;
    }
  }
  const el = h("div", { class: "dotbars" });
  el.innerHTML = `<svg viewBox="0 0 ${n * p} ${(2 * rows + 1) * p}" fill="currentColor"><path d="M0 ${rows * p + p / 2}H${n * p}" stroke="currentColor" stroke-width="1"/>${dots}</svg>`;
  return el;
};

/** Rows of  LABEL  [small dot graph]  value, one graph per row (cores, disks, ...). */
const sparkRows = (rows: { label: string; values: number[]; text: Child }[], o: { rows?: number } = {}) =>
  h("div", { class: "sparkrows" },
    rows.flatMap((r) => [h("span", { class: "rl" }, r.label), dotBars(r.values, { rows: o.rows ?? 3 }), h("span", { class: "rv" }, r.text)]));

// ------------------------------------------------ sigils: tiny marks, one fact each

type SigKind = "state" | "level" | "trend" | "age" | "count" | "dir" | "bars" | "glyph";
/**
 * The marks are drawn in a 12x12 box, 1px line. What they say:
 *   state  "ok" solid square | "idle" open square | "warn" half-filled square | "err" crossed square | anything else: dashed square (unknown)
 *   level  [value, max] (max 8 by default): a square tank filled from the bottom (empty..full)
 *   trend  "up" | "down" | "flat" | "rise" | "fall"  (slow and steep arrows)
 *   age    minutes since the data was fresh: the hand of a one-hand clock in a square (60 or more: dashed, stale)
 *   count  1..6 as dice faces, more as a plus
 *   dir    degrees, 0 = up: a rotated arrow (wind, bearing)
 *   bars   0..4 signal bars
 *   glyph  any icon name, scaled down
 */
const sigPaths = (kind: SigKind, v: any): string => {
  switch (kind) {
    case "state":
      return v === "ok" ? `<rect x="2.5" y="2.5" width="7" height="7" fill="currentColor" stroke="none"/>`
        : v === "idle" ? `<rect x="2.5" y="2.5" width="7" height="7" fill="none"/>`
        : v === "warn" ? `<rect x="2.5" y="2.5" width="7" height="7" fill="none"/><path d="M2.5 2.5v7h7z" fill="currentColor" stroke="none"/>`
        : v === "err" ? `<rect x="1.5" y="1.5" width="9" height="9" fill="none"/><path d="M4 4l4 4M8 4L4 8" fill="none"/>`
        : `<rect x="2.5" y="2.5" width="7" height="7" fill="none" stroke-dasharray="1.5 1.5"/>`;
    case "level": {
      const [val, max] = Array.isArray(v) ? v : [v, 8], f = Math.max(0, Math.min(1, val / max));
      return `<rect x="1.5" y="1.5" width="9" height="9" fill="none"/>` + (f > 0 ? `<rect x="1.5" y="${(10.5 - 9 * f).toFixed(2)}" width="9" height="${(9 * f).toFixed(2)}" fill="currentColor" stroke="none"/>` : "");
    }
    case "trend":
      return v === "up" ? `<path d="M2.5 9.5L9.5 2.5M4.5 2.5h5v5" fill="none"/>`
        : v === "down" ? `<path d="M2.5 2.5l7 7M9.5 4.5v5h-5" fill="none"/>`
        : v === "rise" ? `<path d="M6 10.5v-9M2.5 5L6 1.5 9.5 5" fill="none"/>`
        : v === "fall" ? `<path d="M6 1.5v9M2.5 7L6 10.5 9.5 7" fill="none"/>`
        : `<path d="M1.5 6h9M7.5 3l3 3-3 3" fill="none"/>`;
    case "age": {
      const m = Math.max(0, Number(v) || 0), a = (Math.min(m, 60) / 60) * 2 * Math.PI;
      return `<rect x="1.5" y="1.5" width="9" height="9" fill="none"${m >= 60 ? ' stroke-dasharray="1.6 1.4"' : ""}/>` + `<path d="M6 1.5v1.4M6 6L${(6 + 3.4 * Math.sin(a)).toFixed(2)} ${(6 - 3.4 * Math.cos(a)).toFixed(2)}" fill="none"/>`;
    }
    case "count": {
      const n = Number(v) || 0, d = (x: number, y: number) => `<circle cx="${x}" cy="${y}" r="0.95" fill="currentColor" stroke="none"/>`;
      const pos: Record<number, [number, number][]> = {
        1: [[6, 6]], 2: [[3.6, 3.6], [8.4, 8.4]], 3: [[3.6, 3.6], [6, 6], [8.4, 8.4]], 4: [[3.6, 3.6], [8.4, 3.6], [3.6, 8.4], [8.4, 8.4]],
        5: [[3.6, 3.6], [8.4, 3.6], [6, 6], [3.6, 8.4], [8.4, 8.4]], 6: [[3.6, 3.2], [8.4, 3.2], [3.6, 6], [8.4, 6], [3.6, 8.8], [8.4, 8.8]],
      };
      return `<rect x="1.5" y="1.5" width="9" height="9" fill="none"/>` + (n >= 1 && n <= 6 ? pos[n].map(([x, y]) => d(x, y)).join("") : `<path d="M6 3.5v5M3.5 6h5" fill="none"/>`);
    }
    case "dir":
      return `<g transform="rotate(${Number(v) || 0} 6 6)"><path d="M6 10.8V1.5M3.2 4.3L6 1.5l2.8 2.8" fill="none"/></g>`;
    case "bars": {
      const n = Math.max(0, Math.min(4, Number(v) || 0));
      return [3, 5.5, 8, 10.5].map((hh, i) => `<rect x="${1.4 + i * 2.7}" y="${11 - hh}" width="1.8" height="${hh}" ${i < n ? 'fill="currentColor" stroke="none"' : 'fill="none" stroke-width="0.8"'}/>`).join("");
    }
    case "glyph": {
      return `<g transform="translate(1 1) scale(0.4167)" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round">${ICON_PATHS[String(v)] ?? ""}</g>`;
    }
  }
};

/** One sigil as an inline element (use it next to text, in a table cell, in a module label). */
const sigil = (kind: SigKind, v?: any, size = 12) => {
  const el = h("span", { class: "icon sig", title: `${kind} ${Array.isArray(v) ? v.join("/") : v ?? ""}` });
  el.innerHTML = `<svg viewBox="0 0 12 12" width="${size}" height="${size}" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter">${sigPaths(kind, v)}</svg>`;
  return el;
};
const sigils = (items: [SigKind, any?][], size = 12) => h("span", { class: "sigils" }, items.map(([k, v]) => sigil(k, v, size)));

// ------------------------------------------------ explicit state: status bar, segmented readouts

/** A ruled bar of labelled segments (the first is a solid mark), then a gap, then right-aligned segments. */
const statusStrip = (left: Child[], right: Child[] = []) =>
  h("div", { class: "statusbar" },
    left.map((c, i) => h("span", { class: i === 0 ? "sb-mark" : "" }, c)),
    h("span", { class: "sb-fill" }),
    right.map((c) => h("span", c)));

/** A bar: the filled part is solid, the rest a halftone. value of max. */
const segbar = (value: number, max = 1) =>
  h("div", { class: "segbar" }, h("i", { style: { width: `${(Math.max(0, Math.min(1, value / max)) * 100).toFixed(1)}%` } }));

/** Rows of  LABEL  [segmented bar]  value unit. Dense and exact. */
const readouts = (rows: { label: string; value: number; max?: number; unit?: string; text?: Child }[]) =>
  h("div", { class: "readouts" },
    rows.flatMap((r) => [
      h("span", { class: "rl" }, r.label),
      segbar(r.value, r.max ?? 100),
      h("span", { class: "rv" }, r.text ?? String(r.value), r.unit && h("small", r.unit)),
    ]));

/** Key/value pairs in a grid of columns (default 2), each pair a ruled row. */
const kvgrid = (obj: Record<string, Child>, cols = 2) =>
  h("div", { class: "kvgrid", style: { "--cols": cols } },
    Object.entries(obj).map(([k, v]) => h("div", { class: "kv" }, h("span", { class: "k" }, k), h("span", { class: "v" }, v))));

/** A small framed square: a weather icon over a dithered sky, with a value written in the dark part. */
const skybox = (name: string, text: Child, size = 84) =>
  h("div", { class: "skybox", style: { width: `${size}px`, height: `${size}px` } }, weatherIcon(name, Math.round(size * 0.5)), h("b", text));

// ------------------------------------------------ documents: headers, ruled tables, tone strip, line diagrams

/** The sheet header: small stacked kicker lines, a large code, right-aligned lines. */
const header = (o: { kicker: string[]; code: string; right?: Child[] }) =>
  h("div", { class: "header" },
    h("div", { class: "hk" }, o.kicker.map((k) => h("div", k))),
    h("div", { class: "hc" }, o.code),
    h("div", { class: "hr" }, (o.right ?? []).map((r) => h("div", r))));

/** A ruled table of cells; each cell is {title, rows: [label, value][]}. Like a drawing's header block. */
const ruled = (...cells: { title?: string; rows: [string, Child][] }[]) =>
  h("div", { class: "ruled", style: { "--n": cells.length } },
    cells.map((c) =>
      h("div", { class: "rcell" },
        c.title && h("div", { class: "rt" }, c.title),
        c.rows.map(([k, v]) => h("div", { class: "rr" }, h("span", k), h("b", v))))));

/** A calibration strip of grey steps (the panel dithers them), the way a print sheet carries one. */
const toneStrip = (steps = 16) =>
  h("div", { class: "tonestrip" },
    Array.from({ length: steps }, (_, i) => h("i", { style: { background: `hsl(0 0% ${Math.round((i / (steps - 1)) * 100)}%)` } })));

type Stop = { at: string; to?: string; name: string };
/**
 * Lanes drawn like a timetable diagram: each lane a ruled line in its own style, events as nodes with the
 * time above and the name below, a span as a thick segment, a "now" line. `links` join two lanes at a time.
 */
const routes = (o: { from: string; to: string; now?: string; w?: number; lanes: { label: string; look?: "solid" | "dash" | "dot" | "double"; stops: Stop[] }[]; links?: [number, string, number][] }) => {
  const W = o.w ?? 732, LW = 74, laneH = 50, top = 22, mono = `style="font:600 10px var(--mono)"`, name = `style="font:700 11px var(--mono)"`;
  const t0 = minutes(o.from), t1 = minutes(o.to), X = (m: string | number) => LW + (((typeof m === "number" ? m : minutes(m)) - t0) / (t1 - t0)) * (W - LW - 6);
  const H = top + o.lanes.length * laneH + 22;
  const Y = (i: number) => top + i * laneH + laneH / 2 - 4;
  const dash = { solid: "", dash: 'stroke-dasharray="9 5"', dot: 'stroke-dasharray="1 5" stroke-linecap="round"', double: "" } as const;
  let g = "";
  for (let m = Math.ceil(t0 / 60) * 60; m <= t1; m += 60) g += `<path d="M${X(m).toFixed(1)} ${top - 4}V${H - 22}" stroke-width="0.6" stroke-dasharray="1 4"/>`;
  o.lanes.forEach((ln, i) => {
    const y = Y(i), look = ln.look ?? "solid";
    g += `<text x="0" y="${y + 3}" ${mono} fill="currentColor" stroke="none">${escHtml(ln.label.toUpperCase())}</text>`;
    g += `<path d="M${LW - 6} ${y}H${W}" stroke-width="${look === "double" ? 5 : 2}" ${dash[look]}/>`;
    if (look === "double") g += `<path d="M${LW - 6} ${y}H${W}" stroke="#fff" stroke-width="2"/>`;
    for (const s of ln.stops) {
      const x1 = X(s.at), x2 = s.to ? X(s.to) : x1;
      if (s.to) g += `<path d="M${x1.toFixed(1)} ${y}H${x2.toFixed(1)}" stroke-width="9" stroke-linecap="butt"/>`;
      g += `<rect x="${(x1 - 4).toFixed(1)}" y="${y - 4}" width="8" height="8" fill="#fff" stroke-width="2"/>`;
      if (s.to) g += `<rect x="${(x2 - 4).toFixed(1)}" y="${y - 4}" width="8" height="8" fill="#fff" stroke-width="2"/>`;
      g += `<text x="${x1.toFixed(1)}" y="${y - 10}" text-anchor="middle" ${mono} fill="currentColor" stroke="none">${escHtml(s.at)}</text>`;
      g += `<text x="${((x1 + x2) / 2).toFixed(1)}" y="${y + 21}" text-anchor="middle" ${name} fill="currentColor" stroke="none">${escHtml(s.name)}</text>`;
    }
  });
  for (const [a, m, b] of o.links ?? []) {
    const x = X(m);
    g += `<path d="M${x.toFixed(1)} ${Y(a)}V${Y(b)}" stroke-width="1.5"/><circle cx="${x.toFixed(1)}" cy="${Y(a)}" r="4" fill="currentColor"/><circle cx="${x.toFixed(1)}" cy="${Y(b)}" r="4" fill="currentColor"/>`;
  }
  for (let m = Math.ceil(t0 / 60) * 60; m <= t1; m += 60)
    if ((m / 60) % 2 === 0) g += `<text x="${X(m).toFixed(1)}" y="${H - 6}" text-anchor="middle" ${mono} fill="currentColor" stroke="none">${String(m / 60).padStart(2, "0")}</text>`;
  if (o.now) {
    const nx = X(o.now);
    g += `<path d="M${nx.toFixed(1)} ${top - 8}V${H - 20}" stroke-width="2" stroke-dasharray="3 3"/><rect x="${(nx - 32).toFixed(1)}" y="0" width="64" height="13" fill="currentColor" stroke="none"/><text x="${nx.toFixed(1)}" y="10" text-anchor="middle" ${mono} fill="#fff" stroke="none">NOW ${escHtml(o.now)}</text>`;
  }
  const el = h("div", { class: "routes" });
  el.innerHTML = `<svg viewBox="0 0 ${W} ${H}" fill="none" stroke="currentColor">${g}</svg>`;
  return el;
};

// ------------------------------------------------ bolder pieces: tone, dot matrix, timelines, charts

type Tone = "plain" | "inv" | "sky" | "dots";
/** A big region: plain, inverted (white on black), a dithered sky gradient, or a dot field. */
const panel = (tone: Tone, ...a: Arg[]) => h("section", { class: `panel ${tone}` }, ...a);

/** Tiny all-caps line across the top: left text, right text. */
const meta = (left: Child, right?: Child) => h("div", { class: "meta" }, h("span", left), right != null && h("span", right));

/** Claude's line of the moment: avatar, one sentence, a timestamp. */
const claudeLine = (text: Child, time?: Child) =>
  h("div", { class: "cline" },
    h("div", { class: "avatar" }, icon("sparkle", { size: 24, stroke: 2 })),
    h("div", { class: "txt" }, text),
    time != null && h("div", { class: "t" }, time));

/** A 6-column launcher grid; place children with style gridArea ("row / col / row-end / col-end"). */
const tilegrid = (...a: Arg[]) => h("div", { class: "tilegrid" }, ...a);

const HATCH = (id: string) =>
  `<pattern id="${id}" width="7" height="7" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><path d="M0 0V7" stroke="currentColor" stroke-width="1.8"/></pattern>`;
let svgSeq = 0;

// 5x7 dot-matrix glyphs
const GLYPHS: Record<string, string[]> = {
  "0": ["01110", "10001", "10011", "10101", "11001", "10001", "01110"],
  "1": ["00100", "01100", "00100", "00100", "00100", "00100", "01110"],
  "2": ["01110", "10001", "00001", "00010", "00100", "01000", "11111"],
  "3": ["11110", "00001", "00001", "01110", "00001", "00001", "11110"],
  "4": ["00010", "00110", "01010", "10010", "11111", "00010", "00010"],
  "5": ["11111", "10000", "11110", "00001", "00001", "10001", "01110"],
  "6": ["00110", "01000", "10000", "11110", "10001", "10001", "01110"],
  "7": ["11111", "00001", "00010", "00100", "01000", "01000", "01000"],
  "8": ["01110", "10001", "10001", "01110", "10001", "10001", "01110"],
  "9": ["01110", "10001", "10001", "01111", "00001", "00010", "01100"],
  ":": ["0", "0", "1", "0", "1", "0", "0"],
  ".": ["0", "0", "0", "0", "0", "0", "1"],
  "-": ["00000", "00000", "00000", "11111", "00000", "00000", "00000"],
  " ": ["00", "00", "00", "00", "00", "00", "00"],
};
const dotMatrixSvg = (text: string, pitch: number) => {
  const glyphs = [...text].map((c) => GLYPHS[c] ?? GLYPHS[" "]);
  const rows = Array.from({ length: 7 }, (_, y) => glyphs.map((g) => g[y]).join("0"));
  const cols = rows[0].length;
  let dots = "";
  for (let y = 0; y < 7; y++)
    for (let x = 0; x < cols; x++)
      dots += `<circle cx="${((x + 0.5) * pitch).toFixed(1)}" cy="${((y + 0.5) * pitch).toFixed(1)}" r="${(pitch * (rows[y][x] === "1" ? 0.41 : 0.07)).toFixed(2)}"/>`;
  return `<svg width="${cols * pitch}" height="${7 * pitch}" viewBox="0 0 ${cols * pitch} ${7 * pitch}" fill="currentColor">${dots}</svg>`;
};

/** A dot-matrix clock. Pass `time` ("22:26") for a fixed one, otherwise it follows the time. */
const dotClock = (o: { time?: string; pitch?: number } = {}) => {
  const el = h("div", { class: "dotclock" });
  const draw = () => {
    const d = new Date();
    const t = o.time ?? `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
    if (el.dataset.t === t) return;
    el.dataset.t = t;
    el.innerHTML = dotMatrixSvg(t, o.pitch ?? 20);
  };
  draw();
  return o.time ? el : live(el, draw, 15_000);
};

/** The 24 hours as a ruler: night hatched, day open, a marker where "now" is. Times "HH:MM". */
const dayStrip = (o: { rise: string; set: string; now?: string; w?: number }) => {
  const W = o.w ?? 720, H = 58, x = (m: number) => (m / 1440) * W;
  const d = new Date(), id = `hz${++svgSeq}`;
  const r = minutes(o.rise), st = minutes(o.set), nx = x(minutes(o.now ?? `${d.getHours()}:${d.getMinutes()}`));
  let ticks = "", labels = "";
  for (let hr = 0; hr <= 24; hr++) ticks += `M${x(hr * 60).toFixed(1)} 30V${hr % 6 === 0 ? 42 : 36}`;
  for (const hr of [0, 6, 12, 18, 24])
    labels += `<text x="${x(hr * 60).toFixed(1)}" y="56" text-anchor="${hr === 0 ? "start" : hr === 24 ? "end" : "middle"}" fill="currentColor" stroke="none">${String(hr % 24).padStart(2, "0")}</text>`;
  const el = h("div", { class: "daystrip" });
  el.innerHTML = `<svg viewBox="0 0 ${W} ${H}" fill="none" stroke="currentColor" stroke-linecap="round" style="font:600 13px var(--mono)">
    <defs>${HATCH(id)}</defs>
    <rect x="0" y="14" width="${x(r).toFixed(1)}" height="16" fill="url(#${id})" stroke="none"/>
    <rect x="${x(st).toFixed(1)}" y="14" width="${(W - x(st)).toFixed(1)}" height="16" fill="url(#${id})" stroke="none"/>
    <rect x="1" y="14" width="${W - 2}" height="16" stroke-width="2"/>
    <path d="${ticks}" stroke-width="2"/>${labels}
    <path d="M${nx.toFixed(1)} 8V30" stroke-width="4"/><path d="M${(nx - 8).toFixed(1)} 0h16l-8 10z" fill="currentColor" stroke="none"/></svg>`;
  return el;
};

type Block = { from: string; to: string; label?: string; look?: "solid" | "hatch" | "outline" };
/** Lanes of blocks over a time axis (a Gantt strip), with a "now" line. Times "HH:MM". */
const timeline = (o: { w?: number; from: string; to: string; now?: string; lanes: { label: string; blocks: Block[] }[] }) => {
  const W = o.w ?? 756, LW = 96, laneH = 44, gap = 10, axis = 32;
  const t0 = minutes(o.from), t1 = minutes(o.to);
  const X = (hm: string | number) => LW + (((typeof hm === "number" ? hm : minutes(hm)) - t0) / (t1 - t0)) * (W - LW);
  const H = o.lanes.length * (laneH + gap) + axis, id = `hz${++svgSeq}`;
  const mono = `style="font:600 13px var(--mono)"`, body = `style="font:700 16px var(--body)"`;
  let g = "";
  o.lanes.forEach((lane, i) => {
    const y = i * (laneH + gap);
    g += `<text x="0" y="${y + laneH / 2 + 5}" ${mono} fill="currentColor" stroke="none">${escHtml(lane.label.toUpperCase())}</text>`;
    g += `<path d="M${LW} ${y + laneH}H${W}" stroke-width="1.5" stroke-dasharray="1 6"/>`;
    for (const b of lane.blocks) {
      const x1 = X(b.from), w = Math.max(6, X(b.to) - x1), look = b.look ?? "solid";
      g += `<rect x="${x1.toFixed(1)}" y="${y}" width="${w.toFixed(1)}" height="${laneH}" fill="${look === "solid" ? "currentColor" : look === "hatch" ? `url(#${id})` : "#fff"}" stroke-width="2.5"/>`;
      if (b.label && w > b.label.length * 9.5 + 18) {
        const tw = b.label.length * 9.2 + 14;
        if (look === "hatch") g += `<rect x="${(x1 + 7).toFixed(1)}" y="${y + 9}" width="${tw.toFixed(1)}" height="${laneH - 18}" fill="#fff" stroke="none"/>`;
        g += `<text x="${(x1 + 14).toFixed(1)}" y="${y + laneH / 2 + 5.5}" ${body} fill="${look === "solid" ? "#fff" : "currentColor"}" stroke="none">${escHtml(b.label)}</text>`;
      }
    }
  });
  const ay = o.lanes.length * (laneH + gap) - 2;
  for (let m = Math.ceil(t0 / 60) * 60; m <= t1; m += 60) {
    const hr = m / 60, even = hr % 2 === 0;
    g += `<path d="M${X(m).toFixed(1)} ${ay}v${even ? 9 : 5}" stroke-width="2"/>`;
    if (even) g += `<text x="${X(m).toFixed(1)}" y="${ay + 26}" text-anchor="middle" ${mono} fill="currentColor" stroke="none">${String(hr).padStart(2, "0")}</text>`;
  }
  if (o.now) {
    const nx = X(o.now);
    g += `<path d="M${nx.toFixed(1)} 0V${ay}" stroke-width="3" stroke-dasharray="3 5"/><path d="M${(nx - 7).toFixed(1)} -2h14l-7 9z" fill="currentColor" stroke="none"/>`;
  }
  const el = h("div", { class: "timeline" });
  el.innerHTML = `<svg viewBox="0 -4 ${W} ${H + 4}" fill="none" stroke="currentColor" stroke-linecap="round"><defs>${HATCH(id)}</defs>${g}</svg>`;
  return el;
};

/** Hourly values as a stepped area (hatched), an optional solid highlight window, and a "now" marker. */
const areaChart = (values: number[], o: { w?: number; h?: number; labels?: [string, number][]; now?: number; window?: [number, number]; marks?: boolean; fmt?: (i: number, v: number) => string; floor?: number; ceil?: number; limit?: [number, string] } = {}) => {
  const W = o.w ?? 720, H = o.h ?? 200, padT = 16, padB = 30, n = values.length, base = H - padB;
  const min = o.floor ?? Math.min(...values), max = o.ceil ?? Math.max(...values), span = max - min || 1;
  const X = (i: number) => (i / n) * W, Y = (v: number) => padT + (1 - (v - min) / span) * (base - padT);
  const id = `hz${++svgSeq}`, cid = `cp${svgSeq}`;
  let line = `M0 ${Y(values[0]).toFixed(1)}`;
  values.forEach((v, i) => { line += `${i ? `V${Y(v).toFixed(1)}` : ""}H${X(i + 1).toFixed(1)}`; });
  const area = `${line}V${base}H0Z`;
  let win = "";
  if (o.window) {
    const [a, b] = [X(o.window[0]), X(o.window[1] + 1)];
    win = `<clipPath id="${cid}"><rect x="${a.toFixed(1)}" y="0" width="${(b - a).toFixed(1)}" height="${H}"/></clipPath>
      <path d="${area}" fill="currentColor" clip-path="url(#${cid})" stroke="none"/>
      <path d="M${a.toFixed(1)} ${padT - 8}V${base}M${b.toFixed(1)} ${padT - 8}V${base}" stroke-width="1.2" stroke-dasharray="2 4"/>`;
  }
  const labels = (o.labels ?? []).map(([t, i]) =>
    `<text x="${X(i).toFixed(1)}" y="${H - 6}" text-anchor="${i === 0 ? "start" : "middle"}" fill="currentColor" stroke="none">${escHtml(t)}</text>`).join("");
  let now = "";
  if (o.now != null) {
    const nx = X(o.now), cv = values[Math.min(n - 1, Math.floor(o.now))];
    now = `<path d="M${nx.toFixed(1)} ${padT - 8}V${base}" stroke-width="1.5"/><rect x="${(nx - 4.5).toFixed(1)}" y="${(Y(cv) - 4.5).toFixed(1)}" width="9" height="9" fill="#fff" stroke-width="1.8"/>`;
  }
  // coordinates at the extremes, and a dimension tag over the highlighted window
  let notes = "";
  if (o.limit) notes += `<path d="M0 ${Y(o.limit[0]).toFixed(1)}H${W}" stroke-width="1" stroke-dasharray="6 4"/><text x="${W - 2}" y="${(Y(o.limit[0]) + 11).toFixed(1)}" text-anchor="end" fill="currentColor" stroke="none" style="font:600 10px var(--mono)">${escHtml(o.limit[1])}</text>`;
  if (o.marks) {
    const fmt = o.fmt ?? ((i: number, v: number) => `${String(i).padStart(2, "0")}:00  ${v}`);
    for (const i of [values.indexOf(Math.min(...values)), values.indexOf(Math.max(...values))]) {
      const mx = (X(i) + X(i + 1)) / 2, my = Y(values[i]), right = mx > W * 0.62;
      notes += `<rect x="${(mx - 3).toFixed(1)}" y="${(my - 3).toFixed(1)}" width="6" height="6" fill="#fff" stroke-width="1.5"/>`;
      notes += `<text x="${(mx + (right ? -9 : 9)).toFixed(1)}" y="${(my + (values[i] === Math.max(...values) ? 4 : -7)).toFixed(1)}" text-anchor="${right ? "end" : "start"}" fill="currentColor" stroke="none" style="font:600 10px var(--mono)">${escHtml(fmt(i, values[i]))}</text>`;
    }
    if (o.window) {
      const [a, b] = [X(o.window[0]), X(o.window[1] + 1)], ty = padT - 8, label = `${o.window[1] - o.window[0] + 1} H`;
      notes += `<path d="M${a.toFixed(1)} ${ty}H${b.toFixed(1)}" stroke-width="1"/><rect x="${((a + b) / 2 - 15).toFixed(1)}" y="${ty - 7}" width="30" height="14" fill="currentColor" stroke="none"/><text x="${((a + b) / 2).toFixed(1)}" y="${ty + 3.5}" text-anchor="middle" fill="#fff" stroke="none" style="font:700 9.5px var(--mono)">↔ ${label}</text>`;
    }
  }
  const el = h("div", { class: "areachart" });
  el.innerHTML = `<svg viewBox="0 0 ${W} ${H}" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" style="font:600 13px var(--mono)">
    <defs>${HATCH(id)}</defs><path d="${area}" fill="url(#${id})" stroke="none"/>${win}
    <path d="${line}" stroke-width="1.5"/><path d="M0 ${base}H${W}" stroke-width="1"/>${labels}${now}${notes}</svg>`;
  return el;
};

/**
 * Bars against a limit line: values at or under `limit` are solid, over it hatched (carbon intensity: green under 100 g).
 * `now` outlines one bar and flags it; `window` brackets a stretch (the cleanest hours); `labels` are [text, bar index].
 */
const limitBars = (values: number[], o: { limit: number; w?: number; h?: number; now?: number; window?: [number, number]; labels?: [string, number][]; limitLabel?: string; windowLabel?: string }) => {
  const W = o.w ?? 720, H = o.h ?? 200, n = values.length, padT = 22, padB = 34, base = H - padB;
  const max = Math.max(...values, o.limit * 1.25), Y = (v: number) => base - (v / max) * (base - padT);
  const slot = W / n, bw = slot * 0.68, id = `hz${++svgSeq}`, X = (i: number) => i * slot + (slot - bw) / 2;
  let g = "";
  values.forEach((v, i) => {
    const y = Y(v), under = v <= o.limit;
    g += `<rect x="${X(i).toFixed(1)}" y="${y.toFixed(1)}" width="${bw.toFixed(1)}" height="${(base - y).toFixed(1)}" fill="${under ? "currentColor" : `url(#${id})`}" stroke-width="${under ? 0 : 1.2}"/>`;
  });
  const ly = Y(o.limit);
  g += `<path d="M0 ${ly.toFixed(1)}H${W}" stroke-width="1.5" stroke-dasharray="6 4"/><text x="${W - 2}" y="${(ly - 5).toFixed(1)}" text-anchor="end" fill="currentColor" stroke="none" style="font:600 11px var(--mono)">${escHtml(o.limitLabel ?? String(o.limit))}</text>`;
  g += `<path d="M0 ${base}H${W}" stroke-width="1"/>`;
  if (o.now != null) {
    const x = X(o.now) + bw / 2;
    g += `<path d="M${(x - 5).toFixed(1)} ${padT - 18}h10l-5 8z" fill="currentColor" stroke="none"/><path d="M${x.toFixed(1)} ${padT - 10}V${base}" stroke-width="1" stroke-dasharray="2 3"/>`;
  }
  if (o.window) {
    const a = X(o.window[0]), b = X(o.window[1]) + bw, ty = base + 7;
    g += `<path d="M${a.toFixed(1)} ${ty - 4}v6h${(b - a).toFixed(1)}v-6" stroke-width="1.5"/>`;
    if (o.windowLabel) g += `<text x="${((a + b) / 2).toFixed(1)}" y="${ty + 15}" text-anchor="middle" fill="currentColor" stroke="none" style="font:700 11px var(--mono)">${escHtml(o.windowLabel)}</text>`;
  }
  for (const [t, i] of o.labels ?? []) g += `<text x="${(X(i) + (i === 0 ? 0 : bw / 2)).toFixed(1)}" y="${H - 3}" text-anchor="${i === 0 ? "start" : "middle"}" fill="currentColor" stroke="none" style="font:600 12px var(--mono)">${escHtml(t)}</text>`;
  const el = h("div", { class: "limitbars" });
  el.innerHTML = `<svg viewBox="0 -2 ${W} ${H + 2}" fill="none" stroke="currentColor" stroke-linejoin="miter"><defs>${HATCH(id)}</defs>${g}</svg>`;
  return el;
};

// ---------------------------------------------- quantities, drawn in unusual ways

/** A numeral sized to fill its module (container query units). Keep tiles inside the tilegrid. */
const bignum = (text: Child, o: { unit?: string; chars?: number } = {}) => {
  const raw = typeof text === "string" || typeof text === "number" ? String(text) : "";
  const chars = o.chars ?? ([...raw].reduce((a, c) => a + (c === "." || c === ":" ? 0.45 : c === "½" ? 1.2 : 1), 0) || 3) + (o.unit ? 0.7 : 0);
  return h("div", { class: "bignum", style: { "--chars": String(Math.max(1, chars)) } }, text, o.unit && h("small", o.unit));
};

const place = <T extends HTMLElement>(node: T, area: string): T => {
  node.set({ style: { gridArea: area } });
  const m = area.match(/^\s*(\d+)\s*\/\s*(\d+)/); // "row / col / ..." names the cell like a drawing zone, e.g. B2
  const id = node.querySelector(".tile-id");
  if (m && id) id.textContent = String.fromCharCode(64 + Number(m[2])) + m[1];
  return node;
};

/** LED-style histogram: columns of dots lit from the bottom. Values 0..1. */
const dotBars = (values: number[], o: { rows?: number } = {}) => {
  const rows = o.rows ?? 10, p = 12;
  let dots = "";
  values.forEach((v, x) => {
    const lit = Math.round(Math.max(0, Math.min(1, v)) * rows);
    for (let y = 0; y < rows; y++) dots += `<circle cx="${((x + 0.5) * p).toFixed(1)}" cy="${((y + 0.5) * p).toFixed(1)}" r="${(p * (y >= rows - lit ? 0.4 : 0.07)).toFixed(2)}"/>`;
  });
  const el = h("div", { class: "dotbars" });
  el.innerHTML = `<svg viewBox="0 0 ${values.length * p} ${rows * p}" fill="currentColor">${dots}</svg>`;
  return el;
};

/** A quantity as a grid of cells: `value` of `total` filled, a fraction shows as a partial cell. */
const waffle = (value: number, total: number, o: { cols?: number; hatch?: number } = {}) => {
  const cols = o.cols ?? Math.ceil(Math.sqrt(total * 2)), rows = Math.ceil(total / cols), c = 22, g = 5, id = `hz${++svgSeq}`;
  let cells = "";
  for (let i = 0; i < total; i++) {
    const x = (i % cols) * (c + g), y = Math.floor(i / cols) * (c + g);
    const full = i + 1 <= value + 1e-9, part = !full && i < value, hatched = !full && !part && i < value + (o.hatch ?? 0);
    cells += `<rect x="${x + 1}" y="${y + 1}" width="${c - 2}" height="${c - 2}" ${full ? 'fill="currentColor"' : hatched ? `fill="url(#${id})"` : 'fill="none"'} stroke-width="2"/>`;
    if (part) cells += `<rect x="${x + 1}" y="${y + 1}" width="${((c - 2) * (value - i)).toFixed(1)}" height="${c - 2}" fill="currentColor"/>`;
  }
  const el = h("div", { class: "waffle" });
  el.innerHTML = `<svg viewBox="0 0 ${cols * (c + g) - g} ${rows * (c + g) - g}" fill="none" stroke="currentColor"><defs>${HATCH(id)}</defs>${cells}</svg>`;
  return el;
};

/** One bar split into labelled parts (solid / hatched / outlined), widths in proportion. */
const stackBar = (parts: { label: string; value: number; look?: "solid" | "hatch" | "outline" }[]) =>
  h("div", { class: "stackbar" },
    h("div", { class: "seg-row" }, parts.map((p) => h("div", { class: `seg ${p.look ?? "solid"}`, style: { flexGrow: p.value } }))),
    h("div", { class: "seg-labels" }, parts.map((p) => h("div", { style: { flexGrow: p.value } }, h("b", String(p.value)), " ", p.label))));

/** A half-circle dial with ticks and the value in the middle. */
const arcGauge = (value: number, o: { min?: number; max?: number } = {}) => {
  const min = o.min ?? 0, max = o.max ?? 100, f = Math.max(0, Math.min(1, (value - min) / (max - min)));
  const cx = 100, cy = 100, r = 76;
  const pt = (t: number, rr = r) => [cx - rr * Math.cos(Math.PI * t), cy - rr * Math.sin(Math.PI * t)];
  const arc = (a: number, b: number) => { const [x1, y1] = pt(a), [x2, y2] = pt(b); return `M${x1.toFixed(1)} ${y1.toFixed(1)}A${r} ${r} 0 0 1 ${x2.toFixed(1)} ${y2.toFixed(1)}`; };
  let ticks = "";
  for (let i = 0; i <= 10; i++) { const [ax, ay] = pt(i / 10, r + 9), [bx, by] = pt(i / 10, r + (i % 5 === 0 ? 20 : 15)); ticks += `M${ax.toFixed(1)} ${ay.toFixed(1)}L${bx.toFixed(1)} ${by.toFixed(1)}`; }
  const el = h("div", { class: "arcgauge" });
  el.innerHTML = `<svg viewBox="0 -6 200 122" fill="none" stroke="currentColor" stroke-linecap="butt">
    <path d="${arc(0, 1)}" stroke-width="2"/><path d="${ticks}" stroke-width="2"/>
    ${f > 0.004 ? `<path d="${arc(0, f)}" stroke-width="13"/>` : ""}
    <text x="100" y="98" text-anchor="middle" fill="currentColor" stroke="none" style="font:300 38px var(--body)">${Math.round(value)}</text>
    </svg>`;
  return el;
};

// ------------------------------------------------------ breaking the grid: bands, giants, marks

/** A full-bleed black band, edge to edge. "top"/"bottom" also reach the screen edge. White on black inside. */
const band = (edge: "top" | "bottom" | "mid", ...a: Arg[]) => h("div", { class: `band ${edge}` }, ...a);

/**
 * An oversized numeral (default 240px). Put it in a tile with {bleed: true}: it sits on the module's
 * floor and the module's edge crops it, on purpose. Keep the real value in the label or count.
 */
const giant = (value: Child, o: { size?: number; unit?: string } = {}) =>
  h("div", { class: "giant", style: { "--size": `${o.size ?? 240}px` } }, value, o.unit && h("small", o.unit));

type SigList = [SigKind, any?][] | (() => [SigKind, any?][]);
// ------------------------------------------------ screens and the tab bar
// A screen is a named function that builds the whole scene. The set of screens is the tab bar along the bottom of the display; switching
// is `ui.go("energy")` or, from outside, writing the key "ui.screen" (POST /screen?name=energy): immediate, never journaled.
// A non-home screen returns home after `hold` ms (key "ui.hold", default 2 min) so the wall display never gets stuck on a visitor's request.

interface ScreenDef { name: string; tab: string; build: () => void }
const screenDefs = new Map<string, ScreenDef>();
let currentScreen: string | null = null, screenJobs: number[] = [], returnTimer: number | undefined;
let homeScreen = "home";
const HOLD_DEFAULT = 120_000;

/** Register a screen. `tab` is its label in the tab bar (omit to leave it off the bar: a hidden screen). The first one is home. */
function defineScreen(name: string, build: () => void, o: { tab?: string; hidden?: boolean } = {}): void {
  if (!screenDefs.size) homeScreen = name;
  screenDefs.set(name, { name, tab: o.hidden ? "" : o.tab ?? name.toUpperCase(), build });
}

/** Show a screen now. Its timers and bindings replace the previous screen's. */
function go(name: string, o: { hold?: number } = {}): boolean {
  const def = screenDefs.get(name);
  if (!def) { console.warn("ui.go: no screen named", name, "- have:", [...screenDefs.keys()].join(", ")); return false; }
  for (const id of screenJobs) jobs.delete(id);                        // the old screen's timers die with it
  const before = new Set(jobs.keys());
  def.build();
  screenJobs = [...jobs.keys()].filter((id) => !before.has(id));
  currentScreen = name;
  realClearTimeout(returnTimer);
  const hold = o.hold ?? (data.get("ui.hold")?.v as number | undefined) ?? HOLD_DEFAULT;
  if (name !== homeScreen && hold > 0) returnTimer = realSetTimeout(() => go(homeScreen), hold) as unknown as number;
  if (frameOpts) drawFrame();
  window.dispatchEvent(new CustomEvent("ui:screen", { detail: name }));
  return true;
}
/** Show `name` (or home) unless something is already showing: the page's first paint. */
const start = (name?: string): void => { if (!currentScreen) go(name ?? homeScreen); };

let frameOpts: { title?: string; sheet?: string; tl?: SigList; tr?: SigList; tabs?: boolean } | null = null, frameHooked = false;

/** Draw the drawing-sheet frame: corner targets, sigil clusters in the top corners, zone letters over the grid columns and numbers beside its rows, a terse title block. */
function drawFrame(): void {
  document.getElementById("ui-frame")?.remove();
  if (!frameOpts) return;
  const w = screen.offsetWidth, hh = screen.offsetHeight;
  const sr = screen.getBoundingClientRect(), k = sr.width / w || 1;
  const gridEl = root.querySelector(".tilegrid") as HTMLElement | null;
  const gr = gridEl?.getBoundingClientRect();
  const gx = gr ? (gr.left - sr.left) / k : 24, gy = gr ? (gr.top - sr.top) / k : 26;
  // the tracks as laid out (px before the scene's scale), so every tick sits exactly on a tile edge
  const cs = gridEl ? getComputedStyle(gridEl) : null, gs = gr && gridEl ? gr.width / k / gridEl.offsetWidth : 1;
  const tracks = (v?: string) => (v ?? "").split(" ").map(parseFloat).filter((n) => !isNaN(n)).map((n) => n * gs);
  const colT = tracks(cs?.gridTemplateColumns), rowT = tracks(cs?.gridTemplateRows);
  const cg = parseFloat(cs?.columnGap ?? "8") * gs || 0, rg = parseFloat(cs?.rowGap ?? "8") * gs || 0;
  const f = (n: number) => n.toFixed(1), txt = (x: number, y: number, t: string, anchor = "middle") =>
    `<text x="${f(x)}" y="${f(y)}" text-anchor="${anchor}" fill="currentColor" stroke="none">${escHtml(t)}</text>`;
  let d = "", marks = "", x0 = gx, y0 = gy;
  colT.forEach((cw, i) => {
    marks += txt(x0 + cw / 2, 24, "ABCDEFGH"[i] ?? "");
    d += `M${f(x0)} 26v2M${f(x0 + cw)} 26v2`;
    x0 += cw + cg;
  });
  rowT.forEach((rh, j) => {
    marks += txt(11, y0 + rh / 2 + 3.5, String(j + 1));
    d += `M19 ${f(y0)}h4M19 ${f(y0 + rh)}h4`;
    y0 += rh + rg;
  });
  // sigils sit in the top corners: left = the machine, right = the world
  const sz = 11, gap = 7, sc = sz / 12;
  const cluster = (items: [SigKind, any?][], x0: number, dir: 1 | -1) =>
    items.map(([kind, v], i) => `<g transform="translate(${f(dir === 1 ? x0 + i * (sz + gap) : x0 - (i + 1) * sz - i * gap)} 3) scale(${sc.toFixed(3)})">${sigPaths(kind, v)}</g>`).join("");
  const items = (l?: SigList) => (typeof l === "function" ? l() : l ?? []);   // a function is re-read at every redraw: live sigils
  marks += cluster(items(frameOpts.tl), 28, 1) + cluster(items(frameOpts.tr), w - 26, -1);
  const t = new Date(), clock = `${String(t.getHours()).padStart(2, "0")}:${String(t.getMinutes()).padStart(2, "0")}`;
  const tb = { x: 24, y: hh - 22, w: w - 48, h: 15 };
  const tabs = frameOpts.tabs ? [...screenDefs.values()].filter((sc) => sc.tab) : [];
  if (tabs.length) {
    // the screens you can ask for, as tabs along the bottom: the current one inverted. Display only: say "show energy".
    const clockW = 66, cw = (tb.w - clockW) / tabs.length;
    d += `M${tb.x} ${tb.y}h${tb.w}v${tb.h}h${-tb.w}zM${f(tb.x + tb.w - clockW)} ${tb.y}v${tb.h}`;
    tabs.forEach((sc, i) => {
      const x = tb.x + i * cw, on = sc.name === currentScreen;
      if (i) d += `M${f(x)} ${tb.y}v${tb.h}`;
      // the layer is blended with "difference": white shows black, and black text on the white block shows white on black
      if (on) marks += `<rect x="${f(x)}" y="${tb.y}" width="${f(cw)}" height="${tb.h}" fill="currentColor" stroke="none"/>`;
      marks += `<text x="${f(x + cw / 2)}" y="${f(tb.y + 10.5)}" text-anchor="middle" fill="${on ? "#000" : "currentColor"}" stroke="none" style="font-weight:${on ? 700 : 500}">${i + 1} ${escHtml(sc.tab)}</text>`;
    });
    marks += txt(tb.x + tb.w - clockW / 2, tb.y + 10.5, clock);
  } else {
    d += `M${tb.x} ${tb.y}h${tb.w}v${tb.h}h${-tb.w}zM${tb.x + 54} ${tb.y}v${tb.h}M${tb.x + tb.w - 92} ${tb.y}v${tb.h}`;
    marks += txt(tb.x + 7, tb.y + 10.5, "D1", "start");
    marks += txt(tb.x + 62, tb.y + 10.5, (frameOpts.title ?? "").toUpperCase(), "start");
    marks += txt(tb.x + tb.w - 85, tb.y + 10.5, `${frameOpts.sheet ?? "1/1"}  ${clock}`, "start");
  }
  const target = (cx: number, cy: number) => `<circle cx="${cx}" cy="${cy}" r="3.2"/><path d="M${cx - 6} ${cy}h12M${cx} ${cy - 6}v12"/>`;
  const el = h("div", { id: "ui-frame" });
  el.innerHTML = `<svg width="${w}" height="${hh}" viewBox="0 0 ${w} ${hh}" fill="none" stroke="currentColor" stroke-width="1">${target(12, 11)}${target(w - 12, 11)}<path d="${d}"/>${marks}</svg>`;
  screen.append(el);
}

/**
 * The drawing-sheet frame, in the screen margin: registration targets, two corner clusters of sigils (tl, tr),
 * column letters (A-F) and row numbers (1-3) matching the tilegrid so every module has an address like "B2",
 * and a terse title block. ui.frame({title, sheet, tl, tr}) turns it on, ui.frame(false) off.
 */
function frame(opt: boolean | { title?: string; sheet?: string; tl?: SigList; tr?: SigList; tabs?: boolean } = true): void {
  frameOpts = opt === false ? null : typeof opt === "object" ? opt : {};
  drawFrame();
  if (!frameHooked) {
    frameHooked = true;
    window.addEventListener("ui:settled", () => window.setTimeout(drawFrame, 0));
  }
}


// ---------------------------------------------------------- live widgets

/** Run fn now and every `ms` while `el` stays mounted (stops once removed). */
function live<T extends Element>(el: T, fn: (el: T) => void, ms: number): T {
  let seen = false;
  const tick = () => {
    if (el.isConnected) seen = true;
    else if (seen) return void clearInterval(id);
    fn(el);
  };
  const id = setInterval(tick, ms);
  fn(el);
  return el;
}
const setText = (el: Element, t: string) => { if (el.textContent !== t) el.textContent = t; };

const clock = (o: { seconds?: boolean; date?: boolean; hour12?: boolean } = {}) => {
  const el = h("div", { class: "clock" }, h("div", { class: "time" }), o.date !== false && h("div", { class: "date" }));
  return live(el, () => {
    const d = new Date();
    setText(el.q(".time")!, d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: o.seconds ? "2-digit" : undefined, hour12: o.hour12 ?? false }));
    const dt = el.q(".date");
    if (dt) setText(dt, d.toLocaleDateString([], { weekday: "long", day: "numeric", month: "long" }));
  }, o.seconds ? 1000 : 10_000);
};

const countdown = (to: Date | string | number, label?: string) => {
  const target = new Date(to).getTime();
  const el = h("div", { class: "countdown" }, h("div", { class: "value" }), label && h("div", { class: "label" }, label));
  return live(el, () => {
    const s = Math.max(0, Math.floor((target - Date.now()) / 1000));
    const d = Math.floor(s / 86400), hh = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60);
    setText(el.q(".value")!, d ? `${d}d ${hh}h` : hh ? `${hh}h ${m}m` : `${m}m`);
  }, 30_000);
};

// ------------------------------------------------------------- ui object

const screen: HTMLElement = (document.getElementById("screen") as HTMLElement) ?? document.body.appendChild(h("main", { id: "screen" }));
screen.id = "screen";
/** The scene: everything built goes in here, and it is always made to fit the screen (see adapt()). */
const root: HTMLElement = h("div", { id: "scene" });
screen.replaceChildren(root);

// Fonts: one monospace voice. A bundled file can be added to FONT_FILES later (crates/ui/fonts);
// until then the system monospace is used (on the NixOS box, install the font in the system config).
const FONT_BASE: string = (window as any).UI_FONT_BASE ?? "/fonts";
// [family, file, weight range, width range]. All variable: any weight in range, and "UI Cond" is Archivo pinned to its narrowest width
// (Inter has no width axis; use another wdth family here to get one for the sans).
const FONT_FILES: [string, string, string, string][] = [
  ["Berkeley Mono", "berkeley.woff2", "100 900", "100%"],
  ["Inter", "InterVariable.woff2", "100 900", "100%"],
  ["UI Cond", "ArchivoVariable.woff2", "100 900", "62% 62%"],
];
const fontFormat = (file: string) => (file.endsWith(".woff2") ? ["woff2", "font/woff2"] : file.endsWith(".woff") ? ["woff", "font/woff"] : ["truetype", "font/ttf"]);
const fontFaceCss = () =>
  FONT_FILES.map(([fam, file, wt, st]) => `@font-face{font-family:"${fam}";src:url(${FONT_BASE}/${file}) format("${fontFormat(file)[0]}");font-weight:${wt};font-stretch:${st};font-display:block}`).join("");
/** Resolves once the fonts are loaded. Layout depends on them, so measure (verify) only after this. */
const fontsReady: Promise<void> = Promise.all(FONT_FILES.map(([fam]) => document.fonts.load(`500 1em "${fam}"`))).then(() => undefined, () => undefined);

const style = h("style", `${fontFaceCss()}
#screen{width:800px;height:480px;box-sizing:border-box;padding:0;overflow:hidden;background:#fff;color:#000;
  --mono:"Berkeley Mono","JetBrains Mono","IBM Plex Mono",ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;
  --sans:Inter,"Helvetica Neue",Helvetica,Arial,system-ui,sans-serif;
  --body:var(--sans);--display:var(--sans);
  --cond:"UI Cond","Arial Narrow","Helvetica Neue",Arial,sans-serif;
  --mid:#000;--accent:#000;
  position:relative;overflow-wrap:anywhere;
  font:500 14px/1.38 var(--sans);font-variant-numeric:tabular-nums}
#screen *{box-sizing:border-box;border-radius:0}
#scene{display:flex;flex-direction:column;gap:8px;transform-origin:0 0;padding:26px 24px 30px}
#scene>*{flex-shrink:0}
#scene>.fill-region{flex:1 1 0;min-height:0;display:flex;flex-direction:column;gap:8px}
#scene [data-dropped]{display:none!important}
#screen svg{max-width:100%;height:auto}
#screen .clamp{display:-webkit-box;-webkit-box-orient:vertical;-webkit-line-clamp:var(--lines,2);overflow:hidden}
#screen h1,#screen h2,#screen h3,#screen h4,#screen p,#screen ul,#screen ol,#screen figure,#screen blockquote{margin:0}
#screen h1{font:600 30px/1.1 var(--sans);letter-spacing:-.02em}
#screen h2{font:600 22px/1.15 var(--sans);letter-spacing:-.01em}
#screen h3{font:600 17px/1.2 var(--sans)}
#screen h4{font:600 13px/1.2 var(--sans);text-transform:uppercase;letter-spacing:.06em}
#screen b,#screen strong{font-weight:700}
#screen mark{background:#000;color:#fff;padding:0 .25em}
#screen small{font-size:11px;color:var(--mid);font-weight:500;letter-spacing:.02em}
#screen code{font:inherit;border:1px solid #000;padding:0 .3em}
#screen ul{padding-left:1.4em}
#screen ol{padding-left:2.1em}
#screen li{padding-left:.15em}
#screen li+li{margin-top:2px}
#screen hr{border:0;border-top:1px solid #000;width:100%;margin:0}
#screen .row{display:flex;gap:8px;align-items:stretch}
#screen .row>*{flex:1 1 0;min-width:0}
#screen .row.fit{flex-wrap:wrap}
#screen .row.fit>*{flex:none}
#screen .row>.tearoff,#screen .row>.presence,#screen .row>.dial{flex:none}
#screen .col{display:flex;flex-direction:column;gap:6px}
#screen .grid{display:grid;grid-template-columns:repeat(var(--cols,2),1fr);gap:8px}
#screen .card{border:1px solid #000;padding:8px 10px}
#screen .card-title,#screen .label{font:600 11px/1.2 var(--sans);letter-spacing:.08em;text-transform:uppercase;margin-bottom:6px}
#screen .big{font:500 48px/1 var(--sans);letter-spacing:-.03em}
#screen .badge{display:inline-flex;align-items:center;gap:5px;border:1px solid #000;padding:0 .5em;font:600 12px/1.5 var(--sans)}
#screen .stat .value,#screen .countdown .value{font:700 38px/1 var(--cond);letter-spacing:0}
#screen .stat .value small{font-size:13px;margin-left:3px;font-weight:600}
#screen .stat .delta{font-size:12px;font-weight:600}
#screen table{border-collapse:collapse;width:100%}
#screen th{text-align:left;border-bottom:1px solid #000;padding:2px 8px 3px 0;font:600 11.5px var(--sans);text-transform:uppercase;letter-spacing:.08em}
#screen td{padding:3px 8px 3px 0;border-bottom:1px solid #000;white-space:nowrap}
#screen tr:last-child td{border-bottom:0}
#screen .kv{display:flex;justify-content:space-between;gap:10px;border-bottom:1px solid #000;padding:2px 0}
#screen .kv .k{font:600 11.5px var(--mono);letter-spacing:.06em;text-transform:uppercase;align-self:center} #screen .kv .v{font-weight:700;text-align:right}
#screen .kvgrid{display:grid;grid-template-columns:repeat(var(--cols,2),1fr);column-gap:14px;width:100%}
#screen .skybox{flex:none;border:1px solid #000;background:linear-gradient(180deg,#fff 0%,#fff 36%,#000 100%);display:flex;flex-direction:column;align-items:center;justify-content:space-between;padding:6px 0 5px}
#screen .skybox b{font:600 20px/1 var(--sans);color:#fff}
#screen .progress .track{height:12px;border:1px solid #000;padding:2px}
#screen .progress .fill{height:100%;background:#000;min-width:6px}
#screen .bars{display:flex;align-items:stretch;gap:6px}
#screen .bars .bar{flex:1;display:flex;flex-direction:column;justify-content:flex-end;align-items:center;gap:3px;min-width:0}
#screen .bars .fill{width:60%;background:#000;min-height:2px}
#screen .bar-label{font:500 10.5px/1.1 var(--mono);text-align:center}
#screen .clock .time{font:700 84px/.9 var(--cond);letter-spacing:-.01em}
#screen .clock .date{font:600 14px var(--sans)}
#screen .agenda .event{display:flex;gap:10px;padding:3px 0;border-bottom:1px solid #000}
#screen .agenda .event:last-child{border:0}
#screen .agenda .time{font:600 13px var(--mono);width:48px;flex:none}
#screen .agenda small{display:block}
#screen .weather{display:flex;align-items:center;gap:12px;flex:1;height:100%}
#screen .weather .temp{font:700 44px/1 var(--cond)}
#screen .weather .cond{font:600 13px var(--sans)}
#screen .weather .range{margin-left:auto;font:500 12px var(--sans)}
#screen .card:has(>.weather),#screen section:has(>.weather){display:flex;align-items:center}
#screen .spark{display:inline-block;vertical-align:middle}
#screen .icon{display:inline-flex;vertical-align:middle;line-height:0}
#screen .icon svg{display:block}
#screen .icon.glyph{font-family:var(--sans);line-height:1}
#screen .spacer{flex:1 1 auto}
#screen .center{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;flex:1}
#screen .chips{display:flex;flex-wrap:wrap;gap:6px}
#screen .topbar{display:flex;justify-content:space-between;align-items:center;border-bottom:1px solid #000;padding-bottom:5px}
#screen .topbar .t{font:600 18px var(--sans)}
#screen .topbar .r{font:500 12px var(--sans);display:flex;align-items:center;gap:8px}
#screen .banner{display:flex;align-items:center;gap:10px;background:#000;color:#fff;padding:7px 12px;font-weight:600}
#screen .banner .icon{font-size:20px}
#screen .quote{border-left:3px solid #000;padding-left:12px}
#screen .quote p{font:600 18px/1.3 var(--sans)}
#screen .quote footer{font:500 11px var(--sans);margin-top:6px}
#screen .quote footer::before{content:"— "}
#screen .trend{font-weight:700}
#screen .todo .item{display:flex;align-items:center;gap:8px;padding:2px 0}
#screen .todo .box{width:15px;height:15px;border:1px solid #000;display:inline-grid;place-items:center;flex:none}
#screen .todo .done .box{background:#000;color:#fff}
#screen .todo .done span:last-child{text-decoration:line-through}
#screen .gauge{display:inline-flex;flex-direction:column;align-items:center}
#screen .gauge .label{margin:4px 0 0}
#screen .month-title{font:600 14px var(--sans);margin-bottom:6px}
#screen .month-grid{display:grid;grid-template-columns:repeat(7,1fr);gap:1px;text-align:center}
#screen .month-grid .dow{font:600 10.5px var(--sans)}
#screen .month-grid .day{font:500 13px var(--sans);padding:2px 0}
#screen .month-grid .day.mark{text-decoration:underline}
#screen .month-grid .day.today{background:#000;color:#fff}
#screen .chart{display:block}
#screen .image{display:block;filter:grayscale(1) contrast(1.3)}
#screen .say{display:flex;align-items:stretch;gap:0}
#screen .say .avatar{width:38px;background:#000;color:#fff;display:grid;place-items:center;flex:none}
#screen .say .bubble{flex:1;border:1px solid #000;border-left:0;padding:7px 12px;font:600 15px/1.3 var(--sans)}
#screen .sunarc svg{display:block}
#screen .sunarc .labels{display:flex;justify-content:space-between;align-items:center;font:500 12px var(--sans);margin-top:3px}
#screen .sunarc .labels span{display:inline-flex;align-items:center;gap:5px}
#screen .hourly{display:flex;justify-content:space-between;gap:2px}
#screen .hourly .hr{flex:1;display:flex;flex-direction:column;align-items:center;gap:2px}
#screen .hourly .t{font:500 11px var(--mono)}
#screen .hourly .temp{font:600 14px var(--mono)}
#screen .hourly .pop{width:70%;height:5px;border:1px solid #000}
#screen .hourly .pop i{display:block;height:100%;background:#000}
#screen .habit{display:grid;grid-auto-flow:column;grid-template-rows:repeat(var(--rows,7),12px);gap:3px}
#screen .habit .dot{width:12px;height:12px;border:1px solid #000}
#screen .habit .l1{background:linear-gradient(#000,#000) center/4px 4px no-repeat}
#screen .habit .l2{background:linear-gradient(#000,#000) center/8px 8px no-repeat}
#screen .habit .l3{background:#000}
#screen .tearoff{position:relative;width:110px;border:1px solid #000;text-align:center;background:#fff}
#screen .tearoff .mon{background:#000;color:#fff;font:600 12px var(--sans);letter-spacing:.2em;padding:7px 0 4px}
#screen .tearoff .day{font:700 64px/1 var(--cond);padding:6px 0 0}
#screen .tearoff .dow{font:600 12px var(--sans);padding:0 0 8px}
#screen .tearoff .rings{display:none}
#screen .note{border:1px solid #000;padding:8px 12px}
#screen .note .label{margin-bottom:4px}
#screen .note p{font:600 15px/1.3 var(--sans)}
#screen .presence{display:flex;gap:8px}
#screen .presence .who{display:flex;flex-direction:column;align-items:center;gap:2px}
#screen .presence .face{width:32px;height:32px;display:grid;place-items:center;font:700 15px var(--sans)}
#screen .presence .home .face{background:#000;color:#fff}
#screen .presence .away .face{border:1px dashed #000}
#screen .sd{display:inline-block;width:9px;height:9px;border-radius:50%!important;margin-left:6px;vertical-align:middle;flex:none}
#screen .sd.ok,#screen .sd.on{background:currentColor}
#screen .sd.warn{width:11px;height:11px;background:var(--accent-fill,transparent);border:2px solid currentColor}
#screen .sd.off{border:1px dashed currentColor}
#screen .sd.none{visibility:hidden}
#screen .menu .sd,#screen .hero .sd{margin:0 9px 0 0}
#screen .menu .ml{gap:0}
#screen .tile{position:relative;display:flex;flex-direction:column;min-height:0;border:1px solid #000;padding:7px 11px 8px;overflow:hidden}
#screen .tile-head{display:flex;justify-content:space-between;align-items:baseline;gap:8px;margin-bottom:5px}
#screen .tile-label{font:600 14px/1.2 var(--sans);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
#screen .tile-label .icon{margin-right:6px;vertical-align:-2px}
#screen .tile-id{font:500 11px var(--mono);letter-spacing:.06em;opacity:1;flex:none}
#screen .tile-body{container-type:size;flex:1;display:flex;align-items:center;justify-content:flex-start;font:500 14.5px var(--sans);min-height:0}
#screen .tile-body>*{min-width:0}
#screen .tile-count{font:500 12.5px/1.3 var(--mono);margin-top:5px;border-top:1px solid #000;padding-top:3px}
#screen .tile.inv{background:#000;color:#fff;--mid:#fff;--accent:#fff}
#screen .tile.inv .tile-count{border-top-color:#fff}
#screen .tile.level .water{position:absolute;inset:0;background:#000}
#screen .tile.level .tile-head{position:relative;z-index:1;align-self:flex-start;background:#fff;padding:0 6px 0 0}
#screen .tile.level .tile-count{position:absolute;left:8px;bottom:8px;border:0;margin:0;padding:0}
#screen .tile .chip{display:inline-block;color:#fff;mix-blend-mode:difference;padding:0;font:600 15px/1.1 var(--mono)}
#screen .tile.sky{background:linear-gradient(180deg,#fff 0%,#fff 40%,#000 100%)}
#screen .tile.fill .tile-body>*{flex:1;min-width:0}
#screen .tile.fill svg{display:block;width:100%;height:auto;max-height:100%}
#screen .tile.open{border:0;border-top:1px solid #000;padding:5px 0 0}
#screen .tile.list .tile-body{align-items:stretch;display:block}
#screen .tile.list .mrow{padding:2px 0;font-size:14.5px}
#screen .tile .big-num{font:700 42px/1 var(--cond);letter-spacing:0;white-space:nowrap}
#screen .tile .big-num small{font:600 12px var(--sans);margin-left:3px}
#screen .tile .ask{font:600 20px/1.25 var(--sans)}
#screen .tilegrid{display:grid;grid-template-columns:repeat(6,1fr);grid-template-rows:repeat(3,1fr);gap:8px;flex:1 1 0;min-height:0}
#screen .tilegrid>*{min-width:0;min-height:0;overflow:hidden}
#screen .tile.bleed .tile-body{align-items:flex-end;overflow:visible}
#screen .giant{font:700 var(--size,240px)/.78 var(--cond);letter-spacing:-.02em;white-space:nowrap;margin:0 0 -.15em -.03em;flex:none}
#screen .giant small{font:600 .085em var(--sans);margin-left:.25em;vertical-align:top;color:inherit}
#screen .wxbody{display:flex;flex-direction:column;align-items:center}
#screen .dial{position:relative;flex:none}
#screen .dial svg{display:block}
#screen .dial .mid{position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center}
#screen .dial .mid b{font:600 14px/1 var(--sans)}
#screen .dial .mid small{font-size:9px}
#screen .ruler svg{display:block}
#screen .stats{display:flex;border-top:1px solid #000;border-bottom:1px solid #000}
#screen .stats .cell{flex:1;padding:5px 9px;min-width:0}
#screen .stats .cell+.cell{border-left:1px solid #000}
#screen .stats .k{font:600 11.5px var(--mono);letter-spacing:.08em;text-transform:uppercase}
#screen .stats .v{font:700 28px/1.1 var(--cond)}
#screen .pills{display:flex;flex-wrap:wrap;gap:5px}
#screen .pill{display:inline-flex;align-items:center;gap:5px;border:1px solid #000;padding:1px 10px;font:600 12px/1.5 var(--sans)}
#screen .pill.on{background:#000;color:#fff}
#screen .slider{position:relative;height:30px;border:1px solid #000;overflow:hidden}
#screen .slider .sl-fill{position:absolute;inset:0 auto 0 0;width:var(--p);background:#000}
#screen .slider .sl-txt{position:absolute;inset:0;display:flex;justify-content:space-between;align-items:center;padding:0 10px;font:600 12px var(--sans)}
#screen .slider .sl-txt b{font:600 14px var(--sans)}
#screen .slider .sl-txt.light{color:#fff;clip-path:inset(0 calc(100% - var(--p)) 0 0)}
#screen .menu .mrow{display:flex;justify-content:space-between;align-items:center;gap:10px;padding:3px 0;border-bottom:1px solid #000;font:600 14.5px var(--sans)}
#screen .menu .mrow:last-child{border:0}
#screen .menu .ml{display:inline-flex;align-items:center}
#screen .menu .mv{font:500 13px var(--sans);display:inline-flex;align-items:center}
#screen .device{display:flex;align-items:center;gap:9px;padding:3px 0;border-bottom:1px solid #000}
#screen .device:last-child{border:0}
#screen .device .dv-icon{width:28px;height:28px;border:1px solid #000;display:grid;place-items:center;flex:none}
#screen .device.on .dv-icon{background:#000;color:#fff}
#screen .device .dv-txt{flex:1;min-width:0}
#screen .device .dv-name{font:600 14.5px/1.15 var(--sans)}
#screen .device .switch{position:relative;width:38px;height:18px;border:1px solid #000;flex:none}
#screen .device .switch i{position:absolute;top:2px;left:2px;width:12px;height:12px;background:#000}
#screen .device.on .switch{background:#000}
#screen .device.on .switch i{left:auto;right:2px;background:#fff}
#screen .tile.list .device{padding:1px 0;gap:8px}
#screen .tile.list .device .dv-icon{width:22px;height:22px}
#screen .tile.list .device .dv-icon svg{width:14px;height:14px}
#screen .tile.list .device .dv-name{font-size:14px}
#screen .tile.list .device .switch{width:32px;height:15px}
#screen .tile.list .device .switch i{width:9px;height:9px}
#screen .hero .num{font:600 92px/.88 var(--cond);letter-spacing:-.01em;white-space:nowrap}
#screen .hero sup{font-size:.4em;vertical-align:top;line-height:1.1;margin-left:2px;letter-spacing:0}
#screen .hero .hstatus{display:flex;align-items:center;font:600 14.5px var(--sans)}
#screen .hero .hstatus .sd{margin:0 8px 0 0}
#screen .hero .hstatus small{margin-left:10px}
#screen .meta{display:flex;justify-content:space-between;align-items:center;font:600 11px var(--sans);letter-spacing:.1em;text-transform:uppercase}
#screen .panel{position:relative;overflow:hidden;border:1px solid #000;padding:10px 12px}
#screen .panel.inv{background:#000;color:#fff;--mid:#fff;--accent:#fff}
#screen .panel.sky{background:linear-gradient(180deg,#fff 0%,#fff 40%,#000 100%)}
#screen .panel.dots{background:radial-gradient(#000 1px,transparent 1.4px) 0 0/12px 12px}
#screen .reg{position:relative}
#screen .reg::before{content:"";position:absolute;inset:-6px;pointer-events:none;background:
  linear-gradient(#000,#000) 0 0/10px 1px no-repeat,linear-gradient(#000,#000) 0 0/1px 10px no-repeat,
  linear-gradient(#000,#000) 100% 0/10px 1px no-repeat,linear-gradient(#000,#000) 100% 0/1px 10px no-repeat,
  linear-gradient(#000,#000) 0 100%/10px 1px no-repeat,linear-gradient(#000,#000) 0 100%/1px 10px no-repeat,
  linear-gradient(#000,#000) 100% 100%/10px 1px no-repeat,linear-gradient(#000,#000) 100% 100%/1px 10px no-repeat}
#screen .dotclock svg{display:block}
#screen .cline{display:flex;align-items:stretch;gap:0;border:1px solid #000}
#screen .cline .avatar{width:30px;background:#000;color:#fff;display:grid;place-items:center;flex:none}
#screen .cline .txt{flex:1;font:600 14px/1.3 var(--sans);padding:6px 10px}
#screen .cline .t{font:500 11px var(--sans);padding:6px 10px;border-left:1px solid #000}
#screen .limitbars svg,#screen .daystrip svg,#screen .timeline svg,#screen .areachart svg{display:block;width:100%;height:auto}
#screen .dotbars svg,#screen .waffle svg,#screen .arcgauge svg{display:block;width:100%;height:auto}
#screen .tile.fill .tile-body>.arcgauge{flex:none;width:118px;margin:0 auto}
#screen .tile .net{font:600 14px/1.3 var(--sans);white-space:nowrap}
#screen .tile .net div{display:flex;align-items:center;gap:5px}
#screen .stackbar .seg-row{display:flex;gap:3px;height:22px}
#screen .stackbar .seg{border:1px solid #000;min-width:6px}
#screen .stackbar .seg.solid{background:#000}
#screen .stackbar .seg.hatch{background:repeating-linear-gradient(135deg,#000 0 1px,#fff 1px 5px)}
#screen .stackbar .seg-labels{display:flex;gap:3px;margin-top:5px}
#screen .stackbar .seg-labels>div{min-width:0;font:500 11px var(--mono);white-space:nowrap}
#screen .band{display:flex;align-items:center;justify-content:space-between;gap:12px;background:#000;color:#fff;--mid:#fff;--accent:#fff;
  margin-left:-24px;margin-right:-24px;padding:7px 24px;font:600 11px var(--sans);letter-spacing:.1em;text-transform:uppercase}
#screen .band.top{margin-top:-26px;padding-top:9px}
#screen .band.bottom{margin-bottom:-30px;padding-bottom:12px}
#screen .band span{display:inline-flex;align-items:center}
#screen .statusbar{display:flex;align-items:stretch;border:1px solid #000;height:24px;font:600 12.5px var(--sans);letter-spacing:.06em;text-transform:uppercase}
#screen .statusbar>*{display:flex;align-items:center;gap:6px;padding:0 10px;white-space:nowrap}
#screen .statusbar>*+*{border-left:1px solid #000}
#screen .statusbar>.sb-fill{flex:1;padding:0}
#screen .statusbar>.sb-mark{background:#000;color:#fff;padding:0 9px}
#screen .segbar{height:11px;width:100%;border:1px solid #000;background:radial-gradient(#000 .75px,transparent 1px) 0 0/3px 3px}
#screen .segbar i{display:block;height:100%;background:#000}
#screen .readouts{display:grid;grid-template-columns:auto 1fr auto;gap:3px 10px;align-items:center;width:100%}
#screen .readouts .rl{font:600 12px var(--mono);letter-spacing:.06em;text-transform:uppercase}
#screen .readouts .rv{font:600 13.5px var(--mono);text-align:right;white-space:nowrap}
#screen .readouts .rv small{font-size:10px;margin-left:2px}
#screen .header{display:flex;align-items:flex-end;gap:14px;padding-bottom:6px;border-bottom:1px dotted #000}
#screen .header .hk{font:500 11.5px/1.25 var(--mono);text-transform:uppercase;letter-spacing:.06em;width:104px;flex:none}
#screen .header .hc{flex:1;font:700 56px/.8 var(--cond);letter-spacing:-.01em}
#screen .header .hr{text-align:right;font:500 10.5px/1.3 var(--mono);text-transform:uppercase;letter-spacing:.04em}
#screen .ruled{display:grid;grid-template-columns:repeat(var(--n,3),1fr);border:1px solid #000}
#screen .ruled .rcell{padding:5px 9px 6px;border-left:1px solid #000;min-width:0}
#screen .ruled .rcell:first-child{border-left:0}
#screen .ruled .rt{font:700 11.5px var(--sans);letter-spacing:.08em;text-transform:uppercase;border-bottom:1px solid #000;padding-bottom:3px;margin-bottom:3px}
#screen .ruled .rr{display:flex;justify-content:space-between;gap:8px;font:500 11.5px/1.4 var(--mono);text-transform:uppercase;letter-spacing:.03em}
#screen .ruled .rr b{font-weight:700;text-align:right;white-space:nowrap}
#screen .tonestrip{display:flex;height:13px;border:1px solid #000}
#screen .tonestrip i{flex:1}
#screen .routes svg{display:block;width:100%;height:auto}
#screen .tile.iconic .tile-body{justify-content:center}
#screen .tile.has-waves>*:not(.waves){position:relative;z-index:1}
#screen .waves{position:absolute;inset:0;z-index:0}
#screen .waves svg{display:block;width:100%;height:100%;max-width:none}
#screen .tile.has-waves .tile-count{background:transparent;color:#fff;border-top:0}
#screen .tile.has-waves .tile-label,#screen .tile.has-waves .tile-id{color:#000}
#screen .tile.legend{overflow:visible;padding-top:12px}
#screen .tile.legend .tile-head{position:absolute;top:-8px;left:8px;right:8px;margin:0;pointer-events:none}
#screen .tile.legend .tile-label,#screen .tile.legend .tile-id{background:#fff;padding:0 5px}
#screen .sparkrows{display:grid;grid-template-columns:auto 1fr auto;gap:3px 8px;align-items:center;width:100%}
#screen .sparkrows .rl{font:600 11.5px var(--mono)}
#screen .sparkrows .rv{font:600 12.5px var(--mono);text-align:right;white-space:nowrap}
#screen .dial .ring{position:absolute;top:0}
#screen .dial .mid b{font-family:var(--cond);font-weight:700;letter-spacing:.02em}
#screen .sig svg{display:block}
#screen .tile.usage .fillcurve{position:absolute;inset:0;background:linear-gradient(180deg,#a8a8a8 0%,#303030 100%)}
#screen .tile.usage .tile-head{position:relative;z-index:1}
#screen .tile.usage .tile-count{position:absolute;left:10px;bottom:9px;border:0;margin:0;padding:0}
#screen .menu .ml .icon{margin-right:9px}
#screen .sig-gap{display:inline-block;width:11px;margin-right:9px}
#screen .sigils{display:inline-flex;gap:6px;align-items:center;vertical-align:middle}
#screen .polar svg,#screen .ridge svg{display:block;width:100%;height:auto;max-height:100%}
#screen .tile.fill .tile-body>.polar{flex:none;width:min(100cqw,100cqh);margin:0 auto}
#screen .word{width:100%;text-align:center}
#screen .word .w{font:700 min(40px,24cqw)/1 var(--cond);letter-spacing:.2em;text-indent:.2em;white-space:nowrap}
#screen .word .cap{font:500 11px/1.35 var(--mono);margin-top:7px;letter-spacing:.02em;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
#ui-frame{position:absolute;inset:0;pointer-events:none;color:#fff;mix-blend-mode:difference;z-index:5}
#ui-frame text{font:500 9.5px var(--mono,monospace);letter-spacing:.06em}
#screen .bignum{font:600 min(56cqh, calc(138cqw / var(--chars,3)))/.9 var(--cond);letter-spacing:-.01em;white-space:nowrap}
#screen .bignum small{font:600 .2em var(--mono);letter-spacing:.04em;margin-left:.15em}
#screen .timerring svg{display:block;width:100%;height:auto;max-height:100%}
#screen .tile.fill .tile-body>.timerlive{flex:none;width:min(100cqw,100cqh);margin:0 auto}
#screen .timerlive .timerring{width:100%}
#screen .tile.fill .tile-body>.feed>.timerlive{flex:none;width:min(100cqw,100cqh);margin:0 auto}
#screen .tile.fill .tile-body>.feed>.dial,#screen .tile.fill .tile-body>.feed>.polar{flex:none;margin:0 auto}
#screen .tile.fill .tile-body>.dial{flex:none;margin:0 auto}
#screen .pane{container-type:size;flex:1 1 0;align-self:stretch;min-width:0;min-height:0;display:flex;align-items:center}
#screen .lines{width:100%;height:100%;display:flex;flex-direction:column}
#screen .lines .ln{flex:1 1 0;min-height:0;max-height:42px;display:flex;align-items:center;gap:7px;border-bottom:1px solid #000;font:500 13.5px/1.2 var(--sans)}
#screen .lines .ln:last-child{border-bottom:0}
#screen .lines .l{font:500 11.5px var(--mono);flex:none;display:inline-flex;align-items:center}
#screen .lines .t{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
#screen .lines .r{font:600 11.5px var(--mono);flex:none}
#screen .tile.list .tile-body>.lines{height:100%}
#screen .typewriter{display:grid;width:100%;font:600 17px/1.3 var(--sans)}
#screen .typewriter>*{grid-area:1/1}
#screen .typewriter .ghost{visibility:hidden}
#screen .cursor{display:inline-block;width:.5em;height:.95em;margin-left:2px;vertical-align:-.1em;background:currentColor;animation:ui-blink 1s steps(1) infinite}
@keyframes ui-blink{50%{opacity:0}}
#screen .spinner svg{animation:ui-turn 1s steps(12) infinite}
@keyframes ui-turn{to{transform:rotate(360deg)}}
#screen .stream{width:100%}
#screen .feed{display:contents}
#screen .tile.fill .tile-body>.feed>*{flex:1;min-width:0}
#screen .tile.fill .tile-body>.timerring{flex:none;width:min(100cqw,100cqh);margin:0 auto}
#screen .tile.list .tile-body ol,#screen .tile.list .tile-body ul{width:100%}
`);
document.head.append(style);

let rev = 0, pending = false, timer: number | undefined;
const settleObs = new MutationObserver(() => {
  rev++; pending = true;
  window.clearTimeout(timer);
  timer = realSetTimeout(() => {
    pending = false;
    window.dispatchEvent(new Event("ui:settled"));
    verify(); // live widgets and API data can change size at any time: keep it fitting
  }, 120) as unknown as number;
});
settleObs.observe(root, { subtree: true, childList: true, attributes: true, characterData: true });

const get = (name: string) => root.querySelector<HTMLElement>(`[data-name="${CSS.escape(name)}"]`);

function mount<T extends Node>(node: T, parent: Element = root): T {
  const name = (node as unknown as HTMLElement).dataset?.name;
  const old = name && get(name);
  old ? old.replaceWith(node) : parent.append(node);
  return node;
}

const describe = (c: Element) => {
  const name = (c as HTMLElement).dataset?.name;
  const cls = c.classList.length ? "." + [...c.classList].join(".") : "";
  const own = [...c.childNodes].filter((n) => n.nodeType === 3).map((n) => n.textContent!.trim()).join(" ").trim();
  return c.tagName.toLowerCase() + (name ? `[${name}]` : "") + cls + (own ? ` "${own.length > 40 ? own.slice(0, 40) + "…" : own}"` : "");
};

// ------------------------------------------------------- guaranteed fit
// The screen cannot scroll, so a scene is never allowed to overflow: it is made to fit.
//   1. scale the whole scene down (never below MIN_SCALE, so text stays readable);
//   2. still too big: hide parts that were explicitly marked optional ({priority: 1..9}, lowest first);
//   3. still too big: clip at the screen edge. Nothing unmarked is ever silently removed.
// Nothing here throws or rolls anything back, and the screen is never blank. The report says
// what was done, so Claude can trim the scene instead of relying on it.

const MIN_SCALE = 0.72;
let dropped: Element[] = [];

type FitReport = {
  /** Always true: the screen is guaranteed to contain the scene. */
  fits: true;
  /** 1 = natural size. Less means the scene was too big and was shrunk. */
  scale: number;
  /** What was hidden because even the smallest readable size did not fit. */
  dropped: string[];
  /** True only if nothing was left to hide and the edge had to clip. */
  clipped: boolean;
  /** Spare vertical pixels after fitting. */
  freeY: number;
  /** How far over the scene was at natural size. Positive numbers mean "trim me". */
  natural: { overflowX: number; overflowY: number };
  /** Modules whose content is bigger than their own cell (it is clipped inside the module). */
  crowded: string[];
};

function adapt(): FitReport {
  flush(); // keep legitimate pending changes; this function's own writes are discarded below
  const cs = getComputedStyle(screen);
  const availW = screen.clientWidth - parseFloat(cs.paddingLeft) - parseFloat(cs.paddingRight);
  const availH = screen.clientHeight - parseFloat(cs.paddingTop) - parseFloat(cs.paddingBottom);
  for (const e of dropped) e.removeAttribute("data-dropped"); // decide from scratch every time
  dropped = [];

  // Reflow at width availW/s, then scale by s: lower s gives text more room per line AND shrinks it.
  const fitsAt = (s: number) => {
    root.style.width = `${availW / s}px`;
    root.style.minHeight = "0";
    root.style.transform = s === 1 ? "" : `scale(${s})`;
    return root.scrollHeight * s <= availH + 0.5 && root.scrollWidth * s <= availW + 0.5;
  };

  fitsAt(1);
  const natural = { overflowY: Math.max(0, Math.round(root.scrollHeight - availH)), overflowX: Math.max(0, Math.round(root.scrollWidth - availW)) };
  let scale = 1, clipped = false;

  if (!fitsAt(1)) {
    // Hideable parts: only what was explicitly marked optional (priority 1..9; 10 means never).
    const marked = [...root.querySelectorAll<HTMLElement>("[data-priority]")]
      .map((el) => [el, Number(el.dataset.priority)] as const)
      .filter(([, p]) => p < 10);
    // least important first; among equals, later in the page first
    marked.sort((a, b) => a[1] - b[1] || (a[0].compareDocumentPosition(b[0]) & Node.DOCUMENT_POSITION_FOLLOWING ? 1 : -1));

    let ok = fitsAt(MIN_SCALE);
    for (const [el] of marked) {
      if (ok) break;
      el.setAttribute("data-dropped", "");
      dropped.push(el);
      ok = fitsAt(MIN_SCALE);
    }
    if (ok) {
      // the largest scale in [MIN_SCALE, 1] that fits (smaller is always roomier, so bisect)
      let lo = MIN_SCALE, hi = 1;
      for (let i = 0; i < 9; i++) {
        const mid = (lo + hi) / 2;
        if (fitsAt(mid)) lo = mid; else hi = mid;
      }
      scale = Math.floor(lo * 1000) / 1000;
    } else {
      scale = MIN_SCALE;
      clipped = true;
    }
    fitsAt(scale);
  }

  const contentH = root.scrollHeight;
  root.style.minHeight = `${availH / scale}px`; // lets fill() regions take the remaining space
  settleObs.takeRecords();
  journal.takeRecords(); // our own style/data-dropped writes are not history
  const crowded = [...root.querySelectorAll<HTMLElement>(".tile:not(.bleed),.panel")]
    .filter((e) => !e.closest("[data-dropped]") && (e.scrollHeight > e.clientHeight + 8 || e.scrollWidth > e.clientWidth + 8)).map(describe); // glyph ink may poke a few px past a tight line
  return { fits: true, scale, dropped: dropped.map(describe), clipped, freeY: Math.round(availH - contentH * scale), natural, crowded };
}

/** The fit report for the scene as it is now (re-fits it). */
const fits = (): FitReport => adapt();

// ------------------------------------------------------------------ timers
// Raw setInterval/setTimeout from the REPL are tracked: timers created by a change that is
// rolled back are cancelled with it, and live intervals can be listed and stopped. Replay
// re-creates them, because the eval that created one is in the journal. Internals use the
// real functions so they are never cancelled.

const realSetTimeout = window.setTimeout.bind(window);
const realSetInterval = window.setInterval.bind(window);
const realClearInterval = window.clearInterval.bind(window);
const realClearTimeout = window.clearTimeout.bind(window);

const intervals = new Map<number, { ms: number; name?: string }>();
// Intervals made between begin() and verify() belong to that change. Only those are cancelled
// on rollback; unrelated timers (background widgets, one-shot timeouts) are never touched.
let txOpen = false;
let uncommitted = new Set<number>();

/** The orchestrator calls this right before evaluating a change (tx() does it for you). */
function begin(): void {
  txOpen = true;
  uncommitted.clear();
}

window.setInterval = ((fn: TimerHandler, ms?: number, ...args: any[]) => {
  const id = realSetInterval(fn, ms, ...args);
  intervals.set(id, { ms: ms ?? 0 });
  if (txOpen) uncommitted.add(id);
  return id;
}) as typeof window.setInterval;
// timeout and interval ids share one namespace, so one clear covers both
window.clearInterval = window.clearTimeout = ((id?: number) => {
  if (id != null) intervals.delete(id);
  realClearInterval(id);
}) as typeof window.clearInterval;

// ------------------------------------------------------------ the beat
// E-ink refreshes are slow and visible, so nothing live updates on its own schedule. One master clock ticks on the
// wall clock; every job and every live binding is applied in the same synchronous batch when its period comes due,
// so one beat = one repaint. Data arriving from the server (or the REPL) only marks bindings stale; the DOM is
// touched at the next beat of the binding's tier, never before.
//   tick  1 s     only for what must count (a timer ring)
//   beat  30 s    the default for live data
//   slow  5 min   weather, prices, anything that drifts
// Periods are multiples of the clock, aligned to it, so a 30 s job and a 5 min job coincide every 5 min.

type Tier = "tick" | "beat" | "slow";
const cadence: Record<Tier, number> = { tick: 1000, beat: 30_000, slow: 300_000 };
const periodOf = (r: number | Tier): number => (typeof r === "number" ? Math.max(cadence.tick, r) : cadence[r]);
const slotOf = (r: number | Tier, now = Date.now()): number => Math.floor(now / periodOf(r));

interface Job { name?: string; every: number | Tier; fn: () => void; slot: number }
const jobs = new Map<number, Job>();
let jobSeq = 0, flushes = 0, lastFlush = 0, held = false;

function stop(which: string | number): void {
  for (const [id, t] of intervals) if (id === which || t.name === which) window.clearInterval(id);
  for (const [id, j] of jobs) if (id === which || j.name === which) jobs.delete(id);
}
/**
 * A named job on the beat: `ui.every("clock", "beat", fn)` or with a period in ms (min 1 s, rounded to the clock). Calling again
 * with the same name replaces it. It first runs at the next boundary of its period, with everything else due then.
 */
function every(name: string, ms: number | Tier, fn: () => void): number {
  stop(name);
  const id = -++jobSeq;
  jobs.set(id, { name, every: ms, fn, slot: slotOf(ms) });
  if (txOpen) uncommitted.add(id);
  return id;
}
const timers = () => [
  ...[...intervals].map(([id, t]) => ({ id, ms: t.ms, name: t.name })),
  ...[...jobs].map(([id, j]) => ({ id, ms: periodOf(j.every), name: j.name })),
];

// ---- live data: a keyed store the server (or any script) writes, and bindings that show it
interface Datum { v: any; ts: number }
const data = new Map<string, Datum>();
interface Watcher { keys: string[]; el: Element; draw: () => void; tier: number | Tier; slot: number; stale: boolean; always: boolean; seen: boolean; empty: boolean }
const watchers = new Set<Watcher>();

/** Write a value. This never touches the DOM: bindings that show the key go stale and redraw at their next beat. */
function setData(key: string, v: any, ts: number = Date.now()): void {
  if (key === "ui.screen" && typeof v === "string" && v !== currentScreen) { data.set(key, { v, ts }); go(v); return; }   // navigation is immediate
  const old = data.get(key);
  const changed = !old || JSON.stringify(old.v) !== JSON.stringify(v);
  data.set(key, { v, ts });
  if (changed) for (const w of watchers) if (w.keys.includes(key)) w.stale = true;
}
/** Minutes since a key was last written (Infinity if never): feed a sigil("age", ...) with it. */
const dataAge = (key: string): number => (data.has(key) ? Math.floor((Date.now() - data.get(key)!.ts) / 60000) : Infinity);

/**
 * A live binding: `feed("wifi.bars", (n) => sigil("bars", n))` draws now and redraws only at its tier's next beat after the
 * value changes. With several keys the render gets an array. `always` redraws at every beat (for things that show their age).
 * Until any of its keys has been written it shows `fallback` (default "–"); missing keys of a multi-key feed arrive as undefined.
 */
const feed = (keys: string | string[], render: (v: any, meta: { age: number }) => Child, o: { tier?: number | Tier; fallback?: Child; always?: boolean } = {}): HTMLElement => {
  const list = Array.isArray(keys) ? keys : [keys], tier = o.tier ?? "beat";
  const el = h("span", { class: "feed" });
  const draw = () => {
    const have = list.some((k) => data.has(k));
    const age = Math.max(...list.map(dataAge).filter((a) => a !== Infinity), 0);
    const vals = list.map((k) => data.get(k)?.v);
    el.replaceChildren(frag(have ? render(Array.isArray(keys) ? vals : vals[0], { age }) : o.fallback ?? "–"));
  };
  const empty = !list.some((k) => data.has(k));
  draw();
  watchers.add({ keys: list, el, draw, tier, slot: slotOf(tier), stale: false, always: !!o.always, seen: false, empty });
  return el;
};

function applyFeeds(now: number): number {
  let n = 0;
  for (const w of [...watchers]) {
    if (!w.el.isConnected) { if (w.seen) watchers.delete(w); continue; }
    w.seen = true;
    const slot = slotOf(w.tier, now);
    if (w.empty && w.keys.some((k) => data.has(k))) { w.empty = false; w.slot = slot; w.stale = false; w.draw(); n++; continue; } // first data: don't wait out a whole period
    if ((w.stale || w.always) && slot > w.slot) { w.slot = slot; w.stale = false; w.draw(); n++; }
  }
  return n;
}

/** The master clock's step: everything due this second runs in this one synchronous batch. */
function beatStep(): void {
  if (held) return;
  const now = Date.now();
  flush(); // whatever is already journaled belongs to a change in flight
  let ran = 0;
  for (const [id, j] of [...jobs]) {
    const slot = slotOf(j.every, now);
    if (slot <= j.slot) continue;
    j.slot = slot;
    ran++;
    try { j.fn(); } catch (e) { console.error("ui: job", j.name ?? id, "failed:", e); }
  }
  ran += applyFeeds(now);
  journal.takeRecords(); // live updates are not part of any change: a rollback must never undo them
  if (ran) { flushes++; lastFlush = now; window.dispatchEvent(new CustomEvent("ui:beat", { detail: { ran, at: now } })); }
}
const beatLoop = (): void => { beatStep(); realSetTimeout(beatLoop, cadence.tick - (Date.now() % cadence.tick) + 4); };
realSetTimeout(beatLoop, cadence.tick - (Date.now() % cadence.tick) + 4);

// The clock is data too: "time.minute" is rewritten once a minute, on the minute, so a clock face binds to it like any feed
// (`feed(["time.minute", ...], draw, {tier: 60000})` redraws exactly when the minute changes).
let lastMinute = -1;
every("ui.time", "tick", () => { const m = Math.floor(Date.now() / 60000); if (m !== lastMinute) { lastMinute = m; setData("time.minute", Date.now()); } });

/** Change the periods (ms), e.g. ui.cadence({beat: 60000}). Jobs re-base so nothing fires in a burst. */
function setCadence(c: Partial<Record<Tier, number>> = {}): Record<Tier, number> {
  for (const k of ["tick", "beat", "slow"] as Tier[]) if (c[k]) cadence[k] = Math.max(1000, Math.round(c[k]! / 1000) * 1000);
  for (const j of jobs.values()) j.slot = slotOf(j.every);
  for (const w of watchers) w.slot = slotOf(w.tier);
  return { ...cadence };
}

// ---- the link to the machine (see crates/preview: /sub, /push, /run, feeds/)
let link: "connecting" | "up" | "down" = "connecting";
/** Follow the server's data feed: long-polls /sub, writing every update into the store (and into "link.server"). */
async function connect(base = ""): Promise<void> {
  let since = 0;
  for (;;) {
    try {
      const j = await (await fetch(`${base}/sub?since=${since}`, { cache: "no-store" })).json();
      since = j.seq;
      for (const [k, v, ts] of j.updates) setData(k, v, ts);
      if (link !== "up") { link = "up"; setData("link.server", "up"); }
    } catch {
      if (link !== "down") { link = "down"; setData("link.server", "down"); }
      await new Promise((r) => realSetTimeout(r, 3000));
    }
  }
}
/** Publish a value to the server's store (and so to every page and script). */
const push = (key: string, value: any) => fetch("/push", { method: "POST", body: JSON.stringify({ key, value }) }).then((r) => r.json());
/** Run a named command from the server's commands/ directory (never arbitrary shell): resolves {ok, code, stdout, stderr}. */
const run = (cmd: string, ...args: string[]) =>
  fetch(`/run?cmd=${encodeURIComponent(cmd)}${args.map((a) => `&arg=${encodeURIComponent(a)}`).join("")}`, { method: "POST" }).then((r) => r.json());
if (!(window as any).UI_NO_LINK && location.protocol.startsWith("http")) void connect();

// ------------------------------------------------- transactional display
// Every change under the root is journaled, so a change that throws can be undone atomically
// (nothing half-applied) and the persistence layer knows where each change starts and ends.
// Size is never a reason to undo anything: adapt() makes every scene fit.

let log: MutationRecord[] = [];
const journal = new MutationObserver((r) => log.push(...r));
journal.observe(root, { subtree: true, childList: true, attributes: true, attributeOldValue: true, characterData: true, characterDataOldValue: true });
const flush = () => { log.push(...journal.takeRecords()); };

function undo(r: MutationRecord): void {
  const t = r.target;
  if (r.type === "childList") {
    r.addedNodes.forEach((n) => { if (n.parentNode === t) t.removeChild(n); });
    const before = r.nextSibling && r.nextSibling.parentNode === t ? r.nextSibling : null;
    r.removedNodes.forEach((n) => t.insertBefore(n, before));
  } else if (r.type === "attributes") {
    const el = t as Element;
    if (r.oldValue === null) el.removeAttribute(r.attributeName!);
    else el.setAttribute(r.attributeName!, r.oldValue);
  } else {
    (t as CharacterData).data = r.oldValue ?? "";
  }
}

function rollback(): void {
  flush();
  for (const r of log.splice(0).reverse()) undo(r);
  journal.takeRecords(); // the undo's own mutations are not history
  for (const id of uncommitted) { realClearInterval(id); intervals.delete(id); jobs.delete(id); } // timers and jobs die with the change that made them
  uncommitted.clear();
  txOpen = false;
  adapt();
}

/** Commit the changes since the last call as the new known-good state, and make the scene fit. */
function verify(): FitReport {
  flush();
  const f = adapt();
  log = [];
  uncommitted.clear();
  txOpen = false;
  return f;
}

/** Run fn, then verify(). If fn throws, its changes are rolled back too. */
function tx<T>(fn: () => T): T {
  let v: T;
  begin();
  try { v = fn(); } catch (e) { rollback(); throw e; }
  verify();
  return v;
}

function outline(el: Element = root, depth = 0): string {
  const lines: string[] = [];
  for (const c of el.children) {
    const name = (c as HTMLElement).dataset?.name;
    const cls = c.classList.length ? "." + [...c.classList].join(".") : "";
    const own = [...c.childNodes].filter((n) => n.nodeType === 3).map((n) => n.textContent!.trim()).join(" ").trim();
    lines.push("  ".repeat(depth) + c.tagName.toLowerCase() + (name ? `[${name}]` : "") + cls + (own ? ` "${own.length > 40 ? own.slice(0, 40) + "…" : own}"` : ""));
    if (c.tagName.toLowerCase() !== "svg") lines.push(outline(c, depth + 1));
  }
  return lines.filter(Boolean).join("\n");
}

// ------------------------------------------------------------ 1-bit preview
// The page is rasterised, error-diffused to pure black/white, and overlaid on the
// screen. The panel pipeline should run the same algorithm over real screenshots.

type DitherAlgo = "atkinson" | "floyd" | "threshold";

const KERNELS: Record<"atkinson" | "floyd", [number, number, number][]> = {
  atkinson: [[1, 0, 1 / 8], [2, 0, 1 / 8], [-1, 1, 1 / 8], [0, 1, 1 / 8], [1, 1, 1 / 8], [0, 2, 1 / 8]],
  floyd: [[1, 0, 7 / 16], [-1, 1, 3 / 16], [0, 1, 5 / 16], [1, 1, 1 / 16]],
};

/** Quantise to pure black/white, in place. */
function dither(img: ImageData, algo: DitherAlgo = "atkinson"): ImageData {
  const { width: w, height: ht, data } = img;
  const lum = new Float32Array(w * ht);
  for (let i = 0; i < lum.length; i++) {
    const a = data[i * 4 + 3] / 255; // composite over white
    lum[i] = (0.2126 * data[i * 4] + 0.7152 * data[i * 4 + 1] + 0.0722 * data[i * 4 + 2]) * a + 255 * (1 - a);
  }
  const kernel = algo === "threshold" ? [] : KERNELS[algo];
  for (let y = 0; y < ht; y++)
    for (let x = 0; x < w; x++) {
      const i = y * w + x, old = lum[i], v = old < 128 ? 0 : 255, err = old - v;
      lum[i] = v;
      for (const [dx, dy, f] of kernel) {
        const nx = x + dx, ny = y + dy;
        if (nx >= 0 && nx < w && ny < ht) lum[ny * w + nx] += err * f;
      }
    }
  for (let i = 0; i < lum.length; i++) {
    data[i * 4] = data[i * 4 + 1] = data[i * 4 + 2] = lum[i];
    data[i * 4 + 3] = 255;
  }
  return img;
}

/** Rasterise an element via an SVG foreignObject. */
// An SVG snapshot can't load external fonts, so embed them as data URIs (once).
let inlineFaces: Promise<string> | null = null;
const faceCss = () =>
  (inlineFaces ??= Promise.all(
    FONT_FILES.map(async ([family, file, weight, stretch]) => {
      const res = await fetch(`${FONT_BASE}/${file}`);
      if (!res.ok) return ""; // an optional face that is not installed
      const bytes = new Uint8Array(await res.arrayBuffer());
      let bin = "";
      for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
      return `@font-face{font-family:"${family}";src:url(data:${fontFormat(file)[1]};base64,${btoa(bin)}) format("${fontFormat(file)[0]}");font-weight:${weight};font-stretch:${stretch}}`;
    }),
  ).then((faces) => faces.join("")));

async function rasterize(el: HTMLElement): Promise<ImageData> {
  const w = el.offsetWidth, ht = el.offsetHeight;
  const css = (style.textContent ?? "").replace(/@font-face\{[^}]*\}/g, "") + (await faceCss());
  // a 1-bit panel has no colour: the accent is black there
  const clone = el.cloneNode(true) as HTMLElement;
  clone.style.setProperty("--accent", "#000");
  clone.style.removeProperty("--accent-fill");
  const wrap = h("div", { xmlns: "http://www.w3.org/1999/xhtml" }, h("style", css), clone);
  const xml = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${ht}"><foreignObject width="100%" height="100%">${new XMLSerializer().serializeToString(wrap)}</foreignObject></svg>`;
  const img = new Image();
  img.src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(xml);
  await img.decode();
  const c = document.createElement("canvas");
  c.width = w; c.height = ht;
  const ctx = c.getContext("2d", { willReadFrequently: true })!;
  ctx.fillStyle = "#fff";
  ctx.fillRect(0, 0, w, ht);
  ctx.drawImage(img, 0, 0);
  return ctx.getImageData(0, 0, w, ht);
}

let overlay: HTMLCanvasElement | null = null, einkAlgo: DitherAlgo = "atkinson", paintToken = 0;

async function paintEink(): Promise<void> {
  if (!overlay) return;
  const mine = ++paintToken;
  try {
    const data = dither(await rasterize(screen), einkAlgo);
    if (!overlay || mine !== paintToken) return;
    overlay.width = data.width;
    overlay.height = data.height;
    overlay.getContext("2d")!.putImageData(data, 0, 0);
    const r = screen.getBoundingClientRect();
    Object.assign(overlay.style, { left: `${r.left + scrollX}px`, top: `${r.top + scrollY}px`, width: `${r.width}px`, height: `${r.height}px` });
  } catch (e) {
    console.error("ui.eink: render failed", e);
  }
}
window.addEventListener("ui:settled", () => void paintEink());
window.addEventListener("resize", () => void paintEink());

/** ui.eink() atkinson | ui.eink({algo:"floyd"|"threshold"}) | ui.eink(false) off */
function eink(opt: boolean | { algo?: DitherAlgo } = true): void {
  if (opt === false) {
    overlay?.remove();
    overlay = null;
    paintToken++;
    return;
  }
  einkAlgo = (typeof opt === "object" && opt.algo) || "atkinson";
  if (!overlay) {
    overlay = h("canvas", { style: "position:absolute;pointer-events:none;image-rendering:pixelated;z-index:10" }) as HTMLCanvasElement;
    document.body.append(overlay);
  }
  void paintEink();
}

const HELP = `ui — display-only kit. Everything is a real DOM node; fall back to the DOM any time.
FIT IS AUTOMATIC  The screen is 800x480 with no scrolling, and a scene cannot overflow it: it is scaled down to fit (never below
         0.72x, so text stays readable), then parts you marked optional are hidden, then the edge clips. Nothing errors and the
         screen is never blank. ui.show()/ui.fits() return {scale, dropped, clipped, freeY, natural}. If scale < 1 or anything was
         dropped, the scene was too big: trim it. Mark optional parts {priority: 1..9} (low = hidden first); unmarked content is never removed, only clipped.
         Wrap dynamic/API text in clamp(lines, text); use fill(...) for a region that takes the remaining space.
BUILD    div p section h1..h4 ul li table span em strong small code mark bold italic ... (bare globals)
         props via object arg: p({class:"x", style:{color:"#555"}, name:"intro"}, "text")
LAYOUT   row(...) col(...) grid(n,...) box(...) card("Title",...) big(...) divider() badge(x) heading(x,level)
DATA     stat(label,value,{unit,delta}) list([..]) table(rows,{head}) kv({k:v}) progress(0..1,label)
         bars([n | [label,n]]) sparkline([n]) agenda([{time,title,note}]) weather({temp,cond,icon,hi,lo}) md("# text")
MORE     topbar(title,right) banner(text,{icon}) quote(text,by) todo([str|{text,done}]) gauge(0..1,label) month(date,{marks})
         lineChart([n],{labels}) image(src,{fit,w,h}) icon(name) chips([..]) trend(n) spacer() center(...)
FRIENDLY say(text) sunArc({rise,set,now}) hourly([{t,temp,icon,pop}]) tearoff(date) note(text,{tilt}) presence([{name,home}])
         habitGrid([0..3]) timerRing(sec,total) moon(phase) weatherIcon(kind,size) kinds: clear night partly cloudy rain snow storm fog wind
MODULES  tile(label,body,{count,inverted}) levelTile(label,0..1,text) dayDial({rise,set,now,size,sub}) ruler({min,max,value,target,step,w})
         statStrip([{label,value,sub}]) pills([str|{label,icon}],{active}) slider(0..1,label,{text}) hero(value,{unit,label,status,sub})
         menu([{label,value,dot:ok|warn|off}]) device({name,sub,on,icon}). ui.accent("#d4451a") for colour panels; black otherwise.
SIGILS   sigil(kind,value,size) / sigils([[kind,value]..]) are 12px marks carrying ONE fact: state ok|idle|warn|err|unk, level [v,max] (filled ring),
         trend up|down|flat|rise|fall, age minutes (one-hand clock), count 1..6 (dice), dir degrees, bars 0..4, glyph iconName. Use them inline
         (in a label, a table cell, next to a number) and in the corners: ui.frame({title,sheet, tl:[...], tr:[...]}). Terse beats verbose.
VECTOR   polar([n],{labels,marks}) for cyclic data;
         ridgeline(rows,{labels,hours}) a surface of days; word("FLX","caption") a spaced-caps status screen. A console = square modules.
LOOK     Landscape-like modules: small centred line icons (tone "iconic", count bottom-left), the donut dayDial clock, wavy dithered gradients
         (tile {waves:{from,bands,amp}}), levelTile tanks. Clean sans for text, mono only for data. tone "legend" puts the label on the border.
         btop-like detail: dotBars, dotMirror(up,down), sparkRows([{label,values,text}]), tables with segbar cells.
STYLE    Engineering-graphics: hard corners, monospace, 1px lines, DENSE and EXPLICIT. Show state: units, source, updated-at, ranges, status.
         Flat, not hierarchical. statusStrip([mark, ...],[right...]) on top; ui.frame({title,sheet}) adds zone letters/numbers + title block.
         readouts([{label,value,max,unit,text}]) = LABEL [segmented bar] value. Each tile gets a cell id (B2) from place().
LAYOUT   EVERY SCREEN IS A tilegrid (6 cols x 3 rows) OF MODULES: place(tile(label, visual, {count, inverted, tone}), "row / col / rowEnd / colEnd").
         tone: "fill" (visual fills the module) | "sky" (dithered gradient) | "list" (compact rows). Label top-left, visual, count bottom-left.
         Vary tones: some inverted, one sky or tank, the rest outlined. Pass w: to dayStrip/areaChart/timeline so their text stays 1:1.
         ui.fits() also lists crowded modules (content bigger than its cell).
BREAK    band("top"|"bottom"|"mid", ...) is a full-bleed black bar (header, Claude line, statStrip, ruler inside). giant(value,{size}) is an
         oversized numeral: put it in tile(..., {bleed:true, inverted:true}) so the module edge crops it. tone "open" = no box, just a top rule.
         ui.frame() adds corner registration targets and edge ticks. Alternate boxed, open, inverted and banded regions; avoid all-boxes.
QUANTITY dotBars([0..1]) waffle(value,total,{cols,hatch}) stackBar([{label,value,look}]) arcGauge(v,{min,max}) levelTile dayDial ruler
BOLD     panel("inv"|"sky"|"plain"|"dots",...) meta(left,right) claudeLine(text,time) dotClock({pitch,time}) dayStrip({rise,set,now})
         timeline({from,to,now,lanes:[{label,blocks:[{from,to,label,look:solid|hatch|outline}]}]}) areaChart([n],{h,labels,now,window})
         tilegrid(...) with tiles placed by style gridArea; hero(value,{size}); class "reg" adds corner registration marks.
ICONS    icon(name,{size,fill,stroke}): arrow chevron sun moon cloud rain snow bolt wind drop thermo umbrella home door check cross warn info mail
         star heart phone flag clock bell calendar trash bulb plug wifi battery music book cup utensils car bus bag leaf sparkle
         lock tv pin user sunrise sunset  (any other text renders as a glyph)
         DESIGN FOR 1-BIT: no light grays. Hierarchy = size, weight, caps, borders, inverted blocks (banner/mark).
TIMERS   ui.every(name, ms, fn)  named interval, re-running replaces it (prefer this to raw setInterval; min 1s)
         ui.timers()  ui.stop(nameOrId). Intervals made by a rejected change are cancelled with it.
LIVE     clock({seconds,date,hour12}) countdown(date,label) live(el, fn, ms)
SCENE    ui.show(...nodes)  replace everything | ui.mount(node)  append (replaces node with same name)
         ui.get("name")  ui.outline()  ui.clear()  await ui.settled()  ui.rev
         ui.eink() | ui.eink({algo:"atkinson"|"floyd"|"threshold"}) | ui.eink(false)   dithered 1-bit preview
EDIT     el.child el.kids el.up el.next el.prev el.q(sel) el.qa(sel) el.text = x el.add(...) el.set({..}) el.clear()
LIVE     feed("wifi.bars", n => sigil("bars", n))  a binding: redraws only on its tier'"'"'s next beat after the value changes.
         ui.every(name, "tick"|"beat"|"slow"|ms, fn)  a job on the master clock; ui.cadence({beat: 60000}); ui.hold()/resume().
         ui.set(k, v) ui.get(k) ui.age(k)  the data store (scripts write it via the server); ui.push(k, v); ui.run("cmd", ...args) runs commands/<cmd> on the machine.
MOTION   liveTimer(sec) typewriter(text) countUp(n,{dec,unit}) stream(series,next,draw) spinner() cursor(): stepped, 1 s ticks (ui.every); CSS steps() under that.
LENSES   el.words | el.chars | el.sentences over flattened text:
         el.words[1] = bold("x")   el.words.wrap(1, mark)   el.words.replace(0,2,"hi")
         el.words.insert(i,x)  el.words.remove(i,j)  el.words.length  [...el.words]  el.words.indexOf("w")`;

const ui = {
  root, screen, h, rev: () => rev, mount, get, outline, eink, dither, rasterize,
  /** The scene as HTML: what the store keeps as its snapshot cache. */
  snapshot: () => root.innerHTML.replace(/ data-dropped=""/g, ""), // the fit marker is derived state, not content
  /** Replace the whole scene. Returns the fit report: a scene that doesn't fit must not be shown. */
  show: (...nodes: Child[]) => { root.replaceChildren(frag(nodes)); return fits(); },
  clear: () => { root.replaceChildren(); return root; },
  settled: () => (pending ? new Promise<void>((r) => window.addEventListener("ui:settled", () => r(), { once: true })) : Promise.resolve()),
  /** Resolves when the fonts are loaded. Await it before the first scene: layout depends on them. */
  ready: () => fontsReady,
  /** One accent colour for colour e-ink panels (e.g. red/yellow). Without it everything stays black. */
  accent: (color: string | null) => {
    if (color) { root.style.setProperty("--accent", color); root.style.setProperty("--accent-fill", color); }
    else { root.style.removeProperty("--accent"); root.style.removeProperty("--accent-fill"); }
  },
  /** Names of the built-in icons. */
  icons: () => Object.keys(ICON_PATHS),
  fits, frame, begin, verify, rollback, tx, every, stop, timers,
  /** The master clock: periods, held state, what is scheduled, how many batches ran. */
  beats: () => ({ cadence: { ...cadence }, held, flushes, lastFlush, jobs: [...jobs.values()].map((j) => ({ name: j.name, every: periodOf(j.every) })), bindings: watchers.size, link }),
  cadence: setCadence,
  /** Freeze every beat (night mode): data keeps arriving, nothing redraws until resume(). */
  hold: () => { held = true; }, resume: () => { held = false; },
  /** The data store: ui.set(key, v) (no redraw until the next beat), ui.get(key), ui.age(key) minutes, ui.data() everything. */
  set: setData, get: (k: string) => data.get(k)?.v, age: dataAge, data: () => Object.fromEntries([...data].map(([k, d]) => [k, d.v])),
  connect, push, run, link: () => link,
  /** Screens and the tab bar: defineScreen(name, build, {tab}) go(name) start() screens() current() */
  defineScreen, go, start, screens: () => [...screenDefs.keys()], current: () => currentScreen,
  size: (w: number, ht: number) => { screen.style.width = w + "px"; screen.style.height = ht + "px"; adapt(); },
  css: (text: string) => document.head.append(h("style", text)),
  help: () => HELP,
};

const exported: Record<string, unknown> = {
  ...prims, row, col, grid, box, card, big, heading, divider, badge, stat, list, table, kv, progress,
  bars, sparkline, agenda, weather, md, clock, countdown, live,
  feed, lines, checklist, pane, cursor, spinner, liveTimer, typewriter, countUp, stream,
  icon, weatherIcon, say, sunArc, hourly, habitGrid, timerRing, tearoff, note, presence, moon,
  tile, levelTile, dayDial, ruler, statStrip, pills, slider, menu, device, hero,
  panel, meta, claudeLine, tilegrid, dotClock, dayStrip, timeline, areaChart,
  place, limitBars, dotBars, waffle, stackBar, arcGauge, band, giant,
  statusStrip, segbar, readouts, kvgrid, skybox, header, ruled, toneStrip, routes, waves, dotMirror, sparkRows, bignum, polar, ridgeline, word, sigil, sigils, usageTile,
  clamp, fill, spacer, center, chips, topbar, banner, quote, trend, todo, gauge, month, lineChart, image,
};
const skipped: string[] = [];
for (const [k, v] of Object.entries(exported)) {
  if (k in window) skipped.push(k);
  else (window as any)[k] = v;
}
(window as any).ui = ui;
if (skipped.length) console.warn("ui: globals not installed (name taken):", skipped.join(", "));

export {};
