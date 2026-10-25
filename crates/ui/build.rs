// The fonts are stored with Git LFS. A clone made without git-lfs holds small text "pointer" files instead, which would
// compile in as broken fonts: fail the build with the fix instead.
fn main() {
    let dir = std::path::Path::new("fonts");
    println!("cargo:rerun-if-changed=fonts");
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if let Ok(bytes) = std::fs::read(e.path()) {
                if bytes.starts_with(b"version https://git-lfs") {
                    panic!("{} is a Git LFS pointer, not the font: run `git lfs install && git lfs pull` and build again", e.path().display());
                }
            }
        }
    }
}
