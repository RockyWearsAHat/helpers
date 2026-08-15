//! `lint_match` — LOSSLESS rule matching. A rule is not a bag of features (which discards the
//! structure, and the discarded structure is exactly the false positives) but a generalized
//! sub-tree PATTERN taken from its own example, matched against code by EXACT sub-tree containment
//! with variable binding ([`tree`]). Prose-only rules compile through learned-evidence construct
//! selection instead ([`select`]); grammars resolve at runtime ([`grammar`]); this module owns the
//! [`RuleSet`] build pipeline with its compile-time gates (self-fire, over-fire, reference-fire,
//! dedup) and the firing engine.
//!
//! Cross-module theory, evidence hierarchy, and the failure ledger live in `native/architecture.dx` at the
//! repo root — the single authoritative doc; update it BEFORE changing semantics here.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use tree_sitter::Parser;


mod grammar;
mod select;
mod tree;

#[cfg(test)]
mod tests;

pub use grammar::bundled_language;
pub(crate) use grammar::{code_ngrams, is_construct_keyword, language};
pub use select::Grounding;
pub use tree::RulePattern;

use select::{blank_spans, code_surface_file, description_discriminator, text_discriminator, GroundView};

/// Largest documented example that can still BE a rule. A pointable anti-pattern is at most a
/// screenful; anything bigger is a scraped sample program or a whole manual page, which no single
/// rule describes — and which turns compilation (tree diffing, token-pair diff search) into
/// minutes of work for zero yield. The cap encodes what a rule IS, not what any language looks
/// like.
pub(super) const MAX_EXAMPLE_BYTES: usize = 8192;

/// Smallest reference corpus (in lines) the REFERENCE-FIRE gate may judge from. The gate is
/// statistical — "this detector trips on the language's own normal code" — and a handful of
/// grounding examples or a discovery probe cannot testify to that; below this scale the gate
/// stays out of the way.
const REFERENCE_FIRE_MIN_LINES: usize = 500;

/// Whether `line` (reference-corpus code) addresses `token` with dotted OWNER typography —
/// an occurrence of the token written as `owner.token` (an identifier character, then `.`,
/// then the token at a word boundary). This is the author's own qualified spelling of a
/// MEMBER, read straight off the corpus text: no shape list, no grammar. Used by the compile's
/// member-typography veto (PASS 36) — a bare single-token detector on a member name would fire
/// on every unrelated owner's own use of that identifier.
fn dotted_owner_typography(line: &str, token: &str) -> bool {
    dotted_owner_chain(line, token).is_some()
}

/// The author's own dotted chain `owner.token` read off a reference-corpus `line` that
/// addresses `token` with dotted OWNER typography — the qualified spelling the member veto
/// declares enforceable (`Gadget.grip(handle)` → `gadget.grip`, lowercased because every
/// detector matches the lowercased surface). `None` when the line never spells the token as a
/// member. This is the member veto's ENFORCING arm (owner ruling 2026-07-18, second): the bare
/// token is refused, the author's typography IS the detector.
fn dotted_owner_chain(line: &str, token: &str) -> Option<String> {
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    line.match_indices(token).find_map(|(at, _)| {
        let before = &line[..at];
        let after_ok = line[at + token.len()..].chars().next().map(|c| !ident(c)).unwrap_or(true);
        if !(after_ok && before.ends_with('.')) {
            return None;
        }
        let head = &before[..before.len() - 1];
        let owner_start = head
            .rfind(|c: char| !ident(c))
            .map_or(0, |i| i + head[i..].chars().next().map_or(1, char::len_utf8));
        let owner = &head[owner_start..];
        (!owner.is_empty())
            .then(|| format!("{}.{}", owner.to_lowercase(), token.to_lowercase()))
    })
}

/// The ONE-containment-matcher token form of a graduated CONSTRUCT for a grammarless language
/// (PASS 37): a `host@attr` attribute construct compiles to the tag-scoped ordered pair
/// `[<host, attr]` (both halves inside one tag open — the span law the matcher reads off the
/// `<` typography); an ELEMENT construct `<x>` compiles to the tag-open token `<x` (MEASURED,
/// census: the literal `<x>` only matches an attribute-less tag — `<zapplet code="…">` never
/// fired — while `<x` witnesses every tag open and still never the word at attribute/value
/// position, which carries no `<`); every other construct is its own single containment token,
/// lowercased (matching is case-insensitive). Shared by the compile and the train-time
/// demonstration gate so they can never disagree about what the detector means.
pub(crate) fn construct_tokens(construct: &str) -> Vec<String> {
    if let Some((host, attr)) = construct.split_once('@') {
        if !host.is_empty() && !attr.is_empty() {
            return vec![format!("<{}", host.to_lowercase()), attr.to_lowercase()];
        }
    }
    if let Some(name) = construct.strip_prefix('<').and_then(|c| c.strip_suffix('>')) {
        if !name.is_empty() {
            return vec![format!("<{}", name.to_lowercase())];
        }
    }
    vec![construct.to_lowercase()]
}

/// Whether `tokens` fire anywhere in `text` under the ONE containment matcher — the
/// grammarless demonstration gate's referee (PASS 37; [`select::tokens_fire_text`]).
pub(crate) fn containment_fires(text: &str, tokens: &[String]) -> bool {
    select::tokens_fire_text(text, tokens)
}

/// Bound advice prose from documentation or a pulled module before it is stored and later
/// shown to an agent: strip control characters (ANSI escapes, zero-width and line-break
/// injection that could forge report structure or hide text), collapse whitespace runs, and
/// cap length. Not a proof of safety — a bound on the injection surface of text the machine
/// did not author (native/architecture.dx, "the distribution channel"). Project law is exempt: it is the
/// user's own text.
fn sanitize_advice(desc: &str) -> String {
    const MAX_ADVICE: usize = 400;
    let mut out = String::with_capacity(desc.len().min(MAX_ADVICE));
    let mut last_ws = false;
    for c in desc.chars() {
        // Keep printable graphemes; every control char (incl. ESC, newline, tab, zero-width
        // and bidi-override formatting) collapses to a single space.
        let keep = !c.is_control() && !matches!(c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}' | '\u{feff}');
        if keep && !c.is_whitespace() {
            out.push(c);
            last_ws = false;
        } else if !last_ws {
            out.push(' ');
            last_ws = true;
        }
        if out.len() >= MAX_ADVICE {
            break;
        }
    }
    out.trim().to_string()
}

/// Languages whose files ARE prose: a rule written *for* them governs English text, so its
/// detector matches raw lines — there are no string literals or code comments to mask. Every
/// code language matches its [`select::code_surface_file`] instead (same universe as
/// grounding). Numeric extensions are man sections (POSIX numbers them 1–9): rendered English,
/// never a code-law target — man prose once grounded "never" as code (ledger #14).
pub(crate) fn prose_lang(lang: &str) -> bool {
    matches!(lang, "md" | "markdown" | "txt" | "text" | "rst")
        || (!lang.is_empty() && lang.chars().all(|c| c.is_ascii_digit()))
}

