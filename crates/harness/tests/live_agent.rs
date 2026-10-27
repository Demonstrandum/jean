//! Real-model checks, run by hand: `cargo test -p harness --test live_agent -- --ignored --nocapture`.
//! They use the logged-in `claude` program as the model and the display server at JEAN_DISPLAY (default http://127.0.0.1:8788).

use std::path::Path;
use std::time::Duration;

use harness::audit::Audit;
use harness::claude::ClaudeCli;
use harness::display::Display;
use harness::promote::{Canary, Checks, Live, Pipeline};
use harness::sandbox::Backend;
use harness::tasks::{Status, Tasks};
use harness::work::{Approval, Worker};

struct No;
impl Canary for No {
    fn selftest(&self, _: &harness::builder::Worktree) -> Result<(), String> {
        Err("not used".into())
    }
}
impl Live for No {
    fn reload(&self) -> Result<(), String> {
        Ok(())
    }
    fn healthy(&self) -> bool {
        true
    }
}

#[test]
#[ignore]
fn a_real_builder_changes_a_screen_on_its_own_branch() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let display = Display::new(std::env::var("JEAN_DISPLAY").unwrap_or("http://127.0.0.1:8788".into()));
    let root = std::env::temp_dir().join(format!("live-agent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let tasks = Tasks::open(root.join("tasks")).unwrap();
    let audit = Audit::open(root.join("audit.jsonl"));
    let llm = ClaudeCli { bin: "claude".into(), model: None };
    let backend = Backend::detect().expect("a sandbox");
    let pipeline = Pipeline { live_repo: &repo, audit: &audit, backend, checks: Checks(vec![]), canary: &No, live: &No, probes: 0, probe_every: Duration::ZERO, sleep: &|_| {} };
    let w = Worker { llm: &llm, model: "".into(), tasks: &tasks, audit: &audit, repo: &repo, scratch: root.join("trees"), display: &display, backend, pipeline: &pipeline, approval: Approval::Person };
    tasks.enqueue(1, "Retitle the Claude tile", "On the home screen (crates/ui/seed/10-home.ts) the tile with id \"claude\" has title \"Claude\". Change its title to \"Jean\" and change nothing else.").unwrap();
    let r = w.run_next(1).unwrap();
    let t = &tasks.all().unwrap()[0];
    eprintln!("{r:?}\n{}", t.result);
    assert_eq!(r, Some((1, Status::NeedsApproval)), "{}", t.result);
    let diff = std::process::Command::new("git").args(["-C", &repo.display().to_string(), "diff", "--stat", "HEAD...builder/1"]).output().unwrap();
    eprintln!("{}", String::from_utf8_lossy(&diff.stdout));
    let shown = std::process::Command::new("git").args(["-C", &repo.display().to_string(), "show", "builder/1:crates/ui/seed/10-home.ts"]).output().unwrap();
    assert!(String::from_utf8_lossy(&shown.stdout).contains(r#"title: "Jean""#));
    // clean up the branch and the worktree
    let _ = std::process::Command::new("git").args(["-C", &repo.display().to_string(), "worktree", "remove", "--force", &root.join("trees/builder-1").display().to_string()]).status();
    let _ = std::process::Command::new("git").args(["-C", &repo.display().to_string(), "branch", "-D", "builder/1"]).status();
    let _ = std::fs::remove_dir_all(root);
}
