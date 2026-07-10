//! `memory::retriever` — capped, ranked recall. Pulls back only the most relevant memory,
//! never a bulk dump, and returns provenance with every hit.
//!
//! Ranking fuses signals that each catch a different kind of relevance: training-free
//! semantic similarity (hypervector Hamming, the GPU-accelerated part), keyword overlap,
//! concept/entity match, recency, and importance — then scales by confidence and status so
//! superseded or contradicted items sink. Two guards keep context clean: a hard `cap` on
//! how many items can return, and a relevance floor (relative to the best hit) that drops
//! weakly related items entirely. The cap holds **regardless of how much history is stored**
//! — that is what keeps per-step input flat as the session grows.

use super::concept_index::ConceptIndex;
use super::embed::{fingerprint, keyword_overlap, similarity};
use super::gpu;
use super::store::MemoryStore;
use super::types::{ItemStatus, RetrievalResult};

/// Tunable ranking weights and the two pollution guards. Defaults are balanced for the
/// demo; a deployment can retune without touching the algorithm.
#[derive(Debug, Clone)]
pub struct RetrieverConfig {
    /// Hard maximum number of items returned — the flat-cost guarantee.
    pub cap: usize,
    /// Drop items scoring below `floor_ratio × top_score` (relative relevance floor).
    pub floor_ratio: f32,
    /// Weight on semantic (hypervector) similarity.
    pub w_similarity: f32,
    /// Weight on keyword overlap.
    pub w_keyword: f32,
    /// Weight on concept/entity match.
    pub w_entity: f32,
    /// Weight on recency.
    pub w_recency: f32,
    /// Weight on importance.
    pub w_importance: f32,
}

impl Default for RetrieverConfig {
    fn default() -> Self {
        Self {
            cap: 5,
            floor_ratio: 0.6,
            w_similarity: 0.45,
            w_keyword: 0.25,
            w_entity: 0.15,
            w_recency: 0.05,
            w_importance: 0.10,
        }
    }
}

/// Multiplier applied by lifecycle status so stale/superseded/contradicted items sink
/// without being deleted — down-ranking, never silent removal.
fn status_factor(status: ItemStatus) -> f32 {
    match status {
        ItemStatus::Active => 1.0,
        ItemStatus::Stale => 0.6,
        ItemStatus::Superseded => 0.15,
        ItemStatus::Contradicted => 0.10,
    }
}

