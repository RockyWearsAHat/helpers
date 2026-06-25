//! `lint_match` — LOSSLESS rule matching. A rule is not a bag of features (which discards the
//! structure, and the discarded structure is exactly the false positives) but a generalized
//! sub-tree PATTERN taken from its own example, matched against code by EXACT sub-tree containment
//! with variable binding. Because the whole tree is kept, the relations deep rules need are already
//! present and require no per-relation code:
//!
//!   * **Scope** — "a `break` with no enclosing loop" is the tree path `function → block → break`
//!     with no loop node between; an in-loop break has `for → block → break` and simply does not
//!     match the pattern. Scope falls out of the path.
//!   * **Co-reference** — "the SAME variable in two `isinstance` calls" is one identifier node
//!     appearing in two positions; generalized to a BOUND wildcard, it matches only when both
//!     positions hold the same source text. Def-use falls out of binding.
//!
//! The essential pattern is isolated by `bad ∧ ¬good`: a sub-tree of the bad example whose SHAPE is
//! absent from the documented fix is the violation; shape shared with the fix is incidental context.
//! Operations/keywords/operators are kept exact; variables become bound wildcards; literals become
//! typed wildcards. Matching is then deterministic and exact — no statistics, no float.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser};

/// Resolve a language name to its tree-sitter grammar (mirrors `lint_ast`).
fn language(lang: &str) -> Option<tree_sitter::Language> {
    Some(match lang {
        "rust" => tree_sitter_rust::LANGUAGE.into(),
        "python" => tree_sitter_python::LANGUAGE.into(),
        "javascript" => tree_sitter_javascript::LANGUAGE.into(),
        "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "go" => tree_sitter_go::LANGUAGE.into(),
        "java" => tree_sitter_java::LANGUAGE.into(),
        "ruby" => tree_sitter_ruby::LANGUAGE.into(),
        "c" => tree_sitter_c::LANGUAGE.into(),
        "bash" => tree_sitter_bash::LANGUAGE.into(),
        _ => return None,
    })
}

/// A pattern node: a required AST shape.
///
/// * `text = Some(s)` — this node's source text must equal `s` exactly (an operation name, a
///   keyword, an operator: the part of the rule that is the rule).
/// * `text = None`, `bind = Some(id)` — a wildcard for an operand whose identity matters: it matches
///   any node of `kind`, but every wildcard sharing `id` must bind to the SAME source text
///   (co-reference — the same variable used twice).
/// * `text = None`, `bind = None` — a typed wildcard (any literal/operand of `kind`).
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Pat {
    kind: String,
    text: Option<String>,
    bind: Option<u32>,
    children: Vec<Pat>,
}

/// A compiled rule: the essential generalized pattern, plus the language it parses. Serializable, so
/// a packed module carries the exact pattern and reuses it anywhere with no recompilation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RulePattern {
    lang: String,
    pat: Pat,
}

/// An unnamed token that is a real OPERATOR (`..=`, `==`, `+`, `.`) rather than mere bracketing
/// punctuation (`(){}[],;`). Its `kind()` IS its text, so it both distinguishes `..=` from `..` and
/// needs no source to read. These carry meaning a rule turns on, so they are part of the structure.
fn is_operator_token(node: Node) -> bool {
    if node.is_named() {
        return false;
    }
    let k = node.kind();
    !k.is_empty() && k.len() <= 3 && k.chars().all(|c| c.is_ascii_punctuation()) && !"(){}[],;".contains(k)
}

/// The children that carry meaning: named nodes plus operator tokens, in source order. Bracketing
/// punctuation is dropped (noise). Shared by hashing, compiling, and matching so all three agree on
/// "what the tree IS".
fn meaningful_children<'t>(node: Node<'t>) -> Vec<Node<'t>> {
    let mut cur = node.walk();
    node.children(&mut cur).filter(|c| c.is_named() || is_operator_token(*c)).collect()
}

