//! The self-generated test loop — proving a learned rule by GENERATING violations and LINTING them.
//!
//! Contract: `LINTER.md` → north-star section → "The self-generated test loop". A rule does not
//! graduate because the docs RESTATE it; it graduates because the AI proves WHAT IT UNDERSTANDS by
//! generating code that embodies the violation, running the REAL linter over it, and reconciling the
//! violation the linter FINDS with the rule it LEARNED — **in English**, the incorruptible in-between
//! (owner directive 2026-07-11). You cannot fake equality in a language you truly understand.
//!
//! It adds NO new judgement. The firing check is the frozen [`crate::lint_trace::run_plan`]; the
//! English equality is the frozen [`crate::lint_corroborate::corroborates`] comparator; this module
//! only orchestrates them into the loop and COUNTS the reps.
//!
//! Two properties are load-bearing:
//!   * **Two independent, un-fakeable signals per rep.** A rep [`Rep::Corroborates`] only when BOTH
//!     the real linter flags the self-generated code (behavioral — you can only generate flagging code
//!     for a rule you understand) AND the fired rule's advice reconciles with the learned understanding
//!     in English (semantic). A facade fails one or both — proven by the MEASURED faked-understanding
//!     control (`examples/selftest_probe.rs`), which is behaviorally flagged yet rejected in English.
//!   * **Counting is by DISTINCT self-generated code, not by English restatements.** The independence
//!     axis is the varied violating code, verified by the real linter firing on each. The found advice
//!     is the SAME English every rep, so `lint_ism`'s identity-merge (which would collapse it to one
//!     witness) is deliberately NOT reused; this module reuses only the frozen English gate.

use crate::lint_char::MeaningNetwork;
use crate::lint_corroborate::corroborates;
use crate::lint_english::English;
use crate::lint_trace::{run_plan, Plan};

/// The OWNER-SPECIFIED number of self-generated, really-linted, English-reconciled reps a learned rule
/// must pass to GRADUATE (owner correction 2026-07-12, point 3: "we need 15 examples the AI generates
/// with the expectation of that rule … if both agree … it has the rule correct"). This is a spec
/// PARAMETER — the owner's exact count — NOT a tuned threshold: every per-rep decision is the behavioral
/// `run_plan` firing plus the comparative `corroborates` English gate; only this count is a literal, and
/// it is the owner's, cited as such. Changing the parameter does not touch the frozen comparator/judge.
pub const REQUIRED_REPS: usize = 15;

/// A rule the linter enforces: the firing check plus the English ADVICE it reports when it fires. The
/// `advice` is the "found violation in English" — linter-sourced, on a DIFFERENT derivation path from
/// the AI's [`LearnedRule::understanding`], which is what lets the two sides meet honestly in English.
#[derive(Debug, Clone)]
pub struct KnownRule {
    /// The check that fires this rule, run by the frozen [`run_plan`].
    pub plan: Plan,
    /// The English the linter reports for a finding of `plan` — the finding's advice prose.
    pub advice: String,
}

impl KnownRule {
    /// A known rule from its firing plan and the English advice it reports. Convenience constructor.
    pub fn new(plan: Plan, advice: impl Into<String>) -> Self {
        Self { plan, advice: advice.into() }
    }
}

/// The rule UNDER TEST: the AI's English `understanding` of what the rule means/forbids, a genuine
/// competing sibling `foil` the found advice is scored against (the comparator is comparative — it
/// judges "nearer the understanding than this foil", never against a constant, so a genuine foil is
/// required exactly as `lint_ism`/`lint_corroborate` require), and the `lang` its violations are written
/// in (passed to `run_plan` as the tree-sitter grammar id).
#[derive(Debug, Clone)]
pub struct LearnedRule {
    /// The AI's English account of what the rule means — the "learned rule" side of the equality.
    pub understanding: String,
    /// A genuine competing sibling meaning; the found advice must order strictly nearer the
    /// `understanding` than this foil does.
    pub foil: String,
    /// The language the self-generated violating code is written in (the `run_plan` grammar id).
    pub lang: String,
}

