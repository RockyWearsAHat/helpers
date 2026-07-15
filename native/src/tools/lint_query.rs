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
//!   * `web <construct>` — the language WEB read off the subgraph: the construct node's governing prose,
//!     its meaning-link key-words into the frozen English base, its sources, whether it is PROVEN
//!     (enforced) or merely READ (retained), the concepts it traverses to, and the nearest constructs in
//!     OTHER languages' webs through the shared English base (the cross-language traversal). `language`
//!     scopes the lookup; omitted, the first web that carries the construct is used.
//!   * `verify <statement>` — the SELF-REFEREE (PASS 30): judge a statement against the proven web
//!     itself. Per construct the statement names: PROVEN-COHERENT / CONTRADICTED / UNSUPPORTED /
//!     CONNECTED — or `unconnected` when the web carries none of it (the honest gap).

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
                "kind": { "type": "string", "enum": ["define", "explain", "rules", "learn", "web", "verify"], "description": "The interrogation to run. learn: PROPOSE-then-VERIFY — reason a principle's check by testing candidate senses against bad/good evidence, keep and remember what actually works. web: read a construct off the language subgraph — its governing prose, meaning links, doc-roles, state (proven/read), and cross-language connections. With role= set, web instead LISTS every construct carrying that doc-role, across languages. verify: the SELF-REFEREE — judge a statement against the proven web itself: per named construct, PROVEN-COHERENT / CONTRADICTED / UNSUPPORTED / CONNECTED, or unconnected (the honest gap)." },
                "arg": { "type": "string", "description": "define: a word; explain: a principle sentence; rules: a language id (e.g. rust); learn: the principle sentence; web: a construct token (e.g. document.write, cgi) — or empty when querying by role=; verify: the statement to judge against the proven web (e.g. \"the cgi module is deprecated\")." },
                "scope": { "type": "string", "enum": ["canon", "language"], "description": "explain only: read the prose as the language-agnostic canon (no uses_construct fallback) or as general language-doc prose (default). Canon principles enforce structurally or abstain." },
                "role": { "type": "string", "description": "web only: a DOC-ROLE traversal target — removal | prohibition | deprecated — listing every construct the faculties prove carries it (e.g. what connects to REMOVAL → cgi, telnetlib). Scoped to language= when set, else across every language web." },
                "language": { "type": "string", "description": "learn only: the language of the bad/good evidence (e.g. rust); web only: restrict the construct/role lookup to this language." },
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
        "verify" => verify(arg, args["language"].as_str()),
        "explain" => explain(arg, canon),
        "rules" => rules(arg),
        "learn" => learn(arg, args),
        "web" => web(arg, args["language"].as_str(), args["role"].as_str()),
        other => {
            return Err(format!(
                "lint_query: unknown kind `{other}`. Valid: define | explain | rules | learn | web | verify"
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
    let cached = crate::lint_train::cached_ruleset(lang);
    let module: Option<Vec<Value>> = cached.as_ref().map(|rs| detail_values(rs));
    let module_count = module.as_ref().map(Vec::len).unwrap_or(0);
    // PASS 31 — the conservation ledger: what the compile WITHHELD and why. A rule may be withheld
    // for a named reason; it may never vanish (the train-time invariant enforces this for proven law).
    let withheld: Vec<Value> = cached
        .as_ref()
        .map(|rs| {
            rs.withheld()
                .iter()
                .map(|(id, gate)| json!({ "id": id, "reason": gate }))
                .collect()
        })
        .unwrap_or_default();
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
        "withheld": withheld,
        "completion": completion,
        "module_note": module.is_none()
            .then(|| format!("no trained module for `{lang}` — run lint_config action=train")),
    })
}