/// A canonical hash of a subtree's SHAPE — node kinds plus operators, in order — text-independent
/// for names/literals but KEEPING operators (so `0..=n` and `0..n` hash differently). Used to find
/// the part of the bad example whose shape the documented fix does NOT contain.
fn shape_hash(node: Node) -> String {
    let mut s = String::from(node.kind());
    let kids = meaningful_children(node);
    if !kids.is_empty() {
        s.push('(');
        for (i, k) in kids.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            s.push_str(&shape_hash(*k));
        }
        s.push(')');
    }
    s
}

/// Collect the shape hash of every subtree under `node`.
fn collect_shapes(node: Node, out: &mut HashSet<String>) {
    out.insert(shape_hash(node));
    for c in meaningful_children(node) {
        collect_shapes(c, out);
    }
}

/// For every node in the fix, its kind paired with the sorted multiset of its meaningful children's
/// shape hashes. This lets the localizer recognize a construct the fix kept intact but added
/// siblings INTO (a None-guard, an early return, a `try`/`except` wrap): such a node's own contents
/// are unchanged, so it is incidental context, not the violation — the change is in a sibling.
fn collect_child_shapes(node: Node, out: &mut Vec<(String, Vec<String>)>) {
    let kids = meaningful_children(node);
    let mut shapes: Vec<String> = kids.iter().map(|k| shape_hash(*k)).collect();
    shapes.sort();
    out.push((node.kind().to_string(), shapes));
    for c in kids {
        collect_child_shapes(c, out);
    }
}

/// Whether the sorted multiset `sub` is contained in the sorted multiset `sup`.
fn is_submultiset(sub: &[String], sup: &[String]) -> bool {
    let mut counts: HashMap<&String, i32> = HashMap::new();
    for s in sup {
        *counts.entry(s).or_default() += 1;
    }
    sub.iter().all(|s| match counts.get_mut(s) {
        Some(c) if *c > 0 => {
            *c -= 1;
            true
        }
        _ => false,
    })
}

/// True when the fix kept this exact construct and only INSERTED siblings inside it: some fix node of
/// the same kind has a child-shape multiset that STRICTLY contains this node's. The construct's own
/// children are all preserved by the fix, so the violation is not here — it is in a sibling subtree
/// (the `target=[]` default that sits beside the body the fix only wrapped in a guard). Without this,
/// a fix that adds a guard makes the body itself look novel, and the localizer over-captures the
/// whole unit into a pattern so literal that a stray docstring or log line defeats the match.
fn fix_only_inserted(node: Node, good_children: &[(String, Vec<String>)]) -> bool {
    let kids = meaningful_children(node);
    if kids.is_empty() {
        return false;
    }
    let mut want: Vec<String> = kids.iter().map(|k| shape_hash(*k)).collect();
    want.sort();
    good_children
        .iter()
        .any(|(kind, have)| kind == node.kind() && have.len() > want.len() && is_submultiset(&want, have))
}

