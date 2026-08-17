//! The English-equality corroboration judge — the referee the corroboration loop stands on.
//!
//! Contract: `native/history.dx` → north-star section → "The English-equality corroboration judge". Given two
//! English statements (the expected outcome and the actual outcome, both derived back into English by
//! the corroboration loop), decide whether they assert the **same / consistent** thing — same DIRECTION,
//! same POLARITY. The decision reduces ENTIRELY to two frozen substrates: the dictionary meaning graph
//! ([`MeaningNetwork`], for directed cross-reference) and the negation classifier ([`English::is_negation`],
//! for polarity). Never spelling, never a word list.
//!
//! Two properties are load-bearing and enforced by construction:
//!   * **Comparative, never a magic threshold.** Nothing decides "consistent iff score < K". The public
//!     verdicts are a ranking ([`more_consistent`]) and a margin against a caller-supplied foil
//!     ([`corroborates`]). The corroboration engine always has such a foil — the alternative expectation
//!     it also derived — so equality is judged as a margin, not against a constant.
//!   * **Concepts orthogonal until provably linked.** No concept's meaning is weighted into another's; the
//!     judge only ever READS `definition_words`/`centrality`/`has` off the frozen graph and `is_negation`
//!     off the frozen English brain. It never mutates either.
//!
//! Why not flat meaning overlap: [`MeaningNetwork::related`] measures TOPICAL relatedness (shared defining
//! vocabulary), which is orthogonal to what an assertion claims — it rates the false `dog~bird` in the same
//! band as the true `dog~canine`. The right signal is DIRECTED CROSS-REFERENCE through the definitions
//! ([`reference_hops`]): `"a dog is a canine"` is true because `canine`'s definition contains `dog`, while
//! `"a dog is a bird"` has no short directed path. See the contract for the measured competence and its two
//! honest boundaries (synonyms the dictionary never cross-references; lexical negators `is_negation` does
//! not compound).

use std::cmp::Ordering;
use std::collections::{HashSet, VecDeque};

use crate::lint_ai::token_seed;
use crate::lint_char::MeaningNetwork;
use crate::lint_english::English;

/// The SEARCH HORIZON for [`reference_hops`]: the maximum directed-reference hops the BFS explores before
/// giving up. This is a computational bound on the search, NOT a decision threshold — every verdict this
/// module returns is comparative (which candidate orders nearer), so the horizon only bounds cost and sets
/// the unreachable sentinel ([`horizon + 1`](REFERENCE_HORIZON)). Three hops covers direct cross-references
/// (the common case: `canine ⟶ dog`) and short transitive is-a chains (`mammal ⟶ animal`) while capping
/// runaway traversal of the ~470k-entry graph.
const REFERENCE_HORIZON: usize = 3;

/// The content concepts of an English statement: its tokens the bedrock KNOWS ([`MeaningNetwork::has`]),
/// keeping those whose [`centrality`](MeaningNetwork::centrality) is at or above the statement's own MEDIAN.
/// The median cut is comparative — it drops the shared filler ("the", "is", "a") that every statement
/// carries without any stop list, exactly as the meaning bundles already suppress it — while keeping the
/// distinctive words the statement is actually about. Words unknown to English (jargon like `eval`) are
/// dropped here rather than compared, since an honest English judge cannot speak to a word English does not
/// define.
pub fn content_concepts(m: &MeaningNetwork, statement: &str) -> Vec<String> {
    let known: Vec<String> = statement
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .filter(|t| m.has(t))
        .collect();
    if known.is_empty() {
        return known;
    }
    let mut cents: Vec<u32> = known.iter().map(|w| m.centrality(w)).collect();
    cents.sort_unstable();
    let median = cents[cents.len() / 2];
    known.into_iter().filter(|w| m.centrality(w) >= median).collect()
}