impl LearnedRule {
    /// A learned rule under test from its English understanding, a genuine sibling foil, and its
    /// language. Convenience constructor.
    pub fn new(understanding: impl Into<String>, foil: impl Into<String>, lang: impl Into<String>) -> Self {
        Self { understanding: understanding.into(), foil: foil.into(), lang: lang.into() }
    }
}

/// How one self-generated code sample stood to the learned rule — the outcome of a single rep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rep {
    /// The real linter FLAGGED the sample AND a fired rule's advice reconciled with the understanding
    /// in English (nearer than the foil). Both un-fakeable signals held — this counts toward graduation.
    Corroborates,
    /// The real linter FLAGGED the sample but the fired rule's advice does NOT reconcile with the
    /// understanding (a different rule than understood, or a faked understanding). The two Englishes
    /// contradict — a single one is fatal, exactly as one contradiction blocks in `lint_ism`. Carries
    /// the conflicting advice for the report.
    Mismatch(String),
    /// The self-generated code that SHOULD embody the violation was NOT flagged by any known rule — an
    /// expectation that did not hold, phased out and reported (never hidden).
    NotFlagged,
    /// Flagged, but the comparator has no shared English content to judge the advice against the
    /// understanding (an honest "cannot judge") — neither counts nor blocks.
    Undecidable,
}

/// The graduation verdict for a learned rule against its self-generated rep stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// At least [`REQUIRED_REPS`] distinct self-generated violations were flagged by the real linter and
    /// reconciled with the understanding in English, none contradicting — the rule graduates from a
    /// predicted understanding to a PROVEN one.
    Proven,
    /// The rule did not graduate; the [`Unproven`] reason says why.
    Unproven(Unproven),
}

/// Why a learned rule did NOT graduate — the honest reason, so the loop knows to reshape the faking
/// part (north-star step 4), generate more, or that it simply cannot judge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unproven {
    /// A flagged sample's found advice CONTRADICTED the understanding in English (a faked understanding
    /// or the wrong rule firing). Fatal and short-circuiting; the conflicting advice is carried.
    Contradicted {
        /// The fired rule's advice that did not reconcile with the understanding.
        advice: String,
    },
    /// No contradiction, but fewer than [`REQUIRED_REPS`] reps corroborated. Carries the phased-out
    /// `NotFlagged` count so the loop REPORTS the expectations that did not hold rather than hiding them.
    TooFewReps {
        /// Distinct self-generated violations that were flagged AND reconciled in English.
        corroborated: usize,
        /// The owner-specified count required to graduate ([`REQUIRED_REPS`]).
        required: usize,
        /// Self-generated samples that should have violated but were NOT flagged (phased-out expectations).
        not_flagged: usize,
    },
}

/// Classify ONE self-generated code sample against the learned rule, via the frozen linter and
/// comparator. The sample is FLAGGED iff some rule in `book` fires on it ([`run_plan`] non-empty); the
/// found advice is that fired rule's [`KnownRule::advice`]. A fired advice that reconciles with the
/// `understanding` over the `foil` ([`corroborates`] `Some(true)`) makes the rep [`Rep::Corroborates`];
/// a fired advice that fails to reconcile (`Some(false)`) makes it [`Rep::Mismatch`]; nothing fired is
/// [`Rep::NotFlagged`]; only undecidable comparisons give [`Rep::Undecidable`]. When several rules fire,
/// a single reconciling one wins (the violation WAS found and matched), else a mismatching one is
/// reported.
pub fn classify_sample(m: &MeaningNetwork, en: &English, rule: &LearnedRule, book: &[KnownRule], code: &str) -> Rep {
    let fired: Vec<&KnownRule> =
        book.iter().filter(|r| !run_plan(&r.plan, &rule.lang, code).is_empty()).collect();
    if fired.is_empty() {
        return Rep::NotFlagged;
    }
    let mut mismatch: Option<String> = None;
    let mut decidable = false;
    for r in fired {
        match corroborates(m, en, &rule.understanding, &r.advice, &rule.foil) {
            Some(true) => return Rep::Corroborates,
            Some(false) => {
                decidable = true;
                mismatch.get_or_insert_with(|| r.advice.clone());
            }
            None => {}
        }
    }
    match mismatch {
        Some(advice) if decidable => Rep::Mismatch(advice),
        _ => Rep::Undecidable,
    }
}