/// The SMALLEST subtree of `node` carrying the distinction from the fix: the deepest named node that
/// is novel (its shape is absent from `good_shapes`) yet sits over children the fix DOES share — so
/// the difference is localized right here. This is what isolates `0..=W.len()` from a whole function
/// (the operator diff would otherwise bubble all the way up), while still keeping the function scope
/// for a `break` (because the break-block shape IS shared with the loop fix, descent stops above it).
fn novel_root<'t>(
    node: Node<'t>,
    good_shapes: &HashSet<String>,
    good_kinds: &HashSet<String>,
    good_children: &[(String, Vec<String>)],
) -> Option<Node<'t>> {
    if good_shapes.contains(&shape_hash(node)) {
        return None; // shape shared with the fix → incidental context, not the violation
    }
    if fix_only_inserted(node, good_children) {
        return None; // the fix only added siblings here → this construct is not the violation
    }
    let mut cur = node.walk();
    let novel: Vec<Node> = node
        .named_children(&mut cur)
        .filter(|c| novel_root(*c, good_shapes, good_kinds, good_children).is_some())
        .collect();
    // Descend into the single differing child ONLY when this node's KIND survives in the fix — i.e.
    // the construct is preserved and only its content changed (a `range_expression` `..=`→`..`). If
    // the fix REPLACED this kind (a `lambda` assignment became a `def`, so `assignment` is absent
    // from the fix), the construct itself is the violation — keep it, don't strip its context. Zero
    // or several novel children ⇒ the change is at/across this node ⇒ stop here.
    // A call is atomic — its callee IS the rule's identity (`range`, `re.sub`); never strip it by
    // descending into its arguments. So stop at a call even if the change is in an argument.
    let atomic = matches!(node.kind(), "call" | "call_expression" | "macro_invocation");
    if novel.len() == 1 && good_kinds.contains(node.kind()) && !atomic {
        novel_root(novel[0], good_shapes, good_kinds, good_children)
    } else {
        Some(node)
    }
}

/// With no fix to diff against, a documented bad example often shows the SAME anti-pattern more than
/// once (clippy lists several instances: `if x == true {}` / `if y == false {}`). Keeping the whole
/// multi-statement root then builds a pattern that demands every instance at once — so brittle that
/// not even the example's own reuse matches it. When every meaningful child of `node` shares one
/// shape, they ARE the one violation repeated, so descend to a single representative instance. A node
/// whose children differ in shape is left intact (we cannot localize a real difference without a fix).
fn collapse_repeated(node: Node) -> Node {
    let kids = meaningful_children(node);
    if kids.len() < 2 {
        return node;
    }
    let first = shape_hash(kids[0]);
    if kids.iter().all(|k| shape_hash(*k) == first) {
        return collapse_repeated(kids[0]);
    }
    node
}

/// Collect every node kind under `node`.
fn collect_kinds(node: Node, out: &mut HashSet<String>) {
    out.insert(node.kind().to_string());
    for c in meaningful_children(node) {
        collect_kinds(c, out);
    }
}

/// True when an identifier node names an OPERATION (kept exact), not an operand (generalized): the
/// `function` of a call, an attribute/field/method name, a macro name. Everything else that is an
/// identifier is a variable/operand and becomes a bound wildcard. This is the one general rule that
/// decides "what is the rule" vs "what is incidental", with no per-rule knowledge.
fn is_operation_name(node: Node) -> bool {
    let Some(parent) = node.parent() else { return false };
    let is_field = |names: &[&str]| {
        names
            .iter()
            .find_map(|n| parent.child_by_field_name(n))
            .map(|f| f.id())
            == Some(node.id())
    };
    match parent.kind() {
        // The accessed member is the operation; the receiver (`xs` in `xs.len()`) is an operand.
        "attribute" | "field_expression" | "member_expression" => is_field(&["attribute", "field", "property"]),
        // `f(...)` — the callee is the operation, the arguments are operands.
        "call" | "call_expression" => is_field(&["function"]),
        "scoped_identifier" => is_field(&["name"]),
        "macro_invocation" => true,
        _ => false,
    }
}

/// Identifier-like node kinds whose text names a variable/operand (candidate for a bound wildcard).
fn is_identifier_kind(kind: &str) -> bool {
    matches!(kind, "identifier" | "type_identifier" | "field_identifier" | "shorthand_property_identifier")
}

/// Literal node kinds whose VALUE is incidental — generalized to a typed wildcard (any literal of
/// that kind), so a rule about `"…".join(...)` is not pinned to the example's exact string.
fn is_literal_kind(kind: &str) -> bool {
    kind.contains("string") || kind.contains("integer") || kind.contains("float") || kind.contains("number")
}

