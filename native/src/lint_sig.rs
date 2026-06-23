//! `lint_sig` — the signature detector: the approach that won the measurements, made into a
//! usable linter. It learns from the documentation, with no per-rule or per-node-kind code.
//!
//! Each rule gets its OWN signature (so there is no bundled-expert interference — the
//! "recursive/per-rule capacity" result), grounded in either of two modalities:
//!
//!   * **Structure** — the rare AST structures (from [`crate::lint_ast::generic_features`]) that
//!     appear in the rule's bad example but not its good one and are uncommon in the language
//!     corpus. Two or more such structures ⇒ a precise, zero-false-positive signature.
//!   * **Description** — for rules whose bad and good parse identically (the distinction is a
//!     name/type/semantic, not syntax), the English description names the construct. We mine its
//!     backtick spans for identifiers that are distinctive across rules AND rare in the corpus,
//!     and require that construct present in the code — learning the construct from its
//!     dictionary entry.
//!
//! A rule fires only when its whole signature is present, so unrelated code is never flagged;
//! a rule we can ground in neither modality abstains rather than guess.

use std::collections::{HashMap, HashSet};

use crate::lint_ast::generic_features;

/// One documented rule: id, the two code examples, and the English description.
pub struct Rule {
    /// Stable rule id (e.g. `vec_box`).
    pub id: String,
    /// Code the rule says is wrong.
    pub bad: String,
    /// The corrected form (may be empty).
    pub good: String,
    /// The rule's English description.
    pub description: String,
}

/// A located violation: the 1-based source line and the rule id whose signature matched.
pub struct Hit {
    /// 1-based source line of the matched structure.
    pub line: usize,
    /// The rule id that flagged it.
    pub rule: String,
}

/// How a rule was grounded — surfaced so a report can explain itself.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ground {
    /// Grounded in rare AST structure.
    Structure,
    /// Grounded in a distinctive construct named by the description.
    Description,
}

struct RuleSig {
    id: String,
    ground: Ground,
    /// Required structural features (labels/edges) — all must be present.
    struct_feats: Vec<String>,
    /// Required construct values (node heads) named by the description — all must be present.
    desc_values: Vec<String>,
}

/// A trained signature model: a flat list of per-rule signatures over one language.
pub struct SigModel {
    lang: String,
    sigs: Vec<RuleSig>,
}

/// A structural feature is a "value" feature (carries a node head) when it has no edge `>` and a
/// `:` — its value is the text after the last `:` (`call_expression:unwrap` ⇒ `unwrap`).
fn feature_value(f: &str) -> Option<&str> {
    if f.contains('>') {
        return None;
    }
    f.rsplit_once(':').map(|(_, v)| v)
}

/// The set of structural features present in `code`.
fn feature_set(lang: &str, code: &str) -> HashSet<String> {
    generic_features(lang, code).into_iter().map(|(f, _)| f).collect()
}

/// The set of construct values (node heads) present in `code`.
fn value_set(lang: &str, code: &str) -> HashSet<String> {
    generic_features(lang, code)
        .into_iter()
        .filter_map(|(f, _)| feature_value(&f).map(str::to_string))
        .collect()
}

/// Identifiers inside backtick spans of a description: `transmute`, `Vec`, `#[test]`→`test`.
fn description_tokens(desc: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut rest = desc;
    while let Some(i) = rest.find('`') {
        let after = &rest[i + 1..];
        let Some(j) = after.find('`') else { break };
        for tok in after[..j].split(|c: char| !c.is_alphanumeric() && c != '_') {
            if tok.len() >= 2 && tok.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_') {
                out.insert(tok.to_string());
            }
        }
        rest = &after[j + 1..];
    }
    out
}

/// Rare structural feature: appears in at most this many corpus files.
const STRUCT_DF_MAX: usize = 1;
/// A structural signature needs at least this many rare features to be trusted (zero-FP gate).
const STRUCT_MIN: usize = 2;
/// A description token is distinctive when at most this many rules mention it.
const DESC_TOK_MAX: usize = 2;
/// A description-named construct must be at most this common in the corpus to be a safe trigger.
const DESC_VAL_DF_MAX: usize = 2;

