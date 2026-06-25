//! `lint_practice` — the second detector: practice / "idea" rules learned from prose, not from a
//! code pair. A principle like *"a function should do one thing"* or *"prefer shallow control flow"*
//! has no fixed subtree to match — it is a **measurable property judged against the project's own
//! norm**. This module gives the linter a small, fixed set of language-agnostic structural *senses*
//! (a function's size, how many distinct things it does, how deeply it nests) and lets a prose
//! principle ACTIVATE a sense by the words it uses. So adding a practice rule is pure documentation:
//! summarize a software-practice principle into the corpus and, if its words name a sense the linter
//! has, it begins flagging the project's outliers on that sense — no code change.
//!
//! Why outliers, not a fixed threshold: "700 lines is too long" is not universal (a generated table
//! may be fine; a 60-line orchestrator may be the worst unit in a tidy codebase). The defensible
//! signal is *relative* — a unit that does far more than how this project usually writes code. The
//! corpus says exactly this ("judged distributionally against the project's norm, not by a fixed
//! number"). We use Tukey's fence (above the third quartile by more than 1.5× the interquartile
//! range), and abstain when the project is too small to have a norm.

use tree_sitter::{Node, Parser};

/// The fixed, general structural senses the linter has. Each is computed per function from the AST
/// of any supported language, so a prose principle that names one applies everywhere. Adding a sense
/// is the only thing that needs code; adding a *rule* that uses one is just documentation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Sense {
    /// How much a unit does: distinct calls, branches, and loops it contains — the "responsibility"
    /// load. A unit far above the project norm is doing several jobs and should be split.
    Responsibility,
    /// How deeply control flow nests — readability and testability fall off with depth.
    Complexity,
    /// A unit's source length in lines.
    Length,
}

impl Sense {
    /// The words in a principle's prose that select this sense. A principle activates a sense when
    /// its text contains any of these — so the doc's own language decides what is measured.
    fn keywords(self) -> &'static [&'static str] {
        match self {
            Sense::Responsibility => &["responsibilit", "one thing", "does too much", "split", "single-respons", "cohes"],
            Sense::Complexity => &["complex", "nest", "control flow", "deep", "decision point", "cyclomatic"],
            Sense::Length => &["length", "long", "small", "short", "too big", "lines of code", "line count"],
        }
    }

    /// All senses, for scanning a principle's prose.
    fn all() -> [Sense; 3] {
        [Sense::Responsibility, Sense::Complexity, Sense::Length]
    }
}

/// One practice principle read from the corpus prose: a stable id, the human advice to echo, and the
/// senses its wording activated. A principle that names no known sense activates nothing and is
/// silently inert (it stays human context, exactly as the corpus says narrative sections do).
#[derive(Clone, Debug)]
pub struct Principle {
    /// Stable slug derived from the heading (e.g. `single_responsibility`).
    pub id: String,
    /// Severity bucket carried to the finding.
    pub severity: String,
    /// The principle's advice — the first sentence of its prose, echoed on every finding.
    pub advice: String,
    /// The senses this principle measures (empty ⇒ inert).
    senses: Vec<Sense>,
}

impl Principle {
    /// Read a principle from its heading and prose body. The heading becomes the id and severity (a
    /// trailing `[high]`/`[low]` tag, mirroring the code-rule corpus); the body's wording selects the
    /// senses. Returns `None` when the prose activates no sense — nothing measurable to check.
    pub fn from_section(heading: &str, body: &str) -> Option<Principle> {
        let (raw_title, severity) = split_severity(heading);
        // Drop a trailing qualifier in parentheses (`Single responsibility (behavioral)`) from the
        // id/title — it is an authoring note, not part of the rule's name.
        let title = match raw_title.split_once('(') {
            Some((head, _)) => head.trim().to_string(),
            None => raw_title,
        };
        let text = format!("{title} {body}").to_lowercase();
        let senses: Vec<Sense> = Sense::all().into_iter().filter(|s| s.keywords().iter().any(|k| text.contains(k))).collect();
        if senses.is_empty() {
            return None;
        }
        Some(Principle { id: slugify(&title), severity, advice: first_sentence(body), senses })
    }
}

/// One flagged unit: which principle it offends, the severity, the 1-based line the unit starts on,
/// and a short why ("does 9 things vs a project norm of 2").
pub struct Finding {
    /// Offended principle's id.
    pub rule: String,
    /// Severity bucket.
    pub severity: String,
    /// 1-based start line of the unit.
    pub line: usize,
    /// The principle's advice — the practice being violated.
    pub advice: String,
    /// Human explanation of how far past the norm this unit is.
    pub detail: String,
}