/// Shortest DIRECTED-REFERENCE path between `a` and `b` through the dictionary's own definitions, or `None`
/// when none exists within [`REFERENCE_HORIZON`]. A directed edge `X ⟶ Y` holds iff `Y` is a content word of
/// `X`'s [`definition_words`](MeaningNetwork::definition_words); the search is BIDIRECTIONAL (the nearer of
/// `a ⟶ … ⟶ b` and `b ⟶ … ⟶ a`) because the dictionary encodes is-a in both orientations inconsistently —
/// `mammal`'s definition references its hypernym `animal`, while `canine`'s references its hyponym `dog`.
/// 0 = the same word, 1 = a direct cross-reference. This is the assertional signal flat `related` loses: a
/// true is-a has a short directed path, a same-topic falsehood has only a distant convergent hypernym.
pub fn reference_hops(m: &MeaningNetwork, a: &str, b: &str) -> Option<usize> {
    match (directed_hops(m, a, b), directed_hops(m, b, a)) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    }
}

/// One-way directed BFS `from ⟶ … ⟶ to` over definition-reference edges, bounded at [`REFERENCE_HORIZON`].
/// Private: callers use the bidirectional [`reference_hops`]. 0 = same word, `None` = unreachable in bound.
fn directed_hops(m: &MeaningNetwork, from: &str, to: &str) -> Option<usize> {
    let (from, to) = (from.to_lowercase(), to.to_lowercase());
    if from == to {
        return Some(0);
    }
    let mut seen = HashSet::new();
    let mut frontier = VecDeque::from([(from.clone(), 0usize)]);
    seen.insert(from);
    while let Some((word, depth)) = frontier.pop_front() {
        if depth >= REFERENCE_HORIZON {
            continue;
        }
        let Some(words) = m.definition_words(&word) else { continue };
        for next in words {
            let next = next.to_lowercase();
            if next == to {
                return Some(depth + 1);
            }
            if seen.insert(next.clone()) {
                frontier.push_back((next, depth + 1));
            }
        }
    }
    None
}

/// The directed-reference DISTANCE between two concepts: the [`reference_hops`] if reachable, else a sentinel
/// one step past [`REFERENCE_HORIZON`] (comparative — derived from the horizon, never a hand-set score). Lower
/// = the two concepts assert a nearer relation; the sentinel is what a false, unreachable predicate costs.
fn concept_distance(m: &MeaningNetwork, a: &str, b: &str) -> f64 {
    reference_hops(m, a, b).map_or((REFERENCE_HORIZON + 1) as f64, |h| h as f64)
}

/// Directed centrality-weighted chamfer from `from` to `to`: for each concept in `from`, its NEAREST
/// [`concept_distance`] to any concept in `to`, weighted by the source concept's
/// [`centrality`](MeaningNetwork::centrality) so distinctive concepts dominate. `None` when either side has
/// no content concept (nothing English to compare).
fn directed_alignment(m: &MeaningNetwork, from: &[String], to: &[String]) -> Option<f64> {
    if from.is_empty() || to.is_empty() {
        return None;
    }
    let mut num = 0.0;
    let mut den = 0.0;
    for a in from {
        let nearest = to.iter().map(|b| concept_distance(m, a, b)).fold(f64::MAX, f64::min);
        let w = f64::from(m.centrality(a)).max(1.0);
        num += w * nearest;
        den += w;
    }
    (den > 0.0).then(|| num / den)
}

/// Whether a NEGATION OPERATOR governs `statement` — its polarity is NEGATIVE. Decided by the frozen,
/// definition-compounding [`English::is_negation`] over the statement's own tokens (never a negator word
/// list): `"do not use X"` and `"never use X"` are negative, `"use X"` is positive. `is_negation` scopes to
/// OVERT compounded operators (`not`/`never`/`no…`), so a lexical near-negator like `avoid` (whose definition
/// carries no compounded negator) reads POSITIVE — a documented boundary, not a bug.
pub fn is_negated(en: &English, statement: &str) -> bool {
    statement
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .any(|t| en.is_negation(token_seed(&t.to_lowercase())))
}

/// What a sentence CLAIMS about revocation (PASS 30 — the self-referee's atom). Decided by exactly two
/// frozen faculties: the negation classifier ([`is_negated`]) × the author status anchors
/// (`deprecation-status.json` `prohibits`/`removed` — the same data every attestation faculty joins).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationClaim {
    /// The sentence asserts revocation ("X is deprecated", "X was removed in 3.13").
    Asserts,
    /// The sentence DENIES revocation ("X is not deprecated") — a contradiction candidate against a
    /// web node whose revoked role the machine proved.
    Denies,
    /// No revocation assertion either way — remedy prose ("use urllib.parse instead of cgi"),
    /// descriptions, and imperative guidance all read NEUTRAL: the advice register is unproven (thrice
    /// measured), so the referee never manufactures a contradiction from it.
    Neutral,
}

