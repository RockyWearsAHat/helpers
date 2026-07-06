//! Construct selection for [`super`]: how a rule's ENGLISH names the thing its detector
//! watches. Grounding corpora ([`Grounding`]/[`GroundView`]), the learned-evidence description
//! discriminator, the `bad ∧ ¬good` token-diff fallback, and the code-surface masker that keeps
//! grounding and firing in the same text universe. Theory and failure ledger: `LINTER.md`.

use std::collections::HashSet;

use super::MAX_EXAMPLE_BYTES;

/// The WHOLE identifier runs of `text` — maximal alphanumeric-plus-underscore spans,
/// lowercased, at least 2 chars. Grounding compares these, on BOTH sides (one splitter, by
/// construction — the ledger #11 invariant): a description word grounds only when each of its
/// runs exists as a WHOLE run in the corpus. Sub-word parts are comprehension, not existence —
/// the reader may understand `floor_never_splits` through `never`, but that identifier is not
/// an occurrence of the construct `never` (ledger #14: register words once grounded through
/// exactly such fragments and hijacked every preventive law).
pub(super) fn ground_runs(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|r| r.chars().count() >= 2)
        .map(|r| r.to_lowercase())
}

/// The CODE SURFACE of one source line — the text where a construct legitimately lives. `None`
/// for a whole-line comment; otherwise the line with string-literal interiors blanked and a
/// trailing `//`/`#` comment cut. Grounding and firing MUST share this function: a law grounds
/// only against real code, so its detector must fire only on real code — the same text universe
/// on both sides (LINTER.md ledger #12). The quote characters themselves survive (the string
/// EXISTS; its English contents don't count as code), and a quote with no closing mate on the
/// same line is NOT a string opener — it is code typography (a Rust lifetime `'a`, a stray
/// backtick) and masking to end-of-line would hide real constructs behind it.
pub(super) fn code_surface(line: &str) -> Option<String> {
    let t = line.trim_start();
    let comment = ["//", "#", "*", "/*", "--"];
    if comment.iter().any(|c| t.starts_with(c)) {
        return None;
    }
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '"' | '\'' | '`' => {
                // Scan for the closing mate, honoring escapes. No mate on this line → not a
                // string: keep the character and move on.
                let mut j = i + 1;
                while j < chars.len() && chars[j] != c {
                    j += if chars[j] == '\\' { 2 } else { 1 };
                }
                if j < chars.len() {
                    out.push(c);
                    out.push(c);
                    i = j + 1;
                } else {
                    out.push(c);
                    i += 1;
                }
            }
            '#' => break,
            '/' if chars.get(i + 1) == Some(&'/') => break,
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    Some(out)
}

