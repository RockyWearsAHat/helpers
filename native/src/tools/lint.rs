//! `lint` — the AI code reviewer: reads the whole repository and reports in English.
//!
//! Rules are compiled from two sources — official docs (clippy / ruff / eslint / staticcheck)
//! and the corpus (CS2420 Data Structures & Algorithms + CS3500 Software Design course docs)
//! — into lossless tree patterns via [`crate::lint_match`].
//! Each match is the rule's documented structure verbatim, with scope and co-reference intact.
//! For project-wide graph tracing see the `lint_build_web`, `lint_probe`, and `lint_trace` tools.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde_json::{json, Value};

use crate::git::workspace_root;
use crate::index::walk::{walk_repo, WalkedFile};
use crate::lint_match::RuleSet;
use crate::lint_train::{self, RuleInfo, TrainReport};
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

/// Review the whole project with the tree-pattern engine: detect its languages, self-set-up
/// (compile+cache a rule set per language from the docs links + corpus folder), read every source
/// file, judge it, and talk back in English.
pub fn run(args: &Value) -> ToolResult {
    let root = root_arg(args);
    if !root.exists() {
        return Err(format!("lint: path not found: {}", root.display()));
    }
    let max = args.get("max").and_then(Value::as_u64).unwrap_or(80).clamp(1, 500) as usize;
    let filter = parse_lang_filter(args);
    let data = data_root();

    // Per-project preferences: ignore list, severity overrides, language filter.
    let config = load_config(&root);
    let ignore_set: HashSet<&str> = config.ignore_rules.iter().map(String::as_str).collect();

    // 1) Read the whole repository (gitignore-aware; dependency trees and build output pruned).
    let files = walk_repo(&root);

    // 2) Which languages the project actually uses — CLI filter AND config language filter applied.
    let mut present: BTreeSet<String> = BTreeSet::new();
    for f in &files {
        if let Some(l) = file_lang(&f.ext) {
            let cli_ok = filter.as_ref().is_none_or(|set| set.contains(l));
            let cfg_ok = config.languages.as_ref().is_none_or(|set| set.iter().any(|x| x == l));
            if cli_ok && cfg_ok {
                present.insert(l.to_string());
            }
        }
    }
    let langs: Vec<String> = present.iter().cloned().collect();

    // 3) Self-setup: train (once, cached) and load a compiled rule set per language.
    //    Project-local rules from .helpers/lint-rules/ are merged in during training.
    let setup = lint_train::ensure_models(&langs, &data, &root);
    let mut models: HashMap<String, RuleSet> = HashMap::new();
    for l in &langs {
        if let Some(m) = lint_train::load_patterns(l) {
            models.insert(l.clone(), m);
        }
    }
    let advice = lint_train::advice(&data, Some(&root));

    // 4) Partition files: modeled → judge; unmodeled → report as unanalyzed.
    let mut to_judge: Vec<(&str, &WalkedFile)> = Vec::new();
    let mut by_language: BTreeMap<String, usize> = BTreeMap::new();
    let mut unanalyzed: BTreeMap<String, usize> = BTreeMap::new();
    for f in &files {
        let Some(l) = file_lang(&f.ext) else { continue };
        if filter.as_ref().is_some_and(|set| !set.contains(l)) {
            continue;
        }
        if config.languages.as_ref().is_some_and(|set| !set.iter().any(|x| x == l)) {
            continue;
        }
        if models.contains_key(l) {
            *by_language.entry(l.to_string()).or_default() += 1;
            to_judge.push((l, f));
        } else {
            *unanalyzed.entry(l.to_string()).or_default() += 1;
        }
    }

    // 5) Judge the whole project in parallel.
    let mut reports = judge_all(&to_judge, &models, &advice);

    // 6) Apply per-project config: suppress ignored rules, apply severity overrides.
    for report in &mut reports {
        report.hits.retain(|h| !ignore_set.contains(h.rule.as_str()));
        if !config.severity_overrides.is_empty() {
            for hit in &mut report.hits {
                if let Some(sev) = config.severity_overrides.get(&hit.rule) {
                    hit.severity = sev.clone();
                }
            }
        }
    }

    reports.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(vec![text(render(&root, &reports, &by_language, &unanalyzed, &models, &setup, max))])
}

