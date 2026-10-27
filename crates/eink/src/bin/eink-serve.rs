//! Show the display on a panel over VNC: hold the display page open in headless Chrome, and about every second screenshot it, dither it,
//! and publish it when it has changed.
//!   eink-serve [--display http://127.0.0.1:8788] [--listen 0.0.0.0:5900] [--panel native|rm2|rm2-portrait|rm2-turned-cw|rm2-turned-ccw] [--every-ms 1000] [--chrome PATH] [--dump out.png]

use std::sync::Arc;
use std::time::Duration;

use eink::cdp::{find_chrome, Browser};
use eink::rfb::{serve, Screen};
use eink::{capture, Frame, Panel};

fn main() {
    let mut a = std::env::args().skip(1);
    let (mut display, mut listen, mut panel, mut every, mut chrome) = ("http://127.0.0.1:8788".to_string(), "0.0.0.0:5900".to_string(), Panel::Remarkable2Landscape, 1000u64, find_chrome());
    while let Some(flag) = a.next() {
        let mut val = || a.next().unwrap_or_else(|| panic!("{flag} needs a value"));
        match flag.as_str() {
            "--display" => display = val(),
            "--listen" => listen = val(),
            "--every-ms" => every = val().parse().expect("a number"),
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
    let mut browser = Browser::open(&chrome, &panel.page(&display), panel.scene(), panel.scale()).expect("cannot open the display page");
    std::thread::sleep(Duration::from_secs(2));
    let first = capture(&mut browser, panel).expect("first screenshot");
    let screen: Arc<Screen> = Screen::new(first);
    let addr = serve(&listen, screen.clone()).expect("cannot listen");
    eprintln!("eink-serve: {}x{} on vnc://{addr}", screen.size().0, screen.size().1);
    loop {
        std::thread::sleep(Duration::from_millis(every));
        match capture(&mut browser, panel) {
            Ok(f) => screen.show(f),
            Err(e) => eprintln!("eink-serve: {e}"),
        }
    }
    #[allow(unreachable_code)]
    {
        let _: Frame;
    }
}
