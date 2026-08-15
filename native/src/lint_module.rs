//! `lint_module` — the construct-module TRAINING WORKFLOW, deriving every input of the frozen
//! self-generated test loop ([`crate::lint_selftest`]) from a language's OWN cached documentation.
//!
//! Contract: `native/history.dx` → "The construct-module training workflow". The self-generated test loop is
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
use crate::lint_selftest::{graduate as fold_reps, LearnedRule as RuleUnderTest, Rep, Verdict, REQUIRED_REPS};
use crate::lint_trace::{run_plan, Bridge, Plan};
use crate::linter::LearnedRule;

/// A runaway safety valve on how many distinct code blocks the harvest scans per language — sized far
/// above any real docs site's example diversity while keeping the per-candidate `run_plan` sweep
/// bounded (training must stay seconds, not minutes). Not a working limit: every real JS crawl's
/// blocks fit well under it.
const MAX_HARVEST_BLOCKS: usize = 4000;

/// How many violating blocks the frozen [`prove`] loop is run over per candidate — a bound on the
/// book-sweep cost. Graduation needs only ≥ [`REQUIRED_REPS`] corroborations, and [`prove`] does not
/// short-circuit (it folds EVERY sample, since a late `Mismatch` is fatal), so the per-rep English
/// reconciliation (~0.1–0.26 s each, MEASURED) is the training's dominant cost. Capping at the rep floor
/// plus a small margin (which absorbs undecidable/near-miss reps) roughly HALVES the prove sweep versus a
/// cap of 30 without changing any measured verdict — the four wanted JS classics still graduate. The
/// Outcome reports the TRUE violating count; only the proof sample is capped.
const PROVE_SAMPLE_CAP: usize = REQUIRED_REPS + 4;

/// How many clean corpus blocks self-generation ([`generate_violations`]) may splice into — a bound that
/// keeps the top-up's `run_plan` sweep in the seconds budget while being ample to reach the rep floor from
/// real contexts. Only reached for a SCARCE construct (a common one is covered by harvest and never
/// generates); generation stops the instant it has [`REQUIRED_REPS`] + margin distinct firing samples.
const GENERATE_CONTEXT_CAP: usize = 400;

/// Distinct self-generated violating samples to aim for — the owner's rep floor plus a small margin so
/// undecidable/near-miss reps still leave a graduating count.
const GENERATE_TARGET: usize = REQUIRED_REPS + 4;

/// How many CLEAN near-miss blocks the blind-agreement loop admits as expect-no-flag reps (owner point
/// 3: clean samples count toward agreement). Bounded like the flag sample so the blind sweep stays in
/// the seconds budget; ample to add a squeeze from the clean side alongside the flag reps.
const CLEAN_SAMPLE_CAP: usize = REQUIRED_REPS;

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
    /// The page's own "incorrect code" example blocks that FIRE this construct — real, language-shaped
    /// violating snippets used as SEEDS for self-generation ([`generate_violations`]) when the harvest
    /// corpus is too scarce to reach the rep floor. Empty for a deprecated reference page (no examples);
    /// then carriers are synthesized by splicing the construct into the corpus.
    pub seeds: Vec<String>,
    /// Whether the origin page's deprecation is PAGE-SCOPE TRUTH (its own banner run) and the page
    /// writes the subject as `&lt;name&gt;` — the two element-hood facts the element-shaped graded
    /// tier needs when a proposed candidate FAILS graduation (the `center` blind spot: proposed
    /// candidates are excluded from the read surface, so a failed one could never grade).
    pub page_scope: bool,
    /// See [`Candidate::page_scope`] — the page's own `&lt;name&gt;` element typography for this subject.
    pub element_typography: bool,
    /// Whether the origin page STRUCTURALLY ATTESTS this construct as deprecated (a reference page with a
    /// deprecation notecard — [`crate::lint_lang_layer::DocPage::attested_deprecated`]). Such a candidate
    /// may graduate via the NOTECARD PATH when the English self-test cannot apply (degenerate identical
    /// deprecation prose across a site's pages), because the notecard is a STATED structural fact.
    pub attested_deprecated: bool,
    /// Whether the page's LEAD SUMMARY structurally STATES this construct as its subject ([`stated_by_lead`])
    /// — the covenant-clean `??`-vs-`==` discriminator. Computed at PROPOSE but enforced at EMISSION (the
    /// candidate stays in the pool so the frozen self-test's foil/advice — and thus every OTHER candidate's
    /// verdict — is identical to not gating; only the EMITTED rule set drops an un-stated subject like `??`).
    /// `true` for a deprecated reference page (no paired examples — the notecard is its proof), so CSS/HTML
    /// are unaffected.
    pub stated: bool,
}

/// The outcome of putting one candidate through the frozen loop — reported whether it graduated or
/// not, so the workflow measures honestly (native/architecture.dx: record what does NOT work as prominently as what
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
    /// The graduated rule ready for the module, paired with its source url. `Some` iff the candidate
    /// graduated AND a clean near-miss existed to contrast against — either the English self-test proved it
    /// (`verdict == Proven`), or the NOTECARD PATH did (a structurally-attested deprecation that fired
    /// ≥ `REQUIRED_REPS` with a near-miss, whose degenerate identical prose the English judge could not
    /// apply). For a notecard graduation `verdict` stays the honest English result (often `Contradicted`).
    pub rule: Option<(LearnedRule, String)>,
}

/// One construct the reading layer EXTRACTED from a page but never PROPOSED as a rule candidate — the
/// everything-read surface (PASS 25 rung 1). A page names many constructs its role prohibits ([`DocPage`]);
/// the funnel proposes only the page's strongest SUBJECT, and the rest — sibling keywords, the junk operator
/// tokens, the un-chosen member shapes — used to be DISCARDED. They are now retained as UNPROVEN web nodes:
/// present, queryable, cross-linkable, NEVER fired (coverage = everything-read, enforcement = the proven
/// subset). Carries the construct token, one governing sentence from its own page, the source url, and the
/// page's structural deprecation attestation (so a read node still carries its doc-role).
#[derive(Clone, Debug)]
pub struct ReadConstruct {
    /// The construct token the reader extracted — byte-preserved, the web node id.
    pub construct: String,
    /// A governing sentence from the construct's own page that GOVERNS it — the sentence mentioning it when
    /// one exists, else the page's lead sentence (so the node always carries some prose to link meaning by).
    pub governing: String,
    /// The page the construct was read from — the node's source cite.
    pub url: String,
    /// Whether that page structurally attests deprecation ([`DocPage::attested_deprecated`]) — the read
    /// node's doc-role seed.
    pub attested_deprecated: bool,
    /// Whether the page's deprecation is PAGE-SCOPE TRUTH — its OWN banner text-run
    /// ([`crate::lint_attest::Attestation::attests_page_scope`]), never an item badge or sidebar icon.
    /// The element-shaped graded tier trusts only this (the measured `CSSNumericValue/div` leak:
    /// item-route attestation admits API member pages whose URL payload is a bare member name).
    pub page_scope: bool,
    /// Whether the page's own body writes the subject AS AN ELEMENT — the escaped `&lt;name&gt;`
    /// typography an element page titles itself with. The element-hood proof for the `<x>` graded
    /// shape: an attribute or header subject (`scheme`, `Pragma`) never carries it.
    pub element_typography: bool,
}

/// Whether `sentence` MENTIONS `construct` as a code symbol — the backticked form `` `C` `` or `C`
/// as a punctuation/whitespace-delimited token (so `var` does not match inside `variable`). The
/// derivation of `advice` (a SECOND distinct doc sentence about the construct) stands on this.
fn mentions(sentence: &str, construct: &str) -> bool {
    let lower = sentence.to_lowercase();
    // Prose names a member by its TERMINAL name (`substr`, not `.substr` or `String.prototype.substr`),
    // so a member/qualified construct is mentioned by its last dotted segment. A bare construct's
    // terminal is itself, so this is a no-op for the common case.
    let c = construct.trim_start_matches('.').rsplit('.').next().unwrap_or(construct).to_lowercase();
    // A construct that trims to nothing (a bare `.`/`..` token) mentions nothing — and an empty
    // needle would match at every position, walking `from` past the string (measured panic:
    // batch train, byte index 53 of a 52-byte sentence).
    if c.is_empty() {
        return false;
    }
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

/// How many DISTINCT receivers a receiver-generic member shape (`.split`) may ride in the language's own
/// reference corpus and still be considered UNAMBIGUOUS — enforceable as a member rule. Above this, the
/// member name belongs to several unrelated types (`str.split`, `shlex.split`) and a rule on it would flag
/// idiomatic non-deprecated use. Receiver-IDENTITY is judged separately by [`member_demo_ok`].
const MAX_UNAMBIGUOUS_RECEIVERS: usize = 2;

/// Whether an example block DEMONSTRATES the receiver-generic member `member` (leading dot) as the marked
/// item's own usage — the receiver of a firing occurrence must be either the item's OWN PARENT component
/// (case-insensitive: the classmethod/static style `datetime.utcnow()` under id
/// `datetime.datetime.utcnow`) or a BLOCK-LOCAL instance (an identifier the block itself introduced
/// earlier: rustdoc's `let s = …; s.trim_left()`). A FOREIGN-NAMESPACE receiver (`collections.abc.Sequence`
/// demonstrating the deprecation's own recommended replacement for `typing.Sequence`, MEASURED) matches
/// neither and is rejected — the demo is a different item, so the member rule would flag the fix itself.
fn member_demo_ok(block: &str, member: &str, parent: &str) -> bool {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    let mut from = 0usize;
    while let Some(rel) = block[from..].find(member) {
        let at = from + rel;
        let end = at + member.len();
        let bounded = block[end..].chars().next().map(|c| !is_ident(c)).unwrap_or(true);
        if bounded {
            // A LITERAL/EXPRESSION receiver (`"11foo1bar11".trim_left_matches('1')`, `(a + b).abs_sub`) is
            // an instance usage by construction — the value's own type owns the member.
            if matches!(block[..at].chars().next_back(), Some('"') | Some('\'') | Some(')') | Some(']')) {
                return true;
            }
            let recv: String = block[..at]
                .chars()
                .rev()
                .take_while(|c| is_ident(*c))
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            if !recv.is_empty() {
                let recv_start = at - recv.len();
                let deeper = recv_start > 0 && block[..recv_start].ends_with('.');
                if !deeper {
                    if recv.eq_ignore_ascii_case(parent) {
                        return true;
                    }
                    // Block-local instance: the receiver identifier appears EARLIER in the block in a
                    // non-member position (its own introduction), so this usage is an instance call.
                    let earlier = &block[..recv_start];
                    let mut f = 0usize;
                    while let Some(r2) = earlier[f..].find(recv.as_str()) {
                        let s2 = f + r2;
                        let e2 = s2 + recv.len();
                        let b_ok = s2 == 0
                            || (!is_ident(earlier[..s2].chars().next_back().unwrap_or(' '))
                                && !earlier[..s2].ends_with('.'));
                        let a_ok =
                            earlier[e2..].chars().next().map(|c| !is_ident(c)).unwrap_or(true);
                        if b_ok && a_ok {
                            return true;
                        }
                        f = s2 + 1;
                    }
                }
            }
        }
        from = at + 1;
    }
    false
}

/// The number of DISTINCT receiver identifiers `X` such that `X<member>` appears in the reference corpus
/// (`member` carries its leading dot: `.split` counts `re.split`, `s.split`, …). Text-level and
/// language-free: an identifier run immediately before the member's dot is the receiver. The ambiguity
/// referee for receiver-generic member shapes ([`propose`]'s shape reduction).
fn member_receivers(reference: &[String], member: &str) -> usize {
    let mut receivers: std::collections::HashSet<String> = std::collections::HashSet::new();
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    for blk in reference {
        let mut from = 0usize;
        while let Some(rel) = blk[from..].find(member) {
            let at = from + rel;
            // The receiver is the identifier run ending at the dot; member must end on a boundary.
            let end = at + member.len();
            let bounded = blk[end..].chars().next().map(|c| !is_ident(c)).unwrap_or(true);
            if bounded {
                let recv: String = blk[..at]
                    .chars()
                    .rev()
                    .take_while(|c| is_ident(*c))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                if !recv.is_empty() {
                    receivers.insert(recv);
                }
            }
            from = at + 1;
        }
    }
    receivers.len()
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
    // A RECEIVER-GENERIC MEMBER construct (`.substr`) fires only where its property name follows a `.`;
    // the exact `.substr` substring is the sound necessary pre-filter (`run_plan`/`scan_member` is the
    // real referee), and the leading-`.` boundary logic below would reject it (the char before `.` is a
    // receiver alphanumeric).
    if construct.starts_with('.') {
        return code.contains(construct);
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

/// SELF-GENERATE distinct violating samples for `construct` in `lang`, language-GENERAL and covenant-clean:
/// no per-language template, no hand-written fixture — the construct is DATA from the page, the contexts are
/// real corpus blocks, and the frozen `run_plan` is the ONLY referee of what fires (native/history.dx → Fix 2).
///
/// 1. **Carriers** = the shortest snippets that genuinely FIRE `uses_construct(construct)`. Seeds are the
///    page's own incorrect examples (real, language-shaped). When a page has none (a deprecated REFERENCE
///    page), carriers are SYNTHESIZED by splicing the construct into corpus blocks — [`splice_construct`]
///    swaps a whole name token in a real block (`display`→`box-orient`, `p`→`marquee`) and `run_plan` keeps
///    only variants that fire.
/// 2. **Variation** = each carrier spliced into VARIED real contexts (`ctx\ncarrier`, `carrier\nctx`), kept
///    iff still firing and distinct. Distinct real contexts are the independence axis, exactly as the
///    harvested distinct blocks were. Stops at [`GENERATE_TARGET`].
fn generate_violations(lang: &str, construct: &str, seeds: &[String], contexts: &[&str]) -> Vec<String> {
    let plan = Plan::UsesConstruct { construct: construct.to_string() };
    let fires = |b: &str| !run_plan(&plan, lang, b).is_empty();

    let mut carriers: Vec<String> = seeds.iter().filter(|s| fires(s)).cloned().collect();
    if carriers.is_empty() {
        // No seed: synthesize by splicing the construct into real corpus blocks, keeping only firers.
        for ctx in contexts {
            if let Some(c) = splice_construct(construct, ctx).into_iter().find(|c| fires(c)) {
                carriers.push(c);
            }
            if carriers.len() >= 3 {
                break;
            }
        }
    }
    if carriers.is_empty() {
        return Vec::new();
    }

    let mut out: Vec<String> = carriers.clone();
    let mut seen: std::collections::HashSet<u64> = out.iter().map(|s| crate::lint_ai::token_seed(s)).collect();
    'vary: for ctx in contexts {
        for carrier in &carriers {
            for cand in [format!("{ctx}\n{carrier}"), format!("{carrier}\n{ctx}")] {
                if seen.insert(crate::lint_ai::token_seed(&cand)) && fires(&cand) {
                    out.push(cand);
                    if out.len() >= GENERATE_TARGET {
                        break 'vary;
                    }
                }
            }
        }
    }
    out
}

/// Candidate violating snippets from splicing `construct` into a real corpus `block`: for each distinct
/// whole NAME token in the block (a `[A-Za-z][-A-Za-z0-9_]*` run — a selector/property/tag/identifier
/// slot), the block with that token replaced by `construct`. A generic string op seeded by the corpus's own
/// shape (a real CSS rule, a real element), never a Rust-authored snippet; the caller's `run_plan` decides
/// which splices actually fire. Bounded to the first few slots so the sweep stays cheap.
fn splice_construct(construct: &str, block: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let bytes = block.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_alphabetic() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_') {
                i += 1;
            }
            let tok = &block[start..i];
            if tok != construct && !tokens.iter().any(|t| t == tok) {
                tokens.push(tok.to_string());
            }
        } else {
            i += 1;
        }
    }
    tokens.iter().take(12).map(|t| replace_whole_token(block, t, construct)).collect()
}

