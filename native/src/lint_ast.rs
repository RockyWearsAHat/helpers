//! `lint_ast` — derive checks from the docs, don't hand-write them.
//!
//! The maintenance trap is writing one check per rule: that means re-coding every
//! rule for every language and every toolchain version — by hand, forever. This
//! module never does that. There is **no rule name anywhere in this file**.
//!
//! Instead: tree-sitter parses any language from its (upstream-maintained) grammar,
//! and each documented rule already ships an `exampleBad` and an `exampleGood`. The
//! AST features present in the *bad* example but absent from the *good* one are, by
//! construction, the structural thing the rule is about — its **signature**. We flag
//! target code only where it exhibits that signature. Adding a language or a new tool
//! version is then pure data: drop in the scraped docs and the signatures re-derive
//! themselves. A signature that doesn't actually separate bad from good (empty diff)
//! is discarded, so a rule we can't ground in its own examples never fires.

use std::collections::BTreeSet;
use tree_sitter::{Node, Parser};

/// The tree-sitter language for `lang`, or `None` if we have no grammar for it.
fn language(lang: &str) -> Option<tree_sitter::Language> {
    Some(match lang {
        "rust" => tree_sitter_rust::LANGUAGE.into(),
        "python" => tree_sitter_python::LANGUAGE.into(),
        "javascript" => tree_sitter_javascript::LANGUAGE.into(),
        "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "go" => tree_sitter_go::LANGUAGE.into(),
        _ => return None,
    })
}

/// One documented rule reduced to what checking needs: its id and the two examples.
/// This is exactly what the scraped `lint-index/<tool>.json` already stores.
pub struct RuleExample {
    /// The rule's stable id (e.g. `bool_comparison`), carried through to the hit.
    pub id: String,
    /// Code the rule says is wrong.
    pub bad: String,
    /// The corrected form of that same code.
    pub good: String,
}

/// One violation: the source line and the id of the rule whose signature matched.
pub struct AstHit {
    /// 1-based source line of the matched feature.
    pub line: usize,
    /// The id of the rule that flagged it.
    pub rule_id: String,
}

/// A rule's structural signature: AST features in its bad example but not its good
/// one. Empty ⇒ the examples don't structurally differ ⇒ the rule can't be grounded
/// and is skipped (never a blind flag).
struct Signature {
    id: String,
    features: BTreeSet<String>,
}

/// Parse the source once into a positional feature list, then flag every rule whose
/// signature is wholly present. Language/version-agnostic: `rules` is just the docs.
pub fn check(lang: &str, source: &str, rules: &[RuleExample]) -> Vec<AstHit> {
    let signatures: Vec<Signature> = rules.iter().filter_map(|r| derive(lang, r)).collect();
    if signatures.is_empty() {
        return Vec::new();
    }
    let feats = node_features(lang, source);
    let present: BTreeSet<&str> = feats.iter().map(|(f, _)| f.as_str()).collect();
    let mut hits = Vec::new();
    for sig in &signatures {
        if sig.features.iter().all(|f| present.contains(f.as_str())) {
            // Report at the first line carrying any of the signature's features.
            if let Some((_, line)) = feats
                .iter()
                .find(|(f, _)| sig.features.contains(f.as_str()))
            {
                hits.push(AstHit {
                    line: *line,
                    rule_id: sig.id.clone(),
                });
            }
        }
    }
    hits
}

/// Derive a rule's signature from its examples: features(bad) − features(good). Returns
/// `None` when the diff is empty (the examples don't structurally differ, so we can't
/// ground a check) — that's the self-validation gate, in one line.
fn derive(lang: &str, rule: &RuleExample) -> Option<Signature> {
    if rule.bad.is_empty() || rule.good.is_empty() {
        return None;
    }
    let bad: BTreeSet<String> = node_features(lang, &rule.bad).into_iter().map(|(f, _)| f).collect();
    let good: BTreeSet<String> = node_features(lang, &rule.good).into_iter().map(|(f, _)| f).collect();
    let features: BTreeSet<String> = bad.difference(&good).cloned().collect();
    if features.is_empty() {
        return None;
    }
    Some(Signature {
        id: rule.id.clone(),
        features,
    })
}