/// The code surface of a WHOLE FILE. For a language with a grammar this is AST-exact: every
/// string / comment / heredoc / char node's span is blanked from the same parse tree firing
/// uses, so a multi-line help string, a heredoc body, or a doc comment can never ground or
/// fire a word — line-based masking cannot see those (ledger #14: "never" once grounded as
/// project code in four languages through exactly this hole). Newlines survive so line
/// numbers are stable. Grammarless languages fall back to the per-line [`code_surface`].
pub(super) fn code_surface_file(lang: &str, code: &str) -> String {
    if let Some(language) = super::grammar::language(lang) {
        let mut parser = tree_sitter::Parser::new();
        if parser.set_language(&language).is_ok() {
            if let Some(tree) = parser.parse(code, None) {
                let mut out = code.as_bytes().to_vec();
                blank_english_nodes(tree.root_node(), &mut out);
                if let Ok(s) = String::from_utf8(out) {
                    return s;
                }
            }
        }
    }
    code.lines()
        .map(|l| code_surface(l).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Blank `spans` (byte ranges of English-bearing nodes, collected during the firing walk) out
/// of `code` — the code surface without a second parse. Newlines survive so line numbers are
/// stable; span edges are node boundaries, so UTF-8 stays valid. Falls back to the line-based
/// masker if the blanked bytes are somehow not UTF-8 (defensive; node spans never split chars).
pub(super) fn blank_spans(code: &str, spans: &[(usize, usize)]) -> String {
    let mut out = code.as_bytes().to_vec();
    let len = out.len();
    for &(start, end) in spans {
        for b in &mut out[start.min(len)..end.min(len)] {
            if *b != b'\n' {
                *b = b' ';
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| {
        code.lines().map(|l| code_surface(l).unwrap_or_default()).collect::<Vec<_>>().join("\n")
    })
}

/// Blank the byte span of every node whose kind names English-bearing content. The kind test
/// is grammar typography (node-kind names), not vocabulary: `string`, `comment`, `heredoc`,
/// and `char` cover string/raw-string/template-string literals, line and block comments,
/// heredoc bodies, and char literals across the bundled grammars.
fn blank_english_nodes(node: tree_sitter::Node, out: &mut [u8]) {
    let k = node.kind();
    if k.contains("string") || k.contains("comment") || k.contains("heredoc") || k.contains("char") {
        let end = node.end_byte().min(out.len());
        for b in &mut out[node.start_byte()..end] {
            if *b != b'\n' {
                *b = b' ';
            }
        }
        return;
    }
    let mut cur = node.walk();
    for c in node.children(&mut cur) {
        blank_english_nodes(c, out);
    }
}

// ── Text-pattern fallback (universal — any language, any docs) ───────────────

/// Strip single-line comments (`//` and `#`) from code so doc-page prose like
/// `// example code where clippy issues a warning` never becomes the discriminator.
fn strip_code_comments(code: &str) -> String {
    code.lines()
        .filter(|l| {
            let t = l.trim_start();
            !t.starts_with("//") && !t.starts_with('#') && !t.starts_with('*') && !t.starts_with("/*")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Grounded evidence [`RuleSet::build`] reads English descriptions through: the real-code
/// reference corpus the language's own documentation served, and the learned polarity classifier
/// (which carries the reader and its word-frequency knowledge of the language's prose). Both are
/// LEARNED artifacts — construct selection consults them instead of any authored word list or
/// dictionary, so a miss is fixed by more reading, never by another extraction pass.
#[derive(Default)]
pub struct Grounding {
    /// Real code examples from the language's documentation — the "what's normal" corpus. This
    /// is the ONLY corpus that can ground a LEARNED rule's construct: documented code is code.
    pub reference: Vec<String>,
    /// The project's own sources. They ground and rank the PROJECT'S law (a law names constructs
    /// that live in the code it governs) but never a learned rule — project files carry English
    /// comments and data strings, and teaching vocabulary must not pass as code through them.
    pub project: Vec<String>,
    /// The learned prohibition/endorsement classifier; its reader knows which words are common
    /// connective prose in this language's documentation.
    pub polarity: Option<std::sync::Arc<crate::lint_read::Polarity>>,
    /// Ids of rules the PROJECT itself authored (`.helpers/lint-rules/`, root `lintPref`). Their
    /// rule file is the label: everything a user writes there is law by location, so these are
    /// exempt from the prohibition gate exactly as the live path exempts them from the Hv gate.
    pub trusted: std::collections::HashSet<String>,
    /// Token seeds of the example codes the toolchain FLAGGED during grounding — the
    /// reality-tested labels (LINTER.md, ledger #19). The example-diff compile path may keep
    /// literal example tokens only for an example in here (or when the law's own words name
    /// them): a Clean-parsing example's identifiers are just code the docs showed.
    pub flagged: std::collections::HashSet<u64>,
}

/// [`Grounding`] precomputed for one `RuleSet::build` run: the reference corpus flattened to an
/// identifier set, and the reader borrowed out of the classifier.
pub(super) struct GroundView<'a> {
    /// Tokens of the documentation's real, comment-stripped code — what grounds a LEARNED rule.
    pub(super) code_tokens: std::collections::HashSet<String>,
    /// Tokens of the project's own comment-stripped sources — extra ranking evidence for the
    /// project's law only.
    pub(super) project_tokens: std::collections::HashSet<String>,
    /// Tokens of the project's RAW sources — comment bodies and string interiors included.
    /// A law word that exists only here ("TODO", a port `":8080"`) grounds in the raw
    /// universe, and its detector then fires on raw lines: a law fires in the text universe
    /// it grounded in (ledger #12, generalized). Only non-connective words may ground this
    /// way — comments are English, and head words live in every repo's comments (#14/#15).
    pub(super) project_raw_tokens: std::collections::HashSet<String>,
    /// The reader whose learned frequencies say which words are common prose.
    pub(super) reader: Option<&'a crate::lint_read::Reader>,
    /// The full learned classifier — decides whether a description STATES a violation at all.
    pub(super) polarity: Option<&'a crate::lint_read::Polarity>,
}

impl<'a> GroundView<'a> {
    /// Flatten `g`'s corpora into token sets and borrow its reader and classifier. Each corpus
    /// entry is reduced to its [`code_surface_file`] first: grounding means "occurs in CODE",
    /// and comments and string interiors — including multi-line strings and heredoc bodies —
    /// are English inside a code file, exactly the text that must not launder teaching
    /// vocabulary into constructs (ledger #14).
    ///
    /// Tokenization is [`crate::lint_read::tokens`] — the reader's ONE tokenizer (ledger #2) —
    /// because these sets are compared against description words tokenized the same way. An
    /// ad-hoc splitter here once kept `secret_token` whole while the description side split it
    /// at `_`, so no snake_case identifier could ever ground a law (ledger #11).
    pub(super) fn of(lang: &str, g: &'a Grounding) -> GroundView<'a> {
        let tokens_of = |corpus: &[String]| -> std::collections::HashSet<String> {
            let mut out = std::collections::HashSet::new();
            for code in corpus {
                for run in ground_runs(&code_surface_file(lang, code)) {
                    out.insert(run);
                }
            }
            out
        };
        let raw_tokens_of = |corpus: &[String]| -> std::collections::HashSet<String> {
            let mut out = std::collections::HashSet::new();
            for code in corpus {
                for run in ground_runs(code) {
                    out.insert(run);
                }
            }
            out
        };
        GroundView {
            code_tokens: tokens_of(&g.reference),
            project_tokens: tokens_of(&g.project),
            project_raw_tokens: raw_tokens_of(&g.project),
            reader: g.polarity.as_deref().map(|p| p.reader()),
            polarity: g.polarity.as_deref(),
        }
    }

    /// The polarity CONTEXT of each whitespace word of `desc`, read along the text: a word's
    /// context is the lean of the nearest word (scanning outward through the reading sequence)
    /// that renders a decisive per-token verdict — its own lean first. No punctuation is
    /// consulted and nothing is chopped: "Do not leave TODO comments; file an issue instead"
    /// places TODO two words from "not" (prohibition context) and "issue" beside
    /// "file…instead" (remedy context) purely by learned leans and adjacency. `None` per word
    /// when no decisive word is in reach; all-`None` when no classifier is ready.
    pub(super) fn word_contexts(&self, desc: &str) -> Vec<Option<bool>> {
        let words: Vec<&str> = desc.split_whitespace().collect();
        let Some(p) = self.polarity.filter(|p| p.is_ready()) else {
            return vec![None; words.len()];
        };
        // Each word's OWN lean: the first decisive lean among its inner tokens. COMMON words
        // (corpus-scaled cutoff) never project a lean: a register verb like "use" reads
        // decisively endorsing in many languages' docs and would poison its neighbors'
        // contexts ("Never use unsafe…" must not mark `unsafe` as remedy vocabulary) — only
        // informative words carry context, the same reverse-frequency principle the vote
        // weights follow. The cutoff scales with the reading, so a two-page corpus (where
        // nothing is truly common) keeps every lean.
        let common = |w: &&str| -> bool {
            let toks = crate::lint_read::tokens(w);
            !toks.is_empty() && toks.iter().all(|t| p.reader().is_common_word(t))
        };
        let own: Vec<Option<bool>> = words
            .iter()
            .map(|w| {
                if common(w) {
                    return None;
                }
                crate::lint_read::tokens(w).iter().find_map(|t| p.token_lean(t))
            })
            .collect();
        (0..words.len())
            .map(|i| {
                (0..words.len().max(1))
                    .flat_map(|d| [i.checked_sub(d), i.checked_add(d).filter(|j| *j < words.len())])
                    .flatten()
                    .find_map(|j| own[j])
            })
            .collect()
    }
}

/// Derive the WATCHED TOKEN by READING the rule's English *description* — the prose the
/// documentation actually wrote. This is how a prose-only rule ("Never call `eval` anywhere")
/// becomes a detector with no code example.
///
/// No shape expectations: no backtick, dotted-path, call-syntax, or morphology rules — those are
/// conventions that cannot be guaranteed forever, and each one the engine expects is a way for a
/// valid law to become invisible. Instead there is ONE tokenization (the sentence's
/// whitespace-delimited words, edge punctuation trimmed, so `console.log`, `8080`, and a
/// backticked word each survive exactly as written) and selection is LEARNED evidence only:
///
///   * words whose inner tokens the reader has absorbed as common English are connective prose
///     and drop out — what the reading cannot account for is the construct;
///   * among the salient words, one grounded in the language's real reference code outranks the
///     rest; then the rarer word (fewest reads) wins; then reading order.
///
/// More reading sharpens the selection — the fix for a wrong pick is never a new shape rule.
/// When `bad` is non-empty every candidate must appear in it — the description says what is
/// wrong and the example must exhibit it. With no reader and no example there is no evidence at
/// all, and the engine ABSTAINS rather than guessing.
///
/// `only_grounded` restricts candidates to words that occur in REAL code (the docs' reference
/// corpus or the project's own sources). Learned rules require it: a rare English word in a
/// principle's imperative clause ("don't over-engineer") is not a code construct, and a detector
/// built from one fires on every comment that discusses the principle. The project's own law
/// passes `false` — the rule file is evidence enough that the named thing is worth watching for.
/// `contexts` is each whitespace word's polarity context ([`GroundView::word_contexts`]): a
/// candidate in a remedy context ("…; use the logging module instead") is the alternative the
/// rule endorses, never its construct, so prohibition-context candidates outrank everything and
/// endorsement-context candidates are dropped outright.
///
/// Returns the watched token (lowercased; matched case-insensitively by [`tokens_fire_line`])
/// plus its firing UNIVERSE: `true` when the chosen construct grounded only in the project's
/// raw text (comments/strings), so the detector must fire on raw lines — the universe it
/// grounded in (see [`GroundView::project_raw_tokens`]).
pub(super) fn description_discriminator(
    desc: &str,
    bad: &str,
    ground: &GroundView,
    contexts: &[Option<bool>],
    only_grounded: bool,
) -> Option<(String, bool)> {
    let reader = ground.reader?;
    // "Not common language" (LINTER.md evidence hierarchy #2, ledger #17): a word is
    // connective when common language accounts for it — the dictionary-read LangBrain knows
    // it, or it sits in the docs corpus head. The docs head alone measurably cannot carry
    // this judgment ("never" at 165 reads sat far under a 691-read head cutoff, so register
    // words hijacked selection from the named construct).
    let english = crate::lint_english::brain();
    // "Not common language" (LINTER.md evidence hierarchy #4, ledger #17): common language
    // accounts for a word when the dictionary-read LangBrain knows it or it sits in the docs
    // corpus head. English is asked about the WHOLE word: a compound identifier
    // (`secret_token`, `document.write`) reads as several English tokens, but the compound
    // itself is code typography no dictionary defines — its parts being common must not
    // demote it. The docs head alone measurably cannot carry this judgment ("never" at 165
    // reads sat far under a 691-read head cutoff, so register words hijacked selection).
    let connective = |surface: &str| {
        let inner = crate::lint_read::tokens(surface);
        (inner.len() == 1 && english.is_some_and(|e| e.knows(&inner[0])))
            || (!inner.is_empty() && inner.iter().all(|t| reader.is_head_word(t)))
    };
    // The dictionary judgment is an existence TIE-BREAK, never a veto on preventive laws: a
    // construct can itself be an English word and ground nowhere (`panic` in a clean repo),
    // so among UNGROUNDED words only the docs-corpus head demotes and the sentence's own
    // polarity context stays the deciding evidence (the register residual there is the
    // per-token polarity open problem, LINTER.md).
    let head_only = |surface: &str| {
        let inner = crate::lint_read::tokens(surface);
        !inner.is_empty() && inner.iter().all(|t| reader.is_head_word(t))
    };
    // (surface word, reading position, context tier, in-project?, grounded?, rarity = fewest
    // reads among inner tokens, raw-universe-only?). No stop-list and no frequency CUTOFF
    // anywhere: connective prose simply ranks last by its read counts, which stays true at
    // every corpus size — a threshold that felt right at one scale silently dies at another.
    let mut candidates: Vec<(String, usize, u8, bool, bool, bool, u32, bool)> = Vec::new();
    // Whitespace-word count of the description's FIRST sentence — the clause where the
    // document-order convention says the law names its violation.
    let first_sentence_words = crate::lint_read::sentences(desc)
        .first()
        .map(|s| s.split_whitespace().count())
        .unwrap_or(usize::MAX);
    for (position, raw) in desc.split_whitespace().enumerate() {
        let surface = raw.trim_matches(|c: char| !c.is_alphanumeric());
        if surface.chars().count() < 2 {
            continue;
        }
        let context = contexts.get(position).copied().flatten();
        let inner = crate::lint_read::tokens(surface);
        if inner.is_empty() {
            continue;
        }
        // Documented code grounds anyone; the project's own code additionally grounds (and
        // ranks) only when the rule is not held to `only_grounded` — i.e. the project's law.
        // Existence is judged on WHOLE identifier runs, every run of the word ([`ground_runs`]).
        let runs: Vec<String> = ground_runs(surface).collect();
        let in_docs = !runs.is_empty() && runs.iter().all(|t| ground.code_tokens.contains(t));
        if only_grounded && !in_docs {
            continue;
        }
        let in_code = !runs.is_empty() && runs.iter().all(|t| ground.project_tokens.contains(t));
        // Raw-universe existence: the word lives only in the project's comments/strings. Head
        // words never ground this way — comments are English (#14/#15) — and neither does a
        // BACKTICKED word: backticks mark a code construct (`todo!`), whose law must stay
        // preventive on the code surface rather than fire on comments discussing it.
        let in_raw = !in_code
            && !raw.contains('`')
            && !connective(surface)
            && !runs.is_empty()
            && runs.iter().all(|t| ground.project_raw_tokens.contains(t));
        let in_project = in_code || in_raw;
        let grounded = in_docs || in_project;
        let raw_only = in_raw && !in_docs;
        // Remedy-context vocabulary is endorsed, not forbidden. For LEARNED rules it is
        // ineligible outright ("…; use the logging module instead" can never compile
        // `logging`). For the PROJECT'S LAW it is DEMOTED, never dropped (the tier below): a
        // preventive law names a construct absent from the code by definition, and a
        // register-painted context must not leave the law watching nothing (`no_dbg` once
        // reported unenforceable because every word of its sentence read as remedy). Demotion
        // applies only PAST the first sentence — the author names the violation before the
        // remedy (the document-order convention, ledger #6a), so a docs-register lean on the
        // construct's own word ("dbg! is a useful macro…" paints `dbg` as endorsement) cannot
        // demote it inside the naming sentence.
        if context == Some(false) && only_grounded {
            continue;
        }
        let in_naming_sentence = position < first_sentence_words;
        let tier: u8 = match context {
            Some(true) => 0,                             // forbidding — the violation's vocabulary
            Some(false) if !in_naming_sentence => 2,     // remedy — the endorsed alternative
            _ => 1,                                      // neutral (or first-sentence paint)
        };
        // The author's own MARKING: a backticked word in the NAMING SENTENCE is the named
        // construct. Optional evidence, never a gate (ledger #2 banned shape REQUIREMENTS —
        // an unmarked law still compiles through the ranks below), but when the author did
        // mark, no corpus statistics may outvote them ("project", a real identifier in this
        // repo, once outranked the backticked `XMLHttpRequest` on existence). Only the first
        // sentence counts — authors backtick their remedies too ("…; use `fetch` instead"),
        // and the document-order convention says the violation is named before the remedy.
        let marked = raw.contains('`') && in_naming_sentence;
        let rarity = inner.iter().map(|t| reader.read_count(t)).min().unwrap_or(0);
        candidates.push((surface.to_string(), position, tier, marked, in_project, grounded, rarity, raw_only));
    }
    // Ordering (see LINTER.md, "The evidence hierarchy"). For the PROJECT'S LAW,
    // NOT-CONNECTIVE leads: the unread word is the construct, and both recorded hijack modes —
    // register words reading as decisively forbidding (ledger #15) and ordinary words
    // grounding through the project's own text (ledger #14: "never", "project") — are corpus
    // -head words, so commonness is the one signal that demotes them wherever they grounded.
    // Then existence (project code first, then documented code), then the context tier, then
    // document order among grounded words (the author names the violation before the remedy)
    // or rarity for ungrounded ones. LEARNED doc prose carries no order promise, so rarity
    // decides there (its candidates are all grounded already).
    if only_grounded {
        candidates.sort_by_key(|(surface, position, tier, _marked, _in_project, grounded, rarity, _raw)| {
            (!*grounded, *tier, connective(surface), *rarity, *position)
        });
    } else {
        // LINTER.md, "The evidence hierarchy": not-the-remedy, the author's marking,
        // project-code existence, not-common-language, docs existence, forbidding context,
        // then document order (grounded) / rarity (ungrounded). Existence leads the English
        // judgment because a law's construct may itself be an English word (`panic`, `var`) —
        // living in the project's CODE is what proves it is meant as code; the register
        // hijackers ("never", "import") that once rode existence (#15/#17) now tie there and
        // die on common-language knowledge, and English words can no longer ground through
        // the raw text universe at all.
        candidates.sort_by_key(|(surface, position, tier, marked, in_project, grounded, rarity, _raw)| {
            let order = if *grounded { *position as u64 } else { *rarity as u64 };
            let common = if *grounded { connective(surface) } else { head_only(surface) };
            (*tier == 2, !*marked, !*in_project, !*grounded, common, *tier, order, *position)
        });
    }

    // Validate: when bad is known the candidate must appear in it; when absent, trust the
    // winner — SELF-FIRE and query-time silence guard a wrong pick. The watched token is
    // lowercased and matched case-insensitively: prose capitalizes sentence-initial words
    // ("Unsafe blocks are banned…") but the construct in code is whatever case the code uses,
    // and grounding already matched case-normalized tokens.
    for (surface, _, _, _, _, _, _, raw_only) in &candidates {
        let token = surface.to_lowercase();
        if bad.trim().is_empty() || tokens_fire_text(bad, std::slice::from_ref(&token)) {
            return Some((token, *raw_only));
        }
    }
    None
}

// ── The one containment matcher (compile gates and live firing share it) ─────

/// Whether `tokens` fire on any line of `text` — see [`tokens_fire_line`].
pub(super) fn tokens_fire_text(text: &str, tokens: &[String]) -> bool {
    text.lines().any(|line| tokens_fire_line(line, tokens))
}

/// Whether `tokens` (stored lowercased) occur on this line, in order, each as a WHOLE token:
/// a token edge that is a word character (letter/digit/`_`) must not touch a word character
/// in the line, so `eval` never fires inside `literal_eval` while a trailing `)` needs no
/// gap. Matching is on the lowercased surface — every detector is case-insensitive (LINTER.md
/// ledger #15). This is the ONLY text-matching function: the compile gates (self-fire,
/// over-fire, reference-fire) and the live path both call it, so they can never disagree
/// about what a detector means.
pub(super) fn tokens_fire_line(line: &str, tokens: &[String]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let hay = line.to_lowercase();
    let mut from = 0;
    for token in tokens {
        match find_whole_token(&hay, token, from) {
            Some(end) => from = end,
            None => return false,
        }
    }
    true
}

/// Leftmost whole-token occurrence of `token` in `hay` at or after byte `from`; returns the
/// occurrence's end offset. Word-character edges of the token must not touch word characters.
fn find_whole_token(hay: &str, token: &str, from: usize) -> Option<usize> {
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let bytes = hay.as_bytes();
    let first_word = token.as_bytes().first().copied().is_some_and(is_word);
    let last_word = token.as_bytes().last().copied().is_some_and(is_word);
    let mut at = from;
    while let Some(offset) = hay.get(at..).and_then(|rest| rest.find(token)) {
        let start = at + offset;
        let end = start + token.len();
        let left_ok = !first_word || start == 0 || !is_word(bytes[start - 1]);
        let right_ok = !last_word || end == bytes.len() || !is_word(bytes[end]);
        if left_ok && right_ok {
            return Some(end);
        }
        // Overlap-safe, char-boundary-safe advance past this occurrence's first character.
        at = start + hay[start..].chars().next().map_or(1, char::len_utf8);
    }
    None
}

/// Derive a discriminating token sequence from `bad` and `good` examples using `bad ∧ ¬good`.
///
/// Strips `//`/`#` comment lines first so doc-page prose comments like
/// `// example code where clippy issues a warning` do not pollute the discriminator.
/// Tokens are the reader's word runs ([`crate::lint_read::read_units`]) — the ONE tokenizer
/// every token set in the engine goes through (LINTER.md ledger #2/#11); there is no
/// example-specific token grammar and no enumerated operator/sigil/flag shapes. Tries an
/// ordered same-line two-token pair first (most specific), then a single distinctive token,
/// each validated with [`tokens_fire_text`] — the very matcher that will fire it live.
///
/// Returns `None` when the difference carries no watchable word — pure punctuation or a bare
/// numeric value (`0` vs `1`, port `8080` vs `3000`) is semantics, not a construct, and the
/// AST diff is the path that carries those — so the caller drops such rules rather than
/// emitting a detector that would over-fire.
pub(super) fn text_discriminator(bad: &str, good: &str) -> Option<Vec<String>> {
    // A pointable anti-pattern is at most a screenful ([`MAX_EXAMPLE_BYTES`]); the pair search
    // below probes a candidate window per token, which on a scraped manual page is hours.
    if bad.len() > MAX_EXAMPLE_BYTES || good.len() > MAX_EXAMPLE_BYTES {
        return None;
    }
    // Strip doc-page comments before tokenising — they pollute the discriminator.
    let bad = strip_code_comments(bad);
    let good = strip_code_comments(good);
    let (bad, good) = (bad.as_str(), good.as_str());

    // Word runs ≥2 chars (the reader already drops shorter ones — single letters are
    // variables, not constructs) that carry at least one letter: an all-digit run is a value.
    let word_runs = |text: &str| -> Vec<String> {
        crate::lint_read::read_units(text)
            .into_iter()
            .map(|(full, _)| full)
            .filter(|t| !t.is_empty() && t.chars().any(|c| c.is_alphabetic()))
            .collect()
    };
    let bad_toks = word_runs(bad);
    let good_runs = word_runs(good);
    let good_set: HashSet<&str> = good_runs.iter().map(String::as_str).collect();

    // A candidate only wins by proving itself against both examples with the very matcher
    // that will fire it live — the same self-consistency the compile gates re-check later.
    let discriminates = |tokens: &[String]| -> bool {
        tokens_fire_text(bad, tokens) && !tokens_fire_text(good, tokens)
    };

    // 1. Ordered pair on the same line — any punctuation may sit between the tokens.
    // Two passes: first demand BOTH tokens be absent from the fix (pure bad ∧ ¬good — these
    // generalize), then relax to "not both present" so a pair anchored on one shared name can
    // still discriminate when nothing purer exists.
    for strict in [true, false] {
        for win in bad_toks.windows(2) {
            let in_good = (good_set.contains(win[0].as_str()), good_set.contains(win[1].as_str()));
            if if strict { in_good.0 || in_good.1 } else { in_good.0 && in_good.1 } {
                continue;
            }
            if discriminates(win) {
                return Some(win.to_vec());
            }
        }
    }

    // 2. Single distinctive token.
    for tok in &bad_toks {
        if good_set.contains(tok.as_str()) {
            continue;
        }
        if discriminates(std::slice::from_ref(tok)) {
            return Some(vec![tok.clone()]);
        }
    }

    None
}

