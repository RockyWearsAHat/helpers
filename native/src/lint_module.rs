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

/// How many violating blocks the frozen [`prove`] loop is run over per candidate — a bound on the
/// book-sweep cost, generous above [`REQUIRED_REPS`] so undecidable/mismatch reps still leave a graduating
/// margin. The Outcome reports the TRUE violating count; only the proof sample is capped.
const PROVE_SAMPLE_CAP: usize = 30;

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
/// derivation path from the `understanding` (the governing sentence). Scans the POOLED clean governing
/// sentences (read STRUCTURALLY by [`crate::lint_lang_layer`], no longer the garbled binding prose) for
/// a sentence that mentions the construct, differs from the `understanding`, and CARRIES THE SAME
/// POLARITY ([`lint_corroborate::is_negated`]). The polarity match is load-bearing and MEASURED: the
/// frozen comparator judges polarity FIRST, so pairing a negative governing prohibition ("Never use
/// `var`") with a positive rationale ("Using `var` leaks scope") reads as a CONTRADICTION and blocks
/// graduation. Two same-polarity doc statements about the same construct reconcile on their shared
/// content; a mis-attributed construct (a sibling's prose) does not. `None` when the docs state the
/// construct only once at this polarity: without a second independent same-polarity statement the rule
/// cannot form an un-fakeable English pair, so it must not graduate (a self-comparison is forbidden).
fn derive_advice(pool: &[PooledSentence], en: &English, construct: &str, understanding: &str, url: &str) -> Option<String> {
    let want_negated = crate::lint_corroborate::is_negated(en, understanding);
    let ok = |ps: &&PooledSentence| {
        ps.sentence != understanding
            && mentions(&ps.sentence, construct)
            && crate::lint_corroborate::is_negated(en, &ps.sentence) == want_negated
    };
    // Prefer a second statement from the construct's OWN page (two doc sentences about the same construct
    // reconcile); fall back to any page only if its own page states the construct just once.
    pool.iter()
        .filter(|ps| ps.url == url)
        .find(ok)
        .or_else(|| pool.iter().find(ok))
        .map(|ps| ps.sentence.clone())
}

