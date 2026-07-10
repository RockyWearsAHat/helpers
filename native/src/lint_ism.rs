//! The corroboration engine — the ISM's graduation gate.
//!
//! Contract: `LINTER.md` → north-star section → "The corroboration engine — the graduation gate". A
//! candidate understanding is not "learned" because it was predicted once; it GRADUATES to PROVEN only
//! when it is mirrored **≥ [`REQUIRED_WITNESSES`] times INDEPENDENTLY**, none contradicting (the anti-
//! Dunning-Kruger law, north-star step 5). This module turns that law into a mechanism: it takes a
//! [`Candidate`] truth and a stream of English witness statements and returns a [`Verdict`].
//!
//! It adds NO judgement of its own. Every per-witness decision is a call into the frozen
//! [`crate::lint_corroborate`] comparator (the proven English-equality referee); this module only
//! CLASSIFIES each witness through that comparator, then COUNTS distinct corroborations and GATES.
//!
//! Two properties are load-bearing, inherited from the comparator and enforced by construction:
//!   * **Comparative, never a magic threshold.** No witness is judged against a distance constant. On-
//!     topic-ness is a strict compare of two `reference_distance`s (witness-vs-truth against foil-vs-
//!     truth), exactly as [`crate::lint_corroborate::corroborates`] compares. The only literal is
//!     [`REQUIRED_WITNESSES`] — the OWNER-SPECIFIED witness COUNT (a spec parameter), not a tuned radius.
//!   * **A genuine foil is required.** The comparator is comparative, so a graduation gate needs the
//!     ALTERNATIVE expectation the corroboration loop also derived. A [`Candidate`] therefore carries its
//!     own `foil`; the engine is only as sound as that foil is genuine (see the contract).
//!
//! Safety: the un-cross-referenced-synonym boundary of the comparator is PRESERVED — a witness the
//! comparator cannot see as nearer the truth than the foil never corroborates, so a candidate whose only
//! support is such a synonym never graduates (verified in tests and `examples/relcheck.rs`).

use crate::lint_char::MeaningNetwork;
use crate::lint_corroborate::consistency;
use crate::lint_english::English;

/// The OWNER-SPECIFIED number of genuinely independent, non-contradicting witnesses a candidate truth must
/// gather to GRADUATE to PROVEN (`LINTER.md` north-star step 5). This is a spec PARAMETER — the count that
/// separates a one-off overfit from an independently reproduced truth — NOT a tuned distance threshold. The
/// per-witness corroboration and the independence test are entirely comparative; only this count is a
/// literal, and it is the owner's, cited here as such.
pub const REQUIRED_WITNESSES: usize = 15;

/// A candidate understanding under test, paired with the genuine ALTERNATIVE the corroboration loop also
/// derived. The comparator is comparative — it judges a witness NEARER the `truth` than a foil, never
/// against a constant — so graduation needs both sides. `foil` MUST be a genuine competing assertion on the
/// truth's topic (e.g. the negated/alternative expectation); a degenerate or unrelated foil invalidates the
/// verdict, exactly as the comparator's own contract requires.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// The English statement whose truth is being corroborated.
    pub truth: String,
    /// The genuine competing assertion (the derived alternative expectation) the witness is scored against.
    pub foil: String,
}

impl Candidate {
    /// A candidate truth with its genuine competing foil. Convenience constructor over the two fields.
    pub fn new(truth: impl Into<String>, foil: impl Into<String>) -> Self {
        Self { truth: truth.into(), foil: foil.into() }
    }
}

/// How a single witness stands to a [`Candidate`], decided ENTIRELY by the frozen comparator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corroboration {
    /// The witness is on the truth's topic and agrees in polarity — it asserts the same thing. Counts
    /// toward graduation once proven INDEPENDENT of the witnesses already counted.
    Corroborates,
    /// The witness asserts the OPPOSITE (a polarity flip of the truth) or asserts the FOIL instead — an
    /// incompatible assertion. A single contradiction BLOCKS graduation.
    Contradicts,
    /// The witness is off both the truth's and the foil's topic — it speaks to neither, so it neither
    /// counts nor blocks.
    Neutral,
    /// The comparator cannot judge (a side has no English content concept) — an honest "cannot judge",
    /// never a false match.
    Undecidable,
}

/// The graduation verdict for a candidate against its witness stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// At least [`REQUIRED_WITNESSES`] genuinely independent witnesses corroborated the truth and none
    /// contradicted it — the truth graduates from predicted to PROVEN.
    Proven,
    /// The truth did not graduate; the [`Unproven`] reason says why. Per the ISM commit discipline, an
    /// un-corroborated state is never committed.
    Unproven(Unproven),
}

