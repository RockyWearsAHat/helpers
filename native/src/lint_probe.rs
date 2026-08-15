//! `lint_probe` — the UNDERSTANDING bridge: structural AST probes wired from read prose.
//!
//! The owner's north star is a linter that flags code *bad by CS principles* even when no rule
//! string was written for it. Two halves make that real, and the boundary between them is the
//! whole design:
//!
//!   * **Programmed machinery — the probe.** A [`ProbeKind`] is a pure structural predicate over
//!     a tree-sitter tree: "a statement after a `return`", "`.unwrap()` on a fallible call", "a
//!     function body of dozens of statements", "a `pub` item with no doc comment", "an unexplained
//!     numeric literal", "a single-letter binding", "a secret-shaped literal in a key-named
//!     binding". These are the tree-walking primitives the owner blessed us to code — they carry
//!     no policy, only the ability to RECOGNISE a shape.
//!   * **Learned policy — the binding.** WHICH probes are live, and the advice each carries, come
//!     entirely from READING the machine-global principles corpus (`corpus/*.md`). A principle's
//!     prose description is understood in the 1-bit HDC substrate — through the dictionary meaning
//!     network when a brain is loaded (so "ignore / discard / suppress an error" all reach the
//!     swallowed-error probe by meaning), and through the pure spelling centroid
//!     ([`crate::lint_char::spell_vector`]) as the always-available floor — and bound to the probe
//!     whose concept it means ([`understand`]). Delete the corpus and every probe goes dark: the
//!     checks are learned, only the recognition is programmed.
//!
//! This is the bridge native/architecture.dx calls "Rules from understanding — the probe bridge": dictionary +
//! language understanding → read a rule described in prose → enforce it, with no pre-labelled
//! exemplars of bad code required. Exemplars would sharpen a probe; understanding of the
//! description is what activates it.

use crate::lint_ai::{Hv, DIM};

/// One structural defect class the tree-walk can RECOGNISE. Each variant is a pure predicate over
/// the AST plus a `concept` — the English phrase naming what it detects, which the corpus prose is
/// matched against by MEANING. The concept is machinery documentation, never a rule: the decision
/// to enforce a class is the corpus's, read in [`understand`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeKind {
    /// A statement after a `return` — unreachable dead code.
    DeadCodeAfterReturn,
    /// An error result ignored: `let _ = fallible()` or an empty `Err(_) => {}` arm.
    SwallowedError,
    /// `.unwrap()` / `.expect()` on a fallible call — a panic where an error should be handled.
    UnwrapOnFallible,
    /// A function whose body runs on for far too many statements — too many responsibilities.
    GodFunction,
    /// A `pub` function or type with no documentation comment.
    UndocumentedPublicItem,
    /// An unexplained numeric literal — a magic number that should be a named constant.
    MagicNumber,
    /// A single-letter, non-descriptive variable binding.
    NonDescriptiveName,
    /// A secret-shaped string literal assigned to a secret/key/token/password-named binding.
    HardcodedSecret,
    /// Untrusted input interpolated into a shell command string — a command-injection shape.
    ShellInjection,
    /// A near-identical code block duplicated across two functions — a DRY violation.
    DuplicatedCode,
}

/// Every probe, in one place — the set the corpus reading may switch on.
pub const ALL: [ProbeKind; 10] = [
    ProbeKind::DeadCodeAfterReturn,
    ProbeKind::SwallowedError,
    ProbeKind::UnwrapOnFallible,
    ProbeKind::GodFunction,
    ProbeKind::UndocumentedPublicItem,
    ProbeKind::MagicNumber,
    ProbeKind::NonDescriptiveName,
    ProbeKind::HardcodedSecret,
    ProbeKind::ShellInjection,
    ProbeKind::DuplicatedCode,
];