/// How a rule matches code — either lossless AST pattern (when a grammar is available) or a
/// discriminating token sequence (whole-token containment, universal fallback for any language).
///
/// Both paths go through the same `bad ∧ ¬good` discipline: the pattern is derived from what the
/// `bad` example has that the `good` example does not. The difference is precision: AST patterns
/// capture structure (scope, co-reference); text patterns capture presence of distinctive tokens.
#[derive(Clone, Debug, Serialize, Deserialize)]
enum MatchKind {
    /// Exact generalized subtree match via tree-sitter.
    Ast(RulePattern),
    /// A token sequence over source lines — used when no grammar is available for the
    /// language, or when a prose-only law names a construct. The detector IS its tokens
    /// (lowercased; one distinctive token, or an ordered same-line pair) and fires by
    /// whole-token containment ([`select::tokens_fire_line`]) — no regex engine anywhere.
    Tokens {
        tokens: Vec<String>,
        /// Firing universe: `false` matches the code surface (strings/comments blanked —
        /// ledger #12); `true` matches raw lines, because the rule's construct grounded only
        /// inside the project's comments/strings and that is the universe the law governs
        /// (native/architecture.dx, evidence hierarchy). Project law, plus a learned DEMONSTRATED-SHAPE
        /// detector whose bad/good contrast lives only inside a string literal (PASS 36,
        /// [`select::demonstrated_shape`] — the abstain-trap's law governs the string's
        /// interior); every other learned rule is always code.
        #[serde(default)]
        raw: bool,
    },
    /// A structural AST PROBE (native/architecture.dx, "Rules from understanding — the probe bridge"): the
    /// rule fires wherever a coded predicate recognises a defect SHAPE (dead code, an unwrap, a
    /// magic number …). Which probe — and thus which principle is enforced — was decided by
    /// READING the corpus prose and understanding it ([`crate::lint_probe::understand`]); the
    /// stored string is the probe's stable name.
    Probe(String),
    /// The UNDERSTANDING→TRACE bridge (native/architecture.dx, "the understanding→trace bridge"): the corpus
    /// principle's prose was read by [`crate::lint_trace`] into a composition of GENERIC tracing
    /// primitives (a [`crate::lint_trace::Plan`]) — the rule IS the understanding, no per-principle
    /// detector. The plan fires by walking the parsed tree. This is the primary path for corpus
    /// principles; [`MatchKind::Probe`] is the committed fallback for prose the bridge abstains on.
    Trace(crate::lint_trace::Plan),
}

impl MatchKind {
    /// Lines in `code` where this rule fires. 1-based. The caller hands a token detector the
    /// file's CODE SURFACE ([`code_surface_file`] — string interiors and comments blanked)
    /// for code languages, and the raw text for prose files: a law grounds only against real
    /// code, so its detector must fire in the same universe (native/architecture.dx ledger #12/#14). AST
    /// patterns parse the raw source — a string node is never an identifier. A [`MatchKind::Probe`]
    /// needs `lang` to parse, so every caller threads it through.
    fn matches(&self, lang: &str, code: &str) -> Vec<usize> {
        match self {
            MatchKind::Ast(pat) => pat.matches(code),
            MatchKind::Tokens { tokens, .. } => code
                .lines()
                .enumerate()
                .filter(|(_, line)| select::tokens_fire_line(line, tokens))
                .map(|(i, _)| i + 1)
                .collect(),
            MatchKind::Probe(name) => crate::lint_probe::ProbeKind::from_name(name)
                .map(|k| k.detect(lang, code))
                .unwrap_or_default(),
            MatchKind::Trace(plan) => crate::lint_trace::run_plan(plan, lang, code),
        }
    }
}

/// One documented rule compiled to its exact match kind, carrying the reporting facts a finding needs.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct CompiledRule {
    id: String,
    severity: String,
    /// The rule's English advice — carried WITH the compiled detector so rendering a finding
    /// never re-reads the multi-megabyte learned catalogs it came from.
    #[serde(default)]
    description: String,
    /// Where the rule came from (doc URL or rule-file path) — the finding's citation.
    #[serde(default)]
    source: String,
    kind: MatchKind,
}

/// A language's compiled rule set: every documented rule reduced to its lossless tree pattern. This
/// is the cached, serializable model a lint run loads and matches each file against — deterministic,
/// no thresholds, no statistics. Mirrors the engine's old model API so judging code is unchanged.
#[derive(Serialize, Deserialize)]
pub struct RuleSet {
    /// Language id (e.g. `rust`).
    pub lang: String,
    rules: Vec<CompiledRule>,
    /// Every rule the compile DROPPED, with the gate that dropped it — `(id, named reason)` (PASS 31,
    /// the conservation ledger). Nothing may vanish silently: the train-time invariant reads this to
    /// prove every PROVEN rule is either compiled or withheld for a NAMED, accepted reason, and
    /// `lint_query kind=rules` surfaces it. Persisted with the module (`serde(default)` keeps old
    /// modules decodable as an empty ledger).
    #[serde(default)]
    withheld: Vec<(String, String)>,
    /// Single-pass firing index ([`Batch`]) — derived from `rules`, built lazily once per loaded
    /// model and never serialized.
    #[serde(skip)]
    batch: std::sync::OnceLock<Batch>,
}

/// The rule set regrouped for single-pass firing: AST patterns indexed by the one node kind
/// their root can match, token detectors grouped by firing universe. Pure derivation of the
/// rules — same matches, same lines — paying one tree walk and one pass over each firing
/// universe's lines per file instead of one per rule (the warm run's dominant cost).
struct Batch {
    /// Rule index by AST-pattern root kind: a node tries only the patterns that can match it.
    ast_by_kind: HashMap<String, Vec<usize>>,
    /// Token detectors over the code surface (raw text for prose languages).
    surface: Vec<usize>,
    /// Raw-universe token detectors (project law whose construct lives in comments/strings).
    raw: Vec<usize>,
    /// Structural probe rules — each fires by walking the parsed tree ([`MatchKind::Probe`]).
    probes: Vec<usize>,
}

/// One flagged violation: the rule it violates, that rule's severity, and the 1-based source line.
pub struct Finding {
    /// The matched rule's id.
    pub rule: String,
    /// Severity bucket (`high`/`medium`/`low`).
    pub severity: String,
    /// 1-based source line of the match.
    pub line: usize,
    /// True when the match is a lossless AST pattern with an exact-text anchor (reported directly);
    /// false for token-detector fallbacks and container-only AST patterns, which the live path
    /// confirms through the Hv concept gate first.
    pub precise: bool,
}