/// `haystack` with every WHOLE-token occurrence of `from` replaced by `to` — a token is `from` bounded by
/// non-`[A-Za-z0-9_-]` (or string ends), so replacing `p` in `<p>hi</p>` yields `<C>hi</C>` without
/// touching the `p` inside another word. The name-slot substitution [`splice_construct`] stands on.
fn replace_whole_token(haystack: &str, from: &str, to: &str) -> String {
    if from.is_empty() {
        return haystack.to_string();
    }
    let bytes = haystack.as_bytes();
    let boundary = |b: u8| !(b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    let mut out = String::with_capacity(haystack.len());
    let mut from_i = 0;
    while let Some(rel) = haystack[from_i..].find(from) {
        let start = from_i + rel;
        let end = start + from.len();
        let before_ok = start == 0 || boundary(bytes[start - 1]);
        let after_ok = end >= bytes.len() || boundary(bytes[end]);
        out.push_str(&haystack[from_i..start]);
        if before_ok && after_ok {
            out.push_str(to);
        } else {
            out.push_str(&haystack[start..end]);
        }
        from_i = end;
    }
    out.push_str(&haystack[from_i..]);
    out
}

/// A rule's IDENTITY is the construct's EXACT opaque token, byte-preserved — `uses-<construct>` with
/// the construct verbatim, NO slugging or sanitizing anywhere in identity (owner correction 2026-07-12,
/// point 1). Slugging non-alphanumerics to `-` collided `==` and `++` on `uses--`, and the compiled
/// `RuleSet::build` dedups by id, so one silently shadowed the other and `==` never fired live. Byte
/// preservation keeps every distinct construct a distinct rule (`==` → `uses-==`, `++` → `uses-++`,
/// `document.write` → `uses-document.write`). The id is opaque — nothing parses it (the plan rides the
/// rule's own `construct` field, native/history.dx "no id-parsing hack") — so any construct bytes are safe. The
/// construct is DATA read from the prose, so the id is too. Display names are rendering only.
fn rule_id(construct: &str) -> String {
    format!("uses-{construct}")
}

/// The PROPOSED construct candidates for `lang` from its raw doc `pages`, WITHOUT the expensive prove —
/// a fast diagnostic of how many constructs the structural reading discovers and what they are. Each
/// carries its construct, the governing understanding sentence, and the source url. Pure over the pages
/// and the two frozen brains.
pub fn proposed(lang: &str, pages: &[(String, String)], memory: &Memory, m: &MeaningNetwork, en: &English) -> Vec<Candidate> {
    let bridge = Bridge::new(m, en);
    let attest = crate::lint_attest::Attestation::discover(pages);
    let attested: std::collections::HashSet<String> =
        pages.iter().filter(|(_, b)| attest.attests(b)).map(|(u, _)| u.clone()).collect();
    let constructions = crate::lint_construct::mine_and_prove(pages);
    let construction = construction_attestation(pages, &attested, &constructions);
    // Diagnostic path: the abstention withholds are the TRAIN's to record ([`graduate`]) — dropped
    // here, so the ownership scope is vacuous (empty owned set records nothing).
    let (partition, _) = lang_pages(lang, pages, &bridge, en, &attested, &construction, &Default::default());
    propose(lang, &partition, &bridge, en, &attested, &Default::default(), &construction, memory).0
}

/// The EVERYTHING-READ constructs of `lang`'s partition that the funnel NEVER proposed (PASS 25 rung 1) —
/// the read surface minus the proposed candidate tokens, deduped by construct token. These become the
/// retained-UNPROVEN nodes of the language web (present, queryable, never fired). Pure over the read;
/// keyed identically to [`proposed`] so a proposed construct is never also a read-only node.
fn read_not_proposed(candidates: &[Candidate], read_surface: Vec<ReadConstruct>) -> Vec<ReadConstruct> {
    let proposed: std::collections::HashSet<&str> = candidates.iter().map(|c| c.construct.as_str()).collect();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    // PAGE-SCOPE entries dedup FIRST (a construct read from its own banner-attested page is the
    // authoritative read — a co-read of the same token from another page must not shadow it; the
    // measured `center` loss: its own-page entry lost first-wins dedup to an earlier co-read).
    let (scoped, plain): (Vec<ReadConstruct>, Vec<ReadConstruct>) =
        read_surface.into_iter().partition(|r| r.page_scope);
    scoped
        .into_iter()
        .chain(plain)
        .filter(|r| !proposed.contains(r.construct.as_str()) && seen.insert(r.construct.clone()))
        .collect()
}

/// PASS 34 — THE REFERENCE READ (owner ruling 2026-07-15: the web over the current corpus IS the full
/// ideal — every documented subject is knowledge; rules stay derived views). Appends, to the funnel's
/// read surface, one read per subject of every source page the partition EXCLUDED:
/// - an ATTESTED (or construction-bound) ORPHAN — a page the P=R=1.000 notecard faculty proves
///   deprecated but whose own examples demonstrate nothing — reads through the SAME [`read_doc_page`]
///   a partition page gets, and its subjects enter the web as attested nodes ("URL name + notecard is
///   the proof", the [`is_prohibited_subject`] law). Enforcement stays with the graded tier's gates.
/// - an ordinary REFERENCE page mints its URL-subject as a PLAIN read ([`reference_subjects`]) —
///   unattested, no revoked role, so it can never enforce; it is retained knowledge.
/// The funnel's own reads come FIRST in the returned order, so the web build's first-wins dedup and the
/// proven-rule view are byte-identical to the pre-PASS-34 output by construction.
///
/// PASS 36 — the recall census: the two silent exits of this read are NAMED into `withheld`
/// (deduped `(id, reason)` rows for the one conservation ledger): an attested orphan-arm
/// FALL-THROUGH records `read gate (attested but not demonstrated)` per subject (the attestation
/// is the signal), and a reference page whose examples never spell its URL-subject records
/// `read gate (no example spells the subject)` ([`ReferenceRead::Unspelled`] — the URL-derived
/// candidate subject is the signal). A page with neither signal records nothing. Rows are
/// OWNERSHIP-scoped (`owned`, [`crate::lint_docs::owned_urls`]): only a page this language's OWN
/// source crawled records into ITS ledger — the read surface itself stays whole-corpus.
fn with_reference_read(
    pages: &[(String, String)],
    partition: &[&(String, String)],
    bridge: &Bridge,
    en: &English,
    attested: &std::collections::HashSet<String>,
    page_scope: &std::collections::HashSet<String>,
    construction: &std::collections::HashMap<String, Vec<String>>,
    owned: &std::collections::HashSet<String>,
    mut read: Vec<ReadConstruct>,
    withheld: &mut Vec<(String, String)>,
) -> Vec<ReadConstruct> {
    let in_partition: std::collections::HashSet<&str> =
        partition.iter().map(|(u, _)| u.as_str()).collect();
    for (url, body) in pages {
        if in_partition.contains(url.as_str()) {
            continue;
        }
        if attested.contains(url) || construction.contains_key(url) {
            let p = crate::lint_lang_layer::read_doc_page(url, body, en, bridge, attested, construction);
            // TRUE ORPHAN only (the measured leak law): the attested subject is minted iff the
            // deprecation is PAGE-SCOPE — a banner text run (or a proven construction binding),
            // never a class token alone (item-scope, PASS 14) — AND the page's own examples
            // demonstrate NOTHING. A page with subject-bearing examples already faced this
            // language's grammar gate at the partition; overriding that verdict here is the
            // measured cross-language leak (`String.substr` graded into css).
            // The PRE-STRIP page-scope set — the banner is a cross-page INVARIANT run, so the
            // chrome filter strips it from the body this arm reads; re-checking the stripped body
            // here read every banner page as unattested (MEASURED: xmp/plaintext minted nothing).
            let page_scope = construction.contains_key(url) || page_scope.contains(url);
            let demonstrated = p
                .constructs
                .iter()
                .any(|c| p.example_code.iter().any(|b| construct_in_text(b, c)));
            if page_scope && !demonstrated {
                for c in &p.constructs {
                    let governing = p
                        .governing
                        .iter()
                        .filter(|s| mentions(s, c))
                        .max_by_key(|s| s.len())
                        .or_else(|| p.governing.first())
                        .cloned()
                        .unwrap_or_default();
                    let counter = p.counter_attested.iter().flatten().any(|s| s == c);
                    read.push(ReadConstruct {
                        construct: c.clone(),
                        governing,
                        url: url.clone(),
                        attested_deprecated: p.attested_deprecated && !counter,
                        // The orphan arm's own gate IS page-scope truth (banner or construction).
                        page_scope,
                        element_typography: body.contains(&format!("&lt;{c}&gt;")),
                    });
                }
                continue;
            }
            // Not a true orphan: fall through — the page may still contribute a PLAIN read. The
            // fall-through itself is a NAMED drop (PASS 36): the page IS attested (the signal),
            // yet its subjects leave the attested route here — one ledger row per subject,
            // recorded only when this language's own source crawled the page.
            if owned.contains(url) {
                for c in &p.constructs {
                    note_withhold(withheld, format!("read-{c}"), "read gate (attested but not demonstrated)");
                }
            }
        }
        match reference_subjects(url, body) {
            ReferenceRead::Subject { construct, governing } => {
                read.push(ReadConstruct {
                    construct: construct.clone(),
                    governing,
                    url: url.clone(),
                    attested_deprecated: false,
                    // A demonstrated page can still be a BANNER page (center/strike — the sentence wall
                    // kept them from candidacy, not from truth): carry the pre-strip page-scope fact.
                    page_scope: page_scope.contains(url),
                    element_typography: body.contains(&format!("&lt;{construct}&gt;")),
                });
            }
            ReferenceRead::Unspelled { subject } => {
                if owned.contains(url) {
                    note_withhold(withheld, format!("read-{subject}"), "read gate (no example spells the subject)");
                }
            }
            ReferenceRead::Mute => {}
        }
    }
    read
}

/// Dedup-append one named withhold row (PASS 36) — the same `(id, reason)` pair discipline
/// [`crate::lint_match::RuleSet`]'s ledger holds, applied while the rows are still being collected.
fn note_withhold(rows: &mut Vec<(String, String)>, id: String, reason: &str) {
    let row = (id, reason.to_string());
    if !rows.contains(&row) {
        rows.push(row);
    }
}

/// What an ordinary reference page's plain read yielded ([`reference_subjects`]).
#[derive(Debug, PartialEq)]
enum ReferenceRead {
    /// The URL-derived subject its own example code demonstrates, with its governing sentence.
    Subject { construct: String, governing: String },
    /// The page carries a URL-derived candidate subject, but no own example spells it (or the
    /// page has no example block at all) — the named drop of PASS 36.
    Unspelled { subject: String },
    /// No URL-derived candidate subject of usable shape — nothing was on the table.
    Mute,
}

/// The plain READ subject of an ordinary reference page: the most-specific URL-derived
/// shape ([`crate::lint_lang_layer::member_page_shapes`]) that the page's OWN example code demonstrates
/// (`construct_in_text` over its `<pre><code>` corpus) — the demonstration is the confirmation, so a
/// slug no example spells (`tag_video.asp`) never mints a junk node (it returns as
/// [`ReferenceRead::Unspelled`], the PASS-36 named drop, instead of vanishing). Governing prose is the
/// page's own sentence mentioning the subject, else its lead. Pure; no brain, no grammar, no network.
fn reference_subjects(url: &str, body: &str) -> ReferenceRead {
    let shapes = crate::lint_lang_layer::member_page_shapes(url);
    let unspelled = || match shapes.iter().find(|c| c.len() >= 2) {
        Some(subject) => ReferenceRead::Unspelled { subject: subject.clone() },
        None => ReferenceRead::Mute,
    };
    let own = crate::lint_lang_layer::page_example_corpus(body, false);
    if own.is_empty() {
        return unspelled();
    }
    let Some(construct) = shapes
        .iter()
        .find(|c| c.len() >= 2 && own.iter().any(|blk| construct_in_text(blk, c)))
        .cloned()
    else {
        return unspelled();
    };
    let pool = crate::lint_lang_layer::governing_sentences(body);
    let governing = pool
        .iter()
        .filter(|s| mentions(s, &construct))
        .max_by_key(|s| s.len())
        .or_else(|| pool.first())
        .cloned()
        .unwrap_or_default();
    ReferenceRead::Subject { construct, governing }
}

/// The per-page CONSTRUCTION ATTESTATION map (COMPLETION PASS 23 — the rung-1 consumer wiring): url →
/// the subjects a page attests by BINDING a proven construction on its own prose
/// ([`crate::lint_construct::attested_subjects`]). A page ALREADY attested by the existing faculty
/// (`attested`) is EXCLUDED so its rule stays on the unchanged notecard/reference route (byte-identical);
/// only the removal-ONLY pages (a whole-module removal construction but no inline deprecation class) newly
/// bind here. Empty when no construction is proven for the corpus (every non-python language today), so
/// every other language's reading is untouched. Built over the RAW (pre-chrome-strip) bodies — the
/// construction scaffold VARIES page-to-page, so it survives the strip, but the raw bodies are the exact
/// text the miner proved over.
fn construction_attestation(
    pages: &[(String, String)],
    attested: &std::collections::HashSet<String>,
    constructions: &[crate::lint_construct::ConstructionState],
) -> std::collections::HashMap<String, Vec<String>> {
    let mut map = std::collections::HashMap::new();
    if constructions.is_empty() {
        return map;
    }
    for (url, body) in pages {
        if attested.contains(url) {
            continue;
        }
        let subjects = crate::lint_construct::attested_subjects(constructions, url, body);
        if !subjects.is_empty() {
            map.insert(url.clone(), subjects);
        }
    }
    map
}

/// PARTITION whole-site doc pages to the ones that PROVE in this language — decided by GRAMMAR
/// VERIFICATION, never by URL attribution (owner directive 2026-07-12: "language assignment emerges from
/// understanding/verification, not URL attribution"). A page belongs to `lang`'s partition iff a
/// construct its ROLE prohibits ([`crate::lint_lang_layer::read_doc_page`] — a rule page's subject, a
/// deprecated reference page's subject) FIRES on the page's OWN worked-example code under `lang`'s
/// grammar ([`run_plan`], the frozen referee). This is the owner's squeeze made structural: a CSS
/// deprecation page's example code (`box-orient: horizontal`) fires `uses_construct(box-orient)` only
/// under the CSS grammar, so `box-orient` can never leak into JavaScript even though the whole MDN corpus
/// is proposed to every language; and a CROSS-SECTION page (a Web-API `document.write` page whose example
/// is JavaScript) joins the JS partition regardless of its `/Web/API/` URL shape. A page whose subject
/// fires in NO grammar abstains — never conflated, never guessed. No language or domain is named in code;
/// the tree-sitter grammar of `lang` is the only judge.
///
/// PASS 36 — the recall census: the abstention is no longer a silent drop. A prohibition page whose
/// subjects fire in none of THIS grammar's examples returns, beside the partition, one named
/// withhold row per subject — `(read-<subject>, "read gate (no grammar claims the page)")` —
/// deduped by the pair, for [`graduate`] to thread into the module's one conservation ledger.
/// Signal-gated by construction: only a page whose ROLE prohibits named subjects records
/// ([`PageClaim::Abstains`]); an ordinary page records nothing. Rows are OWNERSHIP-scoped
/// (`owned`, [`crate::lint_docs::owned_urls`]): the pooled site corpus is proposed to every
/// language, so a row is recorded only for a page this language's OWN source crawled — the
/// partition itself stays whole-corpus.
fn lang_pages<'a>(
    lang: &str,
    pages: &'a [(String, String)],
    bridge: &Bridge,
    en: &English,
    attested: &std::collections::HashSet<String>,
    construction: &std::collections::HashMap<String, Vec<String>>,
    owned: &std::collections::HashSet<String>,
) -> (Vec<&'a (String, String)>, Vec<(String, String)>) {
    let mut partition: Vec<&(String, String)> = Vec::new();
    let mut withheld: Vec<(String, String)> = Vec::new();
    for p in pages {
        let (u, body) = p;
        match page_proves_in_lang(lang, u, body, bridge, en, attested, construction) {
            PageClaim::Proves => partition.push(p),
            PageClaim::Abstains(subjects) => {
                if !owned.contains(u) {
                    continue;
                }
                for c in subjects {
                    note_withhold(&mut withheld, format!("read-{c}"), "read gate (no grammar claims the page)");
                }
            }
            PageClaim::Mute => {}
        }
    }
    (partition, withheld)
}

/// How one page relates to a language's grammar-verified partition ([`page_proves_in_lang`]).
enum PageClaim {
    /// A prohibited subject fires on the page's own examples under this grammar — in the partition.
    Proves,
    /// A prohibition page whose named subjects fire in none of this grammar's examples — the
    /// honest abstention, carrying the subjects so the drop is a NAMED withhold (PASS 36).
    Abstains(Vec<String>),
    /// Not a prohibition page at all — nothing was dropped, nothing to record.
    Mute,
}

/// Whether a prohibition/deprecation page PROVES in `lang`: its role names a prohibited subject AND that
/// subject fires on the page's OWN worked-example code under `lang`'s grammar (the frozen `run_plan`).
/// The page's example code is read STRUCTURALLY ([`crate::lint_lang_layer::page_code_corpus`], every
/// `<pre><code>`), so the referee is the language grammar, not the URL — the whole point of the
/// verification-decided partition. A non-prohibition page, or one whose subject the grammar does not fire
/// on the page's own examples, is not in this language's partition ([`PageClaim`] carries which).
fn page_proves_in_lang(lang: &str, url: &str, body: &str, bridge: &Bridge, en: &English, attested: &std::collections::HashSet<String>, construction: &std::collections::HashMap<String, Vec<String>>) -> PageClaim {
    // A CONSTRUCTION-BOUND page joins this language's partition by the construction's PROOF, not a per-page
    // grammar demonstration: the construction was proven over THIS language's own corpus (its witnesses are
    // this language's proven-deprecated subjects), so it already established both the language and the
    // prohibition. Its removal-only subject need not re-demonstrate in a `<pre><code>` block (a removed
    // module's stub page often has none). The map is non-empty only for the language the construction
    // proved in, so this never crosses the partition into another language.
    if construction.contains_key(url) {
        return PageClaim::Proves;
    }
    let page = crate::lint_lang_layer::read_doc_page(url, body, en, bridge, attested, construction);
    if !page.prohibited || page.constructs.is_empty() {
        return PageClaim::Mute;
    }
    // A rendered-marker page (Python/Rust) demonstrates its items in bare inline `<code>`, not
    // `<pre><code>`, so its example corpus is the WIDENED reading; a URL-subject page (MDN) has no
    // markers and reads exactly the frozen `<pre><code>` corpus — byte-identical.
    // A rendered-marker page stays one when every item is counter-attested (PASS 28): the marker
    // typography is the structural fact; counter-attestation only narrows enforcement.
    let own = if page.marked_deprecated.is_empty() && page.counter_attested.is_empty() {
        crate::lint_lang_layer::page_code_corpus(
            std::slice::from_ref(&(url.to_string(), body.to_string())),
            lang,
            MAX_HARVEST_BLOCKS,
        )
    } else {
        crate::lint_lang_layer::page_example_corpus(body, true)
    };
    let proves = page.constructs.iter().any(|c| {
        let plan = Plan::UsesConstruct { construct: c.clone() };
        // PRIMARY-EXAMPLE language gate. The subject's language is where its FIRST DEMONSTRATED usage —
        // the EARLIEST own example block (document order) that contains it — parses CLEANLY under `lang`
        // (`parses_cleanly`, not merely an error-tolerant parse) and FIRES. Two leaks this closes,
        // MEASURED:
        //   (a) error-tolerant leak — a CSS `clip: rect(…)` / an HTML `<center>` exposes a stray leaf
        //       under the JS grammar; the clean-parse requirement rejects it (and `scan_construct` skips
        //       JSX, so `<center>` is not a JS usage either);
        //   (b) remedy-block leak — an HTML `<center>` element page carries a CSS `.center{…}` REMEDY
        //       whose class/value `center` fires cleanly under CSS; gating on the FIRST subject-bearing
        //       block (the `<center>` deprecated-usage demonstration, not the later remedy) keeps the
        //       subject in HTML and out of CSS.
        // The frozen `run_plan` under `lang`'s grammar is the only referee; no language is named.
        own.iter().find(|blk| construct_in_text(blk, c)).is_some_and(|blk| {
            crate::lint_trace::parses_cleanly(lang, blk) && !run_plan(&plan, lang, blk).is_empty()
        })
    });
    if proves {
        PageClaim::Proves
    } else {
        PageClaim::Abstains(page.constructs.clone())
    }
}

