//! The display UI kit. `ts/ui.ts` is the source of truth; this crate strips its
//! types so the orchestrator can inject plain JS into the page / console.

use std::path::Path;

use oxc::allocator::Allocator;
use oxc::codegen::Codegen;
use oxc::parser::Parser;
use oxc::semantic::SemanticBuilder;
use oxc::span::SourceType;
use oxc::transformer::{TransformOptions, Transformer};

/// The TypeScript source, embedded at compile time.
pub const SOURCE: &str = include_str!("../ts/ui.ts");

/// Strip TypeScript types from `src` and return plain JS.
pub fn strip_types(src: &str, filename: &str) -> Result<String, String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, src, SourceType::ts()).parse();
    if !parsed.errors.is_empty() {
        let msgs: Vec<String> = parsed.errors.iter().map(|e| e.to_string()).collect();
        return Err(msgs.join("\n"));
    }
    let mut program = parsed.program;
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let result = Transformer::new(&allocator, Path::new(filename), &TransformOptions::default())
        .build_with_scoping(scoping, &mut program);
    if !result.errors.is_empty() {
        let msgs: Vec<String> = result.errors.iter().map(|e| e.to_string()).collect();
        return Err(msgs.join("\n"));
    }
    Ok(Codegen::new().build(&program).code)
}

/// Turn a TS module source into an injectable script: types stripped, no
/// `export`s, wrapped in an IIFE so nothing leaks except what it installs on `window`.
pub fn bundle(src: &str) -> Result<String, String> {
    let js = strip_types(src, "ui.ts")?;
    let js = js.replace("export {};", "");
    Ok(format!("(() => {{\n{js}\n}})();\n"))
}

/// Bundled variable font files, served at `/fonts/<file>`: Inter and Archivo (SIL OFL, from Fontsource), plus
/// `mono.woff2` when the owner has supplied one (a licensed face, never committed: see build.rs).
fn fonts() -> Vec<(&'static str, &'static [u8])> {
    let mut v: Vec<(&'static str, &'static [u8])> = vec![
        ("InterVariable.woff2", &include_bytes!("../fonts/InterVariable.woff2")[..]),
        ("ArchivoVariable.woff2", &include_bytes!("../fonts/ArchivoVariable.woff2")[..]),
    ];
    #[cfg(has_mono)]
    v.push(("mono.woff2", &include_bytes!("../fonts/mono.woff2")[..]));
    v
}

/// A bundled font file by name.
pub fn font(file: &str) -> Option<&'static [u8]> {
    fonts().into_iter().find(|(name, _)| *name == file).map(|(_, bytes)| bytes)
}

/// Strip types from a REPL eval. Unlike [`bundle`] there is no IIFE wrapper: the result runs as
/// a classic script so top-level `const`/`let` persist between evals.
pub fn strip_eval(src: &str) -> Result<String, String> {
    let js = strip_types(src, "eval.ts")?;
    Ok(js.replace("export {};", ""))
}

/// The embedded `ui.ts` as an injectable script.
pub fn js() -> Result<String, String> {
    bundle(SOURCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_ts_compiles_to_plain_js() {
        let js = js().expect("ui.ts should compile");
        assert!(js.contains("window.ui"), "installs the ui object");
        assert!(js.contains("data-dropped"), "has the guaranteed-fit machinery");
        assert!(!js.contains("OverflowError"), "size is never an error");
        assert!(!js.contains("export "), "no module syntax left");
        assert!(!js.contains("declare global"), "types are gone");
        assert!(!js.contains(": Props"), "annotations are gone");
    }

    #[test]
    fn unknown_fonts_are_not_served() {
        assert!(font("nope.ttf").is_none());
        assert!(font("InterVariable.woff2").is_some());
        assert_eq!(font("mono.woff2").is_some(), std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/mono.woff2")).exists());
    }

    #[test]
    fn strip_eval_removes_types_without_wrapping() {
        let js = strip_eval("const n: number = 1; ui.show(p(String(n)));").unwrap();
        assert!(!js.contains(": number"), "{js}");
        assert!(js.starts_with("const n"), "no IIFE, so const persists: {js}");
    }

    #[test]
    fn syntax_errors_are_reported() {
        assert!(bundle("const = ;").is_err());
    }
}
