//! The builder's task queue: one JSON file per task in a directory, so it survives restarts and can be read by hand.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Queued,
    Running,
    /// The builder finished and proposed a change; a person (or the policy) has not approved the promotion yet.
    NeedsApproval,
    Done,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub details: String,
    pub status: Status,
    pub created_ms: u64,
    /// What the builder reported (a summary, a branch name, or the failure).
    #[serde(default)]
    pub result: String,
}

pub struct Tasks {
    dir: PathBuf,
}

impl Tasks {
    pub fn open(dir: impl Into<PathBuf>) -> Result<Tasks, String> {
        let dir = dir.into();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(Tasks { dir })
    }

    fn path(&self, id: u64) -> PathBuf {
        self.dir.join(format!("{id:06}.json"))
    }

    pub fn all(&self) -> Result<Vec<Task>, String> {
        let mut out = Vec::new();
        for e in fs::read_dir(&self.dir).map_err(|e| e.to_string())?.flatten() {
            if e.path().extension().and_then(|x| x.to_str()) == Some("json") {
                let t: Task = serde_json::from_str(&fs::read_to_string(e.path()).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                out.push(t);
            }
        }
        out.sort_by_key(|t| t.id);
        Ok(out)
    }

    pub fn enqueue(&self, now_ms: u64, title: &str, details: &str) -> Result<Task, String> {
        let id = self.all()?.last().map_or(1, |t| t.id + 1);
        let t = Task { id, title: title.into(), details: details.into(), status: Status::Queued, created_ms: now_ms, result: String::new() };
        self.save(&t)?;
        Ok(t)
    }

    pub fn save(&self, t: &Task) -> Result<(), String> {
        let tmp = self.path(t.id).with_extension("tmp");
        fs::write(&tmp, serde_json::to_string_pretty(t).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        fs::rename(tmp, self.path(t.id)).map_err(|e| e.to_string())
    }

    /// The oldest queued task.
    pub fn next_queued(&self) -> Result<Option<Task>, String> {
        Ok(self.all()?.into_iter().find(|t| t.status == Status::Queued))
    }

    pub fn set(&self, id: u64, status: Status, result: &str) -> Result<(), String> {
        let mut t = self.all()?.into_iter().find(|t| t.id == id).ok_or("no such task")?;
        t.status = status;
        t.result = result.to_string();
        self.save(&t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tasks_queue_in_order_and_keep_their_results() {
        let dir = std::env::temp_dir().join(format!("tasks-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let q = Tasks::open(&dir).unwrap();
        let a = q.enqueue(1, "first", "do a").unwrap();
        let b = q.enqueue(2, "second", "do b").unwrap();
        assert_eq!((a.id, b.id), (1, 2));
        assert_eq!(q.next_queued().unwrap().unwrap().title, "first");
        q.set(1, Status::Running, "").unwrap();
        assert_eq!(q.next_queued().unwrap().unwrap().title, "second");
        q.set(1, Status::NeedsApproval, "branch builder/1").unwrap();
        let all = q.all().unwrap();
        assert_eq!(all[0].status, Status::NeedsApproval);
        assert_eq!(all[0].result, "branch builder/1");
        // a fresh handle sees the same queue (it is on disk)
        assert_eq!(Tasks::open(&dir).unwrap().all().unwrap().len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }
}
