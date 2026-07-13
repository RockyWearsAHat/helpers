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
    /// The graduated rule ready for the module, paired with its source url. `Some` iff the candidate
    /// graduated AND a clean near-miss existed to contrast against — either the English self-test proved it
    /// (`verdict == Proven`), or the NOTECARD PATH did (a structurally-attested deprecation that fired
    /// ≥ `REQUIRED_REPS` with a near-miss, whose degenerate identical prose the English judge could not
    /// apply). For a notecard graduation `verdict` stays the honest English result (often `Contradicted`).
    pub rule: Option<(LearnedRule, String)>,
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
/// real corpus blocks, and the frozen `run_plan` is the ONLY referee of what fires (LINTER.md → Fix 2).
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
/// rule's own `construct` field, LINTER.md "no id-parsing hack") — so any construct bytes are safe. The
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
    let partition = lang_pages(lang, pages, &bridge, en, &attested);
    propose(lang, &partition, &bridge, en, &attested, memory).0
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
fn lang_pages<'a>(
    lang: &str,
    pages: &'a [(String, String)],
    bridge: &Bridge,
    en: &English,
    attested: &std::collections::HashSet<String>,
) -> Vec<&'a (String, String)> {
    pages.iter().filter(|(u, body)| page_proves_in_lang(lang, u, body, bridge, en, attested)).collect()
}

