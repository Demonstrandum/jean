//! A headless Chrome driven over the DevTools protocol: it holds the display page open and hands back screenshots.

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use base64::Engine;
use serde_json::{json, Value};
use tungstenite::{connect, stream::MaybeTlsStream, WebSocket};

pub struct Browser {
    child: Child,
    socket: WebSocket<MaybeTlsStream<std::net::TcpStream>>,
    next: u64,
    profile: PathBuf,
}

/// Where Chrome or Chromium usually is.
pub fn find_chrome() -> Option<PathBuf> {
    ["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "/Applications/Chromium.app/Contents/MacOS/Chromium", "/usr/bin/google-chrome", "/usr/bin/chromium", "/usr/bin/chromium-browser", "/run/current-system/sw/bin/chromium"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
}

impl Browser {
    /// Start Chrome on `url` with a scene of `scene` CSS pixels rendered at `scale` device pixels per CSS pixel.
    pub fn open(chrome: &std::path::Path, url: &str, scene: (usize, usize), scale: f64) -> Result<Browser, String> {
        let port = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?.local_addr().map_err(|e| e.to_string())?.port();
        let profile = std::env::temp_dir().join(format!("eink-chrome-{}-{port}", std::process::id()));
        let child = Command::new(chrome)
            .args(["--headless=new", "--disable-gpu", "--no-sandbox", "--hide-scrollbars", "--no-first-run"])
            .arg(format!("--remote-debugging-port={port}"))
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg(format!("--window-size={},{}", scene.0, scene.1))
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", chrome.display()))?;
        let mut ws_url = None;
        for _ in 0..60 {
            if let Ok(r) = ureq::get(&format!("http://127.0.0.1:{port}/json")).call() {
                if let Ok(v) = r.into_json::<Value>() {
                    ws_url = v.as_array().and_then(|a| a.iter().find(|t| t["type"] == "page")).and_then(|t| t["webSocketDebuggerUrl"].as_str()).map(String::from);
                    if ws_url.is_some() {
                        break;
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        let ws_url = ws_url.ok_or("Chrome did not open a page")?;
        let (socket, _) = connect(ws_url).map_err(|e| e.to_string())?;
        let mut b = Browser { child, socket, next: 0, profile };
        b.call("Emulation.setDeviceMetricsOverride", json!({ "width": scene.0, "height": scene.1, "deviceScaleFactor": scale, "mobile": false }))?;
        Ok(b)
    }

    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.next += 1;
        let id = self.next;
        self.socket.send(tungstenite::Message::text(json!({ "id": id, "method": method, "params": params }).to_string())).map_err(|e| e.to_string())?;
        loop {
            let msg = self.socket.read().map_err(|e| e.to_string())?;
            let tungstenite::Message::Text(t) = msg else { continue };
            let v: Value = serde_json::from_str(&t).map_err(|e| e.to_string())?;
            if v["id"] == id {
                return match v.get("error") {
                    Some(e) => Err(format!("{method}: {e}")),
                    None => Ok(v["result"].clone()),
                };
            }
        }
    }

    /// Evaluate a JavaScript expression in the page and return its value (JSON).
    pub fn evaluate(&mut self, expression: &str) -> Result<Value, String> {
        let r = self.call("Runtime.evaluate", json!({ "expression": expression, "returnByValue": true }))?;
        if let Some(e) = r.get("exceptionDetails") {
            return Err(format!("in the page: {}", e["text"].as_str().unwrap_or("exception")));
        }
        Ok(r["result"]["value"].clone())
    }

    /// The page as a PNG, at the device scale set on opening.
    pub fn screenshot(&mut self) -> Result<Vec<u8>, String> {
        let r = self.call("Page.captureScreenshot", json!({ "format": "png" }))?;
        base64::engine::general_purpose::STANDARD.decode(r["data"].as_str().ok_or("no screenshot data")?).map_err(|e| e.to_string())
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.profile);
    }
}
