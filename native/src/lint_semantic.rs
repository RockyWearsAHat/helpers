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
    /// The leading name token, lowercased (`get_user` ⇒ `get`, `isValid` ⇒ `is`). The part that
    /// carries the behavioral contract a reader expects.
    pub name_lead: String,
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
    /// Fallible results left unhandled (`unwrap` / `expect` / `panic`, plus `let _ =` discards).
    pub forced_results: usize,
    /// Whether the function yields a value: a declared return type, or a `return <expr>`/tail.
    pub produces_value: bool,
    /// Whether the function mutates state: an assignment/compound-assignment in the body.
    pub mutates: bool,
}

/// The leading token of a name, lowercased: up to the first `_` or camelCase boundary. This is
/// the word that sets a reader's expectation — `get`, `is`, `set`, `parse`, `render`.
fn lead_token(name: &str) -> String {
    let n = name.trim_start_matches(|c: char| !c.is_alphabetic());
    let mut out = String::new();
    for c in n.chars() {
        if c == '_' {
            break;
        }
        if !out.is_empty() && c.is_uppercase() {
            break;
        }
        out.push(c.to_ascii_lowercase());
    }
    out
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
        // A declared return type (Rust `-> T`, TS `: T`, Go result) means the function yields a
        // value — unless it is the unit type. Languages without annotations rely on the
        // `return <expr>`/tail detection in `summarize`.
        let returns_typed = node
            .child_by_field_name("return_type")
            .and_then(|n| n.utf8_text(src).ok())
            .is_some_and(|t| !matches!(t.trim(), "()" | "( )" | "void" | "Unit"));
        let name_lead = lead_token(&name);
        let mut m = FnMetrics {
            name,
            name_lead,
            line: node.start_position().row + 1,
            distinct_calls: 0,
            branches: 0,
            loops: 0,
            depth: 0,
            forced_results: 0,
            produces_value: returns_typed,
            mutates: false,
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
    // Mutation: a (re)assignment or compound/augmented assignment — not a `let`/declaration.
    if named && kind_has(kind, &["assignment", "augmented"]) && !kind_has(kind, &["let", "declaration"]) {
        m.mutates = true;
    }
    // Produces a value: an explicit `return <expr>` (a return node with a value child). Covers
    // languages without return-type annotations; the typed case is handled at the function node.
    if named && kind_has(kind, &["return"]) && node.named_child_count() > 0 {
        m.produces_value = true;
    }
    // Error handling, dataflow-lite: forcing a fallible result (`unwrap`/`expect`/`panic`) or
    // explicitly discarding one with `let _ = …` — both walk past a failure instead of handling it.
    if kind_has(kind, &["call", "invocation"]) {
        if let Some(name) = call_name(node, src) {
            if matches!(name.as_str(), "unwrap" | "expect" | "unwrap_err" | "panic") {
                m.forced_results += 1;
            }
            calls.insert(name);
        }
    }
    if named && kind_has(kind, &["let", "declaration", "assignment"]) {
        if let Some(pat) = node.child_by_field_name("pattern").or_else(|| node.child_by_field_name("left")) {
            if pat.utf8_text(src).map(|t| t.trim() == "_").unwrap_or(false) {
                m.forced_results += 1;
            }
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

/// What the corpus has learned about a leading name token: how often functions named with it
/// produce a value and how often they mutate. This is the convention, learned — not declared.
#[derive(Clone, Debug, Default)]
struct NameStat {
    total: usize,
    produces: usize,
    mutates: usize,
}

/// Learned norms: outlier thresholds for size/complexity, plus the learned behavioral contract of
/// each leading name token. All learned from the corpus, so the bar fits the language/project.
#[derive(Clone, Debug)]
pub struct Norms {
    /// Responsibility outlier threshold (90th percentile of the corpus).
    pub responsibility_p90: usize,
    /// Complexity outlier threshold (90th percentile of the corpus).
    pub complexity_p90: usize,
    /// Number of functions the norms were learned from.
    pub sampled: usize,
    /// Per leading-name-token behavioral statistics.
    name_stats: std::collections::HashMap<String, NameStat>,
}

/// One principle judgment about a function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Principle {
    /// Does more distinct things than the corpus norm (single-responsibility).
    SingleResponsibility,
    /// More complex (branch/loop/nesting) than the corpus norm.
    Complexity,
    /// Forces or discards fallible results instead of handling them.
    ErrorHandling,
    /// The name promises behavior the body does not match — a getter that returns nothing, or a
    /// query-named function that mutates — judged against the convention learned from the corpus.
    NamingMismatch,
}

/// A name token must appear at least this many times in the corpus before its learned contract
/// is trusted enough to flag a violation of it.
const NAME_MIN_SAMPLES: usize = 8;

impl Norms {
    /// Learn the normal distribution of behavior from a corpus of `(lang, code)` sources.
    pub fn learn(sources: &[(&str, &str)]) -> Norms {
        let mut resp: Vec<usize> = Vec::new();
        let mut cplx: Vec<usize> = Vec::new();
        let mut name_stats: std::collections::HashMap<String, NameStat> = std::collections::HashMap::new();
        for (lang, code) in sources {
            for f in functions(lang, code) {
                resp.push(f.responsibility());
                cplx.push(f.complexity());
                if !f.name_lead.is_empty() {
                    let s = name_stats.entry(f.name_lead.clone()).or_default();
                    s.total += 1;
                    s.produces += f.produces_value as usize;
                    s.mutates += f.mutates as usize;
                }
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
        Norms {
            responsibility_p90: p90(&resp),
            complexity_p90: p90(&cplx),
            sampled: resp.len(),
            name_stats,
        }
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
        // Naming vs behavior, against the learned convention for this name's leading token.
        if let Some(s) = self.name_stats.get(&m.name_lead) {
            if s.total >= NAME_MIN_SAMPLES {
                let produce_rate = s.produces as f64 / s.total as f64;
                let mutate_rate = s.mutates as f64 / s.total as f64;
                // The corpus says functions with this lead almost always return a value, but this
                // one doesn't — the name promises a result it never yields.
                let promises_value = produce_rate >= 0.85 && !m.produces_value;
                // The corpus says functions with this lead almost never mutate (a query), but this
                // one does — the name reads as a query while it writes.
                let query_but_mutates = mutate_rate <= 0.10 && produce_rate >= 0.5 && m.mutates;
                if promises_value || query_but_mutates {
                    out.push(Principle::NamingMismatch);
                }
            }
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
    fn naming_mismatch_learned_from_convention() {
        // Corpus convention: `get_*` functions return a value (learned from many examples).
        let mut code = String::new();
        for i in 0..12 {
            code.push_str(&format!("fn get_thing{i}() -> i32 {{ {i} }}\n"));
        }
        // The offender: named like a getter, returns nothing.
        code.push_str("fn get_nothing() { let _x = 1; }\n");
        let norms = Norms::learn(&[("rust", &code)]);
        let fns = functions("rust", &code);
        let bad = fns.iter().find(|f| f.name == "get_nothing").unwrap();
        assert!(norms.judge(bad).contains(&Principle::NamingMismatch), "getter returning nothing should flag");
        let good = fns.iter().find(|f| f.name == "get_thing0").unwrap();
        assert!(!norms.judge(good).contains(&Principle::NamingMismatch), "a real getter must not flag");
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