/// `web <construct>` — the language subgraph read off directly (PASS 24). Finds the construct's node in
/// the named language's web (or the first web that carries it), and reports the whole node: its governing
/// prose, its meaning-link key-words into the frozen English base, its sources, whether it is PROVEN
/// (enforced) or merely READ (retained-unproven), the English concepts its meaning traverses to, and the
/// nearest constructs in OTHER languages' webs through the shared base. Interpretation is a QUERY over the
/// graph — no stored labels.
/// `kind=verify` (PASS 30) — the self-referee as a live query: a statement in, the WEB's verdict out.
/// The statement's revocation claim ([`crate::lint_corroborate::revocation_claim`] — negation polarity ×
/// the status anchors) is judged against every web node the statement NAMES (bounded full token, across
/// the loaded language webs or the named one). Per construct: PROVEN-COHERENT (the web proves the
/// statement — its own verified role agrees; sources cited) / CONTRADICTED (the statement denies a role
/// the web proved — one side is wrong) / UNSUPPORTED (the statement asserts a role the web cannot prove)
/// / CONNECTED (a neutral mention of a known node — the web reports what it knows) / and an overall
/// `unconnected: true` when no web node is named at all (the honest gap).
fn verify(statement: &str, language: Option<&str>) -> Value {
    let Some(en) = crate::lint_english::brain() else {
        return json!({ "kind": "verify", "statement": statement, "error": "no English brain on disk — run lint_config action=train first" });
    };
    let mut anchors = crate::lint_attest::prohibition_class_tokens();
    anchors.extend(crate::lint_attest::removal_class_tokens());
    let lower = statement.to_lowercase();
    let negated = crate::lint_corroborate::is_negated(en, statement);
    let claim = crate::lint_corroborate::revocation_claim(&lower, negated, &anchors);
    let langs: Vec<String> = match language {
        Some(l) => vec![l.to_string()],
        None => crate::lint_web::languages_with_web(),
    };
    let is_tok = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.';
    let mentions = |needle: &str| -> bool {
        let mut from = 0usize;
        while let Some(rel) = lower[from..].find(needle) {
            let s = from + rel;
            let e = s + needle.len();
            let before = lower[..s].chars().next_back().map(|c| !is_tok(c)).unwrap_or(true);
            let after = lower[e..].chars().next().map(|c| !is_tok(c)).unwrap_or(true);
            if before && after {
                return true;
            }
            from = e;
        }
        false
    };
    let mut findings: Vec<Value> = Vec::new();
    for lang in &langs {
        for node in crate::lint_web::load(lang) {
            if !mentions(&node.construct.to_lowercase()) {
                continue;
            }
            let revoked = node.attested_deprecated || !node.roles.is_empty();
            use crate::lint_corroborate::RevocationClaim::*;
            let verdict = match (claim, revoked) {
                (Asserts, true) => "PROVEN-COHERENT (the web's own verified role agrees with the statement)",
                (Denies, true) => "CONTRADICTED (the statement denies a role this web proved — one side is wrong)",
                (Asserts, false) => "UNSUPPORTED (the web cannot prove the asserted role — honest abstention)",
                (Denies, false) => "COHERENT (the web carries no revoked role either)",
                (Neutral, _) => "CONNECTED (neutral mention — the web reports what it knows)",
            };
            findings.push(json!({
                "construct": node.construct,
                "language": lang,
                "verdict": verdict,
                "web_state": if node.proven { "PROVEN" } else if node.graded.is_some() { "GRADED" } else { "READ" },
                "doc_roles": node.roles,
                "attested_deprecated": node.attested_deprecated,
                "sources": node.sources,
            }));
        }
    }
    json!({
        "kind": "verify",
        "statement": statement,
        "claim": match claim {
            crate::lint_corroborate::RevocationClaim::Asserts => "asserts-revoked",
            crate::lint_corroborate::RevocationClaim::Denies => "denies-revoked",
            crate::lint_corroborate::RevocationClaim::Neutral => "neutral",
        },
        "unconnected": findings.is_empty(),
        "constructs": findings,
        "webs_available": crate::lint_web::languages_with_web(),
        "note": "the SELF-REFEREE: the verdict is the machine's own proven web judging the statement — coherence corroborates, contradiction means one side is wrong, unconnected is the honest gap.",
    })
}

