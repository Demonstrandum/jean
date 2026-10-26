//! The kit's reference, extracted from its own TypeScript: signatures, doc comments, enums and the typed key catalogue.
//!
//! Nothing here is hand-written documentation: the declarations and their `/** ... */` comments in `ts/ui.ts` and `ts/schema.ts`
//! are parsed (with oxc) into [`DocItem`]s. The server serves them (`GET /docs`), the page reads them (`ui.docs()`), and
//! `cargo run -p ui --bin gen-docs` writes `docs/reference.md` and `feeds/_schema.py` from them.

use std::collections::HashMap;

use oxc::allocator::Allocator;
use oxc::ast::ast::*;
use oxc::parser::Parser;
use oxc::span::{GetSpan, SourceType};
use serde_json::{json, Value};

/// What a documented item is. The numbers match `DocKind` in `ui.ts`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocKind {
    Function = 0,
    Value = 1,
    Interface = 2,
    Type = 3,
    Enum = 4,
    /// A typed live-data key, `K.wx.temp`: its signature is the key type, `members` carries its wire id.
    Key = 5,
}

#[derive(Clone, Debug)]
pub struct DocParam {
    pub name: String,
    pub ty: String,
    pub optional: bool,
}

#[derive(Clone, Debug)]
pub struct DocItem {
    pub name: String,
    pub kind: DocKind,
    pub signature: String,
    pub doc: String,
    pub params: Vec<DocParam>,
    pub returns: String,
    pub members: Vec<String>,
    pub global: bool,
    pub line: usize,
}

impl DocItem {
    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "kind": self.kind as u8,
            "signature": self.signature,
            "doc": self.doc,
            "params": self.params.iter().map(|p| json!({ "name": p.name, "type": p.ty, "optional": p.optional })).collect::<Vec<_>>(),
            "returns": self.returns,
            "members": self.members,
            "global": self.global,
            "line": self.line,
        })
    }

    /// One line for a listing: the name and the first sentence of the comment.
    pub fn summary(&self) -> String {
        let first = self.doc.split(['.', '\n']).next().unwrap_or("").trim();
        first.to_string()
    }
}

struct Ctx<'a> {
    src: &'a str,
    /// `attached_to` (the start of the token a comment precedes) → cleaned JSDoc text.
    docs: HashMap<u32, String>,
    globals: Vec<String>,
    items: Vec<DocItem>,
    line_starts: Vec<usize>,
}

impl<'a> Ctx<'a> {
    fn text(&self, start: u32, end: u32) -> &'a str {
        &self.src[start as usize..end as usize]
    }
    fn span_text(&self, s: oxc::span::Span) -> &'a str {
        self.text(s.start, s.end)
    }
    fn line(&self, pos: u32) -> usize {
        self.line_starts.partition_point(|&s| s <= pos as usize)
    }
    fn doc_at(&self, pos: u32) -> String {
        self.docs.get(&pos).cloned().unwrap_or_default()
    }
}