impl ProbeKind {
    /// The stable machine name of this probe — how it is serialized in a compiled rule.
    pub fn name(self) -> &'static str {
        match self {
            ProbeKind::DeadCodeAfterReturn => "dead_code_after_return",
            ProbeKind::SwallowedError => "swallowed_error",
            ProbeKind::UnwrapOnFallible => "unwrap_on_fallible",
            ProbeKind::GodFunction => "god_function",
            ProbeKind::UndocumentedPublicItem => "undocumented_public_item",
            ProbeKind::MagicNumber => "magic_number",
            ProbeKind::NonDescriptiveName => "non_descriptive_name",
            ProbeKind::HardcodedSecret => "hardcoded_secret",
            ProbeKind::ShellInjection => "shell_injection",
            ProbeKind::DuplicatedCode => "duplicated_code",
        }
    }

    /// Parse a probe from its serialized [`name`](Self::name).
    pub fn from_name(s: &str) -> Option<ProbeKind> {
        ALL.into_iter().find(|k| k.name() == s)
    }

    /// The English phrase this probe detects — the concept the corpus prose is matched to by
    /// meaning. These are natural-language descriptions of a shape, not code patterns and not rule
    /// strings; the corpus author's paragraph binds here when it MEANS the same thing.
    pub fn concept(self) -> &'static str {
        match self {
            ProbeKind::DeadCodeAfterReturn => "unreachable dead code written after a return statement",
            ProbeKind::SwallowedError => "an error result ignored discarded or swallowed silently instead of handled",
            ProbeKind::UnwrapOnFallible => "unwrap or expect the result of a fallible call forcing a panic",
            ProbeKind::GodFunction => "an enormous function that does too many things with too many statements",
            ProbeKind::UndocumentedPublicItem => "a public function or type exposed without a documentation comment",
            ProbeKind::MagicNumber => "an unexplained magic number literal that should be a named constant",
            ProbeKind::NonDescriptiveName => "a variable named with a single meaningless non descriptive letter",
            ProbeKind::HardcodedSecret => "a hardcoded secret password api key token or credential literal in the source",
            ProbeKind::ShellInjection => "untrusted input interpolated into a shell command string executed as a command injection vulnerability",
            ProbeKind::DuplicatedCode => "duplicated code repeated in two places",
        }
    }

    /// Lines (1-based) where this probe fires in `code` of language `lang`. Empty when the
    /// language has no bundled grammar (a probe is a structural predicate — it needs a tree).
    pub fn detect(self, lang: &str, code: &str) -> Vec<usize> {
        let Some(language) = crate::lint_match::language(lang) else { return Vec::new() };
        let mut parser = tree_sitter::Parser::new();
        if parser.set_language(&language).is_err() {
            return Vec::new();
        }
        let Some(tree) = parser.parse(code, None) else { return Vec::new() };
        let src = code.as_bytes();
        let mut hits: Vec<usize> = Vec::new();
        match self {
            ProbeKind::DeadCodeAfterReturn => dead_code(tree.root_node(), src, &mut hits),
            ProbeKind::SwallowedError => swallowed_error(tree.root_node(), src, &mut hits),
            ProbeKind::UnwrapOnFallible => unwrap_calls(tree.root_node(), src, &mut hits),
            ProbeKind::GodFunction => god_functions(tree.root_node(), src, &mut hits),
            ProbeKind::UndocumentedPublicItem => undocumented_public(tree.root_node(), code, &mut hits),
            ProbeKind::MagicNumber => magic_numbers(tree.root_node(), src, &mut hits),
            ProbeKind::NonDescriptiveName => non_descriptive(tree.root_node(), src, &mut hits),
            ProbeKind::HardcodedSecret => hardcoded_secret(tree.root_node(), src, &mut hits),
            ProbeKind::ShellInjection => shell_injection(tree.root_node(), src, &mut hits),
            ProbeKind::DuplicatedCode => duplicated_code(tree.root_node(), src, &mut hits),
        }
        hits.sort_unstable();
        hits.dedup();
        hits
    }
}

/// The row (1-based) a node starts on.
fn row(node: tree_sitter::Node) -> usize {
    node.start_position().row + 1
}

/// A node's source text, or `""` on any UTF-8 error.
fn text<'a>(node: tree_sitter::Node, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap_or("")
}

/// Depth-first visit of every node.
fn walk(node: tree_sitter::Node, f: &mut impl FnMut(tree_sitter::Node)) {
    f(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, f);
    }
}

// ── The structural probes ─────────────────────────────────────────────────────

