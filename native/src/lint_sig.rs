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

use serde::{Deserialize, Serialize};

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
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Ground {
    /// Grounded in rare AST structure.
    Structure,
    /// Grounded in a distinctive construct named by the description.
    Description,
}

#[derive(Clone, Serialize, Deserialize)]
struct RuleSig {
    id: String,
    ground: Ground,
    /// Required structural features (labels/edges) — all must be present.
    struct_feats: Vec<String>,
    /// Required construct values (node heads) named by the description — all must be present.
    desc_values: Vec<String>,
}

/// A trained signature model: a flat list of per-rule signatures over one language.
///
/// It is `Serialize`/`Deserialize` so a trained model can be **packed once and reused
/// anywhere** ([`SigModel::to_json`] / [`SigModel::from_json`]) — the same model linting on
/// any machine, no retraining, the artifact carrying everything it learned from the docs.
#[derive(Clone, Serialize, Deserialize)]
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

/// A structural feature is INCIDENTAL when it pins an example's local name: a bare
/// `identifier:<name>` node (`x`, `y`, `tmp`) the author chose arbitrarily. Such a feature can
/// never generalize — a signature must match the SHAPE of a violation, not the example's local
/// names — so it is excluded from structural signatures. Meaningful names (methods, types) live
/// in `field_identifier:`/`type_identifier:`/value features and are grounded by the description
/// modality instead. This is what lets a learned pattern catch the SAME mistake on new variables.
fn pins_local_name(feat: &str) -> bool {
    feat.split('>')
        .any(|seg| seg.strip_prefix("identifier:").is_some_and(|head| !head.is_empty()))
}

/// Order candidate features most-specific first, so the greedy fit reaches for a meaningful
/// structure (a named call/type, then a structural kind) before a bare operator on ties.
fn order_features(mut pool: Vec<String>) -> Vec<String> {
    fn rank(f: &str) -> u8 {
        let leaf = f.rsplit('>').next().unwrap_or(f);
        if leaf.starts_with("op:") {
            2 // bare operator — least specific
        } else if leaf.contains(':') {
            0 // carries a value (call:len, field_identifier:len) — most specific
        } else {
            1 // structural kind (index_expression, binary_expression)
        }
    }
    pool.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.cmp(b)));
    pool
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
                .filter(|f| {
                    !good.contains(f)
                        && !pins_local_name(f)
                        && feat_df.get(f).copied().unwrap_or(0) <= STRUCT_DF_MAX
                })
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

    /// **Self-validating fit** — the "read the docs, then test, repeatedly, until it reliably tells
    /// right from wrong" loop, made deterministic. For each documented rule it grows the *minimal*
    /// set of structural features that fires on the rule's bad form but matches **none** of the
    /// negatives — the rule's own good form, every other rule's examples, and the `reference`
    /// (known-idiomatic code read from the docs). A rule whose bad form cannot be separated from
    /// every negative ABSTAINS rather than guess, so the model never learns a pattern it cannot
    /// defend against everything it has seen. This is what lets it generalize from a tiny example:
    /// the example says *what* is wrong, the reference corpus says what is *normal*, and the fit
    /// keeps only the difference. Returns the model and the number of (feature, example) tests run.
    pub fn fit(lang: &str, rules: &[Rule], reference: &[&str]) -> (SigModel, usize) {
        let bad_feats: Vec<HashSet<String>> = rules.iter().map(|r| feature_set(lang, &r.bad)).collect();
        let good_feats: Vec<HashSet<String>> =
            rules.iter().map(|r| if r.good.is_empty() { HashSet::new() } else { feature_set(lang, &r.good) }).collect();
        let ref_feats: Vec<HashSet<String>> = reference.iter().map(|c| feature_set(lang, c)).collect();

        let mut sigs = Vec::new();
        let mut tests = 0usize;
        for (i, r) in rules.iter().enumerate() {
            if r.bad.is_empty() {
                continue;
            }
            // Candidate features: distinctive vs the rule's own fix, never pinning a local name.
            let pool = order_features(
                bad_feats[i]
                    .iter()
                    .filter(|f| !good_feats[i].contains(*f) && !pins_local_name(f))
                    .cloned()
                    .collect(),
            );
            if pool.is_empty() {
                continue;
            }
            // Negatives this rule must NOT match: its own good, every sibling example, the reference.
            let mut negatives: Vec<&HashSet<String>> = Vec::new();
            if !r.good.is_empty() {
                negatives.push(&good_feats[i]);
            }
            for (j, rj) in rules.iter().enumerate() {
                if j == i {
                    continue;
                }
                negatives.push(&bad_feats[j]);
                if !rj.good.is_empty() {
                    negatives.push(&good_feats[j]);
                }
            }
            negatives.extend(ref_feats.iter());

            // Greedy minimal cover: repeatedly add the feature that breaks (is absent from) the most
            // still-fully-matching negatives, until none remain — or until no feature makes progress.
            let mut chosen: Vec<String> = Vec::new();
            let mut unbroken: Vec<&HashSet<String>> = negatives;
            while !unbroken.is_empty() {
                let mut best: Option<(&String, usize)> = None;
                for f in pool.iter().filter(|f| !chosen.contains(*f)) {
                    let breaks = unbroken.iter().filter(|n| !n.contains(f.as_str())).count();
                    tests += unbroken.len();
                    if best.map(|(_, b)| breaks > b).unwrap_or(true) {
                        best = Some((f, breaks));
                    }
                }
                match best {
                    Some((f, breaks)) if breaks > 0 => {
                        let f = f.clone();
                        unbroken.retain(|n| n.contains(f.as_str()));
                        chosen.push(f);
                    }
                    _ => break, // no remaining feature separates a negative → cannot ground
                }
            }
            if unbroken.is_empty() && !chosen.is_empty() {
                sigs.push(RuleSig { id: r.id.clone(), ground: Ground::Structure, struct_feats: chosen, desc_values: Vec::new() });
            }
        }
        (SigModel { lang: lang.to_string(), sigs }, tests)
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

    /// The language this model lints.
    pub fn language(&self) -> &str {
        &self.lang
    }

    /// Pack the trained model to JSON — the self-contained artifact that can be stored (GitHub)
    /// and reused on any machine without retraining.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Load a packed model produced by [`SigModel::to_json`], or `None` if the JSON is invalid.
    pub fn from_json(json: &str) -> Option<SigModel> {
        serde_json::from_str(json).ok()
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
