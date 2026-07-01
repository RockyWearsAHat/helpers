//! `memory::audit` — the append-only, plain-language record of everything the system does.
//!
//! Auditability is an invariant, not a feature: every memory write, compaction, retrieval,
//! eviction, contradiction, output continuation, and controller decision creates exactly
//! one [`AuditEntry`] here, with provenance. Reading the log back must explain the run to a
//! human in plain language — so the entries store sentences, not opcodes.

use super::types::{AuditEntry, EventType};
use super::util::now_iso;

/// An append-only audit log. Entries are never edited or removed once written; the log is
/// the system's tamper-evident narrative of what happened and why.
#[derive(Debug, Default)]
pub struct AuditLog {
    entries: Vec<AuditEntry>,
    next_id: u64,
}

impl AuditLog {
    /// A fresh, empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one event. Returns the id assigned so callers can cross-reference if needed.
    /// `confidence` is `1.0` for deterministic events that carry no uncertainty.
    pub fn record(
        &mut self,
        event_type: EventType,
        description: impl Into<String>,
        source: impl Into<String>,
        concept_ids: Vec<String>,
        confidence: f32,
        provenance: Vec<String>,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(AuditEntry {
            id,
            event_type,
            plain_language_description: description.into(),
            source: source.into(),
            concept_ids,
            confidence,
            timestamp: now_iso(),
            provenance,
        });
        id
    }

    /// All entries in append order.
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Count of entries of a given type — handy for tests asserting that an action of a
    /// certain kind was in fact audited.
    pub fn count(&self, event_type: EventType) -> usize {
        self.entries
            .iter()
            .filter(|e| e.event_type == event_type)
            .count()
    }

    /// Render the whole log as human-readable lines for the demo's per-cycle printout.
    pub fn render(&self) -> String {
        self.entries
            .iter()
            .map(|e| {
                let prov = if e.provenance.is_empty() {
                    String::new()
                } else {
                    format!(" ⟵ {}", e.provenance.join(","))
                };
                format!(
                    "#{:<3} [{:?}] {}{}",
                    e.id, e.event_type, e.plain_language_description, prov
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