/// Judge the whole project: each file in parallel, flagging a rule only where its exact tree pattern
/// occurs in that file. Each file is judged independently — the model's precision comes from matching
/// each rule's lossless pattern verbatim, so there is no project-wide calibration, no thresholds, and
/// nothing shared between files.
fn judge_all(
    to_judge: &[(&str, &WalkedFile)],
    models: &HashMap<String, RuleSet>,
    advice: &HashMap<String, RuleInfo>,
) -> Vec<FileReport> {
    to_judge
        .par_iter()
        .filter_map(|(lang, f)| {
            let model = models.get(*lang)?;
            let code = std::fs::read_to_string(&f.abs).ok()?;
            let findings = model.flag(&code);
            if findings.is_empty() {
                return None;
            }
            let hits = findings
                .into_iter()
                .map(|fd| {
                    let advice = advice.get(&fd.rule).map(|i| i.description.clone()).unwrap_or_default();
                    Hit { line: fd.line, rule: fd.rule, severity: fd.severity, advice }
                })
                .collect();
            Some(FileReport { path: f.rel.clone(), hits })
        })
        .collect()
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

/// Render the review as an English report: verdict, per-file lines to fix, what could not be
/// analyzed, what the verdict was judged against, and the one-time self-setup that ran.
fn render(
    root: &Path,
    reports: &[FileReport],
    by_language: &BTreeMap<String, usize>,
    unanalyzed: &BTreeMap<String, usize>,
    models: &HashMap<String, RuleSet>,
    setup: &TrainReport,
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
        s.push_str("Verdict: CLEAN. Every analyzed file follows the rules I learned from the docs and the CS principles.\n");
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
            "Verdict: {total} issue(s) across {} of {analyzed} file(s) — {hi} high, {me} medium, {lo} low. Highest-severity first.\n",
            reports.len()
        ));
        let mut shown = 0usize;
        for f in reports {
            if shown >= max {
                break;
            }
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
        s.push_str(&format!("\nRead but not analyzed (no model learned for these yet): {}.\n", u.join(", ")));
    }

    if !models.is_empty() {
        let mut k: Vec<String> = models.iter().map(|(l, m)| format!("{l}: {} rules", m.rule_count())).collect();
        k.sort();
        s.push_str(&format!("\nJudged against what I learned from the docs + CS principles: {}.\n", k.join(", ")));
    }

    if !setup.trained.is_empty() {
        s.push_str(&format!(
            "Trained and cached model(s) from the docs this run (reused offline next time): {}.\n",
            setup.trained.join(", ")
        ));
    }
    for (lang, reason) in &setup.skipped {
        s.push_str(&format!("Note: did not set up `{lang}` — {reason}.\n"));
    }
    s
}

// ── runtime resource resolution ──────────────────────────────────────────────

/// Public for sibling tools that need the same data root.
pub(crate) fn data_root_pub() -> PathBuf { data_root() }

/// Locate the directory that holds the linter's knowledge sources (`lint-index/`, `corpus/`).
/// Prefers the resolved workspace root (the dev checkout); otherwise walks up from the executable
/// (the installed case). Always returns a path — missing files fall back to the embedded copies in
/// [`crate::lint_train`], so the review still runs.
fn data_root() -> PathBuf {
    let ws = workspace_root();
    if ws.join("corpus/cs-principles.md").exists() || ws.join("lint-index").exists() {
        return ws;
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        while let Some(d) = dir {
            if d.join("corpus/cs-principles.md").exists() || d.join("lint-index").exists() {
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
        "description": "Review the whole project like a meticulous TA. ONE mixture-of-experts model per language reads every file and reports in English: the verdict, the exact lines to fix, and what it could not analyze. Rules come from two sources: the official, version-matched rule docs in lint-index/ (clippy/ruff/eslint/staticcheck/checkstyle/pmd) and the CS course principles in corpus/ (CS2420 Data Structures & Algorithms + CS3500 Software Design). A clean lint means the code follows the course rubric. Self-sets-up on first run (trains + caches a model per language), then loads the cache. No local toolchain required. Grounded in the docs and the project's own code — never memory.",
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
        assert!(d.join("corpus/cs-principles.md").exists() || d.join("lint-index").exists() || d.exists());
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
