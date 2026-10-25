use crate::{Meta, Result, Store};

/// A live runtime (the browser page) that can evaluate a change and say whether it was accepted.
pub trait Runtime {
    /// Evaluate TypeScript `source` as one change and verify it fits. `Err` means it was
    /// rejected or failed; the runtime has already rolled it back.
    fn eval_verify(&mut self, source: &str) -> std::result::Result<(), String>;
    /// DOM snapshot after the last accepted change.
    fn snapshot(&mut self) -> String;
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReplayReport {
    /// Journal entries that replayed cleanly.
    pub replayed: usize,
    /// Entries that no longer verify (seq, reason). Kept in the journal and reported; boot never blocks.
    pub broken: Vec<(u64, String)>,
    /// Pending evals left by a crash that verified and were committed.
    pub recovered: Vec<u64>,
    /// Pending evals left by a crash that did not verify and were discarded.
    pub dropped: Vec<u64>,
}

/// Rebuild runtime state from the store: replay the journal in order (each entry through
/// verify), then settle any eval that was pending when the process last died.
pub fn boot_replay(store: &Store, rt: &mut dyn Runtime) -> Result<ReplayReport> {
    let mut report = ReplayReport::default();

    for entry in store.tail()? {
        match rt.eval_verify(&entry.source) {
            Ok(()) => report.replayed += 1,
            Err(why) => report.broken.push((entry.seq, why)),
        }
    }

    // Pending evals are newer than everything in the tail, so they go last.
    for pending in store.pending()? {
        let seq = pending.seq;
        match rt.eval_verify(&pending.source) {
            Ok(()) => {
                let snapshot = rt.snapshot();
                let meta = Meta { intent: Some("recovered pending eval".into()), ..Meta::default() };
                store.commit(pending, &snapshot, &meta)?;
                report.recovered.push(seq);
            }
            Err(_) => {
                store.abort(pending)?;
                report.dropped.push(seq);
            }
        }
    }
    Ok(report)
}