impl SigModel {
    /// Learn signatures from the documented `rules` and a `corpus` of real code in `lang`. The
    /// corpus is read only to learn how common each structure/construct is — the "study the
    /// language" step that lets rarity, not a hand-written rule, decide what is distinctive.
    pub fn train(lang: &str, rules: &[Rule], corpus: &[&str]) -> SigModel {
        // Document frequency of structural features, and of construct values, across the corpus.
        let mut feat_df: HashMap<String, usize> = HashMap::new();
        let mut val_df: HashMap<String, usize> = HashMap::new();
        for src in corpus {
            let feats = generic_features(lang, src);
            let present: HashSet<&str> = feats.iter().map(|(f, _)| f.as_str()).collect();
            for f in &present {
                *feat_df.entry((*f).to_string()).or_default() += 1;
            }
            let vals: HashSet<&str> = feats.iter().filter_map(|(f, _)| feature_value(f)).collect();
            for v in vals {
                *val_df.entry(v.to_string()).or_default() += 1;
            }
        }
        // Distinctiveness of a description token across rules.
        let mut tok_df: HashMap<String, usize> = HashMap::new();
        let rule_tokens: Vec<HashSet<String>> = rules.iter().map(|r| description_tokens(&r.description)).collect();
        for toks in &rule_tokens {
            for t in toks {
                *tok_df.entry(t.clone()).or_default() += 1;
            }
        }

        let mut sigs = Vec::new();
        for (r, toks) in rules.iter().zip(&rule_tokens) {
            if r.bad.is_empty() {
                continue;
            }
            // Structural signature: rare structures in bad but not good.
            let good = if r.good.is_empty() { HashSet::new() } else { feature_set(lang, &r.good) };
            let struct_feats: Vec<String> = feature_set(lang, &r.bad)
                .into_iter()
                .filter(|f| !good.contains(f) && feat_df.get(f).copied().unwrap_or(0) <= STRUCT_DF_MAX)
                .collect();
            if struct_feats.len() >= STRUCT_MIN {
                sigs.push(RuleSig { id: r.id.clone(), ground: Ground::Structure, struct_feats, desc_values: Vec::new() });
                continue;
            }
            // Description signature: distinctive tokens that name a construct rare in the corpus,
            // and that actually appears in the rule's own bad example (so it is checkable in code).
            let bad_vals = value_set(lang, &r.bad);
            let desc_values: Vec<String> = toks
                .iter()
                .filter(|t| {
                    tok_df.get(*t).copied().unwrap_or(0) <= DESC_TOK_MAX
                        && val_df.get(*t).copied().unwrap_or(0) <= DESC_VAL_DF_MAX
                        && bad_vals.contains(*t)
                })
                .cloned()
                .collect();
            if !desc_values.is_empty() {
                sigs.push(RuleSig { id: r.id.clone(), ground: Ground::Description, struct_feats: Vec::new(), desc_values });
            }
        }
        SigModel { lang: lang.to_string(), sigs }
    }

    /// Number of rules the model could ground (and will therefore ever flag).
    pub fn rule_count(&self) -> usize {
        self.sigs.len()
    }

    /// How many rules were grounded in each modality — `(structure, description)`.
    pub fn grounding(&self) -> (usize, usize) {
        let s = self.sigs.iter().filter(|x| x.ground == Ground::Structure).count();
        (s, self.sigs.len() - s)
    }

    /// Flag `code`: every rule whose whole signature is present, located at the first line that
    /// carries a signature feature. One hit per rule per source (a rule either applies or not).
    pub fn judge_located(&self, code: &str) -> Vec<Hit> {
        let feats = generic_features(&self.lang, code);
        if feats.is_empty() {
            return Vec::new();
        }
        let present: HashSet<&str> = feats.iter().map(|(f, _)| f.as_str()).collect();
        let values: HashSet<&str> = feats.iter().filter_map(|(f, _)| feature_value(f)).collect();

        let mut hits = Vec::new();
        for sig in &self.sigs {
            let structural_ok = !sig.struct_feats.is_empty()
                && sig.struct_feats.iter().all(|f| present.contains(f.as_str()));
            let descriptive_ok = !sig.desc_values.is_empty()
                && sig.desc_values.iter().all(|v| values.contains(v.as_str()));
            if !(structural_ok || descriptive_ok) {
                continue;
            }
            // Locate at the first line carrying any required feature/value.
            let line = feats
                .iter()
                .find(|(f, _)| {
                    sig.struct_feats.iter().any(|s| s == f)
                        || feature_value(f).is_some_and(|v| sig.desc_values.iter().any(|d| d == v))
                })
                .map(|(_, l)| *l)
                .unwrap_or(1);
            hits.push(Hit { line, rule: sig.id.clone() });
        }
        hits
    }

    /// Just the distinct rule ids flagged in `code`.
    pub fn judge(&self, code: &str) -> Vec<String> {
        let mut seen = HashSet::new();
        self.judge_located(code).into_iter().map(|h| h.rule).filter(|r| seen.insert(r.clone())).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: &str, bad: &str, good: &str, desc: &str) -> Rule {
        Rule { id: id.into(), bad: bad.into(), good: good.into(), description: desc.into() }
    }

    #[test]
    fn structural_signature_flags_violation_not_the_fix() {
        let rules = vec![rule(
            "bool_comparison",
            "fn f(x: bool) { if x == true {} }",
            "fn f(x: bool) { if x {} }",
            "Checks for comparing a boolean to `true`.",
        )];
        let corpus = ["fn a() { let y = 1; }", "fn b(z: bool) { if z {} }"];
        let m = SigModel::train("rust", &rules, &corpus);
        assert!(m.judge("fn g(y: bool) { if y == true {} }").contains(&"bool_comparison".to_string()));
        assert!(m.judge("fn g(y: bool) { if y {} }").is_empty(), "the fixed form must not flag");
    }

    #[test]
    fn an_ungroundable_rule_abstains() {
        // bad == good structurally and the description names no checkable construct.
        let rules = vec![rule("noop", "fn f() {}", "fn f() {}", "A purely stylistic preference.")];
        let m = SigModel::train("rust", &rules, &["fn z() {}"]);
        assert_eq!(m.rule_count(), 0);
        assert!(m.judge("fn anything() { let a = 1; }").is_empty());
    }
}