/// Why a candidate did NOT graduate — the honest reason, so the loop knows whether to learn more, discard,
/// or that it simply cannot judge from the substrate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unproven {
    /// A witness CONTRADICTED the truth (polarity flip or asserts-the-foil). A proven truth has no
    /// contradicting witness, so one is fatal; the offending witness is carried for the report.
    Contradicted {
        /// The witness statement that contradicted the candidate truth.
        witness: String,
    },
    /// No contradiction, but fewer than [`REQUIRED_WITNESSES`] genuinely INDEPENDENT witnesses corroborated
    /// (repeats of one phrasing collapse to a single independent witness).
    TooFewIndependent {
        /// The count of distinct, independent corroborating witnesses actually found.
        independent: usize,
        /// The owner-specified count required to graduate ([`REQUIRED_WITNESSES`]).
        required: usize,
    },
    /// The candidate itself is undecidable against its foil — a side carries no English content concept, so
    /// the comparator (and therefore this engine) cannot speak to it at all.
    Undecidable,
}

/// The directed-reference distance of `b` from `a`, ignoring polarity, or `None` when the comparator cannot
/// How `witness` stands to `candidate`, decided by the frozen comparator and two strict distance compares —
/// no distance constant anywhere (see the module contract). Against the TRUTH: on-topic = the witness's
/// content is directed-reference-nearer the truth than the FOIL's content is, and corroboration additionally
/// needs polarity agreement; an on-topic polarity flip is a contradiction. Off the truth's topic: asserting
/// the FOIL (nearer the foil than the truth is, same polarity as the foil) is also a contradiction, else
/// neutral. `Undecidable` when the comparator returns `None` (a side has no English content to judge).
pub fn classify(m: &MeaningNetwork, en: &English, candidate: &Candidate, witness: &str) -> Corroboration {
    // Foil-relative baseline: how near the truth the FOIL's own content sits. The witness must beat it.
    let (Some(truth_witness), Some(truth_foil)) = (
        consistency(m, en, &candidate.truth, witness),
        consistency(m, en, &candidate.truth, &candidate.foil),
    ) else {
        return Corroboration::Undecidable;
    };

    if truth_witness.reference_distance < truth_foil.reference_distance {
        // On the truth's topic — a polarity flip contradicts, agreement corroborates.
        return if truth_witness.polarity_mismatch {
            Corroboration::Contradicts
        } else {
            Corroboration::Corroborates
        };
    }

    // Off the truth's topic: does the witness assert the FOIL (the competing alternative) instead?
    if let (Some(foil_witness), Some(foil_truth)) = (
        consistency(m, en, &candidate.foil, witness),
        consistency(m, en, &candidate.foil, &candidate.truth),
    ) {
        if foil_witness.reference_distance < foil_truth.reference_distance && !foil_witness.polarity_mismatch {
            return Corroboration::Contradicts;
        }
    }
    Corroboration::Neutral
}

/// Whether `witness` is a DISTINCT witness from `prior` — genuinely different material, not a near-duplicate.
/// Two statements are the SAME witness iff they reduce to a directed-reference-IDENTICAL assertion: same
/// polarity AND `reference_distance == 0` (the identity point — every content concept at hop 0 to the other
/// side). This is the comparator's own identity element (`consistency(s, s).reference_distance == 0`), NOT a
/// tuned radius. `true` (independent) when the comparator cannot compare them (no shared English content) —
/// they cannot be the same assertion if the judge cannot even relate them.
fn is_independent_of(m: &MeaningNetwork, en: &English, witness: &str, prior: &str) -> bool {
    match consistency(m, en, witness, prior) {
        Some(c) => c.polarity_mismatch || c.reference_distance != 0.0,
        None => true,
    }
}