/// A function unit located in a file, with its measured senses.
struct Unit {
    line: usize,
    responsibility: f64,
    complexity: f64,
    length: f64,
}

/// Measure every function in `code` for `lang`. Empty when the language has no grammar.
fn units(lang: &str, code: &str) -> Vec<Unit> {
    let Some(language) = crate::lint_ast::language_of(lang) else { return Vec::new() };
    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(code, None) else { return Vec::new() };
    let mut out = Vec::new();
    collect_units(tree.root_node(), &mut out);
    out
}

/// A node that is a function/method definition in any of the supported grammars.
fn is_function(node: Node) -> bool {
    let k = node.kind();
    (k.contains("function") || k.contains("method")) && (k.contains("item") || k.contains("definition") || k.contains("declaration"))
}

/// A node that introduces a branch or loop — a "decision point" for the responsibility/complexity
/// senses. Matched by kind substring so it holds across grammars without naming a language.
fn is_decision(node: Node) -> bool {
    let k = node.kind();
    ["if", "match", "switch", "for", "while", "loop", "case", "catch", "conditional"].iter().any(|d| k.contains(d))
        && !k.contains("identifier")
}

/// A node that is a function/method CALL — the work a unit delegates, counted for responsibility.
fn is_call(node: Node) -> bool {
    let k = node.kind();
    k.contains("call") || k.contains("macro_invocation")
}

/// Walk the tree, emitting a [`Unit`] for every function with its measured senses.
fn collect_units(node: Node, out: &mut Vec<Unit>) {
    if is_function(node) {
        let mut calls = 0u32;
        let mut decisions = 0u32;
        let mut max_depth = 0u32;
        measure(node, 0, &mut calls, &mut decisions, &mut max_depth);
        let length = (node.end_position().row - node.start_position().row + 1) as f64;
        out.push(Unit {
            line: node.start_position().row + 1,
            responsibility: f64::from(calls + decisions),
            complexity: f64::from(max_depth),
            length,
        });
    }
    let mut cur = node.walk();
    for c in node.children(&mut cur) {
        collect_units(c, out);
    }
}

/// Accumulate a unit's call/decision counts and its maximum decision-nesting depth.
fn measure(node: Node, depth: u32, calls: &mut u32, decisions: &mut u32, max_depth: &mut u32) {
    let mut here = depth;
    if is_call(node) {
        *calls += 1;
    }
    if is_decision(node) {
        *decisions += 1;
        here += 1;
        *max_depth = (*max_depth).max(here);
    }
    let mut cur = node.walk();
    for c in node.children(&mut cur) {
        measure(c, here, calls, decisions, max_depth);
    }
}

/// The compiled practice rules for a project: the active principles. Built from the corpus prose,
/// applied to the whole project at once (the norm is project-wide).
pub struct PracticeRules {
    principles: Vec<Principle>,
}

impl PracticeRules {
    /// Build from the corpus's narrative principles (those with no code pair). Principles that name
    /// no measurable sense are dropped.
    pub fn new(principles: Vec<Principle>) -> PracticeRules {
        PracticeRules { principles }
    }

    /// Whether any principle is active (worth running the project pass).
    pub fn is_empty(&self) -> bool {
        self.principles.is_empty()
    }

    /// Flag the project's outlier units. `files` is every analyzed file of one `lang` as
    /// `(path, source)`; findings are returned as `(path, Finding)`. The norm is computed across the
    /// whole project, so a unit is judged against how THIS project writes code. Abstains per sense
    /// when there are too few units to define a norm.
    pub fn flag_project<'a>(&self, lang: &str, files: &'a [(String, String)]) -> Vec<(&'a str, Finding)> {
        // Measure every unit once, remembering which file it came from.
        let mut all: Vec<(usize, Unit)> = Vec::new(); // (file index, unit)
        for (i, (_, code)) in files.iter().enumerate() {
            for u in units(lang, code) {
                all.push((i, u));
            }
        }
        let mut out = Vec::new();
        for p in &self.principles {
            for &sense in &p.senses {
                let values: Vec<f64> = all.iter().map(|(_, u)| value(u, sense)).collect();
                let Some(threshold) = tukey_upper_fence(&values) else { continue };
                let norm = median(&values);
                for (i, u) in &all {
                    let v = value(u, sense);
                    if v > threshold {
                        out.push((
                            files[*i].0.as_str(),
                            Finding {
                                rule: p.id.clone(),
                                severity: p.severity.clone(),
                                line: u.line,
                                advice: p.advice.clone(),
                                detail: explain(sense, v, norm),
                            },
                        ));
                    }
                }
            }
        }
        out
    }
}

/// A unit's value for one sense.
fn value(u: &Unit, sense: Sense) -> f64 {
    match sense {
        Sense::Responsibility => u.responsibility,
        Sense::Complexity => u.complexity,
        Sense::Length => u.length,
    }
}

