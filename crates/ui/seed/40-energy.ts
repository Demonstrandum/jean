// ENERGY: the grid's 24-hour carbon forecast against the green line, your unit rate, last full day's usage, the week.
ui.defineScreen({
  id: "energy",
  tab: "ENERGY",
  tiles: [
    {
      id: "carbon", at: zone(Z.A1, Z.D2), title: "Clean energy · gCO₂/kWh · next 24 h", glyph: "leaf", tone: "fill",
      body: bind([K.carbon.slots, K.carbon.best, K.carbon.green, K.time.minute], ([slots, best, line]) => {
        if (!slots?.length) return div("–");
        const startsOnHour = (i: number) => new Date(slots[i].at).getMinutes() === 0;
        const options: Parameters<typeof limitBars>[1] = {
          limit: line ?? 100, w: 470, h: 200, now: 0, limitLabel: `${line ?? 100} g`,
          labels: slots.map((s, i) => [fmt.hour(s.at), i] as [string, number]).filter(([, i]) => i === 0 || (startsOnHour(i) && new Date(slots[i].at).getHours() % 6 === 0)),
        };
        if (best) {
          const a = slots.findIndex((s) => s.at >= best.from), b = slots.findIndex((s) => s.at >= best.to);
          if (a >= 0) Object.assign(options, { window: [a, (b > a ? b : slots.length) - 1], windowLabel: `cleanest ${fmt.clock(best.from)}` });
        }
        return limitBars(slots.map((s) => s.grams), options);
      }, { tier: Minute }),
      count: bind([K.carbon.region, K.carbon.slots, K.carbon.green], ([region, slots, line]) =>
        `${region ?? "–"} · solid = under ${line ?? 100} g${slots?.length ? ` · ${Math.round((100 * slots.filter((s) => isGreen(s.grams, line ?? 100)).length) / slots.length)}% green` : ""}`),
    },
    {
      id: "grid", at: zone(Z.E1, Z.F1), title: "Grid now", glyph: "leaf",
      body: bind(K.carbon.now, (g) => bignum(fmt.num(g), { unit: "g" })),
      count: bind(K.carbon.index, (index) => [sigil(sig.state(stateOf.carbon(index ?? CarbonIndex.Moderate)), 10), ` ${label.carbon(index ?? CarbonIndex.Moderate)}`]),
    },
    {
      id: "cleanest", at: zone(Z.E2, Z.F2), title: "Cleanest 2 h", glyph: "clock",
      body: bind(K.carbon.best, (best) => (best ? div({ style: "font:600 26px/1.1 var(--cond)" }, `${fmt.clock(best.from)}–${fmt.clock(best.to)}`, div({ style: "font:500 12px var(--mono)" }, `avg ${fmt.num(best.avg)} g`)) : div("–"))),
      count: "run the heavy loads then",
    },
    {
      id: "rate", at: zone(Z.A3, Z.B3), title: "Unit rate", glyph: "plug",
      body: bind([K.power.rate, K.power.standing], ([rate, standing]) => div({ style: "width:100%" }, bignum(fmt.num(rate, 1), { unit: "p" }), readouts([{ label: "Stand", value: standing ?? 0, max: 100, text: fmt.num(standing, 1), unit: "p/d" }]))),
      count: bind(K.power.tariff, (tariff) => tariff ?? "–"),
    },
    {
      id: "usage", at: zone(Z.C3, Z.D3), title: "Last full day", glyph: "bolt",
      body: bind([K.power.usageKwh, K.power.usageCost, K.power.usageDay], ([kwh, cost, day]) => (day == null ? div("–") : div({ style: "display:flex;gap:14px;align-items:flex-end" }, bignum(fmt.num(kwh, 1), { unit: "kWh" }), div({ style: "font:600 20px var(--cond)" }, "£" + fmt.num(cost, 2))))),
      count: bind(K.power.usageDay, (day) => (day == null ? "no meter data yet" : fmt.date(day))),
    },
    {
      id: "week", at: zone(Z.E3, Z.F3), title: "Week · kWh", glyph: "pulse", tone: "fill",
      body: bind(K.power.week, (week) => (week?.length ? areaChart(week.map((d) => d.kwh), { w: 238, h: 64, marks: true, fmt: (_i, v) => `${v}`, labels: [[`−${week.length - 1}d`, 0], ["last", week.length - 1]] }) : div("–")), { tier: Minute }),
      count: bind(K.power.week, (week) => (week?.length ? `${week.length} days · avg ${fmt.num(week.reduce((a, d) => a + d.kwh, 0) / week.length, 1)}` : "no meter data yet")),
    },
  ],
});