impl RuleSet {
    /// Compile a language's documented `(id, severity, bad, good, description)` rules.
    ///
    /// For languages with a tree-sitter grammar: lossless AST patterns via `bad ∧ ¬good`.
    /// For any other language: discriminating token sequences, derived the same way.
    /// Both paths apply the same quality gate: self-fire (must flag its own `bad`) and
    /// over-fire (must not flag any `good` in the corpus). Only rules that pass both survive.
    /// `ground` is the learned evidence prose-only rules are read through ([`Grounding`]);
    /// pass `Grounding::default()` when no docs have been read for the language yet.
    /// Each rule is `(id, severity, bad, good, description, source, construct)`. The final
    /// `construct` is `Some(c)` only for a GRADUATED construct-module rule that carries its own
    /// proven plan (`native/architecture.dx`, "The modular rebuild"): it compiles DIRECTLY to
    /// `uses_construct(c)` and fires that plan in the one walk, never re-derived from the example
    /// diff. `None` keeps the legacy example/token detector path — behavioral scope, no language
    /// named (a rule that HAS a plan fires its plan; one that has only bad/good keeps the old path).
    pub fn build(
        lang: &str,
        rules: &[(String, String, String, String, String, String, Option<String>)],
        ground: &Grounding,
    ) -> RuleSet {
        let trusted = &ground.trusted;
        let reference_corpus = &ground.reference;
        let reality_flagged = ground.flagged.clone();
        let ground = GroundView::of(lang, ground);
        // Ledger #19 — provenance for example-diff tokens: a text detector's literal tokens
        // are evidence only when the toolchain actually FLAGGED the example, or the law's own
        // words name them. A Clean-parsing (or ungroundable) example's identifiers are just
        // code the docs showed — `proc greet` once compiled from nim tutorial narration and
        // fired on every greeter a user ever wrote.
        let traceable = |desc: &str, bad: &str, tokens: &[String]| -> bool {
            reality_flagged.contains(&crate::lint_ai::token_seed(bad))
                // ANY kept token named by the law anchors the detector: an ordered pair's
                // partner token only NARROWS firing (both tokens must share a line), so an
                // unnamed partner cannot broaden a named construct — but a detector with no
                // named token at all is untraceable (`goto cleanup` keeps its pair through
                // "goto"; nim's `proc greet` names neither and dies).
                || tokens.iter().any(|t| select::tokens_fire_text(desc, std::slice::from_ref(t)))
        };
        // OVER-GENERAL SINGLE-TOKEN GUARD (native/architecture.dx, "Entry gates"; the junk-doc-rule FP class).
        // A descriptive REFERENCE section that states no prohibition can still leak a firing
        // single-token detector on an ordinary language keyword or built-in type — a Rust
        // paths.html syntax section compiled `["usize"]` and fired on every `usize`, exactly as a
        // `use`-declaration section compiled `["use"]`. The construct a rule points at is the word
        // the language does NOT use everywhere (`goto`, `panic!`); a token that is either common
        // English (`use`, `match`) OR ubiquitous in the LANGUAGE'S OWN normal code (`usize`, `as`,
        // `HashMap`) is over-general and cannot mark a violation. Both signals are LEARNED — the
        // dictionary's commonness curve and the language's own reference corpus — never an
        // enumerated keyword list (a covenant offense). Learned rules only; project law is trusted.
        let reference_lines: Vec<&str> =
            reference_corpus.iter().flat_map(|e| e.lines()).map(str::trim).filter(|l| !l.is_empty()).collect();
        // The two named over-generality ledger reasons: the plain ubiquitous-token drop, and the
        // PASS-36 contextual refinement (the census's stable "over-general single token" core is
        // the shared prefix, so ledger joins survive the refinement).
        const OVER_GENERAL: &str =
            "over-general single token (a ubiquitous language keyword/type, not a construct a rule points at)";
        const CONTEXTUAL: &str =
            "over-general single token (the docs' own remedy still uses it — contextual)";
        // PASS 36 — the miner-path MEMBER veto's named ledger reason (the graduation path's
        // member veto shares the "member veto" stage prefix, so census/ledger joins line up).
        const MEMBER_VETO: &str =
            "member veto (bare shape; the corpus's own typography is dotted)";
        // Is `token` over-general on its own evidence — a word the LANGUAGE uses everywhere?
        // Returns the named ledger reason when it is, `None` when the token can anchor a detector.
        // `good` is the rule's good example: a real bad∧good CONTRAST (the good form lacks the
        // token) is what tells a legitimately-banned keyword (`var`, contrasted against `let`)
        // apart from a syntax keyword/type a reference section merely mentions (`use`, `usize`,
        // with no good counterpart). Signals, all LEARNED — never an enumerated keyword list:
        //   * common English (`use`, `match`);
        //   * ubiquitous in the language's own reference code — read through the GOOD-CONTRAST
        //     discriminator (the remedy-demonstration doctrine of
        //     [`crate::lint_module::is_prohibited_subject`]): a good example that DROPS the token
        //     is the docs' own remedy demonstrating the construct's absence, so corpus ubiquity
        //     alone cannot veto it (`var` is taught using `var`); a good that still USES the
        //     token demonstrates acceptable uses — a CONTEXTUAL rule, withheld under its own
        //     named reason; no good example at all keeps the plain ubiquity drop;
        //   * the grammar's OWN classification as a keyword/operator or built-in primitive type,
        //     UNLESS a good example contrasts it (then the keyword really is the banned construct).
        let over_general_token = |token: &str, good: &str| -> Option<&'static str> {
            if crate::lint_english::brain().is_some_and(|e| e.is_common(token)) {
                return Some(OVER_GENERAL);
            }
            // MEMBER-TYPOGRAPHY VETO (PASS 36 — the miner-path analogue of the graduation's
            // member veto): the reference corpus's own code addresses `token` with dotted OWNER
            // typography (`Gadget.grip(handle)`), so the bare token names arbitrary user
            // identifiers (any owner's own `grip = …`) and a bare single-token detector cannot
            // mark a violation. Learned from the corpus's own typography, never a shape list;
            // a dotted detector on the qualified form remains enforceable.
            if reference_lines.iter().any(|l| dotted_owner_typography(l, token)) {
                return Some(MEMBER_VETO);
            }
            let denom = reference_lines.len();
            if denom >= 8 {
                let needle = [token.to_string()];
                let hits = reference_lines.iter().filter(|l| select::tokens_fire_line(l, &needle)).count();
                if hits * 100 >= denom * 12 {
                    if good.trim().is_empty() {
                        return Some(OVER_GENERAL);
                    }
                    if select::tokens_fire_text(good, &needle) {
                        return Some(CONTEXTUAL);
                    }
                    // The good form drops the token: a remedy demonstration, not normal use —
                    // fall through to the grammar-role reading.
                }
            }
            match grammar::token_role(lang, token) {
                // A built-in PRIMITIVE TYPE (`usize`, `u32`, `bool`) is essential syntax present
                // in nearly every file and is never a bannable single-token construct — a
                // reference section that mentions it (paths.html's `usize`) must never fire.
                Some("primitive_type") => Some(OVER_GENERAL),
                // A KEYWORD (`var`, `use`) is over-general UNLESS a good example contrasts it: a
                // real bad∧good pair (`var count` vs `let count`) proves the keyword itself is the
                // banned construct, while a reference mention with no counterpart (`use` syntax)
                // is not.
                Some("keyword") => {
                    let contrasted = !good.trim().is_empty()
                        && !select::tokens_fire_text(good, std::slice::from_ref(&token.to_string()));
                    (!contrasted).then_some(OVER_GENERAL)
                }
                _ => None,
            }
        };
        let over_general = |tokens: &[String], good: &str| -> Option<&'static str> {
            match tokens {
                [only] => over_general_token(only, good),
                _ => None,
            }
        };
        let mut compiled = Vec::new();
        let mut seen = HashSet::new();
        let has_grammar = language(lang).is_some();
        // Build-time observability (`HELPERS_LINT_TRACE`): name every rule a compile gate
        // drops and the gate that dropped it — the build-side counterpart of "Your law, as
        // understood". Rules must never vanish undebuggably.
        let trace = std::env::var_os("HELPERS_LINT_TRACE").is_some();
        // PASS 31 — the conservation ledger: EVERY drop is recorded `(id, named gate)`, not only
        // traced. `RefCell` because the retain-closure gates below call this from immutable contexts.
        let withheld_log: std::cell::RefCell<Vec<(String, String)>> = std::cell::RefCell::new(Vec::new());
        let dropped = |id: &str, gate: &str| {
            if trace {
                eprintln!("[lint-build {lang}] {id} dropped: {gate}");
            }
            withheld_log.borrow_mut().push((id.to_string(), gate.to_string()));
        };
        // Ids of graduated rules that compiled their own PROVEN plan — exempt from the statistical
        // reference-fire gate below. That gate is a heuristic for UNproven example-diff detectors
        // (a semantic rule that tree-diffs to a ubiquitous construct); a graduated `uses_construct`
        // rule was already proven through the frozen self-generated loop over the docs' OWN corpus,
        // and its target is legacy-ubiquitous BY DESIGN (`var` is taught using `var`), so the corpus
        // fire-rate must not veto it (`native/architecture.dx`, "The modular rebuild").
        let mut plan_rule_ids: HashSet<String> = HashSet::new();
        // Ids whose detector was REWRITTEN to the author's dotted member typography (owner
        // ruling 2026-07-18, second): the rule's own bad example demonstrates the BARE shape a
        // member page teaches with, which the dotted detector rightly does not fire — its
        // self-fire witness is the reference-corpus line the chain was read from, so the
        // bare-example self-fire gate below exempts exactly these ids (over-fire and
        // reference-fire still run un-exempted).
        let mut member_dotted: HashSet<String> = HashSet::new();
        for (id, severity, bad, good, desc, source, construct) in rules {
            if id.is_empty() || !seen.insert(id.clone()) {
                if !id.is_empty() {
                    dropped(id, "duplicate id (first occurrence wins)");
                }
                continue;
            }
            // bad may be empty when the documentation only provides prose (description-only
            // rules). description_discriminator will read the English doc to derive a pattern;
            // the SELF-FIRE gate below will then validate or drop it.
            if desc.trim().is_empty() && bad.trim().is_empty() {
                dropped(id, "empty (no description and no example — nothing to learn from)");
                continue;
            }
            // Read the description's polarity ALONG the text — each word's context is the
            // nearest decisive lean, no chopping, no punctuation ([`GroundView::word_contexts`]).
            let contexts = ground.word_contexts(desc);
            // The entry ticket for every LEARNED rule, example-backed or not: some SENTENCE of
            // its description must classify as a prohibition under the information-weighted
            // span classifier. The sentence is the verdict unit (ledger #6: never the mixed
            // span; ledger #13: never a single word — one mis-leaning token in a tutorial
            // paragraph must not admit the paragraph, which is how go.dev narration and MDN
            // error-page remedy prose once minted rules that fired on hello-world code).
            // Project rules are law by location and skip the reading; with no ready classifier
            // the question is unanswerable and the author's material is trusted as before.
            let classifier_ready = ground.polarity.is_some_and(|p| p.is_ready());
            // PASS 31 — THE COLLAPSE (owner ruling: understanding drives linting; presentation never
            // vetoes proven law). A rescued rule's description is re-rendered from the FACT below.
            let mut fact_rendered: Option<String> = None;
            if !trusted.contains(id) && classifier_ready {
                let states_violation = ground
                    .polarity
                    .is_some_and(|p| {
                        crate::lint_read::sentences(desc).iter().any(|s| p.classify(s) == Some(true))
                    });
                if !states_violation {
                    // The entry gate judged only the DISPLAY SENTENCE — for a GRADUATED construct rule
                    // the proof is the blind loop's, not the sentence's (MEASURED: the selector's
                    // overt-negator preference stapled MDN's XML/XHTML trivia footnote to
                    // `document.write` and this gate silently unenforced a proven deprecation). Rescue
                    // the rule iff its firing SHAPE is flood-safe: a dotted qualified chain is
                    // inherently narrow; a BARE token is safe only when it is NOT an ordinary English
                    // word by the dictionary's own knowledge (`clear` names arbitrary user identifiers
                    // — every `map.clear()` — and stays withheld with its reason named, while the
                    // jargon compound `createNSResolver` collides with nothing) AND it passes the same
                    // learned over-generality read as any single-token detector. Without a brain a
                    // bare shape cannot be certified flood-safe and is withheld (honest abstention).
                    let bare_safe = |c: &str| {
                        crate::lint_char::brain().is_some_and(|b| {
                            b.meanings().definition_words(&c.to_lowercase()).is_none()
                                && over_general_token(c, good).is_none()
                        })
                    };
                    let shape_safe =
                        construct.as_deref().is_some_and(|c| c.contains('.') || bare_safe(c));
                    if !shape_safe {
                        dropped(
                            id,
                            if construct.is_some() {
                                "entry gate (proven fact withheld: flood-unsafe bare shape)"
                            } else {
                                "entry gate (no sentence classifies as a prohibition)"
                            },
                        );
                        continue;
                    }
                    // Presentation derives from knowledge: the enforcement message is the fact and its
                    // citation (the graded tier's honest register), never the mis-selected sentence.
                    fact_rendered = construct
                        .as_deref()
                        .map(|c| format!("Do not use `{c}`: documented deprecated ⟨{source}⟩."));
                }
            }
            let desc: &String = fact_rendered.as_ref().unwrap_or(desc);
            // A description-derived detector exists only for prose that STATES a violation:
            // project law states one by LOCATION (the user wrote it in a rule file — that is
            // the label), learned prose by the classifier's reading. Within the description,
            // prohibition-context words outrank all others and remedy-context words are never
            // eligible ("…; use the logging module instead" must never compile `logging`) —
            // that is what keeps English understanding from being confused for a lintable
            // code language. Learned rules additionally require the construct to exist in
            // real documented code (`only_grounded`); the project's own law does not.
            let desc_detector = |view: &GroundView| -> Option<(String, bool)> {
                description_discriminator(desc, bad, good, view, &contexts, !trusted.contains(id))
            };
            // PASS 36 — the CONTEXTUAL token: the description names a construct that fires
            // the bad example AND the docs' own good example. Selection's bad∧¬good validation
            // rightly refuses such a token as a detector (a token firing the documented fix cannot
            // mark a violation), but that refusal used to fall through as an anonymous
            // "no detector" row. The remedy-demonstration doctrine
            // ([`crate::lint_module::is_prohibited_subject`]) reads it as a CONTEXTUAL rule — the
            // docs demonstrate the construct's acceptable uses, not a replacement. Asked only
            // AFTER every detector path abstained (the good example set aside for the re-ask), so
            // selection's ranking is undisturbed. Returns the refused token — the anchor the
            // demonstrated-shape escape hatch (owner rulings 2026-07-18, second + third) narrows
            // from; when no shape exists the drop carries its own named ledger reason.
            let contextual_subject = |view: &GroundView| -> Option<String> {
                if good.trim().is_empty() {
                    return None;
                }
                description_discriminator(desc, bad, "", view, &contexts, !trusted.contains(id))
                    .map(|(t, _)| t)
                    .filter(|t| select::tokens_fire_text(good, std::slice::from_ref(t)))
            };
            // The demonstrated-shape compile for a refused contextual/over-general token
            // ([`select::demonstrated_shape`]): the docs' own bad/good contrast narrows the
            // token to the anchored diff — the rule IS the shape the docs demonstrate.
            let shaped = |token: &str| -> Option<MatchKind> {
                select::demonstrated_shape(lang, token, bad, good)
                    .map(|(tokens, raw)| MatchKind::Tokens { tokens, raw })
            };
            // UNDERSTANDING first (native/architecture.dx, "Rules from understanding — the probe bridge"): a
            // machine-global CS-principles doc (`corpus/*.md`) that describes a defect class in
            // prose, with no in-language example, is bound to the STRUCTURAL PROBE whose concept
            // its prose means. This is where reading a principle turns into a check: no probe
            // binds ⇒ there is nothing structural to enforce, and a general principle must not
            // fall back to a token detector on its English words (that was the net-negative noise
            // — a rule watching `command`, a document title firing as law).
            // A machine-global canon principle (`corpus/*.md`) is ALWAYS understanding-routed,
            // even when the doc bundles an illustration example: the canon states language-agnostic
            // design law, and its Java/`// Bad` snippet is a teaching illustration, NOT a token
            // detector source. Requiring `bad.is_empty()` here was the diversion bug — a principle
            // like "6. Never Swallow Exceptions" (illustrated with a Java try/catch) failed the
            // empty-bad test, skipped understanding, and compiled a per-language token detector off
            // the illustration (junk that fired on innocent lines). A canon principle enforces
            // through the trace bridge or abstains; it never becomes a token/example detector.
            let is_corpus_principle = source.contains("/corpus/");
            // UNDERSTANDING→TRACE first (the rule IS the understanding): read the principle's prose
            // into a composition of generic primitives ([`crate::lint_trace`]). Only when the bridge
            // ABSTAINS does the committed per-principle probe fallback get a turn (run alongside
            // until the live anti-cheat passes; native/architecture.dx).
            // The understanding→trace bridge is the SOLE canon path — the brain always runs.
            // Its abstention is MEANINGFUL: the principle maps to no structural shape, so the rule
            // DROPS. The `lint_probe` spelling-centroid fallback is retired (native/architecture.dx end-state):
            // it bound principles by SHARED SPELLING, so canon prose the bridge correctly abstains
            // on ("Comments explain why not what", "Match the algorithm family…") spuriously
            // spell-matched the `duplicated_code` probe and fired on innocent repeated lines. The
            // bridge binds 10/10 of the probe-mechanism fixture through MEANING, so nothing is lost:
            // a canon principle enforces through understanding or drops — never a spelling match.
            // BOTH origins now read through the SAME understanding→trace bridge (the token-miner is
            // retired for modules — native/architecture.dx, "The per-language training pipeline"). A corpus
            // principle reads the language-AGNOSTIC canon (structural primitives only); a language-doc
            // rule reads the GENERAL scope, where a prohibition naming a construct shapes
            // `uses_construct` ("avoid the `with` statement" → uses_construct(with)) — understood from
            // the PROSE, no bad/good example snippet required. Understanding-first means a real rule
            // comes from what the docs MEAN, not from a token diff scraped off an illustration.
            // GRADUATED MODULE RULE first (the rule IS its understood prohibition): a rule that
            // carries a `construct` was proven through the frozen self-generated loop to be
            // `uses_construct(construct)`, so it compiles to that plan DIRECTLY and fires the exact
            // shape the loop proved — never a detector re-derived from the bad/good example diff
            // (which the desc/AST path compiles more conservatively: `eval` became the non-firing
            // `uses_construct(eval())`, `var`/`==` were dropped, `center`/`font`/`page-break-after`
            // became AST patterns that miss real usages). The plan needs an AST to walk, so a
            // grammarless language falls through (its `run_plan` would silently yield nothing).
            let graduated_plan = construct
                .as_deref()
                .filter(|c| !c.is_empty() && has_grammar)
                .map(|c| crate::lint_trace::Plan::UsesConstruct { construct: c.to_string() });
            let bound_trace = if let Some(plan) = graduated_plan {
                plan_rule_ids.insert(id.clone());
                Some(plan)
            } else if is_corpus_principle {
                crate::lint_trace::understand_canon(desc)
            } else if !has_grammar {
                // No bundled grammar ⇒ no AST to trace over. An understanding Plan (even
                // `uses_construct`) is fired by `run_plan`, which needs a tree; without one every
                // trace silently yields nothing. So a grammarless language's doc rule is NOT routed
                // through understanding — it falls to the token detector below, which reads the raw
                // text. (Routing it through understanding unconditionally was the regression that
                // left "Never use the goto statement" a non-firing trace on grammarless flowlang.)
                None
            } else if !bad.trim().is_empty() && !good.trim().is_empty() {
                // PROPOSE-VERIFY-LEARN (native/architecture.dx, "PROPOSE-VERIFY-LEARN is the language path"):
                // understanding PROPOSES candidate checks from the prose; the binding's OWN paired
                // examples PROVE them (fire on `bad`, stay clean on `good`); only a PROVEN plan is
                // learned and remembered ([`lint_trace::learn_verified`]). Verification — not the
                // low-recall positional-negation gate — reaches a real rule the docs phrase as
                // "avoid `eval`" or "`with` is deprecated" (which the gate never fires on), proven
                // against the docs' own example pair. When neither verification nor the gated
                // `understand` shapes a plan, the token/AST detector below carries it (a rule the
                // reader knows from examples but understanding cannot yet express structurally).
                crate::lint_trace::learn_verified(desc, lang, bad, good)
                    .or_else(|| crate::lint_trace::understand(desc))
            } else {
                crate::lint_trace::understand(desc)
            };
            if is_corpus_principle && bound_trace.is_none() {
                dropped(id, "corpus principle: understanding abstains (nothing structural to enforce)");
                continue;
            }
            let kind = if let Some(plan) = bound_trace {
                MatchKind::Trace(plan)
            } else if has_grammar {
                if let Some(pat) = RulePattern::compile(lang, bad, good, desc) {
                    // AST pattern — lossless and most precise.
                    MatchKind::Ast(pat)
                } else if let Some((token, raw)) = desc_detector(&ground) {
                    // English prose is the primary documentation; read it first.
                    // The description names the construct to flag: "avoid `e.printStackTrace()`".
                    MatchKind::Tokens { tokens: vec![token], raw }
                } else if let Some(tokens) = text_discriminator(bad, good) {
                    // Code-diff fallback: description had no extractable term but the bad/good
                    // examples (themselves part of the official documentation) still distinguish.
                    if !trusted.contains(id) && classifier_ready && !traceable(desc, bad, &tokens) {
                        dropped(id, "untraceable example tokens (never reality-flagged; not named by the law's words)");
                        continue;
                    }
                    MatchKind::Tokens { tokens, raw: false }
                } else if let Some(token) = contextual_subject(&ground) {
                    // Demonstrated-shape escape hatch first; a shapeless contextual token keeps
                    // its named ledger drop.
                    match shaped(&token) {
                        Some(kind) => kind,
                        None => {
                            dropped(id, CONTEXTUAL);
                            continue;
                        }
                    }
                } else {
                    dropped(id, "no detector (AST abstained; no groundable word; no token diff)");
                    continue;
                }
            } else {
                // No grammar — token matching only. Documentation prose is the primary signal;
                // code examples (which appear in the same docs) refine when prose is thin.
                // A CONSTRUCT-carrying rule compiles its construct as the one containment token
                // (PASS 36): `uses_construct` plans need an AST to walk, and the construct is the
                // reader's own proven target — never re-derived from prose or an example diff.
                // The over-general/member-veto guard below still runs on it, un-exempted.
                // PASS 37: a `host@attr` attribute construct compiles to the tag-scoped ordered
                // pair `[<host, attr]` ([`construct_tokens`] — both halves inside one tag open).
                if let Some(c) = construct.as_deref().filter(|c| !c.is_empty()) {
                    MatchKind::Tokens { tokens: construct_tokens(c), raw: false }
                } else if let Some((token, raw)) = desc_detector(&ground) {
                    MatchKind::Tokens { tokens: vec![token], raw }
                } else if let Some(tokens) = text_discriminator(bad, good) {
                    if !trusted.contains(id) && classifier_ready && !traceable(desc, bad, &tokens) {
                        dropped(id, "untraceable example tokens (never reality-flagged; not named by the law's words)");
                        continue;
                    }
                    MatchKind::Tokens { tokens, raw: false }
                } else if let Some(token) = contextual_subject(&ground) {
                    // Demonstrated-shape escape hatch first; a shapeless contextual token keeps
                    // its named ledger drop.
                    match shaped(&token) {
                        Some(kind) => kind,
                        None => {
                            dropped(id, CONTEXTUAL);
                            continue;
                        }
                    }
                } else {
                    dropped(id, "no detector (no groundable word; no token diff)");
                    continue;
                }
            };
            // The over-general single-token guard runs on the FINAL detector, whichever path
            // produced it (description or example diff): a learned rule whose whole detector is a
            // ubiquitous keyword/type fires on normal code everywhere and marks no violation.
            // A refusal is not the end (owner rulings 2026-07-18, second + third): a
            // member-vetoed bare token REWRITES to the author's own dotted typography (the
            // qualified form the veto itself declared enforceable), and any other refused token
            // narrows to its demonstrated shape when the docs' bad/good contrast carries one —
            // only a shapeless refusal keeps its named ledger drop.
            let mut kind = kind;
            if !trusted.contains(id) {
                if let MatchKind::Tokens { tokens, raw } = &mut kind {
                    if let Some(reason) = over_general(tokens, good) {
                        if reason == MEMBER_VETO {
                            match reference_lines.iter().find_map(|l| dotted_owner_chain(l, &tokens[0])) {
                                Some(dotted) => {
                                    // The corpus line the chain was read from is this rule's
                                    // self-fire witness: the member page's own bare example
                                    // cannot contain the dotted form ([`member_dotted`]).
                                    member_dotted.insert(id.clone());
                                    *tokens = vec![dotted];
                                }
                                None => {
                                    dropped(id, reason);
                                    continue;
                                }
                            }
                        } else {
                            match select::demonstrated_shape(lang, &tokens[0], bad, good) {
                                Some((shape, shape_raw)) => {
                                    *tokens = shape;
                                    *raw = shape_raw;
                                }
                                None => {
                                    dropped(id, reason);
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
            compiled.push(CompiledRule {
                id: id.clone(),
                severity: severity.clone(),
                // A rule's advice is shown to agents, and doc/registry rules come from prose
                // the machine did not author — so it is BOUNDED here, once, into the shared
                // artifact: control characters stripped (no ANSI escapes, no zero-width or
                // line-break injection into the report) and length-capped. Project law is the
                // user's own text and passes through untouched. Bounding, not proving safe —
                // the residual is the agent's own hygiene (native/architecture.dx, distribution channel).
                description: if trusted.contains(id) { desc.clone() } else { sanitize_advice(desc) },
                source: source.clone(),
                kind,
            });
        }
        // SELF-FIRE: when a bad example is known, the compiled rule must flag it.
        // Description-only rules (bad is empty) skip this gate — they are validated at
        // query time: if the extracted pattern fires on real violations found in the project,
        // it was correct; if nothing matches, it stays silent (never a false flag).
        // Both gates run BEFORE pattern dedup so an invalid rule can never claim a pattern
        // signature and knock out the valid rule that shares it (`seen` above keeps the maps
        // first-wins for duplicate ids, matching which rule actually compiled).
        let mut bad_map: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        let mut good_map: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        for (id, _, bad, good, _, _, _) in rules {
            bad_map.entry(id.as_str()).or_insert(bad.as_str());
            good_map.entry(id.as_str()).or_insert(good.trim());
        }
        // Gates run in the same text universe firing uses: a code-universe text detector sees
        // the example's code surface; raw-universe detectors and AST patterns see the raw
        // example (`text_input`, below).
        let mask = !prose_lang(lang);
        let text_input = |r: &CompiledRule, example: &str| -> String {
            match (&r.kind, mask) {
                (MatchKind::Tokens { raw: false, .. }, true) => code_surface_file(lang, example),
                _ => example.to_string(),
            }
        };
        compiled.retain(|r| {
            let bad = bad_map.get(r.id.as_str()).copied().unwrap_or("").trim();
            // No bad example → description-only rule; let it through without the SELF-FIRE
            // check. A member-typography rewrite is exempt by construction: its bad example is
            // the member page's BARE demonstration, and its self-fire witness is the corpus
            // line its dotted chain was read from ([`member_dotted`]).
            let keep = bad.is_empty()
                || member_dotted.contains(&r.id)
                || !r.kind.matches(lang, &text_input(r, bad)).is_empty();
            if !keep {
                dropped(&r.id, "self-fire (detector misses the rule's own bad example)");
            }
            keep
        });
        // OVER-FIRE: must not flag THIS rule's own `good` example (if it has one).
        compiled.retain(|r| {
            let good = good_map.get(r.id.as_str()).copied().unwrap_or("");
            let keep = good.is_empty() || r.kind.matches(lang, &text_input(r, good)).is_empty();
            if !keep {
                dropped(&r.id, "over-fire (detector flags the rule's own good example)");
            }
            keep
        });
        // REFERENCE-FIRE: a violation detector must stay quiet on the language's own
        // documented-NORMAL code. A rule whose real meaning is semantic (borrow usage, operand
        // nullness) tree-diffs down to a ubiquitous construct — "any `&mut` parameter", the bare
        // `null` literal — and would flag idiomatic code everywhere; running every compiled
        // detector over the reference corpus once at compile time drops exactly those. The bar
        // is two-tier by how much the detector's own shape vouches for it: a structured pattern
        // (depth ≥ 2 with an exact anchor) gets quarantine's 1% bar; a degenerate one (leaf
        // pattern, all-wildcard shape, or any single-token detector) has only the corpus as
        // witness and gets 0.1% — a genuinely banned construct (`goto`) is near-absent from
        // normal examples and passes, a pervasive one (`null`) cannot mark violations and dies.
        // Statistical, so it needs scale ([`REFERENCE_FIRE_MIN_LINES`]); project law is exempt
        // by location. Runs before dedup for the same reason the other gates do: an
        // over-general rule must not claim a pattern signature it cannot keep.
        let ref_lines: usize = reference_corpus.iter().map(|e| e.lines().count()).sum();
        if ref_lines >= REFERENCE_FIRE_MIN_LINES {
            let probe = RuleSet { lang: lang.to_string(), rules: compiled, withheld: Vec::new(), batch: Default::default() };
            let mut fired: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for example in reference_corpus {
                for f in probe.flag(example) {
                    *fired.entry(f.rule).or_default() += 1;
                }
            }
            compiled = probe.rules;
            compiled.retain(|r| {
                let bar = match &r.kind {
                    MatchKind::Ast(p) if p.structured() => ref_lines / 100,
                    _ => ref_lines / 1000,
                };
                let hits = fired.get(&r.id).copied().unwrap_or(0);
                // A graduated construct-module rule is exempt: it was proven over the docs' own
                // corpus and deliberately bans a legacy-ubiquitous construct, so the corpus
                // fire-rate is not evidence against it.
                let keep = trusted.contains(&r.id) || plan_rule_ids.contains(&r.id) || hits <= bar;
                if !keep {
                    dropped(&r.id, &format!("reference-fire ({hits} hits on {ref_lines} normal lines, bar {bar})"));
                }
                keep
            });
        }
        // Dedup identical compiled patterns: noisy docs pages often yield several rule entries
        // that compile to the same pattern (the same wiki page scraped under multiple slugs).
        // One pattern = one rule; keep the first id — the caller orders rules by trust
        // (project > corpus folder > crawled docs), so the most trusted rule wins its pattern.
        let mut seen_patterns = HashSet::new();
        compiled.retain(|r| {
            let keep = seen_patterns.insert(serde_json::to_string(&r.kind).unwrap_or_default());
            if !keep {
                dropped(&r.id, "duplicate compiled pattern (an identical detector already enforces)");
            }
            keep
        });
        RuleSet {
            lang: lang.to_string(),
            rules: compiled,
            withheld: withheld_log.into_inner(),
            batch: Default::default(),
        }
    }

    /// The conservation ledger (PASS 31): every rule this compile dropped, with the gate that dropped
    /// it. The train-time invariant and `lint_query kind=rules` read this — a rule may be withheld,
    /// never vanished.
    pub fn withheld(&self) -> &[(String, String)] {
        &self.withheld
    }

    /// Append a PRE-COMPILE withhold row to the conservation ledger (PASS 36 — the recall census).
    /// The read/mint/veto stages that refuse a fact BEFORE the compile funnel their named
    /// `(id, stage-prefixed reason)` records into this SAME ledger after [`RuleSet::build`], so
    /// `lint_query kind=rules` surfaces every stage's refusals through one surface — no second
    /// ledger to drift. Rows are deduped by the `(id, reason)` pair; compile-stage rows always
    /// precede appended ones, so the train-time invariant's first-wins read is unaffected.
    pub(crate) fn note_withheld(&mut self, id: &str, reason: &str) {
        if !self.withheld.iter().any(|(i, r)| i == id && r == reason) {
            self.withheld.push((id.to_string(), reason.to_string()));
        }
    }

    /// Number of compiled rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// A queryable summary of every compiled rule — `(id, severity, description, detector)` — for
    /// the `lint_query rules` interrogation. `detector` names the understanding/detector behind the
    /// rule: for a corpus principle, the plan understanding SHAPED (`understanding → …`); otherwise
    /// the pattern/token/probe it compiled to.
    pub fn rule_details(&self) -> Vec<(String, String, String, String)> {
        self.rules
            .iter()
            .map(|r| {
                let detector = match &r.kind {
                    MatchKind::Trace(plan) => format!("understanding → {}", plan.describe()),
                    MatchKind::Probe(name) => format!("probe fallback ({name})"),
                    MatchKind::Ast(_) => "AST pattern".to_string(),
                    MatchKind::Tokens { tokens, .. } => format!("tokens `{}`", tokens.join(" … ")),
                };
                (r.id.clone(), r.severity.clone(), r.description.clone(), detector)
            })
            .collect()
    }

    /// The ids of the rules that actually compiled a detector — the honest answer to "which of
    /// the laws I wrote can you enforce?". A caller compares this against what it asked for and
    /// REPORTS the difference; law must never vanish silently.
    pub fn rule_ids(&self) -> impl Iterator<Item = &str> {
        self.rules.iter().map(|r| r.id.as_str())
    }

    /// A compiled rule's reporting facts: `(severity, description, source)`. The model is the
    /// single source of truth for what it enforces — no catalog re-read at render time.
    pub fn info_of(&self, id: &str) -> Option<(&str, &str, &str)> {
        self.rules
            .iter()
            .find(|r| r.id == id)
            .map(|r| (r.severity.as_str(), r.description.as_str(), r.source.as_str()))
    }

    /// What a rule's detector actually watches for — the honest answer to "what did you
    /// understand my law as?". A text rule shows its literal pattern; an AST rule is a
    /// structural match compiled from the rule's own examples. `None` when the rule did not
    /// compile. Surfacing this lets the author correct a mis-read law by rephrasing it,
    /// instead of discovering the misunderstanding through missing findings.
    pub fn detector_of(&self, id: &str) -> Option<String> {
        self.rules.iter().find(|r| r.id == id).map(|r| match &r.kind {
            MatchKind::Ast(_) => "structural pattern from your example".to_string(),
            MatchKind::Tokens { tokens, raw: true } => format!(
                "`{}` (in comments and strings too — that is where it lives in this project)",
                tokens.join(" … ")
            ),
            MatchKind::Tokens { tokens, .. } => format!("`{}`", tokens.join(" … ")),
            MatchKind::Probe(name) => format!("a structural probe understood from the principle ({name})"),
            MatchKind::Trace(plan) => format!("understanding traced from the principle ({})", plan.describe()),
        })
    }

    /// Whether `id` compiled to a structural [`MatchKind::Probe`] — the caller skips building a
    /// concept fingerprint for it (a probe is precise and reports directly; a description-only
    /// concept would only risk vetoing OTHER rules' true findings, native/architecture.dx "Hv concept gate").
    pub fn is_probe(&self, id: &str) -> bool {
        self.rules
            .iter()
            .find(|r| r.id == id)
            .is_some_and(|r| matches!(r.kind, MatchKind::Probe(_) | MatchKind::Trace(_)))
    }

    /// Whether `id`'s detector carries no discriminating structure of its own — a single-token
    /// token detector or an AST pattern that is a bare leaf / all-wildcard shape. The same two-tier
    /// classification the compile-time reference-fire gate uses; the live quarantine holds
    /// degenerate detectors to the stricter fire-rate bar (a reference corpus can be too small
    /// to witness a token that is rare in doc examples but pervasive in real projects — `path`
    /// passed compile on 8 corpus lines and then fired 305× on one repo).
    pub fn degenerate_detector(&self, id: &str) -> bool {
        self.rules.iter().find(|r| r.id == id).is_some_and(|r| match &r.kind {
            MatchKind::Ast(p) => !p.structured(),
            MatchKind::Tokens { tokens, .. } => tokens.len() < 2,
            // A probe/trace is a structured predicate, not a bare token — it earns the lenient
            // quarantine tier (1%), so a lightly-violated principle is still reported.
            MatchKind::Probe(_) | MatchKind::Trace(_) => false,
        })
    }

    /// Flag `code`: every line where a rule fires (AST match or token match), deduped per rule.
    /// Each finding carries `precise` so the caller can confirm the imprecise ones. Imprecise:
    /// token detectors, and AST patterns whose only identity is a container kind — several
    /// distinct rules can compile to the same bare container, so the concept gate must arbitrate.
    pub fn flag(&self, code: &str) -> Vec<Finding> {
        let batch = self.batch();
        let mut lines_per_rule: Vec<Vec<usize>> = vec![Vec::new(); self.rules.len()];
        // ONE parse and ONE walk per file, and the walk yields everything the run needs from
        // the tree: each node tries only the AST patterns whose root kind is that node's kind,
        // and the same walk collects the English-bearing spans that blank into the code
        // surface below — the mask is a byproduct of the walk, never a second parse.
        let needs_surface =
            !batch.surface.is_empty() && !prose_lang(&self.lang);
        let mut english_spans: Vec<(usize, usize)> = Vec::new();
        let mut parsed = false;
        if !batch.ast_by_kind.is_empty() || needs_surface {
            let tree = language(&self.lang).and_then(|language| {
                let mut parser = Parser::new();
                parser.set_language(&language).ok()?;
                parser.parse(code, None)
            });
            if let Some(tree) = &tree {
                self.walk_fire(tree.root_node(), code.as_bytes(), batch, &mut lines_per_rule, &mut english_spans);
                parsed = true;
            }
        }
        // Token rules: one pass over each firing universe's lines. Raw-universe detectors
        // (construct grounded only in comments/strings) and prose files match raw lines; every
        // other detector matches the code surface — each rule fires in the universe it
        // grounded in (ledger #12/#14).
        self.fire_tokens(&batch.raw, code, &mut lines_per_rule);
        if !batch.surface.is_empty() {
            let surface = if prose_lang(&self.lang) {
                std::borrow::Cow::Borrowed(code)
            } else if parsed {
                std::borrow::Cow::Owned(blank_spans(code, &english_spans))
            } else {
                // No grammar (or unparseable): the line-based masker fallback.
                std::borrow::Cow::Owned(code_surface_file(&self.lang, code))
            };
            self.fire_tokens(&batch.surface, &surface, &mut lines_per_rule);
        }
        // Structural probes: each walks the tree itself (native/architecture.dx, "the probe bridge"). They
        // judge the AST directly, so they run over the raw source, not the blanked surface.
        for &i in &batch.probes {
            match &self.rules[i].kind {
                MatchKind::Probe(name) => {
                    if let Some(kind) = crate::lint_probe::ProbeKind::from_name(name) {
                        lines_per_rule[i].extend(kind.detect(&self.lang, code));
                    }
                }
                MatchKind::Trace(plan) => {
                    lines_per_rule[i].extend(crate::lint_trace::run_plan(plan, &self.lang, code));
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        for (i, r) in self.rules.iter().enumerate() {
            let lines = &mut lines_per_rule[i];
            lines.sort_unstable();
            lines.dedup();
            // Probes judge the tree structurally — as exact as a text-anchored AST match — so
            // they report directly rather than through the concept gate, while staying
            // quarantinable like any non-project rule (their id is not project law).
            let precise = matches!(&r.kind, MatchKind::Ast(p) if p.text_anchored())
                || matches!(&r.kind, MatchKind::Probe(_) | MatchKind::Trace(_));
            for &line in lines.iter() {
                out.push(Finding { rule: r.id.clone(), severity: r.severity.clone(), line, precise });
            }
        }
        out
    }

    /// The lazily-built single-pass firing index — derived from the rules on first use (a
    /// deserialized model arrives without it).
    fn batch(&self) -> &Batch {
        self.batch.get_or_init(|| {
            let mut ast_by_kind: HashMap<String, Vec<usize>> = HashMap::new();
            let mut surface: Vec<usize> = Vec::new();
            let mut raw: Vec<usize> = Vec::new();
            let mut probes: Vec<usize> = Vec::new();
            for (i, r) in self.rules.iter().enumerate() {
                match &r.kind {
                    MatchKind::Ast(p) => {
                        ast_by_kind.entry(p.root_kind().to_string()).or_default().push(i)
                    }
                    MatchKind::Tokens { raw: true, .. } => raw.push(i),
                    MatchKind::Tokens { .. } => surface.push(i),
                    MatchKind::Probe(_) | MatchKind::Trace(_) => probes.push(i),
                }
            }
            Batch { ast_by_kind, surface, raw, probes }
        })
    }

    /// The ONE depth-first walk per file: at each node, only the AST patterns rooted at that
    /// node's kind are tried ([`RulePattern::root_kind`]) — match decisions are identical to a
    /// per-rule [`RulePattern::matches_in`] walk — and every English-bearing node's byte span
    /// (string/comment/heredoc/char, the same kind test grounding's masker uses) is collected
    /// for the code-surface blank. Descent continues under English nodes: an AST pattern may
    /// legitimately root at or inside them (a string node is never an identifier, so precision
    /// holds by construction).
    fn walk_fire(
        &self,
        node: tree_sitter::Node,
        src: &[u8],
        batch: &Batch,
        lines_per_rule: &mut [Vec<usize>],
        english_spans: &mut Vec<(usize, usize)>,
    ) {
        let kind = node.kind();
        if kind.contains("string") || kind.contains("comment") || kind.contains("heredoc") || kind.contains("char") {
            english_spans.push((node.start_byte(), node.end_byte()));
        }
        if let Some(candidates) = batch.ast_by_kind.get(kind) {
            for &i in candidates {
                if let MatchKind::Ast(p) = &self.rules[i].kind {
                    if p.matches_at(node, src) {
                        lines_per_rule[i].push(node.start_position().row + 1);
                    }
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk_fire(child, src, batch, lines_per_rule, english_spans);
        }
    }

    /// Fire every token detector in `idxs` over `text`, one pass over its lines.
    fn fire_tokens(&self, idxs: &[usize], text: &str, lines_per_rule: &mut [Vec<usize>]) {
        if idxs.is_empty() {
            return;
        }
        for (i, line) in text.lines().enumerate() {
            for &r in idxs {
                if let MatchKind::Tokens { tokens, .. } = &self.rules[r].kind {
                    if select::tokens_fire_line(line, tokens) {
                        lines_per_rule[r].push(i + 1);
                    }
                }
            }
        }
    }

    /// An empty rule set for `lang` — the identity element [`RuleSet::merged`] folds with
    /// when a language has an overlay but no AI module yet (law-only enforcement).
    pub fn empty(lang: &str) -> RuleSet {
        RuleSet { lang: lang.to_string(), rules: Vec::new(), withheld: Vec::new(), batch: Default::default() }
    }

    /// Merge two rule sets, `first` outranking `second` — the trust-order merge that joins a
    /// project OVERLAY (law + machine corpus) with a shared AI MODULE (doc rules): a rule id
    /// or an identical compiled pattern appearing in both keeps the overlay's version, exactly
    /// the first-wins dedup [`RuleSet::build`] applies within one build.
    pub fn merged(first: RuleSet, second: RuleSet) -> RuleSet {
        let lang = if first.lang.is_empty() { second.lang.clone() } else { first.lang.clone() };
        let mut seen_ids: HashSet<String> = HashSet::new();
        let mut seen_patterns: HashSet<String> = HashSet::new();
        let mut rules = Vec::new();
        for r in first.rules.into_iter().chain(second.rules) {
            if !seen_ids.insert(r.id.clone()) {
                continue;
            }
            if !seen_patterns.insert(serde_json::to_string(&r.kind).unwrap_or_default()) {
                continue;
            }
            rules.push(r);
        }
        RuleSet { lang, rules, withheld: Vec::new(), batch: Default::default() }
    }

    /// Serialize to JSON for caching.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Load from cached JSON.
    pub fn from_json(s: &str) -> Option<RuleSet> {
        serde_json::from_str(s).ok()
    }
}

// ── HLM1 binary codecs (native/architecture.dx, "Save") ────────────────────────────────────
//
// Field order is wire order. The lazy firing index (`batch`) is derived, never serialized —
// exactly the `#[serde(skip)]` contract, kept by construction here.

impl crate::lint_codec::Bin for MatchKind {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        match self {
            MatchKind::Ast(pat) => {
                e.u(0);
                pat.enc(e);
            }
            MatchKind::Tokens { tokens, raw } => {
                e.u(1);
                tokens.enc(e);
                e.boolean(*raw);
            }
            MatchKind::Probe(name) => {
                e.u(2);
                e.str(name);
            }
            // The trace plan is a handful of primitive indices — serialized as JSON on the string
            // stream (tiny; no dedicated wire form warranted).
            MatchKind::Trace(plan) => {
                e.u(3);
                e.str(&serde_json::to_string(plan).unwrap_or_default());
            }
        }
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<MatchKind> {
        match d.u()? {
            0 => Some(MatchKind::Ast(tree::RulePattern::dec(d)?)),
            1 => Some(MatchKind::Tokens { tokens: Vec::dec(d)?, raw: d.boolean()? }),
            2 => Some(MatchKind::Probe(d.str()?)),
            3 => serde_json::from_str(&d.str()?).ok().map(MatchKind::Trace),
            _ => None,
        }
    }
}

impl crate::lint_codec::Bin for CompiledRule {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.str(&self.id);
        e.str(&self.severity);
        e.str(&self.description);
        e.str(&self.source);
        self.kind.enc(e);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<CompiledRule> {
        Some(CompiledRule {
            id: d.str()?,
            severity: d.str()?,
            description: d.str()?,
            source: d.str()?,
            kind: MatchKind::dec(d)?,
        })
    }
}

impl crate::lint_codec::Bin for RuleSet {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.str(&self.lang);
        self.rules.enc(e);
        // PASS 31 — the conservation ledger rides at the END, bounds-safe: an old-format module
        // simply lacks these bytes and decodes to an empty ledger (same trailing shape the web's
        // graded/referee payloads use).
        let (ids, gates): (Vec<String>, Vec<String>) = self.withheld.iter().cloned().unzip();
        ids.enc(e);
        gates.enc(e);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<RuleSet> {
        let lang = d.str()?;
        let rules = Vec::dec(d)?;
        let withheld = match <Vec<String> as crate::lint_codec::Bin>::dec(d) {
            Some(ids) => {
                let gates = <Vec<String> as crate::lint_codec::Bin>::dec(d).unwrap_or_default();
                ids.into_iter().zip(gates).collect()
            }
            None => Vec::new(),
        };
        Some(RuleSet { lang, rules, withheld, batch: Default::default() })
    }
}

