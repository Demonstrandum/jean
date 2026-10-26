// CLAUDE: what it heard, what it said, and any timer. The harness writes K.claude.* and K.timer.* through POST /push.
ui.defineScreen({
  id: "claude",
  tab: "CLAUDE",
  tiles: [
    {
      id: "heard", at: zone(Z.A1, Z.F1), title: "You asked", glyph: "sparkle", tone: "fill",
      body: bind(K.claude.heard, (heard) => div({ class: "ask" }, heard ?? ""), { fallback: div({ class: "ask" }, "Say “Hey Claude”.") }),
      count: bind([K.claude.state, K.claude.at], ([state, at]) => [sigil(sig.state(stateOf.presence(state ?? Presence.Idle)), 10), ` ${label.presence(state ?? Presence.Idle)}${at ? " · " + fmt.clock(at) : ""}`]),
    },
    {
      id: "answer", at: zone(Z.A2, Z.C3), title: "Answer", glyph: "sparkle", inverted: true,
      body: bind(K.claude.say, (say) => div({ style: "font:600 22px/1.3 var(--sans)" }, clamp(7, say ?? "")), { fallback: div({ style: "font:500 17px var(--sans)" }, "Nothing to say yet.") }),
      count: bind(K.claude.at, (at) => (at ? `said ${fmt.clock(at)}` : "waiting")),
    },
    {
      id: "timer", at: zone(Z.D2, Z.F3), title: "Timer", glyph: "clock", tone: "fill",
      body: bind([K.timer.end, K.timer.total, K.timer.label], ([end, total, name]) => {
        const left = Math.round(((end ?? 0) - Date.now()) / 1000);
        return end && left > -60 ? liveTimer(Math.max(0, left), { label: name ?? "timer", total: total ?? left }) : div({ style: "font:500 15px var(--sans)" }, "No timer running.");
      }),
      count: bind(K.timer.end, (end) => (end ? `rings ${fmt.clock(end)}` : "say “set a timer”")),
    },
  ],
});
