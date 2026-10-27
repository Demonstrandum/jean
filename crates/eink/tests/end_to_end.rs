//! Chrome to dither to VNC, against a real browser and a real display server. Skipped where there is no Chrome.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use eink::cdp::{find_chrome, Browser};
use eink::rfb::{serve, Screen};
use eink::{capture, Panel};

struct Kill(Child);
impl Drop for Kill {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn the_real_display_reaches_a_vnc_viewer_dithered_at_the_remarkables_size() {
    let Some(chrome) = find_chrome() else {
        eprintln!("no Chrome here; skipping");
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    assert!(Command::new("cargo").args(["build", "-q", "-p", "preview"]).current_dir(&root).status().unwrap().success());
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let state = std::env::temp_dir().join(format!("eink-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&state);
    let _server = Kill(Command::new(root.join("target/debug/preview")).arg(format!("127.0.0.1:{port}")).arg(&state).arg(&root).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap());
    let base = format!("http://127.0.0.1:{port}");
    for _ in 0..40 {
        if ureq::get(&format!("{base}/status")).call().is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    let panel = Panel::Remarkable2Landscape;
    let mut browser = Browser::open(&chrome, &panel.page(&base), panel.scene(), panel.scale()).unwrap();
    // wait for the seeds to be replayed into the page, then capture
    for _ in 0..60 {
        if ureq::get(&format!("{base}/status")).call().ok().and_then(|r| r.into_json::<serde_json::Value>().ok()).is_some_and(|s| s["connected"] == true && s["replay"]["replayed"].as_u64().unwrap_or(0) >= 7) {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    std::thread::sleep(Duration::from_secs(2));
    let frame = capture(&mut browser, panel).unwrap();
    assert_eq!((frame.w, frame.h), (1872, 1404));
    assert!(frame.px.iter().all(|&p| p == 0 || p == 255), "only black and white");
    let black = frame.px.iter().filter(|&&p| p == 0).count() as f64 / frame.px.len() as f64;
    assert!((0.03..0.6).contains(&black), "the dense black-on-white scene should be some tens of percent black, not {black}");
    // the 4:3 scene fills the panel: the top rows are the sheet's frame, not a white margin
    assert!(frame.px[..1872 * 100].iter().any(|&p| p == 0), "the scene reaches the top edge");

    let screen: Arc<Screen> = Screen::new(frame.clone());
    let addr = serve("127.0.0.1:0", screen, std::sync::Arc::new(|_, _| {})).unwrap();
    let mut s = TcpStream::connect(addr).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    let mut buf = [0u8; 12];
    s.read_exact(&mut buf).unwrap();
    s.write_all(b"RFB 003.008\n").unwrap();
    let mut two = [0u8; 2];
    s.read_exact(&mut two).unwrap();
    s.write_all(&[1]).unwrap();
    let mut four = [0u8; 4];
    s.read_exact(&mut four).unwrap();
    s.write_all(&[1]).unwrap();
    let mut head = [0u8; 24];
    s.read_exact(&mut head).unwrap();
    assert_eq!((u16::from_be_bytes([head[0], head[1]]), u16::from_be_bytes([head[2], head[3]])), (1872, 1404));
    let mut name = [0u8; 4];
    s.read_exact(&mut name).unwrap();
    s.write_all(&[3, 0, 0, 0, 0, 0, 0x07, 0x50, 0x05, 0x7c]).unwrap();
    let mut upd = [0u8; 16];
    s.read_exact(&mut upd).unwrap();
    let mut pixels = vec![0u8; 1872 * 1404 * 4];
    s.read_exact(&mut pixels).unwrap();
    let viewer_black = pixels.chunks(4).filter(|p| p[0] == 0).count();
    assert_eq!(viewer_black, frame.px.iter().filter(|&&p| p == 0).count());
    if let Ok(dir) = std::env::var("EINK_DUMP") {
        std::fs::write(Path::new(&dir).join("panel.pgm"), [format!("P5\n{} {}\n255\n", frame.w, frame.h).into_bytes(), frame.px.clone()].concat()).unwrap();
    }
    let _ = std::fs::remove_dir_all(state);
}
