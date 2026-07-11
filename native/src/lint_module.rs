//! `lint_module` — the construct-module TRAINING WORKFLOW, deriving every input of the frozen
//! self-generated test loop ([`crate::lint_selftest`]) from a language's OWN cached documentation.
//!
//! Contract: `LINTER.md` → "The construct-module training workflow". The self-generated test loop is
//! proven, but in its probes `understanding`/`advice`/`foil`/`samples`/`clean` are hand-written Rust
//! literals — that proves the MECHANISM, not the workflow. This module is the workflow: a PURE function
//! over a read [`Memory`] (plus the two frozen brains) that
//!   1. PROPOSES construct candidates from the docs' governing prose ([`Bridge::constructs_named`]),
//!   2. DERIVES the four loop inputs from data (each a different derivation path),
//!   3. HARVESTS varied violating/clean samples from the language's own crawled code corpus,
//!   4. GRADUATES through the frozen [`prove`], and
//!   5. EMITS each proven rule as a [`LearnedRule`] the existing [`crate::lint_match::RuleSet::build`]
//!      compiles into a live-firing detector.
//!
//! NOTHING here names a language or a construct: the constructs are DATA read from the prose, the
//! samples are DATA harvested from the crawl. The dictionary, comparator, engine, trace bridge, and
//! `lint_selftest` judging are all FROZEN — this module only reads, derives, and orchestrates.

use crate::lint_char::MeaningNetwork;
use crate::lint_english::English;
use crate::lint_read::Memory;
use crate::lint_selftest::{prove, KnownRule, LearnedRule as RuleUnderTest, Verdict, REQUIRED_REPS};
use crate::lint_trace::{run_plan, Bridge, Plan};
use crate::linter::LearnedRule;

/// A runaway safety valve on how many distinct code blocks the harvest scans per language — sized far
/// above any real docs site's example diversity while keeping the per-candidate `run_plan` sweep
/// bounded (training must stay seconds, not minutes). Not a working limit: every real JS crawl's
/// blocks fit well under it.
const MAX_HARVEST_BLOCKS: usize = 4000;

/// A construct-rule candidate proposed from the docs — the construct DATA-read from the prose, the
/// governing sentence it was named in (the `understanding` side), and the source url for citation.
#[derive(Clone, Debug)]
pub struct Candidate {
    /// The code construct whose USE the rule forbids (a backticked symbol or a syntax token) — DATA
    /// from the prose, never a coded token.
    pub construct: String,
    /// The governing sentence the construct was named in — verbatim doc prose, the `understanding`.
    pub understanding: String,
    /// The documentation url the governing sentence came from — the finding's citation.
    pub url: String,
}

/// The outcome of putting one candidate through the frozen loop — reported whether it graduated or
/// not, so the workflow measures honestly (LINTER.md: record what does NOT work as prominently as what
/// does). A `rule` is `Some` only for a `Verdict::Proven` candidate.
#[derive(Clone, Debug)]
pub struct Outcome {
    /// The candidate that was tested.
    pub candidate: Candidate,
    /// Distinct harvested doc blocks on which the construct's plan genuinely FIRED (the violating reps).
    pub violating: usize,
    /// Harvested doc blocks on which it did NOT fire (the clean near-misses / remedy forms).
    pub clean: usize,
    /// The frozen loop's verdict over the harvested reps.
    pub verdict: Verdict,
    /// The proven rule ready for the module (`Some` iff `verdict` is `Proven` and a clean near-miss
    /// existed to contrast against), paired with its source url.
    pub rule: Option<(LearnedRule, String)>,
}

