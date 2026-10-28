//! The builder's working day: take the oldest queued task, give it a worktree, let the builder agent work, gate and commit what it made,
//! then hand it to the promotion pipeline (or leave it for a person to approve).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::agent::{Agent, Profile};
use crate::audit::{Action, Actor, Audit};
use crate::builder::{BuilderTools, Finished, Worktree};
use crate::claude::Llm;
use crate::display::Display;
use crate::prompts;
use crate::promote::{Canary, Live, Outcome, Pipeline};
use crate::sandbox::Backend;
use crate::tasks::{Status, Task, Tasks};

/// Whether a finished change goes live by itself or waits to be approved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Approval {
    Auto,
    Person,
}

/// How many times a builder that stops without calling `finish` is asked to finish or carry on.
const NUDGES: usize = 2;

pub struct Worker<'a> {
    pub llm: &'a dyn Llm,
    pub model: String,
    pub tasks: &'a Tasks,
    pub audit: &'a Audit,
    pub repo: &'a Path,
    /// Where worktrees are made.
    pub scratch: PathBuf,
    pub display: &'a Display,
    pub backend: Backend,
    pub pipeline: &'a Pipeline<'a>,
    pub approval: Approval,
}

impl Worker<'_> {
    fn tree_dir(&self, id: u64) -> PathBuf {
        self.scratch.join(format!("builder-{id}"))
    }

    /// Do the oldest queued task, if there is one. What became of it.
    pub fn run_next(&self, now_ms: u64) -> Result<Option<(u64, Status)>, String> {
        let Some(task) = self.tasks.next_queued()? else { return Ok(None) };
        self.tasks.set(task.id, Status::Running, "")?;
        self.audit.append(now_ms, Actor::Builder, Action::BuildStarted, &format!("#{} {}", task.id, task.title))?;
        let status = match self.build(&task, now_ms) {
            Ok((status, note)) => {
                self.tasks.set(task.id, status, &note)?;
                status
            }
            Err(e) => {
                self.tasks.set(task.id, Status::Failed, &e)?;
                Status::Failed
            }
        };
        Ok(Some((task.id, status)))
    }

    fn build(&self, task: &Task, now_ms: u64) -> Result<(Status, String), String> {
        let tree = Worktree::create(self.repo, &self.tree_dir(task.id), task.id)?;
        let profile = Profile { model: self.model.clone(), system: prompts::builder_system(&task.title, &task.details), max_tokens: 8000, max_steps: 60 };
        // the Claude tile turns black with the job's title and narrates the builder's tool calls
        let activity = std::cell::RefCell::new(crate::activity::Activity::default());
        let feed = crate::activity::Feed { display: self.display, activity: &activity };
        let _ = self.display.show_work(Some(&task.title), now_ms);
        feed.publish();
        let agent = Agent { llm: self.llm, profile, watch: Some(&feed) };
        let mut tools = BuilderTools { tree: &tree, display: self.display, backend: self.backend, finished: Finished::default() };
        // A model sometimes ends its turn with a closing sentence instead of calling `finish`: its work is not lost for that, it is asked to finish properly
        // (or to carry on if it is not done), twice at most.
        let mut history = vec![];
        let mut outcome = agent.run_turn(&mut tools, &mut history, &format!("{}\n\n{}", task.title, task.details));
        for _ in 0..NUDGES {
            let stopped_early = matches!(&outcome, Ok(o) if !o.truncated) && tools.finished.0.is_none();
            if !stopped_early {
                break;
            }
            outcome = agent.run_turn(&mut tools, &mut history, "You stopped without calling finish. If the task is done, run the self-test and call finish with a summary of what you changed; if it is not done, carry on.");
        }
        activity.borrow_mut().clear();
        feed.publish();
        let _ = self.display.show_work(None, now_ms);
        let result = (|| {
            outcome?;
            let Some(summary) = tools.finished.0.clone() else { return Ok((Status::Failed, "the builder stopped without finishing".to_string())) };
            tree.commit(&format!("builder #{}: {}\n\n{}", task.id, task.title, summary))?;
            self.audit.append(now_ms, Actor::Builder, Action::BuildFinished, &format!("#{} {} on {}", task.id, task.title, tree.branch))?;
            match self.approval {
                Approval::Person => Ok((Status::NeedsApproval, format!("branch {}: {summary}", tree.branch))),
                Approval::Auto => self.finish_promotion(&tree, now_ms, &summary),
            }
        })();
        // The branch outlives the worktree; the worktree is only a workspace.
        if self.approval == Approval::Auto || result.is_err() || matches!(&result, Ok((Status::Failed, _))) {
            let _ = tree.remove();
        }
        result
    }

    fn finish_promotion(&self, tree: &Worktree, now_ms: u64, summary: &str) -> Result<(Status, String), String> {
        Ok(match self.pipeline.promote(tree, now_ms)? {
            Outcome::Promoted { commit, .. } => (Status::Done, format!("live at {commit}: {summary}")),
            Outcome::Rejected(why) => (Status::Failed, format!("not promoted: {why}")),
            Outcome::RolledBack(why) => (Status::Failed, format!("rolled back: {why}")),
        })
    }

    /// A person said yes to a task waiting for approval: promote its branch.
    pub fn approve(&self, id: u64, now_ms: u64) -> Result<Status, String> {
        let task = self.tasks.all()?.into_iter().find(|t| t.id == id && t.status == Status::NeedsApproval).ok_or("no such task waiting for approval")?;
        self.audit.append(now_ms, Actor::Person, Action::Approved, &format!("#{} {}", task.id, task.title))?;
        let dir = self.tree_dir(id);
        let tree = Worktree { repo: self.repo.to_path_buf(), dir, branch: format!("builder/{id}"), base: git_out(self.repo, &["merge-base", "HEAD", &format!("builder/{id}")])? };
        let (status, note) = self.finish_promotion(&tree, now_ms, &task.title)?;
        self.tasks.set(id, status, &note)?;
        let _ = tree.remove();
        Ok(status)
    }
}

