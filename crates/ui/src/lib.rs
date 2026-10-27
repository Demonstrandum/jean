//! The display UI kit. `ts/ui.ts` is the source of truth; this crate strips its
//! types so the orchestrator can inject plain JS into the page / console.

use std::path::Path;

pub mod docs;

use oxc::allocator::Allocator;
use oxc::codegen::Codegen;
use oxc::parser::Parser;
use oxc::semantic::SemanticBuilder;
use oxc::span::SourceType;
use oxc::transformer::{TransformOptions, Transformer};

/// The TypeScript source, embedded at compile time.
pub const SOURCE: &str = include_str!("../ts/ui.ts");
/// The data schema: typed keys, enums, structures and the typed functions over them (`K`, `fmt`, `calendar`, ...).
pub const SCHEMA: &str = include_str!("../ts/schema.ts");

/// The screens the display starts with, as journaled evals: `(file name, TypeScript)`. The server applies them, one change each,
/// when the journal is empty; after that the journal is the truth and the agent edits screens with `ui.defineScreen` / `patchTile`.
pub const SEEDS: [(&str, &str); 8] = [
    ("10-home.ts", include_str!("../seed/10-home.ts")),
    ("20-day.ts", include_str!("../seed/20-day.ts")),
    ("30-weather.ts", include_str!("../seed/30-weather.ts")),
    ("40-system.ts", include_str!("../seed/40-system.ts")),
    ("50-energy.ts", include_str!("../seed/50-energy.ts")),
    ("60-lists.ts", include_str!("../seed/60-lists.ts")),
    ("65-room.ts", include_str!("../seed/65-room.ts")),
    ("70-claude.ts", include_str!("../seed/70-claude.ts")),
];

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

/// Bundled variable font files, served at `/fonts/<file>`: Inter and Archivo (SIL OFL, from Fontsource).
const FONTS: [(&str, &[u8]); 2] = [
    ("InterVariable.woff2", include_bytes!("../fonts/InterVariable.woff2")),
    ("ArchivoVariable.woff2", include_bytes!("../fonts/ArchivoVariable.woff2")),
];

/// Licensed faces, compiled in only as age-encrypted files: `(served name, ciphertext)`. The server decrypts them at startup
/// with the key in `secrets/font.key` (see crates/preview); without the key the UI falls back to the system monospace.
pub const ENCRYPTED_FONTS: [(&str, &[u8]); 1] = [("berkeley.woff2", include_bytes!("../fonts/berkeley.woff2.age"))];

/// A bundled (open-licensed) font file by name.
pub fn font(file: &str) -> Option<&'static [u8]> {
    FONTS.iter().find(|(name, _)| *name == file).map(|(_, bytes)| *bytes)
}

/// Strip types from a REPL eval. Unlike [`bundle`] there is no IIFE wrapper: the result runs as
/// a classic script so top-level `const`/`let` persist between evals.
pub fn strip_eval(src: &str) -> Result<String, String> {
    let js = strip_types(src, "eval.ts")?;
    Ok(js.replace("export {};", ""))
}

/// The kit followed by the schema, as one injectable script.
pub fn js() -> Result<String, String> {
    bundle_all(SOURCE, SCHEMA)
}

/// `ui.ts` and `schema.ts` bundled in order (the schema uses what the kit installs on `window`).
pub fn bundle_all(ui_src: &str, schema_src: &str) -> Result<String, String> {
    Ok(format!("{}{}", bundle(ui_src)?, bundle(schema_src)?))
}

/// Every seed as plain JS, in order (types stripped, no wrapping): what the gallery page runs directly.
pub fn seeds_js() -> Result<String, String> {
    let mut out = String::new();
    for (name, src) in SEEDS {
        out.push_str(&strip_eval(src).map_err(|e| format!("{name}: {e}"))?);
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_ts_compiles_to_plain_js() {
        let js = js().expect("ui.ts should compile");
        assert!(js.contains("window.ui"), "installs the ui object");
        assert!(js.contains("data-dropped"), "has the guaranteed-fit machinery");
        assert!(js.contains("defineScreen"), "has the screen registry");
        assert!(js.contains("calendar"), "ships the schema");
        assert!(!js.contains("OverflowError"), "size is never an error");
        assert!(!js.contains("export "), "no module syntax left");
        assert!(!js.contains("declare global"), "types are gone");
        assert!(!js.contains(": Props"), "annotations are gone");
    }

    #[test]
    fn unknown_fonts_are_not_served() {
        assert!(font("nope.ttf").is_none());
        for (name, bytes) in FONTS {
            assert!(bytes.starts_with(b"wOF2"), "{name} is a real woff2 (not an LFS pointer)");
        }
        for (name, bytes) in ENCRYPTED_FONTS {
            assert!(bytes.starts_with(b"age-encryption.org/v1"), "{name} is an age file (not an LFS pointer)");
        }
    }

    #[test]
    fn strip_eval_removes_types_without_wrapping() {
        let js = strip_eval("const n: number = 1; ui.show(p(String(n)));").unwrap();
        assert!(!js.contains(": number"), "{js}");
        assert!(js.starts_with("const n"), "no IIFE, so const persists: {js}");
    }

    #[test]
    fn every_seed_compiles() {
        let js = seeds_js().expect("seeds compile");
        for id in ["home", "day", "weather", "energy", "lists", "claude", "system"] {
            assert!(js.contains(&format!("id: \"{id}\"")), "seed for {id}");
        }
    }

    #[test]
    fn syntax_errors_are_reported() {
        assert!(bundle("const = ;").is_err());
    }
}