/// Extract structural features with their source lines. A "feature" is a salient,
/// position-independent shape of a node — a called method (`call:unwrap`), a macro
/// (`macro:panic`), a comparison against a literal (`cmp:==bool`, `cmp:==empty-str`),
/// a self-comparison (`cmp:self`), a double negation (`unary:double-not`). Because
/// these come from parsed nodes, the same text inside a string or comment is a
/// different node kind and yields no feature — false positives can't arise from that.
pub fn node_features(lang: &str, source: &str) -> Vec<(String, usize)> {
    let Some(language) = language(lang) else {
        return Vec::new();
    };
    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };
    let mut feats = Vec::new();
    collect(tree.root_node(), source.as_bytes(), &mut feats);
    feats
}

fn collect(node: Node, src: &[u8], feats: &mut Vec<(String, usize)>) {
    let line = node.start_position().row + 1;
    for f in salient_features(node, src) {
        feats.push((f, line));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, src, feats);
    }
}

/// The salient, position-independent features of a single node: a called method
/// (`call:unwrap`), a macro (`macro:panic`), a comparison against a literal or itself
/// (`cmp:==bool`, `cmp:self`), a double negation (`unary:double-not`). Returns the empty
/// vec for nodes with no salient feature. Shared by [`node_features`] (which keys rule
/// signatures) and [`structural_tokens`] (which feeds the learned linter), so both see the
/// same notion of "what matters" in a node.
fn salient_features(node: Node, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    match node.kind() {
        "call_expression" | "call" => {
            if let Some(func) = node.child_by_field_name("function") {
                if matches!(func.kind(), "field_expression" | "attribute" | "member_expression") {
                    if let Some(field) = func
                        .child_by_field_name("field")
                        .or_else(|| func.child_by_field_name("attribute"))
                        .or_else(|| func.child_by_field_name("property"))
                    {
                        if let Ok(m) = field.utf8_text(src) {
                            out.push(format!("call:{m}"));
                        }
                    }
                }
            }
        }
        "macro_invocation" => {
            if let Some(mac) = node.child_by_field_name("macro") {
                if let Ok(m) = mac.utf8_text(src) {
                    out.push(format!("macro:{m}"));
                }
            }
        }
        "binary_expression" | "comparison_operator" => {
            if let Some((op, l, r)) = comparison(node, src) {
                if matches!(op.as_str(), "==" | "!=") {
                    let bool_lit = |s: &str| matches!(s, "true" | "false" | "True" | "False");
                    let empty_str = |s: &str| matches!(s, "\"\"" | "''" | "``");
                    if bool_lit(&l) || bool_lit(&r) {
                        out.push(format!("cmp:{op}bool"));
                    }
                    if empty_str(&l) || empty_str(&r) {
                        out.push(format!("cmp:{op}empty-str"));
                    }
                    if l == r && !l.is_empty() {
                        out.push("cmp:self".into());
                    }
                }
            }
        }
        "unary_expression" | "not_operator" => {
            if let Some(operand) = node.named_child(0) {
                let is_not = |s: &str| s.starts_with('!') || s.starts_with("not");
                let outer = node.utf8_text(src).unwrap_or("");
                let inner = operand.utf8_text(src).unwrap_or("");
                if is_not(outer)
                    && matches!(operand.kind(), "unary_expression" | "not_operator")
                    && is_not(inner)
                {
                    out.push("unary:double-not".into());
                }
            }
        }
        // Nested generics as a SINGLE relational feature: `gen:Vec<Box>` vs `gen:Box<Vec>`.
        // Without this, `Vec<Box<_>>` and `Box<Vec<_>>` both merely "contain Vec and Box" and
        // look identical to a flat model; the nesting order is the whole distinction.
        "generic_type" => {
            if let (Some(outer), Some(args)) = (
                node.child_by_field_name("type"),
                node.child_by_field_name("type_arguments"),
            ) {
                if let Some(o) = type_head(outer, src) {
                    let mut cursor = args.walk();
                    for inner in args.named_children(&mut cursor) {
                        if matches!(inner.kind(), "generic_type" | "type_identifier" | "scoped_type_identifier") {
                            if let Some(i) = type_head(inner, src) {
                                out.push(format!("gen:{o}<{i}>"));
                            }
                            break;
                        }
                    }
                }
            }
        }
        // A `match`/`if` arm whose body is the no-op unit `()` vs a real expression is the
        // structural line between e.g. single_match (`_ => ()`) and single_match_else
        // (`_ => bar()`). Emit the wildcard/else arm's body shape so they separate.
        "match_arm" => {
            let is_wild = node
                .child_by_field_name("pattern")
                .map(|p| p.utf8_text(src).map(|t| t.trim() == "_").unwrap_or(false))
                .unwrap_or(false);
            if is_wild {
                if let Some(body) = node.child_by_field_name("value") {
                    let shape = if matches!(body.kind(), "unit_expression") { "unit" } else { body.kind() };
                    out.push(format!("wildarm:{shape}"));
                }
            }
        }
        _ => {}
    }
    out
}

