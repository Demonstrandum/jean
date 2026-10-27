// The data contract between the machine and the display: every live value the feeds write, as a typed key; the enums they are
// written in; the structures they carry; and the typed functions that turn them into what is shown. Nothing here is parsed
// from strings: times are epoch milliseconds, categories are enum members (integers on the wire), amounts are numbers.
//
// feeds/_schema.py is generated from the enums below (the server writes it at startup), so the Python feeds use the same members.

(() => {
  const ui = (window as any).ui;
  const { Link, State } = window as any;

  /** Epoch milliseconds (what `Date.now()` returns): every instant in the schema. */
  type Ms = number;

  // ---------------------------------------------------------------- enums (the wire carries the integer)

  /** The sky, as the weather symbols draw it. */
  enum Sky { Clear, Partly, Cloudy, Rain, Snow, Storm, Fog }
  /** UV index bands. */
  enum UvLevel { Low, Moderate, High, VeryHigh, Extreme }
  /** European air-quality bands (from the European AQI). */
  enum AirBand { Good, Fair, Moderate, Poor, VeryPoor, ExtremelyPoor }
  /** The grid's carbon-intensity index. */
  enum CarbonIndex { VeryLow, Low, Moderate, High, VeryHigh }
  /** The pollutants the air-quality service reports. */
  enum Pollutant { Pm25, Pm10, No2, O3 }
  /** Generation fuels in the grid mix. */
  enum Fuel { Biomass, Coal, Imports, Gas, Nuclear, Other, Hydro, Solar, Wind }
  /** What the assistant is doing. */
  enum Presence { Idle, Listening, Thinking, Speaking }
  /** The tariff's shape: one rate all day, or a rate per half hour. */
  enum RateKind { Flat, HalfHourly }
  /** The eight named phases of the moon. */
  enum MoonPhase { New, WaxingCrescent, FirstQuarter, WaxingGibbous, Full, WaningGibbous, LastQuarter, WaningCrescent }

  // ---------------------------------------------------------------- structures

  /** One hour of the forecast: temperature in °C, rain probability 0 to 100, UV index. */
  interface Hour { readonly at: Ms; readonly temp: number; readonly rain: number; readonly uv: number }
  /** A calendar event. `end` is exclusive; an all-day event starts at local midnight. */
  interface CalEvent { readonly id: string; readonly title: string; readonly start: Ms; readonly end: Ms; readonly allDay: boolean }
  /** A unit rate for the half hour starting at `at`, in pence per kWh including VAT. */
  interface Rate { readonly at: Ms; readonly pence: number }
  /** Grid carbon intensity for the half hour starting at `at`, in gCO₂ per kWh. */
  interface Intensity { readonly at: Ms; readonly grams: number }
  /** A stretch of time with its average value. */
  interface Span { readonly from: Ms; readonly to: Ms; readonly avg: number }
  /** A value at an instant (a daily low or high). */
  interface Peak { readonly at: Ms; readonly value: number }
  /** One fuel's share of generation, in percent. */
  interface MixShare { readonly fuel: Fuel; readonly pct: number }
  /** A process and the share of one core it uses. */
  interface Proc { readonly name: string; readonly cpu: number }
  /** A shopping-list line. */
  interface Grocery { readonly text: string; readonly qty: string; readonly done: boolean }
  /** A note to keep in mind; `due` is whatever the person wrote ("Fri"). */
  interface Note { readonly text: string; readonly due: string }
  /** Electricity used on one local day, in kWh. */
  interface DayUsage { readonly day: Ms; readonly kwh: number }
  /** Load averages over their history windows, as series. */
  interface LoadHistory { readonly m1: readonly number[]; readonly m5: readonly number[]; readonly m15: readonly number[] }

  // ---------------------------------------------------------------- sample data (the fixtures the self-test uses)

  const H = 3_600_000, midnight = new Date().setHours(0, 0, 0, 0), now = Date.now(), hourStart = Math.floor(now / H) * H;
  const wave = (n: number, base: number, amp: number, seed = 0) => Array.from({ length: n }, (_, i) => +(base + amp * Math.sin(i / 2.3 + seed)).toFixed(2));
  const hours: Hour[] = Array.from({ length: 24 }, (_, i) => ({ at: hourStart + i * H, temp: +(14 + 3 * Math.sin(i / 4)).toFixed(1), rain: Math.max(0, Math.round(60 * Math.sin(i / 5))), uv: 0 }));
  const uvDay = Array.from({ length: 24 }, (_, i) => +Math.max(0, 5 * Math.sin(((i - 6) / 12) * Math.PI)).toFixed(1));
  // relative to now, so the fixtures exercise "an event is on now" and "events later" at any time of day (not only the daytime)
  const events: CalEvent[] = [
    { id: "a", title: "Standup", start: now - 0.5 * H, end: now + 0.5 * H, allDay: false },
    { id: "b", title: "Lunch", start: now + 1.5 * H, end: now + 2.5 * H, allDay: false },
    { id: "c", title: "Parcel pickup", start: now + 2 * H, end: now + 2.25 * H, allDay: false },
    { id: "d", title: "Review", start: midnight + 38 * H, end: midnight + 39.5 * H, allDay: false },
  ];

  const intensity: Intensity[] = Array.from({ length: 48 }, (_, i) => ({ at: midnight + i * 1_800_000, grams: Math.round(60 + 35 * Math.sin(i / 8)) }));

  // ---------------------------------------------------------------- the keys

  /** Every live value, by typed handle: K.wx.temp, K.power.rate, ... The value types are the contract with the feeds. */
  const K = {
    time: { minute: ui.keys.minute as Key<Ms> },
    ui: { screen: ui.keys.screen as Key<string>, hold: ui.keys.hold as Key<number> },
    link: { server: ui.keys.link as Key<number> },

    cpu: {
      total: ui.key<number>("cpu.total", "Overall CPU use, percent.", 18),
      hist: ui.key<number[]>("cpu.hist", "Recent overall CPU use, 0..1 per sample, oldest first.", wave(40, 0.3, 0.15)),
      count: ui.key<number>("cpu.count", "Number of CPU cores.", 10),
      cores: ui.key<Record<string, number[]>>("cpu.cores", "Per-core use histories (0..1), by core name; Linux only.", { cpu0: wave(30, 0.3, 0.2), cpu1: wave(30, 0.4, 0.2, 1) }),
    },
    sys: {
      load: ui.key<number>("sys.load", "One-minute load average.", 1.2),
      load5: ui.key<number>("sys.load5", "Five-minute load average.", 1.1),
      load15: ui.key<number>("sys.load15", "Fifteen-minute load average.", 0.9),
      loadHist: ui.key<LoadHistory>("sys.load_hist", "Load average histories.", { m1: wave(30, 1.2, 0.4), m5: wave(30, 1.1, 0.2), m15: wave(30, 0.9, 0.1) }),
      host: ui.key<string>("sys.host", "Host name.", "display"),
      kernel: ui.key<string>("sys.kernel", "Kernel release.", "6.12.8"),
      uptime: ui.key<number>("sys.uptime", "Seconds since boot.", 1_166_000),
      temp: ui.key<number>("sys.temp", "Hottest sensor, °C (Linux).", 52),
    },
    mem: {
      total: ui.key<number>("mem.total", "Installed memory, GB.", 16),
      apps: ui.key<number>("mem.apps", "Memory used by applications, GB.", 5.2),
      cache: ui.key<number>("mem.cache", "Reclaimable cache, GB.", 6.1),
      free: ui.key<number>("mem.free", "Free memory, GB.", 4.7),
      pct: ui.key<number>("mem.pct", "Memory in use, percent.", 70),
      swapUsed: ui.key<number>("swap.used", "Swap in use, GB.", 1),
      swapTotal: ui.key<number>("swap.total", "Swap size, GB.", 2),
    },
    disk: {
      used: ui.key<number>("disk.used", "Disk used, TB.", 0.82),
      total: ui.key<number>("disk.total", "Disk size, TB.", 0.9),
      pct: ui.key<number>("disk.pct", "Disk used, percent.", 91),
      hist: ui.key<number[]>("disk.hist", "Daily disk use, TB, oldest first.", [0.78, 0.79, 0.8, 0.82]),
    },
    proc: {
      top: ui.key<Proc[]>("proc.top", "The busiest processes.", [{ name: "WindowServer", cpu: 52 }, { name: "Safari", cpu: 14 }, { name: "claude", cpu: 8 }]),
      count: ui.key<number>("proc.count", "Number of processes.", 799),
    },
    net: {
      iface: ui.key<string | null>("net.iface", "The default network interface.", "en0"),
      ip: ui.key<string | null>("net.ip", "Local IP address.", "192.168.1.20"),
      down: ui.key<number>("net.down", "Download throughput, MB/s.", 1.4),
      up: ui.key<number>("net.up", "Upload throughput, MB/s.", 0.2),
      downHist: ui.key<number[]>("net.down_hist", "Recent download, MB/s, oldest first.", wave(44, 1, 0.8)),
      upHist: ui.key<number[]>("net.up_hist", "Recent upload, MB/s, oldest first.", wave(44, 0.2, 0.15)),
    },
    wifi: {
      rssi: ui.key<number | null>("wifi.rssi", "Signal strength, dBm.", -48),
      bars: ui.key<number>("wifi.bars", "Signal bars, 0..4.", 4),
      ssid: ui.key<string | null>("wifi.ssid", "Network name, when the system lets scripts read it.", "home"),
    },
    wx: {
      temp: ui.key<number>("wx.temp", "Air temperature, °C.", 16),
      feels: ui.key<number>("wx.feels", "Apparent temperature, °C.", 15),
      sky: ui.key<Sky>("wx.sky", "The sky now.", Sky.Partly),
      wind: ui.key<number>("wx.wind", "Wind speed, mph.", 12),
      windDir: ui.key<number>("wx.wind_dir", "Where the wind comes from, degrees (0 = north).", 215),
      cloud: ui.key<number>("wx.cloud", "Cloud cover in oktas, 0..8.", 6),
      pressure: ui.key<number>("wx.pressure", "Sea-level pressure, hPa.", 1006),
      hours: ui.key<Hour[]>("wx.hours", "The next 24 hours of forecast, starting with the current hour.", hours),
      uvDay: ui.key<number[]>("wx.uv_day", "Today's UV index for each hour from 00:00 (24 values).", uvDay),
      uvNow: ui.key<number>("wx.uv", "UV index now.", 2.4),
      uvMax: ui.key<number>("wx.uv_max", "Today's peak UV index.", 5),
      rainFrom: ui.key<Ms | null>("wx.rain_from", "When rain is next likely (50% or more), or null for none in 24 h.", hourStart + 6 * H),
    },
    sun: {
      rise: ui.key<Ms>("sun.rise", "Sunrise today.", midnight + 7.7 * H),
      set: ui.key<Ms>("sun.set", "Sunset today.", midnight + 16.8 * H),
    },
    moon: { phase: ui.key<number>("moon.phase", "Fraction of the lunar month since new moon: 0 new, 0.5 full.", 0.46) },
    air: {
      eaqi: ui.key<number>("aq.eaqi", "European air-quality index (0 best).", 28),
      pm25: ui.key<number>("aq.pm25", "PM2.5, µg/m³.", 4.7),
      pm10: ui.key<number>("aq.pm10", "PM10, µg/m³.", 9.4),
      no2: ui.key<number>("aq.no2", "Nitrogen dioxide, µg/m³.", 16),
      o3: ui.key<number>("aq.o3", "Ozone, µg/m³.", 48),
      main: ui.key<Pollutant>("aq.main", "The pollutant furthest into its band.", Pollutant.O3),
    },
    carbon: {
      slots: ui.key<Intensity[]>("carbon.slots", "Forecast carbon intensity for the next 24 hours, half-hourly.", intensity),
      now: ui.key<number>("carbon.now", "Carbon intensity now, gCO₂/kWh.", 73),
      index: ui.key<CarbonIndex>("carbon.index", "The grid's own rating of now.", CarbonIndex.Low),
      green: ui.key<number>("carbon.green", "The line under which energy counts as green, gCO₂/kWh.", 100),
      region: ui.key<string>("carbon.region", "The grid region the forecast is for.", "East England"),
      best: ui.key<Span | null>("carbon.best", "The cleanest two hours from now.", { from: now + 2 * H, to: now + 4 * H, avg: 32 }),
      mix: ui.key<MixShare[]>("carbon.mix", "Generation by fuel now.", [{ fuel: Fuel.Wind, pct: 43 }, { fuel: Fuel.Nuclear, pct: 23 }, { fuel: Fuel.Solar, pct: 18 }, { fuel: Fuel.Gas, pct: 4 }]),
    },
    power: {
      tariff: ui.key<string>("power.tariff", "The tariff's name.", "Flexible Octopus"),
      kind: ui.key<RateKind>("power.kind", "Whether the rate is flat or half-hourly.", RateKind.Flat),
      rate: ui.key<number>("power.rate", "Unit rate now, p/kWh inc. VAT.", 26.6),
      standing: ui.key<number>("power.standing", "Standing charge, p/day.", 50.5),
      slots: ui.key<Rate[]>("power.slots", "Today's half-hourly rates (empty for a flat tariff).", []),
      nextSlots: ui.key<Rate[]>("power.next_slots", "Tomorrow's half-hourly rates, once published.", []),
      min: ui.key<Peak | null>("power.min", "Today's lowest rate (half-hourly tariffs).", null),
      max: ui.key<Peak | null>("power.max", "Today's highest rate (half-hourly tariffs).", null),
      avg: ui.key<number | null>("power.avg", "Today's average rate (half-hourly tariffs).", null),
      cheap: ui.key<Span | null>("power.cheap", "The cheapest two hours from now (half-hourly tariffs).", null),
      usageDay: ui.key<Ms | null>("power.usage_day", "The newest local day with a full set of meter readings.", midnight - 24 * H),
      usageSlots: ui.key<number[]>("power.usage_slots", "That day's usage per half hour, kWh.", wave(48, 0.2, 0.15)),
      usageKwh: ui.key<number>("power.usage_kwh", "That day's total, kWh.", 9.6),
      usageCost: ui.key<number>("power.usage_cost", "That day's cost in pounds, standing charge included.", 3.06),
      week: ui.key<DayUsage[]>("power.week", "Daily usage over the last week.", [{ day: midnight - 48 * H, kwh: 8.1 }, { day: midnight - 24 * H, kwh: 9.6 }]),
      latest: ui.key<Ms | null>("power.usage_latest", "When the newest meter reading ends.", now - 12 * H),
      updated: ui.key<Ms>("power.updated", "When the account was last read.", now),
    },
    cal: {
      events: ui.key<CalEvent[]>("cal.events", "Events from today through the next seven days, by start.", events),
    },
    shop: { items: ui.key<Grocery[]>("shop.items", "The shopping list.", [{ text: "Milk", qty: "2", done: false }, { text: "Eggs", qty: "12", done: false }, { text: "Butter", qty: "", done: true }]) },
    notes: { items: ui.key<Note[]>("notes.items", "Notes to keep in mind.", [{ text: "Example note", due: "Fri" }]) },
    claude: {
      state: ui.key<Presence>("claude.state", "What the assistant is doing.", Presence.Idle),
      heard: ui.key<string>("claude.heard", "The last thing it heard.", "How long do I boil an egg?"),
      say: ui.key<string>("claude.say", "The last thing it said.", "About six and a half minutes for a jammy yolk."),
      at: ui.key<Ms>("claude.at", "When it last spoke.", now),
    },
    timer: {
      end: ui.key<Ms | null>("timer.end", "When the running timer rings, or null.", now + 6 * 60_000),
      total: ui.key<number>("timer.total", "The running timer's full length, seconds.", 390),
      label: ui.key<string>("timer.label", "What the timer is for.", "egg"),
    },
  };

  // ---------------------------------------------------------------- typed functions: classification

  /** The UV band for an index value. */
  const uvLevel = (uv: number): UvLevel => (uv < 3 ? UvLevel.Low : uv < 6 ? UvLevel.Moderate : uv < 8 ? UvLevel.High : uv < 11 ? UvLevel.VeryHigh : UvLevel.Extreme);
  /** The European air-quality band for an index value. */
  const airBand = (eaqi: number): AirBand => (eaqi < 20 ? AirBand.Good : eaqi < 40 ? AirBand.Fair : eaqi < 60 ? AirBand.Moderate : eaqi < 80 ? AirBand.Poor : eaqi < 100 ? AirBand.VeryPoor : AirBand.ExtremelyPoor);
  /** The named moon phase for a lunar fraction (0 new, 0.5 full). */
  const moonPhaseOf = (fraction: number): MoonPhase => (Math.floor((fraction % 1) * 8 + 0.5) % 8) as MoonPhase;
  /** Whether a carbon intensity counts as green, against a line (default 100 gCO₂/kWh). */
  const isGreen = (grams: number, line = 100): boolean => grams <= line;

  // ---------------------------------------------------------------- typed functions: labels (enum → words)

  /** Words for the enums. */
  const label = {
    sky: (s: Sky): string => ["Clear", "Partly cloudy", "Cloudy", "Rain", "Snow", "Thunderstorm", "Fog"][s],
    uv: (l: UvLevel): string => ["low", "moderate", "high", "very high", "extreme"][l],
    air: (b: AirBand): string => ["good", "fair", "moderate", "poor", "very poor", "extremely poor"][b],
    carbon: (c: CarbonIndex): string => ["very low", "low", "moderate", "high", "very high"][c],
    pollutant: (p: Pollutant): string => ["PM2.5", "PM10", "NO₂", "O₃"][p],
    fuel: (f: Fuel): string => ["biomass", "coal", "imports", "gas", "nuclear", "other", "hydro", "solar", "wind"][f],
    presence: (p: Presence): string => ["idle", "listening", "thinking", "speaking"][p],
    moon: (m: MoonPhase): string => ["New moon", "Waxing crescent", "First quarter", "Waxing gibbous", "Full moon", "Waning gibbous", "Last quarter", "Waning crescent"][m],
  };
  /** The weather symbol the kit draws for a sky: pass it to weatherIcon(). */
  const skyIcon = (s: Sky): "clear" | "partly" | "cloudy" | "rain" | "snow" | "storm" | "fog" => (["clear", "partly", "cloudy", "rain", "snow", "storm", "fog"] as const)[s];
  /** Sigil states for the bands: ok for good ones. */
  const stateOf = {
    carbon: (c: CarbonIndex) => (c <= CarbonIndex.Low ? State.Ok : c === CarbonIndex.Moderate ? State.Warn : State.Err),
    air: (b: AirBand) => (b <= AirBand.Fair ? State.Ok : b === AirBand.Moderate ? State.Warn : State.Err),
    presence: (p: Presence) => (p === Presence.Idle ? State.Idle : p === Presence.Listening ? State.Ok : State.Warn),
  };

  // ---------------------------------------------------------------- typed functions: formatting

  const two = (n: number) => String(n).padStart(2, "0");
  /** Formatting: numbers, durations and instants as text. Every function takes typed values, never text. */
  const fmt = {
    /** A number with fixed decimals, or a dash for a missing value. */
    num: (v: number | null | undefined, digits = 0): string => (v == null || Number.isNaN(v) ? "–" : v.toFixed(digits)),
    /** Minutes as "45m" or "1h30". */
    duration: (minutes: number): string => (minutes < 60 ? `${Math.round(minutes)}m` : `${Math.floor(minutes / 60)}h${two(Math.round(minutes % 60))}`),
    /** Seconds as "13d 4h" or "5h 12m". */
    uptime: (seconds: number | null | undefined): string => (seconds == null ? "–" : seconds >= 86400 ? `${Math.floor(seconds / 86400)}d ${Math.floor((seconds % 86400) / 3600)}h` : `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`),
    /** An instant as the local 24-hour clock, "09:30". */
    clock: (at: Ms | null | undefined): string => { if (at == null) return "–"; const d = new Date(at); return `${two(d.getHours())}:${two(d.getMinutes())}`; },
    /** An instant's local weekday, "Tue". */
    weekday: (at: Ms): string => new Date(at).toLocaleDateString("en-GB", { weekday: "short" }),
    /** An instant's local date, "Tue 27 Oct". */
    date: (at: Ms): string => new Date(at).toLocaleDateString("en-GB", { weekday: "short", day: "numeric", month: "short" }),
    /** Degrees as a compass point, "SW". */
    compass: (degrees: number): string => ["N", "NE", "E", "SE", "S", "SW", "W", "NW"][Math.round(degrees / 45) % 8],
    /** A percentage rounded, with its sign. */
    percent: (v: number | null | undefined): string => (v == null ? "–" : `${Math.round(v)}%`),
    /** The hour of an instant as two digits, "07". */
    hour: (at: Ms): string => two(new Date(at).getHours()),
  };

  // ---------------------------------------------------------------- typed functions: series

  /** Helpers for number series. */
  const series = {
    /** The last n values, padded with zeros on the left. */
    last: (values: readonly number[] | null | undefined, n: number): number[] => { const t = (values ?? []).slice(-n); return Array(Math.max(0, n - t.length)).fill(0).concat(t); },
    /** Scale to 0..1 against `top`, square-rooted so small values stay visible. */
    sqrtScale: (values: readonly number[], top: number): number[] => values.map((x) => Math.sqrt(Math.min(1, Math.max(0, x / top)))),
  };

  // ---------------------------------------------------------------- typed functions: the calendar

  /** The start of the local day containing `at`. */
  const dayStart = (at: Ms): Ms => new Date(at).setHours(0, 0, 0, 0);
  /** Calendar queries over `K.cal.events`. */
  const calendar = {
    /** Events that start on the local day of `at`. */
    on: (events: readonly CalEvent[], at: Ms): CalEvent[] => events.filter((e) => dayStart(e.start) === dayStart(at)),
    /** Whether an event is over at `at`. */
    over: (e: CalEvent, at: Ms): boolean => e.end <= at,
    /**
     * Pack events into lanes so that none overlap within a lane (the first lane takes what it can, the next takes what is left): the rows
     * of a route diagram. Events that only touch (one ends as the next starts) share a lane.
     */
    lanes: (events: readonly CalEvent[]): CalEvent[][] => {
      const lanes: CalEvent[][] = [];
      for (const e of [...events].sort((a, b) => a.start - b.start || a.end - b.end)) {
        const lane = lanes.find((l) => l[l.length - 1].end <= e.start);
        if (lane) lane.push(e); else lanes.push([e]);
      }
      return lanes;
    },
    /** An event's label in a list: "09:30" today, "Tue 09:30" on another day, "all day" for an all-day event. */
    when: (e: CalEvent, at: Ms): string => `${dayStart(e.start) === dayStart(at) ? "" : fmt.weekday(e.start) + " "}${e.allDay ? "all day" : fmt.clock(e.start)}`,
    /** The next `n` events that are not over, from `at`. */
    upcoming: (events: readonly CalEvent[], at: Ms, n: number): CalEvent[] => events.filter((e) => !calendar.over(e, at)).slice(0, n),
    /** The first timed event that has not started, and minutes until it. */
    next: (events: readonly CalEvent[], at: Ms): { event: CalEvent; inMinutes: number } | undefined => {
      const e = events.find((x) => !x.allDay && x.start >= at);
      return e && { event: e, inMinutes: Math.round((e.start - at) / 60000) };
    },
    /** The day to draw as "the day": today while something timed is still to come, else the next day with something. */
    focus: (events: readonly CalEvent[], at: Ms): { day: Ms; isToday: boolean; events: CalEvent[] } => {
      const today = dayStart(at), timed = events.filter((e) => !e.allDay);
      if (timed.some((e) => dayStart(e.start) === today && !calendar.over(e, at))) return { day: today, isToday: true, events: calendar.on(events, today) };
      const later = timed.find((e) => dayStart(e.start) > today);
      return later ? { day: dayStart(later.start), isToday: false, events: calendar.on(events, later.start) } : { day: today, isToday: true, events: calendar.on(events, today) };
    },
  };

  // ---------------------------------------------------------------- the display's chrome

  /** Set up the frame: registration marks, live sigils in the corners (left = the machine, right = the world) and the tab bar. */
  const shell = (): void => {
    const { sig } = window as any;
    ui.frame({
      tabs: true,
      clock: true,
      tl: () => [
        sig.state(ui.get(K.link.server) === Link.Up ? State.Ok : State.Err),
        sig.bars(ui.get(K.wifi.bars) ?? 0),
        sig.level(ui.get(K.cpu.total) ?? 0, 100),
        sig.level(ui.get(K.disk.pct) ?? 0, 100),
      ],
      tr: () => [
        sig.dir(ui.get(K.wx.windDir) ?? 0),
        sig.level(ui.get(K.wx.cloud) ?? 0, 8),
        sig.glyph("moon"),
        sig.count(calendar.on(ui.get(K.cal.events) ?? [], Date.now()).filter((e: CalEvent) => !calendar.over(e, Date.now())).length),
      ],
    });
  };

  Object.assign(window, { K, Sky, UvLevel, AirBand, CarbonIndex, Pollutant, Fuel, Presence, RateKind, MoonPhase, uvLevel, airBand, moonPhaseOf, isGreen, label, skyIcon, stateOf, fmt, series, dayStart, calendar });
  ui.shell = shell;
})();