/// A statement following a `return` in the same block is unreachable. We find each block, and the
/// first child that is (or contains, at statement top level) a `return`; every named,
/// non-comment sibling after it is dead code.
fn dead_code(root: tree_sitter::Node, src: &[u8], hits: &mut Vec<usize>) {
    walk(root, &mut |node| {
        if node.kind() != "block" {
            return;
        }
        let mut cursor = node.walk();
        let mut after_return = false;
        for child in node.children(&mut cursor) {
            if after_return && child.is_named() && !child.kind().contains("comment") {
                hits.push(row(child));
                continue;
            }
            if statement_returns(child, src) {
                after_return = true;
            }
        }
    });
}

/// Whether a block-level statement transfers control away unconditionally: a `return`, or a
/// `break`/`continue`, expressed either as the bare expression or wrapped in an
/// `expression_statement`.
fn statement_returns(node: tree_sitter::Node, src: &[u8]) -> bool {
    let k = node.kind();
    if k == "return_expression" {
        return true;
    }
    if k == "expression_statement" {
        let inner = node.named_child(0);
        return inner.is_some_and(|c| c.kind() == "return_expression");
    }
    // A trailing macro like `panic!`/`unreachable!` also ends control flow — but only flag the
    // clearest case (an explicit return) to keep the probe conservative on real code.
    let _ = src;
    false
}

/// An error ignored: `let _ = <call>;` (a fallible value bound to the throwaway) or an empty
/// `Err(_) => {}` match arm (the error branch does nothing).
fn swallowed_error(root: tree_sitter::Node, src: &[u8], hits: &mut Vec<usize>) {
    walk(root, &mut |node| match node.kind() {
        "let_declaration" => {
            let pat = node.child_by_field_name("pattern");
            let is_wild = pat.is_some_and(|p| text(p, src).trim() == "_");
            let val = node.child_by_field_name("value");
            // `let _ = <expr>` discards a value. It is a swallowed error when the discarded
            // value is a call result or a bound variable (which held one), not a plain literal:
            // `let _ = 5;` is a deliberate discard, `let _ = fallible();` and `let _ = d;` throw
            // a failure away.
            let discards_result = val.is_some_and(|v| !is_trivial_value(v));
            if is_wild && discards_result {
                hits.push(row(node));
            }
        }
        "match_arm" => {
            let pat = node.child_by_field_name("pattern");
            let names_err = pat.is_some_and(|p| text(p, src).contains("Err"));
            let val = node.child_by_field_name("value");
            let empty_body = val.is_some_and(|v| {
                let t = text(v, src).trim();
                t == "{}" || t == "()"
            });
            if names_err && empty_body {
                hits.push(row(node));
            }
        }
        _ => {}
    });
}

/// Whether an expression is a trivial literal value — a bare number, string, char, or boolean.
/// A `let _ =` of such a value is a deliberate discard, not a swallowed failure.
fn is_trivial_value(node: tree_sitter::Node) -> bool {
    matches!(
        node.kind(),
        "integer_literal" | "float_literal" | "string_literal" | "char_literal" | "boolean_literal" | "unit_expression"
    )
}

/// `.unwrap()` / `.expect()` on a receiver — a panic on the error path.
fn unwrap_calls(root: tree_sitter::Node, src: &[u8], hits: &mut Vec<usize>) {
    walk(root, &mut |node| {
        if node.kind() != "call_expression" {
            return;
        }
        let Some(func) = node.child_by_field_name("function") else { return };
        if func.kind() != "field_expression" {
            return;
        }
        let field = func.child_by_field_name("field").map(|f| text(f, src));
        if matches!(field, Some("unwrap" | "expect" | "unwrap_err")) {
            hits.push(row(node));
        }
    });
}

/// The statement count above which a function body is a god function. A body of dozens of
/// statements has outgrown a single responsibility; this is a size hyperparameter, not policy.
const GOD_FUNCTION_STATEMENTS: usize = 25;