/// Classify `statement`'s revocation claim: a status anchor as a bounded word × the statement's
/// negation polarity. `negated` is passed in so a caller with a precomputed pool polarity
/// (`lint_module`'s sentence pool) never re-runs the classifier. Truth table measured on the real
/// corpus before shipping (native/history.dx PASS 30): (anchor, ¬neg) → Asserts, (anchor, neg) → Denies,
/// no anchor → Neutral.
pub fn revocation_claim(statement: &str, negated: bool, anchors: &[String]) -> RevocationClaim {
    let revokes = statement
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|w| anchors.iter().any(|a| a.eq_ignore_ascii_case(w)));
    match (revokes, negated) {
        (true, false) => RevocationClaim::Asserts,
        (true, true) => RevocationClaim::Denies,
        _ => RevocationClaim::Neutral,
    }
}

/// The CONSISTENCY of two English statements: how nearly they assert the same thing. Ordered
/// LEXICOGRAPHICALLY — `polarity_mismatch` dominates `reference_distance` — so opposite polarity is
/// maximally inconsistent regardless of topic, and among same-polarity statements the directed-reference
/// alignment decides. Lower (via [`Ord`]) = more consistent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Consistency {
    /// `true` when the two statements disagree in polarity (one negates, the other does not) — the
    /// dominant term, so `"do not use eval"` and `"use eval"` never read consistent whatever their words.
    pub polarity_mismatch: bool,
    /// The symmetric centrality-weighted directed-reference chamfer over the statements' content concepts
    /// (lower = a nearer asserted relation). Compared only after polarity agrees.
    pub reference_distance: f64,
}

impl Eq for Consistency {}

impl PartialOrd for Consistency {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Consistency {
    /// Lexicographic: agreement (`false`) before mismatch (`true`), then nearer reference distance first.
    /// `reference_distance` is always finite here (hops or the finite sentinel), so the total order is real.
    fn cmp(&self, other: &Self) -> Ordering {
        self.polarity_mismatch
            .cmp(&other.polarity_mismatch)
            .then(self.reference_distance.total_cmp(&other.reference_distance))
    }
}

/// The CONSISTENCY of statements `a` and `b`: their [`polarity`](is_negated) agreement plus the symmetric
/// directed-reference chamfer over their content concepts. `None` when either statement has no English
/// content concept (the judge has nothing to speak to — an honest "cannot judge", never a false match).
pub fn consistency(m: &MeaningNetwork, en: &English, a: &str, b: &str) -> Option<Consistency> {
    let ca = content_concepts(m, a);
    let cb = content_concepts(m, b);
    let ab = directed_alignment(m, &ca, &cb)?;
    let ba = directed_alignment(m, &cb, &ca)?;
    Some(Consistency {
        polarity_mismatch: is_negated(en, a) != is_negated(en, b),
        reference_distance: (ab + ba) / 2.0,
    })
}

/// The comparative referee primitive: which of `x`, `y` asserts something NEARER the `anchor`?
/// `Ordering::Less` means `x` is more consistent with `anchor` than `y` is (a smaller [`Consistency`]).
/// No absolute threshold — the whole decision is a comparison of two consistencies. `None` when a
/// consistency is undecidable (a statement with no English content concept).
pub fn more_consistent(m: &MeaningNetwork, en: &English, anchor: &str, x: &str, y: &str) -> Option<Ordering> {
    let cx = consistency(m, en, anchor, x)?;
    let cy = consistency(m, en, anchor, y)?;
    Some(cx.cmp(&cy))
}

/// The corroboration verdict: does `actual` corroborate `expected`, judged against a `contrast` foil?
/// True iff `actual` orders STRICTLY nearer `expected` than the `contrast` baseline does — a margin against
/// a foil, never a magic constant. The corroboration loop supplies the foil (the alternative / negated
/// expectation it also derived). Per the measured boundaries (native/architecture.dx), the referee is sound for is-a
/// direction, co-hyponym rejection, and negation flips, but cannot see a synonym the dictionary never
/// cross-references — so the loop must keep the foil genuine and treat an un-cross-referenced synonym as
/// unproven. `None` when a consistency is undecidable.
pub fn corroborates(m: &MeaningNetwork, en: &English, expected: &str, actual: &str, contrast: &str) -> Option<bool> {
    let c_actual = consistency(m, en, expected, actual)?;
    let c_contrast = consistency(m, en, expected, contrast)?;
    Some(c_actual < c_contrast)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lint_char, lint_english};

