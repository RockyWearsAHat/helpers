//! `memory::compactor` — turns old working-set material into concept-linked, recallable
//! memory while guaranteeing no concrete fact is lost.
//!
//! Pipeline per compaction:
//! 1. Concatenate the evicted raw spans (their text is never altered or deleted).
//! 2. Ask the model for a natural-language summary (compressible, lossy-on-the-margin).
//! 3. Deterministically extract the concrete facts that must not be lost.
//! 4. Build the *surface* = summary + extracted facts, and run the recall gate on it.
//! 5. Persist a [`Compaction`] record (provenance + facts + gate verdict) and write a
//!    recallable memory item whose text embeds the facts, so retrieval can find them later.
//!
//! Every memory item written carries pointers back to the raw spans, so even a perfect
//! summary never severs the path to the untouched original.

use super::concept_index::ConceptIndex;
use super::embed::extract_facts;
use super::model::LanguageModel;
use super::recall_gate;
use super::store::MemoryStore;
use super::types::RawSpan;

/// What a compaction produced, returned so the controller can audit it precisely.
#[derive(Debug)]
pub struct CompactionReport {
    /// The persisted compaction record id.
    pub compaction_id: String,
    /// The natural-language summary the model produced.
    pub summary: String,
    /// The concrete facts that had to survive.
    pub facts: Vec<String>,
    /// Whether the recall gate confirmed every fact survived in the surface.
    pub passed: bool,
    /// Facts the gate found missing from the surface (empty on pass).
    pub missing_facts: Vec<String>,
    /// Raw spans summarized (provenance).
    pub source_span_ids: Vec<String>,
    /// Concepts the compaction was filed under.
    pub concept_ids: Vec<String>,
    /// Memory item ids written as a result (recallable knowledge).
    pub item_ids: Vec<String>,
}

/// Compact a batch of raw spans into memory. `summary_tokens` bounds the summary length so
/// compaction actually shrinks the live footprint. Returns a report describing everything
/// done; nothing here is silent.
pub fn compact(
    spans: &[RawSpan],
    store: &mut MemoryStore,
    concepts: &mut ConceptIndex,
    model: &dyn LanguageModel,
    summary_tokens: usize,
) -> CompactionReport {
    let original: String = spans
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let source_span_ids: Vec<String> = spans.iter().map(|s| s.id.clone()).collect();

    let summary = model.summarize(&original, summary_tokens);
    let facts = extract_facts(&original);

    // Surface = what a reader of the compaction can see. Facts are appended verbatim so the
    // gate's exact-match check can confirm survival even when the summary paraphrases.
    let surface = if facts.is_empty() {
        summary.clone()
    } else {
        format!("{summary} [facts: {}]", facts.join(", "))
    };
    let gate = recall_gate::check(&original, &surface);

    // Concepts: union of the spans' concepts plus any newly salient ones from the text.
    let mut concept_ids: Vec<String> = Vec::new();
    for s in spans {
        for c in &s.concept_ids {
            if !concept_ids.contains(c) {
                concept_ids.push(c.clone());
            }
        }
    }
    for c in concepts.assign_or_create(&original, 4) {
        if !concept_ids.contains(&c) {
            concept_ids.push(c);
        }
    }

    let compaction_id = store.add_compaction(
        source_span_ids.clone(),
        summary.clone(),
        facts.clone(),
        gate.clone(),
    );

    // Write recallable memory. On a clean gate, one item carrying the full surface is the
    // most useful recall target. If the gate ever failed (a fact the surface lost), we fall
    // back to writing each fact as its own item so nothing concrete can slip away.
    let mut item_ids = Vec::new();
    if gate.passed {
        let out = store.write_memory(
            &surface,
            concept_ids.clone(),
            source_span_ids.clone(),
            (0.7 + 0.3 * gate.preserved_facts.len() as f32 / facts.len().max(1) as f32).min(1.0),
            0.7,
        );
        if !out.deduped {
            item_ids.push(out.id);
        }
    } else {
        for fact in &facts {
            let out = store.write_memory(fact, concept_ids.clone(), source_span_ids.clone(), 1.0, 0.6);
            if !out.deduped {
                item_ids.push(out.id);
            }
        }
    }

    CompactionReport {
        compaction_id,
        summary,
        facts,
        passed: gate.passed,
        missing_facts: gate.missing_facts,
        source_span_ids,
        concept_ids,
        item_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::model::MockModel;
    use crate::memory::types::SourceRole;

    #[test]
    fn compaction_preserves_facts_and_keeps_raw() {
        let mut store = MemoryStore::new("sess");
        let mut concepts = ConceptIndex::new();
        let a = store.add_raw(SourceRole::User, "The launch deadline is 2026-08-01 for Acme.", vec![]);
        let b = store.add_raw(SourceRole::Assistant, "Understood, blocking other work until then.", vec![]);
        let spans: Vec<_> = [a.clone(), b.clone()]
            .iter()
            .map(|id| store.get_raw(id).unwrap().clone())
            .collect();

        let report = compact(&spans, &mut store, &mut concepts, &MockModel, 8);
        assert!(report.passed, "gate must confirm fact survival: {report:?}");
        assert!(report.facts.iter().any(|f| f == "2026-08-01"));
        // Raw spans remain addressable after compaction (rehydration path intact).
        assert_eq!(store.get_raw(&a).unwrap().text, "The launch deadline is 2026-08-01 for Acme.");
        // A recallable item now embeds the date.
        let item = store.get_item(&report.item_ids[0]).unwrap();
        assert!(item.text.contains("2026-08-01"));
        assert_eq!(item.source_span_ids, vec![a, b]);
    }
}
