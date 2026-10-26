// WEATHER: now, the next 24 hours, rain by hour, wind, the UV day curve (as the tile's background), air quality.
ui.defineScreen({
  id: "weather",
  tab: "WEATHER",
  tiles: [
    {
      id: "now", at: zone(Z.A1, Z.B2), title: "Now", glyph: "cloud",
      body: bind([K.wx.sky, K.wx.temp], ([sky, temp]) => div({ style: "display:flex;flex-direction:column;align-items:center;width:100%;height:100%;gap:4px" },
        weatherIcon(skyIcon(sky ?? Sky.Cloudy), 64), pane(bignum(fmt.num(temp) + "°")))),
      count: bind([K.wx.sky, K.wx.feels], ([sky, feels]) => `${label.sky(sky ?? Sky.Cloudy)} · feels ${fmt.num(feels)}°`),
    },
    {
      id: "temps", at: zone(Z.C1, Z.F1), title: "Next 24 h · °C", glyph: "thermo", tone: "fill",
      body: bind(K.wx.hours, (hours) => areaChart((hours ?? []).map((h) => h.temp), { w: 468, h: 72, labels: [["now", 0], ["+6h", 6], ["+12h", 12], ["+18h", 18]] }), { tier: Minute }),
      count: bind(K.wx.hours, (hours) => { const t = (hours ?? []).map((h) => h.temp); return t.length ? `min ${fmt.num(Math.min(...t), 1)} · max ${fmt.num(Math.max(...t), 1)}` : "–"; }),
    },
    {
      id: "rain", at: zone(Z.C2, Z.F2), title: "Rain chance · next 24 h · by hour", glyph: "drop", tone: "fill",
      body: bind(K.wx.hours, (hours) => dotBars((hours ?? []).map((h) => h.rain / 100), { rows: 3, labels: (hours ?? []).map((h) => fmt.hour(h.at)) })),
      count: bind([K.wx.rainFrom, K.wx.hours], ([from, hours]) => `${from ? `rain from ${fmt.clock(from)}` : "no rain expected"} · peak ${Math.max(0, ...(hours ?? []).map((h) => h.rain))}%`),
    },
    {
      id: "wind", at: zone(Z.A3, Z.B3), title: "Wind", glyph: "wind",
      body: bind([K.wx.wind, K.wx.windDir], ([speed, dir]) => div({ style: "display:flex;gap:14px;align-items:center;width:100%;height:100%" }, sigil(sig.dir(dir ?? 0), 38), pane(bignum(fmt.num(speed), { unit: "mph" })))),
      count: bind([K.wx.windDir, K.wx.pressure], ([dir, pressure]) => `from ${fmt.compass(dir ?? 0)} · ${fmt.num(pressure)} hPa`),
    },
    {
      id: "uv", at: zone(Z.C3, Z.D3), title: "UV index", glyph: "sun",
      body: bind([K.wx.uvNow, K.wx.uvMax], ([uv, max]) => div({ style: "display:flex;gap:16px;align-items:center;width:100%;height:100%" }, pane(bignum(fmt.num(uv, 1))),
        div({ style: "font:600 15px/1.35 var(--sans)" }, label.uv(uvLevel(uv ?? 0)), div({ style: "font:500 12px var(--mono)" }, `max ${fmt.num(max, 1)}`)))),
      count: bind(K.wx.uvNow, (uv) => [sigil(sig.level(Math.min(11, uv ?? 0), 11), 11), " scale 0–11+"]),
      // the day's curve is the tile's background: nothing else moves for it
      back: bind([K.wx.uvDay, K.time.minute], ([day]) => { const d = new Date(); return dayCurve(day ?? Array(24).fill(0), { now: d.getHours() + d.getMinutes() / 60, max: 1.5, w: 243, h: 134, padT: 34, padB: 28.25, baseline: false, guides: [[3, "3"], [6, "6"]] }); }, { tier: Minute }),
    },
    {
      id: "air", at: zone(Z.E3, Z.F3), title: "Air quality", glyph: "leaf",
      body: bind([K.air.eaqi, K.air.pm25], ([eaqi, pm25]) => div({ style: "display:flex;gap:16px;align-items:center;width:100%;height:100%" }, pane(bignum(fmt.num(eaqi))),
        div({ style: "font:600 15px/1.35 var(--sans)" }, label.air(airBand(eaqi ?? 0)), div({ style: "font:500 12px var(--mono)" }, `PM2.5 ${fmt.num(pm25, 1)}`)))),
      count: bind([K.air.eaqi, K.air.main], ([eaqi, main]) => [sigil(sig.state(stateOf.air(airBand(eaqi ?? 0))), 10), ` European AQI · ${label.pollutant(main ?? Pollutant.Pm25)}`]),
    },
  ],
});