/// Graduate `candidate` against its `witnesses` stream per the ISM law: PROVEN iff at least
/// [`REQUIRED_WITNESSES`] genuinely INDEPENDENT witnesses [`Corroboration::Corroborates`] AND none
/// [`Corroboration::Contradicts`]; else [`Verdict::Unproven`] with the reason. A single contradiction is
/// fatal and short-circuits. Distinctness is measured by [`is_independent_of`] (repeats collapse to one).
/// [`Corroboration::Neutral`] witnesses are ignored; [`Corroboration::Undecidable`] on a witness is ignored,
/// but an undecidable CANDIDATE (its truth/foil carry no English content) yields [`Unproven::Undecidable`].
pub fn graduate<I, S>(m: &MeaningNetwork, en: &English, candidate: &Candidate, witnesses: I) -> Verdict
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    // The candidate must be judgeable at all: truth and foil must share enough English for the comparator.
    if consistency(m, en, &candidate.truth, &candidate.foil).is_none() {
        return Verdict::Unproven(Unproven::Undecidable);
    }

    let mut independent: Vec<String> = Vec::new();
    for witness in witnesses {
        let witness = witness.as_ref();
        match classify(m, en, candidate, witness) {
            Corroboration::Contradicts => {
                return Verdict::Unproven(Unproven::Contradicted { witness: witness.to_string() });
            }
            Corroboration::Corroborates => {
                if independent.iter().all(|prior| is_independent_of(m, en, witness, prior)) {
                    independent.push(witness.to_string());
                }
            }
            Corroboration::Neutral | Corroboration::Undecidable => {}
        }
    }

    if independent.len() >= REQUIRED_WITNESSES {
        Verdict::Proven
    } else {
        Verdict::Unproven(Unproven::TooFewIndependent {
            independent: independent.len(),
            required: REQUIRED_WITNESSES,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lint_char, lint_english};

    /// Both frozen brains, or `None` when an artifact is not on disk (the engine is defined only over the
    /// real bedrock, so tests observe against it or skip honestly — never fake a pass).
    fn brains() -> Option<(&'static lint_char::CharReader, &'static English)> {
        Some((lint_char::brain()?, lint_english::brain()?))
    }

    /// Fifteen genuinely different English phrasings that all assert "a dog is a canine" — varied structure,
    /// vocabulary, and predicate, so each is distinct material, not a paraphrase of one template.
    fn dog_is_canine_witnesses() -> Vec<&'static str> {
        vec![
            "a dog is a canine",
            "the dog belongs to the canine family",
            "every dog is a canine animal",
            "a hound is a canine creature",
            "the domestic dog is classified as a canine",
            "a puppy grows into a canine",
            "a dog is a member of the canine group",
            "the dog is a kind of canine",
            "a dog counts as a canine mammal",
            "our pet dog is a canine",
            "the dog species is canine",
            "a dog is fundamentally a canine",
            "a dog remains a canine",
            "a beagle is a canine",
            "the hound is a canine breed",
            "a terrier is a canine",
            "the shepherd dog is a canine",
            "a dog is a domesticated canine",
        ]
    }

    #[test]
    fn fifteen_independent_witnesses_graduate_to_proven() {
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        let candidate = Candidate::new("a dog is a canine", "a dog is a bird");
        let verdict = graduate(m, en, &candidate, dog_is_canine_witnesses());
        assert_eq!(verdict, Verdict::Proven, "≥15 independent corroborations must graduate");
    }

    #[test]
    fn one_contradiction_blocks_graduation() {
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        let candidate = Candidate::new("a dog is a canine", "a dog is a bird");
        let mut witnesses = dog_is_canine_witnesses();
        witnesses.push("a dog is a bird"); // asserts the foil — a contradiction
        match graduate(m, en, &candidate, witnesses) {
            Verdict::Unproven(Unproven::Contradicted { .. }) => {}
            other => panic!("a contradicting witness must block graduation, got {other:?}"),
        }
    }

    #[test]
    fn repeated_phrasing_is_one_independent_witness() {
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        let candidate = Candidate::new("a dog is a canine", "a dog is a bird");
        let witnesses: Vec<&str> = std::iter::repeat("a dog is a canine").take(15).collect();
        match graduate(m, en, &candidate, witnesses) {
            Verdict::Unproven(Unproven::TooFewIndependent { independent, required }) => {
                assert_eq!(independent, 1, "the same phrasing 15× is ONE independent witness");
                assert_eq!(required, REQUIRED_WITNESSES);
            }
            other => panic!("15 identical witnesses must not graduate, got {other:?}"),
        }
    }

    #[test]
    fn ten_good_witnesses_are_too_few() {
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        let candidate = Candidate::new("a dog is a canine", "a dog is a bird");
        let ten: Vec<&str> = dog_is_canine_witnesses().into_iter().take(10).collect();
        match graduate(m, en, &candidate, ten) {
            Verdict::Unproven(Unproven::TooFewIndependent { independent, required }) => {
                assert!(independent < required, "10 independent witnesses is below the required {required}");
            }
            other => panic!("10 witnesses must not graduate, got {other:?}"),
        }
    }

    #[test]
    fn un_cross_referenced_synonym_never_graduates_safety() {
        // SAFETY PROPERTY. `liquid`/`fluid` share no directed edge in the frozen dictionary, so the
        // comparator cannot see `"water is a fluid"` as nearer the truth than the reachable foil
        // `"water is a gas"`. The witness must therefore NOT corroborate, and a candidate supported only by
        // it must stay UNPROVEN — the engine cannot manufacture a proof the comparator cannot see.
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        let candidate = Candidate::new("water is a liquid", "water is a gas");
        assert_ne!(
            classify(m, en, &candidate, "water is a fluid"),
            Corroboration::Corroborates,
            "un-cross-referenced synonym `fluid` must NOT read as corroboration"
        );
        let witnesses: Vec<&str> = std::iter::repeat("water is a fluid").take(15).collect();
        match graduate(m, en, &candidate, witnesses) {
            Verdict::Unproven(_) => {}
            Verdict::Proven => panic!("the synonym boundary must never graduate a candidate"),
        }
    }
}