/// Fold a rep stream into a [`Verdict`] per the loop's law: any [`Rep::Mismatch`] ⇒
/// [`Unproven::Contradicted`] (short-circuits — a faked understanding or wrong rule is fatal); else
/// count [`Rep::Corroborates`]; ≥ [`REQUIRED_REPS`] ⇒ [`Verdict::Proven`], else
/// [`Unproven::TooFewReps`] carrying the phased-out [`Rep::NotFlagged`] count. [`Rep::Undecidable`] is
/// ignored. Separated from [`classify_sample`] so the counting law is testable on synthetic reps alone.
pub fn graduate<I: IntoIterator<Item = Rep>>(reps: I) -> Verdict {
    let mut corroborated = 0usize;
    let mut not_flagged = 0usize;
    for rep in reps {
        match rep {
            Rep::Mismatch(advice) => return Verdict::Unproven(Unproven::Contradicted { advice }),
            Rep::Corroborates => corroborated += 1,
            Rep::NotFlagged => not_flagged += 1,
            Rep::Undecidable => {}
        }
    }
    if corroborated >= REQUIRED_REPS {
        Verdict::Proven
    } else {
        Verdict::Unproven(Unproven::TooFewReps { corroborated, required: REQUIRED_REPS, not_flagged })
    }
}

