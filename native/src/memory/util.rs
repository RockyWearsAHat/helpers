//! `memory::util` — tiny shared helpers for the memory subsystem: stable id minting and a
//! timestamp. Kept in one place so ids are uniformly formatted and time has a single source.

/// ISO-8601 UTC timestamp, matching the rest of the crate's `now_iso` format. Timestamps
/// are recorded for audit/recency; nothing asserts on their exact value, so wall-clock
/// time is fine even in tests.
pub fn now_iso() -> String {
    crate::util::now_iso()
}

/// A monotonic, prefixed id source (`raw-0`, `raw-1`, …). One counter per kind of record
/// keeps ids readable and collision-free within a run.
#[derive(Debug, Clone, Default)]
pub struct IdGen {
    prefix: String,
    next: u64,
}

impl IdGen {
    /// Create a generator that mints `"{prefix}-{n}"` starting at 0.
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            next: 0,
        }
    }

    /// Mint the next id.
    pub fn mint(&mut self) -> String {
        let id = format!("{}-{}", self.prefix, self.next);
        self.next += 1;
        id
    }
}
