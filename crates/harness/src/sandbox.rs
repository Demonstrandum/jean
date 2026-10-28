//! Running commands the builder wrote without trusting them: a typed policy (where it may write, whether it may reach the network),
//! enforced by the operating system (`sandbox-exec` on macOS, `bwrap` on Linux).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// What a sandboxed command may do. Reading is allowed everywhere (the toolchains live outside the worktree); writing only where listed.
#[derive(Clone, Debug)]
pub struct Policy {
    /// The directory the command runs in, and may write to.
    pub workdir: PathBuf,
    /// Further writable directories (a scratch dir, a build cache).
    pub writable: Vec<PathBuf>,
    pub network: bool,
    /// Connections to this machine only (127.0.0.1): a project's tests start servers and talk to them. Implied by `network`; on Linux the unshared network
    /// namespace has its own loopback either way.
    pub loopback: bool,
    pub timeout: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ran {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

impl Ran {
    pub fn ok(&self) -> bool {
        self.code == Some(0) && !self.timed_out
    }
}

/// Which mechanism confines the command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    SandboxExec,
    Bwrap,
}

impl Backend {
    /// The mechanism this machine has, if any. There is no unconfined fallback. Inside a confined command (`JEAN_IN_SANDBOX` is set for those) there is none:
    /// confinement does not nest, so a project's own tests that use the sandbox skip themselves there, as they do on a machine without one.
    pub fn detect() -> Option<Backend> {
        if std::env::var_os("JEAN_IN_SANDBOX").is_some() {
            return None;
        }
        let has = |bin: &str| Command::new("sh").args(["-c", &format!("command -v {bin}")]).stdout(Stdio::null()).status().map(|s| s.success()).unwrap_or(false);
        if cfg!(target_os = "macos") && has("sandbox-exec") {
            Some(Backend::SandboxExec)
        } else if has("bwrap") {
            Some(Backend::Bwrap)
        } else {
            None
        }
    }
}

fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// The sandbox-exec profile for a policy: deny everything, allow reads and processes, allow writes to the listed directories (and the null/tty devices).
pub fn seatbelt_profile(p: &Policy) -> String {
    let mut s = String::from("(version 1)\n(deny default)\n(allow process-fork)\n(allow process-exec)\n(allow signal (target self))\n(allow sysctl-read)\n(allow mach-lookup)\n(allow file-read*)\n(allow file-write* (literal \"/dev/null\") (literal \"/dev/dtracehelper\"))\n");
    for dir in std::iter::once(&p.workdir).chain(p.writable.iter()) {
        s.push_str(&format!("(allow file-write* (subpath \"{}\"))\n", canonical(dir).display()));
    }
    if p.network {
        s.push_str("(allow network*)\n");
    } else if p.loopback {
        s.push_str("(allow network-bind (local ip \"localhost:*\"))\n(allow network-inbound (local ip \"localhost:*\"))\n(allow network-outbound (remote ip \"localhost:*\"))\n");
    }
    s
}

fn bwrap_args(p: &Policy) -> Vec<String> {
    let mut a: Vec<String> = ["--ro-bind", "/", "/", "--dev", "/dev", "--proc", "/proc", "--tmpfs", "/tmp", "--unshare-pid", "--die-with-parent"].iter().map(|s| s.to_string()).collect();
    if !p.network {
        a.push("--unshare-net".into());
    }
    for dir in std::iter::once(&p.workdir).chain(p.writable.iter()) {
        let d = canonical(dir).display().to_string();
        a.extend(["--bind".into(), d.clone(), d]);
    }
    a.extend(["--chdir".into(), canonical(&p.workdir).display().to_string()]);
    a
}

/// Run `argv` under the policy. The output is captured; a command still running at the timeout is killed.
/// The command gets a private temporary directory (`TMPDIR`), writable under the policy and removed afterwards: compilers and test programs make temp files, and
/// the system temp directory is, rightly, not writable from inside.
pub fn run(backend: Backend, policy: &Policy, argv: &[&str]) -> Result<Ran, String> {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let tmp = std::env::temp_dir().join(format!("jean-sandbox-{}-{}", std::process::id(), COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    std::fs::create_dir_all(&tmp).map_err(|e| format!("cannot make a temp dir: {e}"))?;
    let mut with_tmp = policy.clone();
    with_tmp.writable.push(tmp.clone());
    let result = run_confined(backend, &with_tmp, &tmp, argv);
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

fn run_confined(backend: Backend, policy: &Policy, tmp: &Path, argv: &[&str]) -> Result<Ran, String> {
    let mut cmd = match backend {
        Backend::SandboxExec => {
            let mut c = Command::new("sandbox-exec");
            c.args(["-p", &seatbelt_profile(policy)]).args(argv);
            c
        }
        Backend::Bwrap => {
            let mut c = Command::new("bwrap");
            c.args(bwrap_args(policy)).args(argv);
            c
        }
    };
    let mut child = cmd.current_dir(&policy.workdir).env("TMPDIR", canonical(tmp)).env("JEAN_IN_SANDBOX", "1").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("cannot start the sandbox: {e}"))?;
    let out = child.stdout.take().unwrap();
    let err = child.stderr.take().unwrap();
    let read = |mut r: Box<dyn std::io::Read + Send>| std::thread::spawn(move || { let mut s = String::new(); let _ = r.read_to_string(&mut s); s });
    let (to, te) = (read(Box::new(out)), read(Box::new(err)));
    let start = Instant::now();
    let (code, timed_out) = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(st) => break (st.code(), false),
            None if start.elapsed() > policy.timeout => {
                let _ = child.kill();
                let _ = child.wait();
                break (None, true);
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    Ok(Ran { code, stdout: to.join().unwrap_or_default(), stderr: te.join().unwrap_or_default(), timed_out })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sbx-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn policy(d: &Path) -> Policy {
        Policy { workdir: d.to_path_buf(), writable: vec![], network: false, loopback: false, timeout: Duration::from_secs(10) }
    }

    #[test]
    fn the_profile_names_exactly_the_writable_directories() {
        let d = work("profile");
        let mut p = policy(&d);
        p.writable.push(d.join("cache"));
        let s = seatbelt_profile(&p);
        assert!(s.starts_with("(version 1)\n(deny default)"));
        assert_eq!(s.matches("(allow file-write* (subpath").count(), 2);
        assert!(!s.contains("network"));
        p.network = true;
        assert!(seatbelt_profile(&p).contains("(allow network*)"));
        let _ = std::fs::remove_dir_all(d);
    }

    // The tests below need an enforcing backend; where there is none they say so and pass.
    fn backend() -> Option<Backend> {
        let b = Backend::detect();
        if b.is_none() {
            eprintln!("no sandbox backend here; skipping");
        }
        b
    }

    #[test]
    fn a_command_can_write_in_its_workdir_but_nowhere_else() {
        let Some(b) = backend() else { return };
        let d = work("write");
        let outside = std::env::temp_dir().join(format!("sbx-outside-{}", std::process::id()));
        let _ = std::fs::remove_file(&outside);
        let r = run(b, &policy(&d), &["sh", "-c", "echo hi > inside.txt"]).unwrap();
        assert!(r.ok(), "{r:?}");
        assert_eq!(std::fs::read_to_string(d.join("inside.txt")).unwrap().trim(), "hi");
        let r = run(b, &policy(&d), &["sh", "-c", &format!("echo no > {}", outside.display())]).unwrap();
        assert!(!r.ok() && !outside.exists(), "{r:?}");
        let home = std::env::var("HOME").unwrap_or_default();
        let r = run(b, &policy(&d), &["sh", "-c", &format!("echo no > {home}/sbx-home-probe")]).unwrap();
        assert!(!r.ok() && !Path::new(&format!("{home}/sbx-home-probe")).exists());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn a_command_has_a_private_temp_dir_of_its_own_and_it_is_cleaned_up() {
        let Some(b) = backend() else { return };
        let d = work("tmp");
        // what a compiler does: make a temp file under $TMPDIR (the system temp dir is refused; macOS mktemp ignores $TMPDIR, rustc does not)
        let r = run(b, &policy(&d), &["sh", "-c", "f=$(mktemp \"$TMPDIR/x.XXXXXX\") && echo ok > \"$f\" && echo $TMPDIR"]).unwrap();
        assert!(r.ok(), "{r:?}");
        let tmpdir = r.stdout.trim().to_string();
        assert!(tmpdir.contains("jean-sandbox-"), "{tmpdir}");
        assert!(!Path::new(&tmpdir).exists(), "the private temp dir is removed after the command");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn a_confined_command_knows_it_is_confined_and_finds_no_sandbox_to_nest() {
        let Some(b) = backend() else { return };
        let d = work("nest");
        let r = run(b, &policy(&d), &["sh", "-c", "echo $JEAN_IN_SANDBOX"]).unwrap();
        assert_eq!(r.stdout.trim(), "1", "{r:?}");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn the_network_is_closed_unless_allowed() {
        let Some(b) = backend() else { return };
        let d = work("net");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let probe = format!("exec 3<>/dev/tcp/127.0.0.1/{port}");
        let r = run(b, &policy(&d), &["bash", "-c", &probe]).unwrap();
        assert!(!r.ok(), "the sandbox let a connection out: {r:?}");
        let mut open = policy(&d);
        open.network = true;
        assert!(run(b, &open, &["bash", "-c", &probe]).unwrap().ok());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn loopback_can_be_opened_without_opening_the_network() {
        let Some(b) = backend() else { return };
        let d = work("loop");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut p = policy(&d);
        p.loopback = true;
        assert!(run(b, &p, &["bash", "-c", &format!("exec 3<>/dev/tcp/127.0.0.1/{port}")]).unwrap().ok(), "this machine is reachable");
        // 192.0.2.1 is TEST-NET-1: not this machine, so the sandbox must refuse it (on Linux the unshared namespace has nowhere to send it)
        let r = run(b, &p, &["bash", "-c", "exec 3<>/dev/tcp/192.0.2.1/9"]).unwrap();
        assert!(!r.ok(), "the outside is still closed: {r:?}");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn a_runaway_command_is_killed_at_the_timeout() {
        let Some(b) = backend() else { return };
        let d = work("time");
        let mut p = policy(&d);
        p.timeout = Duration::from_millis(300);
        let r = run(b, &p, &["sleep", "30"]).unwrap();
        assert!(r.timed_out && !r.ok());
        let _ = std::fs::remove_dir_all(d);
    }
}
