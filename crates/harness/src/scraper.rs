//! Scrapers that mend themselves, without being able to lie about it. A scraper's manifest declares the typed keys it must produce and the
//! saved pages (fixtures) it must parse. When a live page stops parsing, the page is saved as a new fixture and a repair task is queued;
//! a repair goes live only if the old fixtures still give exactly their recorded answers, the new page gives valid ones,
//! and the manifest and old fixtures were not touched.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::audit::{Action, Actor, Audit};
use crate::sandbox::{self, Backend, Policy};
use crate::tasks::{Task, Tasks};

/// What a produced value must be.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Kind {
    Number { min: f64, max: f64 },
    Text { max_len: usize },
    /// Epoch milliseconds, within a plausible span (2020 to 2100).
    Instant,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeySpec {
    pub key: String,
    #[serde(flatten)]
    pub kind: Kind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fixture {
    /// The saved page, relative to the manifest's directory.
    pub input: String,
    /// The answer it must give exactly (a JSON object), if one has been recorded. Without one, the answer need only be valid.
    #[serde(default)]
    pub expected: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    /// The page to fetch.
    pub url: String,
    /// The program and arguments that parse a saved page: the page's path is appended, and a JSON object of key to value is printed.
    pub parse: Vec<String>,
    pub keys: Vec<KeySpec>,
    pub fixtures: Vec<Fixture>,
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Manifest, String> {
        serde_json::from_str(&std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// What is wrong with a parsed answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    Missing(String),
    WrongType(String),
    OutOfRange(String),
    Unexpected(String),
}

/// An answer against the manifest's key specs: every declared key present, of its type and in its range, and no others.
pub fn validate(m: &Manifest, answer: &Value) -> Vec<Problem> {
    let Some(obj) = answer.as_object() else { return vec![Problem::WrongType("the answer is not an object".into())] };
    let mut out = Vec::new();
    for spec in &m.keys {
        let Some(v) = obj.get(&spec.key) else {
            out.push(Problem::Missing(spec.key.clone()));
            continue;
        };
        match spec.kind {
            Kind::Number { min, max } => match v.as_f64() {
                None => out.push(Problem::WrongType(spec.key.clone())),
                Some(n) if !(min..=max).contains(&n) => out.push(Problem::OutOfRange(spec.key.clone())),
                Some(_) => {}
            },
            Kind::Text { max_len } => match v.as_str() {
                None => out.push(Problem::WrongType(spec.key.clone())),
                Some(s) if s.is_empty() || s.chars().count() > max_len => out.push(Problem::OutOfRange(spec.key.clone())),
                Some(_) => {}
            },
            Kind::Instant => match v.as_u64() {
                None => out.push(Problem::WrongType(spec.key.clone())),
                Some(ms) if !(1_577_836_800_000..4_102_444_800_000).contains(&ms) => out.push(Problem::OutOfRange(spec.key.clone())),
                Some(_) => {}
            },
        }
    }
    for k in obj.keys() {
        if !m.keys.iter().any(|s| &s.key == k) {
            out.push(Problem::Unexpected(k.clone()));
        }
    }
    out
}

/// Run the manifest's parser on one saved page, in the sandbox.
pub fn parse(backend: Backend, m: &Manifest, dir: &Path, page: &Path) -> Result<Value, String> {
    let page = std::fs::canonicalize(page).map_err(|e| format!("{}: {e}", page.display()))?;
    let mut argv: Vec<String> = m.parse.clone();
    argv.push(page.display().to_string());
    let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
    let policy = Policy { workdir: dir.to_path_buf(), writable: vec![], network: false, timeout: Duration::from_secs(30) };
    let r = sandbox::run(backend, &policy, &argv)?;
    if !r.ok() {
        return Err(format!("the parser failed: {}", r.stderr.lines().last().unwrap_or("no message")));
    }
    serde_json::from_str(r.stdout.trim()).map_err(|e| format!("the parser printed something that is not JSON: {e}"))
}

#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    Pass,
    Fail(Vec<String>),
}

/// The fixture gate: every fixture is parsed; those with a recorded answer must match it exactly, the rest must validate.
pub fn run_fixtures(backend: Backend, m: &Manifest, dir: &Path) -> Verdict {
    let mut fails = Vec::new();
    for f in &m.fixtures {
        match parse(backend, m, dir, &dir.join(&f.input)) {
            Err(e) => fails.push(format!("{}: {e}", f.input)),
            Ok(got) => {
                let problems = validate(m, &got);
                if !problems.is_empty() {
                    fails.push(format!("{}: {problems:?}", f.input));
                    continue;
                }
                if let Some(exp) = &f.expected {
                    match std::fs::read_to_string(dir.join(exp)).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
                        Some(want) if want == got => {}
                        Some(want) => fails.push(format!("{}: expected {want}, got {got}", f.input)),
                        None => fails.push(format!("{}: cannot read the recorded answer {exp}", f.input)),
                    }
                }
            }
        }
    }
    if fails.is_empty() { Verdict::Pass } else { Verdict::Fail(fails) }
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok(String::from_utf8_lossy(&out.stdout).trim().to_string()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

/// A repair may add files and change the parser, but must not edit or delete the manifest, the old fixtures or their recorded answers.
/// `scraper_dir` is repo-relative; `base` is the commit the repair started from. The files it tampered with.
pub fn weakened(repo: &Path, base: &str, scraper_dir: &str) -> Result<Vec<String>, String> {
    let mut guarded: Vec<String> = vec![format!("{scraper_dir}/manifest.json")];
    // the guarded set is the one in the base commit's manifest, not the candidate's
    let old: Manifest = serde_json::from_str(&git(repo, &["show", &format!("{base}:{scraper_dir}/manifest.json")])?).map_err(|e| format!("the manifest at {base}: {e}"))?;
    for f in &old.fixtures {
        guarded.push(format!("{scraper_dir}/{}", f.input));
        if let Some(e) = &f.expected {
            guarded.push(format!("{scraper_dir}/{e}"));
        }
    }
    let changed = git(repo, &["diff", "--name-status", base])?;
    Ok(changed.lines().filter_map(|l| l.split_once('\t')).filter(|(st, p)| !st.starts_with('A') && guarded.iter().any(|g| g == p)).map(|(_, p)| p.to_string()).collect())
}

/// A live page that no longer parses: save it as a fixture and queue a repair. The task id, or None if the page is fine.
pub fn check_live(backend: Backend, m: &Manifest, dir: &Path, repo_rel: &str, page: &Path, tasks: &Tasks, audit: &Audit, now_ms: u64) -> Result<Option<Task>, String> {
    let why = match parse(backend, m, dir, page) {
        Err(e) => e,
        Ok(got) => {
            let p = validate(m, &got);
            if p.is_empty() {
                return Ok(None);
            }
            format!("{p:?}")
        }
    };
    let name = format!("fixtures/live-{now_ms}{}", page.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default());
    let dest: PathBuf = dir.join(&name);
    std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::copy(page, &dest).map_err(|e| e.to_string())?;
    let details = format!(
        "The scraper `{n}` ({repo_rel}) no longer parses {url}: {why}\n\nThe page as it is now is saved at {repo_rel}/{name}. Change the parser (not the manifest, not the old fixtures or their recorded answers) so that every old fixture still gives exactly its recorded answer and the new page gives a valid one. Then add the new page to the manifest's fixtures with `expected` set to a file holding the answer. The change is rejected if any old fixture's answer changes.",
        n = m.name,
        url = m.url
    );
    let task = tasks.enqueue(now_ms, &format!("Repair the {} scraper", m.name), &details)?;
    audit.append(now_ms, Actor::Pipeline, Action::TaskQueued, &format!("#{} repair {}: {why}", task.id, m.name))?;
    Ok(Some(task))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const PARSER: &str = r#"
import json, re, sys
html = open(sys.argv[1]).read()
m = re.search(r'<span class="price">£([0-9.]+)</span>', html)
if not m: sys.exit("no price on the page")
print(json.dumps({"fuel.price": float(m.group(1))}))
"#;

    fn setup(name: &str) -> (PathBuf, Manifest) {
        let d = std::env::temp_dir().join(format!("scr-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("fixtures")).unwrap();
        std::fs::write(d.join("parse.py"), PARSER).unwrap();
        std::fs::write(d.join("fixtures/a.html"), r#"<p>Petrol <span class="price">£1.43</span></p>"#).unwrap();
        std::fs::write(d.join("fixtures/a.json"), r#"{"fuel.price": 1.43}"#).unwrap();
        let m = Manifest {
            name: "fuel".into(),
            url: "https://example.invalid/fuel".into(),
            parse: vec!["python3".into(), "parse.py".into()],
            keys: vec![KeySpec { key: "fuel.price".into(), kind: Kind::Number { min: 0.5, max: 4.0 } }],
            fixtures: vec![Fixture { input: "fixtures/a.html".into(), expected: Some("fixtures/a.json".into()) }],
        };
        std::fs::write(d.join("manifest.json"), serde_json::to_string_pretty(&m).unwrap()).unwrap();
        (d, m)
    }

    #[test]
    fn answers_are_checked_against_the_declared_types() {
        let (_, m) = setup("validate");
        assert!(validate(&m, &json!({ "fuel.price": 1.5 })).is_empty());
        assert_eq!(validate(&m, &json!({})), vec![Problem::Missing("fuel.price".into())]);
        assert_eq!(validate(&m, &json!({ "fuel.price": "1.5" })), vec![Problem::WrongType("fuel.price".into())]);
        assert_eq!(validate(&m, &json!({ "fuel.price": 143.0 })), vec![Problem::OutOfRange("fuel.price".into())]);
        assert_eq!(validate(&m, &json!({ "fuel.price": 1.5, "x": 1 })), vec![Problem::Unexpected("x".into())]);
        assert_eq!(Manifest::load(&setup("load").0.join("manifest.json")).unwrap(), m);
    }

    #[test]
    fn the_gate_runs_the_parser_on_every_fixture() {
        let Some(b) = Backend::detect() else { return };
        let (d, m) = setup("gate");
        assert_eq!(run_fixtures(b, &m, &d), Verdict::Pass);
        // a parser that has drifted to give a different answer on the old page fails the gate
        std::fs::write(d.join("parse.py"), PARSER.replace("float(m.group(1))", "float(m.group(1)) + 1")).unwrap();
        assert!(matches!(run_fixtures(b, &m, &d), Verdict::Fail(f) if f[0].contains("expected")));
    }

    #[test]
    fn a_page_that_stops_parsing_becomes_a_fixture_and_a_repair_task() {
        let Some(b) = Backend::detect() else { return };
        let (d, m) = setup("live");
        let tasks = Tasks::open(d.join("tasks")).unwrap();
        let audit = Audit::open(d.join("audit"));
        let good = d.join("now.html");
        std::fs::write(&good, r#"<span class="price">£1.50</span>"#).unwrap();
        assert!(check_live(b, &m, &d, "feeds/scrapers/fuel", &good, &tasks, &audit, 5).unwrap().is_none());
        let broken = d.join("new-layout.html");
        std::fs::write(&broken, r#"<div data-price="1.50">£1.50</div>"#).unwrap();
        let t = check_live(b, &m, &d, "feeds/scrapers/fuel", &broken, &tasks, &audit, 6).unwrap().unwrap();
        assert!(t.title.contains("fuel") && t.details.contains("feeds/scrapers/fuel/fixtures/live-6.html"));
        assert!(d.join("fixtures/live-6.html").exists());
        assert_eq!(audit.entries().unwrap()[0].action, Action::TaskQueued);
    }

    #[test]
    fn weakening_the_gate_is_detected_but_adding_to_it_is_not() {
        let (src, _) = setup("weak");
        let repo = std::env::temp_dir().join(format!("scr-weak-repo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(repo.join("feeds/scrapers")).unwrap();
        std::fs::rename(&src, repo.join("feeds/scrapers/fuel")).unwrap();
        for a in [&["init", "-q", "-b", "main"][..], &["add", "-A"], &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "base"]] {
            git(&repo, a).unwrap();
        }
        let base = git(&repo, &["rev-parse", "HEAD"]).unwrap();
        let dir = "feeds/scrapers/fuel";
        // a proper repair: a new parser and a new fixture
        std::fs::write(repo.join(dir).join("parse.py"), "# new parser\n").unwrap();
        std::fs::write(repo.join(dir).join("fixtures/b.html"), "<b>").unwrap();
        git(&repo, &["add", "-A"]).unwrap();
        assert_eq!(weakened(&repo, &base, dir).unwrap(), Vec::<String>::new());
        // editing a recorded answer, an old fixture, or the manifest is not
        std::fs::write(repo.join(dir).join("fixtures/a.json"), r#"{"fuel.price": 9}"#).unwrap();
        std::fs::remove_file(repo.join(dir).join("fixtures/a.html")).unwrap();
        std::fs::write(repo.join(dir).join("manifest.json"), "{}").unwrap();
        git(&repo, &["add", "-A"]).unwrap();
        let mut bad = weakened(&repo, &base, dir).unwrap();
        bad.sort();
        assert_eq!(bad, vec!["feeds/scrapers/fuel/fixtures/a.html", "feeds/scrapers/fuel/fixtures/a.json", "feeds/scrapers/fuel/manifest.json"]);
        let _ = std::fs::remove_dir_all(repo);
    }
}