/// Whether `sentence` MENTIONS `construct` as a code symbol — the backticked form `` `C` `` or `C`
/// as a punctuation/whitespace-delimited token (so `var` does not match inside `variable`). The
/// derivation of `advice` (a SECOND distinct doc sentence about the construct) stands on this.
fn mentions(sentence: &str, construct: &str) -> bool {
    let lower = sentence.to_lowercase();
    let c = construct.to_lowercase();
    if lower.contains(&format!("`{c}`")) {
        return true;
    }
    // Token-delimited match: the construct bounded by non-alphanumeric (or string ends). This is the
    // same "a construct is one code lexeme" reading `extract_construct` uses, applied to prose.
    let bytes = lower.as_bytes();
    let mut from = 0;
    while let Some(rel) = lower[from..].find(&c) {
        let start = from + rel;
        let end = start + c.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_ok = end >= bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

/// The SECOND, distinct doc sentence that mentions `construct` — the `advice` side, a different
/// derivation path from the `understanding` (the governing sentence). Scans every binding's prose for
/// a sentence that mentions the construct, differs from the `understanding`, and CARRIES THE SAME
/// POLARITY ([`lint_corroborate::is_negated`]). The polarity match is load-bearing and MEASURED: the
/// frozen comparator judges polarity FIRST, so pairing a negative governing prohibition ("Never use
/// `var`") with a positive rationale ("Using `var` leaks scope") reads as a CONTRADICTION and blocks
/// graduation. Two same-polarity doc statements about the same construct reconcile on their shared
/// content; a mis-attributed construct (a sibling's prose) does not. `None` when the docs state the
/// construct only once at this polarity: without a second independent same-polarity statement the rule
/// cannot form an un-fakeable English pair, so it must not graduate (a self-comparison is forbidden).
fn derive_advice(memory: &Memory, en: &English, construct: &str, understanding: &str) -> Option<String> {
    let want_negated = crate::lint_corroborate::is_negated(en, understanding);
    for b in &memory.bindings {
        for s in crate::lint_read::sentences(&b.prose) {
            if s != understanding
                && mentions(s, construct)
                && crate::lint_corroborate::is_negated(en, s) == want_negated
            {
                return Some(s.to_string());
            }
        }
    }
    None
}

/// Collect the language's OWN code corpus from the read memory — every distinct code block the docs
/// served (bindings' bound code + the reference corpus). This is the covenant-clean source of sample
/// variation: real code from the language's documentation, harvested with the construct present or
/// absent. Deduped and bounded by [`MAX_HARVEST_BLOCKS`].
fn harvest_corpus(memory: &Memory) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let push = |code: &str, out: &mut Vec<String>, seen: &mut std::collections::HashSet<u64>| {
        let trimmed = code.trim();
        if trimmed.is_empty() || out.len() >= MAX_HARVEST_BLOCKS {
            return;
        }
        if seen.insert(crate::lint_ai::token_seed(trimmed)) {
            out.push(trimmed.to_string());
        }
    };
    for b in &memory.bindings {
        push(&b.code, &mut out, &mut seen);
    }
    for r in &memory.reference {
        push(r, &mut out, &mut seen);
    }
    out
}

/// A stable, inspectable rule id from a construct name — `uses-<construct>` with non-alphanumerics
/// folded to `-`, so `document.write` → `uses-document-write` and `==` → `uses--` stays distinct per
/// construct. The construct is DATA, so the id is too.
fn rule_id(construct: &str) -> String {
    let slug: String = construct
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    format!("uses-{slug}")
}

/// PROPOSE a construct-rule candidate for every construct a doc sentence PROHIBITS — the construct
/// [`Bridge::constructs_named`] reads from a sentence that [`English::sentence_states_prohibition`]
/// classifies as a prohibition (the same meaning gate `rules_from_memory` uses). The prohibition gate
/// is the DISCRIMINATOR, and it is load-bearing and MEASURED: WITHOUT it the self-generated loop
/// graduates pure syntax — `}`, `const`, `if`, `for`, `this` each fire on ≥10 harvested blocks, so the
/// behavioral axis alone cannot tell a banned construct from ordinary syntax (840 candidates, 61 junk
/// "proven"). The gate is documented LOW-RECALL (it misses a prohibition phrased without a lead
/// negator), which is why some real classics may not be proposed — a reported gap, never junk. Deduped
/// by construct, keeping the FIRST prohibition sentence and its page url.
fn propose(memory: &Memory, bridge: &Bridge, en: &English) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    for b in &memory.bindings {
        for sentence in crate::lint_read::sentences(&b.prose) {
            if !en.sentence_states_prohibition(sentence) {
                continue;
            }
            for (construct, understanding) in bridge.constructs_named(sentence) {
                if out.iter().any(|c| c.construct == construct) {
                    continue;
                }
                out.push(Candidate { construct, understanding, url: b.url.clone() });
            }
        }
    }
    out
}