/// One clean governing sentence in the pooled reading, tagged with whether it came from a PROHIBITED
/// page (so `understanding` selection can prefer a real prohibition statement) and its source url.
#[derive(Clone)]
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
fn propose(lang: &str, pages: &[&(String, String)], bridge: &Bridge, en: &English, attested: &std::collections::HashSet<String>, page_scope: &std::collections::HashSet<String>, construction: &std::collections::HashMap<String, Vec<String>>, memory: &Memory) -> (Vec<Candidate>, Vec<PooledSentence>, Vec<ReadConstruct>) {
    let mut docpages: Vec<crate::lint_lang_layer::DocPage> =
        pages.iter().map(|(url, body)| crate::lint_lang_layer::read_doc_page(url, body, en, bridge, attested, construction)).collect();

    // EVERYTHING-READ SURFACE (PASS 25 rung 1). Captured HERE, before the subject-selection mutation below
    // reduces each page's `constructs` to its chosen subject — this is the FULL set the reading layer
    // extracted (page subjects, item-unit subjects, the code-typography tokens of the governing prose). The
    // funnel proposes only the strongest subject per page; the rest are retained as unproven web nodes. Each
    // read construct carries a governing sentence from its own page (the one mentioning it, else the lead).
    let mut read_surface: Vec<ReadConstruct> = Vec::new();
    for (p, (_, page_body)) in docpages.iter().zip(pages.iter()) {
        for c in &p.constructs {
            let governing = p
                .governing
                .iter()
                .filter(|s| mentions(s, c))
                .max_by_key(|s| s.len())
                .or_else(|| p.governing.first())
                .cloned()
                .unwrap_or_default();
            // PASS 28 — a counter-attested item (its own note excludes it or deprecates only a usage
            // form) stays a READ node but must not carry the revoked doc-role seed: it is not
            // deprecated, so neither the proven funnel nor the graded tier may enforce it.
            let counter = p.counter_attested.iter().flatten().any(|s| s == c);
            read_surface.push(ReadConstruct {
                construct: c.clone(),
                governing,
                url: p.url.clone(),
                attested_deprecated: p.attested_deprecated && !counter,
                page_scope: page_scope.contains(&p.url),
                element_typography: page_body.contains(&format!("&lt;{c}&gt;")),
            });
        }
    }

    // QUALIFIED-MEMBER SHAPE SELECTION (native/history.dx → "QUALIFIED-MEMBER construct extraction"). A
    // deprecated reference page proposes several candidate SHAPES for its subject, most-specific first
    // (qualified `RegExp.input`, receiver-generic member `.substr`, bare `substr`). Keep the FIRST that
    // FIRES on the page's OWN example code under `lang`'s grammar — the covenant-clean squeeze: a
    // prototype method member-shapes to `.substr` (so an ordinary `const substr = 1` never fires), a
    // static keeps its receiver (`RegExp.input`, so `el.input` never fires), and a subject the grammar
    // never confirms as a member falls to bare only where bare is the firing shape (a CSS property, a
    // global function). A page whose subject fires in NO proposed shape contributes nothing. The frozen
    // `run_plan` is the only referee; no language is named.
    for p in &mut docpages {
        if !p.attested_deprecated || p.constructs.len() <= 1 {
            continue;
        }
        let fires_on_own = |c: &str| {
            let plan = Plan::UsesConstruct { construct: c.to_string() };
            p.example_code
                .iter()
                .any(|blk| construct_in_text(blk, c) && !run_plan(&plan, lang, blk).is_empty())
        };
        if p.marked_deprecated.is_empty() {
            // URL-subject page (MDN): one subject, first firing shape wins — the frozen selection.
            let chosen = p.constructs.iter().find(|c| fires_on_own(c)).cloned();
            p.constructs = chosen.into_iter().collect();
        } else {
            // RENDERED-MARKER page: the unit is the ITEM (an API page marks many — ssl.html marks 20).
            // Per item, keep the FIRST shape that fires on the page's own demonstrated usage — with the
            // RECEIVER-GENERIC member (`.split`) admitted only when its member name is UNAMBIGUOUS in the
            // language's own reference corpus ([`member_receivers`] ≤ [`MAX_UNAMBIGUOUS_RECEIVERS`]):
            // `.utcnow`/`.compare_and_swap` are one-receiver names and stay enforceable; `.split`/`.name`
            // ride many receivers (`str.split`, `Path.name`) and a rule on them would flag idiomatic
            // non-deprecated use, so they abstain and the item contributes a receiver-specific shape or
            // nothing. Data-driven (the corpus the language's own docs demonstrate), no word list.
            let mut chosen: Vec<String> = Vec::new();
            for group in &p.marked_deprecated {
                // The item's parent component, from the group's most-specific shape (the full anchor id).
                let parent = group
                    .first()
                    .and_then(|full| {
                        let parts: Vec<&str> = full.split('.').collect();
                        (parts.len() >= 2).then(|| parts[parts.len() - 2].to_string())
                    })
                    .unwrap_or_default();
                let pick = group.iter().find(|c| {
                    if !c.starts_with('.') {
                        return fires_on_own(c);
                    }
                    if member_receivers(&memory.reference, c) > MAX_UNAMBIGUOUS_RECEIVERS {
                        return false;
                    }
                    let plan = Plan::UsesConstruct { construct: (*c).clone() };
                    p.example_code.iter().any(|blk| {
                        construct_in_text(blk, c)
                            && !run_plan(&plan, lang, blk).is_empty()
                            && member_demo_ok(blk, c, &parent)
                    })
                });
                if let Some(c) = pick {
                    if !chosen.contains(c) {
                        chosen.push(c.clone());
                    }
                }
            }
            p.constructs = chosen;
        }
    }
    let docpages = docpages;
    let mut pooled: Vec<PooledSentence> = Vec::new();
    for p in &docpages {
        for s in &p.governing {
            let negated = crate::lint_corroborate::is_negated(en, s);
            pooled.push(PooledSentence { sentence: s.clone(), prohibited: p.prohibited, url: p.url.clone(), negated });
        }
    }

    // ONE construct graduates per page: a rule/reference page prohibits its own SUBJECT, so among the
    // constructs that pass the subject gate, keep the single strongest. This kills the CONTEXTUAL-rule
    // junk that fires on the same counts as a real subject (`var` and junk `if` are numerically IDENTICAL
    // — 2/2 incorrect, 1/2 correct — so only the SUBJECT signal separates them).
    let mut out: Vec<Candidate> = Vec::new();
    for (p, (_, page_body)) in docpages.iter().zip(pages.iter()) {
        // The lead summary's stated subject(s) — the covenant-clean `??`-vs-`==` discriminator. Read once
        // per page from the first governing sentence (the title + lead the reader welds together).
        let lead_named: Vec<String> = p
            .governing
            .first()
            .map(|s| bridge.constructs_named(s))
            .unwrap_or_default()
            .into_iter()
            .map(|(c, _)| crate::lint_lang_layer::normalize_construct(&c))
            .collect();
        // A subject is confirmed either by the URL naming it ([`is_prohibited_subject`], the
        // URL-subject sites — ONE subject per page, the page IS the item), OR — for a RENDERED-MARKER
        // site (Python/Rust) whose URL names the module/type, not the member — by the deprecation marker
        // sitting on the member's OWN item anchor ([`DocPage::marked_deprecated`]) — one candidate PER
        // MARKED ITEM (an API page marks many). The anchor confirmation is the structural parallel of the
        // URL payload; MDN's dotless-id banner yields an empty marked set, so this never admits a
        // URL-subject-site construct the URL did not already name, and never widens MDN past one subject.
        let subjects: Vec<&String> = if p.marked_deprecated.is_empty() {
            p.constructs
                .iter()
                .filter(|c| !out.iter().any(|o| &o.construct == *c))
                // A CONSTRUCTION-BOUND subject bypasses the URL-payload gate: the proven construction
                // (not the URL's path segment) confirmed the subject, so a removed module whose page url
                // carries a `.html` the payload gate would reject (`cgi.html` ≠ `cgi`) still graduates.
                .filter(|c| p.construction_attested || is_prohibited_subject(lang, &p.url, c, &p.incorrect, &p.correct))
                .max_by_key(|c| subject_score(lang, c, &p.incorrect))
                .into_iter()
                .collect()
        } else {
            p.constructs
                .iter()
                .filter(|c| !out.iter().any(|o| &o.construct == *c))
                .filter(|_| p.attested_deprecated)
                .collect()
        };
        for construct in subjects {
        // The stated-subject gate is computed here but NOT used to reject the candidate: rejecting at
        // PROPOSE would shrink the pool and reshuffle the frozen self-test's order-sensitive foil, flipping
        // UNRELATED verdicts (MEASURED: dropping the junk operators `??`/`+=`/`!=` here spuriously
        // Contradicted `eval` and graduated `++`). The candidate STAYS in the pool (identical foils/advice
        // to no gate); `graduate` simply does not EMIT a rule whose subject its own lead never states.
        // A CONSTRUCTION-BOUND subject is STATED by the construction binding itself — the proven scaffold
        // named this page's own subject in its slot, a stronger structural "this page's subject" signal
        // than the lead heuristic (which the removed-module stub prose may not satisfy).
        let stated = p.construction_attested || stated_by_lead(lang, construct, &lead_named, &p.incorrect, &p.correct);
        // The `understanding` is the best real doc sentence MENTIONING the construct: prefer one from the
        // construct's OWN page (page-of-origin), then a prohibited-page statement, then negative polarity,
        // then a longer (more informative) one. Its citation url is the proposing page.
        let best = pooled
            .iter()
            .filter(|ps| mentions(&ps.sentence, construct))
            .max_by_key(|ps| {
                (u32::from(ps.url == p.url), u32::from(ps.prohibited), u32::from(ps.negated), ps.sentence.len())
            });
        if let Some(best) = best {
            let plan = Plan::UsesConstruct { construct: construct.clone() };
            let seeds: Vec<String> =
                p.incorrect.iter().filter(|b| !run_plan(&plan, lang, b).is_empty()).cloned().collect();
            out.push(Candidate {
                construct: construct.clone(),
                understanding: best.sentence.clone(),
                url: p.url.clone(),
                seeds,
                page_scope: page_scope.contains(&p.url),
                element_typography: page_body.contains(&format!("&lt;{construct}&gt;")),
                attested_deprecated: p.attested_deprecated,
                stated,
            });
        }
        }
    }
    (out, pooled, read_surface)
}

/// Whether `construct` is the page's PROHIBITED SUBJECT — what the page is ABOUT and forbids — separating a
/// genuine bare-use prohibition from a CONTEXTUAL rule (which forbids a pattern-in-a-context, not the
/// construct's bare use). A keyword and an operator are confirmed differently because only one of them can
/// live in a URL:
/// - **Keyword / identifier / property / element** — confirmed by the page's URL rule-name PAYLOAD equalling
///   the construct ([`url_payload_equals`]): a doc page NAMES its whole subject in its path
///   (`no-var`→`var`, `no-eval`→`eval`, `no-console`→`console`, `/CSS/box-orient`, `/Element/marquee`).
///   `no-delete-var`→`delete-var`≠`delete` and `no-async-promise-executor`→`async-promise-executor`≠`async`
///   are pattern names, not the bare construct, so they ABSTAIN. This is why the example FIRING is not
///   required for a keyword: the URL is the confirmation, and it correctly admits `eval` even though eval's
///   own `allowIndirect` "correct" example reuses `eval` (the example test alone would wrongly reject it).
///   A per-SOURCE structural marker, INTERIM, exactly like the `/rules/`|`/reference/` keying.
/// - **Multi-character OPERATOR** (`==`/`!=`) — a symbol a URL cannot spell, so it is confirmed by the docs'
///   OWN before/after example pair (frozen `run_plan`): the page must carry correct examples (a genuine
///   remedy demonstration — `no-self-compare` ships NONE, so its incidental `===` abstains), the operator
///   fires on EVERY incorrect example, and NOT on the PRIMARY correct (the remedy drops it; later correct
///   blocks are option exceptions like eqeqeq `smart` `x == null`, so PRIMARY not ALL).
///
/// **The REMEDY-DEMONSTRATION discriminator (kills the CONTEXTUAL bare-use junk `new`/`undefined`/`void`).**
/// An UNCONDITIONAL ban's remedy example demonstrates the construct's ABSENCE — at least one `correct`
/// block drops the construct (`var`→`let`, `==`→`===`, `eval`→`JSON.parse`, `with`→direct access). A
/// CONTEXTUAL rule (`no-new`, `no-undefined`, `no-void`) forbids only a PATTERN, so EVERY one of its
/// `correct` blocks STILL uses the construct — it is demonstrating the construct's acceptable uses, not
/// replacing it. So a candidate whose every own `correct` example still fires the construct is contextual
/// and ABSTAINS. MEASURED (2026-07-11): this is TRUE for exactly `new`/`undefined`/`void` and FALSE for
/// every wanted classic — `eval`'s `allowIndirect` "correct" reuses `eval` but its `JSON.parse` correct is
/// construct-free, so eval is (soundly) kept. A deprecated REFERENCE page has NO correct examples (this
/// test is vacuous there), so CSS/HTML deprecations are unaffected. The residual mixed-pattern junk
/// (`??` on no-constant-binary-expression, `console` on no-console) fires on SOME correct and not others —
/// structurally IDENTICAL to `==`/`eval`, so no example-firing test can drop it without losing the wanted
/// operators; that is an honest, documented limit, not a gate to widen.
fn is_prohibited_subject(lang: &str, url: &str, construct: &str, incorrect: &[String], correct: &[String]) -> bool {
    let plan = Plan::UsesConstruct { construct: construct.to_string() };
    let fires = |b: &str| !run_plan(&plan, lang, b).is_empty();
    // Contextual bare-use ABSTAIN: every remedy example still uses the construct ⇒ the rule demonstrates
    // acceptable uses, not a replacement ⇒ not an unconditional ban. Vacuous when there are no correct
    // examples (a deprecated reference page), so it never touches the notecard-attested CSS/HTML class.
    if !correct.is_empty() && correct.iter().all(|g| fires(g)) {
        return false;
    }
    if is_operator(construct) {
        if correct.is_empty() || incorrect.is_empty() {
            return false;
        }
        return incorrect.iter().all(|b| fires(b)) && !correct.first().is_some_and(|g| fires(g));
    }
    // Keyword/identifier/property/element: the URL names it as the whole subject AND it genuinely FIRES on
    // every bad example (so a rule-name that is NOT a firing token — `max-statements`, `vars-on-top` — is
    // rejected). A reference page carries no examples, so the URL name + deprecation notecard is the proof.
    url_payload_equals(url, construct) && (incorrect.is_empty() || incorrect.iter().all(|b| fires(b)))
}

/// Whether the page's LEAD SUMMARY structurally STATES `construct` as its subject — the covenant-clean
/// `??`-vs-`==` discriminator (native/history.dx → the stated-subject gate, owner ruling 2026-07-11). A genuine
/// single-construct ban NAMES its subject in the title/lead sentence, one of two ways:
/// - **directly** — the lead names the construct itself (`no-console` → "Disallow the use of `console`",
///   `no-eval`, `no-with`); or
/// - **by its REMEDY** — a construct advised AGAINST in favour of a replacement is stated by naming that
///   replacement (`no-var` → "Require `let` … instead of `var`", `eqeqeq` → "Require `===` and `!==`"):
///   the lead names the remedy, and the docs' OWN before/after pair shows the remedy REPLACING the banned
///   construct (the remedy fires the primary correct example and NO incorrect example; the candidate fires
///   an incorrect example and NOT the primary correct).
///
/// `no-constant-binary-expression` states NEITHER for `??`: its lead lists `||`/`&&`/`??` as a CO-EQUAL
/// class (the central extraction reads the rule's real subject, "expressions where the operation doesn't
/// affect the value", naming no single operator), and its correct examples REUSE the same operators as the
/// incorrect ones (no replacement construct is introduced) — so `??` is not the stated subject and is
/// dropped, while `==` (whose remedy `===` IS named and IS introduced in the correct example) survives.
/// `lead_named` is the covenant-clean construct(s) the frozen extraction reads from the lead sentence
/// ([`Bridge::constructs_named`] — one central construct per sentence; NEVER a word list). Vacuously true
/// for a page with NO paired examples (a deprecated reference page — its notecard is the proof, so the
/// lead gate must not touch it), keeping CSS/HTML unaffected.
fn stated_by_lead(lang: &str, construct: &str, lead_named: &[String], incorrect: &[String], correct: &[String]) -> bool {
    // (A) the candidate IS the lead's stated subject.
    if lead_named.iter().any(|s| s == construct) {
        return true;
    }
    // A deprecated reference page carries no paired examples — the lead gate does not apply (the notecard
    // is its proof). Leave it to the notecard path.
    if incorrect.is_empty() || correct.is_empty() {
        return true;
    }
    // (B) REMEDY path: the candidate is the banned counterpart (fires an incorrect example, not the primary
    // correct), and a lead-named construct is the remedy the docs demonstrate (fires the primary correct,
    // fires NO incorrect — it is the introduced replacement). Frozen `run_plan` is the only referee.
    let fires = |c: &str, b: &str| !run_plan(&Plan::UsesConstruct { construct: c.to_string() }, lang, b).is_empty();
    let primary = &correct[0];
    let cand_banned = incorrect.iter().any(|b| fires(construct, b)) && !fires(construct, primary);
    cand_banned
        && lead_named
            .iter()
            .any(|s| s != construct && fires(s, primary) && incorrect.iter().all(|b| !fires(s, b)))
}

/// A rank over subject-gate passers so at most ONE graduates per page: prefer the construct present the most
/// TIMES across the incorrect examples (the true subject recurs; an incidental token appears once), then the
/// longer/more-specific token. Data-only, no construct list.
fn subject_score(lang: &str, construct: &str, incorrect: &[String]) -> (usize, usize) {
    let plan = Plan::UsesConstruct { construct: construct.to_string() };
    let occurrences: usize = incorrect.iter().map(|b| run_plan(&plan, lang, b).len()).sum();
    (occurrences, construct.len())
}

/// Slugify a token to lowercase, non-alphanumerics folded to `-`, empty runs collapsed, ends trimmed —
/// so `box-orient`, `Box Orient`, and `boxOrient`… normalize comparably. Pure lexical, no vocabulary.
fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        out.push(if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' });
    }
    out.split('-').filter(|t| !t.is_empty()).collect::<Vec<_>>().join("-")
}

