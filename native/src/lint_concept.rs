//! `lint_concept` — seed semantic understanding from the docs, not a hand-built analyzer.
//!
//! Type/ownership/dataflow concepts (Copy, `const`, mutable, borrow, iterator, allocation,
//! panic, lifetime…) are exactly what the rule descriptions *explain*: "Checks for `.clone()`
//! on a `Copy` type", "this could be `const`", "holding a `RefCell` across an `await`". So the
//! model can LEARN what those concepts mean in code by reading which constructs each concept
//! co-occurs with across all the documentation — the dictionary→English trick, applied to
//! semantic concepts instead of words. No type engine: the meaning is grown from the docs.
//!
//! This is the initial idea, deliberately simple: a co-occurrence lexicon between the English
//! concepts in a rule's description and the code constructs in its example. It already lets the
//! model say what concepts a piece of code *involves* (its learned semantic profile) and what a
//! concept *looks like* in code — the seed a deeper, dataflow-aware layer can grow from.

use std::collections::HashMap;

use crate::lint_ast::generic_features;

/// The semantic lexicon learned from documentation: which code constructs each English concept
/// co-occurs with, and vice versa. Both directions are kept so the model can read code→concepts
/// (what does this code mean?) and concept→code (what does this concept look like?).
#[derive(Default)]
pub struct Lexicon {
    /// concept term → (construct → co-occurrence count).
    concept_to_constructs: HashMap<String, HashMap<String, u32>>,
    /// construct → (concept → co-occurrence count).
    construct_to_concepts: HashMap<String, HashMap<String, u32>>,
    /// How many rule descriptions each concept appeared in (for frequency filtering).
    concept_doc_freq: HashMap<String, usize>,
    /// Total rules read.
    rules: usize,
}

/// Words that carry no concept — articles, glue, and lint-doc boilerplate that appears in nearly
/// every description. Everything else of length ≥ 4 is a candidate concept; over-common terms are
/// then dropped by document-frequency, so this list only needs the obvious glue.
const STOP: &[&str] = &[
    "the", "and", "for", "that", "this", "with", "into", "from", "are", "was", "but", "not",
    "checks", "check", "lint", "lints", "code", "usage", "used", "use", "uses", "using", "when",
    "will", "would", "should", "could", "can", "its", "their", "they", "them", "which", "where",
    "what", "have", "has", "had", "been", "being", "such", "than", "then", "also", "more", "most",
    "some", "any", "all", "see", "example", "examples", "instead", "rather", "because", "while",
];

/// Content-word concepts in a description: lowercased alphabetic words of length ≥ 4 that are not
/// glue. Backtick code spans contribute their identifiers too (e.g. `RefCell`, `await`).
fn concepts(desc: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in desc.split(|c: char| !c.is_alphanumeric() && c != '_') {
        let w = raw.trim().to_lowercase();
        if w.len() >= 4 && w.chars().next().is_some_and(|c| c.is_alphabetic()) && !STOP.contains(&w.as_str()) {
            out.push(w);
        }
    }
    out
}

/// The code constructs a snippet exhibits: the *named* head/leaf identities of its AST nodes —
/// method and type names like `clone`, `Vec`, `await` — where semantic meaning lives. Pure
/// punctuation/operator values (`.`, `()`, `=`, `"`) are dropped: they co-occur with everything
/// and carry no concept, so they only add noise. Parsed with the page's own language.
fn constructs(lang: &str, code: &str) -> Vec<String> {
    generic_features(lang, code)
        .into_iter()
        .filter_map(|(f, _)| {
            if f.contains('>') {
                return None;
            }
            let v = f.rsplit_once(':').map(|(_, v)| v)?;
            let namelike = v.len() >= 2 && v.chars().all(|c| c.is_alphanumeric() || c == '_');
            namelike.then(|| v.to_string())
        })
        .collect()
}

impl Lexicon {
    /// Learn the lexicon by reading `(language, description, code_example)` triples — the crawled
    /// documentation. Every concept in a description is associated with every construct in that
    /// page's example, parsed in its own language; the counts accumulate the conventional
    /// co-occurrence across the whole crawl, so one lexicon spans every language read.
    pub fn learn(rules: &[(&str, &str, &str)]) -> Lexicon {
        let mut lex = Lexicon::default();
        for (lang, desc, bad) in rules {
            lex.rules += 1;
            let cs: Vec<String> = concepts(desc);
            let ks: Vec<String> = constructs(lang, bad);
            if cs.is_empty() || ks.is_empty() {
                continue;
            }
            let uniq_concepts: std::collections::HashSet<&String> = cs.iter().collect();
            for c in &uniq_concepts {
                *lex.concept_doc_freq.entry((*c).clone()).or_default() += 1;
            }
            for c in &uniq_concepts {
                for k in &ks {
                    *lex.concept_to_constructs.entry((*c).clone()).or_default().entry(k.clone()).or_default() += 1;
                    *lex.construct_to_concepts.entry(k.clone()).or_default().entry((*c).clone()).or_default() += 1;
                }
            }
        }
        lex
    }