fn web(construct: &str, language: Option<&str>, role: Option<&str>) -> Value {
    let brain = crate::lint_char::brain();
    // DOC-ROLE traversal (PASS 25 rung 2): list every construct the faculties prove carries `role`, across
    // every language web (or the named one) — "what connects to REMOVAL" reaches cgi/telnetlib regardless
    // of what its prose's index-words say. The roles are stored proven facts, filtered here, never re-judged.
    if let Some(role) = role.map(str::trim).filter(|r| !r.is_empty()) {
        let carriers: Vec<Value> = match language {
            Some(l) => crate::lint_web::nodes_with_role(&crate::lint_web::load(l), role)
                .into_iter()
                .map(|c| json!({ "language": l, "construct": c }))
                .collect(),
            None => crate::lint_web::roles_across(role)
                .into_iter()
                .map(|(lang, c)| json!({ "language": lang, "construct": c }))
                .collect(),
        };
        return json!({
            "kind": "web",
            "role": role,
            "found": !carriers.is_empty(),
            "carriers": carriers,
            "webs_available": crate::lint_web::languages_with_web(),
            "note": "doc-roles are the proven faculties' own facts (author-metadata attestation family + construction kind), surfaced as first-class traversal targets — never a word list.",
        });
    }
    // Locate the node: the named language's web, else the first web on the machine that carries it.
    let langs: Vec<String> = match language {
        Some(l) => vec![l.to_string()],
        None => crate::lint_web::languages_with_web(),
    };
    let mut found: Option<(String, crate::lint_web::ConstructNode)> = None;
    for lang in &langs {
        if let Some(node) = crate::lint_web::load(lang).into_iter().find(|n| n.construct == construct) {
            found = Some((lang.clone(), node));
            break;
        }
    }
    let Some((lang, node)) = found else {
        return json!({
            "kind": "web", "construct": construct,
            "found": false, "webs_available": crate::lint_web::languages_with_web(),
            "note": "no language web carries this construct — run lint_config action=train, or check the construct token exactly (byte-preserved).",
        });
    };
    // Concept traversal: the concepts the node's meaning links sit nearest to in the frozen meaning space.
    let concepts: Vec<Value> = node
        .meaning_links
        .iter()
        .filter_map(|w| {
            crate::lint_trace::concept_alignment(w)
                .and_then(|a| a.into_iter().next())
                .map(|(name, dist)| json!({ "link": w, "nearest_concept": name, "distance": dist }))
        })
        .collect();
    // Cross-language traversal through the shared English base.
    let cross: Vec<Value> = brain
        .map(|b| crate::lint_web::cross_language(b.meanings(), &lang, &node, 8))
        .unwrap_or_default()
        .into_iter()
        .map(|c| json!({ "language": c.lang, "construct": c.construct, "distance": c.distance, "shared_links": c.shared_links }))
        .collect();
    json!({
        "kind": "web",
        "construct": construct,
        "language": lang,
        "found": true,
        "state": if node.proven {
            "PROVEN (enforced)"
        } else if node.graded.is_some() {
            "GRADED (evidence-graded LOW tier — fires on the attested deprecation)"
        } else {
            "READ (retained, unproven — never fired)"
        },
        "governing_prose": node.governing,
        "meaning_links": node.meaning_links,
        "sources": node.sources,
        "attested_deprecated": node.attested_deprecated,
        "doc_roles": node.roles,
        "rule": node.rule.as_ref().map(|r| json!({ "id": r.id, "severity": r.severity, "description": r.description })),
        "graded": node.graded.as_ref().map(|g| json!({ "fire": g.fire, "severity": g.severity, "description": g.description })),
        "self_referee": node.referee.as_ref().map(|r| json!({
            "coherent_sources": r.coherent,
            "contradictions": r.contradictions.iter().map(|c| json!({ "source": c.source, "sentence": c.sentence })).collect::<Vec<_>>(),
        })),
        "traverses_to_concepts": concepts,
        "cross_language_connections": cross,
        "note": "meaning links are key-words into the FROZEN English web; their meaning is rebound on query, never copied. Interpretation is a traversal, not a stored label.",
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

#[cfg(test)]
mod tests {
    use super::*;

    /// BLACK-BOX (PASS 30): the tool boundary only — JSON arguments into [`run`], JSON text out, no
    /// internals observed. The system-as-a-whole contract: a statement the proven web agrees with is
    /// PROVEN-COHERENT with sources; its denial is CONTRADICTED; an unknown construct is `unconnected`.
    /// Judged against the REAL trained python web on this machine; skips honestly when absent.
    fn run_verify(statement: &str) -> Option<Value> {
        let out = run(&json!({ "kind": "verify", "arg": statement, "language": "python" })).ok()?;
        serde_json::from_str(&out.first()?.text).ok()
    }

    #[test]
    fn verify_judges_statements_against_the_proven_web_end_to_end() {
        if crate::lint_english::brain().is_none() || crate::lint_web::load("python").is_empty() {
            eprintln!("skip: no english brain or no trained python web on disk");
            return;
        }
        // A true statement the web has proven: coherent, with the web's own citations.
        let v = run_verify("the cgi module is deprecated").expect("tool answers");
        assert_eq!(v["claim"], "asserts-revoked");
        assert_eq!(v["unconnected"], false);
        let cgi = v["constructs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["construct"] == "cgi")
            .expect("the web names cgi");
        assert!(cgi["verdict"].as_str().unwrap().starts_with("PROVEN-COHERENT"), "{v}");
        assert!(!cgi["sources"].as_array().unwrap().is_empty(), "verdict carries citations");

        // The statement's denial: the web proved the role, so one side is wrong — CONTRADICTED.
        let v = run_verify("the cgi module is not deprecated").expect("tool answers");
        assert_eq!(v["claim"], "denies-revoked");
        let cgi = v["constructs"].as_array().unwrap().iter().find(|c| c["construct"] == "cgi").unwrap();
        assert!(cgi["verdict"].as_str().unwrap().starts_with("CONTRADICTED"), "{v}");

        // A construct no web carries: the honest gap, stated as such.
        let v = run_verify("the flurbotron9000 gadget is deprecated").expect("tool answers");
        assert_eq!(v["unconnected"], true, "{v}");
        assert!(v["constructs"].as_array().unwrap().is_empty());
    }
}
