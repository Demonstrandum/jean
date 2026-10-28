//! Getting a builder's branch into the live house: gate, checks, canary, swap, watchdog. Any failure stops it; a failure after the swap undoes it.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::audit::{Action, Actor, Audit};
use crate::builder::Worktree;
use crate::sandbox::{self, Backend, Policy};

/// A second copy of the display, running the candidate, that nobody looks at.
pub trait Canary {
    /// Start it from the worktree; its self-test result.
    fn selftest(&self, tree: &Worktree) -> Result<(), String>;
}

/// The live house: how to make it pick up a new commit, and whether it is well.
pub trait Live {
    fn reload(&self) -> Result<(), String>;
    fn healthy(&self) -> bool;
}

/// Commands that must pass in the candidate's worktree before it is tried: `argv` lists, run in the sandbox.
#[derive(Clone, Debug)]
pub struct Checks(pub Vec<Vec<String>>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// It is live. Carries the commit it replaced, to go back to.
    Promoted { commit: String, previous: String },
    /// It never went live; why.
    Rejected(String),
    /// It went live and was taken back; why.
    RolledBack(String),
}

pub struct Pipeline<'a> {
    pub live_repo: &'a Path,
    pub audit: &'a Audit,
    pub backend: Backend,
    pub checks: Checks,
    pub canary: &'a dyn Canary,
    pub live: &'a dyn Live,
    /// Health probes after the swap, and the wait between them.
    pub probes: usize,
    pub probe_every: Duration,
    pub sleep: &'a dyn Fn(Duration),
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok(String::from_utf8_lossy(&out.stdout).trim().to_string()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

impl Pipeline<'_> {
    fn reject(&self, now: u64, branch: &str, why: String) -> Result<Outcome, String> {
        self.audit.append(now, Actor::Pipeline, Action::ChangeRejected, &format!("{branch}: {why}"))?;
        Ok(Outcome::Rejected(why))
    }

    /// Take `tree` (already committed) live, if it deserves to be.
    pub fn promote(&self, tree: &Worktree, now_ms: u64) -> Result<Outcome, String> {
        let branch = tree.branch.as_str();
        let bad = tree.violations()?;
        if !bad.is_empty() {
            let names: Vec<String> = bad.iter().map(|p| p.display().to_string()).collect();
            return self.reject(now_ms, branch, format!("touches protected paths: {}", names.join(", ")));
        }
        for argv in &self.checks.0 {
            let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
            let policy = Policy { workdir: tree.dir.clone(), writable: vec![], network: false, loopback: true, timeout: Duration::from_secs(900) };
            let r = sandbox::run(self.backend, &policy, &argv)?;
            if !r.ok() {
                let tail: String = r.stderr.lines().rev().take(8).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
                return self.reject(now_ms, branch, format!("check `{}` failed: {tail}", argv.join(" ")));
            }
        }
        if let Err(e) = self.canary.selftest(tree) {
            self.audit.append(now_ms, Actor::Pipeline, Action::CanaryFailed, &format!("{branch}: {e}"))?;
            return Ok(Outcome::Rejected(format!("canary: {e}")));
        }
        self.audit.append(now_ms, Actor::Pipeline, Action::CanaryPassed, branch)?;

        if !git(self.live_repo, &["status", "--porcelain", "--untracked-files=no"])?.is_empty() {
            return self.reject(now_ms, branch, "the live checkout has uncommitted changes".into());
        }
        let previous = git(self.live_repo, &["rev-parse", "HEAD"])?;
        if let Err(e) = git(self.live_repo, &["merge", "--ff-only", "-q", branch]) {
            return self.reject(now_ms, branch, format!("cannot fast-forward: {e}"));
        }
        let commit = git(self.live_repo, &["rev-parse", "HEAD"])?;
        self.audit.append(now_ms, Actor::Pipeline, Action::Promoted, &format!("{branch} {previous}..{commit}"))?;

        let mut why = self.live.reload().err().map(|e| format!("reload failed: {e}"));
        for _ in 0..self.probes {
            if why.is_some() {
                break;
            }
            (self.sleep)(self.probe_every);
            if !self.live.healthy() {
                why = Some("the display stopped answering after the change".into());
            }
        }
        if let Some(why) = why {
            git(self.live_repo, &["reset", "--hard", "-q", &previous])?;
            let _ = self.live.reload();
            self.audit.append(now_ms, Actor::Watchdog, Action::RolledBack, &format!("{branch}: {why}; back to {previous}"))?;
            return Ok(Outcome::RolledBack(why));
        }
        Ok(Outcome::Promoted { commit, previous })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;

    struct FakeCanary(Result<(), String>);
    impl Canary for FakeCanary {
        fn selftest(&self, _: &Worktree) -> Result<(), String> {
            self.0.clone()
        }
    }

    /// Healthy for `ok` probes, then not (so a change can break the house after it has gone live).
    struct FakeLive {
        ok: Cell<i32>,
        reloads: RefCell<u32>,
    }
    impl Live for FakeLive {
        fn reload(&self) -> Result<(), String> {
            *self.reloads.borrow_mut() += 1;
            Ok(())
        }
        fn healthy(&self) -> bool {
            self.ok.set(self.ok.get() - 1);
            self.ok.get() >= 0
        }
    }

    struct Rig {
        repo: PathBuf,
        tree: Worktree,
        audit: Audit,
    }

    fn rig(name: &str, file: &str) -> Rig {
        let repo = std::env::temp_dir().join(format!("pro-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(repo.join("crates/ui/seed")).unwrap();
        std::fs::create_dir_all(repo.join("secrets")).unwrap();
        std::fs::write(repo.join("crates/ui/seed/10-home.ts"), "//\n").unwrap();
        std::fs::write(repo.join("secrets/key"), "k\n").unwrap();
        for a in [&["init", "-q", "-b", "main"][..], &["add", "-A"], &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "base"]] {
            git(&repo, a).unwrap();
        }
        let dir = std::env::temp_dir().join(format!("pro-{name}-wt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let tree = Worktree::create(&repo, &dir, 1).unwrap();
        std::fs::write(dir.join(file), "// change\n").unwrap();
        tree.commit("builder change").unwrap();
        let log = std::env::temp_dir().join(format!("pro-{name}-{}.audit", std::process::id()));
        let _ = std::fs::remove_file(&log);
        let audit = Audit::open(log);
        Rig { repo, tree, audit }
    }

    fn pipeline<'a>(r: &'a Rig, canary: &'a dyn Canary, live: &'a dyn Live, sleep: &'a dyn Fn(Duration)) -> Option<Pipeline<'a>> {
        Some(Pipeline { live_repo: &r.repo, audit: &r.audit, backend: Backend::detect()?, checks: Checks(vec![vec!["true".into()]]), canary, live, probes: 3, probe_every: Duration::from_secs(1), sleep })
    }

    fn head(r: &Rig) -> String {
        git(&r.repo, &["rev-parse", "HEAD"]).unwrap()
    }

    #[test]
    fn a_good_change_goes_live_and_is_audited() {
        let r = rig("good", "crates/ui/seed/80-garden.ts");
        let live = FakeLive { ok: Cell::new(99), reloads: RefCell::new(0) };
        let Some(p) = pipeline(&r, &FakeCanary(Ok(())), &live, &|_| {}) else { return };
        let before = head(&r);
        let out = p.promote(&r.tree, 100).unwrap();
        assert!(matches!(&out, Outcome::Promoted { previous, .. } if *previous == before), "{out:?}");
        assert_ne!(head(&r), before);
        assert!(r.repo.join("crates/ui/seed/80-garden.ts").exists());
        assert_eq!(*live.reloads.borrow(), 1);
        let kinds: Vec<Action> = r.audit.entries().unwrap().iter().map(|e| e.action).collect();
        assert_eq!(kinds, vec![Action::CanaryPassed, Action::Promoted]);
    }

    #[test]
    fn a_protected_path_is_rejected_before_anything_runs() {
        let r = rig("prot", "secrets/key");
        let live = FakeLive { ok: Cell::new(99), reloads: RefCell::new(0) };
        let Some(p) = pipeline(&r, &FakeCanary(Ok(())), &live, &|_| {}) else { return };
        let before = head(&r);
        assert!(matches!(p.promote(&r.tree, 1).unwrap(), Outcome::Rejected(w) if w.contains("secrets/key")));
        assert_eq!(head(&r), before);
        assert_eq!(r.audit.entries().unwrap()[0].action, Action::ChangeRejected);
    }

    #[test]
    fn a_failing_check_or_canary_stops_it() {
        let r = rig("fail", "crates/ui/seed/80-garden.ts");
        let live = FakeLive { ok: Cell::new(99), reloads: RefCell::new(0) };
        let Some(mut p) = pipeline(&r, &FakeCanary(Ok(())), &live, &|_| {}) else { return };
        p.checks = Checks(vec![vec!["sh".into(), "-c".into(), "echo nope >&2; exit 3".into()]]);
        assert!(matches!(p.promote(&r.tree, 1).unwrap(), Outcome::Rejected(w) if w.contains("nope")));
        let canary = FakeCanary(Err("home overflows".into()));
        let p = pipeline(&r, &canary, &live, &|_| {}).unwrap();
        assert!(matches!(p.promote(&r.tree, 2).unwrap(), Outcome::Rejected(w) if w.contains("home overflows")));
        assert!(!r.repo.join("crates/ui/seed/80-garden.ts").exists());
        assert!(r.audit.entries().unwrap().iter().any(|e| e.action == Action::CanaryFailed));
    }

    #[test]
    fn a_change_that_breaks_the_house_is_rolled_back() {
        let r = rig("dog", "crates/ui/seed/80-garden.ts");
        // healthy for one probe, then dead
        let live = FakeLive { ok: Cell::new(1), reloads: RefCell::new(0) };
        let Some(p) = pipeline(&r, &FakeCanary(Ok(())), &live, &|_| {}) else { return };
        let before = head(&r);
        assert!(matches!(p.promote(&r.tree, 5).unwrap(), Outcome::RolledBack(_)));
        assert_eq!(head(&r), before);
        assert!(!r.repo.join("crates/ui/seed/80-garden.ts").exists());
        assert_eq!(*live.reloads.borrow(), 2, "reloaded after the swap and again after the rollback");
        assert_eq!(r.audit.entries().unwrap().last().unwrap().action, Action::RolledBack);
        assert_eq!(r.audit.verify().unwrap(), Ok(3));
    }
}
