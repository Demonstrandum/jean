//! Crystallizing the journal. Every accepted change is replayed on boot, so replay time grows with every change; the fix is to replace the
//! history with one equivalent change. The agent writes it, and it is accepted only if a scratch display booted from it renders every screen
//! exactly as one booted from the full history does.

use std::collections::BTreeMap;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use store::Store;

use crate::audit::{Action, Actor, Audit};
use crate::claude::{Llm, Message, Request};

/// Every screen as HTML, by screen name, from a display booted on a copy of a store directory.
pub trait Frames {
    fn frames(&self, store_dir: &Path) -> Result<BTreeMap<String, String>, String>;
}

/// What the gate found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Difference {
    /// Screens present in only one of the two.
    Screens { only_old: Vec<String>, only_new: Vec<String> },
    /// A screen whose HTML differs: where, and a little of each side around it.
    Frame { screen: String, at: usize, old: String, new: String },
}

/// `None` if the two render identically.
pub fn compare(old: &BTreeMap<String, String>, new: &BTreeMap<String, String>) -> Option<Difference> {
    let only_old: Vec<String> = old.keys().filter(|k| !new.contains_key(*k)).cloned().collect();
    let only_new: Vec<String> = new.keys().filter(|k| !old.contains_key(*k)).cloned().collect();
    if !only_old.is_empty() || !only_new.is_empty() {
        return Some(Difference::Screens { only_old, only_new });
    }
    for (screen, a) in old {
        let b = &new[screen];
        if a != b {
            let at = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
            let around = |s: &str| {
                let from = (0..=at.saturating_sub(80)).rev().find(|&i| s.is_char_boundary(i)).unwrap_or(0);
                let to = (at + 120..=s.len()).find(|&i| s.is_char_boundary(i)).unwrap_or(s.len());
                s[from..to.min(s.len())].to_string()
            };
            return Some(Difference::Frame { screen: screen.clone(), at, old: around(a), new: around(b) });
        }
    }
    None
}

fn describe(d: &Difference) -> String {
    match d {
        Difference::Screens { only_old, only_new } => format!("screens differ: only before: {only_old:?}; only after: {only_new:?}"),
        Difference::Frame { screen, at, old, new } => format!("the `{screen}` screen differs at byte {at}.\nBefore: …{old}…\nAfter:  …{new}…"),
    }
}

