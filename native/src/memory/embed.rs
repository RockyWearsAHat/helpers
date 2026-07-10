//! `memory::embed` — training-free text fingerprints and deterministic fact extraction.
//!
//! The retriever ranks by semantic similarity, and that similarity must work **without any
//! extra training**. We get it for free from hyperdimensional computing: every token maps
//! to a fixed random 8192-bit hypervector (a hash, not a learned embedding), and a span's
//! fingerprint is the majority-vote *bundle* of its token vectors. Two texts that share
//! vocabulary land close in Hamming space; unrelated texts sit ~half the bits apart. No
//! corpus, no gradient, no model — just hashing and popcount, which is also why the
//! similarity search maps cleanly onto the GPU popcount kernel.
//!
//! Fact extraction is likewise deterministic: regex/scan for the concrete things a summary
//! must not lose — numbers, dates, IDs, emails, capitalized names, and quoted strings — so
//! the recall gate can check survival with exact string matching, never a model guess.

use crate::lint_ai::{token_hv, Bundler, Hv};

/// Lowercase, split into alphanumeric tokens. Deterministic and Unicode-agnostic on
/// purpose: identical inputs always yield identical tokens, so fingerprints are stable.
pub fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect()
}

/// The training-free fingerprint of a text span: the bundled hypervector of its tokens.
///
/// Bundling is a bitwise majority vote, so the result is a single [`Hv`] that is close (in
/// Hamming distance) to every token it contains and therefore to any query sharing those
/// tokens. Empty text yields the zero vector, which is maximally far from real content.
pub fn fingerprint(text: &str) -> Hv {
    let mut bundler = Bundler::new();
    for tok in tokens(text) {
        bundler.add(&token_hv(&tok));
    }
    if bundler.is_empty() {
        Hv::zero()
    } else {
        bundler.finalize()
    }
}

/// Semantic similarity in `[0, 1]` from Hamming distance: `1 - dist/DIM`. Identical
/// fingerprints score 1.0; unrelated ones hover near 0.5 (random codes differ in ~half
/// their bits). Cheap, training-free, and monotonic — all the retriever needs.
pub fn similarity(a: &Hv, b: &Hv) -> f32 {
    let dist = a.distance(b) as f32;
    1.0 - dist / crate::lint_ai::DIM as f32
}

/// Fraction of the query's distinct tokens that appear in `text` — the keyword-overlap
/// signal the retriever fuses with semantic similarity. Returns `0.0` for an empty query.
pub fn keyword_overlap(query: &str, text: &str) -> f32 {
    let q = tokens(query);
    if q.is_empty() {
        return 0.0;
    }
    let hay: std::collections::HashSet<String> = tokens(text).into_iter().collect();
    let mut distinct: std::collections::HashSet<&String> = std::collections::HashSet::new();
    let mut hit = 0usize;
    for t in &q {
        if distinct.insert(t) && hay.contains(t) {
            hit += 1;
        }
    }
    hit as f32 / distinct.len() as f32
}

/// Extract the concrete facts a compaction must preserve. These are the tokens whose loss
/// would silently corrupt memory: numbers, dates, IDs, emails, quoted strings, and
/// capitalized proper nouns. The recall gate later checks each of these survived by exact
/// match, so extraction is intentionally deterministic and conservative.
pub fn extract_facts(text: &str) -> Vec<String> {
    let mut facts: Vec<String> = Vec::new();
    let push = |s: String, facts: &mut Vec<String>| {
        if !s.is_empty() && !facts.contains(&s) {
            facts.push(s);
        }
    };

    // Quoted strings (single or double) — promises and exact phrasings often live here.
    for (open, close) in [('"', '"'), ('\'', '\'')] {
        let mut rest = text;
        while let Some(i) = rest.find(open) {
            let after = &rest[i + open.len_utf8()..];
            if let Some(j) = after.find(close) {
                push(after[..j].trim().to_string(), &mut facts);
                rest = &after[j + close.len_utf8()..];
            } else {
                break;
            }
        }
    }

    // Tokenize on whitespace so we keep things like `2026-08-01`, `v0.3.8`, `$2,500`,
    // `TICKET-42`, and `a@b.com` intact as single facts.
    for raw in text.split_whitespace() {
        let word = raw.trim_matches(|c: char| matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')' | '('));
        if word.is_empty() {
            continue;
        }
        let has_digit = word.chars().any(|c| c.is_ascii_digit());
        let is_email = word.contains('@') && word.contains('.');
        let is_idish = word.contains('-') && word.chars().any(|c| c.is_ascii_uppercase());
        // Capitalized proper noun (length-gated to skip sentence-initial filler like "The").
        let is_proper = word.len() >= 4
            && word.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            && word.chars().skip(1).any(|c| c.is_ascii_lowercase());
        if has_digit || is_email || is_idish || is_proper {
            push(word.to_string(), &mut facts);
        }
    }
    facts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn related_text_is_closer_than_unrelated() {
        let base = fingerprint("the project deadline is august first");
        let related = fingerprint("when is the project deadline");
        let unrelated = fingerprint("bananas grow in tropical climates");
        assert!(
            similarity(&base, &related) > similarity(&base, &unrelated),
            "related text must fingerprint closer than unrelated"
        );
    }

    #[test]
    fn extracts_dates_numbers_ids_and_names() {
        let facts = extract_facts("Ship v0.3.8 to Acme by 2026-08-01; ping alex@x.com about TICKET-42.");
        for needle in ["v0.3.8", "2026-08-01", "alex@x.com", "TICKET-42", "Acme"] {
            assert!(facts.iter().any(|f| f == needle), "missing fact {needle} in {facts:?}");
        }
    }

    #[test]
    fn keyword_overlap_counts_shared_distinct_tokens() {
        assert_eq!(keyword_overlap("project deadline", "the deadline of the project"), 1.0);
        assert_eq!(keyword_overlap("project deadline", "bananas and mangoes"), 0.0);
    }
}
