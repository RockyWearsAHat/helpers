//! `lint_semantic` — judge what code *does*, not just its shape, in any language.
//!
//! Syntactic rules (clippy/ruff/…) are a prebuilt checklist. The real power of an AI linter is
//! comprehending behavior so it can grade CS2420/CS3500 principles — single responsibility,
//! complexity, error handling — regardless of language. Those are *behavioral* properties, and
//! they are derivable generically from the parse tree with no per-language or per-rule code:
//!
//!   * **responsibility** — how many distinct things a function does (distinct calls + branches
//!     + loops). A unit that does one thing scores low; a god-function scores high.
//!   * **complexity** — branching + looping + nesting depth (a cyclomatic-style proxy).
//!   * **error handling** — fallible results forced/ignored (`unwrap`/`expect`/bare `?`-less).
//!
//! The thresholds are not hand-set: [`Norms::learn`] reads a corpus and learns the normal
//! distribution, then [`Norms::judge`] flags the *outliers* — the functions that violate a
//! principle relative to how this language/project actually writes code. "Studied the language,
//! then judged it," applied to meaning rather than surface.

use std::collections::HashSet;

use tree_sitter::{Node, Parser};

/// tree-sitter language for `lang`, or `None` if we have no grammar (mirrors [`crate::lint_ast`]).
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

/// The behavioral measurements of one function — the raw material for a principle judgment.
#[derive(Clone, Debug)]
pub struct FnMetrics {
    /// The function's name (best-effort from the grammar's `name` field).
    pub name: String,
    /// 1-based line where the function starts.
    pub line: usize,
    /// Distinct callee names invoked in the body — distinct *concerns* touched.
    pub distinct_calls: usize,
    /// Conditional nodes (if / match / switch / case): decision points.
    pub branches: usize,
    /// Loop nodes (for / while / loop).
    pub loops: usize,
    /// Maximum block-nesting depth inside the body.
    pub depth: usize,
    /// Fallible results left unhandled (`unwrap` / `expect` and friends).
    pub forced_results: usize,
}

impl FnMetrics {
    /// Responsibility load: the number of distinct concerns the function juggles. High ⇒ it is
    /// probably doing more than one thing (single-responsibility risk).
    pub fn responsibility(&self) -> usize {
        self.distinct_calls + self.branches + self.loops
    }
    /// Cyclomatic-style complexity proxy.
    pub fn complexity(&self) -> usize {
        self.branches + self.loops + self.depth
    }
}

/// True if a node kind names a function-like definition across the supported grammars. Generic:
/// matched by substring, never an exhaustive per-language list.
fn is_function(kind: &str) -> bool {
    kind.contains("function") || kind == "method_definition" || kind == "method_declaration"
}

/// Extract per-function metrics from `code`. One pass: find function-like nodes, then summarize
/// each subtree. Language-agnostic — the categories are matched by node-kind substrings the
/// upstream grammars share.
pub fn functions(lang: &str, code: &str) -> Vec<FnMetrics> {
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
    collect_functions(tree.root_node(), code.as_bytes(), &mut out);
    out
}

fn collect_functions(node: Node, src: &[u8], out: &mut Vec<FnMetrics>) {
    if is_function(node.kind()) {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(src).ok())
            .unwrap_or("<anon>")
            .to_string();
        let mut m = FnMetrics {
            name,
            line: node.start_position().row + 1,
            distinct_calls: 0,
            branches: 0,
            loops: 0,
            depth: 0,
            forced_results: 0,
        };
        let mut calls = HashSet::new();
        summarize(node, src, 0, &mut m, &mut calls);
        m.distinct_calls = calls.len();
        out.push(m);
        // Do not recurse into nested functions as part of this one; collect them separately.
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            collect_nested(child, src, out);
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions(child, src, out);
    }
}

/// Find functions nested inside another function (closures/inner fns) as their own units.
fn collect_nested(node: Node, src: &[u8], out: &mut Vec<FnMetrics>) {
    if is_function(node.kind()) {
        collect_functions(node, src, out);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_nested(child, src, out);
    }
}

/// Does a node kind contain `word` as a `_`-delimited token? Token-level so `if_expression`
/// matches `if` but `identifier` (ident-IF-ier) does NOT — the substring trap that inflated
/// every count. Generic across grammars, which all name kinds in `snake_case`.
fn kind_has(kind: &str, words: &[&str]) -> bool {
    kind.split('_').any(|t| words.contains(&t))
}