    /// Both frozen brains, or `None` when the bedrock this judge is defined over is not here (the judge
    /// observes against the real graph or skips honestly — never fakes a pass).
    ///
    /// The MEANING NETWORK's emptiness is part of that check, not just the artifact's presence: measured
    /// 2026-08-17, a `char.global.bin` can be 45MB of read characters and still carry ZERO bound
    /// meanings, because `ensure_brain` binds them inside `if let Some(defs) = dictionary_definitions(…)`
    /// and this machine's dictionary had moved out from under the resolver. Every verdict below then
    /// reads `None` — six identical assertion failures that say "the judge is broken" when what is
    /// actually true is "this machine has no dictionary". Skipping names the real cause and the remedy;
    /// the deterministic guard against the resolver breaking again is
    /// `lint_english::tests::dictionary_resolver_finds_a_body_wherever_the_os_keeps_it`.
    fn brains() -> Option<(&'static lint_char::CharReader, &'static English)> {
        let (br, en) = (lint_char::brain()?, lint_english::brain()?);
        if br.meanings().is_empty() {
            return None;
        }
        Some((br, en))
    }

    /// The one skip message, so a starved machine reads as starved instead of broken.
    const NO_BEDROCK: &str =
        "skip: no dictionary meaning network on this machine — rebuild with \
         `cargo test --release --lib rebuild_machine_brains -- --ignored`";

    #[test]
    fn revocation_claim_reads_the_measured_truth_table() {
        // PASS 30 — the self-referee's atom, the exact table measured on the real corpus (native/history.dx):
        // (anchor, ¬neg) → Asserts; (anchor, neg) → Denies; no anchor → Neutral either polarity. The
        // remedy sentence is the load-bearing Neutral: it must never manufacture a contradiction.
        let anchors = vec!["deprecated".to_string(), "removed".to_string()];
        assert_eq!(revocation_claim("the cgi module is deprecated", false, &anchors), RevocationClaim::Asserts);
        assert_eq!(revocation_claim("cgi was removed in python 3.13", false, &anchors), RevocationClaim::Asserts);
        assert_eq!(revocation_claim("cgi is not deprecated", true, &anchors), RevocationClaim::Denies);
        assert_eq!(revocation_claim("use urllib.parse instead of cgi", false, &anchors), RevocationClaim::Neutral);
        assert_eq!(revocation_claim("do not use the cgi module", true, &anchors), RevocationClaim::Neutral);
        // Bounded word: an anchor inside a longer token never matches.
        assert_eq!(revocation_claim("the undeprecatedish flag", false, &anchors), RevocationClaim::Neutral);
        // No anchors (a machine without the datum) ⇒ honest abstention.
        assert_eq!(revocation_claim("x is deprecated", false, &[]), RevocationClaim::Neutral);
    }

    #[test]
    fn is_a_direction_separates_true_from_same_topic_false() {
        // THE HEADLINE FIX. Flat overlap put dog~canine (3608) and dog~bird (3834) in one band. Directed
        // cross-reference separates them: canine's definition contains "dog" (a true is-a edge), bird's
        // does not. A true restatement must corroborate over the same-topic falsehood.
        let Some((br, en)) = brains() else {
            eprintln!("{NO_BEDROCK}");
            return;
        };
        let m = br.meanings();
        let anchor = "a dog is a canine";
        assert_eq!(
            corroborates(m, en, anchor, "a dog is a canine animal", "a dog is a bird"),
            Some(true),
            "restatement must corroborate over the same-topic is-a falsehood"
        );
        // And the margin is real, not a tie inside a band.
        let good = consistency(m, en, anchor, "a dog is a canine animal").unwrap();
        let bad = consistency(m, en, anchor, "a dog is a bird").unwrap();
        assert!(good < bad, "restatement {good:?} must order below falsehood {bad:?}");
    }

