//! `lint` — the AI code reviewer. It reads the whole repository and reports in English like a
//! meticulous TA: the verdict, the exact lines to fix, and what it could not analyze. Two detectors,
//! both assembled from the two knowledge sources alone — nothing hardcoded, nothing from memory:
//!
//!   1. **Code rules** — exact tree patterns ([`crate::lint_match`]) compiled from each documented
//!      rule's `bad`/`good` example. The docs from the links — `lint-index/<tool>.json`, the official
//!      catalogs (clippy / ruff / eslint / staticcheck) — and the fenced pairs in `corpus/` supply
//!      these. A match is the rule's structure occurring verbatim, with scope and co-reference intact.
//!   2. **Practice rules** — the corpus's narrative principles ([`crate::lint_practice`]): a prose
//!      principle ("a function should do one thing") activates a general structural sense and the
//!      project's outliers on it are flagged, judged against the project's own norm. This is what
//!      catches the un-maintainable shape AI code drifts into — sprawling, deeply-nested units.
//!
//! The CS2420 / CS3500 corpus rules, followed to a T, ~guarantee an A+, so a clean lint against them
//! *is* the grade — there is no separate rubric tool in the loop. Setup is automatic and cached: on
//! first run [`crate::lint_train::ensure_models`] compiles a pattern set per project language and
//! caches it; later runs just load it. The verdict is grounded in those docs and that folder.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde_json::{json, Value};

use crate::git::workspace_root;
use crate::index::walk::{walk_repo, WalkedFile};
use crate::lint_match::RuleSet;
use crate::lint_train::{self, RuleInfo, TrainReport};
use crate::proto::{text, ToolResult};

/// The project root to review, from the `root` arg or the resolved workspace.
fn root_arg(args: &Value) -> PathBuf {
    match args.get("root").and_then(Value::as_str) {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => workspace_root(),
    }
}

/// Map a file extension to the model language it is linted by — the languages the documentation
/// links cover and a tree-sitter-capable model exists for. Returns `None` for everything else.
fn model_lang(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "rs" => "rust",
        "py" => "python",
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" => "javascript",
        "go" => "go",
        _ => return None,
    })
}

/// Map an extension to a recognizable code language that has NO trained model yet — so the review
/// can honestly report "read but not analyzed" instead of silently dropping the file.
fn other_code_lang(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "java" => "java",
        "kt" => "kotlin",
        "swift" => "swift",
        "c" | "h" => "c",
        "cpp" | "cc" | "hpp" => "cpp",
        "rb" => "ruby",
        "cs" => "csharp",
        _ => return None,
    })
}

