//! Show the display on a panel over VNC: hold the display page open in headless Chrome, and about every second screenshot it, dither it,
//! and publish it when it has changed.
//! Behind AppLoad as one big framebuffer the reMarkable re-drives its whole window on every update, however small, so a tick flashes everything: that is
//! why the tablet runs the tile app (crates/eink/device/tiles), where a tick dirties one small item. For a viewer that cannot be local, `--calm-ms N` holds
//! small changes to the clock-aligned slot of N ms and `--no-seconds` drops the seconds glyph; a touch or any big change always goes out at once.
//!   eink-serve [--calm-ms 0] [--no-seconds] [--display http://127.0.0.1:8788] [--listen 0.0.0.0:5900] [--panel native|rm2|rm2-portrait|rm2-turned-cw|rm2-turned-ccw] [--every-ms 1000] [--chrome PATH] [--dump out.png]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use std::time::Duration;

use eink::cdp::{find_chrome, Browser};
use eink::rfb::{serve, Gesture, OnGesture, Screen};
use eink::{capture, Frame, Panel};

/// Pixels changed below which a frame counts as a tick (the seconds, a percentage), not a new screen.
const SMALL_CHANGE: usize = 12_000;

fn main() {
    let mut a = std::env::args().skip(1);
    let mut dump: Option<String> = None;
    let mut calm_ms: u64 = 0;      // with a number, small changes wait for the next slot of this length on the wall clock; 0 (the default) sends everything as it comes
    let mut seconds = true;        // the seconds glyph ticks every second (local on the tablet, with the tile app)
    let (mut display, mut listen, mut panel, mut every, mut chrome) = ("http://127.0.0.1:8788".to_string(), "0.0.0.0:5900".to_string(), Panel::Remarkable2Landscape, 1000u64, find_chrome());
    while let Some(flag) = a.next() {
        let mut val = || a.next().unwrap_or_else(|| panic!("{flag} needs a value"));
        match flag.as_str() {
            "--display" => display = val(),
            "--listen" => listen = val(),
            "--every-ms" => every = val().parse().expect("a number"),
            "--dump" => dump = Some(val()),
            "--calm-ms" => calm_ms = val().parse().expect("a number"),
            "--no-seconds" => seconds = false,
            "--chrome" => chrome = Some(val().into()),
            "--panel" => {
                panel = match val().as_str() {
                    "native" => Panel::Native,
                    "rm2" => Panel::Remarkable2Landscape,
                    "rm2-portrait" => Panel::Remarkable2Portrait,
                    "rm2-turned-cw" => Panel::Remarkable2TurnedCw,
                    "rm2-turned-ccw" => Panel::Remarkable2TurnedCcw,
                    other => panic!("unknown panel {other}"),
                }
            }
            other => panic!("unknown option {other}"),
        }
    }
    let chrome = chrome.expect("no Chrome found; pass --chrome");
    let browser = Arc::new(Mutex::new(Browser::open(&chrome, &format!("{}{}", panel.page(&display), if seconds { "" } else { "&seconds=off" }), panel.scene(), panel.scale()).expect("cannot open the display page")));
    std::thread::sleep(Duration::from_secs(2));
    let first = capture(&mut browser.lock().unwrap(), panel).expect("first screenshot");
    if let Some(path) = &dump {
        eink::write_png(path, &first).expect("cannot write the dump");
    }
    let screen: Arc<Screen> = Screen::new(first);
    // a touch on the panel: find its place in the scene, ask the page what it should show, show it in the page at once (no waiting for the poll or the
    // next beat), and send the new frame straight away. The page itself is display-only; this is the relay.
    let last_slot = Arc::new(AtomicU64::new(0));
    let slot_of = move |ms: u64| if calm_ms == 0 { ms } else { ms / calm_ms };
    let now_ms = || SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
    let show = {
        let (browser, screen, last_slot) = (browser.clone(), screen.clone(), last_slot.clone());
        move |what: &str, query: String| {
            let started = std::time::Instant::now();
            if std::env::var_os("EINK_DEBUG").is_some() {
                eprintln!("eink-serve: gesture arrived at {} ms", eink::rfb::clock_ms());
            }
            let mut b = browser.lock().unwrap();
            match b.evaluate(&query) {
                Ok(serde_json::Value::String(name)) => {
                    eprintln!("eink-serve: {what} shows {name}");
                    if let Err(e) = b.evaluate(&format!("ui.go({})", serde_json::to_string(&name).unwrap())) {
                        eprintln!("eink-serve: cannot show {name}: {e}");
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(60)); // let the page settle its layout and fonts
                    match capture(&mut b, panel) {
                        Ok(f) => {
                            screen.show(f);
                            last_slot.store(slot_of(now_ms()), Ordering::Relaxed);
                            eprintln!("eink-serve: shown at {} ms", eink::rfb::clock_ms());
                            eprintln!("eink-serve: {what}: frame ready {} ms after the gesture", started.elapsed().as_millis());
                        }
                        Err(e) => eprintln!("eink-serve: {e}"),
                    }
                }
                Ok(_) => eprintln!("eink-serve: {what} hits nothing"),
                Err(e) => eprintln!("eink-serve: {e}"),
            }
        }
    };
    let on_gesture: OnGesture = Arc::new(move |g| match g {
        Gesture::Tap { x, y } => {
            if let Some((sx, sy)) = panel.to_scene(x, y) {
                show(&format!("tap at {sx:.0},{sy:.0}"), format!("ui.tap({sx:.1}, {sy:.1})"));
            }
        }
        Gesture::Swipe { from, to } => {
            // a swipe across the screen (in the scene's own orientation) turns the page: left for the next screen, right for the previous
            if let (Some((x0, y0)), Some((x1, y1))) = (panel.to_scene(from.0, from.1), panel.to_scene(to.0, to.1)) {
                let (dx, dy) = (x1 - x0, y1 - y0);
                if dx.abs() > dy.abs() {
                    show(if dx < 0.0 { "swipe left" } else { "swipe right" }, format!("ui.step({})", if dx < 0.0 { 1 } else { -1 }));
                }
            }
        }
    });
    let addr = serve(&listen, screen.clone(), on_gesture).expect("cannot listen");
    eprintln!("eink-serve: {}x{} on vnc://{addr}", screen.size().0, screen.size().1);
    loop {
        std::thread::sleep(Duration::from_millis(every));
        match capture(&mut browser.lock().unwrap(), panel) {
            Ok(f) => {
                let (rects, area) = screen.damage_to(&f);
                if rects == 0 {
                    continue;
                }
                let slot = slot_of(now_ms());
                // a big change (a new screen) goes out now; a tick-sized one waits for the next slot of the wall clock
                let send = area > SMALL_CHANGE || slot != last_slot.load(Ordering::Relaxed);
                if std::env::var_os("EINK_DEBUG").is_some() {
                    eprintln!("eink-serve: {rects} rects, {area} px changed, {}", if send { "sent" } else { "held" });
                }
                if send {
                    screen.show(f);
                    last_slot.store(slot, Ordering::Relaxed);
                }
            }
            Err(e) => eprintln!("eink-serve: {e}"),
        }
    }
    #[allow(unreachable_code)]
    {
        let _: Frame;
    }
}