/// Retrieve the most relevant memory for `query`, capped and ranked, with provenance.
///
/// The returned vector is sorted by descending relevance and never exceeds `cfg.cap`. Items
/// below the relative relevance floor are omitted so unrelated history is not pulled into
/// context. The store is read immutably; the caller is responsible for `touch`-ing the
/// returned ids to refresh recency.
pub fn retrieve(
    query: &str,
    store: &MemoryStore,
    concepts: &ConceptIndex,
    cfg: &RetrieverConfig,
) -> Vec<RetrievalResult> {
    // Snapshot the candidate items and their fingerprints in a stable, parallel order.
    let items: Vec<&super::types::MemoryItem> = store.indexed().map(|(it, _)| it).collect();
    if items.is_empty() {
        return Vec::new();
    }
    let fps: Vec<crate::lint_ai::Hv> = store.indexed().map(|(_, fp)| *fp).collect();

    let query_fp = fingerprint(query);
    let query_concepts = concepts.assign(query);

    // The hot, GPU-accelerable step: Hamming distance to every item fingerprint at once.
    let distances = gpu::query_distances(&query_fp, &fps);

    // Recency normalization over candidates (ISO timestamps sort chronologically).
    let (min_ts, max_ts) = items.iter().fold(
        (items[0].last_accessed_at.as_str(), items[0].last_accessed_at.as_str()),
        |(lo, hi), it| {
            (
                lo.min(it.last_accessed_at.as_str()),
                hi.max(it.last_accessed_at.as_str()),
            )
        },
    );

    let mut scored: Vec<(f32, RetrievalResult)> = Vec::with_capacity(items.len());
    for (idx, item) in items.iter().enumerate() {
        let sem = 1.0 - distances[idx] as f32 / crate::lint_ai::DIM as f32;
        debug_assert!((sem - similarity(&query_fp, &fps[idx])).abs() < 1e-4);
        let kw = keyword_overlap(query, &item.text);
        let ent = ConceptIndex::overlap(&query_concepts, &item.concept_ids);
        // ISO-8601 timestamps sort chronologically, so the most-recently-accessed item
        // equals `max_ts`. A light touch: freshest = 1.0, oldest = 0.0, ties = 0.5.
        let rec = if max_ts == min_ts {
            0.5
        } else if item.last_accessed_at.as_str() == max_ts {
            1.0
        } else if item.last_accessed_at.as_str() == min_ts {
            0.0
        } else {
            0.5
        };
        let base = cfg.w_similarity * sem
            + cfg.w_keyword * kw
            + cfg.w_entity * ent
            + cfg.w_recency * rec
            + cfg.w_importance * item.importance;
        let score = base * status_factor(item.status) * item.confidence;

        let reason = format!(
            "sim={sem:.2} kw={kw:.2} entity={ent:.2} importance={imp:.2} status={st:?}",
            imp = item.importance,
            st = item.status
        );
        scored.push((
            score,
            RetrievalResult {
                memory_item_id: item.id.clone(),
                relevance_score: score,
                reason_selected: reason,
                provenance: item.source_span_ids.clone(),
            },
        ));
    }

    // Rank, apply the relative floor, then enforce the hard cap.
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let top = scored.first().map(|(s, _)| *s).unwrap_or(0.0);
    let floor = top * cfg.floor_ratio;
    scored
        .into_iter()
        .filter(|(s, _)| *s >= floor && *s > 0.0)
        .take(cfg.cap)
        .map(|(_, r)| r)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::SourceRole;

    fn seed() -> (MemoryStore, ConceptIndex) {
        let mut store = MemoryStore::new("sess");
        let mut concepts = ConceptIndex::new();
        let r = store.add_raw(SourceRole::User, "the project deadline is august first", vec![]);
        store.write_memory(
            "the project deadline is august first",
            concepts.assign_or_create("the project deadline is august first", 4),
            vec![r],
            1.0,
            0.9,
        );
        for (i, noise) in [
            "i had a sandwich for lunch",
            "the weather is sunny today",
            "my favorite color is blue",
        ]
        .iter()
        .enumerate()
        {
            let rid = store.add_raw(SourceRole::User, noise, vec![]);
            store.write_memory(noise, concepts.assign_or_create(noise, 4), vec![rid], 1.0, 0.2);
            let _ = i;
        }
        (store, concepts)
    }

    #[test]
    fn retrieves_relevant_and_not_noise() {
        let (store, concepts) = seed();
        let hits = retrieve("when is the project deadline", &store, &concepts, &RetrieverConfig::default());
        assert!(!hits.is_empty(), "must recall the deadline fact");
        assert!(
            store.get_item(&hits[0].memory_item_id).unwrap().text.contains("deadline"),
            "top hit should be the deadline fact, got {:?}",
            hits[0]
        );
        // It must not dump the unrelated sandwich/weather/color facts.
        assert!(hits.len() < 4, "retrieval must be capped/filtered, not a bulk dump");
        // Every hit carries provenance back to a raw span.
        assert!(hits.iter().all(|h| !h.provenance.is_empty()));
    }

    #[test]
    fn respects_hard_cap_regardless_of_store_size() {
        let mut store = MemoryStore::new("sess");
        let mut concepts = ConceptIndex::new();
        for n in 0..2000 {
            let text = format!("fact number {n} about the deadline project");
            let rid = store.add_raw(SourceRole::User, &text, vec![]);
            store.write_memory(&text, concepts.assign_or_create(&text, 4), vec![rid], 1.0, 0.5);
        }
        let cfg = RetrieverConfig { cap: 5, ..Default::default() };
        let hits = retrieve("deadline project", &store, &concepts, &cfg);
        assert!(hits.len() <= 5, "cap must hold no matter how big the store is");
    }
}
