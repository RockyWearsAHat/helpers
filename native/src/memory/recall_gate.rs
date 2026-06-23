//! `memory::recall_gate` — the check that a compaction did not silently lose a concrete
//! fact. It runs *before* a compaction is accepted as a memory item.
//!
//! Strategy: deterministic first. The required facts are the concrete tokens extracted from
//! the original raw text (numbers, dates, ids, names, quoted strings). A compaction passes
//! only if every required fact is recoverable from the compaction's own surface — its
//! summary text or its explicitly extracted facts — by exact, case-insensitive match. No
//! model judgment is involved for these, because losing "2026-08-01" is not a matter of
//! opinion. (Fuzzy/subjective content would be where model probes belong; the MVP needs
//! none.) Crucially, even a *failing* gate never loses information: the raw spans are
//! immutable, so a rejected compaction just means "keep relying on raw / rehydrate."

use super::embed::extract_facts;
use super::types::RecallGateResult;

/// Verify that every concrete fact in `original` survives in `compaction_surface` (the
/// summary text concatenated with the compaction's extracted facts).
///
/// Returns a [`RecallGateResult`] listing exactly which facts were preserved and which were
/// missing, so a rejection is explainable and auditable rather than a bare boolean.
pub fn check(original: &str, compaction_surface: &str) -> RecallGateResult {
    let required = extract_facts(original);
    let haystack = compaction_surface.to_lowercase();
    let mut preserved = Vec::new();
    let mut missing = Vec::new();
    for fact in required {
        if haystack.contains(&fact.to_lowercase()) {
            preserved.push(fact);
        } else {
            missing.push(fact);
        }
    }
    RecallGateResult {
        passed: missing.is_empty(),
        preserved_facts: preserved,
        missing_facts: missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_when_facts_are_carried_in_extracted_facts() {
        // A heavily lossy summary still passes because the concrete facts were extracted.
        let original = "We agreed to ship v0.3.8 to Acme by 2026-08-01.";
        let surface = "agreement reached | v0.3.8 Acme 2026-08-01";
        let r = check(original, surface);
        assert!(r.passed, "facts present in surface must pass: {r:?}");
        assert!(r.missing_facts.is_empty());
    }

    #[test]
    fn fails_and_names_the_missing_fact() {
        let original = "Deadline is 2026-08-01.";
        let surface = "there is a deadline at some point";
        let r = check(original, surface);
        assert!(!r.passed);
        assert!(r.missing_facts.iter().any(|f| f == "2026-08-01"));
    }
}