/// True when `text` is named by the rule's `desc` (lowercased) — the rule is explicitly ABOUT this
/// value (`"0.0.0.0"`, the `xml.sax` module), so it is essential and kept exact, not generalized.
/// This is the docs themselves disambiguating "the value IS the rule" from "the value is incidental".
fn named_in_desc(text: &str, desc: &str) -> bool {
    let t = text.trim_matches(|c| c == '"' || c == '\'' || c == '`').to_lowercase();
    t.len() >= 2 && desc.contains(&t)
}

/// Compile a code node into a generalized pattern. Operands (variables) become bound wildcards;
/// operations/keywords/operators stay exact; literals are typed wildcards UNLESS the rule's `desc`
/// names their value (then they are essential and kept exact). `binds` co-references repeated vars.
fn compile(node: Node, src: &[u8], desc: &str, binds: &mut HashMap<String, u32>) -> Pat {
    let kind = node.kind().to_string();
    // An operator token's kind IS its text, so a kind match alone pins it exactly — a typed
    // wildcard of that kind matches only that operator.
    if is_operator_token(node) {
        return Pat { kind, text: None, bind: None, children: Vec::new() };
    }
    let own_text = node.utf8_text(src).unwrap_or("");

    // A literal is one leaf (a string in Python's grammar has start/content/end children — descend
    // and they would be generalized away). Its VALUE matters only when the rule NAMES it (`0.0.0.0`),
    // else any literal of the kind matches.
    if is_literal_kind(&kind) {
        let text = named_in_desc(own_text, desc).then(|| own_text.to_string());
        return Pat { kind, text, bind: None, children: Vec::new() };
    }
    // A bare operand identifier → bound wildcard (co-reference by name) UNLESS the rule names it
    // (e.g. an imported module `xml.sax`), in which case it is the rule's subject and kept exact.
    if is_identifier_kind(&kind) && !is_operation_name(node) {
        if named_in_desc(own_text, desc) {
            return Pat { kind, text: Some(own_text.to_string()), bind: None, children: Vec::new() };
        }
        let next = binds.len() as u32;
        let id = *binds.entry(own_text.to_string()).or_insert(next);
        return Pat { kind, text: None, bind: Some(id), children: Vec::new() };
    }
    let kids = meaningful_children(node);
    // Leaf with meaning (operation name, keyword): keep its exact text.
    let text = if kids.is_empty() {
        Some(own_text.trim().to_string()).filter(|t| !t.is_empty())
    } else {
        None
    };
    let children = kids.iter().map(|c| compile(*c, src, desc, binds)).collect();
    Pat { kind, text, bind: None, children }
}

/// A collection-literal node kind (`[]`, `{}`, `(a, b)`) across the grammars we parse. Unlike a bare
/// identifier or operator, a collection literal is a CONCRETE construct a rule can turn on (a mutable
/// default argument, a list where a generator belongs), so it counts as an anchoring identity even as
/// a typed wildcard — the rule is "a value of this kind in this slot".
fn is_container_kind(kind: &str) -> bool {
    matches!(kind, "list" | "dictionary" | "set" | "tuple" | "array" | "object" | "array_expression")
}

/// Whether `pat` keeps at least one anchoring IDENTITY — a leaf whose retained text carries a word (a
/// method/operation name, a keyword, or a doc-named literal like `0.0.0.0`), or a collection literal
/// ([`is_container_kind`]). Operators and punctuation are exact-by-kind but not an identity (too
/// common), so they do not count. A pattern with no anchor is pure structure-plus-wildcards and would
/// match a generic shape; it has no rule to match and abstains. (A pattern that does anchor but is
/// still too broad is caught downstream by the self-test against the docs' own good examples.)
fn has_named_anchor(pat: &Pat) -> bool {
    let word = pat.text.as_deref().is_some_and(|t| t.chars().any(|c| c.is_ascii_alphanumeric()));
    word || is_container_kind(&pat.kind) || pat.children.iter().any(has_named_anchor)
}

