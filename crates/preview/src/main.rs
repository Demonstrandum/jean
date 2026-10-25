//! Local dev orchestrator. Serves the demo pages and `/ui.js`, and drives a browser page as
//! the display runtime over HTTP long-polling, so it works with any browser (no CDP needed):
//!
//! ```text
//!  you / Claude --POST /eval--> server --(queue)--> page --GET /poll
//!                                  ^                  |  eval + ui.verify()
//!                                  +---POST /result---+
//!  accepted -> store.commit (journal + snapshot + git commit);  rejected -> store.abort
//! ```
//!
//! When the page connects (`POST /hello`) the journal is replayed into it through the same
//! channel, so a restart rebuilds the screen from the store.
//!
//! Live data rides alongside: `feeds/*` scripts (supervised) and `POST /push` write into a hub; pages long-poll
//! `/sub` for it; `POST /run?cmd=name` runs a named script from `commands/`. See hub.rs and feeds.rs.
//!
//! usage: preview [addr=127.0.0.1:8788] [store-dir=display-state] [root=.]   (feeds/ and commands/ live under root)

mod feeds;
mod hub;

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use store::{Meta, ReplayReport, Runtime, Store};
use tiny_http::{Header, Method, Request, Response, Server};

const EVAL_TIMEOUT: Duration = Duration::from_secs(30);

struct Cmd {
    id: u64,
    js: String,
    kind: Kind,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// A change: run, verify, commit or abort.
    Eval,
    /// A read: run in a throwaway scope, roll back, never journaled.
    Query,
    Reload,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Eval => "eval",
            Kind::Query => "query",
            Kind::Reload => "reload",
        }
    }
}

struct PageResult {
    ok: bool,
    error: Option<String>,
    snapshot: Option<String>,
    free_y: Option<i64>,
    value: Option<String>,
    /// The page's fit report (scale, dropped, ...): how the scene was adapted to the screen.
    fit: Option<Value>,
}

enum Polled {
    Cmd(Cmd),
    Empty,
    /// A newer page has connected; this one must stop.
    Stale,
}

struct Queue {
    cmds: VecDeque<Cmd>,
    /// Current page session. Only this session's poller may take commands, so a stale page
    /// (reloaded, or left open across a server restart) can't swallow one.
    sid: u64,
}

/// The queue between the HTTP handlers and the page's worker loop.
struct Bridge {
    state: Mutex<Queue>,
    cv: Condvar,
    waiting: Mutex<HashMap<u64, Sender<PageResult>>>,
    next: AtomicU64,
    last_seen: Mutex<Option<Instant>>,
}

impl Bridge {
    fn new() -> Self {
        Bridge {
            // start from the clock so session ids never repeat across restarts
            state: Mutex::new(Queue {
                cmds: VecDeque::new(),
                sid: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64),
            }),
            cv: Condvar::new(),
            waiting: Mutex::new(HashMap::new()),
            next: AtomicU64::new(0),
            last_seen: Mutex::new(None),
        }
    }

    fn touch(&self) {
        *self.last_seen.lock().unwrap() = Some(Instant::now());
    }

    fn connected(&self) -> bool {
        self.last_seen.lock().unwrap().map_or(false, |t| t.elapsed() < Duration::from_secs(30))
    }

    /// A page loaded: it becomes the only session. Queued commands and stale pollers are dropped.
    fn new_session(&self) -> u64 {
        let mut st = self.state.lock().unwrap();
        st.sid += 1;
        st.cmds.clear();
        self.cv.notify_all();
        drop(st);
        self.touch();
        self.state.lock().unwrap().sid
    }

    /// Long-poll for session `sid`: the next command, nothing after 20s, or "you are stale".
    fn poll(&self, sid: u64) -> Polled {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut st = self.state.lock().unwrap();
        loop {
            if st.sid != sid {
                return Polled::Stale;
            }
            if let Some(cmd) = st.cmds.pop_front() {
                drop(st);
                self.touch();
                return Polled::Cmd(cmd);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                drop(st);
                self.touch();
                return Polled::Empty;
            }
            st = self.cv.wait_timeout(st, left).unwrap().0;
        }
    }

    fn push(&self, js: String, kind: Kind) -> (u64, Receiver<PageResult>) {
        let id = self.next.fetch_add(1, Ordering::SeqCst) + 1;
        let (tx, rx) = mpsc::channel();
        self.waiting.lock().unwrap().insert(id, tx);
        self.state.lock().unwrap().cmds.push_back(Cmd { id, js, kind });
        self.cv.notify_all();
        (id, rx)
    }

    fn eval(&self, js: String) -> Result<PageResult, String> {
        self.run(js, Kind::Eval)
    }

    fn run(&self, js: String, kind: Kind) -> Result<PageResult, String> {
        if !self.connected() {
            return Err("display page is not connected (open /live)".into());
        }
        let (id, rx) = self.push(js, kind);
        rx.recv_timeout(EVAL_TIMEOUT).map_err(|_| {
            self.waiting.lock().unwrap().remove(&id);
            self.state.lock().unwrap().cmds.retain(|c| c.id != id);
            "timed out waiting for the display page".to_string()
        })
    }

    fn resolve(&self, id: u64, res: PageResult) {
        if let Some(tx) = self.waiting.lock().unwrap().remove(&id) {
            let _ = tx.send(res);
        }
    }
}

