//! `memory::store` — the long-term store: immutable raw spans, versioned memory items, and
//! compactions. This is where the system's unboundedness lives; the live working set never
//! does.
//!
//! Two rules are load-bearing here:
//! * **Raw spans are immutable.** [`MemoryStore::add_raw`] is the only way in, and nothing
//!   ever deletes or rewrites a span — so any fact can be rehydrated from untouched source.
//! * **Knowledge changes are versioned, never silent.** Writing a fact that contradicts an
//!   existing one supersedes the old item (status flip + provenance), it does not overwrite.

use super::embed::fingerprint;
use super::util::IdGen;
use super::types::{Compaction, ItemStatus, MemoryItem, RawSpan, SourceRole};
use super::util::now_iso;
use crate::lint_ai::Hv;

/// A memory item paired with its cached training-free fingerprint, so the retriever never
/// recomputes the hypervector on every query.
struct Indexed {
    item: MemoryItem,
    fingerprint: Hv,
}

/// What a write to memory did, so the caller can audit it precisely.
#[derive(Debug, Default)]
pub struct WriteOutcome {
    /// The id of the active item representing this fact after the write.
    pub id: String,
    /// Ids of items this write superseded (contradiction versioning).
    pub superseded: Vec<String>,
    /// True when the write matched an existing identical fact and added nothing new.
    pub deduped: bool,
}

/// The append-mostly long-term store. Raw spans are append-only and immutable; items are
/// appended and may have their *status* transitioned (active → superseded/contradicted)
/// but their text and provenance are never rewritten.
#[derive(Default)]
pub struct MemoryStore {
    session_id: String,
    raw: Vec<RawSpan>,
    items: Vec<Indexed>,
    compactions: Vec<Compaction>,
    raw_ids: IdGen,
    item_ids: IdGen,
    compaction_ids: IdGen,
}

impl MemoryStore {
    /// A store for one session.
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            raw_ids: IdGen::new("raw"),
            item_ids: IdGen::new("item"),
            compaction_ids: IdGen::new("compaction"),
            ..Default::default()
        }
    }

    /// Ingest one immutable raw span and return its id. The text is stored verbatim and is
    /// never altered afterward.
    pub fn add_raw(
        &mut self,
        role: SourceRole,
        text: &str,
        concept_ids: Vec<String>,
    ) -> String {
        let id = self.raw_ids.mint();
        self.raw.push(RawSpan {
            id: id.clone(),
            session_id: self.session_id.clone(),
            source_role: role,
            text: text.to_string(),
            created_at: now_iso(),
            concept_ids,
        });
        id
    }

    /// Look up a raw span by id — the rehydration path when a summary proves insufficient.
    pub fn get_raw(&self, id: &str) -> Option<&RawSpan> {
        self.raw.iter().find(|s| s.id == id)
    }

    /// All raw spans, in ingest order.
    pub fn raw_spans(&self) -> &[RawSpan] {
        &self.raw
    }

    /// Write a fact into memory, deduplicating exact repeats and versioning contradictions.
    ///
    /// * If an active item has identical normalized text, the write is a no-op dedupe.
    /// * If an active item shares this fact's *subject* (the phrase before "is"/":"/"=")
    ///   but differs, that item is marked [`ItemStatus::Superseded`] and the new item is
    ///   added — knowledge is never silently overwritten.
    pub fn write_memory(
        &mut self,
        text: &str,
        concept_ids: Vec<String>,
        source_span_ids: Vec<String>,
        confidence: f32,
        importance: f32,
    ) -> WriteOutcome {
        let norm = normalize(text);

        // Dedupe: identical active fact already present.
        if let Some(existing) = self
            .items
            .iter()
            .find(|i| i.item.status == ItemStatus::Active && normalize(&i.item.text) == norm)
        {
            return WriteOutcome {
                id: existing.item.id.clone(),
                superseded: Vec::new(),
                deduped: true,
            };
        }

        // Contradiction versioning: supersede active items with the same subject.
        let mut superseded = Vec::new();
        if let Some(subject) = derive_subject(text) {
            for indexed in self.items.iter_mut() {
                if indexed.item.status == ItemStatus::Active
                    && derive_subject(&indexed.item.text).as_deref() == Some(subject.as_str())
                {
                    indexed.item.status = ItemStatus::Superseded;
                    superseded.push(indexed.item.id.clone());
                }
            }
        }

        let id = self.item_ids.mint();
        let now = now_iso();
        let version = superseded.len() as u32 + 1;
        let fp = fingerprint(text);
        self.items.push(Indexed {
            item: MemoryItem {
                id: id.clone(),
                concept_ids,
                text: text.to_string(),
                source_span_ids,
                confidence,
                importance,
                created_at: now.clone(),
                last_accessed_at: now,
                status: ItemStatus::Active,
                version,
            },
            fingerprint: fp,
        });
        WriteOutcome {
            id,
            superseded,
            deduped: false,
        }
    }

    /// All memory items (any status), in write order.
    pub fn items(&self) -> impl Iterator<Item = &MemoryItem> {
        self.items.iter().map(|i| &i.item)
    }

    /// Item plus fingerprint pairs, for the retriever's similarity scan.
    pub fn indexed(&self) -> impl Iterator<Item = (&MemoryItem, &Hv)> {
        self.items.iter().map(|i| (&i.item, &i.fingerprint))
    }

    /// Look up an item by id.
    pub fn get_item(&self, id: &str) -> Option<&MemoryItem> {
        self.items.iter().find(|i| i.item.id == id).map(|i| &i.item)
    }

    /// Record that an item was just retrieved, refreshing its recency for future ranking.
    pub fn touch(&mut self, id: &str) {
        if let Some(i) = self.items.iter_mut().find(|i| i.item.id == id) {
            i.item.last_accessed_at = now_iso();
        }
    }

    /// Persist a compaction record (provenance + extracted facts + gate verdict).
    pub fn add_compaction(
        &mut self,
        source_span_ids: Vec<String>,
        summary_text: String,
        extracted_facts: Vec<String>,
        recall_gate_result: super::types::RecallGateResult,
    ) -> String {
        let id = self.compaction_ids.mint();
        self.compactions.push(Compaction {
            id: id.clone(),
            source_span_ids,
            summary_text,
            extracted_facts,
            recall_gate_result,
            created_at: now_iso(),
            version: 1,
        });
        id
    }

    /// All compactions written so far.
    pub fn compactions(&self) -> &[Compaction] {
        &self.compactions
    }
}

