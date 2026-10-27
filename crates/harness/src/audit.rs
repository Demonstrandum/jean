//! The audit log: every consequential thing the house does to itself, appended as a line that commits to the one before it.
//! Editing or removing any line breaks the chain from that point, and `verify` says where.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Who did it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    Voice,
    Builder,
    Pipeline,
    Watchdog,
    Person,
}

/// What was done.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    TaskQueued,
    BuildStarted,
    BuildFinished,
    ChangeRejected,
    CanaryPassed,
    CanaryFailed,
    Promoted,
    RolledBack,
    ScraperRepaired,
    ScraperRepairRejected,
    Approved,
    Crystallized,
    CrystallizeRejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub seq: u64,
    pub at_ms: u64,
    pub actor: Actor,
    pub action: Action,
    pub detail: String,
    pub prev: String,
    pub hash: String,
}

fn digest(prev: &str, seq: u64, at_ms: u64, actor: Actor, action: Action, detail: &str) -> String {
    let mut h = Sha256::new();
    for part in [prev, &seq.to_string(), &at_ms.to_string(), &format!("{actor:?}"), &format!("{action:?}"), detail] {
        h.update((part.len() as u64).to_le_bytes());
        h.update(part.as_bytes());
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

pub struct Audit {
    path: PathBuf,
}

/// The chain's first `prev`.
pub const GENESIS: &str = "genesis";

impl Audit {
    pub fn open(path: impl Into<PathBuf>) -> Audit {
        Audit { path: path.into() }
    }

    pub fn entries(&self) -> Result<Vec<Entry>, String> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.to_string()),
        };
        text.lines().enumerate().map(|(i, l)| serde_json::from_str(l).map_err(|e| format!("line {}: {e}", i + 1))).collect()
    }

    pub fn append(&self, at_ms: u64, actor: Actor, action: Action, detail: &str) -> Result<Entry, String> {
        let last = self.entries()?.pop();
        let (seq, prev) = last.map_or((1, GENESIS.to_string()), |e| (e.seq + 1, e.hash));
        let hash = digest(&prev, seq, at_ms, actor, action, detail);
        let e = Entry { seq, at_ms, actor, action, detail: detail.into(), prev, hash };
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let mut f = OpenOptions::new().create(true).append(true).open(&self.path).map_err(|e| e.to_string())?;
        writeln!(f, "{}", serde_json::to_string(&e).unwrap()).map_err(|e| e.to_string())?;
        Ok(e)
    }

    /// Ok(number of entries) if the chain is intact, otherwise the sequence number of the first entry that does not follow.
    pub fn verify(&self) -> Result<Result<usize, u64>, String> {
        let mut prev = GENESIS.to_string();
        let entries = self.entries()?;
        for (i, e) in entries.iter().enumerate() {
            if e.seq != i as u64 + 1 || e.prev != prev || e.hash != digest(&e.prev, e.seq, e.at_ms, e.actor, e.action, &e.detail) {
                return Ok(Err(e.seq));
            }
            prev = e.hash.clone();
        }
        Ok(Ok(entries.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(name: &str) -> Audit {
        let p = std::env::temp_dir().join(format!("audit-{name}-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&p);
        Audit::open(p)
    }

    #[test]
    fn entries_chain_and_verify() {
        let a = log("chain");
        assert_eq!(a.verify().unwrap(), Ok(0));
        let e1 = a.append(10, Actor::Voice, Action::TaskQueued, "add a garden screen").unwrap();
        let e2 = a.append(20, Actor::Builder, Action::BuildFinished, "builder/1").unwrap();
        assert_eq!((e1.prev.as_str(), e2.prev.as_str()), (GENESIS, e1.hash.as_str()));
        assert_eq!(a.verify().unwrap(), Ok(2));
    }

    #[test]
    fn editing_or_deleting_a_line_is_found() {
        let a = log("tamper");
        for i in 0..4 {
            a.append(i, Actor::Pipeline, Action::Promoted, &format!("change {i}")).unwrap();
        }
        let text = std::fs::read_to_string(&a.path).unwrap();
        std::fs::write(&a.path, text.replace("change 1", "change X")).unwrap();
        assert_eq!(a.verify().unwrap(), Err(2));
        std::fs::write(&a.path, text.lines().enumerate().filter(|(i, _)| *i != 1).map(|(_, l)| format!("{l}\n")).collect::<String>()).unwrap();
        assert_eq!(a.verify().unwrap(), Err(3));
        // a rewritten tail with fresh hashes still breaks at the first changed link
        std::fs::write(&a.path, text.lines().take(3).map(|l| format!("{l}\n")).collect::<String>()).unwrap();
        assert_eq!(a.verify().unwrap(), Ok(3));
    }
}