fn clean_jsdoc(raw: &str) -> String {
    raw.trim_start_matches('*')
        .lines()
        .map(|l| l.trim().trim_start_matches('*').trim())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn params_of(cx: &Ctx, ps: &FormalParameters) -> Vec<DocParam> {
    ps.items
        .iter()
        .map(|p| {
            let ty = p.type_annotation.as_ref().map(|t| squash(cx.span_text(t.type_annotation.span()))).unwrap_or_default();
            let name = squash(cx.span_text(p.pattern.span()));
            DocParam { name, ty, optional: p.optional || p.initializer.is_some() }
        })
        .collect()
}

fn returns_of(cx: &Ctx, rt: &Option<oxc::allocator::Box<'_, TSTypeAnnotation<'_>>>) -> String {
    rt.as_ref().map(|t| squash(cx.span_text(t.type_annotation.span()))).unwrap_or_default()
}

fn push(cx: &mut Ctx, item: DocItem) {
    cx.items.push(item);
}

fn visit_object(cx: &mut Ctx, prefix: &str, obj: &ObjectExpression, line_of: u32) {
    let mut members = Vec::new();
    for p in &obj.properties {
        let ObjectPropertyKind::ObjectProperty(prop) = p else { continue };
        let Some(key) = prop.key.static_name() else { continue };
        members.push(key.to_string());
        if prop.shorthand {
            continue;
        }
        let name = format!("{prefix}.{key}");
        let doc = cx.doc_at(prop.span.start);
        match &prop.value {
            Expression::ArrowFunctionExpression(f) => {
                let sig = squash(cx.text(prop.key.span().start, f.body.span.start)).trim_end_matches("=>").trim().replace(": (", "(").to_string();
                let sig = format!("{name}{}", sig.trim_start_matches(&*key).trim_start_matches(':').trim_start());
                let item = DocItem { name, kind: DocKind::Function, signature: sig, doc, params: params_of(cx, &f.params), returns: returns_of(cx, &f.return_type), members: vec![], global: false, line: cx.line(prop.span.start) };
                push(cx, item);
            }
            Expression::ObjectExpression(inner) => visit_object(cx, &name, inner, prop.span.start),
            _ => {
                if !doc.is_empty() {
                    let item = DocItem { name, kind: DocKind::Value, signature: squash(cx.span_text(prop.span)), doc, params: vec![], returns: String::new(), members: vec![], global: false, line: cx.line(prop.span.start) };
                    push(cx, item);
                }
            }
        }
    }
    let _ = line_of;
    // the object itself lists its members (a namespace like `fmt`)
    if let Some(it) = cx.items.iter_mut().find(|i| i.name == prefix) {
        it.members = members;
    }
}

/// The type arguments of a call like `ui.key<number>(...)`, as written.
fn type_args_text(cx: &Ctx, call: &CallExpression) -> String {
    call.type_arguments.as_ref().map(|t| squash(cx.span_text(t.span)).trim_start_matches('<').trim_end_matches('>').to_string()).unwrap_or_default()
}

fn string_arg(call: &CallExpression, i: usize) -> Option<String> {
    match call.arguments.get(i)? {
        Argument::StringLiteral(s) => Some(s.value.to_string()),
        _ => None,
    }
}

/// Walk `K = { wx: { temp: ui.key<number>("wx.temp", "doc", 16) } }` into key items.
fn visit_keys(cx: &mut Ctx, prefix: &str, obj: &ObjectExpression) {
    for p in &obj.properties {
        let ObjectPropertyKind::ObjectProperty(prop) = p else { continue };
        let Some(key) = prop.key.static_name() else { continue };
        let name = format!("{prefix}.{key}");
        match &prop.value {
            Expression::ObjectExpression(inner) => visit_keys(cx, &name, inner),
            Expression::CallExpression(call) => {
                let (Some(id), Some(doc)) = (string_arg(call, 0), string_arg(call, 1)) else { continue };
                let ty = type_args_text(cx, call);
                let item = DocItem { name, kind: DocKind::Key, signature: format!("{prefix}.{key}: Key<{ty}>"), doc, params: vec![], returns: ty, members: vec![format!("id: {id}")], global: false, line: cx.line(prop.span.start) };
                push(cx, item);
            }
            Expression::TSAsExpression(_) => {} // aliases of the kit's own keys (time, ui, link)
            _ => {}
        }
    }
}

fn visit_statements(cx: &mut Ctx, body: &[Statement]) {
    for st in body {
        match st {
            Statement::FunctionDeclaration(f) => {
                let Some(id) = &f.id else { continue };
                let end = f.body.as_ref().map(|b| b.span.start).unwrap_or(f.span.end);
                let sig = squash(cx.text(f.span.start, end)).trim_end_matches(';').trim().to_string();
                let doc = cx.doc_at(f.span.start);
                let name = id.name.to_string();
                // overload signatures (no body) are folded into the implementation's entry
                if let Some(prev) = cx.items.iter_mut().rev().find(|i| i.name == name && i.kind == DocKind::Function && i.line > 0) {
                    if f.body.is_none() || prev.signature.contains('\n') || prev.doc.is_empty() {
                        prev.signature = format!("{}\n{}", prev.signature, sig);
                        if prev.doc.is_empty() {
                            prev.doc = doc;
                        }
                        if f.body.is_some() {
                            continue;
                        }
                        continue;
                    }
                }
                let item = DocItem { name, kind: DocKind::Function, signature: sig, doc, params: params_of(cx, &f.params), returns: returns_of(cx, &f.return_type), members: vec![], global: false, line: cx.line(f.span.start) };
                push(cx, item);
            }
            Statement::VariableDeclaration(vd) => {
                for d in &vd.declarations {
                    let BindingPattern::BindingIdentifier(id) = &d.id else { continue };
                    let name = id.name.to_string();
                    let doc = cx.doc_at(vd.span.start);
                    let line = cx.line(vd.span.start);
                    match &d.init {
                        Some(Expression::ArrowFunctionExpression(f)) => {
                            let head = squash(cx.text(f.span.start, f.body.span.start));
                            let head = head.trim_end_matches("=>").trim();
                            let item = DocItem { name: name.clone(), kind: DocKind::Function, signature: format!("{name}{}", head.strip_prefix("async").map(|s| format!(" async{s}")).unwrap_or_else(|| head.to_string())), doc, params: params_of(cx, &f.params), returns: returns_of(cx, &f.return_type), members: vec![], global: false, line };
                            push(cx, item);
                        }
                        Some(Expression::FunctionExpression(f)) => {
                            let item = DocItem { name: name.clone(), kind: DocKind::Function, signature: format!("{name}{}", squash(cx.text(f.params.span.start, f.body.as_ref().map(|b| b.span.start).unwrap_or(f.span.end)))), doc, params: params_of(cx, &f.params), returns: returns_of(cx, &f.return_type), members: vec![], global: false, line };
                            push(cx, item);
                        }
                        Some(Expression::ObjectExpression(obj)) if name == "K" => visit_keys(cx, "K", obj),
                        Some(Expression::ObjectExpression(obj)) => {
                            if name == "exported" {
                                for p in &obj.properties {
                                    if let ObjectPropertyKind::ObjectProperty(prop) = p {
                                        if let Some(k) = prop.key.static_name() {
                                            cx.globals.push(k.to_string());
                                        }
                                    }
                                }
                                continue;
                            }
                            let ty = d.type_annotation.as_ref().map(|t| format!(": {}", squash(cx.span_text(t.type_annotation.span())))).unwrap_or_default();
                            let item = DocItem { name: name.clone(), kind: DocKind::Value, signature: format!("{name}{ty}"), doc, params: vec![], returns: String::new(), members: vec![], global: false, line };
                            push(cx, item);
                            visit_object(cx, &name, obj, vd.span.start);
                        }
                        _ => {
                            let ty = d.type_annotation.as_ref().map(|t| format!(": {}", squash(cx.span_text(t.type_annotation.span())))).unwrap_or_default();
                            if !doc.is_empty() {
                                let item = DocItem { name: name.clone(), kind: DocKind::Value, signature: format!("{name}{ty}"), doc, params: vec![], returns: String::new(), members: vec![], global: false, line };
                                push(cx, item);
                            }
                        }
                    }
                }
            }
            Statement::TSInterfaceDeclaration(i) => {
                let members = i.body.body.iter().map(|m| squash(cx.span_text(m.span()))).collect();
                let item = DocItem { name: i.id.name.to_string(), kind: DocKind::Interface, signature: format!("interface {}", i.id.name), doc: cx.doc_at(i.span.start), params: vec![], returns: String::new(), members, global: false, line: cx.line(i.span.start) };
                push(cx, item);
            }
            Statement::TSTypeAliasDeclaration(t) => {
                let item = DocItem { name: t.id.name.to_string(), kind: DocKind::Type, signature: squash(cx.span_text(t.span)).trim_end_matches(';').to_string(), doc: cx.doc_at(t.span.start), params: vec![], returns: String::new(), members: vec![], global: false, line: cx.line(t.span.start) };
                push(cx, item);
            }
            Statement::TSEnumDeclaration(e) => {
                let mut next: i64 = 0;
                let mut members = Vec::new();
                for m in &e.body.members {
                    let name = match &m.id {
                        TSEnumMemberName::Identifier(i) => i.name.to_string(),
                        TSEnumMemberName::String(s) => s.value.to_string(),
                        _ => continue,
                    };
                    match &m.initializer {
                        Some(Expression::NumericLiteral(n)) => {
                            next = n.value as i64;
                            members.push(format!("{name} = {next}"));
                        }
                        Some(Expression::StringLiteral(s)) => members.push(format!("{name} = \"{}\"", s.value)),
                        _ => members.push(format!("{name} = {next}")),
                    }
                    next += 1;
                }
                let item = DocItem { name: e.id.name.to_string(), kind: DocKind::Enum, signature: format!("enum {}", e.id.name), doc: cx.doc_at(e.span.start), params: vec![], returns: String::new(), members, global: false, line: cx.line(e.span.start) };
                push(cx, item);
            }
            Statement::ExpressionStatement(es) => {
                // an IIFE: recurse into its body; Object.assign(window, { ... }): its names are globals
                if let Expression::CallExpression(call) = es.expression.without_parentheses() {
                    match call.callee.without_parentheses() {
                        Expression::ArrowFunctionExpression(f) => visit_statements(cx, &f.body.statements),
                        Expression::FunctionExpression(f) => {
                            if let Some(b) = &f.body {
                                visit_statements(cx, &b.statements);
                            }
                        }
                        Expression::StaticMemberExpression(m) if m.property.name == "assign" => {
                            if let Some(Argument::ObjectExpression(o)) = call.arguments.get(1) {
                                for p in &o.properties {
                                    if let ObjectPropertyKind::ObjectProperty(prop) = p {
                                        if let Some(k) = prop.key.static_name() {
                                            cx.globals.push(k.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

/// Extract every documented declaration from one TypeScript source.
pub fn extract(src: &str) -> Vec<DocItem> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, src, SourceType::ts()).parse();
    let mut docs = HashMap::new();
    for c in parsed.program.comments.iter() {
        let raw = &src[c.span.start as usize..c.span.end as usize];
        if raw.starts_with("/**") {
            let inner = raw.trim_start_matches("/*").trim_end_matches("*/");
            docs.insert(c.attached_to, clean_jsdoc(inner));
        }
    }
    let mut line_starts = vec![0usize];
    line_starts.extend(src.match_indices('\n').map(|(i, _)| i + 1));
    let mut cx = Ctx { src, docs, globals: vec![], items: vec![], line_starts };
    visit_statements(&mut cx, &parsed.program.body);
    let globals: Vec<String> = cx.globals.clone();
    for it in cx.items.iter_mut() {
        it.global = globals.iter().any(|g| g == &it.name) || it.name.starts_with("K.");
    }
    cx.items
}

/// The whole reference: the kit and the schema together.
pub fn all() -> Vec<DocItem> {
    let mut items = extract(crate::SOURCE);
    items.extend(extract(crate::SCHEMA));
    items
}

/// Items whose name or comment mentions `query` (case-insensitive); an exact name match comes first.
pub fn search(items: &[DocItem], query: &str) -> Vec<DocItem> {
    let q = query.to_lowercase();
    let mut hits: Vec<DocItem> = items.iter().filter(|i| i.name.to_lowercase().contains(&q) || i.doc.to_lowercase().contains(&q) || i.signature.to_lowercase().contains(&q)).cloned().collect();
    hits.sort_by_key(|i| (i.name.to_lowercase() != q, !i.name.to_lowercase().contains(&q)));
    hits
}

/// The reference as Markdown (what `docs/reference.md` holds).
pub fn markdown(items: &[DocItem]) -> String {
    let mut out = String::from("# Reference\n\nGenerated from the doc comments and signatures in `crates/ui/ts/ui.ts` and `crates/ui/ts/schema.ts` by `cargo run -p ui --bin gen-docs`. Do not edit by hand.\n");
    let sections: [(&str, &[DocKind]); 4] = [
        ("Functions and components", &[DocKind::Function]),
        ("Values and namespaces", &[DocKind::Value]),
        ("Types, interfaces and enums", &[DocKind::Interface, DocKind::Type, DocKind::Enum]),
        ("Live data keys", &[DocKind::Key]),
    ];
    for (title, kinds) in sections {
        out.push_str(&format!("\n## {title}\n"));
        for it in items.iter().filter(|i| kinds.contains(&i.kind)) {
            out.push_str(&format!("\n### `{}`\n\n```ts\n{}\n```\n", it.name, it.signature));
            if !it.doc.is_empty() {
                out.push_str(&format!("\n{}\n", it.doc));
            }
            if !it.members.is_empty() && it.kind != DocKind::Value {
                out.push('\n');
                for m in &it.members {
                    out.push_str(&format!("- `{m}`\n"));
                }
            }
        }
    }
    out
}

/// A Python module with the schema's enums as `IntEnum`s, so feeds write the same integers the page reads.
pub fn schema_py(items: &[DocItem]) -> String {
    let mut out = String::from("\"\"\"Generated from crates/ui/ts/schema.ts: do not edit. The enums the feeds write (integers on the wire).\"\"\"\nfrom enum import IntEnum\n");
    for it in items.iter().filter(|i| i.kind == DocKind::Enum) {
        out.push_str(&format!("\n\nclass {}(IntEnum):\n", it.name));
        if !it.doc.is_empty() {
            out.push_str(&format!("    \"\"\"{}\"\"\"\n", it.doc.replace('\n', " ")));
        }
        for m in &it.members {
            out.push_str(&format!("    {m}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_signatures_docs_and_enums() {
        let items = extract(
            r#"
/** Makes a thing.
 * Second line. */
const tile = (label: string, body?: Child, n = 3): HTMLElement => h("section");
/** The sky. */
enum Sky { Clear, Partly = 5, Cloudy }
/** An hour. */
interface Hour { readonly at: number; readonly temp: number }
function feed<T>(key: Key<T>): HTMLElement;
function feed(keys: any): HTMLElement { return h("span"); }
const fmt = { /** Fixed decimals. */ num: (v: number, d = 0): string => v.toFixed(d) };
"#,
        );
        let tile = items.iter().find(|i| i.name == "tile").expect("tile");
        assert_eq!(tile.kind, DocKind::Function);
        assert!(tile.signature.starts_with("tile(label: string"), "{}", tile.signature);
        assert!(tile.doc.starts_with("Makes a thing."), "{}", tile.doc);
        assert_eq!(tile.params.len(), 3);
        assert!(tile.params[1].optional && tile.params[2].optional && !tile.params[0].optional);
        assert_eq!(tile.returns, "HTMLElement");
        let sky = items.iter().find(|i| i.name == "Sky").expect("Sky");
        assert_eq!(sky.members, ["Clear = 0", "Partly = 5", "Cloudy = 6"]);
        assert!(items.iter().any(|i| i.name == "Hour" && i.kind == DocKind::Interface && i.members.len() == 2));
        assert_eq!(items.iter().filter(|i| i.name == "feed").count(), 1, "overloads fold into one entry");
        let num = items.iter().find(|i| i.name == "fmt.num").expect("fmt.num");
        assert_eq!(num.doc, "Fixed decimals.");
    }

    #[test]
    fn the_real_kit_is_documented() {
        let items = all();
        for name in ["tile", "feed", "bind", "defineScreen", "zone", "limitBars", "dayCurve", "Col", "fmt.clock", "calendar.focus", "K.wx.temp", "Sky"] {
            let it = items.iter().find(|i| i.name == name).unwrap_or_else(|| panic!("{name} is in the reference"));
            assert!(!it.doc.is_empty(), "{name} has a doc comment");
        }
        let py = schema_py(&items);
        assert!(py.contains("class Sky(IntEnum)") && py.contains("Clear = 0"), "{py}");
    }
}
