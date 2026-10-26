// SYSTEM: the computer itself (feeds/machine.py, net.py, wifi.sh).
ui.defineScreen({
  id: "system",
  tab: "SYSTEM",
  tiles: [
    {
      id: "cpu", at: zone(Z.A1, Z.D2), title: "¹ cpu", tone: "legend",
      body: bind([K.cpu.hist, K.cpu.cores, K.sys.loadHist, K.cpu.count, K.cpu.total], ([hist, cores, loads, count, total]) => {
        const names = Object.keys(cores ?? {}).slice(0, 4);
        const right = names.length
          ? sparkRows(names.map((name) => ({ label: "C" + name.replace(/\D/g, ""), values: series.last(cores![name], 18), text: fmt.percent(cores![name].at(-1)! * 100) })), { rows: 4 })
          : sparkRows([["1m", loads?.m1], ["5m", loads?.m5], ["15m", loads?.m15]].map(([name, values]) => ({ label: name as string, values: series.last((values as number[] | undefined)?.map((v) => v / (count || 1)), 18), text: fmt.num((values as number[] | undefined)?.at(-1), 2) })), { rows: 4 });
        return div({ style: "display:flex;gap:16px;width:100%;height:100%;align-items:stretch" },
          div({ style: "flex:1.15;min-width:0;display:flex;flex-direction:column;justify-content:space-between;gap:4px" }, dotBars(series.last(hist, 22), { rows: 15 }), small(`total ${fmt.percent(total)} · ${count ?? "–"} cores`)),
          div({ style: "flex:1;min-width:0;display:flex;flex-direction:column;justify-content:center" }, right));
      }),
      count: bind([K.sys.load, K.sys.load5, K.sys.load15, K.sys.temp], ([a, b, c, temp]) => [sigil(sig.trend(Trend.Flat)), ` load ${fmt.num(a, 2)} ${fmt.num(b, 2)} ${fmt.num(c, 2)}${temp != null ? ` · ${temp}°C` : ""}`]),
    },
    {
      id: "mem", at: zone(Z.E1, Z.F1), title: "² mem", tone: "legend",
      body: bind([K.mem.apps, K.mem.cache, K.mem.free, K.mem.swapUsed, K.mem.swapTotal], ([apps, cache, free, swapUsed, swapTotal]) => div({ style: "width:100%" },
        stackBar([{ label: "apps", value: +fmt.num(apps, 1), look: "solid" }, { label: "cache", value: +fmt.num(cache, 1), look: "hatch" }, { label: "free", value: +fmt.num(free, 1), look: "outline" }]), div({ style: "height:7px" }),
        readouts([{ label: "Swap", value: swapUsed ?? 0, max: Math.max(swapTotal ?? 1, 0.1), text: fmt.num(swapUsed, 1), unit: `/${fmt.num(swapTotal)}G` }]))),
      count: bind(K.mem.total, (total) => `${fmt.num(total)} GB`),
    },
    {
      id: "net", at: zone(Z.E2, Z.F2), title: "³ net", tone: "legend",
      body: bind([K.net.downHist, K.net.upHist], ([down, up]) => { const top = Math.max(0.5, ...(down ?? []), ...(up ?? [])); return div({ style: "width:100%" }, dotMirror(series.sqrtScale(series.last(down, 44), top), series.sqrtScale(series.last(up, 44), top), { rows: 5 })); }),
      count: bind([K.net.iface, K.net.down, K.net.up], ([iface, down, up]) => [ui.h("b", "↓ " + fmt.num(down, 1)), " ", ui.h("b", "↑ " + fmt.num(up, 1)), ` MB/s · ${iface ?? "–"}`]),
    },
    {
      id: "host", at: zone(Z.A3, Z.C3), title: "⁶ system", tone: "legend",
      body: bind([K.sys.host, K.sys.kernel, K.sys.uptime, K.net.ip, K.wifi.rssi, K.proc.count], ([host, kernel, uptime, ip, rssi, procs]) =>
        kvgrid({ host: host ?? "–", kernel: kernel ?? "–", up: fmt.uptime(uptime), ip: ip ?? "–", wifi: rssi != null ? `${rssi} dBm` : "–", procs: String(procs ?? "–") }, 2)),
      count: "feeds/machine.py · every 3 s",
    },
    {
      id: "disk", at: zone(Z.D3), title: "⁴ disk", tone: ["legend", "usage"],
      back: bind([K.disk.hist, K.disk.total, K.disk.used], ([hist, total, used]) => usageFill(hist?.length ? hist : [used ?? 0], total ?? 1)),
      count: bind(K.disk.used, (used) => chip(`${fmt.num(used, 2)} TB`)),
    },
    {
      id: "proc", at: zone(Z.E3, Z.F3), title: "⁵ proc", tone: ["legend", "list"],
      body: bind(K.proc.top, (top) => lines((top ?? []).slice(0, 3).map((p) => [sigil(sig.state(p.cpu > 50 ? State.Warn : State.Ok), 10), p.name, fmt.num(p.cpu, 1) + "%"]))),
      count: bind(K.proc.count, (n) => `top 3 of ${n ?? "–"}`),
    },
  ],
});