    #[test]
    fn co_hyponym_is_not_an_is_a() {
        // A co-hyponym (fish) is topically near its sibling's subject but is NOT what the subject is — no
        // directed path cat/mammal ⟶ fish. Against a sibling-level anchor the true generalization
        // corroborates over the false sibling.
        let Some((br, en)) = brains() else {
            eprintln!("{NO_BEDROCK}");
            return;
        };
        let m = br.meanings();
        assert_eq!(
            corroborates(m, en, "the cat is a mammal", "the cat is an animal", "the cat is a fish"),
            Some(true),
            "a co-hyponym substitution must not read as the is-a a true generalization does"
        );
    }

    #[test]
    fn negation_flip_dominates_topic() {
        // Polarity is the dominant term: "do not use eval" is corroborated by "never use eval" (same
        // polarity) over "use eval" (opposite polarity), even though "use eval" shares more words. This is
        // the negation FLIP the flat metric was blind to.
        let Some((br, en)) = brains() else {
            eprintln!("{NO_BEDROCK}");
            return;
        };
        let m = br.meanings();
        assert_eq!(
            corroborates(m, en, "do not use eval", "never use eval", "use eval"),
            Some(true),
            "same-polarity restatement must corroborate over the polarity flip"
        );
        // The flip is what carries it: opposite polarity is a mismatch, same polarity is not.
        assert!(consistency(m, en, "do not use eval", "use eval").unwrap().polarity_mismatch);
        assert!(!consistency(m, en, "do not use eval", "never use eval").unwrap().polarity_mismatch);
    }

    #[test]
    fn antonym_is_a_different_assertion() {
        // Antonyms share defining vocabulary (flat overlap reads them near) but assert opposite properties
        // and share no directed reference path. A near-synonym must corroborate over the antonym.
        let Some((br, en)) = brains() else {
            eprintln!("{NO_BEDROCK}");
            return;
        };
        let m = br.meanings();
        assert_eq!(
            corroborates(m, en, "the sun is bright", "the sun is luminous", "the sun is dark"),
            Some(true),
            "a near-synonym must corroborate over the antonym"
        );
    }

    #[test]
    fn identical_statement_is_maximally_consistent() {
        let Some((br, en)) = brains() else {
            eprintln!("{NO_BEDROCK}");
            return;
        };
        let m = br.meanings();
        let c = consistency(m, en, "water is a liquid", "water is a liquid").unwrap();
        assert!(!c.polarity_mismatch);
        assert_eq!(c.reference_distance, 0.0, "a statement is exactly consistent with itself");
    }

    #[test]
    fn no_english_content_is_undecidable_not_a_false_match() {
        // Jargon unknown to English (`eval` alone) yields no content concept — the honest answer is "cannot
        // judge" (None), NOT a false zero-distance match.
        let Some((br, en)) = brains() else {
            eprintln!("{NO_BEDROCK}");
            return;
        };
        let m = br.meanings();
        assert_eq!(consistency(m, en, "zzqx", "qxzz"), None);
    }

    #[test]
    fn honest_boundary_synonym_without_cross_reference_is_unproven() {
        // OBSERVES the documented boundary as data (not a shape assertion): where two true synonyms do NOT
        // cross-reference in the frozen definitions, directed reference cannot see their equality —
        // `liquid`/`fluid` share no edge, so `"water is a fluid"` does NOT order below the false
        // `"water is a gas"` (which IS reachable). If the frozen graph ever cross-references them, this
        // flips and the native/architecture.dx boundary should be revisited.
        let Some((br, en)) = brains() else {
            eprintln!("{NO_BEDROCK}");
            return;
        };
        let m = br.meanings();
        // The synonym `delete`/`remove` DOES cross-reference — that case is sound.
        assert_eq!(
            corroborates(m, en, "delete the file", "remove the file", "create the file"),
            Some(true),
            "a cross-referenced synonym IS corroborated over an unrelated action"
        );
        // But `liquid`/`fluid` do not, so the referee cannot prove the synonym here.
        let fluid = consistency(m, en, "water is a liquid", "water is a fluid").unwrap();
        let gas = consistency(m, en, "water is a liquid", "water is a gas").unwrap();
        assert!(
            !(fluid < gas),
            "documented boundary: un-cross-referenced synonym `fluid` ({fluid:?}) is NOT provably nearer \
             than the false `gas` ({gas:?}); if this ever holds, the graph cross-references them now"
        );
    }
}