impl RulePattern {
    /// Build a rule pattern from its documented `bad` example and (optional) `good` fix, in `lang`.
    /// `desc` is the rule's English description: a literal/name it mentions is kept exact (the rule
    /// is about that value), everything else generalizes. Returns `None` when the example does not
    /// parse or carries no distinctive structure.
    pub fn compile(lang: &str, bad: &str, good: &str, desc: &str) -> Option<RulePattern> {
        let language = language(lang)?;
        let mut parser = Parser::new();
        parser.set_language(&language).ok()?;
        let bad_tree = parser.parse(bad, None)?;
        let mut good_shapes = HashSet::new();
        let mut good_kinds = HashSet::new();
        let mut good_children = Vec::new();
        if !good.trim().is_empty() {
            if let Some(gt) = parser.parse(good, None) {
                collect_shapes(gt.root_node(), &mut good_shapes);
                collect_kinds(gt.root_node(), &mut good_kinds);
                collect_child_shapes(gt.root_node(), &mut good_children);
            }
        }
        // With a fix to diff against, isolate the smallest distinguishing construct. With no fix,
        // we cannot localize — keep the whole bad construct (its context, e.g. a `break`'s scope).
        let root = if good_shapes.is_empty() {
            collapse_repeated(bad_tree.root_node())
        } else {
            novel_root(bad_tree.root_node(), &good_shapes, &good_kinds, &good_children)?
        };
        // Skip past trivial single-child wrappers (module / expression_statement) to the construct.
        let mut node = root;
        while node.named_child_count() == 1 && matches!(node.kind(), "module" | "program" | "source_file" | "expression_statement" | "block") {
            node = node.named_child(0).unwrap();
        }
        let mut binds = HashMap::new();
        let pat = compile(node, bad.as_bytes(), &desc.to_lowercase(), &mut binds);
        // A pattern that is a lone wildcard or a single bare leaf carries no rule — abstain.
        if pat.children.is_empty() && pat.text.is_none() {
            return None;
        }
        // The rule's IDENTITY is the named tokens it turns on — a method/operation name, a keyword,
        // or a literal the docs name (`len`, `true`, `break`, `re.sub`, `0.0.0.0`). A pattern that
        // generalized down to pure structure plus wildcards, with no such anchor, matches a generic
        // shape (`let x = a::b::c` for `absolute_paths`) rather than its own rule — it has no
        // identity to match, so it abstains. Operators/punctuation alone (`::`, `=`) are not an
        // identity: they are too common to be the rule. This is the docs deciding what is learnable
        // from a single example: a rule whose essence is types or dataflow leaves no syntactic
        // anchor and is correctly not learned here.
        if !has_named_anchor(&pat) {
            return None;
        }
        Some(RulePattern { lang: lang.to_string(), pat })
    }

    /// Every 1-based line in `code` where the rule's pattern occurs (exact sub-tree match with
    /// consistent variable binding). Empty when the rule does not apply — deterministically.
    pub fn matches(&self, code: &str) -> Vec<usize> {
        let Some(language) = language(&self.lang) else { return Vec::new() };
        let mut parser = Parser::new();
        if parser.set_language(&language).is_err() {
            return Vec::new();
        }
        let Some(tree) = parser.parse(code, None) else { return Vec::new() };
        let mut hits = Vec::new();
        find(tree.root_node(), &self.pat, code.as_bytes(), &mut hits);
        hits
    }
}

/// Try the pattern at `node` and recurse into children, collecting match lines.
fn find(node: Node, pat: &Pat, src: &[u8], hits: &mut Vec<usize>) {
    let mut binds: HashMap<u32, String> = HashMap::new();
    if match_at(node, pat, src, &mut binds) {
        hits.push(node.start_position().row + 1);
    }
    let mut cur = node.walk();
    for c in node.children(&mut cur) {
        find(c, pat, src, hits);
    }
}

