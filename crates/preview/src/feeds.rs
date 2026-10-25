//! Feeds and commands: how the machine talks to the display.
//!
//! * `feeds/<name>.{sh,py}` (or any executable): started by the server, kept alive, restarted on exit (5 s back-off) and
//!   when the file changes. Whatever a feed prints to stdout becomes data: a JSON object `{"wifi.rssi": -61}`,
//!   `{"key": "wifi.rssi", "value": -61}`, or a line `wifi.rssi=-61`. This is the "custom loop": the agent writes
//!   the script, drops it in, and the values start flowing; if it breaks, fixing the script is the whole repair.
//! * `commands/<name>`: run on demand by the page (`ui.run("name", args…)`), 10 s limit, by name only (no arbitrary shell).

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use serde_json::{json, Value};

use crate::hub::Hub;

const RESTART_AFTER: Duration = Duration::from_secs(5);
const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

struct Feed {
    child: Option<Child>,
    mtime: SystemTime,
    next_start: Instant,
    restarts: u32,
    last_line: Arc<AtomicU64>,
}

pub struct Feeds {
    root: PathBuf,
    hub: Arc<Hub>,
    state: Mutex<HashMap<String, Feed>>,
}

/// Parse one line of feed output into `(key, value)` pairs.
pub fn parse_line(line: &str) -> Vec<(String, Value)> {
    let line = line.trim();
    if line.is_empty() {
        return vec![];
    }
    if line.starts_with('{') {
        return match serde_json::from_str::<Value>(line) {
            Ok(Value::Object(m)) => match (m.get("key").and_then(|k| k.as_str()), m.get("value")) {
                (Some(k), Some(v)) if m.len() <= 3 => vec![(k.to_string(), v.clone())],
                _ => m.into_iter().collect(),
            },
            _ => vec![],
        };
    }
    match line.split_once('=') {
        Some((k, v)) if !k.trim().is_empty() => {
            let v = v.trim();
            vec![(k.trim().to_string(), serde_json::from_str(v).unwrap_or_else(|_| Value::String(v.to_string())))]
        }
        _ => vec![],
    }
}

fn is_script(path: &Path) -> bool {
    path.is_file() && path.file_name().and_then(|n| n.to_str()).map_or(false, |n| !n.starts_with('.') && !n.starts_with('_'))
}

fn command_for(path: &Path) -> Command {
    if fs::metadata(path).map_or(false, |m| m.permissions().mode() & 0o111 != 0) {
        return Command::new(path);
    }
    let interp = match path.extension().and_then(|e| e.to_str()) {
        Some("py") => "python3",
        Some("js") => "node",
        _ => "sh",
    };
    let mut c = Command::new(interp);
    c.arg(path);
    c
}

impl Feeds {
    pub fn new(root: PathBuf, hub: Arc<Hub>) -> Arc<Feeds> {
        Arc::new(Feeds { root, hub, state: Mutex::new(HashMap::new()) })
    }

    /// Start the supervisor: rescans `feeds/` every second.
    pub fn supervise(self: &Arc<Self>) {
        let me = self.clone();
        thread::spawn(move || loop {
            me.scan();
            thread::sleep(Duration::from_secs(1));
        });
    }