/// Tally a function body's behavioral signals, tracking nesting depth.
fn summarize(node: Node, src: &[u8], depth: usize, m: &mut FnMetrics, calls: &mut HashSet<String>) {
    let kind = node.kind();
    let mut d = depth;
    // Only NAMED nodes carry structure; the anonymous keyword tokens (`for`, `if`, …) would
    // otherwise double-count alongside their `*_expression` node.
    let named = node.is_named();
    if named && kind_has(kind, &["block", "body"]) {
        d = depth + 1;
        m.depth = m.depth.max(d);
    }
    // A decision point: an if/match/switch/ternary expression or statement — counted once, not
    // per arm. Token-level match so it never catches `identifier`/`specifier`/`pattern`.
    if named
        && kind_has(kind, &["if", "elif", "match", "switch", "ternary", "conditional"])
        && !kind_has(kind, &["arm", "pattern", "clause", "block", "body", "case"])
    {
        m.branches += 1;
    }
    if named && kind_has(kind, &["for", "while", "loop", "foreach"]) {
        m.loops += 1;
    }
    if kind_has(kind, &["call", "invocation"]) {
        if let Some(name) = call_name(node, src) {
            if matches!(name.as_str(), "unwrap" | "expect" | "unwrap_err" | "panic") {
                m.forced_results += 1;
            }
            calls.insert(name);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        // Don't descend into nested function definitions — those are separate units.
        if is_function(child.kind()) && child.id() != node.id() {
            continue;
        }
        summarize(child, src, d, m, calls);
    }
}

/// Best-effort callee name of a call node: the `function` field, or the trailing method/field
/// identifier. Grammar-driven, not a hardcoded list.
fn call_name(node: Node, src: &[u8]) -> Option<String> {
    let f = node.child_by_field_name("function").or_else(|| node.child_by_field_name("callee"))?;
    if f.named_child_count() == 0 {
        return f.utf8_text(src).ok().map(str::to_string);
    }
    // Method/field call: take the last identifier-ish segment.
    f.child_by_field_name("field")
        .or_else(|| f.child_by_field_name("name"))
        .or_else(|| f.child_by_field_name("property"))
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// Learned norms for a metric: the threshold (a high percentile) above which a function is an
/// outlier worth flagging. Learned from the corpus, so the bar fits the language/project.
#[derive(Clone, Debug)]
pub struct Norms {
    /// Responsibility outlier threshold (90th percentile of the corpus).
    pub responsibility_p90: usize,
    /// Complexity outlier threshold (90th percentile of the corpus).
    pub complexity_p90: usize,
    /// Number of functions the norms were learned from.
    pub sampled: usize,
}

/// One principle judgment about a function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Principle {
    /// Does more distinct things than `0` of corpus functions (single-responsibility).
    SingleResponsibility,
    /// More complex (branch/loop/nesting) than the corpus norm.
    Complexity,
    /// Forces fallible results instead of handling them.
    ErrorHandling,
}

impl Norms {
    /// Learn the normal distribution of behavior from a corpus of `(lang, code)` sources.
    pub fn learn(sources: &[(&str, &str)]) -> Norms {
        let mut resp: Vec<usize> = Vec::new();
        let mut cplx: Vec<usize> = Vec::new();
        for (lang, code) in sources {
            for f in functions(lang, code) {
                resp.push(f.responsibility());
                cplx.push(f.complexity());
            }
        }
        resp.sort_unstable();
        cplx.sort_unstable();
        let p90 = |v: &[usize]| -> usize {
            if v.is_empty() {
                return usize::MAX; // nothing learned ⇒ never flag
            }
            v[(v.len() * 9 / 10).min(v.len() - 1)].max(1)
        };
        Norms { responsibility_p90: p90(&resp), complexity_p90: p90(&cplx), sampled: resp.len() }
    }

    /// Judge a function against the learned norms: the principles it violates (possibly none).
    pub fn judge(&self, m: &FnMetrics) -> Vec<Principle> {
        let mut out = Vec::new();
        if m.responsibility() > self.responsibility_p90 {
            out.push(Principle::SingleResponsibility);
        }
        if m.complexity() > self.complexity_p90 {
            out.push(Principle::Complexity);
        }
        if m.forced_results > 0 {
            out.push(Principle::ErrorHandling);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_behavior_generically() {
        let code = r#"
            fn small(x: i32) -> i32 { x + 1 }
            fn big(items: &[i32]) -> i32 {
                let mut total = 0;
                for it in items {
                    if *it > 0 { total += foo(*it); } else { total += bar(*it); }
                }
                total.checked_add(1).unwrap()
            }
        "#;
        let fns = functions("rust", code);
        let big = fns.iter().find(|f| f.name == "big").expect("found big");
        assert!(big.loops >= 1 && big.branches >= 1, "loops/branches counted: {big:?}");
        assert!(big.forced_results >= 1, "unwrap flagged as forced result");
        let small = fns.iter().find(|f| f.name == "small").unwrap();
        assert!(small.responsibility() < big.responsibility(), "big does more than small");
    }

    #[test]
    fn norms_flag_the_outlier_not_the_simple_fn() {
        // A realistic distribution: many simple functions, one god-function. p90 then lands among
        // the simple ones, so only the genuine outlier is flagged.
        let mut code = String::new();
        for i in 0..15 {
            code.push_str(&format!("fn simple{i}() -> i32 {{ {i} }}\n"));
        }
        code.push_str(
            r#"
            fn god(xs: &[i32]) -> i32 {
                let mut t = 0;
                for x in xs { if *x > 0 { t += f1(*x); } else if *x < 0 { t += f2(*x); } else { t += f3(*x); } }
                while t > 100 { t = g1(t); if t == 0 { break; } }
                t
            }
        "#,
        );
        let norms = Norms::learn(&[("rust", &code)]);
        let fns = functions("rust", &code);
        let god = fns.iter().find(|f| f.name == "god").unwrap();
        assert!(norms.judge(god).contains(&Principle::Complexity), "god fn should flag complexity");
        let simple = fns.iter().find(|f| f.name == "simple0").unwrap();
        assert!(norms.judge(simple).is_empty(), "a one-liner should not be flagged");
    }
}
