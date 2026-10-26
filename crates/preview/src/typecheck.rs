//! Type-check an eval against the kit's generated declarations (`docs/ui.d.ts`, from `ui::docs::dts`) with `deno check`, before it is run.
//! The agent's code is TypeScript; this is what makes the typed API (keys, enums, zones, icon names) catch mistakes instead of the page.
//! Optional: if `deno` is not installed the check is skipped, never failed.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use serde_json::{json, Value};

/// One compiler diagnostic, with the line counted within the checked snippet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl Diagnostic {
    pub fn to_json(&self) -> Value {
        json!({ "code": self.code, "message": self.message, "line": self.line, "column": self.column })
    }
}

/// Whether `deno` can be run (checked once).
pub fn available() -> bool {
    static OK: OnceLock<bool> = OnceLock::new();
    *OK.get_or_init(|| Command::new("deno").arg("--version").output().map(|o| o.status.success()).unwrap_or(false))
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Check `snippet` as a continuation of the `context` scripts (earlier journal entries, already accepted): their top-level names are in scope,
/// and only diagnostics inside the snippet are returned.
pub fn check(dts: &str, context: &[String], snippet: &str) -> Result<Vec<Diagnostic>, String> {
    let dir: PathBuf = std::env::temp_dir().join(format!("display-check-{}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::SeqCst)));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let result = (|| {
        fs::write(dir.join("ui.d.ts"), dts).map_err(|e| e.to_string())?;
        fs::write(
            dir.join("deno.json"),
            r#"{ "compilerOptions": { "lib": ["dom", "dom.iterable", "esnext"], "strict": true, "noImplicitAny": true, "types": ["./ui.d.ts"] } }"#,
        )
        .map_err(|e| e.to_string())?;
        let prefix: String = context.iter().map(|c| format!("{c}\n")).collect();
        let first = prefix.lines().count() + 1;
        fs::write(dir.join("check.ts"), format!("{prefix}{snippet}\n")).map_err(|e| e.to_string())?;
        let out = Command::new("deno")
            .args(["check", "--config", "deno.json", "check.ts"])
            .current_dir(&dir)
            .env("NO_COLOR", "1")
            .env("DENO_NO_UPDATE_CHECK", "1")
            .output()
            .map_err(|e| format!("cannot run deno: {e}"))?;
        let text = format!("{}\n{}", String::from_utf8_lossy(&out.stderr), String::from_utf8_lossy(&out.stdout));
        Ok(parse(&text, first))
    })();
    let _ = fs::remove_dir_all(&dir);
    result
}

/// Read deno's diagnostics: a `TS1234 [ERROR]: message` line followed (below) by `at file:///.../check.ts:LINE:COL`.
pub fn parse(text: &str, first_line: usize) -> Vec<Diagnostic> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let l = l.trim_start_matches("error: ");
        let Some(rest) = l.strip_prefix("TS") else { continue };
        let Some((code, msg)) = rest.split_once(" [ERROR]: ") else { continue };
        let at = lines[i + 1..].iter().take(12).find_map(|x| {
            let x = x.trim();
            let loc = x.strip_prefix("at ")?;
            let path = loc.rsplitn(3, ':').collect::<Vec<_>>(); // col, line, path
            (path.len() == 3 && path[2].ends_with("check.ts")).then(|| (path[1].parse::<usize>().ok(), path[0].parse::<usize>().ok()))
        });
        let (Some((Some(line), Some(col))), true) = (at, !code.is_empty()) else { continue };
        if line >= first_line {
            out.push(Diagnostic { code: format!("TS{code}"), message: msg.to_string(), line: line - first_line + 1, column: col });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_deno_output_and_drops_context_lines() {
        let text = "error: TS2339 [ERROR]: Property 'x' does not exist.\n  foo.x\n      ~\n    at file:///tmp/check.ts:5:5\n\nTS2304 [ERROR]: Cannot find name 'y'.\n    at file:///tmp/check.ts:2:1\n\nFound 2 errors.\n";
        let d = parse(text, 4);
        assert_eq!(d, vec![Diagnostic { code: "TS2339".into(), message: "Property 'x' does not exist.".into(), line: 2, column: 5 }]);
    }

    #[test]
    fn the_shipped_seeds_type_check_against_the_generated_declarations() {
        if !available() {
            eprintln!("deno not installed: skipping");
            return;
        }
        let dts = ui::docs::dts(&ui::docs::all(), ui::SOURCE);
        for (name, src) in ui::SEEDS {
            let d = check(&dts, &[], src).expect("deno ran");
            assert!(d.is_empty(), "{name}: {d:?}");
        }
        // and a wrong snippet is caught
        let bad = check(&dts, &[], "fmt.num(\"12\");").expect("deno ran");
        assert_eq!(bad.len(), 1, "{bad:?}");
        assert_eq!(bad[0].code, "TS2345");
    }
}