/// The head type name of a type node — `Vec` for `Vec<T>`, `Box` for `Box<U>`, the last
/// segment of a scoped path — so nested generics reduce to a comparable `Outer<Inner>` shape.
fn type_head(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "type_identifier" | "primitive_type" => node.utf8_text(src).ok().map(str::to_string),
        "generic_type" => node.child_by_field_name("type").and_then(|t| type_head(t, src)),
        "scoped_type_identifier" => node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(src).ok())
            .map(str::to_string),
        _ => node.utf8_text(src).ok().map(|s| s.rsplit("::").next().unwrap_or(s).to_string()),
    }
}

/// Linearize `code` into a **structural token stream** for the learned linter: a pre-order
/// walk emitting each named AST node's *kind* (`match_expression`, `else_clause`,
/// `string_literal`, `identifier`, …) plus the salient features above. This is the
/// representation that lets the model reason over STRUCTURE rather than surface text —
/// `match` with an `else_clause` differs from one without it, and `println!("{}", "x")`
/// (a `string_literal` arg) differs from `println!("{}", x)` (an `identifier` arg), so
/// sibling rules and literal-vs-variable cases become separable. Variable *names* are
/// deliberately dropped (only node kinds + salient names survive), so it generalizes
/// instead of memorizing identifiers. Empty for languages with no grammar.
pub fn structural_tokens(lang: &str, code: &str) -> Vec<(String, usize)> {
    let Some(language) = language(lang) else {
        return Vec::new();
    };
    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(code, None) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    structural_collect(tree.root_node(), code.as_bytes(), &mut out);
    out
}

