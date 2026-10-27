// ROOM: the living room's own sensors and lights, from Home Assistant (feeds/homeassistant.py).
ui.defineScreen({
  id: "room",
  tab: "室",
  tiles: [
    {
      id: "climate", at: zone(Z.A1, Z.C2), title: "Climate", glyph: "thermo",
      body: bind([K.room.temp, K.room.humidity], ([temp, humidity]) => div({ style: "display:flex;gap:28px;align-items:flex-end" },
        bignum(fmt.num(temp, 1), { unit: "°C" }), bignum(fmt.num(humidity), { unit: "%" }))),
      count: "temperature · humidity",
    },
    {
      id: "light", at: zone(Z.D1, Z.D2), title: "Light", glyph: "sun",
      body: bind(K.room.lux, (lux) => bignum(fmt.num(lux), { unit: "lx" })),
      count: bind(K.room.lux, (lux) => (lux == null ? "no sensor" : lux < 50 ? "dim" : lux < 300 ? "indoor" : "bright")),
    },
    {
      id: "motion", at: zone(Z.E1, Z.F2), title: "Motion", glyph: "clock",
      body: bind([K.time.minute, K.room.motion], ([, at]) => (at == null ? div("None seen") : bignum(fmt.duration(Math.max(0, Math.floor((Date.now() - at) / 60000))), {})), { tier: Minute }),
      count: bind(K.room.motion, (at) => (at == null ? "since start" : `last ${fmt.clock(at)}`)),
    },
    {
      id: "lights", at: zone(Z.A3, Z.C3), title: "Lights", glyph: "sun", tone: "list",
      body: bind(K.room.lights, (lights) => (lights?.length ? checklist(lights.slice(0, 6).map((l) => [l.name, l.on ? "on" : "off", l.on])) : div("No lights"))),
      count: bind(K.room.lights, (lights) => `${(lights ?? []).filter((l) => l.on).length} of ${(lights ?? []).length} on`),
    },
    {
      id: "doors", at: zone(Z.D3, Z.F3), title: "Doors", glyph: "flag", tone: "list",
      body: bind(K.room.doors, (doors) => (doors?.length ? lines(doors.slice(0, 5).map((d) => [sigil(sig.state(d.open ? State.Warn : State.Ok), 10), d.name, d.open ? "open" : "closed"])) : div("No sensors"))),
      count: bind(K.room.doors, (doors) => `${(doors ?? []).filter((d) => d.open).length} open`),
    },
  ],
});
