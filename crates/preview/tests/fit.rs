//! Every screen fits its sheet, with sample data and with none: the real server, the real page in headless Chrome (scripts/selftest.sh).
//! Skipped (with a note) where no Chrome or Chromium is installed.

use std::process::Command;

#[test]
fn every_screen_fits_in_a_real_browser() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new("sh")
        .arg(root.join("scripts/selftest.sh"))
        .env("SERVER_BIN", env!("CARGO_BIN_EXE_preview"))
        .current_dir(&root)
        .output()
        .expect("run scripts/selftest.sh");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    match out.status.code() {
        Some(0) => {}
        Some(2) => eprintln!("skipped: no Chrome/Chromium ({text})"),
        _ => panic!("a screen does not fit:\n{text}"),
    }
}