fn structural_collect(node: Node, src: &[u8], out: &mut Vec<(String, usize)>) {
    if node.is_named() {
        let line = node.start_position().row + 1;
        out.push((node.kind().to_string(), line));
        // Emit the TEXT of API-level names — types, primitives, fields — because they are
        // structural (shared vocabulary, not variable names): `Vec<Box<_>>` vs `Box<Vec<_>>`
        // is the SAME node kinds in a different order, separable only once the names appear in
        // pre-order. Local variable identifiers are intentionally left as the bare kind so the
        // model generalizes instead of memorizing them.
        if matches!(node.kind(), "type_identifier" | "primitive_type" | "field_identifier") {
            if let Ok(name) = node.utf8_text(src) {
                out.push((format!("name:{name}"), line));
            }
        }
        for f in salient_features(node, src) {
            out.push((f, line));
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        structural_collect(child, src, out);
    }
}

/// A **fully generic** structural encoding — ZERO per-rule or per-node-kind special cases. Each
/// named node becomes a label (`kind:head`, the head pulled from whatever naming field the
/// grammar exposes) plus a parent→child edge to its context. Every structural distinction is
/// therefore present (nesting order shows up as edges, arm shape as a child kind, …); which ones
/// MATTER is not decided here — it is learned downstream by weighting features by how rare they
/// are in the language corpus (common grammar ≈ noise, rare structure ≈ meaning). This is the
/// "read the docs, learn the language" path: no hand-written rules, just the tree + statistics.
pub fn generic_features(lang: &str, code: &str) -> Vec<(String, usize)> {
    let Some(language) = language(lang) else {
        return Vec::new();
    };
    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(code, None) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    generic_walk(tree.root_node(), None, code.as_bytes(), &mut out);
    out
}

fn generic_walk(node: Node, parent: Option<&str>, src: &[u8], out: &mut Vec<(String, usize)>) {
    let mut here = parent.map(str::to_string);
    if node.is_named() {
        let line = node.start_position().row + 1;
        let label = generic_label(node, src);
        out.push((label.clone(), line));
        if let Some(p) = parent {
            out.push((format!("{p}>{label}"), line));
        }
        here = Some(label);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        generic_walk(child, here.as_deref(), src, out);
    }
}

/// A node's label: its kind plus its head identifier when [`generic_head`] finds one. A leaf
/// that is neither a variable `identifier` nor a literal *value* keeps its text (types, fields,
/// keywords, operators carry structural identity); variables and literal values reduce to kind.
fn generic_label(node: Node, src: &[u8]) -> String {
    let kind = node.kind();
    if let Some(head) = generic_head(node, src) {
        return format!("{kind}:{head}");
    }
    if node.named_child_count() == 0
        && kind != "identifier"
        && kind != "string_content"
        && !kind.ends_with("_literal")
    {
        if let Ok(t) = node.utf8_text(src) {
            let t = t.trim();
            if !t.is_empty() && t.len() <= 24 {
                return format!("{kind}:{t}");
            }
        }
    }
    kind.to_string()
}

/// The head identifier of a node via the grammar's naming fields (`name`/`type`/`function`/
/// `macro`/`field`), recursing through nested types/paths. Generic — reads field names, never a
/// hardcoded node list — so `Vec<Box<_>>` heads at `Vec` and its inner type at `Box`.
fn generic_head(node: Node, src: &[u8]) -> Option<String> {
    for field in ["name", "type", "function", "macro", "field"] {
        if let Some(c) = node.child_by_field_name(field) {
            if c.named_child_count() == 0 {
                let t = c.utf8_text(src).ok()?.trim().to_string();
                return (!t.is_empty() && t.len() <= 24).then_some(t);
            }
            return generic_head(c, src);
        }
    }
    None
}

/// The operator + operand texts of a binary comparison. Grammar-agnostic: finds the
/// operator child, then the nearest NAMED node on each side (works whether the grammar
/// uses left/right fields or bare operand children, e.g. Python `comparison_operator`).
fn comparison(node: Node, src: &[u8]) -> Option<(String, String, String)> {
    let mut cursor = node.walk();
    let children: Vec<Node> = node.children(&mut cursor).collect();
    drop(cursor);
    let op_idx = children
        .iter()
        .position(|ch| matches!(ch.kind(), "==" | "!=" | "<=" | ">=" | "<" | ">"))?;
    let op = children[op_idx].utf8_text(src).ok()?.to_string();
    let left = children[..op_idx].iter().rev().find(|c| c.is_named())?;
    let right = children[op_idx + 1..].iter().find(|c| c.is_named())?;
    Some((
        op,
        left.utf8_text(src).ok()?.to_string(),
        right.utf8_text(src).ok()?.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: &str, bad: &str, good: &str) -> RuleExample {
        RuleExample {
            id: id.into(),
            bad: bad.into(),
            good: good.into(),
        }
    }

    fn ids(lang: &str, code: &str, rules: &[RuleExample]) -> Vec<String> {
        check(lang, code, rules)
            .into_iter()
            .map(|h| h.rule_id)
            .collect()
    }

    #[test]
    fn signature_is_derived_from_the_examples_not_hand_written() {
        // The docs give bad/good; the diff (cmp:==bool) becomes the check itself.
        let rules = vec![rule(
            "bool_comparison",
            "fn f(x: bool) { if x == true {} }",
            "fn f(x: bool) { if x {} }",
        )];
        assert!(ids("rust", "fn g(y: bool) { if y == true {} }", &rules).contains(&"bool_comparison".into()));
        // clean code that doesn't exhibit the signature is not flagged
        assert!(ids("rust", "fn g(y: bool) { if y {} }", &rules).is_empty());
    }

    #[test]
    fn the_signature_inside_a_string_is_not_code() {
        let rules = vec![rule(
            "bool_comparison",
            "fn f(x: bool) { if x == true {} }",
            "fn f(x: bool) { if x {} }",
        )];
        // `== true` only appears inside a string literal — a different node kind.
        assert!(ids("rust", r#"fn g() { let e = "if x == true {}"; }"#, &rules).is_empty());
    }

    #[test]
    fn an_ungroundable_rule_never_fires() {
        // bad == good ⇒ empty signature ⇒ skipped, no blind flag.
        let rules = vec![rule("noop", "fn f() {}", "fn f() {}")];
        assert!(ids("rust", "fn anything() { let a = 1; }", &rules).is_empty());
        // a rule missing an example is likewise skipped.
        let rules = vec![rule("half", "fn f(x: bool) { if x == true {} }", "")];
        assert!(ids("rust", "fn g(y: bool) { if y == true {} }", &rules).is_empty());
    }

    #[test]
    fn works_across_languages_from_the_same_data_path() {
        // Same mechanism, Python grammar, no per-language code.
        let rules = vec![rule("bool_comparison", "if x == True:\n    pass", "if x:\n    pass")];
        assert!(ids("python", "if y == True:\n    pass", &rules).contains(&"bool_comparison".into()));
        assert!(ids("python", "if y:\n    pass", &rules).is_empty());
    }
}