/// Exact match of one pattern node against one code node, threading variable bindings.
fn match_at(node: Node, pat: &Pat, src: &[u8], binds: &mut HashMap<u32, String>) -> bool {
    if node.kind() != pat.kind {
        return false;
    }
    if let Some(id) = pat.bind {
        // A bound wildcard: any node of this kind, but the same id must always be the same text.
        let text = node.utf8_text(src).unwrap_or("").to_string();
        return match binds.get(&id) {
            Some(prev) => prev == &text,
            None => {
                binds.insert(id, text);
                true
            }
        };
    }
    if let Some(t) = &pat.text {
        return node.utf8_text(src).map(|x| x.trim() == t).unwrap_or(false);
    }
    if pat.children.is_empty() {
        return true; // typed wildcard (any literal/operand/operator of this kind)
    }
    // Structural node: its meaningful children must match the pattern's children in order.
    let kids = meaningful_children(node);
    if kids.len() != pat.children.len() {
        return false;
    }
    kids.iter().zip(&pat.children).all(|(c, p)| match_at(*c, p, src, binds))
}

/// One documented rule compiled to its exact pattern, carrying the reporting facts a finding needs.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct CompiledRule {
    id: String,
    severity: String,
    pat: RulePattern,
}

/// A language's compiled rule set: every documented rule reduced to its lossless tree pattern. This
/// is the cached, serializable model a lint run loads and matches each file against — deterministic,
/// no thresholds, no statistics. Mirrors the engine's old model API so judging code is unchanged.
#[derive(Serialize, Deserialize)]
pub struct RuleSet {
    /// Language id (e.g. `rust`).
    pub lang: String,
    rules: Vec<CompiledRule>,
}

/// One flagged violation: the rule it violates, that rule's severity, and the 1-based source line.
pub struct Finding {
    /// The matched rule's id.
    pub rule: String,
    /// Severity bucket (`high`/`medium`/`low`).
    pub severity: String,
    /// 1-based source line of the match.
    pub line: usize,
}

impl RuleSet {
    /// Compile a language's documented `(id, severity, bad, good, description)` rules into exact
    /// patterns. A rule whose example carries no distinctive structure (its difference is data the
    /// tree abstracts away) compiles to nothing and is dropped — the docs decide what is learnable.
    pub fn build(lang: &str, rules: &[(String, String, String, String, String)]) -> RuleSet {
        let mut compiled = Vec::new();
        let mut seen = HashSet::new();
        for (id, severity, bad, good, desc) in rules {
            if id.is_empty() || bad.trim().is_empty() || !seen.insert(id.clone()) {
                continue;
            }
            if let Some(pat) = RulePattern::compile(lang, bad, good, desc) {
                compiled.push(CompiledRule { id: id.clone(), severity: severity.clone(), pat });
            }
        }
        // SELF-FIRE: a compiled pattern must flag its own `bad` example. A pattern that does not
        // self-fire is broken by construction — the extraction failed to represent the violation —
        // and would be a guaranteed false negative on the simplest possible case. Drop such rules
        // before they reach the good-corpus test; keeping them would corrupt the ruleset silently.
        let bad_map: std::collections::HashMap<&str, &str> =
            rules.iter().map(|(id, _, bad, _, _)| (id.as_str(), bad.as_str())).collect();
        compiled.retain(|r| {
            let bad = bad_map.get(r.id.as_str()).copied().unwrap_or("");
            !r.pat.matches(bad).is_empty()
        });
        // OVER-FIRE: drop rules whose pattern fires on any doc-certified clean example. Such a rule
        // is flagging correct code — its violation is semantic (dataflow, ownership) not syntactic.
        // The docs' own `good` examples form the reference; no external corpus is needed.
        let good_corpus: Vec<&str> =
            rules.iter().map(|(_, _, _, g, _)| g.trim()).filter(|g| !g.is_empty()).collect();
        compiled.retain(|r| !good_corpus.iter().any(|g| !r.pat.matches(g).is_empty()));
        RuleSet { lang: lang.to_string(), rules: compiled }
    }

