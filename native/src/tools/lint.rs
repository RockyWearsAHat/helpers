//! `lint` — the AI code reviewer.
//!
//! Training: [`crate::lint_train::ensure_models`] reads two documentation sources (`extraDocs/` and
//! `.helpers/lint-rules/`) plus the official-doc catalogs and returns, per language, a
//! [`crate::lint_train::LangModel`] — the [`crate::lint_match::RuleSet`] firing engine, the
//! [`crate::lint_ai::ConceptModel`] confirmation gate. One call, checksum-cached.
//!
//! Analysis, per file, per language:
//!   1. **Rule firing** — `RuleSet::flag` matches each documented rule's lossless AST pattern
//!      (or, for grammarless languages, its discriminating token regex) against the file: a
//!      finding is the rule's structure occurring, on the construct's line.
//!   2. **Confirmation gate** — precise AST matches report directly; imprecise text-fallback
//!      matches (grammarless languages, description-derived regexes) pass through
//!      `ConceptModel::confirms`, which bundles the matched construct's tokens and keeps the
//!      finding only when the fired rule is the concept it is closest to — so a regex that hit a
//!      token belonging to a different rule is dropped, with no hand-kept word list.
//!   3. **Self-validation** — doc rules that fire like scrape noise (>1% of all scanned lines, or
//!      concentrated in one file) are quarantined and reported, never shown as findings.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::git::workspace_root;
use crate::index::walk::walk_repo;
use crate::lint_train;
use crate::proto::{text, ToolResult};
use crate::util::file_lang;

/// Per-project linter preferences loaded from `.helpers/lint.json`.
///
/// Agents and users write this file (via `lint_config`) to tailor which rules fire,
/// what languages are reviewed, and how severe each finding is reported.
#[derive(Default, serde::Deserialize)]
pub struct LintConfig {
    /// Rule ids to suppress entirely — they will never appear in lint output.
    #[serde(default)]
    pub ignore_rules: Vec<String>,
    /// Override severity for specific rules: `{"rule-id": "high"|"medium"|"low"}`.
    #[serde(default)]
    pub severity_overrides: HashMap<String, String>,
    /// When set, only these languages are reviewed (in addition to any `--lang` CLI flag).
    #[serde(default)]
    pub languages: Option<Vec<String>>,
}

