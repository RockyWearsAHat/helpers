//! `memory::concept_index` — files memory under concepts (topics/entities) so retrieval can
//! be scoped by relevance instead of dumping every vaguely related item into context.
//!
//! Context pollution — not lack of memory — is the primary failure mode, so the concept
//! index exists to *narrow*. It maps text to the concepts it mentions (by name or alias),
//! and the retriever fuses that entity-match signal with semantic similarity. The vocabulary
//! grows on its own: salient words seen during ingest become concepts, so no schema has to
//! be declared up front.

use std::collections::HashMap;

use super::embed::tokens;
use super::types::Concept;

/// A growing registry of concepts plus the alias→id lookup that maps text onto them.
#[derive(Default)]
pub struct ConceptIndex {
    concepts: Vec<Concept>,
    /// Alias (lowercased) → concept id.
    alias_to_id: HashMap<String, String>,
}

impl ConceptIndex {
    /// An empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a concept (idempotent on id). Returns the concept id. Aliases and the name
    /// are all indexed so any of them maps text onto this concept.
    pub fn register(&mut self, name: &str, aliases: &[&str], description: &str) -> String {
        let id = normalize_concept(name);
        if !self.concepts.iter().any(|c| c.id == id) {
            self.concepts.push(Concept {
                id: id.clone(),
                name: name.to_string(),
                aliases: aliases.iter().map(|a| a.to_string()).collect(),
                description: description.to_string(),
            });
            self.alias_to_id.insert(id.clone(), id.clone());
            for a in aliases {
                self.alias_to_id.insert(normalize_concept(a), id.clone());
            }
        }
        id
    }

    /// Return the concept ids `text` mentions via any registered name or alias.
    pub fn assign(&self, text: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for tok in tokens(text) {
            if let Some(id) = self.alias_to_id.get(&tok) {
                if !out.contains(id) {
                    out.push(id.clone());
                }
            }
        }
        out
    }

    /// Assign concepts to `text`, auto-registering up to `max_new` new concepts from its
    /// salient tokens (length ≥ 5) so the vocabulary self-organizes during ingest. This is
    /// how a fresh session builds a concept space without a predeclared schema.
    pub fn assign_or_create(&mut self, text: &str, max_new: usize) -> Vec<String> {
        let mut assigned = self.assign(text);
        let mut created = 0;
        for tok in tokens(text) {
            if created >= max_new {
                break;
            }
            if tok.len() >= 5 && !self.alias_to_id.contains_key(&tok) {
                let id = self.register(&tok, &[], "auto-registered from ingest");
                if !assigned.contains(&id) {
                    assigned.push(id);
                }
                created += 1;
            }
        }
        assigned
    }

    /// Overlap of two concept-id sets as a `[0,1]` signal (intersection over query size).
    /// This is the entity-match component of the retriever's fused score.
    pub fn overlap(query_concepts: &[String], item_concepts: &[String]) -> f32 {
        if query_concepts.is_empty() {
            return 0.0;
        }
        let hits = query_concepts
            .iter()
            .filter(|c| item_concepts.contains(c))
            .count();
        hits as f32 / query_concepts.len() as f32
    }

    /// All registered concepts.
    pub fn concepts(&self) -> &[Concept] {
        &self.concepts
    }
}

/// Normalize a concept name/alias to its id form: lowercase, single-token-joined.
fn normalize_concept(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigns_by_name_and_alias() {
        let mut idx = ConceptIndex::new();
        idx.register("deadline", &["due date", "due"], "when work is due");
        assert_eq!(idx.assign("what is the deadline"), vec!["deadline"]);
        assert_eq!(idx.assign("the due date moved"), vec!["deadline"]);
    }

    #[test]
    fn overlap_is_intersection_over_query() {
        let q = vec!["deadline".to_string(), "budget".to_string()];
        let item = vec!["deadline".to_string()];
        assert_eq!(ConceptIndex::overlap(&q, &item), 0.5);
    }
}
