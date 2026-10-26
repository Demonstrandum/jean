//! Writes the generated files: `docs/reference.md` (the kit's reference) and `feeds/_schema.py` (the schema's enums for the feeds).
//! usage: cargo run -p ui --bin gen-docs [repo-root=.]

use std::fs;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    let items = ui::docs::all();
    fs::create_dir_all(root.join("docs")).expect("docs dir");
    fs::write(root.join("docs/reference.md"), ui::docs::markdown(&items)).expect("write reference.md");
    fs::write(root.join("docs/ui.d.ts"), ui::docs::dts(&items, ui::SOURCE)).expect("write ui.d.ts");
    fs::create_dir_all(root.join("feeds")).expect("feeds dir");
    fs::write(root.join("feeds/_schema.py"), ui::docs::schema_py(&ui::docs::extract(ui::SCHEMA))).expect("write _schema.py");
    println!("{} items -> docs/reference.md, docs/ui.d.ts, feeds/_schema.py", items.len());
}