/// Whether the page's URL rule-name PAYLOAD equals `construct` — the doc page's own path segment for its
/// subject, minus a leading `no-` negator (`no-var`→`var`, `no-console`→`console`, `.../box-orient`→
/// `box-orient`, `.../Element/marquee`→`marquee`). EQUALITY, not containment, is the discriminator: a
/// pattern-named rule (`no-delete-var`→`delete-var`, `no-async-promise-executor`→`async-promise-executor`)
/// does NOT equal its incidental construct and is rejected. Pure DATA-vs-DATA, no construct or language name.
fn url_payload_equals(url: &str, construct: &str) -> bool {
    let last = url.trim_end_matches('/').rsplit('/').next().unwrap_or("");
    let seg = slugify(last);
    let payload = seg.strip_prefix("no-").unwrap_or(&seg);
    // A member/qualified construct (`RegExp.input`, `.substr`) names its subject in its TERMINAL dotted
    // segment; the URL's last segment is that same subject (`…/RegExp/input`, `…/String/substr`). So the
    // payload is compared to the construct's terminal-segment slug, not the whole dotted slug — a bare
    // construct's terminal is itself, so this is unchanged for the common case, and a pattern rule
    // (`no-delete-var` → `delete-var` ≠ `delete`) is still rejected.
    let terminal = construct.trim_start_matches('.').rsplit('.').next().unwrap_or(construct);
    let cslug = slugify(terminal);
    !cslug.is_empty() && payload == cslug
}

/// Whether `construct` is a multi-character OPERATOR — two or more symbol characters, no alphanumerics
/// (`==`, `!=`, `===`). A symbol a URL cannot spell, so the subject gate vets it by the example firing
/// alone. Mirrors [`crate::lint_lang_layer`]'s operator reading.
fn is_operator(construct: &str) -> bool {
    construct.len() >= 2 && construct.chars().all(|c| !c.is_ascii_alphanumeric() && c != '.')
}

/// What the GENERATOR expects the blind linter to do with a self-generated sample — DERIVED from the
/// rule's understanding, one of two ways (owner correction 2026-07-12, point 3): a violation it expects
/// to FLAG, or a clean near-miss it expects the linter to leave alone. The lint side NEVER sees this.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expect {
    /// The generator wrote this to embody the violation — it expects the rule to FIRE.
    Flag,
    /// The generator wrote this as a valid near-miss (the construct absent / the remedy form) — it
    /// expects NO flag. A clean sample counts toward agreement exactly as a flagging one does.
    Clean,
}

/// A self-generated example paired with the generator's expectation. The blind lint path ([`blind_fires`])
/// receives the `code` ALONE; `expect` is compared to the blind outcome only AFTERWARD, so the linter can
/// never bias its outcome by what it "should" say — the type separation IS the blindness (owner point 3).
struct Sample {
    /// The self-generated code the blind linter receives — the only thing it sees.
    code: String,
    /// The generator's expectation, derived from the understanding — invisible to the lint side.
    expect: Expect,
}

/// The BLIND lint outcome for one sample: whether the language's real firing engine flags `code`.
/// Receives the CODE ONLY — no [`Expect`], no understanding, no advice — so the outcome cannot be
/// influenced by the generator's expectation (the demonstrable blindness of owner point 3; this is the
/// same `run_plan` the live linter fires). A separate function so the blindness is a TYPE fact.
fn blind_fires(plan: &Plan, lang: &str, code: &str) -> bool {
    !run_plan(plan, lang, code).is_empty()
}

/// The BLIND-AGREEMENT graduation loop (owner correction 2026-07-12, point 3), computed over this
/// workflow's SINGLE-RULE self-test book. Two sides that share the same understanding substrate but NOT
/// the expectation: the GENERATOR tagged each [`Sample`] with an [`Expect`]; the blind linter
/// ([`blind_fires`], code only) produces the outcome; each rep reduces to an agreement judged by the
/// FROZEN comparator and folded by the FROZEN counting law [`fold_reps`]. The three-argument English
/// reconciliation [`crate::lint_corroborate::corroborates`]`(understanding, advice, foil)` is CONSTANT
/// across the ≤ ([`PROVE_SAMPLE_CAP`]+clean) reps of a one-rule book, so it is computed ONCE and reused
/// (the memoization native/architecture.dx's speed pass blessed — a pure comparator re-evaluated identically per rep).
///
/// Per rep, expectation × blind outcome:
/// - **Flag, fired** — the sides agree behaviorally; the English judge decides whether the found advice
///   reconciles with the understanding (frozen `Some(true)`→[`Rep::Corroborates`], `Some(false)`→
///   [`Rep::Mismatch`] (fatal), `None`→[`Rep::Undecidable`]).
/// - **Flag, not fired** — a phased-out expectation ([`Rep::NotFlagged`]); reported, never hidden.
/// - **Clean, not fired** — the sides AGREE this near-miss is valid; it COUNTS ([`Rep::Corroborates`])
///   but ONLY when the rule's English genuinely reconciles (`Some(true)`) — "the agreement comes from
///   the KNOWLEDGE" (owner). Without reconciling knowledge a clean agreement is [`Rep::Undecidable`]
///   (neither counts nor blocks), so a non-understood rule cannot graduate on clean samples alone.
/// - **Clean, fired** — a false positive: the rule flags code the understanding calls valid, a genuine
///   self-contradiction, fatal exactly as a flagged [`Rep::Mismatch`].
///
/// For an ALL-`Flag` sample set this is bit-identical to the frozen [`crate::lint_selftest::prove`]
/// (asserted by `blind_prove_matches_frozen_prove`); the clean reps are the new, additive squeeze.
fn prove_blind(
    m: &MeaningNetwork,
    en: &English,
    rule: &RuleUnderTest,
    plan: &Plan,
    advice: &str,
    samples: &[Sample],
) -> Verdict {
    let reconciled = crate::lint_corroborate::corroborates(m, en, &rule.understanding, advice, &rule.foil);
    let reps = samples.iter().map(|s| {
        let fires = blind_fires(plan, &rule.lang, &s.code); // BLIND: code only, no expectation
        match (s.expect, fires) {
            (Expect::Flag, true) => match reconciled {
                Some(true) => Rep::Corroborates,
                Some(false) => Rep::Mismatch(advice.to_string()),
                None => Rep::Undecidable,
            },
            (Expect::Flag, false) => Rep::NotFlagged,
            (Expect::Clean, false) => match reconciled {
                Some(true) => Rep::Corroborates,
                _ => Rep::Undecidable,
            },
            (Expect::Clean, true) => Rep::Mismatch(advice.to_string()),
        }
    });
    fold_reps(reps)
}

