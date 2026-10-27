//! The builder agent's workspace and tools: a git worktree on its own branch, file access confined to it, commands only through the sandbox,
//! and a gate on the finished diff so protected paths cannot change however the change was made.

use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::{json, Value};

use crate::agent::Tools;
use crate::claude::ToolDef;
use crate::display::Display;
use crate::sandbox::{self, Backend, Policy};

/// Paths (and everything under them) the builder may never change: the supervisor, the promotion pipeline, the deployment, the secrets.
pub const PROTECTED: [&str; 9] = ["crates/harness", "crates/store", "nix", "flake.nix", "flake.lock", "secrets", ".git", ".gitattributes", "scripts"];

pub fn is_protected(rel: &Path) -> bool {
    PROTECTED.iter().any(|p| rel.starts_with(p)) || rel.file_name().is_some_and(|n| n == "config.json")
}

/// A repo-relative path from the model's text: no absolute paths, no `..`.
pub fn relative(text: &str) -> Result<PathBuf, String> {
    let p = Path::new(text);
    if p.is_absolute() || p.components().any(|c| matches!(c, Component::ParentDir | Component::Prefix(_))) || text.trim().is_empty() {
        return Err(format!("`{text}` is not a path inside the project"));
    }
    Ok(p.components().filter(|c| !matches!(c, Component::CurDir)).collect())
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().map_err(|e| format!("git: {e}"))?;
    if out.status.success() { Ok(String::from_utf8_lossy(&out.stdout).trim().to_string()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

/// A checkout of the project for one task.
pub struct Worktree {
    pub repo: PathBuf,
    pub dir: PathBuf,
    pub branch: String,
    pub base: String,
}

impl Worktree {
    /// A new worktree at `dir` on a new branch `builder/<id>`, from the repo's current commit.
    pub fn create(repo: &Path, dir: &Path, id: u64) -> Result<Worktree, String> {
        let branch = format!("builder/{id}");
        let base = git(repo, &["rev-parse", "HEAD"])?;
        git(repo, &["worktree", "add", "-q", "-b", &branch, &dir.display().to_string(), &base])?;
        Ok(Worktree { repo: repo.to_path_buf(), dir: dir.to_path_buf(), branch, base })
    }

    /// Everything changed since the base (committed or not), as repo-relative paths.
    pub fn changed(&self) -> Result<Vec<PathBuf>, String> {
        git(&self.dir, &["add", "-A"])?;
        let names = git(&self.dir, &["diff", "--cached", "--name-only", &self.base])?;
        Ok(names.lines().map(PathBuf::from).collect())
    }

    /// The changed paths that are protected: a non-empty answer means the change must not go ahead.
    pub fn violations(&self) -> Result<Vec<PathBuf>, String> {
        Ok(self.changed()?.into_iter().filter(|p| is_protected(p)).collect())
    }

    /// Commit everything on the branch; the commit id.
    pub fn commit(&self, message: &str) -> Result<String, String> {
        git(&self.dir, &["add", "-A"])?;
        git(&self.dir, &["-c", "user.name=jean-builder", "-c", "user.email=builder@jean.invalid", "commit", "-q", "--allow-empty", "-m", message])?;
        git(&self.dir, &["rev-parse", "HEAD"])
    }

    pub fn remove(&self) -> Result<(), String> {
        git(&self.repo, &["worktree", "remove", "--force", &self.dir.display().to_string()]).map(|_| ())
    }
}

/// How a builder run ended: set by the `finish` tool.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Finished(pub Option<String>);

pub struct BuilderTools<'a> {
    pub tree: &'a Worktree,
    pub display: &'a Display,
    pub backend: Backend,
    pub finished: Finished,
}

fn def(name: &str, description: &str, props: Value, required: &[&str]) -> ToolDef {
    ToolDef { name: name.into(), description: description.into(), input_schema: json!({ "type": "object", "properties": props, "required": required }) }
}

fn arg<'v>(input: &'v Value, key: &str) -> Result<&'v str, String> {
    input[key].as_str().ok_or_else(|| format!("missing text argument `{key}`"))
}

const MAX_READ: usize = 60_000;