    /// True if a concept is distinctive enough to trust: seen in a few rules but not the majority
    /// (mid-frequency = a domain concept, not glue and not a one-off).
    fn is_domain_concept(&self, c: &str) -> bool {
        let df = self.concept_doc_freq.get(c).copied().unwrap_or(0);
        df >= 2 && df * 4 <= self.rules.max(1) // appears in 2..=25% of rules
    }

    /// What a concept LOOKS LIKE in code: the constructs most associated with it, learned from the
    /// docs. `meaning_of("clone")` ⇒ the clone-ish constructs; `meaning_of("iterator")` ⇒ map/filter…
    pub fn meaning_of(&self, concept: &str, top: usize) -> Vec<(String, u32)> {
        let mut v: Vec<(String, u32)> = self
            .concept_to_constructs
            .get(concept)
            .map(|m| m.iter().map(|(k, n)| (k.clone(), *n)).collect())
            .unwrap_or_default();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(top);
        v
    }

    /// The semantic concepts a piece of `code` involves, learned from the docs: each construct in
    /// the code votes for the concepts it co-occurred with, weighted by how distinctive that
    /// construct is for the concept. The model's read of "what does this code mean?".
    pub fn concepts_of(&self, lang: &str, code: &str, top: usize) -> Vec<(String, f64)> {
        let mut score: HashMap<String, f64> = HashMap::new();
        let ks = constructs(lang, code);
        let uniq: std::collections::HashSet<&String> = ks.iter().collect();
        for k in uniq {
            if let Some(concepts) = self.construct_to_concepts.get(k) {
                let total: u32 = concepts.values().sum();
                for (c, n) in concepts {
                    if self.is_domain_concept(c) {
                        // How strongly this construct points at the concept, times the concept's
                        // inverse document frequency — so generic doc words (type/function/written)
                        // that co-occur with everything are damped and real domain concepts surface.
                        let df = self.concept_doc_freq.get(c).copied().unwrap_or(1);
                        let idf = ((self.rules as f64 + 1.0) / (df as f64 + 1.0)).ln().max(0.0);
                        *score.entry(c.clone()).or_default() += (*n as f64 / total.max(1) as f64) * idf;
                    }
                }
            }
        }
        let mut v: Vec<(String, f64)> = score.into_iter().collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        v.truncate(top);
        v
    }

    /// Number of distinct concepts learned.
    pub fn concept_count(&self) -> usize {
        self.concept_to_constructs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learns_a_concept_from_its_description() {
        // Two rules teach "clone" alongside a .clone() call, amid unrelated rules so the
        // frequency filter (a domain concept appears in a minority of rules) holds.
        let mut rules: Vec<(&str, &str, &str)> = vec![
            ("rust", "Checks for `clone` on a value that is already owned.", "let y = x.clone();"),
            ("rust", "Redundant `clone` of a reference that is copied.", "let z = a.clone();"),
        ];
        let filler: [(&str, &str, &str); 12] = [
            ("rust", "Checks for needless iterator collect.", "let v: Vec<_> = it.collect();"),
            ("rust", "Prefer matches over equality on enums.", "if e == Foo {}"),
            ("rust", "Avoid redundant return statements.", "fn f() -> i32 { return 1; }"),
            ("rust", "Detects manual swap of two variables.", "let t = a; a = b; b = t;"),
            ("rust", "Suggests using is_empty over len zero.", "if v.len() == 0 {}"),
            ("rust", "Avoid casting with as where From fits.", "let x = y as i64;"),
            ("rust", "Checks for needless borrow in calls.", "foo(&bar);"),
            ("rust", "Detects boolean comparison to true.", "if flag == true {}"),
            ("rust", "Prefer push_str over push of string.", "s.push_str(\"x\");"),
            ("rust", "Avoid unwrap on results in library code.", "let r = parse().unwrap();"),
            ("rust", "Detects single-character string splits.", "s.split(\"a\");"),
            ("rust", "Prefer or_default over or_insert default.", "m.entry(k).or_default();"),
        ];
        rules.extend_from_slice(&filler);
        let lex = Lexicon::learn(&rules);
        // The concept "clone" should map to the clone construct.
        let m = lex.meaning_of("clone", 3);
        assert!(m.iter().any(|(k, _)| k == "clone"), "concept 'clone' learned the clone construct: {m:?}");
        // Code that clones should evoke the "clone" concept.
        let cs = lex.concepts_of("rust", "fn f(x: String) -> String { x.clone() }", 5);
        assert!(cs.iter().any(|(c, _)| c == "clone"), "cloning code evokes 'clone': {cs:?}");
    }
}