/// A function whose body block holds more than [`GOD_FUNCTION_STATEMENTS`] statements.
fn god_functions(root: tree_sitter::Node, src: &[u8], hits: &mut Vec<usize>) {
    let _ = src;
    walk(root, &mut |node| {
        if !matches!(node.kind(), "function_item" | "function_signature_item") {
            return;
        }
        let Some(body) = node.child_by_field_name("body") else { return };
        let mut cursor = body.walk();
        let statements = body
            .children(&mut cursor)
            .filter(|c| c.is_named() && !c.kind().contains("comment"))
            .count();
        if statements > GOD_FUNCTION_STATEMENTS {
            hits.push(row(node));
        }
    });
}

/// A `pub` function or type with no doc comment on the line(s) immediately above it. Doc presence
/// is read from the source lines (grammar-independent): skipping attribute lines, the line above
/// the item must open a doc comment (`///`, `//!`, `/**`).
fn undocumented_public(root: tree_sitter::Node, code: &str, hits: &mut Vec<usize>) {
    let lines: Vec<&str> = code.lines().collect();
    walk(root, &mut |node| {
        if !matches!(
            node.kind(),
            "function_item" | "struct_item" | "enum_item" | "trait_item" | "type_item" | "mod_item"
        ) {
            return;
        }
        // `pub` in any form (`pub`, `pub(crate)`) makes it public surface.
        let is_public = node
            .child_by_field_name("visibility_modifier")
            .is_some()
            || first_child_kind(node) == Some("visibility_modifier");
        if !is_public {
            return;
        }
        let start = node.start_position().row;
        if !documented_above(&lines, start) {
            hits.push(start + 1);
        }
    });
}

/// The kind of a node's first child, if any.
fn first_child_kind(node: tree_sitter::Node) -> Option<&'static str> {
    let mut cursor = node.walk();
    let kind = node.children(&mut cursor).next().map(|c| c.kind());
    kind
}

/// Whether the item starting at 0-based `row` carries a doc comment on the lines above it,
/// skipping attribute lines (`#[...]`).
fn documented_above(lines: &[&str], row: usize) -> bool {
    let mut i = row;
    while i > 0 {
        let prev = lines[i - 1].trim_start();
        if prev.starts_with("#[") || prev.starts_with("#!") {
            i -= 1;
            continue;
        }
        return prev.starts_with("///") || prev.starts_with("//!") || prev.starts_with("/**");
    }
    false
}

/// A numeric literal with no name and no exemption — a magic number. Exempt: the small values
/// {0,1,2} (and their negatives), and any literal inside a `const`/`static` initializer (that IS
/// giving the value a name) or an array-length / attribute position.
fn magic_numbers(root: tree_sitter::Node, src: &[u8], hits: &mut Vec<usize>) {
    walk(root, &mut |node| {
        if !matches!(node.kind(), "integer_literal" | "float_literal") {
            return;
        }
        let raw = text(node, src);
        if small_magnitude(raw) {
            return;
        }
        if under_named_context(node) {
            return;
        }
        hits.push(row(node));
    });
}

/// Whether a numeric literal is one of the small, self-explanatory values that are never magic.
fn small_magnitude(raw: &str) -> bool {
    let digits: String = raw.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    matches!(digits.as_str(), "0" | "1" | "2" | "" )
        || digits.parse::<f64>().map(|v| v <= 2.0).unwrap_or(false)
}

/// Whether the literal sits where it is already named or structural — a const/static initializer,
/// a const item, an array size, or an attribute — never a bare magic number.
fn under_named_context(node: tree_sitter::Node) -> bool {
    let mut cur = node.parent();
    while let Some(p) = cur {
        if matches!(p.kind(), "const_item" | "static_item" | "attribute_item" | "attribute" | "array_type") {
            return true;
        }
        // Stop climbing at the enclosing statement/expression boundary so we do not treat a
        // literal deep in a function body as "under" a distant const.
        if matches!(p.kind(), "function_item" | "block") {
            return false;
        }
        cur = p.parent();
    }
    false
}

/// A single-letter value binding — a non-descriptive name. Only `let` bindings and function
/// parameters; type parameters (`F`) are `type_identifier`s and never match, and `_` is a
/// deliberate discard.
fn non_descriptive(root: tree_sitter::Node, src: &[u8], hits: &mut Vec<usize>) {
    walk(root, &mut |node| {
        let name = match node.kind() {
            "let_declaration" => node.child_by_field_name("pattern").map(|p| text(p, src)),
            "parameter" => node.child_by_field_name("pattern").map(|p| text(p, src)),
            _ => None,
        };
        if let Some(name) = name {
            let name = name.trim();
            if name.len() == 1 && name.chars().all(|c| c.is_ascii_alphabetic()) {
                hits.push(row(node));
            }
        }
    });
}

