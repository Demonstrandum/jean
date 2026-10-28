//! Can a real Claude drive the display? Each trial gives an actual model (the logged-in `claude` program) a spoken-style request and the voice agent's tools,
//! on a fresh scratch display (the preview server on a new store with the shipped screens, and headless Chrome as its page), then checks the display's real
//! state. A trial that fails means the REPL is not intuitive enough for an agent: fix the API, the reference or the prompt, not the trial.
//!
//!   cargo test -p harness --test ui_trials -- --ignored --nocapture
//!   JEAN_TRIALS=retitle,undo cargo test ...    (only those)        JEAN_TRIAL_MODEL=claude-sonnet-5-5 (the model)

use std::cell::RefCell;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use harness::claude::{Block, Llm, Request, Response, ClaudeCli};
use harness::display::Display;
use harness::session::Session;
use harness::tasks::Tasks;
use serde_json::{json, Value};
use store::{Meta, Store};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn chrome() -> Option<PathBuf> {
    ["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "/Applications/Chromium.app/Contents/MacOS/Chromium", "/usr/bin/google-chrome", "/usr/bin/chromium", "/usr/bin/chromium-browser"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
}

/// A display of its own: server, store and page, cleaned up on drop.
struct Scratch {
    children: Vec<Child>,
    dirs: Vec<PathBuf>,
    base: String,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        for c in &mut self.children {
            let _ = c.kill();
            let _ = c.wait();
        }
        for d in &self.dirs {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}

fn wait_for(what: &str, mut ok: impl FnMut() -> bool) {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(60) {
        if ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    panic!("timed out waiting for {what}");
}

fn start_display(name: &str) -> Scratch {
    let root = root();
    let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let dir = std::env::temp_dir().join(format!("trial-{name}-{}-{port}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // as the server does on a first start: the shipped screens are the first entries of the journal
    let store = Store::open(dir.join("store")).unwrap();
    let mut seeds: Vec<_> = std::fs::read_dir(root.join("crates/ui/seed")).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "ts")).collect();
    seeds.sort();
    for f in &seeds {
        let p = store.begin(&std::fs::read_to_string(f).unwrap()).unwrap();
        store.commit(p, "<dom>", &Meta::default()).unwrap();
    }
    drop(store);
    let mut s = Scratch { children: vec![], dirs: vec![dir.clone()], base: format!("http://127.0.0.1:{port}") };
    s.children.push(Command::new(root.join("target/debug/preview")).arg(format!("127.0.0.1:{port}")).arg(dir.join("store")).arg(&root).stdout(Stdio::null()).stderr(Stdio::null()).spawn().expect("the display server"));
    let base = s.base.clone();
    wait_for("the display server", || ureq::get(&format!("{base}/status")).call().is_ok());
    s.children.push(
        Command::new(chrome().expect("Chrome"))
            .args(["--headless=new", "--disable-gpu", "--no-sandbox", "--window-size=1000,800"])
            .arg(format!("--user-data-dir={}", dir.join("chrome").display()))
            .arg(format!("{base}/live?panel&size=800x600"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("the page"),
    );
    let n = seeds.len() as u64;
    wait_for("the page to replay the screens", || {
        ureq::get(&format!("{base}/status")).call().ok().and_then(|r| r.into_json::<Value>().ok()).is_some_and(|v| v["connected"] == true && v["replay"]["replayed"].as_u64().unwrap_or(0) >= n)
    });
    std::thread::sleep(Duration::from_secs(2));
    s
}

/// The model, recorded: every tool call and answer, so a failed trial can be read.
struct Recording<'a> {
    inner: &'a dyn Llm,
    log: RefCell<Vec<String>>,
}

impl Llm for Recording<'_> {
    fn complete(&self, req: &Request) -> Result<Response, String> {
        let r = self.inner.complete(req)?;
        for b in &r.content {
            match b {
                Block::Text { text } => self.log.borrow_mut().push(format!("SAYS   {text}")),
                Block::ToolUse { name, input, .. } => self.log.borrow_mut().push(format!("CALLS  {name} {input}")),
                _ => {}
            }
        }
        if let Some(last) = req.messages.last() {
            for b in &last.content {
                if let Block::ToolResult { content, is_error, .. } = b {
                    self.log.borrow_mut().push(format!("RESULT {}{}", if *is_error { "(error) " } else { "" }, content.chars().take(400).collect::<String>()));
                }
            }
        }
        Ok(r)
    }
}

struct Trial {
    name: &'static str,
    /// What is said to it, one turn each (a later turn sees the earlier ones).
    says: &'static [&'static str],
    /// Looks at the display afterwards: Ok, or what is wrong.
    check: fn(&Display) -> Result<(), String>,
}

fn q(d: &Display, code: &str) -> Result<Value, String> {
    let v = d.query(code).map_err(|e| format!("query {code}: {e}"))?;
    if v["ok"] == true { Ok(v["value"].clone()) } else { Err(format!("query {code}: {}", v["error"])) }
}

fn fits(d: &Display) -> Result<(), String> {
    let v = q(d, "ui.describe().fit")?;
    if v["fits"] == false || v["clipped"] == true { Err(format!("the screen does not fit: {v}")) } else { Ok(()) }
}

const TRIALS: &[Trial] = &[
    Trial {
        name: "retitle",
        says: &["Rename the weather tile on the home screen to \"Weather now\"."],
        check: |d| {
            let t = q(d, r#"ui.screenOf("home").tile("weather").title"#)?;
            if t == "Weather now" { Ok(()) } else { Err(format!("the title is {t}")) }
        },
    },
    Trial {
        name: "message",
        says: &["Put a big message on the display that says: Dinner is ready."],
        check: |d| {
            let shown = q(d, r#"(() => { const x = ui.describe(); return x.tiles.some((t) => /dinner is ready/i.test(t.text)); })()"#)?;
            if shown == true { fits(d) } else { Err(format!("the current screen does not show it: {}", q(d, "ui.describe().tiles.map((t) => t.text)").unwrap_or_default())) }
        },
    },
    Trial {
        name: "new_screen",
        says: &["Make a new screen called garden with one tile titled Soil that says Moist 42 percent, and show it."],
        check: |d| {
            let ids = q(d, "ui.screenIds()")?;
            if !ids.as_array().is_some_and(|a| a.iter().any(|x| x == "garden")) {
                return Err(format!("no garden screen: {ids}"));
            }
            let ok = q(d, r#"(() => { const t = ui.screenOf("garden").tiles(); return t.some((x) => /soil/i.test(x.title)); })()"#)?;
            if ok != true {
                return Err("no Soil tile on the garden screen".into());
            }
            let cur = q(d, "ui.current()")?;
            if cur != "garden" { Err(format!("it is not showing the garden screen: {cur}")) } else { fits(d) }
        },
    },
    Trial {
        name: "chart",
        says: &["Replace the Link tile on the home screen with a tile titled Visitors that draws a little line chart of these numbers: 3, 5, 2, 8, 6."],
        check: |d| {
            let r = q(d, r#"(() => { const t = ui.describe().tiles.find((x) => x.title === "Visitors"); return t ? !!document.querySelector('[data-tile="' + t.id + '"] svg') : "no Visitors tile"; })()"#)?;
            if r == true { fits(d) } else { Err(format!("{r}")) }
        },
    },
    Trial {
        name: "live_black",
        says: &["Make the Grid tile on the home screen turn black whenever the carbon intensity is above 100 grams."],
        check: |d| {
            d.push(&json!({ "carbon.now": 250 })).map_err(|e| e.to_string())?;
            std::thread::sleep(Duration::from_secs(3));
            let hi = q(d, r#"ui.describe().tiles.find((t) => t.id === "grid").inverted"#)?;
            d.push(&json!({ "carbon.now": 40 })).map_err(|e| e.to_string())?;
            std::thread::sleep(Duration::from_secs(3));
            let lo = q(d, r#"ui.describe().tiles.find((t) => t.id === "grid").inverted"#)?;
            if hi == true && lo == false { Ok(()) } else { Err(format!("inverted at 250: {hi}, at 40: {lo}")) }
        },
    },
    Trial {
        name: "live_count",
        says: &["On the home screen, make the Shop tile's title say how many items are still to get, and keep it up to date if the list changes."],
        check: |d| {
            // the sample list has three open items; push a list with four, and the title has to follow within a beat of the tick tier
            d.push(&json!({ "shop.items": [{ "text": "Milk", "qty": "2", "done": false }, { "text": "Eggs", "qty": "1", "done": false }, { "text": "Jam", "qty": "1", "done": false }, { "text": "Rice", "qty": "1", "done": false }, { "text": "Tea", "qty": "1", "done": true }] })).map_err(|e| e.to_string())?;
            let mut title = String::new();
            for _ in 0..40 {
                std::thread::sleep(Duration::from_secs(1));
                title = q(d, r#"ui.describe().tiles.find((t) => t.id === "shop").title"#)?.to_string();
                if title.contains('4') {
                    return Ok(());
                }
            }
            Err(format!("with 4 items to get the title is {title}"))
        },
    },
    Trial {
        name: "live_chart",
        says: &["On the weather screen add a tile titled Next hours that draws a line chart of the temperature for the next twelve hours, from the live forecast."],
        check: |d| {
            q(d, r#"ui.go("weather")"#)?;
            std::thread::sleep(Duration::from_secs(2));
            let r = q(d, r#"(() => { const t = ui.describe().tiles.find((x) => /next hours/i.test(x.title)); return t ? !!document.querySelector('[data-tile="' + t.id + '"] svg') : "no Next hours tile on the weather screen"; })()"#)?;
            if r == true { fits(d) } else { Err(format!("{r}")) }
        },
    },
    Trial {
        name: "party",
        says: &["Make a screen called party with three tiles: one with a big message that says Welcome, one with the shopping list, and one with the clock. Show it."],
        check: |d| {
            let ids = q(d, "ui.screenIds()")?;
            if !ids.as_array().is_some_and(|a| a.iter().any(|x| x == "party")) {
                return Err(format!("no party screen: {ids}"));
            }
            let n = q(d, r#"ui.screenOf("party").tiles().length"#)?;
            let welcome = q(d, r#"(() => { ui.go("party"); return ui.describe().tiles.some((t) => /welcome/i.test(t.text)); })()"#)?;
            if n.as_u64().unwrap_or(0) < 3 {
                return Err(format!("only {n} tiles"));
            }
            if welcome != true {
                return Err("no tile says Welcome".into());
            }
            fits(d)
        },
    },
    Trial {
        name: "rearrange",
        says: &["Remove the Link tile from the home screen and make the weather tile bigger so it uses the space."],
        check: |d| {
            let gone = q(d, r#"!ui.screenOf("home").tiles().some((t) => t.id === "link")"#)?;
            if gone != true {
                return Err("the Link tile is still there".into());
            }
            let cells = q(d, r#"(() => { const z = ui.screenOf("home").tile("weather").at; return (z.to.col - z.from.col + 1) * (z.to.row - z.from.row + 1); })()"#)?;
            if cells.as_u64().unwrap_or(0) < 3 {
                return Err(format!("the weather tile covers {cells} cells, it was 2"));
            }
            fits(d)
        },
    },
    Trial {
        name: "long_text",
        says: &["Put this on the Claude tile of the home screen, as its body: Today the household agreed that the kitchen shelf will be cleared by Friday, the recycling goes out on Thursday evening, the boiler service is booked for the morning of the fourteenth, and everybody should remember that the spare key lives behind the blue pot by the back door."],
        check: |d| {
            let r = q(d, r#"(() => { const t = ui.describe().tiles.find((x) => x.id === "claude"); return t ? /spare key|boiler|recycling/i.test(t.text) : "no claude tile"; })()"#)?;
            if r != true {
                return Err(format!("the text is not on the tile: {r}"));
            }
            fits(d)
        },
    },
    Trial {
        name: "undo",
        says: &["Rename the weather tile on the home screen to Sky.", "Actually, put it back the way it was."],
        check: |d| {
            let t = q(d, r#"ui.screenOf("home").tile("weather").title"#)?;
            if t == "Wx" { Ok(()) } else { Err(format!("the title is {t}, not Wx")) }
        },
    },
];

#[test]
#[ignore]
fn real_claudes_drive_the_display() {
    if chrome().is_none() {
        eprintln!("no Chrome here; skipping");
        return;
    }
    assert!(Command::new("cargo").args(["build", "-q", "-p", "preview"]).current_dir(root()).status().unwrap().success());
    let only: Option<Vec<String>> = std::env::var("JEAN_TRIALS").ok().map(|s| s.split(',').map(str::to_string).collect());
    let cli = ClaudeCli { bin: "claude".into(), model: std::env::var("JEAN_TRIAL_MODEL").ok() };
    let mut failures = Vec::new();
    for t in TRIALS.iter().filter(|t| only.as_ref().is_none_or(|o| o.iter().any(|n| n == t.name))) {
        let started = Instant::now();
        let scratch = start_display(t.name);
        let display = Display::new(&scratch.base);
        let tasks_dir = std::env::temp_dir().join(format!("trial-tasks-{}-{}", t.name, std::process::id()));
        let tasks = Tasks::open(&tasks_dir).unwrap();
        let rec = Recording { inner: &cli, log: RefCell::new(vec![]) };
        let mut session = Session::new(&rec, &display, &tasks, "", chrono_tz::Europe::London, "London");
        // while the agent works, watch what the display is told about it: the keys the black Claude tile follows
        let watching = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let seen_live = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let watcher = {
            let (watching, seen_live, base) = (watching.clone(), seen_live.clone(), scratch.base.clone());
            std::thread::spawn(move || {
                let d = Display::new(base);
                while watching.load(std::sync::atomic::Ordering::Relaxed) {
                    if let Ok(data) = d.data() {
                        let steps = data["claude.steps"]["value"].as_array().map(|a| a.iter().map(|x| format!("{}{}", x["tool"].as_str().unwrap_or(""), match x["ok"] { Value::Bool(true) => "✓", Value::Bool(false) => "✗", _ => "…" })).collect::<Vec<_>>().join(" ")).unwrap_or_default();
                        let line = format!("state {} | {}", data["claude.state"]["value"], steps);
                        let mut v = seen_live.lock().unwrap();
                        if v.last() != Some(&line) {
                            v.push(line);
                        }
                    }
                    std::thread::sleep(Duration::from_millis(250));
                }
            })
        };
        let mut replies = Vec::new();
        for say in t.says {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
            replies.push(session.handle(say, now).unwrap_or_else(|e| format!("(error: {e})")));
        }
        watching.store(false, std::sync::atomic::Ordering::Relaxed);
        let _ = watcher.join();
        let verdict = (t.check)(&display).map_err(|e| {
            let seen = q(&display, r#"(() => { const x = ui.describe(); return x.screen + ": " + x.tiles.map((t) => t.id + "[" + t.title + "]").join(" "); })()"#).unwrap_or_default();
            format!("{e} (the display shows {seen})")
        });
        let calls = rec.log.borrow().iter().filter(|l| l.starts_with("CALLS")).count();
        eprintln!("\n=== {} — {} ({} tool calls, {:.0}s)", t.name, if verdict.is_ok() { "PASS" } else { "FAIL" }, calls, started.elapsed().as_secs_f32());
        for l in rec.log.borrow().iter() {
            eprintln!("  {}", l.chars().take(700).collect::<String>());
        }
        eprintln!("  REPLIES {replies:?}");
        for l in seen_live.lock().unwrap().iter() {
            eprintln!("  LIVE    {l}");
        }
        if let Err(e) = verdict {
            eprintln!("  WHY     {e}");
            failures.push(format!("{}: {e}", t.name));
        }
        let _ = std::fs::remove_dir_all(tasks_dir);
    }
    assert!(failures.is_empty(), "these trials failed:\n{}", failures.join("\n"));
}
