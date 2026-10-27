//! Durable state for the display: an append-only journal of verified evals, one git commit
//! each, plus a DOM snapshot cache. The browser is disposable; this crate is the memory.
//!
//! Layout of the store directory (its own git repo):
//!
//! ```text
//! journal/000142.ts      accepted evals, one file each, in order
//! journal/pending/       evals sent to the runtime but not yet verified (crash recovery)
//! snapshot.html          DOM after the latest accepted eval (cache + divergence check)
//! .seq                   last sequence number issued (not tracked by git)
//! ```
//!
//! Flow per change: [`Store::begin`] writes the pending file, the runtime evaluates and
//! verifies, then [`Store::commit`] (accepted) or [`Store::abort`] (rejected).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

mod replay;
pub use replay::{boot_replay, ReplayReport, Runtime};

#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error(e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;

/// An accepted eval in the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub seq: u64,
    pub source: String,
}

/// An eval that has been issued but not yet accepted or rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub seq: u64,
    pub source: String,
}

/// Context recorded in the commit message trailers.
#[derive(Debug, Default, Clone)]
pub struct Meta {
    /// Claude's one-line intent; becomes the commit subject.
    pub intent: Option<String>,
    pub utterance: Option<String>,
    pub turn: Option<String>,
    /// Spare vertical pixels after the change (from the fit report).
    pub free_y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    /// Open the store at `dir`, creating the directory and git repo on first use.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Store> {
        let store = Store { dir: dir.into() };
        fs::create_dir_all(store.pending_dir())?;
        if !store.dir.join(".git").exists() {
            store.git(&["init", "-q", "-b", "main"])?;
        }
        let ignore = store.dir.join(".gitignore");
        if !ignore.exists() {
            fs::write(ignore, ".seq\njournal/pending/\n")?;
        }
        if store.head().is_none() {
            store.git(&["add", "-A"])?;
            store.git(&["commit", "-q", "-m", "init display store"])?;
        }
        Ok(store)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Short hash of HEAD.
    pub fn head(&self) -> Option<String> {
        self.git(&["rev-parse", "--short", "HEAD"]).ok().filter(|s| !s.is_empty())
    }

    /// The cached DOM snapshot, if any change has been committed.
    pub fn snapshot(&self) -> Option<String> {
        fs::read_to_string(self.dir.join("snapshot.html")).ok()
    }

    /// Accepted evals in order.
    pub fn tail(&self) -> Result<Vec<Entry>> {
        Ok(read_numbered(&self.journal_dir())?
            .into_iter()
            .map(|(seq, source)| Entry { seq, source })
            .collect())
    }

    /// Evals that were issued but never settled (the process died mid-change).
    pub fn pending(&self) -> Result<Vec<Pending>> {
        Ok(read_numbered(&self.pending_dir())?
            .into_iter()
            .map(|(seq, source)| Pending { seq, source })
            .collect())
    }

    /// Record that `source` is about to be evaluated. Sequence numbers only go up.
    pub fn begin(&self, source: &str) -> Result<Pending> {
        let seq = self.next_seq()?;
        fs::write(self.dir.join(".seq"), seq.to_string())?;
        fs::write(self.pending_path(seq), source)?;
        Ok(Pending { seq, source: source.to_string() })
    }

    /// The eval was verified: move it into the journal, refresh the snapshot, commit.
    /// Returns the short commit hash.
    pub fn commit(&self, pending: Pending, snapshot: &str, meta: &Meta) -> Result<String> {
        fs::rename(self.pending_path(pending.seq), self.journal_path(pending.seq))?;
        fs::write(self.dir.join("snapshot.html"), snapshot)?;
        self.git(&["add", "-A"])?;
        self.git(&["commit", "-q", "-m", &message(&pending, meta)])?;
        self.head().ok_or_else(|| Error("no HEAD after commit".into()))
    }

    /// The eval was rejected: forget it. Nothing is committed.
    pub fn abort(&self, pending: Pending) -> Result<()> {
        match fs::remove_file(self.pending_path(pending.seq)) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }

    /// Undo the newest journal entry with a new commit (history is kept). The snapshot goes
    /// back to what it was after the previous entry. The caller must restart/replay the
    /// runtime afterwards. Returns the seq that was undone.
    pub fn undo_last(&self) -> Result<Option<u64>> {
        let tail = self.tail()?;
        let Some(last) = tail.last() else { return Ok(None) };
        fs::remove_file(self.journal_path(last.seq))?;

        let snapshot_path = self.dir.join("snapshot.html");
        let previous = tail.len().checked_sub(2).map(|i| tail[i].seq);
        match previous.and_then(|seq| self.snapshot_at(seq)) {
            Some(html) => fs::write(&snapshot_path, html)?,
            None => {
                let _ = fs::remove_file(&snapshot_path);
            }
        }
        self.git(&["add", "-A"])?;
        self.git(&["commit", "-q", "-m", &format!("Undo seq {}\n\nUndoes: {}", last.seq, last.seq)])?;
        Ok(Some(last.seq))
    }

    /// Crystallize: replace every journal entry up to `upto` with one entry holding `source`, which must rebuild the same display on its own
    /// (the caller proves that first). Later entries stay. Tagged `cp-<seq>`; returns the short commit hash.
    pub fn collapse(&self, upto: u64, source: &str) -> Result<String> {
        let covered: Vec<Entry> = self.tail()?.into_iter().filter(|e| e.seq <= upto).collect();
        let Some(top) = covered.last().map(|e| e.seq) else { return Err(Error(format!("no journal entries up to {upto}"))) };
        for e in &covered {
            fs::remove_file(self.journal_path(e.seq))?;
        }
        fs::write(self.journal_path(top), source)?;
        self.git(&["add", "-A"])?;
        self.git(&["commit", "-q", "-m", &format!("crystallize: {} entries into one\n\nSeq: {top}\nCrystallized: {}", covered.len(), covered.len())])?;
        self.git(&["tag", "-f", &format!("cp-{top}")])?;
        self.head().ok_or_else(|| Error("no HEAD after commit".into()))
    }

    /// `snapshot.html` as it was in the commit that recorded `seq`.
    fn snapshot_at(&self, seq: u64) -> Option<String> {
        let hash = self
            .git(&["log", "-1", "--format=%H", &format!("--grep=^Seq: {seq}$")])
            .ok()
            .filter(|h| !h.is_empty())?;
        self.git(&["show", &format!("{hash}:snapshot.html")]).ok()
    }

    fn journal_dir(&self) -> PathBuf {
        self.dir.join("journal")
    }
    fn pending_dir(&self) -> PathBuf {
        self.dir.join("journal/pending")
    }
    fn journal_path(&self, seq: u64) -> PathBuf {
        self.journal_dir().join(format!("{seq:06}.ts"))
    }
    fn pending_path(&self, seq: u64) -> PathBuf {
        self.pending_dir().join(format!("{seq:06}.ts"))
    }