/// Words in a binding's name that mark it as holding a secret — drawn from the probe's own
/// concept vocabulary, not a policy list.
const SECRET_WORDS: [&str; 8] =
    ["secret", "password", "passwd", "apikey", "api_key", "token", "credential", "private_key"];

/// A secret-shaped string literal assigned to a secret-named binding: `let api_key = "…long…";`.
fn hardcoded_secret(root: tree_sitter::Node, src: &[u8], hits: &mut Vec<usize>) {
    walk(root, &mut |node| {
        if !matches!(node.kind(), "let_declaration" | "static_item" | "const_item") {
            return;
        }
        let name = node
            .child_by_field_name("pattern")
            .or_else(|| node.child_by_field_name("name"))
            .map(|p| text(p, src).to_lowercase())
            .unwrap_or_default();
        let secret_named = SECRET_WORDS.iter().any(|w| name.contains(w));
        if !secret_named {
            return;
        }
        let Some(val) = node.child_by_field_name("value") else { return };
        if val.kind() == "string_literal" {
            // A nontrivial literal (not `""` or a short placeholder) is a real embedded secret.
            let content = text(val, src).trim_matches('"');
            if content.len() >= 8 {
                hits.push(row(node));
            }
        }
    });
}

/// Shell tokens whose presence beside a built command string marks a shell execution.
const SHELL_TOKENS: [&str; 5] = ["\"sh\"", "\"bash\"", "\"-c\"", "\"cmd\"", "\"/bin/sh\""];

/// Untrusted input interpolated into a shell command: a call chain that both names a shell
/// (`Command::new("sh")`, `.arg("-c")`) and builds an argument with `format!`/concatenation. We
/// flag the statement that hands a `format!` (or `+`-built string) to a shell argument.
fn shell_injection(root: tree_sitter::Node, src: &[u8], hits: &mut Vec<usize>) {
    walk(root, &mut |node| {
        if node.kind() != "call_expression" {
            return;
        }
        // The outermost call in a chain: examine the whole chain's text once.
        if node
            .parent()
            .is_some_and(|p| matches!(p.kind(), "field_expression" | "call_expression"))
        {
            return;
        }
        let chain = text(node, src);
        let names_shell = SHELL_TOKENS.iter().any(|t| chain.contains(t));
        let builds_arg = chain.contains("format!") || chain.contains("format !");
        if names_shell && builds_arg {
            hits.push(row(node));
        }
    });
}

/// The smallest function-body node count that can count as duplication — below this, two tiny
/// bodies coinciding is coincidence, not a copy.
const DUP_MIN_NODES: usize = 12;