/// Normalize text for dedupe comparison: lowercase, collapse internal whitespace, trim.
fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Derive a fact's subject — the phrase before the first `is` / `:` / `=` separator — used
/// to detect when a new fact contradicts an old one. Returns `None` for facts with no
/// recognizable subject, which are then treated as standalone (never auto-superseded).
fn derive_subject(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    for sep in [" is ", ":", "="] {
        if let Some(idx) = lower.find(sep) {
            let subj = lower[..idx].trim();
            if !subj.is_empty() {
                return Some(subj.split_whitespace().collect::<Vec<_>>().join(" "));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_spans_are_immutable_and_addressable() {
        let mut s = MemoryStore::new("sess");
        let id = s.add_raw(SourceRole::User, "the deadline is august first", vec![]);
        assert_eq!(s.get_raw(&id).unwrap().text, "the deadline is august first");
        // No API exists to mutate text; rehydration always returns the original.
        s.add_raw(SourceRole::Assistant, "noted", vec![]);
        assert_eq!(s.get_raw(&id).unwrap().text, "the deadline is august first");
    }

    #[test]
    fn dedupe_skips_identical_active_facts() {
        let mut s = MemoryStore::new("sess");
        let a = s.write_memory("deadline is august first", vec![], vec![], 1.0, 1.0);
        let b = s.write_memory("Deadline is   August first", vec![], vec![], 1.0, 1.0);
        assert!(b.deduped);
        assert_eq!(a.id, b.id);
    }

    #[test]
    fn contradiction_supersedes_never_overwrites() {
        let mut s = MemoryStore::new("sess");
        let a = s.write_memory("deadline is august first", vec![], vec!["raw-0".into()], 1.0, 1.0);
        let b = s.write_memory("deadline is september tenth", vec![], vec!["raw-9".into()], 1.0, 1.0);
        assert!(b.superseded.contains(&a.id), "old fact must be superseded");
        // Both items still exist; the old one is retained (not deleted) for audit.
        assert_eq!(s.get_item(&a.id).unwrap().status, ItemStatus::Superseded);
        assert_eq!(s.get_item(&b.id).unwrap().status, ItemStatus::Active);
    }
}