/// A store directory with the journal entries up to `upto` replaced by `candidate` (or, with None, copied as is): only the journal is copied.
pub fn scratch_store(store: &Store, dest: &Path, replace: Option<(u64, &str)>) -> Result<(), String> {
    let _ = std::fs::remove_dir_all(dest);
    std::fs::create_dir_all(dest.join("journal")).map_err(|e| e.to_string())?;
    let tail = store.tail().map_err(|e| e.to_string())?;
    match replace {
        None => {
            for e in &tail {
                std::fs::write(dest.join(format!("journal/{:06}.ts", e.seq)), &e.source).map_err(|e| e.to_string())?;
            }
        }
        Some((upto, source)) => {
            let top = tail.iter().filter(|e| e.seq <= upto).map(|e| e.seq).max().ok_or("nothing to replace")?;
            std::fs::write(dest.join(format!("journal/{top:06}.ts")), source).map_err(|e| e.to_string())?;
            for e in tail.iter().filter(|e| e.seq > upto) {
                std::fs::write(dest.join(format!("journal/{:06}.ts", e.seq)), &e.source).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

pub enum Result_ {
    /// Too short to be worth it.
    NotNeeded,
    /// Replaced `from` entries with one; the commit.
    Crystallized { from: usize, commit: String },
    /// The agent never produced something equivalent; the last difference.
    GaveUp(String),
}

pub struct Crystallizer<'a> {
    pub store: &'a Store,
    pub llm: &'a dyn Llm,
    pub model: String,
    pub frames: &'a dyn Frames,
    pub scratch: PathBuf,
    pub audit: &'a Audit,
    /// Do nothing while the tail is shorter than this.
    pub min_tail: usize,
    pub attempts: usize,
}

/// The one fenced code block of a reply (or the whole reply if it has none).
fn code_of(reply: &str) -> String {
    match reply.find("```") {
        Some(start) => {
            let rest = &reply[start + 3..];
            let body = rest.split_once('\n').map_or(rest, |(_, b)| b);
            body.split("```").next().unwrap_or(body).trim().to_string() + "\n"
        }
        None => reply.trim().to_string() + "\n",
    }
}

impl Crystallizer<'_> {
    pub fn run(&self, now_ms: u64) -> Result<Result_, String> {
        let tail = self.store.tail().map_err(|e| e.to_string())?;
        if tail.len() < self.min_tail.max(2) {
            return Ok(Result_::NotNeeded);
        }
        let upto = tail.last().unwrap().seq;
        let old_dir = self.scratch.join("old");
        scratch_store(self.store, &old_dir, None)?;
        let before = self.frames.frames(&old_dir)?;
        let history: String = tail.iter().map(|e| format!("// ---- change {} ----\n{}\n", e.seq, e.source.trim_end())).collect();
        let mut messages = vec![Message::user(format!(
            "Below are {} display changes, applied in order, each as TypeScript run against the display library. Write ONE change that, run alone on a fresh display, leaves every screen exactly as all of these do together: drop what later changes overwrite, merge repeated patches into the final definitions, and keep anything that still has an effect. Answer with the TypeScript in a single code block and nothing else.\n\n{history}",
            tail.len()
        ))];
        let mut last = String::new();
        for attempt in 1..=self.attempts.max(1) {
            let resp = self.llm.complete(&Request {
                model: self.model.clone(),
                max_tokens: 16_000,
                system: "You compact the history of a display's changes into one equivalent change. The result is checked by rendering every screen from sample data and comparing byte for byte.".into(),
                messages: messages.clone(),
                tools: vec![],
            })?;
            let reply = resp.text();
            let candidate = code_of(&reply);
            let new_dir = self.scratch.join(format!("new-{attempt}"));
            scratch_store(self.store, &new_dir, Some((upto, &candidate)))?;
            let diff = match self.frames.frames(&new_dir) {
                Ok(after) => compare(&before, &after).map(|d| describe(&d)),
                Err(e) => Some(format!("the candidate did not run: {e}")),
            };
            match diff {
                None => {
                    let commit = self.store.collapse(upto, &candidate).map_err(|e| e.to_string())?;
                    self.audit.append(now_ms, Actor::Builder, Action::Crystallized, &format!("{} entries into one at {commit}", tail.len()))?;
                    let _ = std::fs::remove_dir_all(&self.scratch);
                    return Ok(Result_::Crystallized { from: tail.len(), commit });
                }
                Some(why) => {
                    messages.push(Message { role: crate::claude::Role::Assistant, content: resp.content });
                    messages.push(Message::user(format!("That is not equivalent: {why}\n\nFix it and answer again with one code block.")));
                    last = why;
                }
            }
        }
        self.audit.append(now_ms, Actor::Builder, Action::CrystallizeRejected, &last)?;
        let _ = std::fs::remove_dir_all(&self.scratch);
        Ok(Result_::GaveUp(last))
    }
}

/// The instant the scratch displays believe it is (2026-02-25 ~08:53 UTC), so sample data and clocks agree between them.
pub const FROZEN_NOW_MS: u64 = 1_772_000_000_000;

/// The real thing: a display server on a copy of the store, with headless Chrome as its page, asked for `/frames`.
pub struct ScratchDisplay {
    /// The display server binary and the project root it serves (`preview <addr> <state dir> <root>`).
    pub server_bin: PathBuf,
    pub root: PathBuf,
    pub chrome: PathBuf,
}

struct Guard(Vec<Child>, Vec<PathBuf>);
impl Drop for Guard {
    fn drop(&mut self) {
        for c in &mut self.0 {
            let _ = c.kill();
            let _ = c.wait();
        }
        for p in &self.1 {
            let _ = std::fs::remove_dir_all(p);
        }
    }
}

impl Frames for ScratchDisplay {
    fn frames(&self, store_dir: &Path) -> Result<BTreeMap<String, String>, String> {
        let port = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?.local_addr().map_err(|e| e.to_string())?.port();
        let profile = std::env::temp_dir().join(format!("crystal-chrome-{}-{port}", std::process::id()));
        let mut guard = Guard(vec![], vec![profile.clone()]);
        guard.0.push(Command::new(&self.server_bin).arg(format!("127.0.0.1:{port}")).arg(store_dir).arg(&self.root).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| format!("cannot start the display: {e}"))?);
        let base = format!("http://127.0.0.1:{port}");
        wait_for(|| ureq::get(&format!("{base}/status")).call().is_ok(), 40)?;
        guard.0.push(
            Command::new(&self.chrome)
                .args(["--headless=new", "--disable-gpu", "--no-sandbox", "--window-size=1000,800"])
                .arg(format!("--user-data-dir={}", profile.display()))
                .arg(format!("{base}/live?now={FROZEN_NOW_MS}"))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("cannot start the browser: {e}"))?,
        );
        let expected = std::fs::read_dir(store_dir.join("journal")).map_err(|e| e.to_string())?.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "ts")).count();
        wait_for(
            || ureq::get(&format!("{base}/status")).call().ok().and_then(|r| r.into_json::<serde_json::Value>().ok()).is_some_and(|s| s["connected"] == true && s["replay"]["replayed"].as_u64().unwrap_or(0) as usize + s["replay"]["broken"].as_array().map_or(0, Vec::len) >= expected),
            120,
        )?;
        let v: serde_json::Value = ureq::get(&format!("{base}/frames")).timeout(Duration::from_secs(60)).call().map_err(|e| e.to_string())?.into_json().map_err(|e| e.to_string())?;
        if v["ok"] != true {
            return Err(v["error"].as_str().unwrap_or("frames failed").to_string());
        }
        serde_json::from_value(v["frames"].clone()).map_err(|e| e.to_string())
    }
}

