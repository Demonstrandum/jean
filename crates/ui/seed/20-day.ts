// DAY: the day as a route diagram (today while something is still to come, else the next day with events), busy hours, what is next, the sun and moon.
ui.defineScreen({
  id: "day",
  tab: "DAY",
  tiles: [
    {
      id: "route", at: zone(Z.A1, Z.F2), title: "Day", glyph: "calendar", tone: "fill",
      body: bind([K.time.minute, K.cal.events], ([, events]) => {
        const now = Date.now(), focus = calendar.focus(events ?? [], now), timed = focus.events.filter((e) => !e.allDay);
        const from = Math.min(focus.day + 7 * 3_600_000, ...timed.map((e) => e.start)), to = Math.max(focus.day + 21 * 3_600_000, ...timed.map((e) => e.end + 1_800_000));
        const hour = 3_600_000;
        return routes({
          from: Math.floor(from / hour) * hour, to: Math.ceil(to / hour) * hour, now: focus.isToday ? now : undefined, w: 722,
          // events that overlap get their own lane, so nothing is drawn on top of anything
          lanes: calendar.lanes(timed).map((lane, i) => ({
            label: i === 0 ? (focus.isToday ? "Today" : fmt.weekday(focus.day)) : "",
            look: (["solid", "dash", "dot"] as const)[i % 3],
            stops: lane.map((e) => ({ at: e.start, to: Math.max(e.end, e.start + 900_000), name: e.title })),
          })),
        });
      }, { tier: Minute }),
      count: bind([K.time.minute, K.cal.events], ([, events]) => {
        const focus = calendar.focus(events ?? [], Date.now());
        return `${focus.isToday ? "today" : fmt.date(focus.day)} · ${focus.events.length} event${focus.events.length === 1 ? "" : "s"} · Google Calendar`;
      }, { tier: Minute }),
    },
    {
      id: "busy", at: zone(Z.A3, Z.B3), title: "Busy", glyph: "clock",
      body: bind([K.time.minute, K.cal.events], ([, events]) => {
        const hours = calendar.focus(events ?? [], Date.now()).events.filter((e) => !e.allDay).reduce((sum, e) => sum + (e.end - e.start) / 3_600_000, 0);
        return div({ style: "width:100%" }, waffle(hours, 14, { cols: 14 }), div({ style: "height:7px" }), readouts([{ label: "Busy", value: hours, max: 14, text: fmt.num(hours, 1), unit: "h" }]));
      }, { tier: Minute }),
      count: bind([K.time.minute, K.cal.events], ([, events]) => { const f = calendar.focus(events ?? [], Date.now()); return f.isToday ? "of 14 h today" : `of 14 h · ${fmt.weekday(f.day)}`; }, { tier: Minute }),
    },
    {
      id: "next", at: zone(Z.C3, Z.D3), title: "Next", glyph: "flag", tone: "list",
      body: bind([K.time.minute, K.cal.events], ([, events]) => {
        const now = Date.now(), up = calendar.upcoming(events ?? [], now, 3);
        return up.length ? lines(up.map((e) => [calendar.when(e, now), e.title, e.allDay ? "" : fmt.duration((e.end - e.start) / 60000)])) : div("Nothing this week");
      }, { tier: Minute }),
      count: "next 7 days · Google Calendar",
    },
    {
      id: "sun", at: zone(Z.E3, Z.F3), title: "Sun & moon", glyph: "sun",
      body: bind([K.sun.rise, K.sun.set, K.moon.phase], ([rise, set, phase]) => div({ style: "display:flex;gap:12px;align-items:center;width:100%" },
        moon(phase ?? 0, 46), kvgrid({ rise: fmt.clock(rise), set: fmt.clock(set), moon: label.moon(moonPhaseOf(phase ?? 0)) }, 1))),
      count: bind([K.sun.rise, K.sun.set], ([rise, set]) => (rise && set ? `day ${fmt.duration((set - rise) / 60000)}` : "")),
    },
  ],
});