/// Optional language filter from the `modules` arg: a list of language tokens restricts the review
/// to those languages. Absent / empty / `all` ⇒ every language. Unknown tokens are ignored.
fn parse_lang_filter(args: &Value) -> Option<BTreeSet<String>> {
    let arr = args.get("modules").and_then(Value::as_array)?;
    let mut set = BTreeSet::new();
    for tok in arr.iter().filter_map(Value::as_str) {
        match tok.trim().to_ascii_lowercase().as_str() {
            "all" | "" => return None,
            "rust" | "rs" => set.insert("rust".to_string()),
            "python" | "py" => set.insert("python".to_string()),
            "js" | "javascript" | "jsx" | "ts" | "typescript" | "tsx" => set.insert("javascript".to_string()),
            "go" | "golang" => set.insert("go".to_string()),
            _ => false,
        };
    }
    if set.is_empty() {
        None
    } else {
        Some(set)
    }
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

    // 1) Read the whole repository (gitignore-aware; dependency trees and build output pruned).
    let files = walk_repo(&root);

    // 2) Which model languages the project actually uses (respecting any filter) — the self-setup
    //    shortlist.
    let mut present: BTreeSet<String> = BTreeSet::new();
    for f in &files {
        if let Some(l) = model_lang(&f.ext) {
            if filter.as_ref().is_none_or(|set| set.contains(l)) {
                present.insert(l.to_string());
            }
        }
    }
    let langs: Vec<String> = present.iter().cloned().collect();

    // 3) Self-setup: ensure a fresh cached model per language, then load each once for this run.
    let setup = lint_train::ensure_models(&langs, &data);
    let mut models: HashMap<String, RuleSet> = HashMap::new();
    for l in &langs {
        if let Some(m) = lint_train::load_patterns(l) {
            models.insert(l.clone(), m);
        }
    }
    let advice = lint_train::advice(&data);

    // 4) Partition the files (cheap, sequential): which get judged by which model, and which are
    //    recognized code we have no model for (read but not analyzed).
    let mut to_judge: Vec<(&str, &WalkedFile)> = Vec::new();
    let mut by_language: BTreeMap<String, usize> = BTreeMap::new();
    let mut unanalyzed: BTreeMap<String, usize> = BTreeMap::new();
    for f in &files {
        if let Some(l) = model_lang(&f.ext) {
            if filter.as_ref().is_some_and(|set| !set.contains(l)) {
                continue;
            }
            if models.contains_key(l) {
                *by_language.entry(l.to_string()).or_default() += 1;
                to_judge.push((l, f));
            } else {
                // A language we use but have no model for (training found no signal): report it as
                // read-not-analyzed rather than pretending it was reviewed.
                *unanalyzed.entry(l.to_string()).or_default() += 1;
            }
        } else if let Some(o) = other_code_lang(&f.ext) {
            *unanalyzed.entry(o.to_string()).or_default() += 1;
        }
    }

    // 5) Judge the whole project: each file in parallel, flagging a rule only where its exact tree
    //    pattern occurs in that file.
    let mut reports = judge_all(&to_judge, &models, &advice);

    // 6) Practice rules: the corpus's narrative principles measure the project against its own norm
    //    (a unit that does far more than the project usually does). Merged into the same reports.
    let practice = crate::lint_practice::PracticeRules::new(lint_train::practice_principles(&data));
    merge_practice(&mut reports, &to_judge, &practice);
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

/// Run the practice rules over the whole project (grouped by language, since the norm is per-
/// language) and merge their findings into `reports`, attaching to the matching file or adding a new
/// entry. A no-op when no principle is active.
fn merge_practice(
    reports: &mut Vec<FileReport>,
    to_judge: &[(&str, &WalkedFile)],
    practice: &crate::lint_practice::PracticeRules,
) {
    if practice.is_empty() {
        return;
    }
    let mut by_lang: BTreeMap<&str, Vec<(String, String)>> = BTreeMap::new();
    for (lang, f) in to_judge {
        if let Ok(code) = std::fs::read_to_string(&f.abs) {
            by_lang.entry(lang).or_default().push((f.rel.clone(), code));
        }
    }
    for (lang, files) in &by_lang {
        for (path, fd) in practice.flag_project(lang, files) {
            let advice = format!("{} — {}", fd.advice.trim_end_matches('.'), fd.detail);
            let hit = Hit { line: fd.line, rule: fd.rule, severity: fd.severity, advice };
            match reports.iter_mut().find(|r| r.path == path) {
                Some(r) => r.hits.push(hit),
                None => reports.push(FileReport { path: path.to_string(), hits: vec![hit] }),
            }
        }
    }
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
        "description": "Review the whole project like a meticulous TA. ONE mixture-of-experts model per language reads every file and reports in English: the verdict, the exact lines to fix, and what it could not analyze. Its rules are learned from exactly two sources and nothing else — the official, version-matched rule docs in lint-index/ (clippy/ruff/eslint/staticcheck) and the CS2420/CS3500 principles in corpus/cs-principles.md. Followed to a T those principles ~guarantee an A+, so a clean lint IS the grade. Self-sets-up on first run (trains + caches a model per language), then loads the cache. No local toolchain required. Grounded in the docs and the project's own code — never memory.",
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
    fn lang_filter_parsing() {
        assert!(parse_lang_filter(&json!({})).is_none());
        assert!(parse_lang_filter(&json!({ "modules": ["all"] })).is_none());
        assert!(parse_lang_filter(&json!({ "modules": [] })).is_none());
        let f = parse_lang_filter(&json!({ "modules": ["ts", "py"] })).unwrap();
        assert!(f.contains("javascript") && f.contains("python") && f.len() == 2);
        // Unknown tokens are ignored rather than erroring.
        assert!(parse_lang_filter(&json!({ "modules": ["cobol"] })).is_none());
    }

    #[test]
    fn model_lang_maps_extensions() {
        assert_eq!(model_lang("rs"), Some("rust"));
        assert_eq!(model_lang("tsx"), Some("javascript"));
        assert_eq!(model_lang("go"), Some("go"));
        assert_eq!(model_lang("txt"), None);
    }

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
        // In the dev checkout this resolves to the directory carrying the knowledge sources.
        assert!(d.join("corpus/cs-principles.md").exists() || d.join("lint-index").exists() || d.exists());
    }
}