/// Load `.helpers/lint.json` from the project root, returning defaults on any read/parse error.
pub fn load_config(project_root: &Path) -> LintConfig {
    let path = project_root.join(".helpers/lint.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// The project root to review, from the `root` arg or the resolved workspace.
fn root_arg(args: &Value) -> PathBuf {
    match args.get("root").and_then(Value::as_str) {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => workspace_root(),
    }
}

/// Optional language filter from the `modules` arg. Absent / empty / `all` ⇒ every language.
///
/// Extension-like aliases ("ts", "py") are resolved via `file_lang` to their canonical name.
/// Canonical names ("typescript", "python") and unknown names pass through unchanged — an
/// unknown language produces no files in the output rather than being silently discarded,
/// which surfaces the typo instead of hiding it.
fn parse_lang_filter(args: &Value) -> Option<BTreeSet<String>> {
    let arr = args.get("modules").and_then(Value::as_array)?;
    let mut set = BTreeSet::new();
    for tok in arr.iter().filter_map(Value::as_str) {
        let s = tok.trim().to_ascii_lowercase();
        match s.as_str() {
            "all" | "" => return None,
            other => { set.insert(file_lang(other).unwrap_or(other).to_string()); }
        }
    }
    if set.is_empty() { None } else { Some(set) }
}

/// One reported violation in a file.
struct Hit {
    /// 1-based source line.
    line: usize,
    /// The rule id the model attributed.
    rule: String,
    /// Severity bucket (`high`/`medium`/`low`).
    severity: String,
    /// English advice — the rule's description from its source.
    advice: String,
}

/// A file's place in the review.
struct FileReport {
    /// Repo-relative path.
    path: String,
    /// Findings in this file.
    hits: Vec<Hit>,
}

/// Extract the alphanumeric/underscore tokens of one 1-based source line — the "construct" the Hv
/// gate confirms a text-fallback finding against. Out-of-range lines yield no tokens (gate abstains).
fn line_tokens(src: &str, line: usize) -> Vec<String> {
    let Some(text) = src.lines().nth(line.saturating_sub(1)) else { return Vec::new() };
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|t| t.len() >= 2)
        .map(str::to_string)
        .collect()
}

/// Review the whole project: detect its languages, build one `LangModel` per language (rule set +
/// concept gate + principles), match each file with the rule set, confirm text-fallback findings
/// through the concept gate, add structural outliers, and report the result in English.
pub fn run(args: &Value) -> ToolResult {
    let root = root_arg(args);
    if !root.exists() {
        return Err(format!("lint: path not found: {}", root.display()));
    }
    let max = args.get("max").and_then(Value::as_u64).unwrap_or(80).clamp(1, 500) as usize;
    let filter = parse_lang_filter(args);
    let data = data_root();

    // Load per-project config, then merge in feedback-driven auto-suppressions: rules a developer
    // has flagged as false positives enough times (see `crate::lint_feedback`) are folded into the
    // same `ignore_rules` list, so they are suppressed through the normal config path below.
    let mut config = load_config(&root);
    let auto_suppressed = crate::lint_feedback::merge_auto_suppressed(&mut config, &root);
    let ignore_set: HashSet<&str> = config.ignore_rules.iter().map(String::as_str).collect();

    // 1) Walk the project and partition by language: those with a tree-sitter grammar are analyzed
    //    with the AST engine; the rest are still analyzed via the token-regex fallback, so nothing
    //    is dropped for lacking a grammar.
    let files = walk_repo(&root);
    let mut by_language: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for f in &files {
        // A known extension resolves to its canonical grammar name; an UNKNOWN one becomes a
        // language named by the extension itself — no built-in language list. Such files ride
        // the token-regex engine, project rules (`.helpers/lint-rules/<ext>.md`, `any.md`), and
        // on-the-fly docs discovery; unreadable (non-UTF-8) files fall out at the read below.
        let ext = f.ext.to_lowercase();
        if ext.is_empty() || !ext.chars().all(|c| c.is_ascii_alphanumeric()) { continue; }
        let l = file_lang(&ext).unwrap_or(&ext);
        if filter.as_ref().is_some_and(|set| !set.contains(l)) { continue; }
        if config.languages.as_ref().is_some_and(|set| !set.iter().any(|x| x == l)) { continue; }
        if let Ok(src) = std::fs::read_to_string(&f.abs) {
            by_language.entry(l.to_string()).or_default().push((f.rel.clone(), src));
        }
    }

    // 2) Train / load one model per detected language (checksum-cached), and the advice map that
    //    carries each rule's English description and severity for rendering.
    let langs: Vec<String> = by_language.keys().cloned().collect();
    let (report, models) = lint_train::ensure_models(&langs, &data, &root);
    let advice = lint_train::advice(&data, Some(root.as_path()));

    let mut sources: Vec<String> = Vec::new();
    if !models.is_empty() {
        let total: usize = models.values().map(|m| m.rules.rule_count()).sum();
        sources.push(format!("{total} rules across {} language(s)", models.len()));
    }
    // Name the languages whose rules were (re)learned from the live official docs this run, so the
    // report shows the reading-and-testing path actually ran — not just that rules exist.
    if !report.crawled.is_empty() {
        sources.push(format!("learned live from official docs: {}", report.crawled.join(", ")));
    }

    let mut reports: Vec<FileReport> = Vec::new();
    let push_hit = |reports: &mut Vec<FileReport>, path: &str, hit: Hit| {
        if let Some(r) = reports.iter_mut().find(|r| r.path == path) {
            r.hits.push(hit);
        } else {
            reports.push(FileReport { path: path.to_string(), hits: vec![hit] });
        }
    };

    // 3) Rule firing + confirmation gate. Findings are staged (not reported yet) so the
    //    self-validation pass below can measure each rule's fire rate against reality first.
    let total_lines: usize = by_language.values().flatten().map(|(_, src)| src.lines().count()).sum();
    let mut staged: Vec<(String, Hit, bool)> = Vec::new(); // (path, hit, unverified doc rule?)
    for (lang, lang_files) in &by_language {
        let Some(model) = models.get(lang) else { continue };
        // Rules the project itself authored are the user's explicit law for this codebase —
        // trusted fully, never gated, however weak their compiled anchor is.
        let trusted = lint_train::project_rule_ids(&root, lang);
        // Precise AST matches are exact and staged directly. Imprecise matches — text-fallback
        // regexes and container-only AST patterns (several distinct rules compile to the same bare
        // `list`) — must clear the Hv concept gate: the matched construct's tokens must agree with
        // the fired rule's fingerprint more than with any other rule's, so a match that belongs to
        // a different concept is dropped — no word blocklist involved. All of a language's gated
        // findings go through ONE batched query ([`crate::lint_ai::ConceptModel::confirms_batch`]),
        // a single popcount-grid dispatch instead of a scalar scan per finding.
        let mut pending: Vec<(String, crate::lint_match::Finding, bool)> = Vec::new();
        let mut gate_tokens: Vec<(usize, Vec<String>)> = Vec::new(); // (pending idx, construct tokens)
        for (path, src) in lang_files {
            for finding in model.rules.flag(src) {
                let doc_rule = !trusted.contains(&finding.rule);
                if !finding.precise && doc_rule {
                    gate_tokens.push((pending.len(), line_tokens(src, finding.line)));
                }
                pending.push((path.clone(), finding, doc_rule));
            }
        }
        let items: Vec<(&str, Vec<&str>)> = gate_tokens
            .iter()
            .map(|(i, toks)| (pending[*i].1.rule.as_str(), toks.iter().map(String::as_str).collect()))
            .collect();
        let mut rejected: HashSet<usize> = HashSet::new();
        for (kept, (i, _)) in model.concept.confirms_batch(&items).into_iter().zip(&gate_tokens) {
            if !kept {
                rejected.insert(*i);
            }
        }
        for (i, (path, finding, doc_rule)) in pending.into_iter().enumerate() {
            if rejected.contains(&i) {
                continue;
            }
            let (severity, advice_text) = advice
                .get(&finding.rule)
                .map(|info| {
                    let sev = if info.severity.is_empty() { finding.severity.clone() } else { info.severity.clone() };
                    let adv = if info.description.is_empty() { format!("violates `{}`", finding.rule) } else { info.description.clone() };
                    (sev, adv)
                })
                .unwrap_or_else(|| (finding.severity.clone(), format!("violates `{}`", finding.rule)));
            staged.push((path, Hit { line: finding.line, rule: finding.rule, severity, advice: advice_text }, doc_rule));
        }
    }

    // 3b) Self-validation against reality: a doc-learned rule that fires on more than 1% of every
    //     line scanned (and at least 20 lines) is a mislearned pattern — noisy docs scraping, not
    //     hundreds of real mistakes. Quarantine it wholesale and say so, instead of flooding the
    //     report. Project-authored rules are never quarantined: a project-wide convention
    //     violation legitimately fires everywhere.
    let file_lines: HashMap<&str, usize> =
        by_language.values().flatten().map(|(p, s)| (p.as_str(), s.lines().count())).collect();
    let mut fires: HashMap<&str, usize> = HashMap::new();
    let mut file_fires: HashMap<(&str, &str), usize> = HashMap::new();
    for (path, hit, doc_rule) in &staged {
        if *doc_rule {
            *fires.entry(hit.rule.as_str()).or_default() += 1;
            *file_fires.entry((hit.rule.as_str(), path.as_str())).or_default() += 1;
        }
    }
    // Concentrated mislearning: ≥20 fires inside one file covering >10% of its lines is a pattern
    // matching the file's fabric (every `[`, every backtick), not 20+ separate mistakes.
    let concentrated: HashSet<&str> = file_fires.iter()
        .filter(|((_, path), &n)| n >= 20 && n * 10 > file_lines.get(path).copied().unwrap_or(usize::MAX))
        .map(|((rule, _), _)| *rule)
        .collect();
    let quarantined: std::collections::BTreeSet<String> = fires.iter()
        .filter(|(id, &n)| (n >= 20 && n * 100 > total_lines) || concentrated.contains(*id))
        .map(|(id, _)| id.to_string())
        .collect();
    for (path, hit, _) in staged.into_iter().filter(|(_, h, _)| !quarantined.contains(&h.rule)) {
        push_hit(&mut reports, &path, hit);
    }

    // 5) Apply per-project config: suppress ignored rules, apply severity overrides.
    for report in &mut reports {
        report.hits.retain(|h| !ignore_set.contains(h.rule.as_str()));
        for hit in &mut report.hits {
            if let Some(sev) = config.severity_overrides.get(&hit.rule) {
                hit.severity = sev.clone();
            }
        }
    }

    let analyzed: BTreeMap<String, usize> = by_language.iter().map(|(l, fs)| (l.clone(), fs.len())).collect();
    let unanalyzed: BTreeMap<String, usize> = BTreeMap::new();
    reports.sort_by(|a, b| a.path.cmp(&b.path));
    let mut body = render(&root, &reports, &analyzed, &unanalyzed, &sources, max);
    body.push_str(&render_quarantine(&quarantined));
    body.push_str(&render_feedback(&root, &auto_suppressed));
    Ok(vec![text(body)])
}

/// Report doc-learned rules the self-validation pass quarantined: their fire rate against the real
/// project marks the learned pattern as noise (a badly scraped docs page), not a real convention.
/// Empty when nothing was quarantined.
fn render_quarantine(quarantined: &std::collections::BTreeSet<String>) -> String {
    if quarantined.is_empty() {
        return String::new();
    }
    let ids: Vec<&str> = quarantined.iter().map(String::as_str).collect();
    format!(
        "\nQuarantined {} mislearned rule(s) (fired on >1% of all scanned lines, or >10% of one \
         file — noisy docs scrape, not real findings): {}.\nRe-learn the language (delete its cached model; the next lint retrains) \
         to refresh them.\n",
        ids.len(),
        ids.join(", ")
    )
}

/// Render the feedback footer: rules auto-suppressed from false-positive flags, and missed findings
/// still pending formalization. Empty string when there is no feedback, so clean projects stay quiet.
fn render_feedback(root: &Path, auto_suppressed: &BTreeSet<String>) -> String {
    let mut s = String::new();
    if !auto_suppressed.is_empty() {
        let rules: Vec<&str> = auto_suppressed.iter().map(String::as_str).collect();
        s.push_str(&format!(
            "\nAuto-suppressed from your feedback ({}+ false-positive flags each): {}.\n\
             Re-enable with `lint_config action=unignore rule=<id>`.\n",
            crate::lint_feedback::SUPPRESS_THRESHOLD,
            rules.join(", ")
        ));
    }
    let records = crate::lint_feedback::read_all(root);
    let pending = crate::lint_feedback::pending_missed(&records);
    if !pending.is_empty() {
        s.push_str(&format!("\nPending rules ({} missed finding(s) you flagged — formalize with `lint_rule`):\n", pending.len()));
        for r in pending {
            let loc = r.line.map(|l| format!("{}:{}", r.file, l)).unwrap_or_else(|| r.file.clone());
            let desc = r.description.as_deref().unwrap_or("(no description)");
            let draft = if r.bad.is_some() && r.language.is_some() { " [draft seeded]" } else { "" };
            s.push_str(&format!("  - {loc}: {desc}{draft}\n"));
        }
    }
    s
}

// ── English report ────────────────────────────────────────────────────────────

/// Severity ordering for display: high first.
fn severity_rank(sev: &str) -> u8 {
    match sev {
        "high" => 0,
        "low" => 2,
        _ => 1,
    }
}

/// Collapse a file's hits into readable lines: one per distinct rule, carrying the advice once and
/// the lines it occurred on (capped), highest-severity first.
fn group_hits(hits: &[Hit]) -> Vec<String> {
    let mut groups: Vec<(String, String, String, Vec<usize>)> = Vec::new(); // (rule, sev, advice, lines)
    for h in hits {
        let advice = if h.advice.is_empty() { format!("violates `{}`", h.rule) } else { h.advice.clone() };
        if let Some(g) = groups.iter_mut().find(|g| g.0 == h.rule) {
            g.3.push(h.line);
        } else {
            groups.push((h.rule.clone(), h.severity.clone(), advice, vec![h.line]));
        }
    }
    groups.sort_by(|a, b| severity_rank(&a.1).cmp(&severity_rank(&b.1)).then_with(|| b.3.len().cmp(&a.3.len())));
    groups
        .into_iter()
        .map(|(rule, sev, advice, mut lines)| {
            lines.sort_unstable();
            let count = lines.len();
            let shown: Vec<String> = lines.iter().take(6).map(usize::to_string).collect();
            let more = if count > 6 { format!(", +{} more", count - 6) } else { String::new() };
            let occ = if count == 1 { format!("L{}", lines[0]) } else { format!("×{count} (lines {}{more})", shown.join(", ")) };
            format!("[{sev}] [{rule}] {advice}  {occ}")
        })
        .collect()
}

/// Render the review: verdict, per-file findings, what could not be analyzed, training sources.
fn render(
    root: &Path,
    reports: &[FileReport],
    by_language: &BTreeMap<String, usize>,
    unanalyzed: &BTreeMap<String, usize>,
    sources: &[String],
    max: usize,
) -> String {
    let mut s = String::new();
    let analyzed: usize = by_language.values().sum();
    let langs: Vec<String> = by_language.iter().map(|(l, n)| format!("{l} ({n})")).collect();
    s.push_str(&format!(
        "I read {} and analyzed {analyzed} source file(s): {}.\n\n",
        root.display(),
        if langs.is_empty() { "none".to_string() } else { langs.join(", ") }
    ));

    let total: usize = reports.iter().map(|f| f.hits.len()).sum();
    if total == 0 {
        s.push_str("Verdict: CLEAN. No structural outliers detected against the project's own norm.\n");
    } else {
        let (mut hi, mut me, mut lo) = (0usize, 0usize, 0usize);
        for f in reports {
            for h in &f.hits {
                match h.severity.as_str() {
                    "high" => hi += 1,
                    "low" => lo += 1,
                    _ => me += 1,
                }
            }
        }
        s.push_str(&format!(
            "Verdict: {total} issue(s) across {} of {analyzed} file(s) — {hi} high, {me} medium, {lo} low.\n",
            reports.len()
        ));
        let mut shown = 0usize;
        for f in reports {
            if shown >= max { break; }
            s.push_str(&format!("\n{}\n", f.path));
            for line in group_hits(&f.hits) {
                if shown >= max {
                    s.push_str("  …raise `max` to see more.\n");
                    break;
                }
                s.push_str(&format!("  {line}\n"));
                shown += 1;
            }
        }
    }

    if !unanalyzed.is_empty() {
        let u: Vec<String> = unanalyzed.iter().map(|(l, n)| format!("{l} ({n})")).collect();
        s.push_str(&format!("\nLanguages without AST support (not analyzed): {}.\n", u.join(", ")));
    }

    if !sources.is_empty() {
        s.push_str(&format!("\nTrained from: {}.\n", sources.join(", ")));
    }
    s
}

// ── runtime resource resolution ──────────────────────────────────────────────

/// Public for sibling tools that need the same data root.
pub(crate) fn data_root_pub() -> PathBuf { data_root() }

/// Locate the directory that holds the linter's knowledge sources (`extraDocs/`, `lint-index/`).
/// Prefers the resolved workspace root (the dev checkout); otherwise walks up from the executable.
/// Always returns a path — missing files fall back to the embedded copies in [`crate::lint_train`].
fn data_root() -> PathBuf {
    let ws = workspace_root();
    if ws.join("extraDocs").exists() || ws.join("lint-index").exists() {
        return ws;
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        while let Some(d) = dir {
            if d.join("extraDocs").exists() || d.join("lint-index").exists() {
                return d;
            }
            dir = d.parent().map(Path::to_path_buf);
        }
    }
    ws
}


// ── schema ───────────────────────────────────────────────────────────────────

/// MCP schema for the `lint` tool.
pub fn schema() -> Value {
    json!({
        "name": "lint",
        "description": "AI lint for the whole project. Trains itself from live official docs per language plus two file sources: extraDocs/ (global principles) and .helpers/lint-rules/ (project-local rules, the project's law). Fires learned AST patterns via tree-sitter, confirms imprecise matches with the Hv concept gate, and self-quarantines mislearned rules. Flag wrong/missing findings with lint_flag.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "root": { "type": "string", "description": "Project root. Defaults to the current workspace." },
                "max": { "type": "integer", "description": "Max finding lines to list (1-500). Default 80." },
                "modules": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional language filter: `rust`, `python`, `js`/`ts`, `go`. `all` or omitted reviews every language."
                }
            },
            "required": []
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_hits_orders_by_severity_and_collapses() {
        let hits = vec![
            Hit { line: 9, rule: "a".into(), severity: "low".into(), advice: "x".into() },
            Hit { line: 3, rule: "b".into(), severity: "high".into(), advice: "y".into() },
            Hit { line: 5, rule: "b".into(), severity: "high".into(), advice: "y".into() },
        ];
        let lines = group_hits(&hits);
        assert!(lines[0].contains("[high]") && lines[0].contains("×2"), "high collapses first: {lines:?}");
        assert!(lines[1].contains("[low]"));
    }

    #[test]
    fn data_root_resolves_to_a_dir_with_sources_or_workspace() {
        let d = data_root();
        assert!(d.join("extraDocs").exists() || d.join("lint-index").exists() || d.exists());
    }

    #[test]
    fn unknown_lang_in_filter_passes_through_not_silently_dropped() {
        // An unrecognised language name should reach the filter set unchanged so
        // the caller sees zero files for it rather than "all languages" being reviewed.
        let f = parse_lang_filter(&json!({ "modules": ["elixir"] })).unwrap();
        assert!(f.contains("elixir"), "unknown lang passes through: {f:?}");
    }

    #[test]
    fn extension_aliases_resolve_to_canonical_names() {
        let f = parse_lang_filter(&json!({ "modules": ["ts", "py"] })).unwrap();
        assert!(f.contains("typescript") && f.contains("python"));
    }
}