/// GRADUATE construct rules for `lang` from a read [`Memory`] — the whole workflow, pure over the
/// memory and the two frozen brains. Returns an [`Outcome`] per proposed candidate (graduated or not)
/// so the caller can measure honestly. Only `Outcome::rule.is_some()` candidates are module-ready.
///
/// The frozen loop's independence axis is DISTINCT harvested violating blocks; its English gate is the
/// two-doc-sentence reconciliation over a sibling foil. A candidate graduates iff ≥ [`REQUIRED_REPS`]
/// distinct real blocks fire AND the two doc sentences reconcile AND none contradicts (LINTER.md).
pub fn graduate(lang: &str, memory: &Memory, m: &MeaningNetwork, en: &English) -> Vec<Outcome> {
    let bridge = Bridge::new(m, en);
    let candidates = propose(memory, &bridge, en);
    let corpus = harvest_corpus(memory);

    // The linter's BOOK of known rules: every candidate's firing plan + its derived advice, so firing
    // is realistic (any candidate may fire on any block). A candidate with no distinct second doc
    // sentence has no un-fakeable advice and is dropped from the book (and cannot graduate).
    let advices: Vec<Option<String>> = candidates
        .iter()
        .map(|c| derive_advice(memory, en, &c.construct, &c.understanding))
        .collect();
    let book: Vec<KnownRule> = candidates
        .iter()
        .zip(&advices)
        .filter_map(|(c, adv)| {
            adv.as_ref().map(|a| {
                KnownRule::new(Plan::UsesConstruct { construct: c.construct.clone() }, a.clone())
            })
        })
        .collect();

    let mut outcomes = Vec::new();
    for (i, cand) in candidates.iter().enumerate() {
        // HARVEST: partition the corpus into violating (the plan fires) and clean (it does not).
        let plan = Plan::UsesConstruct { construct: cand.construct.clone() };
        let mut violating: Vec<&str> = Vec::new();
        let mut clean: Vec<&str> = Vec::new();
        for block in &corpus {
            if run_plan(&plan, lang, block).is_empty() {
                clean.push(block);
            } else {
                violating.push(block);
            }
        }
        // DERIVE the foil: a SIBLING candidate's understanding (a genuine competing meaning). No
        // sibling ⇒ no genuine foil ⇒ the comparator cannot judge, so nothing graduates.
        let foil = candidates
            .iter()
            .enumerate()
            .find(|(j, other)| *j != i && advices[*j].is_some() && other.construct != cand.construct)
            .map(|(_, other)| other.understanding.clone());
        let advice = advices[i].clone();

        let verdict = match (&advice, &foil) {
            (Some(_advice), Some(foil)) => {
                let rule = RuleUnderTest::new(cand.understanding.clone(), foil.clone(), lang.to_string());
                prove(m, en, &rule, &book, &violating)
            }
            // Missing an un-fakeable advice or a genuine foil is an honest "cannot judge" — reported
            // as too-few-reps with the real firing count so the gap is visible, never a false pass.
            _ => Verdict::Unproven(crate::lint_selftest::Unproven::TooFewReps {
                corroborated: 0,
                required: REQUIRED_REPS,
                not_flagged: 0,
            }),
        };

        // EMIT: a proven candidate with a clean near-miss to contrast against becomes a module rule
        // in the shape `RuleSet::build` compiles into a firing detector (bad ∧ ¬good).
        let rule = if matches!(verdict, Verdict::Proven) {
            let bad = violating.iter().min_by_key(|b| b.len()).map(|b| b.to_string());
            let good = clean.iter().min_by_key(|b| b.len()).map(|b| b.to_string());
            match (bad, good) {
                (Some(bad), good) => Some((
                    LearnedRule {
                        language: lang.to_string(),
                        id: rule_id(&cand.construct),
                        severity: "medium".to_string(),
                        description: cand.understanding.clone(),
                        bad,
                        good: good.unwrap_or_default(),
                    },
                    cand.url.clone(),
                )),
                _ => None,
            }
        } else {
            None
        };

        outcomes.push(Outcome {
            candidate: cand.clone(),
            violating: violating.len(),
            clean: clean.len(),
            verdict,
            rule,
        });
    }
    outcomes
}

