//! `lint_query` — interrogate the AI's UNDERSTANDING and the rules it shapes, returning REAL,
//! structured data (definitions, distances, step traces, counts) rather than free-form prose. This
//! is the debugger for "understanding shapes rules": an unqueryable machine tells us nothing about
//! its states or whether it is learning, so every answer here is checkable data.
//!
//! Query kinds (`kind` + `arg`):
//!   * `define <word>` — what the AI has learned this word MEANS: whether it is known, its
//!     definition words, and the tracing concepts it sits nearest to in the meaning space (with
//!     distances). Validates the dictionary understanding directly.
//!   * `explain <principle prose>` — understanding APPLIED to that sentence, step by step: did the
//!     prohibition gate fire, which salient concepts were extracted, which primitive each aligned
//!     to (distance + margin), and the rule understanding shaped — or, on abstain, exactly why.
//!   * `rules <language>` — the rules currently enforced for a language, counted and listed, each
//!     with the understanding behind it (the principle prose + the plan it shaped).

use serde_json::{json, Value};

use crate::proto::{text, ToolResult};

/// The MCP schema for `lint_query`.
pub fn schema() -> Value {
    json!({
        "name": "lint_query",
        "description": "Interrogate the AI linter's understanding and state, returning structured data. kind=define <word> (is it known, its definition words, nearest tracing concepts + distances); kind=explain <principle prose> (the prohibition gate, salient concepts, each concept's aligned primitive + distance/margin, the rule understanding shaped or why it abstained); kind=rules <language> (count + list of enforced rules, each with its principle and shaped plan).",
        "inputSchema": {
            "type": "object",
            "properties": {
                "kind": { "type": "string", "enum": ["define", "explain", "rules"], "description": "The interrogation to run." },
                "arg": { "type": "string", "description": "define: a word; explain: a principle sentence; rules: a language id (e.g. rust)." }
            },
            "required": ["kind", "arg"]
        }
    })
}

/// Run a query. Structured JSON out; an unknown kind is an error, a missing brain is reported as
/// data (never a silent empty).
pub fn run(args: &Value) -> ToolResult {
    let kind = args["kind"].as_str().unwrap_or("");
    let arg = args["arg"].as_str().unwrap_or("").trim();
    let out = match kind {
        "define" => define(arg),
        "explain" => explain(arg),
        "rules" => rules(arg),
        other => {
            return Err(format!(
                "lint_query: unknown kind `{other}`. Valid: define | explain | rules"
            ))
        }
    };
    Ok(vec![text(serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string()))])
}

/// `define <word>` — the dictionary understanding of one word, as real data.
fn define(word: &str) -> Value {
    let brain = crate::lint_char::brain();
    let known = brain.is_some_and(|b| b.has_meaning(word));
    let definition_words: Option<Vec<String>> =
        brain.and_then(|b| b.meanings().definition_words(word)).map(<[String]>::to_vec);
    let nearest: Vec<Value> = crate::lint_trace::concept_alignment(word)
        .unwrap_or_default()
        .into_iter()
        .take(6)
        .map(|(name, dist)| json!({ "concept": name, "distance": dist }))
        .collect();
    json!({
        "kind": "define",
        "word": word,
        "brain_loaded": brain.is_some(),
        "known": known,
        "definition_words": definition_words,
        "nearest_concepts": nearest,
        "note": "distance is Hamming over 8192-bit meaning vectors (0 = exact/synonymous, ~4096 = unrelated).",
    })
}

/// `explain <principle prose>` — understanding applied, step by step.
fn explain(prose: &str) -> Value {
    let Some(ex) = crate::lint_trace::explain(prose) else {
        return json!({ "kind": "explain", "prose": prose, "brain_loaded": false,
                       "note": "no character/English brain loaded — run lint_config action=train" });
    };
    let concepts: Vec<Value> = ex
        .concepts
        .iter()
        .map(|c| {
            json!({
                "word": c.word,
                "aligned_to": c.aligned,
                "nearest": c.nearest,
                "distance": c.distance,
                "runner_up": c.runner_up,
                "ratio": c.ratio,
                "centrality": c.centrality,
            })
        })
        .collect();
    json!({
        "kind": "explain",
        "sentence": ex.sentence,
        "prohibition_gate_fired": ex.prohibition,
        "operators": ex.operators,
        "inner_negations": ex.inner_negations,
        "concepts": concepts,
        "shaped_rule": ex.plan.as_ref().map(|p| p.describe()),
        "enforces": ex.plan.is_some(),
        "abstain_reason": ex.abstain,
    })
}

/// `rules <language>` — the rules currently enforced for a language, split by ORIGIN so the
/// listing reflects what genuinely enforces. Two groups, exactly as the live lint merges them
/// (overlay ⊕ module):
///   * `understanding_rules` — the machine-global CS-principles corpus, read FRESH and shaped by
///     the understanding→trace bridge (or the probe fallback). These are the rules the AI derives
///     from prose, the north star of the system.
///   * `module_rules` — the crawled-doc rules baked into the trained language module (AST/token
///     detectors learned from the language's own documentation).
/// Kept separate rather than a flat list: a stale crawled token-detector and an understanding-shaped
/// trace rule are different kinds of thing, and conflating them was what made this query misleading.
fn rules(lang: &str) -> Value {
    let understanding: Vec<Value> = detail_values(&crate::lint_train::corpus_ruleset(lang));
    let module: Option<Vec<Value>> =
        crate::lint_train::cached_ruleset(lang).map(|rs| detail_values(&rs));
    let module_count = module.as_ref().map(Vec::len).unwrap_or(0);
    json!({
        "kind": "rules",
        "language": lang,
        "count": understanding.len() + module_count,
        "understanding_count": understanding.len(),
        "understanding_rules": understanding,
        "module_count": module_count,
        "module_rules": module,
        "module_note": module.is_none()
            .then(|| format!("no trained module for `{lang}` — run lint_config action=train")),
    })
}

/// One rule set's `(id, severity, detector, principle)` rows as JSON — shared by both origin groups.
fn detail_values(rs: &crate::lint_match::RuleSet) -> Vec<Value> {
    rs.rule_details()
        .into_iter()
        .map(|(id, severity, description, detector)| {
            json!({ "id": id, "severity": severity, "detector": detector, "principle": description })
        })
        .collect()
}
