//! The data hub: a keyed store of live values that scripts (feeds, `POST /push`) write and pages follow.
//!
//! Pages long-poll `/sub?since=<seq>`; a write wakes them at once, so updates are event-driven, but the *page* decides
//! when to redraw (on its master beat), so a burst of writes never causes a burst of e-ink refreshes.

use std::collections::BTreeMap;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

struct Entry {
    value: Value,
    ts: u64,
    seq: u64,
}

struct Inner {
    map: BTreeMap<String, Entry>,
    seq: u64,
}

pub struct Hub {
    inner: Mutex<Inner>,
    cv: Condvar,
}

impl Hub {
    pub fn new() -> Hub {
        // seq starts from the clock so a restarted server is always ahead of what a page last saw
        Hub { inner: Mutex::new(Inner { map: BTreeMap::new(), seq: now_ms() }), cv: Condvar::new() }
    }

    /// Write one key. Every write bumps the sequence (even an unchanged value: it refreshes the key's age);
    /// the page decides whether a changed value needs a redraw.
    pub fn set(&self, key: &str, value: Value, ts: Option<u64>) {
        let mut g = self.inner.lock().unwrap();
        g.seq += 1;
        let seq = g.seq;
        g.map.insert(key.to_string(), Entry { value, ts: ts.unwrap_or_else(now_ms), seq });
        drop(g);
        self.cv.notify_all();
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        self.inner.lock().unwrap().map.get(key).map(|e| e.value.clone())
    }

    /// Everything, as `{key: {value, ts}}`.
    pub fn snapshot(&self) -> Value {
        let g = self.inner.lock().unwrap();
        Value::Object(g.map.iter().map(|(k, e)| (k.clone(), json!({ "value": e.value, "ts": e.ts }))).collect())
    }

    /// Updates newer than `since`: returns at once if there are any (or `since` is 0, a first connect), otherwise waits
    /// up to `timeout` for one. Result: the current sequence and `[key, value, ts]` rows.
    pub fn wait(&self, since: u64, timeout: Duration) -> (u64, Vec<(String, Value, u64)>) {
        let deadline = Instant::now() + timeout;
        let mut g = self.inner.lock().unwrap();
        loop {
            let from = if since > g.seq { 0 } else { since };
            let rows: Vec<_> = g.map.iter().filter(|(_, e)| e.seq > from).map(|(k, e)| (k.clone(), e.value.clone(), e.ts)).collect();
            if !rows.is_empty() || since == 0 {
                return (g.seq, rows);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return (g.seq, rows);
            }
            g = self.cv.wait_timeout(g, left).unwrap().0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn first_connect_gets_everything_then_only_news() {
        let hub = Hub::new();
        hub.set("a", json!(1), None);
        hub.set("b", json!("x"), None);
        let (seq, rows) = hub.wait(0, Duration::from_millis(10));
        assert_eq!(rows.len(), 2);
        let (seq2, rows) = hub.wait(seq, Duration::from_millis(20));
        assert_eq!(seq2, seq);
        assert!(rows.is_empty(), "nothing new: times out empty");
        hub.set("a", json!(2), None);
        let (_, rows) = hub.wait(seq, Duration::from_millis(10));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "a");
    }

    #[test]
    fn a_write_wakes_a_waiting_page() {
        let hub = Arc::new(Hub::new());
        let (seq, _) = hub.wait(0, Duration::from_millis(1));
        let h2 = hub.clone();
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            h2.set("wifi.rssi", json!(-61), None);
        });
        let started = Instant::now();
        let (_, rows) = hub.wait(seq, Duration::from_secs(5));
        assert_eq!(rows[0].0, "wifi.rssi");
        assert!(started.elapsed() < Duration::from_secs(2), "woken by the write, not the timeout");
        t.join().unwrap();
    }

    #[test]
    fn a_page_ahead_of_a_restarted_server_resyncs() {
        let hub = Hub::new();
        hub.set("k", json!(1), None);
        let (_, rows) = hub.wait(u64::MAX / 2, Duration::from_millis(10));
        assert_eq!(rows.len(), 1, "a `since` from the future means the server restarted: send everything");
    }
}
