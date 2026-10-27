//! The fixture gate as a command, for the promotion pipeline's checks: `fixtures <manifest.json> [<base-commit> <repo-relative scraper dir>]`.
//! Exit 0 if every fixture gives its recorded answer (and, with a base, the gate was not weakened); 1 otherwise.

use std::path::Path;

use harness::sandbox::Backend;
use harness::scraper::{run_fixtures, weakened, Manifest, Verdict};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(manifest) = args.first() else {
        eprintln!("usage: fixtures <manifest.json> [<base-commit> <scraper-dir>]");
        std::process::exit(2)
    };
    let path = Path::new(manifest);
    let dir = path.parent().unwrap_or(Path::new("."));
    let m = Manifest::load(path).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1)
    });
    let backend = Backend::detect().unwrap_or_else(|| {
        eprintln!("no sandbox available to run the parser in");
        std::process::exit(1)
    });
    let mut failed = false;
    if let (Some(base), Some(sdir)) = (args.get(1), args.get(2)) {
        match weakened(Path::new("."), base, sdir) {
            Ok(bad) if bad.is_empty() => {}
            Ok(bad) => {
                eprintln!("the gate was weakened: {}", bad.join(", "));
                failed = true;
            }
            Err(e) => {
                eprintln!("{e}");
                failed = true;
            }
        }
    }
    if let Verdict::Fail(why) = run_fixtures(backend, &m, dir) {
        for w in why {
            eprintln!("{w}");
        }
        failed = true;
    }
    std::process::exit(if failed { 1 } else { 0 });
}