/// Run the whole loop: classify every self-generated `samples` code against the learned rule, then
/// [`graduate`] the reps. The end-to-end entry — [`classify_sample`] per sample, folded by the counting
/// law. `samples` are the AI's self-generated violations (varied across reps); ~10–12 of genuinely
/// different code is the owner's spec, and the un-fakeable independence axis.
pub fn prove<S: AsRef<str>>(
    m: &MeaningNetwork,
    en: &English,
    rule: &LearnedRule,
    book: &[KnownRule],
    samples: &[S],
) -> Verdict {
    graduate(samples.iter().map(|s| classify_sample(m, en, rule, book, s.as_ref())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lint_char, lint_english};

    /// Both frozen brains, or `None` when an artifact is not on disk (the loop is defined only over the
    /// real bedrock, so tests observe against it or skip honestly — never fake a pass).
    fn brains() -> Option<(&'static lint_char::CharReader, &'static English)> {
        Some((lint_char::brain()?, lint_english::brain()?))
    }

    // ── The counting law, on synthetic reps (no linter, fully abstract) ──────────────────────────

    #[test]
    fn enough_corroborating_reps_graduate() {
        let reps: Vec<Rep> = std::iter::repeat_with(|| Rep::Corroborates).take(REQUIRED_REPS).collect();
        assert_eq!(graduate(reps), Verdict::Proven, "≥ REQUIRED_REPS corroborations must graduate");
    }

    #[test]
    fn one_short_does_not_graduate_and_reports_phased_out() {
        let mut reps: Vec<Rep> =
            std::iter::repeat_with(|| Rep::Corroborates).take(REQUIRED_REPS - 1).collect();
        reps.push(Rep::NotFlagged);
        reps.push(Rep::Undecidable);
        match graduate(reps) {
            Verdict::Unproven(Unproven::TooFewReps { corroborated, required, not_flagged }) => {
                assert_eq!(corroborated, REQUIRED_REPS - 1);
                assert_eq!(required, REQUIRED_REPS);
                assert_eq!(not_flagged, 1, "the phased-out NotFlagged expectation is reported, not hidden");
            }
            other => panic!("one short of the bar must not graduate, got {other:?}"),
        }
    }

    #[test]
    fn a_single_mismatch_is_fatal() {
        // A genuine English contradiction blocks even amid many corroborations — the anti-facade gate.
        let mut reps: Vec<Rep> = std::iter::repeat_with(|| Rep::Corroborates).take(20).collect();
        reps.insert(3, Rep::Mismatch("the found violation is about something else".into()));
        match graduate(reps) {
            Verdict::Unproven(Unproven::Contradicted { advice }) => {
                assert!(advice.contains("something else"), "the conflicting advice is carried: {advice}");
            }
            other => panic!("one mismatch must block graduation, got {other:?}"),
        }
    }

    // ── The full wiring, real linter + frozen comparator, abstract construct + abstract English ──
    //
    // The grammar id is used ONLY as a lexer for the abstract identifier tokens `alpha`/`beta`; no real
    // construct name or meaning appears. The English (understanding/advice/foil) uses the comparator's
    // proven abstract fixtures (a co-hyponym is-a: `mammal`/`animal`/`fish`), never a linted language's
    // meaning. The demo on a CONCRETE rule (`var`) is measured in `examples/selftest_probe.rs`, not here.

    /// The abstract token both known rules fire on — an opaque identifier, not a real construct.
    const CONSTRUCT: &str = "alpha";

    #[test]
    fn flagged_and_reconciled_sample_corroborates() {
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        let rule = LearnedRule::new("the cat is a mammal", "the cat is a fish", "javascript");
        // The genuine rule's advice is the true generalization; it reconciles over the sibling foil.
        let book = vec![KnownRule::new(Plan::UsesConstruct { construct: CONSTRUCT.into() }, "the cat is an animal")];
        assert_eq!(
            classify_sample(m, en, &rule, &book, "alpha;"),
            Rep::Corroborates,
            "a flagged sample whose found advice reconciles must corroborate"
        );
    }

    #[test]
    fn unflagged_sample_is_a_phased_out_expectation() {
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        let rule = LearnedRule::new("the cat is a mammal", "the cat is a fish", "javascript");
        let book = vec![KnownRule::new(Plan::UsesConstruct { construct: CONSTRUCT.into() }, "the cat is an animal")];
        // `beta` never appears — nothing fires, so the should-violate expectation did not hold.
        assert_eq!(classify_sample(m, en, &rule, &book, "beta;"), Rep::NotFlagged);
    }

    #[test]
    fn flagged_but_wrong_meaning_is_a_mismatch() {
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        let rule = LearnedRule::new("the cat is a mammal", "the cat is a fish", "javascript");
        // The fired rule's advice asserts the FOIL (fish), not the understanding — the wrong rule fired.
        let book = vec![KnownRule::new(Plan::UsesConstruct { construct: CONSTRUCT.into() }, "the cat is a fish")];
        match classify_sample(m, en, &rule, &book, "alpha;") {
            Rep::Mismatch(advice) => assert!(advice.contains("fish"), "the conflicting advice is reported"),
            other => panic!("a flagged sample whose advice contradicts must be a Mismatch, got {other:?}"),
        }
    }

    #[test]
    fn genuine_rule_graduates_faked_understanding_is_rejected_end_to_end() {
        // The whole loop on the SAME flagged samples: a genuine understanding graduates; a faked one
        // (whose English does not reconcile with the found advice) is rejected despite identical firing.
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        // The found advice restates the genuine is-a; the comparator's proven directed-reference
        // separation (a true restatement at distance 0.0 vs a same-topic falsehood at 1.26) is what
        // graduates the genuine understanding and rejects the faked one.
        let book = vec![KnownRule::new(Plan::UsesConstruct { construct: CONSTRUCT.into() }, "a dog is a canine animal")];
        let samples: Vec<String> = (0..REQUIRED_REPS + 2).map(|i| format!("alpha;\n// rep {i}")).collect();

        let genuine = LearnedRule::new("a dog is a canine", "a dog is a bird", "javascript");
        assert_eq!(prove(m, en, &genuine, &book, &samples), Verdict::Proven, "genuine understanding graduates");

        // A faked understanding: the found advice is NOT nearer the faked understanding (`a dog is a
        // bird`) than the genuine foil (`a dog is a canine`) — the English judge rejects the facade.
        let faked = LearnedRule::new("a dog is a bird", "a dog is a canine", "javascript");
        match prove(m, en, &faked, &book, &samples) {
            Verdict::Unproven(_) => {}
            Verdict::Proven => panic!("a faked understanding must NOT graduate — the anti-facade property"),
        }
    }
}