/// Whether `code` contains `construct` as a code token — the sound harvest pre-filter. A SYMBOL
/// construct (`==`) must be a whole whitespace/punctuation-delimited token (so it is not found inside
/// `===`); an alphanumeric construct (`var`) is matched on a non-alphanumeric, non-dot boundary (so it
/// is not found inside `variable` and a dotted member is matched whole). Presence of the text is
/// NECESSARY for `scan_construct` to fire, so a block failing this is provably clean without a parse.
fn construct_in_text(code: &str, construct: &str) -> bool {
    if construct.is_empty() {
        return false;
    }
    let is_symbol = construct.chars().all(|c| !c.is_ascii_alphanumeric() && c != '.');
    if is_symbol {
        return code
            .split_whitespace()
            .any(|t| t.trim_matches(|c: char| c == '(' || c == ')' || c == ';' || c == '{' || c == '}' || c == ',') == construct);
    }
    let bytes = code.as_bytes();
    let mut from = 0;
    while let Some(rel) = code[from..].find(construct) {
        let start = from + rel;
        let end = start + construct.len();
        let boundary = |b: u8| !(b.is_ascii_alphanumeric() || b == b'.');
        let before_ok = start == 0 || boundary(bytes[start - 1]);
        let after_ok = end >= bytes.len() || boundary(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
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

/// The PROPOSED construct candidates for `lang` from its raw doc `pages`, WITHOUT the expensive prove —
/// a fast diagnostic of how many constructs the structural reading discovers and what they are. Each
/// carries its construct, the governing understanding sentence, and the source url. Pure over the pages
/// and the two frozen brains.
pub fn proposed(lang: &str, pages: &[(String, String)], memory: &Memory, m: &MeaningNetwork, en: &English) -> Vec<Candidate> {
    let bridge = Bridge::new(m, en);
    let partition = lang_pages(pages, memory);
    propose(lang, &partition, &bridge, en).0
}

/// PARTITION raw doc pages to the ones this language's docs were read from — the crawl's OWN per-page
/// language attribution, reused: a page attributes to `lang` iff the language's read [`Memory`] holds a
/// binding from that page (`memory.bindings[].url`). This is the "never conflate languages" law
/// (LINTER.md) enforced at PROPOSE: a CSS deprecation page and a JS rule page live in the same MDN crawl,
/// but each language proposes ONLY from its own attributed pages, so a CSS construct never enters the JS
/// module. When the memory carries no bindings (nothing to attribute against), every page is kept — the
/// caller has already scoped the pages to one language.
fn lang_pages<'a>(pages: &'a [(String, String)], memory: &Memory) -> Vec<&'a (String, String)> {
    let urls: std::collections::HashSet<&str> = memory.bindings.iter().map(|b| b.url.as_str()).collect();
    if urls.is_empty() {
        return pages.iter().collect();
    }
    pages.iter().filter(|(u, _)| urls.contains(u.as_str())).collect()
}

/// One clean governing sentence in the pooled reading, tagged with whether it came from a PROHIBITED
/// page (so `understanding` selection can prefer a real prohibition statement) and its source url.
struct PooledSentence {
    sentence: String,
    prohibited: bool,
    /// The page the sentence came from — so `advice` can prefer a SECOND sentence from the construct's
    /// OWN page (two statements about the same construct on its own page reconcile; a cross-page sentence
    /// that merely mentions the name is unrelated chrome that contradicts).
    url: String,
    /// Precomputed negation polarity ([`lint_corroborate::is_negated`]) — computed ONCE per sentence,
    /// not per candidate, so `understanding` selection over the pool stays fast.
    negated: bool,
}

/// PROPOSE a construct-rule candidate for every construct a language-doc page structurally PROHIBITS,
/// read by [`crate::lint_lang_layer`] (a rule page bans the construct it documents; a deprecated
/// reference page bans its subject) — NOT the low-recall [`English::sentence_states_prohibition`] prose
/// gate, MEASURED to fire on none of the clean real prohibition sentences ("This rule disallows `with`
/// statements", "discouraging the use of `var`"). The structural page ROLE is the discriminator, so the
/// behavioral loop is never asked to tell a banned construct from ordinary syntax (the measured `}`/
/// `const`/`if` junk class): only constructs a prohibition PAGE names are proposed. Returns the deduped
/// candidates plus the POOLED clean governing sentences (the `advice` search space). The `understanding`
/// is a real doc sentence mentioning the construct — preferring one from a prohibited page and in
/// negative polarity; a construct no clean sentence mentions cannot form an un-fakeable English pair and
/// is dropped (never a synthesized sentence).
fn propose(lang: &str, pages: &[&(String, String)], bridge: &Bridge, en: &English) -> (Vec<Candidate>, Vec<PooledSentence>) {
    let docpages: Vec<crate::lint_lang_layer::DocPage> =
        pages.iter().map(|(url, body)| crate::lint_lang_layer::read_doc_page(url, body, en, bridge)).collect();
    let mut pooled: Vec<PooledSentence> = Vec::new();
    for p in &docpages {
        for s in &p.governing {
            let negated = crate::lint_corroborate::is_negated(en, s);
            pooled.push(PooledSentence { sentence: s.clone(), prohibited: p.prohibited, url: p.url.clone(), negated });
        }
    }

    let mut out: Vec<Candidate> = Vec::new();
    for p in &docpages {
        for construct in &p.constructs {
            if out.iter().any(|c| &c.construct == construct) {
                continue;
            }
            // VERIFY against the page's OWN bad/good examples with the frozen firing (the north-star's
            // propose-verify-learn over the docs' own examples). A candidate is a genuine prohibition iff
            // it fires on STRICTLY MORE incorrect than correct blocks — comment/string-safe (`run_plan`
            // skips lexical text, so the `/*eslint no-var*/` config comment never counts) and it excludes
            // the remedy (`const`/`let`/`===`, which fire on 0 incorrect) and a construct a good example
            // legitimately uses (`declare var` in a `.d.ts`, which fires on BOTH). Pages with no examples
            // (a deprecated reference page) pass — the deprecation notecard is their prohibition proof.
            if !confirmed_by_examples(lang, construct, &p.incorrect, &p.correct) {
                continue;
            }
            // The `understanding` is the best real doc sentence MENTIONING the construct: prefer one from
            // the construct's OWN page (page-of-origin), then a prohibited-page statement, then negative
            // polarity, then a longer (more informative) one. Its citation url is the proposing page.
            let best = pooled
                .iter()
                .filter(|ps| mentions(&ps.sentence, construct))
                .max_by_key(|ps| {
                    (
                        u32::from(ps.url == p.url),
                        u32::from(ps.prohibited),
                        u32::from(ps.negated),
                        ps.sentence.len(),
                    )
                });
            if let Some(best) = best {
                out.push(Candidate {
                    construct: construct.clone(),
                    understanding: best.sentence.clone(),
                    url: p.url.clone(),
                });
            }
        }
    }
    (out, pooled)
}

/// Whether the page's OWN examples confirm `construct` as a genuine prohibition of its BARE use: it
/// fires (frozen `run_plan`) on some INCORRECT example AND does NOT fire on the page's PRIMARY (first)
/// CORRECT example — the docs' main remedy demonstration, which omits the banned construct. Later correct
/// blocks are OPTION-specific exceptions (eqeqeq's "smart" `x == null`, no-var's ambient `declare var`, an
/// `allowIndirect` `window.eval`) that legitimately use the construct, so the first correct block — not a
/// count over all of them — is the honest "the remedy drops it" test. A page with no incorrect examples
/// (a deprecated reference page) passes: its deprecation notecard is the prohibition proof. Firing is
/// AST-grained, so a construct named only in the `/*eslint no-var*/` config comment never counts.
fn confirmed_by_examples(lang: &str, construct: &str, incorrect: &[String], correct: &[String]) -> bool {
    if incorrect.is_empty() {
        return true;
    }
    let plan = Plan::UsesConstruct { construct: construct.to_string() };
    let fires = |b: &str| !run_plan(&plan, lang, b).is_empty();
    let fires_incorrect = incorrect.iter().any(|b| fires(b));
    let fires_primary_correct = correct.first().is_some_and(|g| fires(g));
    fires_incorrect && !fires_primary_correct
}

/// GRADUATE construct rules for `lang` — the whole workflow, pure over the language's raw doc `pages`
/// (the PROPOSE source, read structurally by [`crate::lint_lang_layer`]), the read [`Memory`] (the
/// harvest corpus), and the two frozen brains. Returns an [`Outcome`] per proposed candidate (graduated
/// or not) so the caller can measure honestly. Only `Outcome::rule.is_some()` candidates are
/// module-ready.
///
/// The frozen loop's independence axis is DISTINCT harvested violating blocks; its English gate is the
/// two-doc-sentence reconciliation over a sibling foil. A candidate graduates iff ≥ [`REQUIRED_REPS`]
/// distinct real blocks fire AND the two doc sentences reconcile AND none contradicts (LINTER.md).
pub fn graduate(
    lang: &str,
    pages: &[(String, String)],
    memory: &Memory,
    m: &MeaningNetwork,
    en: &English,
) -> Vec<Outcome> {
    let bridge = Bridge::new(m, en);
    let partition = lang_pages(pages, memory);
    let (candidates, pool) = propose(lang, &partition, &bridge, en);
    let corpus = harvest_corpus(memory);

    // The linter's BOOK of known rules: every candidate's firing plan + its derived advice, so firing
    // is realistic (any candidate may fire on any block). A candidate with no distinct second doc
    // sentence has no un-fakeable advice and is dropped from the book (and cannot graduate).
    let advices: Vec<Option<String>> = candidates
        .iter()
        .map(|c| derive_advice(&pool, en, &c.construct, &c.understanding, &c.url))
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
        // HARVEST: partition the corpus into violating (the plan fires) and clean (it does not). A
        // block whose TEXT does not contain the construct cannot possibly fire `uses_construct(C)`
        // (`scan_construct` matches an AST node whose text equals `C`), so it is clean WITHOUT a parse —
        // a sound pre-filter (no false negative) that turns the harvest from O(candidates × corpus
        // parses) into a parse only where the construct's text appears, keeping training in seconds.
        let plan = Plan::UsesConstruct { construct: cand.construct.clone() };
        let mut violating: Vec<&str> = Vec::new();
        let mut clean: Vec<&str> = Vec::new();
        for block in &corpus {
            let fires = construct_in_text(block, &cand.construct) && !run_plan(&plan, lang, block).is_empty();
            if fires {
                violating.push(block);
            } else {
                clean.push(block);
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
                // Prove on a bounded SAMPLE of the violating blocks: graduation needs only ≥ REQUIRED_REPS
                // corroborations, so a cap keeps the book-sweep (O(samples × book) run_plans per
                // candidate) in seconds without changing the verdict — the true violating count stays in
                // the Outcome. A margin above the floor absorbs undecidable/mismatch reps.
                let sample: Vec<&str> = violating.iter().take(PROVE_SAMPLE_CAP).copied().collect();
                prove(m, en, &rule, &book, &sample)
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
    let data_root = crate::tools::lint::data_root_pub();
    let (pages, _fp) = crate::lint_docs::raw_pages(&data_root, lang);
    if pages.is_empty() {
        return Vec::new();
    }
    graduate(lang, &pages, memory, br.meanings(), en)
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

    /// A synthetic-but-honest rule PAGE stands in for a crawl page: the construct is PROPOSED
    /// structurally (a `/rules/` page bans the construct its own incorrect example uses and its correct
    /// example drops), the `understanding`/`advice` are DERIVED from the page's clean governing prose,
    /// and the violating/clean samples are HARVESTED from the memory corpus — no hand-written candidate,
    /// sample, understanding, advice, or foil. Two sibling rule pages supply the genuine foil. Skips
    /// without the frozen brains (never fakes a pass). The REAL three-language measurement is
    /// `examples/web_module_train.rs`.
    #[test]
    fn graduates_a_construct_from_structural_pages_and_harvested_blocks() {
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        // Two sibling rule pages, each proposing one construct from its OWN bad/good examples.
        let var_page = r#"<html><body><h1>no-var</h1>
            <p>This rule is aimed at discouraging the use of <code>var</code> and encouraging the use of <code>const</code> or <code>let</code> instead.</p>
            <p>The <code>var</code> keyword declares a variable whose scope leaks out of its enclosing block.</p>
            <p>Examples of <strong>incorrect</strong> code for this rule:</p>
            <div class="incorrect"><pre><code>var x = 1;</code></pre></div>
            <p>Examples of <strong>correct</strong> code for this rule:</p>
            <div class="correct"><pre><code>let x = 1;</code></pre></div></body></html>"#;
        let eval_page = r#"<html><body><h1>no-eval</h1>
            <p>This rule is aimed at disallowing the use of the <code>eval()</code> function.</p>
            <p>The <code>eval()</code> function executes a string of code as a security risk.</p>
            <p>Examples of <strong>incorrect</strong> code for this rule:</p>
            <div class="incorrect"><pre><code>eval("x");</code></pre></div>
            <p>Examples of <strong>correct</strong> code for this rule:</p>
            <div class="correct"><pre><code>JSON.parse("{}");</code></pre></div></body></html>"#;
        let pages = vec![
            ("https://docs/latest/rules/no-var".to_string(), var_page.to_string()),
            ("https://docs/latest/rules/no-eval".to_string(), eval_page.to_string()),
        ];
        // Harvest corpus: ≥ REQUIRED_REPS distinct real-shaped violating blocks using `var`, plus clean
        // near-misses (the remedy form, `var` absent). Distinct code = the independence axis.
        let mut memory = Memory::default();
        for i in 0..REQUIRED_REPS + 2 {
            memory.reference.push(format!("var v{i} = {i};"));
        }
        memory.reference.push("let a = 1;".to_string());
        memory.reference.push("const b = 2;".to_string());

        let outcomes = graduate("javascript", &pages, &memory, m, en);
        let var = outcomes
            .iter()
            .find(|o| o.candidate.construct == "var")
            .expect("var proposed from the /rules/ page's own bad/good examples");
        eprintln!("var: violating={} clean={} verdict={:?}", var.violating, var.clean, var.verdict);
        // The remedy `const`/`let` must NOT be proposed (used in the correct example).
        assert!(!outcomes.iter().any(|o| o.candidate.construct == "const" || o.candidate.construct == "let"),
            "the remedy const/let is excluded by the page's own good example");
        assert!(var.violating >= REQUIRED_REPS, "≥ REQUIRED_REPS harvested violations: {}", var.violating);
        assert_eq!(var.verdict, Verdict::Proven, "genuinely-understood var graduates from harvest");
        let (rule, url) = var.rule.as_ref().expect("a proven rule is emitted");
        assert_eq!(rule.id, "uses-var");
        assert!(rule.bad.contains("var"), "bad is a harvested violating block: {}", rule.bad);
        assert!(!rule.bad.contains("var") || !rule.good.contains("var"), "good is a clean near-miss");
        assert_eq!(url, "https://docs/latest/rules/no-var", "the rule cites its source page");
    }
}