fn git_out(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).envs([("GIT_CONFIG_COUNT", "2"), ("GIT_CONFIG_KEY_0", "commit.gpgsign"), ("GIT_CONFIG_VALUE_0", "false"), ("GIT_CONFIG_KEY_1", "tag.gpgsign"), ("GIT_CONFIG_VALUE_1", "false")]).output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok(String::from_utf8_lossy(&out.stdout).trim().to_string()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

/// The canary is the project's own self-test script, run from the candidate's worktree (so it tests the candidate).
pub struct ScriptCanary;

impl Canary for ScriptCanary {
    fn selftest(&self, tree: &Worktree) -> Result<(), String> {
        let out = Command::new("sh").arg(tree.dir.join("scripts/selftest.sh")).current_dir(&tree.dir).output().map_err(|e| e.to_string())?;
        match out.status.code() {
            Some(0) => Ok(()),
            Some(2) => Err("no browser here to run the self-test".into()),
            _ => Err(format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).lines().rev().take(12).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")),
        }
    }
}

/// The live display: reloading runs a restart command (`systemctl restart jean-display`), health is the display answering with a page connected.
pub struct CommandLive<'a> {
    pub display: &'a Display,
    pub restart: Vec<String>,
}

impl Live for CommandLive<'_> {
    fn reload(&self) -> Result<(), String> {
        let Some((bin, args)) = self.restart.split_first() else { return Ok(()) };
        let st = Command::new(bin).args(args).status().map_err(|e| format!("{bin}: {e}"))?;
        if st.success() { Ok(()) } else { Err(format!("{bin} failed")) }
    }
    fn healthy(&self) -> bool {
        self.display.healthy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::testing::{call, say, Script};
    use crate::display::testing::mock;
    use crate::promote::Checks;
    use serde_json::json;
    use std::time::Duration;

    struct Ok_;
    impl Canary for Ok_ {
        fn selftest(&self, _: &Worktree) -> Result<(), String> {
            Ok(())
        }
    }
    impl Live for Ok_ {
        fn reload(&self) -> Result<(), String> {
            Ok(())
        }
        fn healthy(&self) -> bool {
            true
        }
    }

    fn repo(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("wrk-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("crates/ui/seed")).unwrap();
        std::fs::write(d.join("crates/ui/seed/10-home.ts"), "//\n").unwrap();
        for a in [&["init", "-q", "-b", "main"][..], &["add", "-A"], &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "base"]] {
            git_out(&d, a).unwrap();
        }
        d
    }

    fn rig(name: &str, approval: Approval, script: Vec<crate::claude::Response>, f: impl FnOnce(&Worker, &PathBuf, &Tasks, &Audit)) {
        let Some(backend) = Backend::detect() else { return };
        let m = mock();
        let display = Display::new(&m.base);
        let repo = repo(name);
        let root = std::env::temp_dir().join(format!("wrk-{name}-state-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let tasks = Tasks::open(root.join("tasks")).unwrap();
        let audit = Audit::open(root.join("audit.jsonl"));
        let llm = Script::new(script);
        let pipeline = Pipeline { live_repo: &repo, audit: &audit, backend, checks: Checks(vec![]), canary: &Ok_, live: &Ok_, probes: 1, probe_every: Duration::ZERO, sleep: &|_| {} };
        let w = Worker { llm: &llm, model: "m".into(), tasks: &tasks, audit: &audit, repo: &repo, scratch: root.join("trees"), display: &display, backend, pipeline: &pipeline, approval };
        f(&w, &repo, &tasks, &audit);
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(repo);
    }

    fn garden() -> Vec<crate::claude::Response> {
        vec![call("a", "write_file", json!({ "path": "crates/ui/seed/80-garden.ts", "content": "// garden\n" })), call("b", "finish", json!({ "summary": "added a garden screen" })), say("done")]
    }

    #[test]
    fn an_automatic_task_is_built_promoted_and_audited() {
        rig("auto", Approval::Auto, garden(), |w, repo, tasks, audit| {
            w.tasks.enqueue(1, "Add a garden screen", "soil moisture").unwrap();
            assert_eq!(w.run_next(10).unwrap(), Some((1, Status::Done)));
            assert!(repo.join("crates/ui/seed/80-garden.ts").exists());
            assert!(tasks.all().unwrap()[0].result.starts_with("live at"));
            let kinds: Vec<Action> = audit.entries().unwrap().iter().map(|e| e.action).collect();
            assert_eq!(kinds, vec![Action::BuildStarted, Action::BuildFinished, Action::CanaryPassed, Action::Promoted]);
            assert_eq!(w.run_next(11).unwrap(), None);
        });
    }

    #[test]
    fn a_task_waits_for_approval_and_goes_live_when_approved() {
        rig("person", Approval::Person, garden(), |w, repo, tasks, audit| {
            w.tasks.enqueue(1, "Add a garden screen", "").unwrap();
            assert_eq!(w.run_next(10).unwrap(), Some((1, Status::NeedsApproval)));
            assert!(!repo.join("crates/ui/seed/80-garden.ts").exists());
            assert!(tasks.all().unwrap()[0].result.contains("builder/1"));
            assert_eq!(w.approve(1, 20).unwrap(), Status::Done);
            assert!(repo.join("crates/ui/seed/80-garden.ts").exists());
            assert!(audit.entries().unwrap().iter().any(|e| e.action == Action::Approved && e.actor == Actor::Person));
            assert!(w.approve(1, 30).is_err(), "approving twice is refused");
        });
    }

    #[test]
    fn a_builder_that_never_finishes_fails_and_changes_nothing() {
        // it is asked twice to finish or carry on, and does not
        rig("quit", Approval::Auto, vec![call("a", "write_file", json!({ "path": "x.ts", "content": "" })), say("I give up"), say("No, really"), say("Really")], |w, repo, tasks, _| {
            w.tasks.enqueue(1, "Something", "").unwrap();
            assert_eq!(w.run_next(10).unwrap(), Some((1, Status::Failed)));
            assert!(!repo.join("x.ts").exists());
            assert!(tasks.all().unwrap()[0].result.contains("without finishing"));
        });
    }

    #[test]
    fn a_builder_that_stops_with_a_sentence_is_asked_to_finish_and_its_work_is_kept() {
        // the model writes the file and a closing sentence, no `finish`; it is nudged, calls it, and the change goes through
        let script = vec![call("a", "write_file", json!({ "path": "crates/ui/seed/80-garden.ts", "content": "// garden\n" })), say("All done, the garden screen is added."), call("b", "finish", json!({ "summary": "added a garden screen" })), say("Finished.")];
        rig("nudge", Approval::Person, script, |w, repo, tasks, _| {
            w.tasks.enqueue(1, "Add a garden screen", "").unwrap();
            assert_eq!(w.run_next(10).unwrap(), Some((1, Status::NeedsApproval)), "{}", tasks.all().unwrap()[0].result);
            assert!(tasks.all().unwrap()[0].result.contains("builder/1"));
            let _ = repo;
        });
    }

    #[test]
    fn a_builder_that_edits_a_protected_file_through_a_command_is_rejected() {
        let script = vec![call("a", "run", json!({ "argv": ["sh", "-c", "mkdir -p nix && echo x > nix/module.nix"] })), call("b", "finish", json!({ "summary": "sneaky" })), say("done")];
        rig("sneak", Approval::Auto, script, |w, repo, tasks, audit| {
            w.tasks.enqueue(1, "Sneak", "").unwrap();
            assert_eq!(w.run_next(10).unwrap(), Some((1, Status::Failed)));
            assert!(!repo.join("nix").exists());
            assert!(tasks.all().unwrap()[0].result.contains("protected"));
            assert!(audit.entries().unwrap().iter().any(|e| e.action == Action::ChangeRejected));
        });
    }
}