/// The LIVE entry the module build calls (the covenant-clean successor to
/// [`crate::lint_docs::rules_from_memory`] for MODULES): graduate `lang`'s construct rules from the
/// read `memory` through the frozen loop, returning `(rule, source url)` for every PROVEN rule only.
/// Loads the two frozen brains; empty when either is unavailable (the loop is defined only over the
/// real bedrock — never fake a rule). Never trains, never touches the network.
pub fn graduated_rules(lang: &str, memory: &Memory) -> Vec<(LearnedRule, String)> {
    let (Some(br), Some(en)) = (crate::lint_char::brain(), crate::lint_english::brain()) else {
        return Vec::new();
    };
    graduate(lang, memory, br.meanings(), en)
        .into_iter()
        .filter_map(|o| o.rule)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint_read::Binding;

    /// Both frozen brains, or `None` when an artifact is not on disk (the workflow is defined only over
    /// the real bedrock, so tests observe against it or skip honestly — never fake a pass).
    fn brains() -> Option<(&'static crate::lint_char::CharReader, &'static English)> {
        Some((crate::lint_char::brain()?, crate::lint_english::brain()?))
    }

    fn binding(url: &str, slug: &str, prose: &str, code: &str) -> Binding {
        Binding {
            url: url.to_string(),
            slug: slug.to_string(),
            prose: prose.to_string(),
            code: code.to_string(),
            bind: crate::lint_ai::Hv::zero(),
        }
    }

    #[test]
    fn mentions_is_token_delimited_not_substring() {
        assert!(mentions("Never use the `var` keyword.", "var"));
        assert!(mentions("The var statement is old.", "var"));
        assert!(!mentions("A variable holds a value.", "var"), "must not match inside 'variable'");
        assert!(mentions("Use === instead of ==.", "=="));
    }

    #[test]
    fn rule_id_is_a_stable_construct_slug() {
        assert_eq!(rule_id("var"), "uses-var");
        assert_eq!(rule_id("document.write"), "uses-document-write");
    }

    #[test]
    fn harvest_dedups_and_gathers_bindings_and_reference() {
        let mut memory = Memory::default();
        memory.bindings.push(binding("u", "s", "p", "var x = 1;"));
        memory.bindings.push(binding("u", "s", "p", "var x = 1;")); // dup
        memory.reference.push("let y = 2;".to_string());
        let corpus = harvest_corpus(&memory);
        assert_eq!(corpus.len(), 2, "the duplicate block collapses; the reference block joins");
    }

    /// The workflow graduates a construct from HARVESTED real blocks and DERIVED English — no
    /// hand-written sample, understanding, advice, or foil. A synthetic-but-honest memory stands in for
    /// a crawl: two constructs (`var`, `eval`) each with a governing sentence, a distinct rationale
    /// sentence (the advice path), and ≥ REQUIRED_REPS harvested violating blocks. Skips without the
    /// frozen brains (never fakes a pass). The REAL MDN/ESLint measurement is
    /// `examples/js_module_train.rs`.
    #[test]
    fn graduates_a_construct_from_harvested_blocks_and_derived_english() {
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        let mut memory = Memory::default();
        // Governing prose (understanding) + a distinct rationale sentence (advice), for two siblings.
        memory.bindings.push(binding(
            "https://docs/var",
            "no-var",
            "Never use the `var` keyword to declare a variable. Never declare a variable with `var`, whose scope leaks out of its block.",
            "var x = 1;",
        ));
        memory.bindings.push(binding(
            "https://docs/eval",
            "no-eval",
            "Never use the `eval` function to execute a string of code. Never call `eval` on an arbitrary string of code as a security risk.",
            "eval(userInput);",
        ));
        // Harvest corpus: ≥ REQUIRED_REPS distinct real-shaped violating blocks using `var`, plus
        // clean near-misses (the remedy form, `var` absent). Distinct code = the independence axis.
        for i in 0..REQUIRED_REPS + 2 {
            memory.reference.push(format!("var v{i} = {i};"));
        }
        memory.reference.push("let a = 1;".to_string());
        memory.reference.push("const b = 2;".to_string());

        let outcomes = graduate("javascript", &memory, m, en);
        let var = outcomes
            .iter()
            .find(|o| o.candidate.construct == "var")
            .expect("var proposed from its governing prose");
        eprintln!(
            "var: violating={} clean={} verdict={:?}",
            var.violating, var.clean, var.verdict
        );
        assert!(var.violating >= REQUIRED_REPS, "≥ REQUIRED_REPS harvested violations: {}", var.violating);
        assert_eq!(var.verdict, Verdict::Proven, "genuinely-understood var graduates from harvest");
        let (rule, url) = var.rule.as_ref().expect("a proven rule is emitted");
        assert_eq!(rule.id, "uses-var");
        assert!(rule.bad.contains("var"), "bad is a harvested violating block: {}", rule.bad);
        assert!(!rule.bad.contains("var") || !rule.good.contains("var"), "good is a clean near-miss");
        assert_eq!(url, "https://docs/var", "the rule cites its source page");
    }
}