impl BuilderTools<'_> {
    /// A path inside the worktree that exists or may be created, with no symlink leading out.
    fn inside(&self, text: &str) -> Result<(PathBuf, PathBuf), String> {
        let rel = relative(text)?;
        let full = self.tree.dir.join(&rel);
        let root = std::fs::canonicalize(&self.tree.dir).map_err(|e| e.to_string())?;
        let mut probe = full.clone();
        while !probe.exists() {
            probe = probe.parent().ok_or("bad path")?.to_path_buf();
        }
        if !std::fs::canonicalize(&probe).map_err(|e| e.to_string())?.starts_with(&root) {
            return Err(format!("`{text}` leads outside the project"));
        }
        Ok((rel, full))
    }
}

impl Tools for BuilderTools<'_> {
    fn defs(&self) -> Vec<ToolDef> {
        vec![
            def("list_dir", "List a directory of the project (path relative to the project root; `.` for the root).", json!({ "path": { "type": "string" } }), &["path"]),
            def("read_file", "Read a text file of the project.", json!({ "path": { "type": "string" } }), &["path"]),
            def("write_file", "Create or replace a file. Protected paths (the harness, the store, nix, secrets, scripts) are refused.", json!({ "path": { "type": "string" }, "content": { "type": "string" } }), &["path", "content"]),
            def("run", "Run a command in the project, in a sandbox: it can write only inside the project and has no network. Give the program and its arguments as a list, not a shell string.", json!({ "argv": { "type": "array", "items": { "type": "string" } }, "timeout_s": { "type": "integer" } }), &["argv"]),
            def("look_up", "Search the display library's reference.", json!({ "query": { "type": "string" } }), &["query"]),
            def("selftest", "Show every screen with sample data and report how each fits.", json!({}), &[]),
            def("finish", "Say the work is done. The change will be checked, and a person or the policy decides whether it goes live.", json!({ "summary": { "type": "string" } }), &["summary"]),
        ]
    }

    fn call(&mut self, name: &str, input: &Value) -> Result<String, String> {
        match name {
            "list_dir" => {
                let (_, full) = self.inside(arg(input, "path")?)?;
                let mut names: Vec<String> = std::fs::read_dir(&full)
                    .map_err(|e| e.to_string())?
                    .flatten()
                    .filter(|e| e.file_name() != ".git")
                    .map(|e| format!("{}{}", e.file_name().to_string_lossy(), if e.path().is_dir() { "/" } else { "" }))
                    .collect();
                names.sort();
                Ok(names.join("\n"))
            }
            "read_file" => {
                let (_, full) = self.inside(arg(input, "path")?)?;
                let text = std::fs::read_to_string(&full).map_err(|e| e.to_string())?;
                if text.len() > MAX_READ {
                    let cut = (0..=MAX_READ).rev().find(|&i| text.is_char_boundary(i)).unwrap_or(0);
                    return Ok(format!("{}\n[... {} more bytes not shown]", &text[..cut], text.len() - cut));
                }
                Ok(text)
            }
            "write_file" => {
                let (rel, full) = self.inside(arg(input, "path")?)?;
                if is_protected(&rel) {
                    return Err(format!("`{}` is protected and cannot be changed by the builder", rel.display()));
                }
                if let Some(parent) = full.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let content = arg(input, "content")?;
                std::fs::write(&full, content).map_err(|e| e.to_string())?;
                Ok(format!("wrote {} bytes to {}", content.len(), rel.display()))
            }
            "run" => {
                let argv: Vec<&str> = input["argv"].as_array().ok_or("`argv` must be a list")?.iter().filter_map(Value::as_str).collect();
                if argv.is_empty() {
                    return Err("`argv` is empty".into());
                }
                let timeout = Duration::from_secs(input["timeout_s"].as_u64().unwrap_or(120).min(600));
                let policy = Policy { workdir: self.tree.dir.clone(), writable: vec![], network: false, timeout };
                let r = sandbox::run(self.backend, &policy, &argv)?;
                let mut out = format!("exit {}{}\n{}", r.code.map_or("none".to_string(), |c| c.to_string()), if r.timed_out { " (timed out)" } else { "" }, r.stdout);
                if !r.stderr.is_empty() {
                    out.push_str(&format!("\n[stderr]\n{}", r.stderr));
                }
                let out = if out.len() > MAX_READ { format!("{}\n[... truncated]", &out[..(0..=MAX_READ).rev().find(|&i| out.is_char_boundary(i)).unwrap_or(0)]) } else { out };
                if r.ok() { Ok(out) } else { Err(out) }
            }
            "look_up" => self.display.docs(arg(input, "query")?).map(|v| v.to_string()),
            "selftest" => self.display.selftest().map(|v| v.to_string()),
            "finish" => {
                self.finished = Finished(Some(arg(input, "summary")?.to_string()));
                Ok("recorded; the change will now be checked".into())
            }
            other => Err(format!("no tool `{other}`")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::testing::mock;

    fn repo(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bld-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("crates/ui/seed")).unwrap();
        std::fs::create_dir_all(d.join("secrets")).unwrap();
        std::fs::write(d.join("crates/ui/seed/10-home.ts"), "// home\n").unwrap();
        std::fs::write(d.join("secrets/key"), "s3cret\n").unwrap();
        for a in [&["init", "-q"][..], &["add", "-A"], &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "base"]] {
            git(&d, a).unwrap();
        }
        d
    }

    #[test]
    fn paths_are_relative_and_protected_ones_are_named() {
        assert!(relative("../etc/passwd").is_err() && relative("/etc/passwd").is_err() && relative("").is_err());
        assert_eq!(relative("./a/b.ts").unwrap(), PathBuf::from("a/b.ts"));
        for p in ["crates/harness/src/lib.rs", "nix/module.nix", "flake.nix", "secrets/font.key", "scripts/selftest.sh", "feeds/config.json", ".git/config"] {
            assert!(is_protected(Path::new(p)), "{p}");
        }
        for p in ["crates/ui/seed/80-garden.ts", "feeds/garden.py", "crates/ui/ts/schema.ts"] {
            assert!(!is_protected(Path::new(p)), "{p}");
        }
    }

    #[test]
    fn the_builder_can_write_a_screen_but_not_a_protected_file_and_cannot_escape() {
        let m = mock();
        let display = Display::new(&m.base);
        let repo = repo("tools");
        let dir = std::env::temp_dir().join(format!("bld-tools-wt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let tree = Worktree::create(&repo, &dir, 7).unwrap();
        let Some(backend) = Backend::detect() else { return };
        let mut t = BuilderTools { tree: &tree, display: &display, backend, finished: Finished::default() };
        assert!(t.call("write_file", &json!({ "path": "crates/ui/seed/80-garden.ts", "content": "ui.defineScreen({})\n" })).is_ok());
        assert!(t.call("write_file", &json!({ "path": "secrets/key", "content": "x" })).unwrap_err().contains("protected"));
        assert!(t.call("write_file", &json!({ "path": "../outside.txt", "content": "x" })).is_err());
        assert!(t.call("read_file", &json!({ "path": "/etc/hosts" })).is_err());
        // a symlink out of the tree is refused
        std::os::unix::fs::symlink("/etc", dir.join("link")).unwrap();
        assert!(t.call("read_file", &json!({ "path": "link/hosts" })).unwrap_err().contains("outside"));
        assert_eq!(t.call("list_dir", &json!({ "path": "crates/ui/seed" })).unwrap(), "10-home.ts\n80-garden.ts");
        // commands run confined: they cannot touch secrets by writing outside, and a protected file changed through `run` is caught by the diff gate
        assert!(t.call("run", &json!({ "argv": ["sh", "-c", "echo hacked > secrets/key"] })).is_ok());
        let bad = tree.violations().unwrap();
        assert_eq!(bad, vec![PathBuf::from("secrets/key")]);
        assert!(t.call("finish", &json!({ "summary": "garden screen" })).is_ok());
        assert_eq!(t.finished.0.as_deref(), Some("garden screen"));
        tree.remove().unwrap();
        let _ = std::fs::remove_dir_all(repo);
    }

    #[test]
    fn a_clean_change_commits_on_its_own_branch() {
        let repo = repo("commit");
        let dir = std::env::temp_dir().join(format!("bld-commit-wt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let tree = Worktree::create(&repo, &dir, 3).unwrap();
        std::fs::write(dir.join("crates/ui/seed/80-garden.ts"), "// garden\n").unwrap();
        assert_eq!(tree.changed().unwrap(), vec![PathBuf::from("crates/ui/seed/80-garden.ts")]);
        assert!(tree.violations().unwrap().is_empty());
        let id = tree.commit("builder: garden").unwrap();
        assert_eq!(git(&repo, &["rev-parse", "builder/3"]).unwrap(), id);
        assert_ne!(id, tree.base);
        tree.remove().unwrap();
        let _ = std::fs::remove_dir_all(repo);
    }
}