/// Whether a prohibition/deprecation page PROVES in `lang`: its role names a prohibited subject AND that
/// subject fires on the page's OWN worked-example code under `lang`'s grammar (the frozen `run_plan`).
/// The page's example code is read STRUCTURALLY ([`crate::lint_lang_layer::page_code_corpus`], every
/// `<pre><code>`), so the referee is the language grammar, not the URL — the whole point of the
/// verification-decided partition. A non-prohibition page, or one whose subject the grammar does not fire
/// on the page's own examples, is not in this language's partition.
fn page_proves_in_lang(lang: &str, url: &str, body: &str, bridge: &Bridge, en: &English, attested: &std::collections::HashSet<String>) -> bool {
    let page = crate::lint_lang_layer::read_doc_page(url, body, en, bridge, attested);
    if !page.prohibited || page.constructs.is_empty() {
        return false;
    }
    // A rendered-marker page (Python/Rust) demonstrates its items in bare inline `<code>`, not
    // `<pre><code>`, so its example corpus is the WIDENED reading; a URL-subject page (MDN) has no
    // markers and reads exactly the frozen `<pre><code>` corpus — byte-identical.
    let own = if page.marked_deprecated.is_empty() {
        crate::lint_lang_layer::page_code_corpus(
            std::slice::from_ref(&(url.to_string(), body.to_string())),
            lang,
            MAX_HARVEST_BLOCKS,
        )
    } else {
        crate::lint_lang_layer::page_example_corpus(body, &page.marked_deprecated)
    };
    page.constructs.iter().any(|c| {
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
    })
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
fn propose(lang: &str, pages: &[&(String, String)], bridge: &Bridge, en: &English, attested: &std::collections::HashSet<String>, memory: &Memory) -> (Vec<Candidate>, Vec<PooledSentence>) {
    let mut docpages: Vec<crate::lint_lang_layer::DocPage> =
        pages.iter().map(|(url, body)| crate::lint_lang_layer::read_doc_page(url, body, en, bridge, attested)).collect();

    // QUALIFIED-MEMBER SHAPE SELECTION (LINTER.md → "QUALIFIED-MEMBER construct extraction"). A
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
    for p in &docpages {
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
                .filter(|c| is_prohibited_subject(lang, &p.url, c, &p.incorrect, &p.correct))
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
        let stated = stated_by_lead(lang, construct, &lead_named, &p.incorrect, &p.correct);
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
                attested_deprecated: p.attested_deprecated,
                stated,
            });
        }
        }
    }
    (out, pooled)
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
/// `??`-vs-`==` discriminator (LINTER.md → the stated-subject gate, owner ruling 2026-07-11). A genuine
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
/// (the memoization LINTER.md's speed pass blessed — a pure comparator re-evaluated identically per rep).
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
/// distinct real blocks fire AND the two doc sentences reconcile AND none contradicts (LINTER.md).
pub fn graduate(
    lang: &str,
    pages: &[(String, String)],
    memory: &Memory,
    m: &MeaningNetwork,
    en: &English,
) -> Vec<Outcome> {
    let bridge = Bridge::new(m, en);
    // The LEARNED deprecation attestation, keyed by the author's OWN METADATA TYPOGRAPHY (frontmatter
    // `status:` enum joined to the crawled pages by slug — COMPLETION PASS 13). Discovered from and applied
    // to the ORIGINAL (pre-chrome-strip) bodies: the attestation is a PAGE-ROLE fact of the whole page,
    // and the notecard's banner TEXT run is cross-page-identical, so the chrome filter (which removes
    // invariant text runs for clean PROSE) strips it away — reading the role after the strip would lose it
    // exactly as it would lose a class attribute. The attested URL SET is captured here and threaded down,
    // so every downstream reader keys on the URL, not the stripped body. Replaces the hardcoded
    // `has_deprecation_notecard` substring; MEASURED P=R=1.000 vs it (`examples/metajoin`).
    let attest = crate::lint_attest::Attestation::discover(pages);
    let attested: std::collections::HashSet<String> =
        pages.iter().filter(|(_, b)| attest.attests(b)).map(|(u, _)| u.clone()).collect();
    // CROSS-PAGE-INVARIANCE CHROME FILTER (LINTER.md → "Cross-page invariance = chrome, discarded";
    // owner north-star). A site's navigation, breadcrumb, footer, and sidebar-menu text recurs
    // IDENTICALLY across its pages and carries zero governing meaning, so it is discarded — site-
    // scoped, learned from the corpus by exact text-run recurrence, no element name and no site name
    // named ([`crate::lint_graph::site_chrome`]). This removes the W3Schools `<div id="leftmenuinner">`
    // menu that a semantic-element chrome drop cannot catch, the measured blocker for reading W3S
    // prose. Applied before any page is read into prohibition prose or example code, so every reader
    // downstream — `read_doc_page`, `page_code_corpus`, the grammar partition — sees clean bodies.
    let chrome = crate::lint_graph::site_chrome(pages);
    let stripped: Vec<(String, String)> =
        pages.iter().map(|(u, b)| (u.clone(), chrome.strip(u, b))).collect();
    let pages: &[(String, String)] = &stripped;
    let partition = lang_pages(lang, pages, &bridge, en, &attested);
    let (candidates, pool) = propose(lang, &partition, &bridge, en, &attested, memory);
    let corpus = harvest_corpus(memory);

    // Each candidate's derived advice (its SECOND, distinct doc sentence). A candidate with no such
    // sentence has no un-fakeable English pair and cannot graduate.
    let advices: Vec<Option<String>> = candidates
        .iter()
        .map(|c| derive_advice(&pool, en, &c.construct, &c.understanding, &c.url))
        .collect();

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
        // (LINTER.md → the two walls → Fix 2). Harvested reps stay primary; generation only supplies count.
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

        // NOTECARD GRADUATION PATH (LINTER.md → the notecard-as-proof route, owner ruling 2026-07-11).
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
        let rule = if cand.stated && (matches!(verdict, Verdict::Proven) || notecard_proven) {
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
                        // The rule IS its understood prohibition: carry the construct so the live
                        // build compiles `uses_construct(construct)` and fires the SAME plan the
                        // frozen loop proved — never a detector re-derived from the example diff.
                        construct: Some(cand.construct.clone()),
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

/// Whether `lang`'s documentation READ PASS has completed (owner correction 2026-07-12, point 2): the
/// crawl produced a page corpus — `pages` (the read pass's own persisted raw-page output) is non-empty.
/// A cold cache (no pages) is an INCOMPLETE read, and a language is never graduated from a half-read
/// crawl. This is a STRUCTURAL precondition (the read output exists), not a heuristic over the prose.
fn read_pass_complete(pages: &[(String, String)]) -> bool {
    !pages.is_empty()
}

/// The result of a graduation pass: the PROVEN construct rules AND the exact corpus the pass read.
/// `corpus_urls` is the re-check basis for contradiction-driven reshape (LINTER.md → Item 3c): the set
/// of page URLs this pass proposed over, so the caller can tell a rule whose source page is STILL in the
/// corpus (its absence from `rules` is a genuine failure to re-prove — a contradiction) from one whose
/// page has LEFT the corpus (a subset crawl — retain the last proof, unrefreshed).
pub struct GraduatedModule {
    /// Every rule the pass PROVED this crawl, as `(rule, source url)`.
    pub rules: Vec<(LearnedRule, String)>,
    /// The URLs of every page the pass read (raw pages ∪ whole-site corpus) — the re-check basis.
    pub corpus_urls: std::collections::HashSet<String>,
}

/// The LIVE entry the module build calls (the covenant-clean successor to
/// [`crate::lint_docs::rules_from_memory`] for MODULES): graduate `lang`'s construct rules from the
/// read `memory` through the frozen loop, returning every PROVEN rule AND the corpus it read (the
/// [`GraduatedModule`] contract — the corpus is Item 3c's re-check basis). Loads the two frozen brains;
/// an empty module when either is unavailable (the loop is defined only over the real bedrock — never
/// fake a rule). Never trains, never touches the network.
pub fn graduated_rules(lang: &str, memory: &Memory) -> GraduatedModule {
    let empty = || GraduatedModule { rules: Vec::new(), corpus_urls: std::collections::HashSet::new() };
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
    let (mut pages, _fp) = crate::lint_docs::raw_pages(&data_root, lang);
    {
        let mut seen: std::collections::HashSet<String> = pages.iter().map(|(u, _)| u.clone()).collect();
        for (u, b) in crate::lint_docs::site_corpus(&data_root, lang) {
            if seen.insert(u.clone()) {
                pages.push((u, b));
            }
        }
    }
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
    // OFFLINE-ROBUSTNESS FALLBACK (LINTER.md → "the recommended unlock"). The frozen loop's evidence is
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
    let rules = graduate(lang, &pages, memory, br.meanings(), en)
        .into_iter()
        .filter_map(|o| o.rule)
        .collect();
    GraduatedModule { rules, corpus_urls }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint_read::Binding;

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
    /// changes a graduation decision (LINTER.md: bit-identical funnel); the clean reps are additive.
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
        let first = proven(graduate("javascript", &pages, &memory, m, en));
        let second = proven(graduate("javascript", &pages, &memory, m, en));
        // The fixpoint claim is DETERMINISM — it holds whether or not this machine's brain state
        // graduates the fixture, so it never false-fails on brain-state/test-order (the graduation count
        // itself is the sibling test's job, gated on the suite's brain). Reported, not asserted.
        eprintln!("fixpoint: pass graduated {} rule(s)", first.len());
        assert_eq!(first, second, "graduation is deterministic — fixpoint reached in one iteration");
    }
}
