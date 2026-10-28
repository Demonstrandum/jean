//! A client for the display server (crates/preview): evals and queries into the page, live data, screens, the docs, the self-test.

use serde_json::{json, Value};

pub struct Display {
    pub base: String,
}

fn client_err(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, r) => {
            let body = r.into_string().unwrap_or_default();
            // the server's own error text is the useful part
            serde_json::from_str::<Value>(&body).ok().map(|v| v.to_string()).unwrap_or(format!("HTTP {code}: {body}"))
        }
        other => other.to_string(),
    }
}

impl Display {
    pub fn new(base: impl Into<String>) -> Display {
        Display { base: base.into().trim_end_matches('/').to_string() }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    fn post(&self, path: &str, body: &str) -> Result<Value, String> {
        ureq::post(&self.url(path)).timeout(std::time::Duration::from_secs(60)).send_string(body).map_err(client_err)?.into_json().map_err(|e| e.to_string())
    }

    fn get(&self, path: &str) -> Result<Value, String> {
        ureq::get(&self.url(path)).timeout(std::time::Duration::from_secs(30)).call().map_err(client_err)?.into_json().map_err(|e| e.to_string())
    }

    /// A journaled, verified, committed change to the page. `intent` becomes the commit subject.
    pub fn eval(&self, code: &str, intent: &str) -> Result<Value, String> {
        let enc: String = intent.bytes().map(|b| if b.is_ascii_alphanumeric() { (b as char).to_string() } else { format!("%{b:02X}") }).collect();
        self.post(&format!("/eval?intent={enc}"), code)
    }

    /// Undo the last journaled change.
    pub fn undo(&self) -> Result<Value, String> {
        self.post("/undo", "")
    }

    /// A read-only evaluation in the page (rolled back, never journaled).
    pub fn query(&self, code: &str) -> Result<Value, String> {
        self.post("/query", code)
    }

    /// Write live data: `{"claude.say": "..."}` (wire names; the keys are typed on the page side).
    pub fn push(&self, values: &Value) -> Result<Value, String> {
        self.post("/push", &values.to_string())
    }

    /// Switch screens; `hold_ms` None keeps the server's default.
    pub fn screen(&self, name: &str, hold_ms: Option<u64>) -> Result<Value, String> {
        let hold = hold_ms.map(|h| format!("&hold={h}")).unwrap_or_default();
        self.post(&format!("/screen?name={name}{hold}"), "")
    }

    /// The reference: items matching `query`, in full.
    pub fn docs(&self, query: &str) -> Result<Value, String> {
        let enc: String = query.bytes().map(|b| if b.is_ascii_alphanumeric() || b == b'.' { (b as char).to_string() } else { format!("%{b:02X}") }).collect();
        self.get(&format!("/docs?q={enc}"))
    }

    /// Run a named command from the server's commands/ directory.
    pub fn run(&self, cmd: &str, args: &[String]) -> Result<Value, String> {
        let args: String = args.iter().map(|a| format!("&arg={}", a.bytes().map(|b| if b.is_ascii_alphanumeric() { (b as char).to_string() } else { format!("%{b:02X}") }).collect::<String>())).collect();
        self.post(&format!("/run?cmd={cmd}{args}"), "")
    }

    /// Every live value the display has (wire name → {value, ts}).
    pub fn data(&self) -> Result<Value, String> {
        self.get("/data")
    }

    /// Show every screen with sample data and report how each fits.
    pub fn selftest(&self) -> Result<Value, String> {
        self.get("/selftest")
    }

    pub fn status(&self) -> Result<Value, String> {
        self.get("/status")
    }

    /// Whether the server answers, and a display page is connected to it.
    pub fn healthy(&self) -> bool {
        self.status().map(|s| s["connected"] == true).unwrap_or(false)
    }

    /// The live steps of what the assistant is doing (`claude.steps`).
    pub fn show_steps(&self, a: &crate::activity::Activity) -> Result<Value, String> {
        self.push(&json!({ "claude.steps": a.steps() }))
    }

    /// The assistant is on a background job (the builder): the Claude tile turns black with the job's title; `None` ends it.
    pub fn show_work(&self, task: Option<&str>, at_ms: u64) -> Result<Value, String> {
        self.push(&json!({ "claude.state": if task.is_some() { 4 } else { 0 }, "claude.task": task.unwrap_or(""), "claude.at": at_ms }))
    }

    /// The harness's face on the CLAUDE screen: what it heard, what it said, what it is doing.
    pub fn show_exchange(&self, heard: Option<&str>, said: Option<&str>, presence: u8, at_ms: u64) -> Result<Value, String> {
        let mut v = json!({ "claude.state": presence, "claude.at": at_ms });
        if let Some(h) = heard {
            v["claude.heard"] = json!(h);
        }
        if let Some(s) = said {
            v["claude.say"] = json!(s);
        }
        self.push(&v)
    }
}

#[cfg(test)]
pub mod testing {
    use std::sync::{Arc, Mutex};

    /// A stand-in display server that records every request as `(method, url, body)` and answers `{"ok":true}`.
    pub struct Mock {
        pub base: String,
        pub seen: Arc<Mutex<Vec<(String, String, String)>>>,
    }

    pub fn mock() -> Mock {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let port = server.server_addr().to_ip().unwrap().port();
        let seen: Arc<Mutex<Vec<(String, String, String)>>> = Arc::default();
        let s2 = seen.clone();
        std::thread::spawn(move || {
            for mut req in server.incoming_requests() {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                s2.lock().unwrap().push((req.method().to_string(), req.url().to_string(), body));
                let reply = if req.url().starts_with("/docs") { r#"[{"name":"tile","signature":"tile(...)"}]"# } else { r#"{"ok":true}"# };
                let _ = req.respond(tiny_http::Response::from_string(reply).with_header(tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap()));
            }
        });
        Mock { base: format!("http://127.0.0.1:{port}"), seen }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_calls_go_to_the_right_routes() {
        let m = testing::mock();
        let d = Display::new(&m.base);
        d.eval("ui.go('home')", "show home").unwrap();
        d.screen("energy", Some(600000)).unwrap();
        d.push(&json!({ "claude.say": "hi" })).unwrap();
        assert_eq!(d.docs("tile").unwrap()[0]["name"], "tile");
        d.run("shop", &["add".into(), "oat milk".into()]).unwrap();
        let seen = m.seen.lock().unwrap();
        assert_eq!(seen[0], ("POST".into(), "/eval?intent=show%20home".into(), "ui.go('home')".into()));
        assert_eq!(seen[1].1, "/screen?name=energy&hold=600000");
        assert_eq!(seen[2].2, r#"{"claude.say":"hi"}"#);
        assert_eq!(seen[3].1, "/docs?q=tile");
        assert_eq!(seen[4].1, "/run?cmd=shop&arg=add&arg=oat%20milk");
    }
}