/// `store::Runtime` over the page: strip types, send, wait for the verify outcome.
struct PageRuntime {
    bridge: Arc<Bridge>,
    snap: String,
}

impl Runtime for PageRuntime {
    fn eval_verify(&mut self, source: &str) -> Result<(), String> {
        let js = ui::strip_eval(source)?;
        let res = self.bridge.eval(js)?;
        if res.ok {
            self.snap = res.snapshot.unwrap_or_default();
            Ok(())
        } else {
            Err(res.error.unwrap_or_else(|| "rejected".into()))
        }
    }
    fn snapshot(&mut self) -> String {
        self.snap.clone()
    }
}

struct App {
    store: Store,
    bridge: Arc<Bridge>,
    /// One change at a time: evals, undo and boot replay all take this.
    change_lock: Mutex<()>,
    replay: Mutex<Option<ReplayReport>>,
    here: PathBuf,
    hub: Arc<hub::Hub>,
    feeds: Arc<feeds::Feeds>,
}

fn main() {
    let addr = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:8788".into());
    let dir = std::env::args().nth(2).unwrap_or_else(|| "display-state".into());
    let store = Store::open(&dir).expect("open store");
    println!("store: {dir} (head {})", store.head().unwrap_or_default());

    let root = PathBuf::from(std::env::args().nth(3).unwrap_or_else(|| ".".into()));
    let hub = Arc::new(hub::Hub::new());
    let feeds = feeds::Feeds::new(root, hub.clone());
    feeds.supervise();

    let app = Arc::new(App {
        store,
        bridge: Arc::new(Bridge::new()),
        change_lock: Mutex::new(()),
        replay: Mutex::new(None),
        here: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        hub,
        feeds,
    });
    let server = Arc::new(Server::http(&addr).expect("bind"));
    println!("preview on http://{addr}   (display page: /live)");

    // long-polls block a thread each, so serve with a small pool
    let workers: Vec<_> = (0..16)
        .map(|_| {
            let (server, app) = (server.clone(), app.clone());
            thread::spawn(move || {
                for req in server.incoming_requests() {
                    handle(&app, req);
                }
            })
        })
        .collect();
    for w in workers {
        let _ = w.join();
    }
}

