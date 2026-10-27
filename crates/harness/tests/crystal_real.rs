//! The equivalence gate against a real display: the preview server and headless Chrome. Skipped where there is no Chrome.

use std::path::{Path, PathBuf};
use std::process::Command;

use harness::crystal::{compare, scratch_store, Difference, Frames, ScratchDisplay};
use store::{Meta, Store};

fn chrome() -> Option<PathBuf> {
    ["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "/Applications/Chromium.app/Contents/MacOS/Chromium", "/usr/bin/google-chrome", "/usr/bin/chromium", "/usr/bin/chromium-browser"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

#[test]
fn frames_from_a_real_display_tell_equivalent_histories_from_different_ones() {
    let Some(chrome) = chrome() else {
        eprintln!("no Chrome here; skipping");
        return;
    };
    let root = root();
    assert!(Command::new("cargo").args(["build", "-q", "-p", "preview"]).current_dir(&root).status().unwrap().success());
    let display = ScratchDisplay { server_bin: root.join("target/debug/preview"), root: root.clone(), chrome };

    // a history of three changes to the home screen's Claude tile
    let dir = std::env::temp_dir().join(format!("crys-real-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::open(dir.join("store")).unwrap();
    // as the server does on a first start, the shipped screens are the first entries
    let mut seeds: Vec<_> = std::fs::read_dir(root.join("crates/ui/seed")).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "ts")).collect();
    seeds.sort();
    for f in &seeds {
        let p = store.begin(&std::fs::read_to_string(f).unwrap()).unwrap();
        store.commit(p, "<dom>", &Meta::default()).unwrap();
    }
    let n = seeds.len() as u64;
    let steps = [
        r#"ui.screenOf("home").patchTile("claude", { title: "One" });"#,
        r#"ui.screenOf("home").patchTile("claude", { title: "Two" });"#,
        r#"ui.screenOf("home").patchTile("claude", { title: "Three" });"#,
    ];
    for s in steps {
        let p = store.begin(s).unwrap();
        store.commit(p, "<dom>", &Meta::default()).unwrap();
    }
    // the screens also have to exist: the seeds are the base of a real store
    let before_dir = dir.join("before");
    scratch_store(&store, &before_dir, None).unwrap();
    let before = display.frames(&before_dir).expect("frames of the full history");
    assert!(before.contains_key("home") && before["home"].contains("Three"), "{:?}", before.keys().collect::<Vec<_>>());

    // the seeds plus only the last change are equivalent to the whole history; the seeds plus only the first are not
    let all_seeds: String = seeds.iter().map(|f| std::fs::read_to_string(f).unwrap() + "\n").collect();
    let same = dir.join("same");
    scratch_store(&store, &same, Some((n + 3, &format!("{all_seeds}{}", steps[2])))).unwrap();
    assert_eq!(compare(&before, &display.frames(&same).unwrap()), None);
    let other = dir.join("other");
    scratch_store(&store, &other, Some((n + 3, &format!("{all_seeds}{}", steps[0])))).unwrap();
    let d = compare(&before, &display.frames(&other).unwrap());
    assert!(matches!(d, Some(Difference::Frame { ref screen, .. }) if screen == "home"), "{d:?}");
    let _ = std::fs::remove_dir_all(dir);
}