    fn next_seq(&self) -> Result<u64> {
        let mut max: u64 = fs::read_to_string(self.dir.join(".seq"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        for dir in [self.journal_dir(), self.pending_dir()] {
            for (seq, _) in read_numbered(&dir)? {
                max = max.max(seq);
            }
        }
        Ok(max + 1)
    }

    fn git(&self, args: &[&str]) -> Result<String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(&self.dir)
            .args(["-c", "user.name=display", "-c", "user.email=display@localhost", "-c", "commit.gpgsign=false"])
            .args(args)
            .output()?;
        if !out.status.success() {
            return Err(Error(format!(
                "git {}: {}",
                args.first().copied().unwrap_or(""),
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

/// `NNNNNN.ts` files in `dir` as (seq, contents), ordered by seq. Other names are ignored.
fn read_numbered(dir: &Path) -> Result<Vec<(u64, String)>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ts") {
            continue;
        }
        let Some(seq) = path.file_stem().and_then(|s| s.to_str()).and_then(|s| s.parse::<u64>().ok()) else {
            continue;
        };
        out.push((seq, fs::read_to_string(&path)?));
    }
    out.sort_by_key(|(seq, _)| *seq);
    Ok(out)
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn message(pending: &Pending, meta: &Meta) -> String {
    let subject = match &meta.intent {
        Some(intent) if !intent.trim().is_empty() => one_line(intent),
        _ => one_line(pending.source.lines().find(|l| !l.trim().is_empty()).unwrap_or("eval")),
    };
    let subject: String = subject.chars().take(72).collect();
    let mut msg = format!("{subject}\n\nSeq: {}", pending.seq);
    if let Some(u) = &meta.utterance {
        msg.push_str(&format!("\nUtterance: {}", one_line(u)));
    }
    if let Some(t) = &meta.turn {
        msg.push_str(&format!("\nTurn: {}", one_line(t)));
    }
    if let Some(y) = meta.free_y {
        msg.push_str(&format!("\nFree-Y: {y}"));
    }
    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Tmp {
            static N: AtomicU64 = AtomicU64::new(0);
            let p = std::env::temp_dir().join(format!("store-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
            let _ = fs::remove_dir_all(&p);
            Tmp(p)
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Runtime that records what it was asked to evaluate and rejects sources containing "BAD".
    #[derive(Default)]
    struct Fake {
        seen: Vec<String>,
    }
    impl Runtime for Fake {
        fn eval_verify(&mut self, source: &str) -> std::result::Result<(), String> {
            self.seen.push(source.to_string());
            if source.contains("BAD") { Err("OverflowError: 12px too tall".into()) } else { Ok(()) }
        }
        fn snapshot(&mut self) -> String {
            format!("<dom after {}>", self.seen.len())
        }
    }

    fn accept(store: &Store, src: &str, snapshot: &str) -> String {
        let p = store.begin(src).unwrap();
        store.commit(p, snapshot, &Meta::default()).unwrap()
    }

    #[test]
    fn collapse_replaces_the_covered_entries_and_keeps_later_ones() {
        let t = Tmp::new("collapse");
        let store = Store::open(&t.0).unwrap();
        for i in 1..=5 {
            accept(&store, &format!("change {i}"), &format!("<dom {i}>"));
        }
        store.collapse(3, "base of 1..3").unwrap();
        let tail = store.tail().unwrap();
        assert_eq!(tail.iter().map(|e| (e.seq, e.source.as_str())).collect::<Vec<_>>(), vec![(3, "base of 1..3"), (4, "change 4"), (5, "change 5")]);
        assert!(store.git(&["tag", "-l", "cp-3"]).unwrap().contains("cp-3"));
        // new entries still number on from the highest
        assert_eq!(store.begin("next").unwrap().seq, 6);
        // an empty range is an error, not a silent no-op
        let empty = Tmp::new("collapse-empty");
        assert!(Store::open(&empty.0).unwrap().collapse(9, "x").is_err());
    }

    #[test]
    fn open_initializes_a_repo_with_an_empty_journal() {
        let t = Tmp::new("open");
        let s = Store::open(&t.0).unwrap();
        assert!(s.head().is_some());
        assert!(s.tail().unwrap().is_empty());
        assert!(s.pending().unwrap().is_empty());
        assert!(Store::open(&t.0).is_ok(), "reopening is fine");
    }

    #[test]
    fn commit_writes_journal_snapshot_and_trailers() {
        let t = Tmp::new("commit");
        let s = Store::open(&t.0).unwrap();
        let p = s.begin("ui.mount(banner(\"hi\"));").unwrap();
        let meta = Meta { intent: Some("show a banner".into()), utterance: Some("put up\na banner".into()), turn: Some("t7".into()), free_y: Some(38) };
        let hash = s.commit(p, "<banner>", &meta).unwrap();

        assert_eq!(s.tail().unwrap(), vec![Entry { seq: 1, source: "ui.mount(banner(\"hi\"));".into() }]);
        assert_eq!(s.snapshot().as_deref(), Some("<banner>"));
        assert!(s.pending().unwrap().is_empty());

        let msg = s.git(&["log", "-1", "--format=%B"]).unwrap();
        assert!(msg.starts_with("show a banner"), "{msg}");
        for trailer in ["Seq: 1", "Utterance: put up a banner", "Turn: t7", "Free-Y: 38"] {
            assert!(msg.contains(trailer), "missing {trailer} in {msg}");
        }
        assert_eq!(s.head().unwrap(), hash);
    }

    #[test]
    fn abort_commits_nothing_and_seq_never_goes_backwards() {
        let t = Tmp::new("abort");
        let s = Store::open(&t.0).unwrap();
        let head = s.head();
        let p = s.begin("BAD").unwrap();
        assert_eq!(p.seq, 1);
        s.abort(p).unwrap();
        assert_eq!(s.head(), head, "no commit");
        assert!(s.tail().unwrap().is_empty() && s.pending().unwrap().is_empty());
        assert_eq!(s.begin("next").unwrap().seq, 2, "aborted seq is not reused");
    }

    #[test]
    fn undo_steps_back_through_history_and_restores_snapshots() {
        let t = Tmp::new("undo");
        let s = Store::open(&t.0).unwrap();
        accept(&s, "one", "<1>");
        accept(&s, "two", "<2>");
        accept(&s, "three", "<3>");

        assert_eq!(s.undo_last().unwrap(), Some(3));
        assert_eq!(s.snapshot().as_deref(), Some("<2>"));
        assert_eq!(s.undo_last().unwrap(), Some(2), "undo works repeatedly");
        assert_eq!(s.snapshot().as_deref(), Some("<1>"));
        assert_eq!(s.tail().unwrap().iter().map(|e| e.seq).collect::<Vec<_>>(), vec![1]);

        assert_eq!(s.undo_last().unwrap(), Some(1));
        assert_eq!(s.snapshot(), None);
        assert_eq!(s.undo_last().unwrap(), None, "nothing left");

        assert_eq!(s.begin("again").unwrap().seq, 4, "seq continues after undo");
        assert!(s.git(&["log", "--format=%s"]).unwrap().contains("Undo seq 3"), "history is kept");
    }

    #[test]
    fn boot_replay_runs_the_journal_in_order_and_reports_broken_entries() {
        let t = Tmp::new("replay");
        let s = Store::open(&t.0).unwrap();
        accept(&s, "a", "<a>");
        accept(&s, "BAD later", "<b>"); // was fine when written, fails now
        accept(&s, "c", "<c>");

        let mut rt = Fake::default();
        let report = boot_replay(&s, &mut rt).unwrap();
        assert_eq!(rt.seen, vec!["a", "BAD later", "c"], "in order, and the broken one doesn't stop the rest");
        assert_eq!(report.replayed, 2);
        assert_eq!(report.broken.len(), 1);
        assert_eq!(report.broken[0].0, 2);
        assert_eq!(s.tail().unwrap().len(), 3, "broken entries stay in the journal");
    }

    #[test]
    fn a_pending_eval_from_a_crash_is_recovered_or_dropped() {
        let t = Tmp::new("crash");
        {
            let s = Store::open(&t.0).unwrap();
            accept(&s, "a", "<a>");
            s.begin("b-was-in-flight").unwrap(); // process dies here
            s.begin("BAD-was-in-flight").unwrap();
        }
        let s = Store::open(&t.0).unwrap(); // restart
        assert_eq!(s.pending().unwrap().len(), 2);

        let mut rt = Fake::default();
        let report = boot_replay(&s, &mut rt).unwrap();
        assert_eq!(report.replayed, 1);
        assert_eq!(report.recovered, vec![2]);
        assert_eq!(report.dropped, vec![3]);
        assert!(s.pending().unwrap().is_empty());
        assert_eq!(s.tail().unwrap().iter().map(|e| e.seq).collect::<Vec<_>>(), vec![1, 2]);
        assert!(s.git(&["log", "-1", "--format=%s"]).unwrap().contains("recovered pending eval"));
    }
}