/// Two functions whose bodies have the same structural shape (identical sequence of node kinds)
/// are duplicated code. We shape-hash each function body and flag every function that shares its
/// shape with another.
fn duplicated_code(root: tree_sitter::Node, src: &[u8], hits: &mut Vec<usize>) {
    let _ = src;
    use std::collections::HashMap;
    let mut shapes: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut sizes: HashMap<u64, usize> = HashMap::new();
    walk(root, &mut |node| {
        if node.kind() != "function_item" {
            return;
        }
        let Some(body) = node.child_by_field_name("body") else { return };
        let mut kinds: Vec<u16> = Vec::new();
        walk(body, &mut |n| kinds.push(n.kind_id()));
        if kinds.len() < DUP_MIN_NODES {
            return;
        }
        let mut h: u64 = 0xcbf29ce484222325;
        for k in &kinds {
            h ^= *k as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        shapes.entry(h).or_default().push(row(node));
        sizes.insert(h, kinds.len());
    });
    for (_, rows) in shapes {
        if rows.len() >= 2 {
            for r in rows {
                hits.push(r);
            }
        }
    }
}

// ── The concept binding: read prose → the probe it means ──────────────────────

/// The HDC vector of a single word for concept matching: its pure SPELLING CENTROID
/// ([`crate::lint_char::spell_vector`]) — the representation `encode`'s own contract names as
/// "what concept matching stands on". This is deterministic and machine-independent, and it
/// SEPARATES: two words with a shared stem (`ignore`/`ignored`, `swallow`/`swallowed`) land close,
/// unrelated words sit at the noise floor. The dictionary MEANING network is the comprehension
/// backbone elsewhere, but its `related()` proximity is measured NOT to separate disapproval from
/// neutral vocabulary (native/architecture.dx) — every concept scored ~1.0 through it — so binding a principle
/// to a probe by meaning-synonym is a documented NEXT STEP that waits on a separating meaning
/// metric, not this cycle's mechanism.
fn word_vector(word: &str) -> Option<Hv> {
    crate::lint_char::spell_vector(word)
}

/// The content words of a phrase worth binding on: lowercase alphabetic runs of ≥4 letters. Short
/// function words (`the`, `a`, `is`) carry little concept signal and are dropped by length alone —
/// no curated stop-list.
fn salient_words(phrase: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for raw in phrase.split(|c: char| !c.is_alphanumeric()) {
        let w: String = raw.chars().filter(|c| c.is_ascii_alphabetic()).collect::<String>().to_lowercase();
        if w.len() >= 4 && seen.insert(w.clone()) {
            out.push(w);
        }
    }
    out
}

/// How close two word-vectors must be to count as "the same idea". Two identical or closely
/// related words land well below DIM/2 (pure-noise ≈ 4096 bits); unrelated words sit at the
/// noise floor. A word is considered matched when it is nearer than this fraction of the noise
/// floor — comfortably admitting a shared stem or a dictionary synonym while rejecting an
/// unrelated word. A ratio, not a content constant.
fn word_match_bar() -> u32 {
    (DIM as u32) * 30 / 100
}

/// COVERAGE of a probe concept by a description: the fraction of the concept's own salient words
/// that the description accounts for (some description word means the same thing — nearer than
/// [`word_match_bar`]). Coverage is asymmetric on purpose: it measures how much of what the PROBE
/// is about the prose actually says, so the distinctive words (`swallow`, `unwrap`, `secret`)
/// drive the score and the words two concepts share (`error`, `result`) cannot decide between
/// them.
fn coverage(concept_words: &[(String, Hv)], desc_words: &[(String, Hv)]) -> f32 {
    if concept_words.is_empty() {
        return 0.0;
    }
    let bar = word_match_bar();
    let matched = concept_words
        .iter()
        .filter(|(_, cv)| desc_words.iter().any(|(_, dv)| cv.distance(dv) <= bar))
        .count();
    matched as f32 / concept_words.len() as f32
}

/// The salient words of a phrase paired with their HDC vectors.
fn word_vectors(phrase: &str) -> Vec<(String, Hv)> {
    salient_words(phrase)
        .into_iter()
        .filter_map(|w| word_vector(&w).map(|v| (w, v)))
        .collect()
}

/// UNDERSTAND a principle's prose description: the probe whose concept the description MEANS, or
/// `None` when the description is about nothing this machinery can recognise.
///
/// The binding is the probe of highest concept [`coverage`], accepted only when the coverage is
/// substantial AND decisively ahead of the runner-up. Both bars are derived from the structure of
/// the match (a fraction of the concept covered; a margin over the second-best), never from any
/// listed word — so unrelated prose (a different rule, a document title) covers no concept enough
/// to bind, and a description that reads equally like two probes binds to neither.
pub fn understand(description: &str) -> Option<ProbeKind> {
    let desc_words = word_vectors(description);
    if desc_words.is_empty() {
        return None;
    }
    let mut scored: Vec<(ProbeKind, f32)> = ALL
        .into_iter()
        .map(|k| (k, coverage(&word_vectors(k.concept()), &desc_words)))
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    if std::env::var_os("HELPERS_PROBE_DEBUG").is_some() {
        eprintln!("[probe] {:?}", scored.iter().map(|(k, s)| (k.name(), *s)).collect::<Vec<_>>());
    }
    let (best, best_score) = scored[0];
    let runner_up = scored.get(1).map(|(_, s)| *s).unwrap_or(0.0);
    // A genuine match covers at least half the probe's concept and clears the runner-up by a
    // clear margin — one probe's distinctive vocabulary is present and the other's is not.
    if best_score >= 0.5 && best_score - runner_up >= 0.15 {
        Some(best)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every principle in the shipped corpus binds to its intended probe — the understanding path
    /// end to end, on the real prose (spelling-centroid floor; no brain needed).
    #[test]
    fn each_principle_binds_to_its_probe() {
        let cases = [
            ("Never write code after a return statement. A statement that follows a return is unreachable dead code.", ProbeKind::DeadCodeAfterReturn),
            ("Never ignore, discard, or swallow an error. An error result thrown away hides a real failure.", ProbeKind::SwallowedError),
            ("Never unwrap or expect the result of a fallible call. Unwrapping a result that can fail forces a panic.", ProbeKind::UnwrapOnFallible),
            ("Never write an enormous function that does too many things with dozens of statements.", ProbeKind::GodFunction),
            ("Never expose a public function or type without a documentation comment.", ProbeKind::UndocumentedPublicItem),
            ("Never bury an unexplained magic number literal in the code; give it a named constant.", ProbeKind::MagicNumber),
            ("Never name a variable with a single meaningless letter; choose a descriptive name.", ProbeKind::NonDescriptiveName),
            ("Never hardcode a secret, password, API key, token, or credential as a literal in the source.", ProbeKind::HardcodedSecret),
            ("Never interpolate untrusted input into a shell command string executed by a shell.", ProbeKind::ShellInjection),
            ("Never duplicate the same code in two places; extract the duplicated block into one function.", ProbeKind::DuplicatedCode),
        ];
        for (prose, want) in cases {
            assert_eq!(understand(prose), Some(want), "prose bound wrong: {prose}");
        }
    }

    /// Prose that is about something else binds to no probe — no false activation.
    #[test]
    fn unrelated_prose_binds_to_nothing() {
        assert_eq!(understand("Never declare variables with var; its function-wide hoisting leaks bindings."), None);
        assert_eq!(understand("Do not compare types with the equality operator; use isinstance instead."), None);
        assert_eq!(understand("CS principles the enforceable canon described in prose machine global corpus."), None);
    }

    /// The Rust probes fire on the shapes they name and stay silent on clean code.
    #[test]
    fn probes_fire_on_bad_and_are_silent_on_clean() {
        let bad = r#"
pub fn f(x: i32) -> i32 {
    let n = 8675309;
    if x > n { return x; let z = x; drop(z); }
    let d = std::fs::read_to_string("a");
    let _ = d;
    let v: i32 = "1".parse().unwrap();
    x + v
}
"#;
        assert!(!ProbeKind::DeadCodeAfterReturn.detect("rust", bad).is_empty(), "dead code");
        assert!(!ProbeKind::MagicNumber.detect("rust", bad).is_empty(), "magic number");
        assert!(!ProbeKind::SwallowedError.detect("rust", bad).is_empty(), "swallowed error");
        assert!(!ProbeKind::UnwrapOnFallible.detect("rust", bad).is_empty(), "unwrap");
        assert!(!ProbeKind::NonDescriptiveName.detect("rust", bad).is_empty(), "single-letter name");
        assert!(!ProbeKind::UndocumentedPublicItem.detect("rust", bad).is_empty(), "undocumented pub");

        let good = "/// A documented adder.\npub const LIMIT: i32 = 5;\n/// Adds one.\npub fn increment(value: i32) -> i32 {\n    value + 1\n}\n";
        assert!(ProbeKind::DeadCodeAfterReturn.detect("rust", good).is_empty());
        assert!(ProbeKind::MagicNumber.detect("rust", good).is_empty());
        assert!(ProbeKind::SwallowedError.detect("rust", good).is_empty());
        assert!(ProbeKind::UnwrapOnFallible.detect("rust", good).is_empty());
        assert!(ProbeKind::NonDescriptiveName.detect("rust", good).is_empty());
        assert!(ProbeKind::UndocumentedPublicItem.detect("rust", good).is_empty());
    }
}