fn handle(app: &Arc<App>, mut req: Request) {
    let url = req.url().to_string();
    let (path, query) = url.split_once('?').unwrap_or((url.as_str(), ""));
    let post = *req.method() == Method::Post;
    let mut body = String::new();
    if post {
        let _ = req.as_reader().read_to_string(&mut body);
    }

    match (post, path) {
        (false, "/" | "/index.html") => file(req, app, "static/index.html", "text/html; charset=utf-8"),
        (false, "/live") => file(req, app, "static/live.html", "text/html; charset=utf-8"),
        (false, "/ui.js") => {
            // from the source tree when it exists (edit ui.ts and reload), else the copy compiled into the binary
            let src = fs::read_to_string(app.here.join("../ui/ts/ui.ts")).unwrap_or_else(|_| ui::SOURCE.to_string());
            match ui::bundle(&src) {
                Ok(js) => text(req, 200, "text/javascript; charset=utf-8", js),
                Err(e) => text(req, 500, "text/javascript", format!("console.error({e:?});")),
            }
        }
        (false, p) if p.starts_with("/fonts/") => match ui::font(&p["/fonts/".len()..]) {
            Some(bytes) => {
                let resp = Response::from_data(bytes.to_vec())
                    .with_header(Header::from_bytes("Content-Type", "font/woff2").unwrap())
                    .with_header(Header::from_bytes("Cache-Control", "public, max-age=3600").unwrap());
                let _ = req.respond(resp);
            }
            None => text(req, 404, "text/plain", "no such font".into()),
        },
        (false, "/status") => reply(req, 200, status(app)),
        (false, "/poll") => {
            let sid = query_param(query, "sid").and_then(|s| s.parse().ok()).unwrap_or(0);
            let v = match app.bridge.poll(sid) {
                Polled::Cmd(c) => json!({ "id": c.id, "js": c.js, "kind": c.kind.as_str() }),
                Polled::Empty => json!({}),
                Polled::Stale => json!({ "stale": true }),
            };
            reply(req, 200, v)
        }
        (true, "/result") => {
            let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            if let Some(id) = v["id"].as_u64() {
                app.bridge.resolve(
                    id,
                    PageResult {
                        ok: v["ok"].as_bool().unwrap_or(false),
                        error: v["error"].as_str().map(String::from),
                        snapshot: v["snapshot"].as_str().map(String::from),
                        free_y: v["free_y"].as_i64(),
                        value: v["value"].as_str().map(String::from),
                        fit: v.get("fit").cloned(),
                    },
                );
            }
            reply(req, 200, json!({ "ok": true }))
        }
        (true, "/query") => {
            // read-only: evaluated in a throwaway scope, rolled back, never journaled
            let js = match ui::strip_eval(&body) {
                Ok(js) => js,
                Err(e) => return reply(req, 400, json!({ "ok": false, "error": format!("syntax: {e}") })),
            };
            let _guard = app.change_lock.lock().unwrap();
            match app.bridge.run(js, Kind::Query) {
                Ok(res) if res.ok => reply(req, 200, json!({ "ok": true, "value": res.value.and_then(|v| serde_json::from_str::<Value>(&v).ok()) })),
                Ok(res) => reply(req, 200, json!({ "ok": false, "error": res.error.unwrap_or_default() })),
                Err(e) => reply(req, 503, json!({ "ok": false, "error": e })),
            }
        }
        (false, "/sub") => {
            // long-poll for live data: returns as soon as something newer than `since` exists
            let since = query_param(query, "since").and_then(|s| s.parse().ok()).unwrap_or(0);
            let (seq, rows) = app.hub.wait(since, Duration::from_secs(20));
            reply(req, 200, json!({ "seq": seq, "updates": rows.into_iter().map(|(k, v, ts)| json!([k, v, ts])).collect::<Vec<_>>() }))
        }
        (false, "/data") => reply(req, 200, app.hub.snapshot()),
        (false, "/feeds") => reply(req, 200, app.feeds.status()),
        (true, "/push") => {
            // {"key": "...", "value": ...} or {"a.b": 1, "c": 2}; the same shapes a feed prints
            let rows = feeds::parse_line(&body.replace('\n', " "));
            if rows.is_empty() {
                return reply(req, 400, json!({ "ok": false, "error": "expected {\"key\":..,\"value\":..} or an object of keys" }));
            }
            for (k, v) in &rows {
                app.hub.set(k, v.clone(), None);
            }
            reply(req, 200, json!({ "ok": true, "keys": rows.len() }))
        }
        (true, "/run") => {
            let name = query_param(query, "cmd").unwrap_or_default();
            let args: Vec<String> = query.split('&').filter_map(|kv| kv.split_once('=')).filter(|(k, _)| *k == "arg").map(|(_, v)| percent_decode(v)).collect();
            reply(req, 200, app.feeds.run_command(&name, &args))
        }
        (true, "/hello") => {
            // a page just loaded: it becomes the only session, and its state is rebuilt from the store
            let sid = app.bridge.new_session();
            let app = app.clone();
            thread::spawn(move || {
                let _guard = app.change_lock.lock().unwrap();
                let mut rt = PageRuntime { bridge: app.bridge.clone(), snap: String::new() };
                match store::boot_replay(&app.store, &mut rt) {
                    Ok(report) => {
                        println!("replay: {} ok, {} broken, {} recovered, {} dropped", report.replayed, report.broken.len(), report.recovered.len(), report.dropped.len());
                        *app.replay.lock().unwrap() = Some(report);
                    }
                    Err(e) => eprintln!("replay failed: {e}"),
                }
            });
            reply(req, 200, json!({ "ok": true, "sid": sid }))
        }
        (true, "/eval") => {
            let meta = Meta {
                intent: query_param(query, "intent"),
                utterance: query_param(query, "utterance"),
                turn: query_param(query, "turn"),
                free_y: None,
            };
            let (status, v) = eval_route(app, &body, meta);
            reply(req, status, v)
        }
        (true, "/undo") => {
            let _guard = app.change_lock.lock().unwrap();
            match app.store.undo_last() {
                Ok(Some(seq)) => {
                    app.bridge.push(String::new(), Kind::Reload); // page reloads, then replays the shortened journal
                    reply(req, 200, json!({ "ok": true, "undone": seq }))
                }
                Ok(None) => reply(req, 200, json!({ "ok": false, "error": "nothing to undo" })),
                Err(e) => reply(req, 500, json!({ "ok": false, "error": e.to_string() })),
            }
        }
        _ => text(req, 404, "text/plain", "not found".into()),
    }
}