/// A human "how far past the norm" note for a finding.
fn explain(sense: Sense, value: f64, norm: f64) -> String {
    let (v, n) = (value as u64, norm as u64);
    match sense {
        Sense::Responsibility => format!("does ~{v} distinct things (calls + branches); project norm is ~{n} — split it"),
        Sense::Complexity => format!("nests control flow {v} deep; project norm is ~{n} — flatten it"),
        Sense::Length => format!("{v} lines; project norm is ~{n} — break it up"),
    }
}

/// Tukey's upper outlier fence (Q3 + 1.5·IQR) for `values`, or `None` when there are too few units
/// (< 8) to have a meaningful norm — abstaining rather than judging a project by a handful of units.
fn tukey_upper_fence(values: &[f64]) -> Option<f64> {
    if values.len() < 8 {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q1 = quantile(&v, 0.25);
    let q3 = quantile(&v, 0.75);
    let iqr = q3 - q1;
    if iqr > 0.0 {
        return Some(q3 + 1.5 * iqr);
    }
    // A tight distribution (most units identical) collapses the IQR — but that is exactly when a
    // unit doing several times the norm is the clearest outlier. Fall back to a relative multiple of
    // the typical value (still judged against the project, not a fixed line count). When the typical
    // value is zero (no work at all), there is nothing to be an outlier of.
    (q3 > 0.0).then_some(q3 * 2.0)
}

/// The `q` quantile of an already-sorted slice (linear interpolation).
fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let pos = q * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        sorted[lo] + (pos - lo as f64) * (sorted[hi] - sorted[lo])
    }
}

/// The median of `values` (the reported norm).
fn median(values: &[f64]) -> f64 {
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    quantile(&v, 0.5)
}

/// Split a corpus heading into its title and severity tag (`Foo [high]` → `("Foo", "high")`),
/// defaulting to medium. Mirrors the code-rule corpus's `[severity]` convention.
fn split_severity(heading: &str) -> (String, String) {
    let h = heading.trim();
    if let (Some(open), Some(close)) = (h.rfind('['), h.rfind(']')) {
        if close > open {
            let sev = h[open + 1..close].trim().to_lowercase();
            if matches!(sev.as_str(), "high" | "medium" | "low") {
                return (h[..open].trim().to_string(), sev);
            }
        }
    }
    (h.to_string(), "medium".to_string())
}

/// A kebab/underscore slug from a heading title.
fn slugify(title: &str) -> String {
    let mut s = String::new();
    let mut prev_us = false;
    for c in title.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c);
            prev_us = false;
        } else if !prev_us && !s.is_empty() {
            s.push('_');
            prev_us = true;
        }
    }
    s.trim_matches('_').to_string()
}

/// The first sentence of a prose body, trimmed — the advice echoed on a finding.
fn first_sentence(body: &str) -> String {
    let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
    match flat.find(". ") {
        Some(i) => flat[..=i].trim().to_string(),
        None => flat,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prose_activates_the_sense_it_names() {
        let p = Principle::from_section(
            "Single responsibility",
            "A function should do one thing. When a unit juggles many distinct calls it is doing too much — split it.",
        )
        .expect("activates a sense");
        assert_eq!(p.id, "single_responsibility");
        assert!(p.senses.contains(&Sense::Responsibility));
        // A purely narrative principle with no measurable wording stays inert.
        assert!(Principle::from_section("Naming", "Use clear, intention-revealing names.").is_none());
    }

    #[test]
    fn flags_the_project_outlier_not_the_norm() {
        // Eight tidy functions (one call each) and one that does far more: the outlier is flagged,
        // the tidy ones are not — judged against the project's own norm.
        let tidy = (0..8)
            .map(|i| format!("fn f{i}() {{ g(); }}"))
            .collect::<Vec<_>>()
            .join("\n");
        let big = "fn big() { a(); if x { b(); } for y in z { c(); d(); } while w { e(); } match m { _ => f() } h(); i(); }";
        let code = format!("{tidy}\n{big}");
        let files = vec![("lib.rs".to_string(), code)];
        let rules = PracticeRules::new(vec![Principle::from_section(
            "Single responsibility",
            "A function should do one thing; splitting many distinct calls and branches.",
        )
        .unwrap()]);
        let hits = rules.flag_project("rust", &files);
        assert!(hits.iter().any(|(_, f)| f.rule == "single_responsibility" && f.line == 9), "the big function is flagged: {:?}", hits.iter().map(|(_, f)| f.line).collect::<Vec<_>>());
        assert!(hits.iter().all(|(_, f)| f.line == 9), "the tidy functions are within the norm");
    }
}
