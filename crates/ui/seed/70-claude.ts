// CLAUDE: what it heard, what it said, and any timer. The harness writes K.claude.* and K.timer.* through POST /push.
ui.defineScreen({
  id: "claude",
  tab: "腦",
  tiles: [
    {
      id: "heard", at: zone(Z.A1, Z.F1), title: "You asked", glyph: "sparkle", tone: "fill",
      body: bind(K.claude.heard, (heard) => div({ class: "ask" }, heard ?? ""), { fallback: div({ class: "ask" }, "Say “Hey Claude”.") }),
      count: bind([K.claude.state, K.claude.at], ([state, at]) => [sigil(sig.state(stateOf.presence(state ?? Presence.Idle)), 10), ` ${label.presence(state ?? Presence.Idle)}${at ? " · " + fmt.clock(at) : ""}`]),
    },
    {
      id: "answer", at: zone(Z.A2, Z.C3), title: "Answer", glyph: "sparkle",
      // black while the assistant is busy: what it is doing, step by step, and what it says
      inverted: invertWhen(K.claude.state, (state) => (state ?? Presence.Idle) !== Presence.Idle),
      body: bind([K.claude.state, K.claude.heard, K.claude.say, K.claude.steps, K.claude.task], ([state, heard, say, steps, task]) => claudeLive(state ?? Presence.Idle, state !== Presence.Idle ? undefined : heard, say, steps ?? [], task, "large"), { tier: Tier.Tick, fallback: div({ style: "font:500 17px var(--sans)" }, "Nothing to say yet.") }),
      count: bind([K.claude.state, K.claude.at], ([state, at]) => ((state ?? Presence.Idle) !== Presence.Idle ? label.presence(state ?? Presence.Idle) : at ? `said ${fmt.clock(at)}` : "waiting"), { tier: Tier.Tick, fallback: "waiting" }),
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
