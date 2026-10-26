// The real screens of the display. Each is a named function that builds the whole scene; their tabs are the tab bar along the bottom
// (ui.frame({tabs: true})). The first one defined is home. Every live value is a `feed(...)` bound to a key a script writes
// (feeds/*.py through the server), redrawn on the beat. To add a screen: ui.defineScreen("name", () => sheet(...), {tab: "NAME"}).
// Example/sample layouts live in crates/preview/static/index.html, the gallery.

(() => {
  const ui = (window as any).ui, w = window as any;
  const { feed, tile, place, tilegrid, fill, div, small, lines, checklist, pane, bignum, readouts, dayDial, weatherIcon, areaChart, limitBars, dotBars, dotMirror, sparkRows, stackBar, kvgrid, usageTile, sigil, moon, liveTimer, routes, waffle, center, clamp, table, segbar } = w;

  const sheet = (...modules: any[]) => ui.show(fill(tilegrid(...modules)));
  const MIN = { tier: 60000 };                                   // minute-aligned: clocks, "next in", past or future
  const num = (v: any, d = 0) => (v == null || isNaN(v) ? "–" : Number(v).toFixed(d));
  const dur = (m: number) => (m < 60 ? `${m}m` : `${Math.floor(m / 60)}h${String(m % 60).padStart(2, "0")}`);
  const upfmt = (s: number | null) => (s == null ? "–" : s >= 86400 ? `${Math.floor(s / 86400)}d ${Math.floor((s % 86400) / 3600)}h` : `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`);
  const last = (a: number[] | null, n: number) => { const t = (a ?? []).slice(-n); return Array(Math.max(0, n - t.length)).fill(0).concat(t); };
  const nowHM = () => new Date().toTimeString().slice(0, 5);
  const sqrtScale = (a: number[], top: number) => a.map((x) => Math.sqrt(Math.min(1, x / top)));
  const pctCell = (v: number) => div({ style: "display:flex;align-items:center;gap:7px;min-width:92px" }, div({ style: "flex:1" }, segbar(Math.min(100, v), 100)), v.toFixed(1));

  // ---- HOME
  ui.defineScreen("home", () => {
    sheet(
      place(center(feed(["time.minute", "sun.rise", "sun.set"], ([, r, s]) => dayDial({ rise: r ?? "07:00", set: s ?? "17:00", size: 244 }), MIN)), "1 / 1 / 3 / 3"),
      place(tile("Wx", feed("wx.kind", (k) => weatherIcon(k, 46)), { glyph: "cloud", tone: "iconic", waves: { from: 34 },
        count: feed(["wx.temp", "wx.text", "wx.rain_from"], ([t, x, r]) => [div(`${num(t)}°C ${(x ?? "").toLowerCase()}`), div(r ? `rain ${r}` : "dry 24 h")]) }), "1 / 3 / 3 / 4"),
      place(tile("Calendar", feed(["time.minute", "cal.week"], ([, wk]) => {
        const today = new Date().toLocaleDateString("sv"), up = (wk ?? []).filter((e) => !(e.date === today && e.past)).slice(0, 3);
        const day = (e) => (e.date === today ? "" : new Date(e.date + "T12:00").toLocaleDateString("en-GB", { weekday: "short" }) + " ");
        return up.length ? lines(up.map((e) => [day(e) + (e.all_day ? "all day" : e.t), e.title, e.dur ? dur(e.dur) : ""])) : div({ style: "font:600 15px var(--sans)" }, "Nothing this week");
      }, MIN), { glyph: "calendar", tone: "list", count: feed(["time.minute", "cal.next"], ([, n]) => (n ? `next in ${dur(Math.max(0, n.in_min))}` : "clear for a week"), MIN) }), "1 / 4 / 2 / 6"),
      place(tile("Shop", feed("shop.items", (it) => (it.length ? checklist(it.slice(0, 6).map((i) => [i.text, i.qty, i.done])) : div("Nothing needed"))), { glyph: "bag", tone: "list", count: feed("shop.open", (n) => `${n} to get`) }), "1 / 6 / 3 / 7"),
      place(tile("Grid", feed("carbon.now", (g) => bignum(num(g), { unit: "g" })), { glyph: "leaf", count: feed(["carbon.index", "carbon.best"], ([i, b]) => [sigil("state", i === "very low" || i === "low" ? "ok" : i === "moderate" ? "warn" : "err", 10), ` ${b ? "best " + b[0] : i ?? ""}`]) }), "2 / 4 / 3 / 5"),
      place(tile("Sys", feed(["cpu.total", "mem.pct", "disk.pct"], ([c, m, d]) => lines([["cpu", "", num(c) + "%"], ["ram", "", num(m) + "%"], ["disk", "", num(d) + "%"]])), { glyph: "cpu", tone: "list", count: feed("sys.uptime", (s) => "up " + upfmt(s)) }), "2 / 5 / 3 / 6"),
      place(tile("Next 24 h", feed("wx.hourly", (t) => areaChart(t, { w: 330, h: 64, labels: [["now", 0], ["+6h", 6], ["+12h", 12], ["+18h", 18]] }), MIN), { glyph: "thermo", tone: "fill",
        count: feed("wx.hourly", (t) => `min ${num(Math.min(...t), 1)} · max ${num(Math.max(...t), 1)} °C`) }), "3 / 1 / 4 / 3"),
      place(tile("Link", feed(["wifi.bars", "net.down", "net.up"], ([b, d, u]) => div({ style: "display:flex;flex-direction:column;gap:6px;align-items:flex-start" }, sigil("bars", b ?? 0, 26), div({ style: "font:600 13px var(--mono)" }, `↓ ${num(d, 1)}`), div({ style: "font:600 13px var(--mono)" }, `↑ ${num(u, 1)}`))), { glyph: "wifi", count: feed("wifi.rssi", (r) => (r == null ? "MB/s" : `${r} dBm`)) }), "3 / 3 / 4 / 4"),
      place(tile("Claude", feed("claude.say", (s) => div({ style: "font:600 17px/1.35 var(--sans)" }, s), { fallback: div({ style: "font:500 15px var(--sans)" }, "Nothing to say yet.") }), { glyph: "sparkle", count: feed("claude.at", (t) => `said ${t}`, { fallback: "waiting" }) }), "3 / 4 / 4 / 7"),
    );
  }, { tab: "HOME" });

  // ---- DAY: Google Calendar (feeds/gcal.py)
  ui.defineScreen("day", () => {
    sheet(
      place(tile("Today", feed(["time.minute", "cal.today"], ([, ev]) => {
        const m = (hm) => +hm.slice(0, 2) * 60 + +hm.slice(3);
        const tt = (x) => `${String(Math.floor(x / 60)).padStart(2, "0")}:${String(x % 60).padStart(2, "0")}`;
        const stops = (ev ?? []).filter((e) => !e.all_day).map((e) => ({ at: e.t, to: tt(Math.min(1439, m(e.t) + Math.max(15, e.dur || 0))), name: e.title }));
        const timed = (ev ?? []).filter((e) => !e.all_day), from = Math.min(7 * 60, ...timed.map((e) => m(e.t))), to = Math.max(21 * 60, ...timed.map((e) => m(e.t) + e.dur + 30));
        return routes({ from: tt(Math.floor(from / 60) * 60), to: tt(Math.min(1439, Math.ceil(to / 60) * 60)), now: nowHM(), w: 722, lanes: [{ label: "Today", look: "solid", stops }] });
      }, MIN), { glyph: "calendar", count: feed("cal.today", (ev) => `${(ev ?? []).length} events today · Google Calendar`), tone: "fill" }), "1 / 1 / 3 / 7"),
      place(tile("Busy", feed("cal.today", (ev) => { const h = (ev ?? []).reduce((a, e) => a + (e.dur || 0), 0) / 60; return div({ style: "width:100%" }, waffle(h, 14, { cols: 14 }), div({ style: "height:7px" }), readouts([{ label: "Busy", value: h, max: 14, text: num(h, 1), unit: "h" }])); }), { glyph: "clock", count: "of 14 h" }), "3 / 1 / 4 / 3"),
      place(tile("Next", feed(["time.minute", "cal.week"], ([, wk]) => {
        const today = new Date().toLocaleDateString("sv"), up = (wk ?? []).filter((e) => !(e.date === today && e.past)).slice(0, 3);
        const day = (e) => (e.date === today ? "" : new Date(e.date + "T12:00").toLocaleDateString("en-GB", { weekday: "short" }) + " ");
        return up.length ? lines(up.map((e) => [day(e) + (e.all_day ? "day" : e.t), e.title, e.dur ? dur(e.dur) : ""])) : div("Nothing this week");
      }, MIN), { glyph: "flag", tone: "list", count: "next 7 days · Google Calendar" }), "3 / 3 / 4 / 5"),
      place(tile("Sun & moon", feed(["sun.rise", "sun.set", "moon.phase", "moon.name"], ([r, s, ph, nm]) => div({ style: "display:flex;gap:12px;align-items:center;width:100%" }, moon(ph ?? 0, 46), kvgrid({ rise: r ?? "–", set: s ?? "–", moon: nm ?? "–" }, 1))), { glyph: "sun", count: feed(["sun.rise", "sun.set"], ([r, s]) => { if (!r) return ""; const m = (x) => +x.slice(0, 2) * 60 + +x.slice(3); return `day ${dur(m(s) - m(r))}`; }) }), "3 / 5 / 4 / 7"),
    );
  }, { tab: "DAY" });

  // ---- WEATHER: Open-Meteo (feeds/weather.py)
  ui.defineScreen("weather", () => {
    const compass = (d) => ["N", "NE", "E", "SE", "S", "SW", "W", "NW"][Math.round((d ?? 0) / 45) % 8];
    sheet(
      place(tile("Now", feed(["wx.kind", "wx.temp"], ([k, t]) => div({ style: "display:flex;flex-direction:column;align-items:center;width:100%;height:100%;gap:4px" }, weatherIcon(k ?? "cloudy", 64), pane(bignum(num(t, 0) + "°")))),
        { glyph: "cloud", count: feed(["wx.text", "wx.feels"], ([x, f]) => `${x ?? "–"} · feels ${num(f, 0)}°`) }), "1 / 1 / 3 / 3"),
      place(tile("Next 24 h · °C", feed("wx.hourly", (t) => areaChart(t, { w: 468, h: 72, marks: true, fmt: (i, v) => `+${i}h ${v}°`, labels: [["now", 0], ["+6h", 6], ["+12h", 12], ["+18h", 18]] }), MIN), { glyph: "thermo", tone: "fill",
        count: feed("wx.hourly", (t) => `min ${num(Math.min(...t), 1)} · max ${num(Math.max(...t), 1)}`) }), "1 / 3 / 2 / 7"),
      place(tile("Rain chance · next 24 h", feed("wx.pop", (p) => dotBars((p ?? []).map((x) => (x ?? 0) / 100), { rows: 3 })), { glyph: "drop", tone: "fill",
        count: feed(["wx.rain_from", "wx.pop"], ([r, p]) => (r ? `rain from ${r}` : "no rain expected") + ` · peak ${Math.max(0, ...(p ?? []))}%`) }), "2 / 3 / 3 / 7"),
      place(tile("Wind", feed(["wx.wind", "wx.wind_dir"], ([w, d]) => div({ style: "display:flex;gap:10px;align-items:center;width:100%;height:100%" }, sigil("dir", d ?? 0, 34), pane(bignum(num(w), { unit: "mph" })))), { glyph: "wind", count: feed("wx.wind_dir", (d) => `from the ${compass(d)}`) }), "3 / 1 / 4 / 3"),
      place(tile("Pressure", feed("wx.pressure", (p) => bignum(num(p), { unit: "hPa" })), { glyph: "info", count: feed("wx.cloud", (c) => [sigil("level", [c ?? 0, 8], 11), ` cloud ${c ?? "–"}/8`]) }), "3 / 3 / 4 / 5"),
      place(tile("Sun & moon", feed(["sun.rise", "sun.set", "moon.phase", "moon.name"], ([r, s, ph, nm]) => div({ style: "display:flex;gap:12px;align-items:center;width:100%" }, moon(ph ?? 0, 46), kvgrid({ rise: r ?? "–", set: s ?? "–", moon: nm ?? "–" }, 1))), { glyph: "sun",
        count: feed(["sun.rise", "sun.set"], ([r, s]) => { if (!r) return ""; const m = (x) => +x.slice(0, 2) * 60 + +x.slice(3); return `day ${dur(m(s) - m(r))}`; }) }), "3 / 5 / 4 / 7"),
    );
  }, { tab: "WEATHER" });

  // ---- ENERGY: the grid's carbon forecast (National Grid ESO) and your Octopus account
  ui.defineScreen("energy", () => {
    sheet(
      place(tile("Clean energy · gCO₂/kWh · next 24 h", feed(["carbon.slots", "carbon.best", "carbon.green", "time.minute"], ([sl, best, lim]) => {
        if (!sl) return div("–");
        const o = { limit: lim ?? 100, w: 470, h: 200, now: 0, limitLabel: `${lim ?? 100} g`, labels: sl.map((x, i) => [x[0].slice(0, 2), i]).filter(([h, i]) => (sl[i][0].endsWith(":00") && +h % 6 === 0) || i === 0) };
        if (best) { const a = sl.findIndex((x) => x[0] === best[0]); if (a >= 0) { o.window = [a, Math.min(a + 3, sl.length - 1)]; o.windowLabel = `cleanest ${best[0]}`; } }
        return limitBars(sl.map((x) => x[1]), o);
      }, MIN), { glyph: "leaf", tone: "fill", count: feed(["carbon.region", "carbon.slots"], ([r, sl]) => `${r ?? "–"} · solid = under 100 g${sl ? ` · ${Math.round((100 * sl.filter((x) => x[1] <= 100).length) / sl.length)}% green` : ""}`) }), "1 / 1 / 3 / 5"),
      place(tile("Grid now", feed("carbon.now", (g) => bignum(num(g), { unit: "g" })), { glyph: "leaf", count: feed("carbon.index", (i) => [sigil("state", i === "very low" || i === "low" ? "ok" : i === "moderate" ? "warn" : "err", 10), ` ${i ?? "–"}`]) }), "1 / 5 / 2 / 7"),
      place(tile("Cleanest 2 h", feed("carbon.best", (b) => (b ? div({ style: "font:600 26px/1.1 var(--cond)" }, `${b[0]}–${b[1]}`, div({ style: "font:500 12px var(--mono)" }, `avg ${b[2]} g`)) : div("–"))), { glyph: "clock", count: "run the heavy loads then" }), "2 / 5 / 3 / 7"),
      place(tile("Unit rate", feed(["power.rate", "power.standing"], ([r, sc]) => div({ style: "width:100%" }, bignum(num(r, 1), { unit: "p" }), readouts([{ label: "Stand", value: sc ?? 0, max: 100, text: num(sc, 1), unit: "p/d" }]))), { glyph: "plug", count: feed("power.tariff", (t) => t ?? "–") }), "3 / 1 / 4 / 3"),
      place(tile("Last full day", feed(["power.usage_kwh", "power.usage_cost"], ([k, c]) => (k == null ? div("–") : div({ style: "display:flex;gap:14px;align-items:flex-end" }, bignum(num(k, 1), { unit: "kWh" }), div({ style: "font:600 20px var(--cond)" }, "£" + num(c, 2))))), { glyph: "bolt", count: feed("power.usage_day", (d) => d ?? "no meter data yet") }), "3 / 3 / 4 / 5"),
      place(tile("Week · kWh", feed("power.week", (w) => (w && w.length ? areaChart(w.map((x) => x[1]), { w: 238, h: 64, marks: true, fmt: (i, v) => `${v}`, labels: [[`−${w.length - 1}d`, 0], ["last", w.length - 1]] }) : div("–")), MIN), { glyph: "pulse", tone: "fill", count: feed("power.week", (w) => (w && w.length ? `${w.length} days · avg ${num(w.reduce((a, x) => a + x[1], 0) / w.length, 1)}` : "no meter data yet")) }), "3 / 5 / 4 / 7"),
    );
  }, { tab: "ENERGY" });

  // ---- LISTS: data/groceries.json and data/notes.json (Claude edits them when you ask)
  ui.defineScreen("lists", () => {
    sheet(
      place(tile("Shopping", feed("shop.items", (it) => (it && it.length ? checklist(it.slice(0, 9).map((i) => [i.text, i.qty, i.done])) : div({ style: "font:600 17px var(--sans)" }, "Nothing needed"))), { glyph: "bag", tone: "list", count: feed("shop.open", (n) => `${n ?? 0} to get · data/groceries.json`) }), "1 / 1 / 4 / 4"),
      place(tile("Notes", feed("notes.items", (it) => (it && it.length ? lines(it.slice(0, 9).map((n) => [n.due ?? "", n.text, ""])) : div({ style: "font:600 17px var(--sans)" }, "No notes. Ask Claude to remember something."))), { glyph: "flag", tone: "list", count: feed("notes.open", (n) => `${n ?? 0} notes · data/notes.json`) }), "1 / 4 / 4 / 7"),
    );
  }, { tab: "LISTS" });

  // ---- CLAUDE: what it heard, what it said, and any timer (the harness writes claude.* and timer.* through POST /push)
  ui.defineScreen("claude", () => {
    sheet(
      place(tile("You asked", feed("claude.heard", (t) => div({ class: "ask" }, t), { fallback: div({ class: "ask" }, "Say “Hey Claude”.") }), { glyph: "sparkle", tone: "fill",
        count: feed(["claude.state", "claude.at"], ([st, at]) => [sigil("state", st === "thinking" || st === "speaking" ? "warn" : st === "listening" ? "ok" : "idle", 10), ` ${st ?? "idle"}${at ? " · " + at : ""}`]) }), "1 / 1 / 2 / 7"),
      place(tile("Answer", feed("claude.say", (t) => div({ style: "font:600 22px/1.3 var(--sans)" }, clamp(7, t)), { fallback: div({ style: "font:500 17px var(--sans)" }, "Nothing to say yet.") }), { inverted: true, glyph: "sparkle", count: feed("claude.at", (at) => (at ? `said ${at}` : "waiting")) }), "2 / 1 / 4 / 4"),
      place(tile("Timer", feed(["timer.end", "timer.total", "timer.label"], ([end, total, label]) => {
        const left = Math.round(((end ?? 0) - Date.now()) / 1000);
        return end && left > -60 ? liveTimer(Math.max(0, left), { label: label ?? "timer", total: total ?? left }) : div({ style: "font:500 15px var(--sans)" }, "No timer running.");
      }), { glyph: "clock", tone: "fill", count: feed("timer.end", (e) => (e ? `rings ${new Date(e).toTimeString().slice(0, 5)}` : "say “set a timer”")) }), "2 / 4 / 4 / 7"),
    );
  }, { tab: "CLAUDE" });

  // ---- SYSTEM: the computer itself (feeds/machine.py, net.py, wifi.sh)
  ui.defineScreen("system", () => {
    sheet(
      place(tile("¹ cpu", feed(["cpu.hist", "cpu.cores", "sys.load_hist", "cpu.count", "cpu.total"], ([h, cores, lh, n, tot]) => {
        const right = cores && Object.keys(cores).length
          ? sparkRows(Object.keys(cores).slice(0, 4).map((k) => ({ label: "C" + k.slice(3), values: last(cores[k], 18), text: num(cores[k].at(-1) * 100) + "%" })), { rows: 4 })
          : sparkRows([["1m", "1"], ["5m", "5"], ["15m", "15"]].map(([label, k]) => ({ label, values: last((lh?.[k] ?? []).map((v) => v / (n || 1)), 18), text: num(lh?.[k]?.at(-1), 2) })), { rows: 4 });
        return div({ style: "display:flex;gap:16px;width:100%;height:100%;align-items:stretch" },
          div({ style: "flex:1.15;min-width:0;display:flex;flex-direction:column;justify-content:space-between;gap:4px" }, dotBars(last(h, 22), { rows: 15 }), small(`total ${num(tot)}% · ${n ?? "–"} cores`)),
          div({ style: "flex:1;min-width:0;display:flex;flex-direction:column;justify-content:center" }, right));
      }), { tone: "legend", count: feed(["sys.load", "sys.load5", "sys.load15", "sys.temp"], ([a, b, c, t]) => [sigil("trend", "flat"), ` load ${num(a, 2)} ${num(b, 2)} ${num(c, 2)}` + (t != null ? ` · ${t}°C` : "")]) }), "1 / 1 / 3 / 5"),
      place(tile("² mem", feed(["mem.apps", "mem.cache", "mem.free", "swap.used", "swap.total"], ([a, c, f, su, st]) => div({ style: "width:100%" }, stackBar([{ label: "apps", value: +num(a, 1), look: "solid" }, { label: "cache", value: +num(c, 1), look: "hatch" }, { label: "free", value: +num(f, 1), look: "outline" }]), div({ style: "height:7px" }),
        readouts([{ label: "Swap", value: su ?? 0, max: Math.max(st ?? 1, 0.1), text: num(su, 1), unit: `/${num(st, 0)}G` }]))), { tone: "legend", count: feed("mem.total", (t) => `${num(t, 0)} GB`) }), "1 / 5 / 2 / 7"),
      place(tile("³ net", feed(["net.down_hist", "net.up_hist"], ([d, u]) => { const top = Math.max(0.5, ...(d ?? []), ...(u ?? [])); return div({ style: "width:100%" }, dotMirror(sqrtScale(last(d, 44), top), sqrtScale(last(u, 44), top), { rows: 5 })); }),
        { tone: "legend", count: feed(["net.iface", "net.down", "net.up"], ([i, d, u]) => [ui.h("b", "↓ " + num(d, 1)), " ", ui.h("b", "↑ " + num(u, 1)), ` MB/s · ${i ?? "–"}`]) }), "2 / 5 / 3 / 7"),
      place(tile("⁶ system", feed(["sys.host", "sys.kernel", "sys.uptime", "net.ip", "wifi.ssid", "wifi.rssi", "proc.count"], ([host, kern, up, ip, ssid, rssi, np]) => kvgrid({ host: host ?? "–", kernel: kern ?? "–", up: upfmt(up), ip: ip ?? "–", wifi: rssi != null ? `${rssi} dBm` : "–", procs: np ?? "–" }, 2)), { tone: "legend", count: "feeds/machine.py · every 3 s" }), "3 / 1 / 4 / 4"),
      feed(["disk.hist", "disk.total", "disk.used"], ([h, t, u]) => place(usageTile("⁴ disk", h && h.length ? h : [u ?? 0], t ?? 1, `${num(u, 2)} TB`).set({ class: "legend" }), "3 / 4 / 4 / 5")),
      place(tile("⁵ proc", feed("proc.top", (rows) => lines((rows ?? []).slice(0, 3).map((r) => [sigil("state", r.cpu > 50 ? "warn" : "ok", 10), r.name, num(r.cpu, 1) + "%"]))), { tone: "legend list", count: feed("proc.count", (n) => `top 3 of ${n}`) }), "3 / 5 / 4 / 7"),
    );
  }, { tab: "SYSTEM" });

  // The display's chrome: registration marks, live sigils in the corners (left = the machine: link, wifi, cpu, disk; right = the world:
  // wind, cloud cover, moon, events left) and the tab bar. Call once after the fonts are ready; it also shows the home screen.
  ui.shell = (name?: string) => {
    ui.frame({
      tabs: true,
      tl: () => [["state", ui.get("link.server") === "up" ? "ok" : "err"], ["bars", ui.get("wifi.bars") ?? 0], ["level", [ui.get("cpu.total") ?? 0, 100]], ["level", [ui.get("disk.pct") ?? 0, 100]]],
      tr: () => [["dir", ui.get("wx.wind_dir") ?? 0], ["level", [ui.get("wx.cloud") ?? 0, 8]], ["glyph", "moon"], ["count", ui.get("cal.left") ?? 0]],
    });
    ui.start(name);
  };
})();
