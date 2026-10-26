// HOME: the day dial, weather, the next events, shopping, grid carbon, the machine, the link, and Claude's last words.
// Every live value is a typed key bound with `bind(...)`; a tile is data, so `ui.screenOf("home").patchTile("grid", {...})` changes one.
ui.defineScreen({
  id: "home",
  tab: "首",
  tiles: [
    {
      id: "day", at: zone(Z.A1, Z.B2), title: "", frameless: true,
      body: bind([K.time.minute, K.sun.rise, K.sun.set], ([, rise, set]) => dayDial({ rise: rise ?? 0, set: set ?? 0, size: 244 }), { tier: Minute }),
    },
    {
      id: "weather", at: zone(Z.C1, Z.C2), title: "Wx", glyph: "cloud", tone: "iconic", waves: { from: 34 },
      body: bind(K.wx.sky, (sky) => weatherIcon(skyIcon(sky ?? Sky.Cloudy), 46)),
      count: bind([K.wx.temp, K.wx.sky, K.wx.rainFrom], ([temp, sky, rain]) => [
        div(`${fmt.num(temp)}°C ${label.sky(sky ?? Sky.Cloudy).toLowerCase()}`),
        div(rain ? `rain ${fmt.clock(rain)}` : "dry 24 h"),
      ]),
    },
    {
      id: "calendar", at: zone(Z.D1, Z.E1), title: "Calendar", glyph: "calendar", tone: "list",
      body: bind([K.time.minute, K.cal.events], ([, events]) => {
        const now = Date.now(), up = calendar.upcoming(events ?? [], now, 3);
        return up.length
          ? lines(up.map((e) => [calendar.when(e, now), e.title, e.allDay ? "" : fmt.duration((e.end - e.start) / 60000)]))
          : div({ style: "font:600 15px var(--sans)" }, "Nothing this week");
      }, { tier: Minute }),
      count: bind([K.time.minute, K.cal.events], ([, events]) => {
        const next = calendar.next(events ?? [], Date.now());
        return next ? `next in ${fmt.duration(Math.max(0, next.inMinutes))}` : "clear for a week";
      }, { tier: Minute }),
    },
    {
      id: "shop", at: zone(Z.F1, Z.F2), title: "Shop", glyph: "bag", tone: "list",
      body: bind(K.shop.items, (items) => (items?.length ? checklist(items.slice(0, 6).map((i) => [i.text, i.qty, i.done])) : div("Nothing needed"))),
      count: bind(K.shop.items, (items) => `${(items ?? []).filter((i) => !i.done).length} to get`),
    },
    {
      id: "grid", at: zone(Z.D2), title: "Grid", glyph: "leaf",
      body: bind(K.carbon.now, (g) => bignum(fmt.num(g), { unit: "g" })),
      count: bind([K.carbon.index, K.carbon.best], ([index, best]) => [sigil(sig.state(stateOf.carbon(index ?? CarbonIndex.Moderate)), 10), ` ${best ? "best " + fmt.clock(best.from) : label.carbon(index ?? CarbonIndex.Moderate)}`]),
    },
    {
      id: "sys", at: zone(Z.E2), title: "Sys", glyph: "cpu", tone: "list",
      body: bind([K.cpu.total, K.mem.pct, K.disk.pct], ([cpu, mem, disk]) => lines([["cpu", "", fmt.percent(cpu)], ["ram", "", fmt.percent(mem)], ["disk", "", fmt.percent(disk)]])),
      count: bind(K.sys.uptime, (s) => `up ${fmt.uptime(s)}`),
    },
    {
      id: "next24", at: zone(Z.A3, Z.B3), title: "Next 24 h", glyph: "thermo", tone: "fill",
      body: bind(K.wx.hours, (hours) => areaChart((hours ?? []).map((h) => h.temp), { w: 330, h: 64, labels: [["now", 0], ["+6h", 6], ["+12h", 12], ["+18h", 18]] }), { tier: Minute }),
      count: bind(K.wx.hours, (hours) => { const t = (hours ?? []).map((h) => h.temp); return t.length ? `min ${fmt.num(Math.min(...t), 1)} · max ${fmt.num(Math.max(...t), 1)} °C` : "–"; }),
    },
    {
      id: "link", at: zone(Z.C3), title: "Link", glyph: "wifi",
      body: bind([K.wifi.bars, K.net.down, K.net.up], ([bars, down, up]) =>
        div({ style: "display:flex;flex-direction:column;gap:6px;align-items:flex-start" }, sigil(sig.bars(bars ?? 0), 26),
          div({ style: "font:600 13px var(--mono)" }, `↓ ${fmt.num(down, 1)}`), div({ style: "font:600 13px var(--mono)" }, `↑ ${fmt.num(up, 1)}`))),
      count: bind(K.wifi.rssi, (rssi) => (rssi == null ? "MB/s" : `${rssi} dBm`)),
    },
    {
      id: "claude", at: zone(Z.D3, Z.F3), title: "Claude", glyph: "sparkle",
      body: bind(K.claude.say, (say) => div({ style: "font:600 17px/1.35 var(--sans)" }, say ?? ""), { fallback: div({ style: "font:500 15px var(--sans)" }, "Nothing to say yet.") }),
      count: bind(K.claude.at, (at) => `said ${fmt.clock(at)}`, { fallback: "waiting" }),
    },
  ],
});