    /// Number of compiled rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Flag `code`: every line where a rule's exact pattern occurs, deduped per rule. A file with no
    /// match yields nothing — the match is deterministic sub-tree containment, so there is no
    /// project calibration and no shared state between files.
    pub fn flag(&self, code: &str) -> Vec<Finding> {
        let mut out = Vec::new();
        for r in &self.rules {
            let mut lines = r.pat.matches(code);
            lines.sort_unstable();
            lines.dedup();
            for line in lines {
                out.push(Finding { rule: r.id.clone(), severity: r.severity.clone(), line });
            }
        }
        out
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The model end-to-end: a documented rule compiles, flags its bad form, clears its good form.
    #[test]
    fn ruleset_flags_bad_clears_good() {
        let rules = vec![(
            "bool_comparison".to_string(),
            "low".to_string(),
            "fn f(x: bool) { if x == true {} }".to_string(),
            "fn f(x: bool) { if x {} }".to_string(),
            "Comparing a bool to true is redundant.".to_string(),
        )];
        let m = RuleSet::build("rust", &rules);
        assert_eq!(m.rule_count(), 1, "the rule compiles to a pattern");
        let fires = |code: &str| m.flag(code).iter().any(|f| f.rule == "bool_comparison");
        assert!(fires("fn g(y: bool) { if y == true {} }"), "flags the violation (any variable)");
        assert!(!fires("fn g(y: bool) { if y {} }"), "clears the fixed form");
    }

    #[test]
    fn scope_falls_out_of_the_tree_break_outside_loop() {
        // The rule is taught by example: a `break` directly in a function (no loop). The fix puts
        // it in a loop. The pattern keeps the SCOPE path, so it matches a bare-function break and
        // NOT an in-loop break — with zero scope-specific code.
        let rule = RulePattern::compile(
            "python",
            "def f():\n    break",
            "def f():\n    for x in xs:\n        break", "break statements outside of loops",
        )
        .expect("rule compiles");
        assert!(!rule.matches("def g():\n    break").is_empty(), "break with no loop is flagged");
        assert!(
            rule.matches("def h():\n    for y in ys:\n        break").is_empty(),
            "break inside a loop is NOT flagged (scope from the tree path)"
        );
    }

    #[test]
    fn co_reference_falls_out_of_binding_isinstance_or() {
        // The rule: the SAME target in two `isinstance` calls joined by `or`. Co-reference is just
        // one variable appearing twice → a bound wildcard. No def-use engine.
        let rule = RulePattern::compile(
            "python",
            "isinstance(x, A) or isinstance(x, B)",
            "isinstance(x, (A, B))",
            "multiple isinstance calls on the same target",
        )
        .expect("rule compiles");
        assert!(
            !rule.matches("if isinstance(item, dict) or isinstance(item, list):\n    pass").is_empty(),
            "same target in two isinstance/or is flagged"
        );
        assert!(
            rule.matches("if isinstance(item, dict) and item.get('k'):\n    pass").is_empty(),
            "a single isinstance with `and` is NOT flagged (structure + operator are exact)"
        );
        assert!(
            rule.matches("if isinstance(a, dict) or isinstance(b, list):\n    pass").is_empty(),
            "DIFFERENT targets are NOT flagged (binding requires the same variable)"
        );
    }

    #[test]
    fn fix_that_adds_a_guard_localizes_to_the_violation_not_the_whole_unit() {
        // The fix for a mutable default argument both changes the param (`[]`→`None`) AND inserts a
        // None-guard into the body. A naive diff sees two novel sites and captures the WHOLE function
        // verbatim — a pattern so literal a stray docstring defeats it. The localizer must see the
        // body as fix-inserted context and pin the rule to the `target=[]` default itself.
        let rule = RulePattern::compile(
            "python",
            "def append_item(item, target=[]):\n    target.append(item)\n    return target",
            "def append_item(item, target=None):\n    if target is None:\n        target = []\n    target.append(item)\n    return target",
            "A mutable default argument is shared across calls",
        )
        .expect("a collection-literal default is a learnable rule");
        // Real code with a docstring and an extra statement must still match (the over-fit pattern did not).
        assert!(
            !rule.matches("def f(x, acc=[]):\n    \"\"\"doc.\"\"\"\n    acc.append(x)\n    log(x)\n    return acc").is_empty(),
            "flags a mutable default regardless of surrounding body"
        );
        // The idiomatic None-default form is clean.
        assert!(
            rule.matches("def f(x, acc=None):\n    if acc is None:\n        acc = []\n    acc.append(x)\n    return acc").is_empty(),
            "the None-default fix is not flagged"
        );
    }

    #[test]
    fn empty_good_repeated_instances_collapse_to_one_and_fire() {
        // The clippy seed shows `bool_comparison` as the same anti-pattern twice with NO fix:
        // `if x == true {}` / `if y == false {}`. The whole-tree pattern demanded both at once, so
        // not even a single real use (or the example's own first line) matched. Collapsing repeated
        // instances must yield a pattern that fires on one ordinary comparison.
        let rule = RulePattern::compile("rust", "if x == true {}\nif y == false {}", "", "Comparing a bool to true is redundant.")
            .expect("repeated-instance example compiles to a single pattern");
        assert!(!rule.matches("fn g(flag: bool) { if flag == true {} }").is_empty(), "flags a real == true use");
        assert!(rule.matches("fn g(flag: bool) { if flag {} }").is_empty(), "the idiomatic form is clean");
    }

    #[test]
    fn operation_name_is_exact_not_a_wildcard() {
        // `re.sub` with a literal pattern → use str.replace. The operation `.sub` is kept exact; the
        // string is a typed wildcard, so any `re.sub("…", …)` matches but `.replace(` does not.
        let rule = RulePattern::compile("python", "re.sub(\"abc\", \"\", s)", "s.replace(\"abc\", \"\")", "unnecessary regular expression")
            .expect("rule compiles");
        assert!(!rule.matches("y = re.sub(\"x\", \"\", text)").is_empty(), "any re.sub literal call matches");
        assert!(rule.matches("y = text.replace(\"x\", \"\")").is_empty(), "the fixed form does not match");
    }

    /// Zero-false-negatives property: every rule that survives build() fires on its own bad example.
    /// This is guaranteed by construction (build() drops non-self-firing rules), but this test makes
    /// the invariant explicit and catches any regression in the self-fire gate.
    #[test]
    fn every_compiled_rule_fires_on_its_own_bad_example() {
        let rules = vec![
            ("bool_comparison".to_string(), "low".to_string(),
             "fn f(x: bool) { if x == true {} }".to_string(),
             "fn f(x: bool) { if x {} }".to_string(),
             "Comparing a bool to true is redundant.".to_string()),
            // A semantically-unlearnable rule: the bad == good (over-fires the good corpus) — build()
            // must drop it. Including it proves the self-fire gate alone does not keep bad rules.
            ("always_fires".to_string(), "high".to_string(),
             "fn f() { let x = 1; }".to_string(),
             "fn f() { let x = 1; }".to_string(),
             "Fires on correct code too.".to_string()),
        ];
        let m = RuleSet::build("rust", &rules);
        // Only `bool_comparison` survives: it self-fires AND doesn't fire on the good corpus.
        assert_eq!(m.rule_count(), 1, "only the self-firing, non-over-firing rule compiles");
        // The surviving rule fires on its bad example — the zero-FN guarantee.
        let flags = m.flag("fn g(y: bool) { if y == true {} }");
        assert!(flags.iter().any(|f| f.rule == "bool_comparison"), "surviving rule self-fires");
    }
}
