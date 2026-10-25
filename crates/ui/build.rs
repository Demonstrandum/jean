// The monospace face (Berkeley Mono) is licensed and is not part of the repository. If the owner puts it at
// crates/ui/fonts/mono.woff2 it is compiled in; without it the UI falls back to the system monospace.
fn main() {
    println!("cargo:rerun-if-changed=fonts/mono.woff2");
    println!("cargo::rustc-check-cfg=cfg(has_mono)");
    if std::path::Path::new("fonts/mono.woff2").exists() {
        println!("cargo:rustc-cfg=has_mono");
    }
}