/// GRADUATE construct rules for `lang` — the whole workflow, pure over the language's raw doc `pages`
/// (the PROPOSE source, read structurally by [`crate::lint_lang_layer`]), the read [`Memory`] (the
/// harvest corpus), and the two frozen brains. Returns an [`Outcome`] per proposed candidate (graduated
/// or not) so the caller can measure honestly. Only `Outcome::rule.is_some()` candidates are
/// module-ready.
///
/// The frozen loop's independence axis is DISTINCT harvested violating blocks; its English gate is the
/// two-doc-sentence reconciliation over a sibling foil. A candidate graduates iff ≥ [`REQUIRED_REPS`]
/// distinct real blocks fire AND the two doc sentences reconcile AND none contradicts (native/architecture.dx).
///
/// The final element is PASS 36's read-stage conservation rows — every named `(id, reason)`
/// withhold this pass's readers refused (grammar abstention, member veto, orphan fall-through,
/// unspelled reference subject), deduped, for the train to append to the module ledger. Read-stage
/// rows are OWNERSHIP-scoped by `owned` ([`crate::lint_docs::owned_urls`], the URLs this
/// language's own registered sources crawled): the pooled site corpus is shared by every language
/// of a host, so without the scope every read-stage refusal would land in EVERY language's ledger.
/// Learning and the partition stay whole-corpus — only ledger rows are scoped.
pub fn graduate(
    lang: &str,
    mut pages: Vec<(String, String)>,
    memory: &Memory,
    m: &MeaningNetwork,
    en: &English,
    constructions: &[crate::lint_construct::ConstructionState],
    owned: &std::collections::HashSet<String>,
) -> (
    Vec<Outcome>,
    Vec<ReadConstruct>,
    std::collections::HashMap<String, crate::lint_web::Corroboration>,
    std::collections::HashSet<String>,
    Vec<(String, String)>,
) {
    let bridge = Bridge::new(m, en);
    // The LEARNED deprecation attestation, keyed by the author's OWN METADATA TYPOGRAPHY (frontmatter
    // `status:` enum joined to the crawled pages by slug — COMPLETION PASS 13). Discovered from and applied
    // to the ORIGINAL (pre-chrome-strip) bodies: the attestation is a PAGE-ROLE fact of the whole page,
    // and the notecard's banner TEXT run is cross-page-identical, so the chrome filter (which removes
    // invariant text runs for clean PROSE) strips it away — reading the role after the strip would lose it
    // exactly as it would lose a class attribute. The attested URL SET is captured here and threaded down,
    // so every downstream reader keys on the URL, not the stripped body. Replaces the hardcoded
    // `has_deprecation_notecard` substring; MEASURED P=R=1.000 vs it (`examples/metajoin`).
    let attest = crate::lint_attest::Attestation::discover(&pages);
    // THE HONEST ATTESTED SET (PASS 35 — one fact, fixed at the source): a page is attested
    // deprecated iff its OWN banner text-run says so (page scope), or its OWN item anchors carry
    // the status typography (the rendered-marker route — the badge sits on the page's content,
    // not its navigation). A page that merely LINKS to deprecated things (MDN sidebar icons made
    // `div`'s page read as deprecated — the measured junk root) attests NOTHING; it falls to the
    // plain reference read. Every downstream consumer (partition, orphan arm, read flags, referee
    // widening, graded tiers) inherits this one corrected fact.
    let attested: std::collections::HashSet<String> = pages
        .iter()
        .filter(|(_, b)| {
            attest.attests_page_scope(b)
                || !crate::lint_lang_layer::attested_item_shapes(b).0.is_empty()
        })
        .map(|(u, _)| u.clone())
        .collect();
    // The PAGE-SCOPE subset — banner-run truth only (the element-shaped graded tier's referee).
    let page_scope: std::collections::HashSet<String> = pages
        .iter()
        .filter(|(_, b)| attest.attests_page_scope(b))
        .map(|(u, _)| u.clone())
        .collect();
    // THE RUNG-1 CONSUMER (PASS 23): the pages a PROVEN CONSTRUCTION binds on their own prose, each mapping
    // to the removal subject(s) the construction named — built over the RAW (pre-strip) bodies the miner
    // proved over, excluding pages the existing faculty already attests. Empty (⇒ inert) for every corpus
    // that proves no construction, so every other language stays byte-identical.
    let construction = construction_attestation(&pages, &attested, constructions);
    // CROSS-PAGE-INVARIANCE CHROME FILTER (native/laws.dx → "Cross-page invariance = chrome, discarded";
    // owner north-star). A site's navigation, breadcrumb, footer, and sidebar-menu text recurs
    // IDENTICALLY across its pages and carries zero governing meaning, so it is discarded — site-
    // scoped, learned from the corpus by exact text-run recurrence, no element name and no site name
    // named ([`crate::lint_graph::site_chrome`]). This removes the W3Schools `<div id="leftmenuinner">`
    // menu that a semantic-element chrome drop cannot catch, the measured blocker for reading W3S
    // prose. Applied before any page is read into prohibition prose or example code, so every reader
    // downstream — `read_doc_page`, `page_code_corpus`, the grammar partition — sees clean bodies.
    let chrome = crate::lint_graph::site_chrome(&pages);
    // Strip IN PLACE (crash lesson 2026-07-15): a second, stripped copy of a whole-site corpus
    // beside the original doubled a multi-gigabyte allocation; each page's raw body is dropped as
    // its stripped body replaces it.
    for (u, b) in &mut pages {
        *b = chrome.strip(u, b);
    }
    let mut page_store = pages;
    let pages: &[(String, String)] = &page_store;
    // PASS 36 — the recall census: every read-stage refusal below funnels into this one row set
    // (grammar abstention, member veto, orphan fall-through, unspelled reference subject), returned
    // for the train to append to the module's conservation ledger. Deduped by (id, reason).
    let (partition, mut withheld) = lang_pages(lang, pages, &bridge, en, &attested, &construction, owned);
    let (candidates, pool, read_surface) =
        propose(lang, &partition, &bridge, en, &attested, &page_scope, &construction, memory);
    // PASS 34 — the MEMBER-SHAPE law (measured: `/Web/API/SharedStorage/clear`, a genuinely
    // deprecated MEMBER page with no static examples, emitted BARE `clear` and the corpus harvest
    // self-witnessed on foreign receivers — `m.clear()` flagged in clean modern code). A page whose
    // URL parent AND grandparent are both pool pages IS a member page (owner/subject under a hub);
    // its subject must never enforce bare. With no example-derived qualified shape to fall back on,
    // the candidate ABSTAINS honestly. `/reference/`-marked URLs keep their own owner law
    // ([`crate::lint_lang_layer::member_page_shapes`]); dotted shapes are untouched.
    let pool_urls: std::collections::HashSet<&str> =
        pages.iter().map(|(u, _)| u.trim_end_matches('/')).collect();
    let body_of: std::collections::HashMap<&str, &str> =
        pages.iter().map(|(u, b)| (u.as_str(), b.as_str())).collect();
    let is_member_page = |url: &str| -> bool {
        if url.to_lowercase().contains("/reference/") {
            return false;
        }
        let t = url.trim_end_matches('/');
        let Some((parent, subject)) = t.rsplit_once('/') else { return false };
        let Some((grand, parent_seg)) = parent.rsplit_once('/').map(|(g, s)| (g, s)) else { return false };
        if !pool_urls.contains(parent) || !pool_urls.contains(grand) {
            return false;
        }
        // THE AUTHOR'S OWN TYPOGRAPHY decides member-hood (whole-site lesson: section INDEX pages
        // exist for everything, so parent+grandparent-in-pool alone read every interface page as a
        // member — `HMDVRDevice` lost its rule). A MEMBER page titles itself `Owner: subject` /
        // `Owner.subject`; an interface page titles itself alone.
        body_of.get(url).is_some_and(|b| {
            b.contains(&format!("{parent_seg}: {subject}")) || b.contains(&format!("{parent_seg}.{subject}"))
        })
    };
    // The veto is a PARTITION, not a filter (PASS 36): each vetoed candidate stands in the
    // conservation ledger under its rule id, never deleted silently.
    let (candidates, vetoed): (Vec<Candidate>, Vec<Candidate>) = candidates
        .into_iter()
        .partition(|c| c.construct.contains('.') || !is_member_page(&c.url));
    for c in &vetoed {
        note_withhold(
            &mut withheld,
            rule_id(&c.construct),
            "member veto (bare shape from member page; parent typography)",
        );
    }
    // The everything-read surface the funnel never proposed — retained as the web's unproven nodes.
    let read_surface = read_not_proposed(&candidates, read_surface);
    // PASS 34 — the REFERENCE READ: every partition-excluded page contributes its subject(s) too
    // (attested orphans as attested nodes; ordinary reference pages as plain reads). Appended AFTER
    // the funnel's reads so dedup and the proven view stay byte-identical.
    let read_surface = with_reference_read(
        pages,
        &partition,
        &bridge,
        en,
        &attested,
        &page_scope,
        &construction,
        owned,
        read_surface,
        &mut withheld,
    );
    let corpus = harvest_corpus(memory);

    // Each candidate's derived advice (its SECOND, distinct doc sentence). A candidate with no such
    // sentence has no un-fakeable English pair and cannot graduate.
    let advices: Vec<Option<String>> = candidates
        .iter()
        .map(|c| derive_advice(&pool, en, &c.construct, &c.understanding, &c.url))
        .collect();

    // PASS 30 referee-pool widening, MOVED ahead of the loop (crash lesson 2026-07-15, third strike):
    // this is the LAST reader of page bodies, so extracting the attested-orphan governing prose here
    // lets the whole-site body store be FREED before the long candidate loop below — the loop runs on
    // the harvest corpus and the sentence pools alone.
    let partition_urls: std::collections::HashSet<String> =
        partition.iter().map(|(u, _)| u.clone()).collect();
    let mut referee_pool: Vec<PooledSentence> = pool.clone();
    for (url, body) in
        pages.iter().filter(|(u, _)| attested.contains(u) && !partition_urls.contains(u.as_str()))
    {
        let p = crate::lint_lang_layer::read_doc_page(url, body, en, &bridge, &attested, &construction);
        for s in &p.governing {
            let negated = crate::lint_corroborate::is_negated(en, s);
            referee_pool.push(PooledSentence {
                sentence: s.clone(),
                prohibited: p.prohibited,
                url: url.clone(),
                negated,
            });
        }
    }
    // The LIVING-NAME inventory (the PASS-34 flood law's referee): every non-attested page's
    // URL-subject — a name documented as a LIVING construct anywhere in the corpus. URLs only.
    let living_names: std::collections::HashSet<String> = pages
        .iter()
        .filter(|(u, _)| !attested.contains(u))
        .filter_map(|(u, _)| u.trim_end_matches('/').rsplit('/').next().map(str::to_string))
        .collect();
    drop(partition);
    page_store.clear();
    page_store.shrink_to_fit();

    let mut outcomes = Vec::new();
    for (i, cand) in candidates.iter().enumerate() {
        // HARVEST: partition the corpus into violating (the plan fires) and clean (it does not). A
        // block whose TEXT does not contain the construct cannot possibly fire `uses_construct(C)`
        // (`scan_construct` matches an AST node whose text equals `C`), so it is clean WITHOUT a parse —
        // a sound pre-filter (no false negative) that turns the harvest from O(candidates × corpus
        // parses) into a parse only where the construct's text appears, keeping training in seconds.
        let plan = Plan::UsesConstruct { construct: cand.construct.clone() };
        let mut violating: Vec<String> = Vec::new();
        let mut clean: Vec<&str> = Vec::new();
        for block in &corpus {
            let fires = construct_in_text(block, &cand.construct) && !run_plan(&plan, lang, block).is_empty();
            if fires {
                violating.push(block.clone());
            } else {
                clean.push(block);
            }
        }
        // TOP UP with SELF-GENERATED violations when the idiomatic corpus is too scarce to reach the rep
        // floor (every CSS/HTML deprecation, JS `with`): splice the construct into varied real corpus
        // contexts, seeded by the page's own incorrect examples, frozen `run_plan` the only referee
        // (native/history.dx → the two walls → Fix 2). Harvested reps stay primary; generation only supplies count.
        // Skipped when the candidate has no derived advice — it cannot graduate anyway, so generating reps
        // for it is wasted `run_plan` sweeps (the training-time cut that keeps the workflow in seconds).
        if (advices[i].is_some() || cand.attested_deprecated) && violating.len() < REQUIRED_REPS {
            let contexts: Vec<&str> = clean.iter().take(GENERATE_CONTEXT_CAP).copied().collect();
            for g in generate_violations(lang, &cand.construct, &cand.seeds, &contexts) {
                if !violating.iter().any(|v| v == &g) {
                    violating.push(g);
                }
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
            (Some(advice), Some(foil)) => {
                let rule = RuleUnderTest::new(cand.understanding.clone(), foil.clone(), lang.to_string());
                // The self-test book is JUST THIS CANDIDATE's own rule (its plan + its derived advice) — the
                // honest per-candidate self-test (as `examples/js_graduate.rs`). A SHARED all-candidates book
                // cross-contaminates: a `var` sample that also contains `===`/`??`/`-0` fires a SIBLING rule
                // whose advice contradicts the `var` understanding → a spurious `Mismatch` that wrongly
                // blocked `==`/`eval` (MEASURED). Each candidate is proven on its own merits; the English
                // foil (a sibling's understanding) still supplies the un-fakeable comparison.
                let plan = Plan::UsesConstruct { construct: cand.construct.clone() };
                // BLIND-AGREEMENT sample set (owner point 3): the generator tags each self-generated block
                // with its expectation — the harvested/generated VIOLATIONS expect a flag, the harvested
                // CLEAN near-misses expect no flag. The blind lint (`blind_fires`, code only) then judges
                // agreement. Both kinds count toward the rep floor; the flag reps stay primary (capped at
                // PROVE_SAMPLE_CAP), the clean reps add the squeeze from the other side (CLEAN_SAMPLE_CAP).
                // Bounded so the sweep stays in the seconds budget; the true counts stay in the Outcome.
                let mut samples: Vec<Sample> = violating
                    .iter()
                    .take(PROVE_SAMPLE_CAP)
                    .map(|c| Sample { code: c.clone(), expect: Expect::Flag })
                    .collect();
                samples.extend(
                    clean
                        .iter()
                        .take(CLEAN_SAMPLE_CAP)
                        .map(|c| Sample { code: c.to_string(), expect: Expect::Clean }),
                );
                prove_blind(m, en, &rule, &plan, advice, &samples)
            }
            // Missing an un-fakeable advice or a genuine foil is an honest "cannot judge" — reported
            // as too-few-reps with the real firing count so the gap is visible, never a false pass.
            _ => Verdict::Unproven(crate::lint_selftest::Unproven::TooFewReps {
                corroborated: 0,
                required: REQUIRED_REPS,
                not_flagged: 0,
            }),
        };

        // NOTECARD GRADUATION PATH (native/history.dx → the notecard-as-proof route, owner ruling 2026-07-11).
        // When the origin page STRUCTURALLY ATTESTS its subject deprecated (a reference notecard) AND a
        // reference site publishes IDENTICAL deprecation boilerplate for every such construct, the English
        // self-test's foil is degenerate BY CONSTRUCTION — the frozen comparator honestly cannot apply
        // (`Contradicted`/undecidable on indistinguishable prose). The page's own notecard is a STATED
        // structural fact, not a predicted understanding, so it graduates the rule directly on the three
        // structural conditions the self-test would otherwise stand in for: (a) the notecard attests the
        // page's OWN subject deprecated (`attested_deprecated`); (b) `uses_construct(subject)` fires on
        // ≥ REQUIRED_REPS distinct own/generated violations AND stays clean on a real near-miss; (c) the
        // subject passed the URL-payload gate (already enforced at `propose`, so every candidate reaching
        // here satisfies it). This route is ONLY for structurally-attested deprecations; a rule page
        // (`attested_deprecated == false`) always takes the English self-test.
        let notecard_proven =
            cand.attested_deprecated && violating.len() >= REQUIRED_REPS && !clean.is_empty();

        // EMIT: a graduated candidate (English self-test PROVEN, or notecard-proven) whose subject its own
        // lead STATES ([`Candidate::stated`] — the `??`-vs-`==` discriminator), with a clean near-miss to
        // contrast against, becomes a module rule in the shape `RuleSet::build` compiles into a firing
        // detector (bad ∧ ¬good). The lead gate is enforced HERE, not at propose, so the pool (and every
        // other candidate's frozen verdict) is untouched — only the un-stated subject is withheld.
        // PASS 34 — the LIVING-NAME law (measured: MathML's genuinely-deprecated `href` proved bare
        // under the error-tolerant html grammar and flagged every `<a href>`). A BARE construct's
        // name is flood-unsafe iff the corpus ALSO documents the SAME name as a LIVING subject — a
        // NON-attested page whose URL-subject is this name (`HTMLAnchorElement/href` lives, so bare
        // `href` names TWO constructs and enforcing it flags the living one; `xlink:href`/
        // `mathcolor`/`marquee` exist only as deprecated subjects and stay enforceable). Probed on
        // the real corpus: href/version/clip/rel withheld, xlink:href/mathcolor/zoomAndPan/marquee/
        // center/big kept — every verdict truth-checked. No thresholds, no block scans: the corpus's
        // own page inventory is the referee. The withheld fact stays an attested web node.
        let name_collides = !cand.construct.contains('.')
            && !cand.construct.starts_with(':')
            && living_names.contains(&cand.construct);
        // PASS 35 — ELEMENT-SHAPE RESCUE for a colliding bare: if EVERY firing of the construct in
        // its own violating evidence is at ELEMENT POSITION (the name node's preceding source byte
        // is `<` — the author's own tag typography), the rule enforces as the `<x>` shape, which the
        // living name cannot collide with (`<frame>` fires on tags; `Window.frame` never does). The
        // exemplars are the referee — no element list, no language named.
        let emitted_construct = if name_collides && (matches!(verdict, Verdict::Proven) || notecard_proven) {
            let element_form = format!("<{}>", cand.construct);
            let element_plan = Plan::UsesConstruct { construct: element_form.clone() };
            let bare_plan = Plan::UsesConstruct { construct: cand.construct.clone() };
            let _ = &bare_plan;
            // The element-positioned witnesses must satisfy the SAME witness law the bare proof did —
            // the `<x>` rule re-proves on the subset of its own evidence that fires at tag position
            // (an attribute-text or identifier hit is not element evidence and simply drops out).
            let element_witnesses =
                violating.iter().filter(|b| !run_plan(&element_plan, lang, b).is_empty()).count();
            (element_witnesses >= REQUIRED_REPS.min(violating.len().max(1)))
                .then_some(element_form)
        } else {
            None
        };
        let name_collides = name_collides && emitted_construct.is_none();
        let rule = if cand.stated && !name_collides && (matches!(verdict, Verdict::Proven) || notecard_proven) {
            // The enforced shape: the candidate's own token, or its PASS-35 element form when the
            // living-name collision was disambiguated by the exemplars' own tag positions.
            let fire = emitted_construct.clone().unwrap_or_else(|| cand.construct.clone());
            let bad = violating.iter().min_by_key(|b| b.len()).map(|b| b.to_string());
            let good = clean.iter().min_by_key(|b| b.len()).map(|b| b.to_string());
            match (bad, good) {
                (Some(bad), good) => Some((
                    LearnedRule {
                        language: lang.to_string(),
                        id: rule_id(&fire),
                        severity: "medium".to_string(),
                        description: cand.understanding.clone(),
                        bad,
                        good: good.unwrap_or_default(),
                        // The rule IS its understood prohibition: carry the construct so the live
                        // build compiles `uses_construct(construct)` and fires the SAME plan the
                        // frozen loop proved — never a detector re-derived from the example diff.
                        construct: Some(fire),
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
    // PASS 30 — the SELF-REFEREE: the machine's own read judging every revoked-role construct against
    // every OTHER source's claim, over the widened pool extracted above (the referee hears the
    // language's WHOLE attested corpus, not only the rule-learning partition).
    // An ELEMENT-typography subject is refereed by its OWN documented typography (`<center>`,
    // the decoded `&lt;x&gt;` form other pages' prose carries), never the bare word — measured
    // (PASS 37 production closure, class 6): the bare token `center` matched "the view from a
    // center point" on an unrelated API page, and that common-word collision recorded a
    // contradiction that withheld the element's graded form.
    let targets: Vec<(String, String, Vec<String>)> = outcomes
        .iter()
        .filter(|o| o.candidate.attested_deprecated)
        .map(|o| {
            (
                o.candidate.construct.clone(),
                mention_needle(&o.candidate.construct, o.candidate.element_typography),
                vec![o.candidate.url.clone()],
            )
        })
        .chain(read_surface.iter().filter(|r| r.attested_deprecated).map(|r| {
            (
                r.construct.clone(),
                mention_needle(&r.construct, r.element_typography),
                vec![r.url.clone()],
            )
        }))
        .collect();
    let referee = self_referee(&referee_pool, &targets);
    (outcomes, read_surface, referee, living_names, withheld)
}

/// The largest number of corroborating sources / contradiction records the referee persists per node —
/// the payload stays a summary, never a transcript.
const MAX_REFEREE_RECORDS: usize = 8;

/// The persisted head of a contradiction sentence — enough to read the disagreement, never a page dump.
const MAX_CONTRADICTION_HEAD: usize = 160;

/// PASS 30 — the SELF-REFEREE over the corpus's own sentence pool: for each revoked-role `(construct,
/// own-source urls)` target, judge every OTHER source's governing sentence that names the construct
/// (bounded full token) with [`crate::lint_corroborate::revocation_claim`]. An ASSERTING sentence from a
/// distinct source corroborates (coherent url, deduped); a DENYING sentence is a CONTRADICTION record —
/// first-class signal that one side is wrong. Neutral mentions carry nothing. Pure over the pool;
/// returns only non-empty records (a construct nobody else speaks about is the honest sparse state).
fn self_referee(
    pool: &[PooledSentence],
    targets: &[(String, String, Vec<String>)],
) -> std::collections::HashMap<String, crate::lint_web::Corroboration> {
    let mut anchors = crate::lint_attest::prohibition_class_tokens();
    anchors.extend(crate::lint_attest::removal_class_tokens());
    let mut out = std::collections::HashMap::new();
    if anchors.is_empty() || targets.is_empty() {
        return out;
    }
    let lowered: Vec<(String, &PooledSentence)> =
        pool.iter().map(|p| (p.sentence.to_lowercase(), p)).collect();
    for (construct, mention, own) in targets {
        let needle = mention.to_lowercase();
        let mut rec = crate::lint_web::Corroboration::default();
        for (sentence, p) in &lowered {
            if own.contains(&p.url) || !mentions_full_token(sentence, &needle) {
                continue;
            }
            match crate::lint_corroborate::revocation_claim(sentence, p.negated, &anchors) {
                crate::lint_corroborate::RevocationClaim::Asserts => {
                    if rec.coherent.len() < MAX_REFEREE_RECORDS && !rec.coherent.contains(&p.url) {
                        rec.coherent.push(p.url.clone());
                    }
                }
                crate::lint_corroborate::RevocationClaim::Denies => {
                    if rec.contradictions.len() < MAX_REFEREE_RECORDS {
                        let head: String = p.sentence.chars().take(MAX_CONTRADICTION_HEAD).collect();
                        rec.contradictions.push(crate::lint_web::Contradiction {
                            source: p.url.clone(),
                            sentence: head,
                        });
                    }
                }
                crate::lint_corroborate::RevocationClaim::Neutral => {}
            }
        }
        if !rec.coherent.is_empty() || !rec.contradictions.is_empty() {
            out.insert(construct.clone(), rec);
        }
    }
    out
}

/// The MENTION form other prose addresses a construct by — its own documented typography (PASS 37
/// production closure, class 6): an element-typography subject is written `<x>` (the decoded
/// `&lt;x&gt;` every reference carries), everything else by its plain construct token. The needle
/// the self-referee matches; never a judgement, only the subject's own spelling.
fn mention_needle(construct: &str, element_typography: bool) -> String {
    if element_typography {
        format!("<{construct}>")
    } else {
        construct.to_string()
    }
}

/// Bounded FULL-TOKEN mention: `construct` appears in `sentence` delimited by non-identifier characters,
/// with dots part of the token — so `ssl.SSLSocket.read` never matches inside a longer chain and a bare
/// `read` never rides every sentence containing the word. Both sides lower-case by contract.
fn mentions_full_token(sentence: &str, construct: &str) -> bool {
    let is_tok = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.';
    let mut from = 0usize;
    while let Some(rel) = sentence[from..].find(construct) {
        let s = from + rel;
        let e = s + construct.len();
        let before = sentence[..s].chars().next_back().map(|c| !is_tok(c)).unwrap_or(true);
        let after = sentence[e..].chars().next().map(|c| !is_tok(c)).unwrap_or(true);
        if before && after {
            return true;
        }
        from = e;
    }
    false
}

/// Whether `lang`'s documentation READ PASS has completed (owner correction 2026-07-12, point 2): the
/// crawl produced a page corpus — `pages` (the read pass's own persisted raw-page output) is non-empty.
/// A cold cache (no pages) is an INCOMPLETE read, and a language is never graduated from a half-read
/// crawl. This is a STRUCTURAL precondition (the read output exists), not a heuristic over the prose.
fn read_pass_complete(pages: &[(String, String)]) -> bool {
    !pages.is_empty()
}

/// The doc-roles that attest a REVOKED construct (deprecated / removed / prohibited) — the ONLY nodes
/// eligible for the graded tier (PASS 27). A read node with none of these is knowledge, never a finding.
const REVOKED_ROLES: [&str; 3] = ["deprecated", "removal", "prohibition"];

/// How many example blocks the graded gates read from the corpus — a RUNAWAY bound only, sized far above
/// any real docs corpus so it never truncates one. MEASURED: capping at [`MAX_HARVEST_BLOCKS`] (4000)
/// truncated python's corpus inside its first NINE pages (reference pages carry hundreds of blocks each),
/// which STARVED the usage-death gate — `.read`/`.write`/`.Sequence` all read "dead" on a 9-page corpus
/// and would flood real code. Usage-death is meaningful only over the WHOLE corpus; the text pre-filter
/// ([`construct_in_text`]) keeps the sweep cheap, and `run_plan` parses only blocks that could fire.
const GRADED_CORPUS_CAP: usize = 1 << 20;

/// Whether `construct` is QUALIFIED-SAFE to fire a graded finding on — the PASS-26 cut, reused verbatim: a
/// real `owner.member` (or deeper) where every dotted part is an identifier (≥2 chars, alnum/underscore,
/// alphabetic/underscore start). REJECTS a bare leading-dot member (`.read`), a doc URL basename
/// (`ssl.html`, `struct.Vec.html`), a rustdoc anchor form (`method.x`/`struct.X`), and a single bare
/// generic token — the junk classes a naive graded flood would fire on across every clean modern file.
fn qualified_safe(c: &str) -> bool {
    if c.starts_with('.') {
        return false;
    }
    if [".html", ".htm", ".php"].iter().any(|e| c.ends_with(e)) {
        return false;
    }
    let anchor = [
        "method.", "struct.", "primitive.", "trait.", "enum.", "fn.", "macro.", "constant.", "type.",
        "mod.", "keyword.", "associatedtype.", "union.", "associatedconstant.",
    ];
    let low = c.to_lowercase();
    if anchor.iter().any(|p| low.starts_with(p)) {
        return false;
    }
    let parts: Vec<&str> = c.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    parts.iter().all(|p| {
        p.len() >= 2
            && p.chars().next().is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
            && p.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    })
}

/// COMPUTE the graded (LOW-severity) firing forms for the revoked-role READ nodes (PASS 27) — the
/// evidence-graded tier that replaces abstention. For each UNPROVEN construct the reader retained
/// (`read_surface`) that carries a REVOKED doc-role (`roles`/`attested_deprecated`) and is QUALIFIED-SAFE,
/// pick a flood-safe firing form and KEEP it only if it stays clean on the corpus's own other-page code:
///
///   1. member = the construct's terminal dotted segment; the receiver-generic form is `.member`.
///   2. USAGE-DEATH (member scope): count OTHER-page example blocks (`code_by_url`, excluding the node's own
///      source page) whose text carries `.member`. The member is DEAD iff that count is 0 — the PASS-26
///      usage-death signal, which failed as a family discriminator but SUCCEEDS as a member-scope safety
///      gate (`.blink` 0/N fires; `.bold`, alive across the corpus, abstains from the receiver-generic form).
///   3. DEATH-VERDICT CALIBRATION (comparative, from the language's own candidate distribution): the death
///      verdicts are TRUSTED only when the same measurement finds at least one ALIVE member among this
///      language's own qualified candidates. A corpus that never witnesses life cannot certify death —
///      MEASURED: python's 9-page corpus read ALL 92 candidates "dead" (including `.read`/`.write`, which
///      flood every real file object) while javascript's 3052-page corpus split 10 dead / 11 alive. A
///      degenerate all-dead distribution is corpus POVERTY, not universal death: every form falls to the
///      dotted-literal tier. No constant — the corpus's own distribution is the referee.
///   4. FORM: trusted-dead ⇒ fire the receiver-generic `.member` (high recall, real `x.member` usage);
///      alive or uncalibrated ⇒ the dotted-literal `owner.member` (safe, low recall — fires only on the
///      exact deprecated static text, so a LIVE remedy like `collections.abc.Sequence` is never flagged
///      for `typing.Sequence`).
///   5. PROVEN-COVERAGE DEDUP: a form a PROVEN rule already covers is SKIPPED — the graded tier only adds
///      NEW enforcement. Covered = the proven constructs contain the fire token, the construct itself, or
///      the receiver-generic `.member` (whose member scan already fires every `X.member`, dotted included) —
///      MEASURED: javascript's proven set already carries `.blink`/`.getYear`/`.compile`, so their graded
///      duplicates would double-report every kitchen-sink line.
///   6. CLEAN-NEAR-MISS (graduation-lite): run the chosen plan over the other-page blocks that textually
///      could fire it; if it fires on ANY, the form is not flood-safe → DROP (abstain). The attestation is
///      the evidence a violation-witness search would otherwise supply; this check is the flood guard.
///
/// Returns `construct → GradedForm`. Pure over the read surface + corpus + the frozen `run_plan`; empty for
/// a language whose revoked read nodes all fail the gates (the honest number — ship only what passes).
fn graded_forms(
    lang: &str,
    read_surface: &[ReadConstruct],
    outcomes: &[Outcome],
    roles: &std::collections::HashMap<String, String>,
    proven_constructs: &std::collections::HashSet<String>,
    code_by_url: &[(String, String)],
) -> std::collections::HashMap<String, crate::lint_web::GradedForm> {
    let corpus_pages = code_by_url.iter().map(|(u, _)| u).collect::<std::collections::HashSet<_>>().len();
    // Phase 1 — the qualified revoked candidates with their member-scope death verdicts.
    struct Candidate<'a> {
        read: &'a ReadConstruct,
        member_form: String,
        dead: bool,
    }
    let mut candidates: Vec<Candidate> = Vec::new();
    for r in read_surface {
        // REVOKED gate: the node's construction kind is a revocation, or the page attests deprecation.
        let role_specific = roles.get(&r.construct).map(String::as_str);
        let revoked = r.attested_deprecated || role_specific.is_some_and(|k| REVOKED_ROLES.contains(&k));
        if !revoked || !qualified_safe(&r.construct) {
            continue;
        }
        let member = r.construct.rsplit('.').next().unwrap_or(&r.construct);
        let member_form = format!(".{member}");
        // USAGE-DEATH at member scope, EXCLUDING the construct's own source page (the deprecation's own
        // illustration must not count as living usage — the PASS-26 own-page exclusion).
        let dead = !code_by_url.iter().any(|(u, blk)| u != &r.url && construct_in_text(blk, &member_form));
        candidates.push(Candidate { read: r, member_form, dead });
    }
    // Phase 2 — calibration: death is trusted only when this corpus proved it can witness life.
    let corpus_discriminates = candidates.iter().any(|c| !c.dead);
    // Phase 3 — form selection + proven-coverage dedup + clean-near-miss.
    let mut out = std::collections::HashMap::new();
    for c in candidates {
        let r = c.read;
        let trusted_dead = c.dead && corpus_discriminates;
        let fire = if trusted_dead { c.member_form.clone() } else { r.construct.clone() };
        if proven_constructs.contains(&fire)
            || proven_constructs.contains(&r.construct)
            || proven_constructs.contains(&c.member_form)
        {
            continue; // a proven rule already enforces this shape — the graded tier adds only NEW coverage
        }
        let plan = Plan::UsesConstruct { construct: fire.clone() };
        // LANGUAGE OWNERSHIP (the grammar as referee, qualified tier): the chosen form must FIRE
        // under THIS language on the construct's OWN page examples — measured, the union pool
        // minted dotted forms (`rel.prerender`, `gamepad.displayId`) into modules whose grammars
        // can never fire them: planted-but-silent junk. SCOPE (measured, second strike): the gate
        // judges on `<pre>` blocks, but an ITEM-ROUTE page (its own anchors carry the status —
        // python's rendered markers) demonstrates its items in INLINE code the block corpus never
        // holds, and its item anchors already lock the language — the gate blanked python's entire
        // graded tier (80→0). Item-route pages are exempt; banner-page mints stay gated.
        let item_route_page = r.attested_deprecated && !r.page_scope;
        if !item_route_page {
            let own_fires = code_by_url
                .iter()
                .filter(|(u, _)| u == &r.url)
                .any(|(_, blk)| !run_plan(&plan, lang, blk).is_empty());
            if !own_fires {
                continue;
            }
        }
        // CLEAN-NEAR-MISS: the chosen form must fire on NONE of the corpus's other-page blocks.
        let flags_clean = code_by_url
            .iter()
            .filter(|(u, blk)| u != &r.url && construct_in_text(blk, &fire))
            .any(|(_, blk)| !run_plan(&plan, lang, blk).is_empty());
        if flags_clean {
            continue;
        }
        // The evidence message — a prohibition (so the build's entry gate reads it as one), citing the
        // attested deprecation and the usage-death basis the RUNG requires.
        let role_specific = roles.get(&r.construct).map(String::as_str);
        let role = role_specific.filter(|k| *k != "deprecated").unwrap_or("deprecated");
        let basis = if trusted_dead {
            format!("its member `{}` is absent from the corpus's {corpus_pages} current example pages (usage-dead)", c.member_form)
        } else {
            format!("fires only on the exact deprecated `{}` form", r.construct)
        };
        let description = format!(
            "Do not use `{}`: documented {role} ⟨{}⟩ — {basis}.",
            r.construct, r.url
        );
        out.insert(
            r.construct.clone(),
            crate::lint_web::GradedForm { fire, severity: "low".to_string(), description, source: r.url.clone() },
        );
    }
    // PASS 35 — ELEMENT-SHAPED graded forms (owner: "if it knows it, true knowledge, it will have
    // things to enforce"). A BARE revoked subject the qualified tier cannot take (`font`, `dir` — no
    // owner, no examples on its own page) enforces as the `<x>` element shape, which is
    // identification-proof BY CONSTRUCTION: a tag named `font` IS the font element, by the grammar
    // itself. Candidates come from BOTH surfaces: the read surface (never-proposed subjects) AND the
    // failed outcomes (proposed candidates the sentence wall kept from graduating — the measured
    // `center` blind spot). Gates, all measured: PAGE-SCOPE banner truth (item badges and sidebar
    // icons admitted `CSSNumericValue/div`); the page's own `&lt;name&gt;` element typography (an
    // attribute subject never carries it); the URL-SUBJECT law (co-reads minted `<div>`); and THE
    // GRAMMAR AS REFEREE (the minimal demo must fire under THIS language — the union pool otherwise
    // minted `<font>` into the css and javascript modules as planted-but-silent junk).
    let element_candidates: Vec<(&String, &String)> = read_surface
        .iter()
        .filter(|r| r.page_scope && r.element_typography)
        .map(|r| (&r.construct, &r.url))
        .chain(
            outcomes
                .iter()
                .filter(|o| o.rule.is_none() && o.candidate.page_scope && o.candidate.element_typography)
                .map(|o| (&o.candidate.construct, &o.candidate.url)),
        )
        .collect();
    for (construct, url) in element_candidates {
        if out.contains_key(construct)
            || construct.contains('.')
            || construct.starts_with(':')
            || construct.starts_with('<')
            || construct.len() < 2
            || !url_payload_equals(url, construct)
        {
            continue;
        }
        let fire = format!("<{construct}>");
        if proven_constructs.contains(&fire) || proven_constructs.contains(construct) {
            continue;
        }
        let demo = format!("<{construct}>x</{construct}>");
        if run_plan(&Plan::UsesConstruct { construct: fire.clone() }, lang, &demo).is_empty() {
            continue;
        }
        let role_specific = roles.get(construct).map(String::as_str);
        let role = role_specific.filter(|k| *k != "deprecated").unwrap_or("deprecated");
        let description = format!(
            "Do not use `<{construct}>`: documented {role} ⟨{url}⟩ — fires only at tag position, the element itself."
        );
        out.insert(
            construct.clone(),
            crate::lint_web::GradedForm { fire, severity: "low".to_string(), description, source: url.clone() },
        );
    }
    out
}

/// PASS 37 — THE ATTESTATION READ (native/history.dx, "COMPLETION PASS 37" implementation subsection):
/// the three documentation shapes the attribute-dimension audit proved the reader had never read,
/// each an attestation SOURCE the page itself publishes — the per-attribute Deprecated badge in an
/// element page's definition list (law 1, construct `host@attr`), the index-section obsolete list,
/// and the "Not Supported" page-role notice (law 2, element constructs). Runs over the
/// PRE-chrome-strip pages (the banner-attester precedent: a recurring notice run is exactly what
/// the chrome filter strips), scoped to pages this language's OWN sources crawled (`owned`) that
/// do not POSITIVELY attribute to another registered language by URL segment (the PASS-36
/// attribution doctrine). Returns, in one pass: the read-surface KNOWLEDGE nodes (every
/// attestation lands in the web unconditionally — the two-axis law), the graded-LOW enforcement
/// forms whose demonstration the grammar (or, grammarless, the ONE containment matcher) fires,
/// and a NAMED ledger row for every attestation whose demonstration failed — nothing silent.
fn pass37_attestations(
    lang: &str,
    pages: &[(String, String)],
    owned: &std::collections::HashSet<String>,
    extra_langs: &std::collections::HashSet<String>,
    en: &English,
    m: &MeaningNetwork,
) -> (Vec<ReadConstruct>, Vec<(String, crate::lint_web::GradedForm)>, Vec<(String, String)>) {
    let lang_lc = lang.to_lowercase();
    let has_grammar = crate::lint_match::language(lang).is_some();
    // The demonstration gate: the grammar as referee where one exists (the PASS-35 element-arm
    // law — the minimal demo must fire under THIS language, which keeps `<font>` out of css/js);
    // a grammarless language validates the SAME demo through the one containment matcher.
    let demo_fires = |fire: &str, demo: &str| -> bool {
        if has_grammar {
            !run_plan(&Plan::UsesConstruct { construct: fire.to_string() }, lang, demo).is_empty()
        } else {
            crate::lint_match::containment_fires(demo, &crate::lint_match::construct_tokens(fire))
        }
    };
    let mut reads: Vec<ReadConstruct> = Vec::new();
    let mut graded: Vec<(String, crate::lint_web::GradedForm)> = Vec::new();
    let mut withheld: Vec<(String, String)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    // The PASS-36 attribution doctrine, literally: skip only a page that POSITIVELY
    // attributes to ANOTHER REGISTERED LANGUAGE by URL segment (`/Web/JavaScript/…`,
    // `/css/…`). The general attributor's extension-claim guesses are NOT positive
    // attribution — MEASURED (the v105 closure trace): `hint_language` read `/tags/` as
    // cpp (the ctags claim) and `/Web/API/` as svg, silently unscoping the w3schools tag
    // pages (`applet`/`basefont`) and every interface page from the attestation read.
    let foreign = |url: &str| -> bool {
        url.split("://")
            .nth(1)
            .unwrap_or(url)
            .split('/')
            .skip(1)
            .map(|s| s.split(['?', '#', '.']).next().unwrap_or(""))
            .filter(|s| s.chars().count() >= 2)
            .any(|s| {
                let t = s.to_lowercase();
                t != lang_lc && extra_langs.contains(&t)
            })
    };
    // The HOST(s) of a page (the element-hood proof): an ELEMENT page proves its URL-subject
    // in its own `&lt;host&gt;` element typography (PASS 35 A3; single-letter elements — `a` —
    // are elements too); an INTERFACE page proves its hosts by its OWN documented mapping (the
    // interface-host law, class 2: the mapping sentence's `<x>` typography in the URL
    // segment's exact-case interface spelling, else the references link arm), first-written
    // host first.
    let page_hosts = |url: &str, body: &str| -> (String, Vec<String>, bool, bool) {
        let segment = url
            .split(['?', '#'])
            .next()
            .unwrap_or(url)
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("");
        let subject = segment.to_lowercase();
        let subject_ok = !subject.is_empty() && subject.chars().all(|c| c.is_ascii_alphanumeric());
        if subject_ok && body.contains(&format!("&lt;{subject}&gt;")) {
            return (subject.clone(), vec![subject], false, false);
        }
        let (hosts, enumeration) = if subject_ok {
            crate::lint_html_layer::interface_hosts(segment, body)
        } else {
            (Vec::new(), false)
        };
        (subject, hosts, enumeration, true)
    };
    // THE PRIMARY-CLAIM LAW (v105 closure, measured on HTMLParamElement): a mapping sentence
    // may write a SECOND element as context ("…`<param>` …, acting as a parameter for an
    // `<object>` element") — a SECONDARY mapping-sentence claim yields to any page whose
    // mapping writes that element FIRST (HTMLObjectElement's own primary claim), so
    // `object@type` junk never mints from the param page while `<th>`/`<td>` (no competing
    // primary) both stay hosts of the table-cell interface. A references-arm host list is an
    // ENUMERATION — every entry an equal claim (HTMLMediaElement's `<video>` and `<audio>`),
    // never yielded. Competing claims are those of OTHER INTERFACE pages only — an element
    // page naming its own subject is not a competing mapping (measured: `Elements/td`'s
    // self-claim silenced the table-cell interface's `<td>` and lost `td@nowrap`).
    // Corpus-corroborated — the pages' own claims referee each other.
    // …and a competing claim exists only where the claimant page ITSELF documents attribute
    // badges (a real mapping page): a per-property page (`…/HTMLTableCellElement/noWrap`)
    // writes the same typography in its prose but defines nothing — MEASURED, its incidental
    // first-`<td>` claim silenced the table-cell interface's `<td>` and lost `td@nowrap`.
    let primary_claims: std::collections::HashMap<String, String> = pages
        .iter()
        .filter(|(url, _)| owned.contains(url) && !foreign(url))
        .filter_map(|(url, body)| {
            let (subject, hosts, _, interface) = page_hosts(url, body);
            let claims = interface
                && !hosts.is_empty()
                && !crate::lint_html_layer::attribute_badges(&subject, body).is_empty();
            claims.then(|| hosts.first().map(|h| (h.clone(), url.clone()))).flatten()
        })
        .fold(std::collections::HashMap::new(), |mut m, (h, url)| {
            m.entry(h).or_insert(url);
            m
        });
    for (url, body) in pages {
        if !owned.contains(url) || foreign(url) {
            continue;
        }
        // LAW 1 — attribute badges, admitted only under the element-hood proof.
        let (subject, hosts, enumeration, _interface) = page_hosts(url, body);
        let hosts: Vec<String> = hosts
            .iter()
            .enumerate()
            .filter(|(i, h)| {
                enumeration
                    || *i == 0
                    || primary_claims.get(*h).is_none_or(|primary| primary == url)
            })
            .map(|(_, h)| h.clone())
            .collect();
        if !hosts.is_empty() {
            for (attr, governing) in crate::lint_html_layer::attribute_badges(&subject, body) {
                for host in &hosts {
                    let construct = format!("{host}@{attr}");
                    if !seen.insert(construct.clone()) {
                        continue;
                    }
                    reads.push(ReadConstruct {
                        construct: construct.clone(),
                        governing: governing.clone(),
                        url: url.clone(),
                        attested_deprecated: true,
                        page_scope: false, // the badge is ITEM scope, never a page banner
                        element_typography: false,
                    });
                    let demo = format!("<{host} {attr}=\"x\">");
                    if demo_fires(&construct, &demo) {
                        graded.push((
                            construct.clone(),
                            crate::lint_web::GradedForm {
                                fire: construct.clone(),
                                severity: "low".to_string(),
                                description: format!(
                                    "Do not use the `{attr}` attribute on `<{host}>`: {governing} ⟨{url}⟩ — fires only inside its own host's tag open."
                                ),
                                source: url.clone(),
                            },
                        ));
                    } else {
                        note_withhold(
                            &mut withheld,
                            rule_id(&construct),
                            "attest gate (demonstration does not fire under this language)",
                        );
                    }
                }
            }
        }
        // LAW 2 — index-section entries + the "Not Supported" page-role notice: element
        // attestations enforcing as the PASS-35 `<x>` tag-position shape.
        let mut elements = crate::lint_html_layer::obsolete_index_entries(body, en, m);
        if let Some(one) = crate::lint_html_layer::not_supported_subject(url, body, en) {
            if !elements.iter().any(|(e, _)| e == &one.0) {
                elements.push(one);
            }
        }
        for (element, governing) in elements {
            if !seen.insert(element.clone()) {
                continue;
            }
            reads.push(ReadConstruct {
                construct: element.clone(),
                governing: governing.clone(),
                url: url.clone(),
                attested_deprecated: true,
                page_scope: false, // section/notice truth, not a page banner run
                element_typography: true,
            });
            let fire = format!("<{element}>");
            let demo = format!("<{element}>x</{element}>");
            if demo_fires(&fire, &demo) {
                graded.push((
                    element.clone(),
                    crate::lint_web::GradedForm {
                        fire,
                        severity: "low".to_string(),
                        description: format!(
                            "Do not use `<{element}>`: {governing} ⟨{url}⟩ — fires only at tag position, the element itself."
                        ),
                        source: url.clone(),
                    },
                ));
            } else {
                note_withhold(
                    &mut withheld,
                    rule_id(&element),
                    "attest gate (demonstration does not fire under this language)",
                );
            }
        }
    }
    (reads, graded, withheld)
}

/// The result of a graduation pass: the PROVEN construct rules AND the exact corpus the pass read.
/// `corpus_urls` is the re-check basis for contradiction-driven reshape (native/history.dx → Item 3c): the set
/// of page URLs this pass proposed over, so the caller can tell a rule whose source page is STILL in the
/// corpus (its absence from `rules` is a genuine failure to re-prove — a contradiction) from one whose
/// page has LEFT the corpus (a subset crawl — retain the last proof, unrefreshed).
pub struct GraduatedModule {
    /// Every rule the pass PROVED this crawl, as `(rule, source url)`.
    pub rules: Vec<(LearnedRule, String)>,
    /// The URLs of every page the pass read (raw pages ∪ whole-site corpus) — the re-check basis.
    pub corpus_urls: std::collections::HashSet<String>,
    /// The PROVEN CONSTRUCTION states this pass mined + proved over the corpus (PASS 22): sentence-scale
    /// invariant scaffolds proven under the frozen ISM law against the machine's own proven-deprecated
    /// subject set ([`crate::lint_construct`]). Persisted retain-and-grow beside the graduated ledger.
    pub constructions: Vec<crate::lint_construct::ConstructionState>,
    /// The GRADED (LOW-severity) rules this pass derived from the web's revoked-role read nodes (PASS 27) —
    /// the evidence-graded tier, as `(rule, source)`. A SEPARATE tier from `rules`: the caller appends them
    /// AFTER the proven set (never through the contradiction re-check), so the proven order stays identical.
    pub graded: Vec<(LearnedRule, String)>,
    /// PASS 36 — the read-stage conservation rows this pass refused, as named `(id, reason)`
    /// withholds (grammar abstention, member veto, orphan fall-through, unspelled reference
    /// subject), deduped. The train appends them to the compiled module's one ledger
    /// ([`crate::lint_match::RuleSet::withheld`]) before saving, so `lint_query kind=rules`
    /// surfaces every stage's refusals — nothing read vanishes silently.
    pub withheld: Vec<(String, String)>,
}

/// The LIVE entry the module build calls (the covenant-clean successor to
/// [`crate::lint_docs::rules_from_memory`] for MODULES): graduate `lang`'s construct rules from the
/// read `memory` through the frozen loop, returning every PROVEN rule AND the corpus it read (the
/// [`GraduatedModule`] contract — the corpus is Item 3c's re-check basis). Loads the two frozen brains;
/// an empty module when either is unavailable (the loop is defined only over the real bedrock — never
/// fake a rule). Never trains, never touches the network.
pub fn graduated_rules(lang: &str, memory: &Memory) -> GraduatedModule {
    let empty = || GraduatedModule {
        rules: Vec::new(),
        corpus_urls: std::collections::HashSet::new(),
        constructions: Vec::new(),
        graded: Vec::new(),
        withheld: Vec::new(),
    };
    let (Some(br), Some(en)) = (crate::lint_char::brain(), crate::lint_english::brain()) else {
        return empty();
    };
    let data_root = crate::tools::lint::data_root_pub();
    // WHOLE-SITE PROPOSE (owner directive 2026-07-12): the candidate source is the ENTIRE registered-site
    // corpus, read with NO section/language filter ([`crate::lint_docs::site_corpus`]), UNIONED with this
    // language's own read pages (the Memory basis). Every page of the site is proposed to every language;
    // the language partition is decided downstream by GRAMMAR VERIFICATION ([`lang_pages`]), never by URL.
    // A page whose subject fires in no grammar abstains — this is how a cross-section page (MDN Web-API
    // `document.write`) reaches JavaScript while a CSS property never leaks into it.
    // ONE resident copy (crash lesson 2026-07-15, second strike): `raw_pages` and `site_corpus`
    // decode the SAME host-matched crawl caches — the language's own sources are host-matched by
    // construction — so the old raw_pages ∪ site_corpus merge held the whole-site corpus TWICE
    // while deduping. `site_corpus` alone is the superset, already URL-deduped.
    let pages = crate::lint_docs::site_corpus(&data_root, lang);
    // FULL-DOCS-READ PRECONDITION (owner correction 2026-07-12, point 2): a language is TESTED only
    // after its registered documentation has been READ at least once — a STRUCTURAL precondition, the
    // crawl's read pass having produced a page corpus (`raw_pages` non-empty; the pages are the read
    // pass's own persisted output). No raw pages ⇒ the read pass has not completed for this language ⇒
    // it is NOT graduated (it stays on the miner / abstains), never tested from a half-read crawl.
    if !read_pass_complete(&pages) {
        return empty();
    }
    // The re-check basis (Item 3c): every page URL this pass proposes over. Captured BEFORE the
    // corpus-enrichment fallback (which only pushes reference code blocks, never new pages).
    let corpus_urls: std::collections::HashSet<String> = pages.iter().map(|(u, _)| u.clone()).collect();
    // OFFLINE-ROBUSTNESS FALLBACK (native/history.dx → "the recommended unlock"). The frozen loop's evidence is
    // the harvested code corpus. On a machine whose read `Memory` is sparse — a legacy no-memory catalog,
    // or a source that could not refresh with bindings — that corpus is empty and NOTHING graduates even
    // though the crawl cache holds the very example code the harness graduates from. When the memory-borne
    // corpus is too thin to reach the rep floor, reconstruct it from the raw pages' OWN `<pre><code>`
    // interiors (exactly `examples/web_module_train`'s reconstruction, now the shipped path), so the flip
    // engages from a valid crawl cache alone. A rich read `Memory` (css here) is left untouched — the
    // fallback only supplies the corpus the machine is missing, never overrides real bindings.
    let enriched;
    let memory = if harvest_corpus(memory).len() < REQUIRED_REPS {
        let mut m = memory.clone();
        for block in crate::lint_lang_layer::page_code_corpus(&pages, lang, MAX_HARVEST_BLOCKS) {
            m.reference.push(block);
        }
        enriched = m;
        &enriched
    } else {
        memory
    };
    // PASS 22/23 — mine + PROVE the construction states over this language's own corpus ONCE, persist them
    // retain-and-grow beside the graduated ledger, AND feed them to `graduate` as the rung-1 CONSUMER's
    // basis. Today this proves ONE state for python (`the last version of python … module was`, 24 removed
    // modules); every other language proves none, so `constructions` is empty and the consumer is inert —
    // those modules stay byte-identical. For python the consumer newly graduates the removal-only subjects.
    let constructions = crate::lint_construct::mine_and_prove(&pages);
    crate::lint_construct::persist(lang, &constructions);
    // PASS 24 — THE LANGUAGE WEB. The graduation pass reads a construct⊗prose⊗meaning⊗source⊗attestation
    // web; persist it as the language's subgraph and DERIVE the rules from it. Every candidate is a READ
    // node (retained, queryable); every emitted rule is a PROVEN node carrying that exact rule as its
    // compiled view. `derive_rules` projects the proven nodes in outcome order, so the live rule set is a
    // VIEW over the web — byte-identical to the old `filter_map(|o| o.rule)` (the proven nodes carry those
    // same `(rule, url)` pairs). Delete the web and re-read it: re-deriving reproduces the same rules.
    // The doc-role each construction-consumed subject carries (PASS 25 rung 2) — the proven construction's
    // KIND ("removal"/"prohibition"), keyed by subject. Empty for every language that proves no
    // construction, so those webs carry only the author-metadata "deprecated" role.
    let roles = crate::lint_construct::subject_roles(&constructions, &pages);
    // PASS 27 — THE GRADED TIER. The web's revoked-role READ nodes (attested-deprecated, never proven) get a
    // train-time-computed flood-safe firing form ([`graded_forms`]): the corpus's OWN example code (with
    // page provenance so a construct's own illustration is excluded) supplies the member-scope usage-death
    // and clean-near-miss gates. Persisted ON the node; derived to LOW rules AFTER the proven view.
    // Both computed BEFORE graduation over the same RAW bodies as always — graduation then CONSUMES the
    // page vector (crash lesson: the whole-site corpus must never be resident twice).
    let code_by_url = crate::lint_lang_layer::page_code_blocks_by_url(&pages, GRADED_CORPUS_CAP);
    // The ledger-ownership scope (PASS 36): read-stage withhold rows are recorded only for pages
    // this language's OWN sources crawled — the pooled corpus above stays the whole-site propose.
    let owned = crate::lint_docs::owned_urls(&data_root, lang);
    // PASS 37 — the attestation read, over the PRE-chrome-strip pages (graduate strips in
    // place, so this must precede it): attribute badges, index-section obsolescence, and the
    // "Not Supported" notice, as knowledge + graded forms + named withhold rows.
    let extra_langs: std::collections::HashSet<String> = {
        let mut s: std::collections::HashSet<String> =
            crate::lint_train::registered_languages(&data_root).into_iter().collect();
        s.insert(lang.to_lowercase());
        s
    };
    let (p37_reads, p37_graded, p37_withheld) =
        pass37_attestations(lang, &pages, &owned, &extra_langs, en, br.meanings());
    let (outcomes, mut read_surface, referee, living_names, mut withheld) =
        graduate(lang, pages, memory, br.meanings(), en, &constructions, &owned);
    // Knowledge lands unconditionally (the two-axis law): every attestation is a read-surface
    // node, first-wins against the funnel's own reads; every failed demonstration is a NAMED row.
    for r in p37_reads {
        if !read_surface.iter().any(|x| x.construct == r.construct)
            && !outcomes.iter().any(|o| o.candidate.construct == r.construct)
        {
            read_surface.push(r);
        }
    }
    for (id, reason) in p37_withheld {
        note_withhold(&mut withheld, id, &reason);
    }
    // The proven constructs (this pass's enforced shapes) — the graded tier never duplicates them.
    let proven_constructs: std::collections::HashSet<String> = outcomes
        .iter()
        .filter(|o| o.rule.is_some())
        .map(|o| o.candidate.construct.clone())
        .collect();
    let mut graded_forms =
        graded_forms(lang, &read_surface, &outcomes, &roles, &proven_constructs, &code_by_url);
    // PASS 37 — the attestation tier merges WITHOUT overwriting (an existing qualified/element
    // form for the same construct already enforces; the attestation adds only new coverage).
    for (construct, form) in p37_graded {
        if !proven_constructs.contains(&construct) && !proven_constructs.contains(&form.fire) {
            graded_forms.entry(construct).or_insert(form);
        }
    }
    // PASS 30 — the self-referee's TEETH: a contradicted node's graded form is withheld (the evidence-
    // graded tier requires uncontradicted evidence; the contradiction stays on the node for the human).
    // It engages exactly when a source disagrees with the web. PASS 37 closure (class 6's second
    // face, measured on `image`): an ELEMENT-TYPOGRAPHY form (`fire = <image>`) is vetoed only by a
    // contradiction that SPEAKS IN that typography — the stored record for the bare word (the
    // Notification `image` member's referee row, a different construct sharing the key) never
    // silences the element's own tier; the element is refereed by its own documented spelling.
    graded_forms.retain(|c, form| {
        referee.get(c).is_none_or(|r| {
            r.contradictions.is_empty()
                || (form.fire.starts_with('<')
                    && !r.contradictions.iter().any(|x| {
                        x.sentence.to_lowercase().contains(&form.fire.to_lowercase())
                    }))
        })
    });
    let web = crate::lint_web::build(br.meanings(), &living_names, &outcomes, &read_surface, &roles, &graded_forms, &referee);
    crate::lint_web::persist(lang, &web);
    let rules = crate::lint_web::derive_rules(lang, &web);
    let graded = crate::lint_web::derive_graded_rules(lang, &web);
    GraduatedModule { rules, corpus_urls, constructions, graded, withheld }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint_read::Binding;

    /// Every fixture page is this test language's OWN crawl — the ledger-ownership scope a real
    /// train reads from its per-tool crawl caches ([`crate::lint_docs::owned_urls`]).
    fn own_urls(pages: &[(String, String)]) -> std::collections::HashSet<String> {
        pages.iter().map(|(u, _)| u.clone()).collect()
    }

    /// DEV PROBE (ignored): replicate the PASS-34 bare-flood gate over the REAL html corpus for the
    /// named constructs — measures which arm let a flooding bare shape (`href`) through.
    #[test]
    #[ignore = "dev probe: needs the machine's whole-site crawl caches"]
    fn probe_bare_flood_gate_on_real_corpus() {
        let data_root = std::path::PathBuf::from("/Users/alexwaldmann/bin");
        let pages = crate::lint_docs::site_corpus(&data_root, "html");
        let attest = crate::lint_attest::Attestation::discover(&pages);
        let attested: std::collections::HashSet<String> =
            pages.iter().filter(|(_, b)| attest.attests(b)).map(|(u, _)| u.clone()).collect();
        let code_by_url = crate::lint_lang_layer::page_code_blocks_by_url(&pages, GRADED_CORPUS_CAP);
        eprintln!("pages {} blocks {} attested-pages {}", pages.len(), code_by_url.len(), attested.len());
        let _ = code_by_url;
        eprintln!("union markers: {}", attest.markers().len());
        for name in ["font", "center", "strike", "dir", "xmp", "plaintext"] {
            let Some((u, b)) = pages.iter().find(|(u, _)| u.ends_with(&format!("/Elements/{name}")) || u.ends_with(&format!("/Global_attributes/{name}"))) else {
                eprintln!("  {name}: NO PAGE");
                continue;
            };
            let ps = attest.attests_page_scope(b);
            let items = !crate::lint_lang_layer::attested_item_shapes(b).0.is_empty();
            let runs: std::collections::HashSet<String> = crate::lint_attest::runs_of(b).into_iter().collect();
            let m0 = attest.markers().first().map(|m| runs.contains(m)).unwrap_or(false);
            let typo = b.contains(&format!("&lt;{name}&gt;"));
            eprintln!("  {name}: page_scope={ps} item_route={items} marker0_in_runs={m0} elt_typo={typo} url={u}");
        }
        // PASS 37 closure — ownership/attribution trace for the attestation read's scope.
        {
            let owned = crate::lint_docs::owned_urls(&data_root, "html");
            let mut extra: std::collections::HashSet<String> =
                crate::lint_train::registered_languages(&data_root).into_iter().collect();
            extra.insert("html".to_string());
            eprintln!("OWNED html: {} urls; registered langs: {}", owned.len(), extra.len());
            for u in [
                "https://www.w3schools.com/tags/tag_applet.asp",
                "https://www.w3schools.com/tags/tag_basefont.asp",
                "https://developer.mozilla.org/en-US/docs/Web/API/HTMLDivElement",
                "https://developer.mozilla.org/en-US/docs/Web/API/HTMLTableCellElement",
                "https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements",
                "https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/a",
            ] {
                eprintln!(
                    "SCOPE {u}: owned={} attributed={:?}",
                    owned.contains(u),
                    crate::lint_docs::attribute_page(u, &[], &extra)
                );
            }
        }
        // Trace the KNOWN banner run's gate counts at union scale.
        let needle = "This feature is no longer recommended";
        let families = crate::lint_attest::frontmatter_families();
        let dep: std::collections::HashSet<&str> =
            families.get("deprecated").map(|s| s.iter().map(String::as_str).collect()).unwrap_or_default();
        eprintln!("dep-family slugs: {}", dep.len());
        let (mut d, mut o, mut t) = (0, 0, 0);
        for (url, body) in &pages {
            if !crate::lint_attest::runs_of(body).iter().any(|r| r.contains(needle)) {
                continue;
            }
            t += 1;
            let slug = url.split("/docs/").nth(1).unwrap_or("").to_lowercase();
            let in_dep = dep.iter().any(|s| s.to_lowercase() == slug);
            if in_dep {
                d += 1;
            }
            for (v, set) in families.iter() {
                if v != "deprecated" && set.iter().any(|s| s.to_lowercase() == slug) && !in_dep {
                    o += 1;
                    break;
                }
            }
        }
        eprintln!("banner run: total-pages {t} dep {d} negative {o}");
        // CENTER TRACE: run the real graduation and report which surface/gate sheds `center`.
        {
            let (Some(br), Some(en)) = (crate::lint_char::brain(), crate::lint_english::brain()) else { panic!("brains") };
            let memory = crate::lint_train::cached_memory("html").unwrap_or_default();
            let constructions = crate::lint_construct::load("html");
            let (outcomes, read, _ref, _liv, _withheld) =
                graduate("html", pages.clone(), &memory, br.meanings(), en, &constructions, &own_urls(&pages));
            for o in outcomes.iter().filter(|o| o.candidate.construct == "center") {
                eprintln!("CENTER candidate: url={} page_scope={} elt_typo={} rule={} verdict={:?}",
                    o.candidate.url, o.candidate.page_scope, o.candidate.element_typography, o.rule.is_some(), o.verdict);
            }
            for r in read.iter().filter(|r| r.construct == "center") {
                eprintln!("CENTER read: url={} page_scope={} elt_typo={}", r.url, r.page_scope, r.element_typography);
            }
            if !outcomes.iter().any(|o| o.candidate.construct == "center")
                && !read.iter().any(|r| r.construct == "center")
            {
                eprintln!("CENTER: in NEITHER surface");
            }
            // Continue the trace through every later stage: graded map → referee → web → rules.
            let constructions2 = crate::lint_construct::load("html");
            let roles = crate::lint_construct::subject_roles(&constructions2, &[]);
            let code_by_url = crate::lint_lang_layer::page_code_blocks_by_url(&pages, GRADED_CORPUS_CAP);
            let proven: std::collections::HashSet<String> =
                outcomes.iter().filter(|o| o.rule.is_some()).map(|o| o.candidate.construct.clone()).collect();
            eprintln!("CENTER proven-set holds center: {}", proven.contains("center"));
            let forms = graded_forms("html", &read, &outcomes, &roles, &proven, &code_by_url);
            eprintln!("CENTER graded_forms: {:?}", forms.get("center"));
            eprintln!("CENTER demo fires: {}", !run_plan(&Plan::UsesConstruct { construct: "<center>".into() }, "html", "<center>x</center>").is_empty());
            let living: std::collections::HashSet<String> = Default::default();
            let web = crate::lint_web::build(br.meanings(), &living, &outcomes, &read, &roles, &forms, &Default::default());
            let node = web.iter().find(|n| n.construct == "center");
            eprintln!("CENTER web node: proven={:?} graded={:?}",
                node.map(|n| n.proven), node.map(|n| n.graded.is_some()));
            let rules = crate::lint_web::derive_graded_rules("html", &web);
            eprintln!("CENTER graded rule derived: {}", rules.iter().any(|(r, _)| r.id.contains("center")));
        }
        let living: std::collections::HashSet<&str> = pages
            .iter()
            .filter(|(u, _)| !attested.contains(u))
            .filter_map(|(u, _)| u.trim_end_matches('/').rsplit('/').next())
            .collect();
        for construct in
            ["href", "xlink:href", "mathcolor", "version", "clip", "frame", "zoomAndPan", "marquee", "center", "big", "rel"]
        {
            eprintln!("{construct:>12}: living-subject={} → withhold-bare={}", living.contains(construct), living.contains(construct));
        }
    }

    /// PASS 34 — the reference read's pure arm: a page whose own example demonstrates its URL-subject
    /// mints exactly that subject with its own governing sentence; a slug page whose examples never
    /// spell it, and a page with no examples at all, mint nothing — and PASS 36 makes each such
    /// refusal a NAMED [`ReferenceRead::Unspelled`] (demonstration is the confirmation).
    #[test]
    fn reference_subjects_mints_only_demonstrated_url_subjects() {
        let body = "<html><body><h1>The video element</h1>\
             <p>The video element embeds a media player for video playback.</p>\
             <pre><code>video controls src=movie.mp4</code></pre></body></html>";
        let ReferenceRead::Subject { construct, governing } =
            reference_subjects("https://docs.test/Web/HTML/Reference/Elements/video", body)
        else {
            panic!("demonstrated subject mints");
        };
        assert_eq!(construct, "video");
        assert!(governing.contains("video"), "governing prose is the page's own sentence: {governing}");
        // The same body under a slug URL its examples never spell: nothing minted — the drop is named.
        assert_eq!(
            reference_subjects("https://docs.test/tags/tag_video.asp", body),
            ReferenceRead::Unspelled { subject: "tag_video.asp".to_string() }
        );
        // No examples at all: nothing to confirm with, nothing minted — same named drop.
        assert_eq!(
            reference_subjects("https://docs.test/Web/HTML/Reference/Elements/video", bare_body()),
            ReferenceRead::Unspelled { subject: "video".to_string() }
        );
    }

    /// A prose-only reference page body (no example block) — the [`ReferenceRead::Unspelled`] fixture.
    fn bare_body() -> &'static str {
        "<html><body><h1>The video element</h1><p>Prose only.</p></body></html>"
    }

    #[test]
    fn self_referee_corroborates_across_sources_and_records_contradictions() {
        // PASS 30 — white-box: the referee over a synthetic pool. Target `cgi` originates on page A;
        // page B asserts its revocation (coherent), page C denies it (contradiction record), page A's own
        // sentence is excluded (independence), and a neutral remedy mention carries nothing.
        let pool = vec![
            PooledSentence { sentence: "The cgi module is deprecated since 3.11.".into(), prohibited: true, url: "https://d/A".into(), negated: false },
            PooledSentence { sentence: "Tools built on the cgi module are deprecated too.".into(), prohibited: true, url: "https://d/B".into(), negated: false },
            PooledSentence { sentence: "The cgi module is not deprecated on this platform.".into(), prohibited: false, url: "https://d/C".into(), negated: true },
            PooledSentence { sentence: "Use urllib.parse instead of cgi.".into(), prohibited: false, url: "https://d/D".into(), negated: false },
        ];
        let targets = vec![("cgi".to_string(), "cgi".to_string(), vec!["https://d/A".to_string()])];
        let map = self_referee(&pool, &targets);
        let rec = map.get("cgi").expect("cgi carries a referee record");
        assert_eq!(rec.coherent, vec!["https://d/B".to_string()], "own page excluded, asserting source counted");
        assert_eq!(rec.contradictions.len(), 1);
        assert_eq!(rec.contradictions[0].source, "https://d/C");
        // A target nobody else speaks about carries NO record (the honest sparse state).
        let silent =
            self_referee(&pool, &[("telnetlib".to_string(), "telnetlib".to_string(), vec![])]);
        assert!(silent.is_empty());
        // Bounded full-token mention: `cgi` never rides `cgitb`, and a dotted chain is one token.
        assert!(!mentions_full_token("the cgitb module is deprecated", "cgi"));
        assert!(mentions_full_token("the cgi module is deprecated", "cgi"));
        assert!(!mentions_full_token("ssl.sslsocket.read is fine", "read"));
        // PASS 37 class 6 — an ELEMENT-typography subject is refereed by its `<x>` typography:
        // the bare common word never matches, the typographic mention does.
        let word_collision = PooledSentence {
            sentence: "The view from a center point of the field.".into(),
            prohibited: false,
            url: "https://d/E".into(),
            negated: true,
        };
        let element_targets =
            vec![("center".to_string(), "<center>".to_string(), vec!["https://d/O".to_string()])];
        let quiet = self_referee(&[word_collision], &element_targets);
        assert!(quiet.is_empty(), "a common-word mention never referees an element subject");
        let typographic = PooledSentence {
            sentence: "The <center> element is not deprecated here.".into(),
            prohibited: false,
            url: "https://d/E".into(),
            negated: true,
        };
        let heard = self_referee(&[typographic], &element_targets);
        assert!(heard.get("center").is_some_and(|r| !r.contradictions.is_empty()));
    }

    #[test]
    fn member_demo_ok_accepts_own_parent_local_and_literal_receivers_rejects_foreign() {
        // Classmethod style: receiver equals the item's own parent component.
        assert!(member_demo_ok("dt = datetime.utcnow()", ".utcnow", "datetime"));
        // Block-local instance: the receiver was introduced earlier in the same block.
        assert!(member_demo_ok("let s = \" hi \"; let t = s.trim_left();", ".trim_left", "method"));
        // Literal receiver: the value's own type owns the member.
        assert!(member_demo_ok("assert_eq!(\"x\", \"1x1\".trim_left_matches('1'));", ".trim_left_matches", "method"));
        // Foreign namespace: the demo names ANOTHER item (the recommended replacement) — rejected.
        assert!(!member_demo_ok("x: collections.abc.Sequence[int] = []", ".Sequence", "typing"));
        assert!(!member_demo_ok("d = collections.OrderedDict()", ".OrderedDict", "typing"));
    }

    #[test]
    fn member_receivers_counts_distinct_receivers() {
        let corpus = vec![
            "words = text.split(',')".to_string(),
            "parts = name.split('.')".to_string(),
            "re.split(pattern, s)".to_string(),
            "dt = datetime.utcnow()".to_string(),
        ];
        assert!(member_receivers(&corpus, ".split") >= 3, "split rides many receivers");
        assert_eq!(member_receivers(&corpus, ".utcnow"), 1, "utcnow is bound to one receiver");
    }

    /// PASS 27 — the qualified-safe cut rejects the measured junk classes (bare members, URL basenames,
    /// rustdoc anchors, bare generics) and admits real `owner.member` forms.
    #[test]
    fn qualified_safe_rejects_junk_admits_owner_member() {
        assert!(qualified_safe("typing.Sequence"));
        assert!(qualified_safe("Object.__defineGetter__"));
        assert!(qualified_safe("importlib.abc.Loader.load_module"));
        assert!(!qualified_safe(".read"), "bare leading-dot member");
        assert!(!qualified_safe("ssl.html"), "doc URL basename");
        assert!(!qualified_safe("struct.Vec.html"), "doc URL basename");
        assert!(!qualified_safe("method.foo"), "rustdoc anchor form");
        assert!(!qualified_safe("compile"), "bare generic single identifier");
    }

    /// PASS 27 — the graded gates measured end to end over a synthetic corpus: a discriminating corpus
    /// trusts death (receiver-generic form), an all-dead corpus is POVERTY (every form falls to the
    /// dotted-literal), a proven-covered form is deduped, and a form firing on the corpus's own clean
    /// blocks is dropped (the flood guard).
    #[test]
    fn graded_forms_gates_calibrate_dedup_and_stay_flood_safe() {
        let read = |c: &str, url: &str| ReadConstruct {
            page_scope: true,
            element_typography: true,
            construct: c.to_string(),
            governing: format!("The {c} member is deprecated."),
            url: url.to_string(),
            attested_deprecated: true,
        };
        let dead = read("Legacy.deadfn", "https://d/legacy-deadfn");
        let alive = read("Legacy.alive", "https://d/legacy-alive");
        let roles = std::collections::HashMap::new();
        let no_proven = std::collections::HashSet::new();
        // Corpus: `.alive` rides another page's code; `.deadfn` appears nowhere else. Each subject's
        // OWN page demonstrates its usage (the language-ownership gate's evidence — a form must fire
        // under this grammar on its own page's examples).
        let corpus = vec![
            ("https://d/legacy-deadfn".to_string(), "old.deadfn(); Legacy.deadfn();".to_string()),
            ("https://d/legacy-alive".to_string(), "Legacy.alive();".to_string()),
            ("https://d/other".to_string(), "let x = obj.alive();".to_string()),
            ("https://d/other2".to_string(), "const y = 1;".to_string()),
        ];
        // Discriminating corpus (one alive, one dead): death is TRUSTED — the dead member fires
        // receiver-generic, the alive one falls to its dotted-literal form.
        let forms = graded_forms("javascript", &[dead.clone(), alive.clone()], &[], &roles, &no_proven, &corpus);
        assert_eq!(forms.get("Legacy.deadfn").unwrap().fire, ".deadfn", "trusted-dead fires receiver-generic");
        assert_eq!(forms.get("Legacy.alive").unwrap().fire, "Legacy.alive", "alive member falls to dotted-literal");
        assert_eq!(forms.get("Legacy.deadfn").unwrap().severity, "low");
        // All-dead distribution (only the dead candidate): corpus POVERTY — the verdict is NOT trusted,
        // the form falls to dotted-literal (the python 9-page `.read` flood, measured).
        let poverty = graded_forms("javascript", &[dead.clone()], &[], &roles, &no_proven, &corpus);
        assert_eq!(poverty.get("Legacy.deadfn").unwrap().fire, "Legacy.deadfn", "uncalibrated death never fires receiver-generic");
        // Proven-coverage dedup: a proven `.deadfn` rule already fires every `X.deadfn` — skipped.
        let proven: std::collections::HashSet<String> = [".deadfn".to_string()].into();
        let deduped = graded_forms("javascript", &[dead.clone(), alive.clone()], &[], &roles, &proven, &corpus);
        assert!(!deduped.contains_key("Legacy.deadfn"), "a proven-covered form is never duplicated");
        assert!(deduped.contains_key("Legacy.alive"), "uncovered forms still graduate");
        // Clean-near-miss flood guard: the alive DOTTED form appears in another page's own current code —
        // firing it would flag the corpus's own clean examples, so it is dropped entirely.
        let contested = vec![
            ("https://d/legacy-deadfn".to_string(), "old.deadfn(); Legacy.deadfn();".to_string()),
            ("https://d/legacy-alive".to_string(), "Legacy.alive();".to_string()),
            ("https://d/other".to_string(), "let x = obj.alive();\nLegacy.alive();".to_string()),
            ("https://d/other2".to_string(), "const y = 1;".to_string()),
        ];
        let dropped = graded_forms("javascript", &[dead.clone(), alive.clone()], &[], &roles, &no_proven, &contested);
        assert!(!dropped.contains_key("Legacy.alive"), "a form firing on the corpus's own clean code is dropped");
    }

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

    /// The blind loop's parity contract: over an ALL-`Flag` sample set [`prove_blind`] returns the SAME
    /// [`Verdict`] as the frozen [`crate::lint_selftest::prove`] for the single-rule book `graduate`
    /// builds — across the firing (Corroborates/Mismatch/Undecidable) and non-firing (NotFlagged) rep
    /// classes. Guards that the flag half of the blind loop (and its once-computed comparator) never
    /// changes a graduation decision (native/architecture.dx: bit-identical funnel); the clean reps are additive.
    #[test]
    fn blind_prove_matches_frozen_prove() {
        use crate::lint_selftest::{prove, KnownRule};
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        // A mix of understanding/advice/foil triples exercising different comparator outcomes, each with
        // firing (`var …`) and non-firing (`let …`) samples so both rep classes are covered.
        let cases = [
            ("Never use the `var` keyword.", "The `var` statement is discouraged and should be avoided.", "Use the `eval` function to run code."),
            ("Avoid the `var` declaration.", "The `eval` function executes a string as code.", "Never use the `var` keyword."),
            ("The `var` keyword declares a variable.", "A widget renders a colourful banner.", "Bananas grow in tropical climates."),
        ];
        let mut samples: Vec<String> = (0..REQUIRED_REPS + 3).map(|i| format!("var v{i} = {i};")).collect();
        samples.push("let a = 1;".to_string()); // a non-firing near-miss → NotFlagged
        samples.push("const b = 2;".to_string());
        let refs: Vec<&str> = samples.iter().map(String::as_str).collect();
        // The blind loop over an ALL-`Flag` set must reduce to the frozen prove.
        let flag_samples: Vec<Sample> =
            samples.iter().map(|c| Sample { code: c.clone(), expect: Expect::Flag }).collect();
        for (understanding, advice, foil) in cases {
            let rule = RuleUnderTest::new(understanding.to_string(), foil.to_string(), "javascript".to_string());
            let plan = Plan::UsesConstruct { construct: "var".to_string() };
            let book = [KnownRule::new(plan.clone(), advice.to_string())];
            let frozen = prove(m, en, &rule, &book, &refs);
            let blind = prove_blind(m, en, &rule, &plan, advice, &flag_samples);
            assert_eq!(frozen, blind, "prove_blind (all-Flag) must equal frozen prove for ({understanding} | {advice} | {foil})");
        }
    }

    /// The novel half of owner point 3: CLEAN expect-no-flag samples count toward agreement when the
    /// rule's English reconciles, and a clean sample that FIRES is a fatal false positive. Uses the
    /// frozen comparator's proven abstract is-a fixture (`a dog is a canine`) and the opaque token
    /// `alpha` (fires) / `beta` (never appears) — no real construct or meaning.
    #[test]
    fn clean_samples_count_toward_agreement_and_a_clean_firing_is_fatal() {
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        let rule = RuleUnderTest::new("a dog is a canine", "a dog is a bird", "javascript".to_string());
        let plan = Plan::UsesConstruct { construct: "alpha".to_string() };
        let advice = "a dog is a canine animal"; // reconciles Some(true) over the bird foil

        // Too few flag reps ALONE (5 < REQUIRED_REPS) is not enough; the clean near-misses (`beta;`,
        // which never fires) top the agreement over the floor — the squeeze from the other side.
        let mut samples: Vec<Sample> =
            (0..5).map(|i| Sample { code: format!("alpha; // {i}"), expect: Expect::Flag }).collect();
        let flag_only = prove_blind(m, en, &rule, &plan, advice, &samples);
        assert!(matches!(flag_only, Verdict::Unproven(_)), "5 flag reps alone are below the floor");
        samples.extend((0..REQUIRED_REPS).map(|i| Sample { code: format!("beta{i};"), expect: Expect::Clean }));
        assert_eq!(prove_blind(m, en, &rule, &plan, advice, &samples), Verdict::Proven,
            "clean expect-no-flag reps count toward agreement when the English reconciles");

        // A clean sample that FIRES (`alpha;` tagged Clean) is a false positive — fatal.
        let contradicting = vec![Sample { code: "alpha;".to_string(), expect: Expect::Clean }];
        assert!(matches!(prove_blind(m, en, &rule, &plan, advice, &contradicting), Verdict::Unproven(_)),
            "a clean sample the rule flags is a fatal self-contradiction");
    }

    #[test]
    fn mentions_is_token_delimited_not_substring() {
        assert!(mentions("Never use the `var` keyword.", "var"));
        assert!(mentions("The var statement is old.", "var"));
        assert!(!mentions("A variable holds a value.", "var"), "must not match inside 'variable'");
        assert!(mentions("Use === instead of ==.", "=="));
    }

    #[test]
    fn rule_id_byte_preserves_the_construct_no_collision() {
        assert_eq!(rule_id("var"), "uses-var");
        // Byte-preserved: dotted members and operators keep their exact bytes (owner point 1).
        assert_eq!(rule_id("document.write"), "uses-document.write");
        // The collision the correction fixes: `==` and `++` were both `uses--`; now distinct.
        assert_eq!(rule_id("=="), "uses-==");
        assert_eq!(rule_id("++"), "uses-++");
        assert_ne!(rule_id("=="), rule_id("++"), "distinct constructs must have distinct ids");
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

        let (outcomes, _read, _referee, _living, _withheld) = graduate("javascript", pages.clone(), &memory, m, en, &[], &own_urls(&pages));
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

    /// PASS 24 — RULES ARE VIEWS. The rules the live path fires are DERIVED from the language web
    /// ([`crate::lint_web::derive_rules`] over [`crate::lint_web::build`]), and that derivation is
    /// BYTE-IDENTICAL to the old direct `graduate(...).filter_map(|o| o.rule)`: every proven outcome is a
    /// proven web node carrying that exact `(rule, url)`, in the same order. This test proves the seam over
    /// the real graduation fixture — the web is the source of truth, the rule list is its projection.
    #[test]
    fn rules_are_a_byte_identical_view_over_the_web() {
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
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
        let mut memory = Memory::default();
        for i in 0..REQUIRED_REPS + 2 {
            memory.reference.push(format!("var v{i} = {i};"));
        }
        memory.reference.push("let a = 1;".to_string());
        memory.reference.push("const b = 2;".to_string());

        let (outcomes, read, referee, _living, _withheld) = graduate("javascript", pages.clone(), &memory, m, en, &[], &own_urls(&pages));
        let direct: Vec<(LearnedRule, String)> = outcomes.iter().filter_map(|o| o.rule.clone()).collect();
        let web = crate::lint_web::build(m, &Default::default(), &outcomes, &read, &std::collections::HashMap::new(), &std::collections::HashMap::new(), &referee);
        let viewed = crate::lint_web::derive_rules("javascript", &web);
        assert_eq!(direct, viewed, "the web-derived rules must equal the direct emitted rules byte-for-byte");
        // Everything READ is retained: every proposed candidate AND every never-proposed read construct is
        // a node; only the proven ones are rules — the derive-view is unmoved by the retained-unproven nodes.
        assert_eq!(web.len(), outcomes.len() + read.len(), "every read construct is a web node");
        assert_eq!(web.iter().filter(|n| n.proven).count(), direct.len(), "proven nodes == rules");
        assert!(web.iter().filter(|n| !n.proven).count() >= read.len(), "the never-proposed read constructs are retained unproven");
    }

    /// Item 3d — FIXPOINT in ONE iteration. Graduation is a deterministic pure function of a FROZEN brain
    /// and a FIXED corpus (it never reads its own output or the ledger), so a second pass over the same
    /// inputs yields the byte-identical proven set — the proven set is already at fixpoint after one pass;
    /// a literal re-iteration would only burn compute to confirm no change. This test IS that measurement.
    #[test]
    fn graduation_reaches_fixpoint_in_one_iteration() {
        let Some((br, en)) = brains() else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
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
        let mut memory = Memory::default();
        for i in 0..REQUIRED_REPS + 2 {
            memory.reference.push(format!("var v{i} = {i};"));
        }
        memory.reference.push("let a = 1;".to_string());
        memory.reference.push("const b = 2;".to_string());

        let proven = |outcomes: Vec<Outcome>| -> Vec<(String, String)> {
            let mut ids: Vec<(String, String)> =
                outcomes.into_iter().filter_map(|o| o.rule).map(|(r, _)| (r.id, r.description)).collect();
            ids.sort();
            ids
        };
        let first = proven(graduate("javascript", pages.clone(), &memory, m, en, &[], &own_urls(&pages)).0);
        let second = proven(graduate("javascript", pages.clone(), &memory, m, en, &[], &own_urls(&pages)).0);
        // The fixpoint claim is DETERMINISM — it holds whether or not this machine's brain state
        // graduates the fixture, so it never false-fails on brain-state/test-order (the graduation count
        // itself is the sibling test's job, gated on the suite's brain). Reported, not asserted.
        eprintln!("fixpoint: pass graduated {} rule(s)", first.len());
        assert_eq!(first, second, "graduation is deterministic — fixpoint reached in one iteration");
    }
}
