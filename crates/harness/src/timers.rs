//! Kitchen timers: several can run; the display shows the next one to ring (K.timer.*), and the harness says so when one does.

use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timer {
    pub id: u64,
    pub label: String,
    pub total_s: u64,
    pub end_ms: u64,
}

#[derive(Default)]
pub struct Timers {
    items: Vec<Timer>,
    next_id: u64,
}

impl Timers {
    pub fn start(&mut self, now_ms: u64, seconds: u64, label: &str) -> Timer {
        self.next_id += 1;
        let t = Timer { id: self.next_id, label: label.to_string(), total_s: seconds, end_ms: now_ms + seconds * 1000 };
        self.items.push(t.clone());
        t
    }

    /// Cancel by label (case-insensitive) or, with an empty label, the one that rings next. True if something was cancelled.
    pub fn cancel(&mut self, label: &str) -> bool {
        let before = self.items.len();
        if label.trim().is_empty() {
            if let Some(next) = self.current().map(|t| t.id) {
                self.items.retain(|t| t.id != next);
            }
        } else {
            self.items.retain(|t| !t.label.eq_ignore_ascii_case(label.trim()));
        }
        self.items.len() != before
    }

    /// The timers that have rung by `now_ms`, removed from the list, in the order they rang.
    pub fn due(&mut self, now_ms: u64) -> Vec<Timer> {
        let (done, left): (Vec<_>, Vec<_>) = self.items.drain(..).partition(|t| t.end_ms <= now_ms);
        self.items = left;
        let mut done = done;
        done.sort_by_key(|t| t.end_ms);
        done
    }

    /// The timer that rings next.
    pub fn current(&self) -> Option<&Timer> {
        self.items.iter().min_by_key(|t| t.end_ms)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The live data the display needs: `timer.end` (epoch ms or null), `timer.total` (seconds), `timer.label`.
    pub fn to_push(&self) -> Value {
        match self.current() {
            Some(t) => json!({ "timer.end": t.end_ms, "timer.total": t.total_s, "timer.label": t.label }),
            None => json!({ "timer.end": null, "timer.total": 0, "timer.label": "" }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_next_to_ring_is_current_and_due_removes_what_rang() {
        let mut t = Timers::default();
        t.start(1_000, 600, "pasta");
        t.start(1_000, 390, "egg");
        assert_eq!(t.current().unwrap().label, "egg");
        assert_eq!(t.to_push()["timer.end"], 391_000);
        assert!(t.due(300_000).is_empty());
        let rang = t.due(391_000);
        assert_eq!(rang.len(), 1);
        assert_eq!(rang[0].label, "egg");
        assert_eq!(t.current().unwrap().label, "pasta");
    }

    #[test]
    fn cancel_by_label_or_the_next_one() {
        let mut t = Timers::default();
        t.start(0, 60, "a");
        t.start(0, 30, "b");
        assert!(t.cancel("A"));
        assert_eq!(t.len(), 1);
        assert!(t.cancel(""));
        assert!(t.is_empty());
        assert!(!t.cancel("nothing"));
        assert_eq!(t.to_push()["timer.end"], Value::Null);
    }
}