fn wait_for(mut ok: impl FnMut() -> bool, tries: usize) -> Result<(), String> {
    for _ in 0..tries {
        if ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err("timed out waiting for the scratch display".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::testing::{say, Script};
    use store::Meta;

    /// A display whose screens are `screen NAME = TEXT` lines, later lines overwriting earlier ones.
    struct Toy;
    impl Frames for Toy {
        fn frames(&self, dir: &Path) -> Result<BTreeMap<String, String>, String> {
            let mut files: Vec<_> = std::fs::read_dir(dir.join("journal")).unwrap().flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
            files.sort();
            let mut out = BTreeMap::new();
            for f in files {
                for l in std::fs::read_to_string(f).unwrap().lines() {
                    if l.contains("BROKEN") {
                        return Err("ReferenceError: nope".into());
                    }
                    if let Some(rest) = l.strip_prefix("screen ") {
                        let (name, text) = rest.split_once(" = ").unwrap();
                        out.insert(name.to_string(), text.to_string());
                    }
                }
            }
            Ok(out)
        }
    }

    fn rig(name: &str) -> (Store, PathBuf, Audit) {
        let root = std::env::temp_dir().join(format!("crys-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let store = Store::open(root.join("store")).unwrap();
        for src in ["screen home = a", "screen home = b", "screen day = x", "screen home = c"] {
            let p = store.begin(src).unwrap();
            store.commit(p, "<dom>", &Meta::default()).unwrap();
        }
        let audit = Audit::open(root.join("audit"));
        (store, root, audit)
    }

    fn reply(code: &str) -> crate::claude::Response {
        say(&format!("Here you go:\n```ts\n{code}\n```"))
    }

    #[test]
    fn differences_name_the_screen_and_where() {
        let a = BTreeMap::from([("home".to_string(), "<p>cpu 41</p>".to_string())]);
        let b = BTreeMap::from([("home".to_string(), "<p>cpu 42</p>".to_string())]);
        assert!(compare(&a, &a).is_none());
        assert!(matches!(compare(&a, &b), Some(Difference::Frame { screen, at: 8, .. }) if screen == "home"));
        let c = BTreeMap::from([("day".to_string(), String::new())]);
        assert!(matches!(compare(&a, &c), Some(Difference::Screens { .. })));
        assert_eq!(code_of("Sure.\n```ts\nlet a = 1;\n```\nDone"), "let a = 1;\n");
    }

    #[test]
    fn an_equivalent_candidate_replaces_the_history() {
        let (store, root, audit) = rig("ok");
        let llm = Script::new(vec![reply("screen home = c\nscreen day = x")]);
        let c = Crystallizer { store: &store, llm: &llm, model: "m".into(), frames: &Toy, scratch: root.join("scratch"), audit: &audit, min_tail: 3, attempts: 3 };
        let r = c.run(1).unwrap();
        assert!(matches!(r, Result_::Crystallized { from: 4, .. }));
        let tail = store.tail().unwrap();
        assert_eq!(tail.len(), 1);
        assert!(tail[0].source.contains("screen day = x"));
        assert_eq!(audit.entries().unwrap()[0].action, Action::Crystallized);
        // and a boot from what is left renders the same
        assert_eq!(Toy.frames(store.dir()).unwrap()["home"], "c");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_wrong_candidate_is_sent_back_with_the_difference_and_a_right_one_accepted() {
        let (store, root, audit) = rig("retry");
        let llm = Script::new(vec![reply("screen home = c"), reply("screen home = c\nscreen day = x")]);
        let c = Crystallizer { store: &store, llm: &llm, model: "m".into(), frames: &Toy, scratch: root.join("scratch"), audit: &audit, min_tail: 2, attempts: 3 };
        assert!(matches!(c.run(1).unwrap(), Result_::Crystallized { .. }));
        let seen = llm.seen.borrow();
        let feedback = format!("{:?}", seen[1].messages.last().unwrap());
        assert!(feedback.contains("not equivalent") && feedback.contains("only before"), "{feedback}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_candidate_that_never_matches_leaves_the_journal_alone() {
        let (store, root, audit) = rig("giveup");
        let llm = Script::new(vec![reply("screen home = zzz"), reply("BROKEN"), reply("screen day = x")]);
        let c = Crystallizer { store: &store, llm: &llm, model: "m".into(), frames: &Toy, scratch: root.join("scratch"), audit: &audit, min_tail: 2, attempts: 3 };
        assert!(matches!(c.run(1).unwrap(), Result_::GaveUp(_)));
        assert_eq!(store.tail().unwrap().len(), 4);
        assert_eq!(audit.entries().unwrap().last().unwrap().action, Action::CrystallizeRejected);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_short_tail_is_left_alone() {
        let (store, root, audit) = rig("short");
        let llm = Script::new(vec![]);
        let c = Crystallizer { store: &store, llm: &llm, model: "m".into(), frames: &Toy, scratch: root.join("s"), audit: &audit, min_tail: 10, attempts: 3 };
        assert!(matches!(c.run(1).unwrap(), Result_::NotNeeded));
        let _ = std::fs::remove_dir_all(root);
    }
}