/// One change: strip types, record as pending, run it in the page, then commit or abort.
fn eval_route(app: &App, source: &str, mut meta: Meta) -> (u16, Value) {
    let js = match ui::strip_eval(source) {
        Ok(js) => js,
        Err(e) => return (400, json!({ "ok": false, "error": format!("syntax: {e}") })),
    };
    let _guard = app.change_lock.lock().unwrap();
    let pending = match app.store.begin(source) {
        Ok(p) => p,
        Err(e) => return (500, json!({ "ok": false, "error": e.to_string() })),
    };
    let seq = pending.seq;
    match app.bridge.eval(js) {
        Ok(res) if res.ok => {
            meta.free_y = res.free_y;
            match app.store.commit(pending, &res.snapshot.unwrap_or_default(), &meta) {
                Ok(commit) => (200, json!({ "ok": true, "seq": seq, "commit": commit, "free_y": res.free_y, "fit": res.fit })),
                Err(e) => (500, json!({ "ok": false, "error": e.to_string() })),
            }
        }
        Ok(res) => {
            let _ = app.store.abort(pending);
            (200, json!({ "ok": false, "rejected": true, "error": res.error.unwrap_or_default() }))
        }
        Err(e) => {
            let _ = app.store.abort(pending);
            (503, json!({ "ok": false, "error": e }))
        }
    }
}

fn status(app: &App) -> Value {
    let replay = app.replay.lock().unwrap().as_ref().map(|r| {
        json!({
            "replayed": r.replayed,
            "broken": r.broken.iter().map(|(s, why)| json!({ "seq": s, "why": why })).collect::<Vec<_>>(),
            "recovered": r.recovered,
            "dropped": r.dropped,
        })
    });
    json!({
        "connected": app.bridge.connected(),
        "head": app.store.head(),
        "tail": app.store.tail().map(|t| t.len()).unwrap_or(0),
        "pending": app.store.pending().map(|p| p.len()).unwrap_or(0),
        "replay": replay,
    })
}

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| percent_decode(v))
    })
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 3 <= b.len() => {
                let hex = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
                match hex {
                    Some(v) => {
                        out.push(v);
                        i += 3;
                    }
                    None => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Pages come from the source tree when it exists (edit and reload), else from the copies compiled into the binary.
fn file(req: Request, app: &App, rel: &str, ctype: &str) {
    let embedded = match rel {
        "static/index.html" => INDEX,
        "static/live.html" => LIVE,
        _ => "",
    };
    let body = fs::read_to_string(app.here.join(rel)).unwrap_or_else(|_| embedded.to_string());
    text(req, 200, ctype, body)
}

const INDEX: &str = include_str!("../static/index.html");
const LIVE: &str = include_str!("../static/live.html");

fn reply(req: Request, status: u16, v: Value) {
    text(req, status, "application/json", v.to_string())
}

fn text(req: Request, status: u16, ctype: &str, body: String) {
    let resp = Response::from_string(body)
        .with_status_code(status)
        .with_header(Header::from_bytes("Content-Type", ctype).unwrap())
        .with_header(Header::from_bytes("Cache-Control", "no-store").unwrap());
    let _ = req.respond(resp);
}