    fn scan(&self) {
        let dir = self.root.join("feeds");
        let mut seen = Vec::new();
        if let Ok(rd) = fs::read_dir(&dir) {
            for e in rd.flatten() {
                let path = e.path();
                if !is_script(&path) {
                    continue;
                }
                let name = e.file_name().to_string_lossy().into_owned();
                let mtime = fs::metadata(&path).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
                seen.push(name.clone());
                let mut st = self.state.lock().unwrap();
                let feed = st.entry(name.clone()).or_insert_with(|| Feed {
                    child: None,
                    mtime,
                    next_start: Instant::now(),
                    restarts: 0,
                    last_line: Arc::new(AtomicU64::new(0)),
                });
                if feed.mtime != mtime {
                    // edited: restart it now with the new code
                    if let Some(c) = feed.child.as_mut() {
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                    feed.child = None;
                    feed.mtime = mtime;
                    feed.next_start = Instant::now();
                }
                if let Some(c) = feed.child.as_mut() {
                    if let Ok(Some(status)) = c.try_wait() {
                        eprintln!("feed {name}: exited ({status}); restarting in {}s", RESTART_AFTER.as_secs());
                        feed.child = None;
                        feed.next_start = Instant::now() + RESTART_AFTER;
                        feed.restarts += 1;
                    }
                }
                if feed.child.is_none() && Instant::now() >= feed.next_start {
                    match self.spawn(&name, &path, feed.last_line.clone()) {
                        Ok(child) => feed.child = Some(child),
                        Err(e) => {
                            eprintln!("feed {name}: cannot start: {e}");
                            feed.next_start = Instant::now() + RESTART_AFTER;
                        }
                    }
                }
            }
        }
        // removed files stop their feeds
        let mut st = self.state.lock().unwrap();
        st.retain(|name, feed| {
            let keep = seen.contains(name);
            if !keep {
                if let Some(c) = feed.child.as_mut() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
            }
            keep
        });
    }

    fn spawn(&self, name: &str, path: &Path, last_line: Arc<AtomicU64>) -> std::io::Result<Child> {
        let logs = self.root.join("feeds").join(".logs");
        fs::create_dir_all(&logs)?;
        let log = fs::OpenOptions::new().create(true).append(true).open(logs.join(format!("{name}.log")))?;
        let mut child = command_for(path)
            .current_dir(&self.root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(log))
            .spawn()?;
        let out = child.stdout.take().expect("piped");
        let hub = self.hub.clone();
        thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                for (k, v) in parse_line(&line) {
                    hub.set(&k, v, None);
                }
                last_line.store(crate::hub::now_ms(), Ordering::Relaxed);
            }
        });
        println!("feed {name}: started");
        Ok(child)
    }

    /// What is running, for `GET /feeds`.
    pub fn status(&self) -> Value {
        let st = self.state.lock().unwrap();
        Value::Array(
            st.iter()
                .map(|(name, f)| json!({ "name": name, "running": f.child.is_some(), "restarts": f.restarts, "last_line": f.last_line.load(Ordering::Relaxed) }))
                .collect(),
        )
    }

    /// Run `commands/<name>` with `args` (name must be `[A-Za-z0-9_-]+`; extension optional). Output is captured.
    pub fn run_command(&self, name: &str, args: &[String]) -> Value {
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return json!({ "ok": false, "error": "bad command name" });
        }
        let dir = self.root.join("commands");
        let path = fs::read_dir(&dir).ok().and_then(|rd| {
            rd.flatten().map(|e| e.path()).find(|p| is_script(p) && p.file_stem().and_then(|s| s.to_str()) == Some(name))
        });
        let Some(path) = path else {
            return json!({ "ok": false, "error": format!("no such command: {name}") });
        };
        let mut child = match command_for(&path).args(args).current_dir(&self.root).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
            Ok(c) => c,
            Err(e) => return json!({ "ok": false, "error": e.to_string() }),
        };
        let (mut out, mut err) = (child.stdout.take().unwrap(), child.stderr.take().unwrap());
        let t_out = thread::spawn(move || { let mut s = String::new(); let _ = out.read_to_string(&mut s); s });
        let t_err = thread::spawn(move || { let mut s = String::new(); let _ = err.read_to_string(&mut s); s });
        let start = Instant::now();
        let code = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status.code(),
                Ok(None) if start.elapsed() > COMMAND_TIMEOUT => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return json!({ "ok": false, "error": "timed out", "stdout": t_out.join().unwrap_or_default() });
                }
                Ok(None) => thread::sleep(Duration::from_millis(20)),
                Err(e) => return json!({ "ok": false, "error": e.to_string() }),
            }
        };
        json!({ "ok": code == Some(0), "code": code, "stdout": t_out.join().unwrap_or_default(), "stderr": t_err.join().unwrap_or_default() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_parse_in_all_three_shapes() {
        assert_eq!(parse_line(r#"{"wifi.rssi": -61, "wifi.bars": 3}"#).len(), 2);
        assert_eq!(parse_line(r#"{"key":"a.b","value":[1,2]}"#), vec![("a.b".to_string(), json!([1, 2]))]);
        assert_eq!(parse_line("sys.load=0.31"), vec![("sys.load".to_string(), json!(0.31))]);
        assert_eq!(parse_line("net.ssid=home wifi"), vec![("net.ssid".to_string(), json!("home wifi"))]);
        assert!(parse_line("   ").is_empty());
        assert!(parse_line("just text").is_empty());
    }

    #[test]
    fn commands_run_by_name_only() {
        let dir = std::env::temp_dir().join(format!("feedtest-{}", std::process::id()));
        fs::create_dir_all(dir.join("commands")).unwrap();
        fs::write(dir.join("commands/hello.sh"), "echo hi $1").unwrap();
        let feeds = Feeds::new(dir.clone(), Arc::new(Hub::new()));
        let r = feeds.run_command("hello", &["there".into()]);
        assert_eq!(r["ok"], true, "{r}");
        assert_eq!(r["stdout"].as_str().unwrap().trim(), "hi there");
        assert_eq!(feeds.run_command("../etc/passwd", &[])["ok"], false);
        assert_eq!(feeds.run_command("missing", &[])["ok"], false);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_feed_script_becomes_data() {
        let dir = std::env::temp_dir().join(format!("feedtest2-{}", std::process::id()));
        fs::create_dir_all(dir.join("feeds")).unwrap();
        fs::write(dir.join("feeds/t.sh"), "echo 'x.y=7'; sleep 30").unwrap();
        let hub = Arc::new(Hub::new());
        let feeds = Feeds::new(dir.clone(), hub.clone());
        feeds.scan();
        let (_, rows) = hub.wait(0, Duration::from_millis(1));
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut got = rows;
        while got.is_empty() && Instant::now() < deadline {
            got = hub.wait(0, Duration::from_millis(1)).1;
            thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(got[0].0, "x.y");
        assert_eq!(got[0].1, json!(7));
        // stop it: removing the file kills the feed on the next scan
        fs::remove_file(dir.join("feeds/t.sh")).unwrap();
        feeds.scan();
        assert!(feeds.status().as_array().unwrap().is_empty());
        let _ = fs::remove_dir_all(dir);
    }
}
