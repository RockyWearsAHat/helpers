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
                "kind": { "type": "string", "enum": ["define", "explain", "rules", "learn"], "description": "The interrogation to run. learn: PROPOSE-then-VERIFY — reason a principle's check by testing candidate senses against bad/good evidence, keep and remember what actually works." },
                "arg": { "type": "string", "description": "define: a word; explain: a principle sentence; rules: a language id (e.g. rust); learn: the principle sentence." },
                "scope": { "type": "string", "enum": ["canon", "language"], "description": "explain only: read the prose as the language-agnostic canon (no uses_construct fallback) or as general language-doc prose (default). Canon principles enforce structurally or abstain." },
                "language": { "type": "string", "description": "learn only: the language of the bad/good evidence (e.g. rust)." },
                "bad": { "type": "string", "description": "learn only: code that BREAKS the principle — the check must fire on it." },
                "good": { "type": "string", "description": "learn only: code that OBEYS the principle — the check must stay clean on it." }
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
    // `scope` selects how `explain` reads the prose: the language-agnostic CANON (no construct
    // fallback — a principle enforces structurally or abstains) vs general LANGUAGE-doc prose (a
    // construct-naming prohibition may shape `uses_construct`). Default is language-doc so
    // `explain "never use the var keyword"` still shapes `uses_construct(var)`.
    let canon = matches!(args["scope"].as_str(), Some("canon"));
    let out = match kind {
        "define" => define(arg),
        "explain" => explain(arg, canon),
        "rules" => rules(arg),
        "learn" => learn(arg, args),
        other => {
            return Err(format!(
                "lint_query: unknown kind `{other}`. Valid: define | explain | rules | learn"
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
    // The LEARNED USAGE sense — the words this term co-occurs with across explanatory prose, most
    // distinctive-and-frequent first. Present only for words the explanation corpus observed; this
    // is where a jargon term's PROGRAMMING sense shows up, distinct from its dictionary definition.
    let usage_words: Option<Vec<Value>> = brain.and_then(|b| b.meanings().usage_words(word)).map(|u| {
        u.iter().map(|(w, c)| json!({ "word": w, "count": c })).collect()
    });
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
        "usage_words": usage_words,
        "nearest_concepts": nearest,
        "note": "distance is Hamming over 8192-bit meaning vectors (0 = exact/synonymous, ~4096 = unrelated).",
    })
}

/// `explain <principle prose>` — understanding applied, step by step. `canon` reads the prose as a
/// language-agnostic canon principle (construct fallback suppressed) rather than general
/// language-doc prose.
fn explain(prose: &str, canon: bool) -> Value {
    let Some(ex) = crate::lint_trace::explain(prose, canon) else {
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

/// `learn <principle>` — PROPOSE-then-VERIFY: the AI reasons the principle's structural check by
/// TESTING candidate senses against the `bad`/`good` evidence, keeps the one that catches the bad
/// shape and spares the good, and REMEMBERS it so later runs recall it without re-deriving. This is
/// the Ornith method with no judge model — reality referees. Reports the check learned (and whether
/// the descriptor-word path would have reached it), or an honest abstain when nothing verified.
fn learn(principle: &str, args: &Value) -> Value {
    let lang = args["language"].as_str().unwrap_or("rust");
    let bad = args["bad"].as_str().unwrap_or("");
    let good = args["good"].as_str().unwrap_or("");
    if bad.is_empty() || good.is_empty() {
        return json!({ "kind": "learn", "principle": principle,
            "note": "learn needs both `bad` (code that breaks the rule) and `good` (code that obeys it) — the check is chosen by which one it actually catches." });
    }
    let word_path = crate::lint_trace::understand(principle).map(|p| p.describe());
    let learned = crate::lint_trace::learn_verified(principle, lang, bad, good);
    let verified = learned.as_ref().map(|p| p.describe());
    let bad_lines = learned.as_ref().map(|p| crate::lint_trace::run_plan(p, lang, bad));
    let good_lines = learned.as_ref().map(|p| crate::lint_trace::run_plan(p, lang, good));
    json!({
        "kind": "learn",
        "principle": principle,
        "language": lang,
        "word_path_plan": word_path,
        "verified_plan": verified,
        "learned": learned.is_some(),
        "fires_on_bad": bad_lines,
        "clean_on_good": good_lines.as_ref().map(|l: &Vec<usize>| l.is_empty()),
        "note": learned.is_some()
            .then(|| "reasoned by verification and remembered — future runs recall this check.".to_string())
            .unwrap_or_else(|| "no candidate sense both fired on bad and stayed clean on good — honest abstain.".to_string()),
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
    // Item 3d — the COMPLETION surface: the knowledge snapshot the module was proven at fixpoint against,
    // and whether a changed corpus/brain has reopened it for re-proving through the 3c re-check.
    let completion = crate::lint_train::module_completion(lang).map(|c| {
        json!({
            "complete": c.complete,
            "state": if c.complete { "COMPLETE (proven set at fixpoint against current knowledge)" }
                     else { "reopened (corpus or brain changed — next train re-proves via the 3c re-check)" },
            "train_version": c.train_version,
            "sources_fp": c.sources_fp,
            "brain_fp": c.brain_fp.to_string(),
            "trained_at": c.trained_at,
        })
    });
    json!({
        "kind": "rules",
        "language": lang,
        "count": understanding.len() + module_count,
        "understanding_count": understanding.len(),
        "understanding_rules": understanding,
        "module_count": module_count,
        "module_rules": module,
        "completion": completion,
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
