//! `cs_lint` — a deterministic CS2420/CS3500 software-principle scanner.
//!
//! Ported from the MyEditor quality engine: it reports principle *violations*
//! (single responsibility, documentation, error handling, maintainability) as
//! one clean, prioritized list with `file:line`, a message, and a concrete
//! suggestion — so an agent can see exactly what to fix and track progress.
//!
//! It complements `git-cs-grade` (which produces the rubric grade): `helpers grade`
//! tells you *where you stand*; `cs_lint` tells you *the specific lines to fix*.
//! Fully deterministic, no AI.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use regex::Regex;
use serde_json::{json, Value};

use crate::git::workspace_root;
use crate::index::walk::walk_repo;
use crate::lint_index::{self, Resolution};
use crate::proto::{text, ToolResult};

// ── thresholds (mirrors the MyEditor quality engine) ─────────────────────────
const SOURCE_LONG_FILE: usize = 700;
const TEST_LONG_FILE: usize = 900;
const LONG_FN_HARD: usize = 320; // span alone flags
const LONG_FN_SOFT: usize = 200; // span + decisions flags
const LONG_FN_DECISIONS: usize = 20;
const LARGE_BLOCK: usize = 55;

#[derive(Clone)]
struct Issue {
    severity: Sev,
    category: &'static str,
    file: String,
    line: usize,
    message: String,
    suggestion: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Sev {
    // Ordered so High sorts first.
    High = 0,
    Medium = 1,
    Low = 2,
}

impl Sev {
    fn label(self) -> &'static str {
        match self {
            Sev::High => "high",
            Sev::Medium => "medium",
            Sev::Low => "low",
        }
    }
}

fn root_arg(args: &Value) -> PathBuf {
    match args.get("root").and_then(Value::as_str) {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => workspace_root(),
    }
}

/// Scan the project and return the prioritized CS2420/CS3500 violation list.
pub fn run(args: &Value) -> ToolResult {
    let root = root_arg(args);
    if !root.exists() {
        return Err(format!("cs_lint: path not found: {}", root.display()));
    }
    let max = args
        .get("max")
        .and_then(Value::as_u64)
        .unwrap_or(80)
        .clamp(1, 500) as usize;

    let mut issues: Vec<Issue> = Vec::new();
    let mut lang_counts: HashMap<&'static str, usize> = HashMap::new();
    for f in walk_repo(&root) {
        let Some(lang) = Lang::from_ext(&f.ext) else {
            continue;
        };
        if is_declaration_file(&f.rel) {
            continue;
        }
        *lang_counts.entry(lang.id()).or_insert(0) += 1;
        let Ok(content) = std::fs::read_to_string(&f.abs) else {
            continue;
        };
        // Respect opt-out markers, matching the MyEditor engine.
        let lower = content.to_lowercase();
        let allow_long_fn = lower.contains("quality:allow-long-function");
        let allow_block = lower.contains("quality:allow-large-block");
        let allow_long_file = lower.contains("quality:allow-long-file");
        let lines: Vec<&str> = content.lines().collect();
        scan_file(
            &f.rel,
            lang,
            &lines,
            allow_long_fn,
            allow_block,
            allow_long_file,
            &mut issues,
        );
    }

    issues.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });

    // Fast-path note: if a packed lint index covers the dominant language's
    // toolchain, surface that its official rules are active. The built-in checks
    // above remain the fallback and run unconditionally.
    let official_note = dominant_lang(&lang_counts).and_then(official_rules_note);

    let mut out = render(&issues, max);
    if let Some(note) = official_note {
        out.push('\n');
        out.push_str(&note);
    }
    Ok(vec![text(out)])
}

/// The language with the most scanned files, if any — the one whose packed lint
/// index (if present) is most relevant to surface.
fn dominant_lang(counts: &HashMap<&'static str, usize>) -> Option<&'static str> {
    counts
        .iter()
        .max_by_key(|(lang, n)| (**n, **lang))
        .map(|(lang, _)| *lang)
}

/// The canonical (tool, toolchain-version-command) pair Helpers indexes for a
/// language id, or `None` when no official lint index is tracked for it.
fn lang_tool(lang: &str) -> Option<(&'static str, &'static [&'static str])> {
    match lang {
        "rust" => Some(("clippy", &["rustc", "--version"])),
        "python" => Some(("ruff", &["ruff", "--version"])),
        "js" => Some(("eslint", &["eslint", "--version"])),
        "go" => Some(("go-vet", &["go", "version"])),
        _ => None,
    }
}

/// Detect a tool's toolchain version by running its version command and
/// extracting the first dotted-number token (e.g. `rustc 1.95.0` → `1.95.0`).
/// Returns `None` when the command is unavailable or prints no version.
fn detect_version(cmd: &[&str]) -> Option<String> {
    let (prog, args) = cmd.split_first()?;
    let output = Command::new(prog).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let re = Regex::new(r"\d+\.\d+(?:\.\d+)?").ok()?;
    re.find(&text).map(|m| m.as_str().to_string())
}

/// Build the "official rules active" note for `lang` when a packed lint index
/// resolves on the fast path (file present, checksum intact, version covered).
/// Returns `None` when no index is tracked, the toolchain can't be detected, or
/// the fast path misses — so the renderer stays unchanged with no index present.
fn official_rules_note(lang: &str) -> Option<String> {
    let (tool, version_cmd) = lang_tool(lang)?;
    let version = detect_version(version_cmd)?;
    match lint_index::resolve(tool, lang, &version) {
        Resolution::Packed(idx) => Some(format!(
            "\n_Official rules: {} {} rules active for {} v{} (sourced from docs v{}). \
             These augment the built-in checks above._\n",
            idx.rule_count, tool, tool, version, idx.docs_version
        )),
        Resolution::NeedsCrawl { .. } => None,
    }
}

// ── languages ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum Lang {
    Rust,
    Go,
    Js,
    Python,
    JavaLike,
}

impl Lang {
    fn from_ext(ext: &str) -> Option<Lang> {
        Some(match ext {
            "rs" => Lang::Rust,
            "go" => Lang::Go,
            "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" => Lang::Js,
            "py" => Lang::Python,
            "java" | "cs" | "kt" | "swift" | "cpp" | "cc" | "c" => Lang::JavaLike,
            _ => return None,
        })
    }
    fn brace_based(self) -> bool {
        !matches!(self, Lang::Python)
    }
    /// Stable lowercase id used to key the packed lint index by language.
    fn id(self) -> &'static str {
        match self {
            Lang::Rust => "rust",
            Lang::Go => "go",
            Lang::Js => "js",
            Lang::Python => "python",
            Lang::JavaLike => "java",
        }
    }
}

// ── per-file scanning ────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn scan_file(
    rel: &str,
    lang: Lang,
    lines: &[&str],
    allow_long_fn: bool,
    allow_block: bool,
    allow_long_file: bool,
    out: &mut Vec<Issue>,
) {
    let fns = fn_pattern(lang);
    let mut missing_docs: Vec<String> = Vec::new();

    for (idx, raw) in lines.iter().enumerate() {
        if let Some(caps) = fns.captures(raw) {
            let name = captured_name(&caps);
            let public = is_public(lang, name, raw);
            let span = if lang.brace_based() {
                brace_span(lines, idx)
            } else {
                indent_span(lines, idx)
            };
            let decisions = decision_count(lines, idx, span);

            if !allow_long_fn && flag_long_fn(rel, name, span, decisions) {
                out.push(Issue {
                    severity: Sev::Medium,
                    category: "cs-principle",
                    file: rel.to_string(),
                    line: idx + 1,
                    message: format!(
                        "Function `{name}` spans {span} lines with {decisions} decision points; likely violating single responsibility."
                    ),
                    suggestion: "Extract focused helpers so each unit has one clear responsibility.",
                });
            }
            if public && !has_doc_above(lang, lines, idx) {
                missing_docs.push(name.to_string());
            }
        }
    }

    if !missing_docs.is_empty() {
        let preview = missing_docs.join(", ");
        let preview = if preview.len() > 160 {
            format!("{}…", &preview[..160])
        } else {
            preview
        };
        out.push(Issue {
            severity: Sev::Medium,
            category: "documentation-gap",
            file: rel.to_string(),
            line: 1,
            message: format!(
                "{} public function(s) lack a doc comment: {preview}",
                missing_docs.len()
            ),
            suggestion: "Add a concise contract comment for each exported/public function.",
        });
    }

    // Long file.
    if !allow_long_file {
        let limit = if is_test_path(rel) {
            TEST_LONG_FILE
        } else {
            SOURCE_LONG_FILE
        };
        if lines.len() > limit {
            out.push(Issue {
                severity: Sev::Low,
                category: "maintainability",
                file: rel.to_string(),
                line: 1,
                message: format!(
                    "File is {} lines (> {limit}); hard to navigate.",
                    lines.len()
                ),
                suggestion: "Split into cohesive modules with single responsibilities.",
            });
        }
    }

    // Large uncommented blocks + error handling.
    if !allow_block {
        large_uncommented_blocks(rel, lang, lines, out);
    }
    error_handling(rel, lang, lines, out);
}

/// Per-language function-declaration regex (capture 1 = name).
fn fn_pattern(lang: Lang) -> Regex {
    let p = match lang {
        Lang::Rust => {
            r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*[(<]"
        }
        Lang::Go => r"^\s*func\s+(?:\([^)]*\)\s*)?([A-Za-z_][A-Za-z0-9_]*)\s*\(",
        Lang::Js => {
            r"^\s*(?:export\s+(?:default\s+)?)?(?:async\s+)?function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(|^\s*(?:export\s+)?(?:const|let)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:async\s*)?\("
        }
        Lang::Python => r"^\s*(?:async\s+)?def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(",
        Lang::JavaLike => {
            r"^\s*(?:(?:public|private|protected|internal|static|final|virtual|override|abstract|synchronized|async|sealed|partial)\s+)+[A-Za-z_][A-Za-z0-9_<>,\[\].?]*\s+([A-Za-z_][A-Za-z0-9_]*)\s*\([^;]*\)\s*\{"
        }
    };
    Regex::new(p).expect("valid fn regex")
}

/// JS has two capture groups (function / const-arrow); fold to the matched one.
fn captured_name<'a>(caps: &regex::Captures<'a>) -> &'a str {
    caps.get(1)
        .or_else(|| caps.get(2))
        .map(|m| m.as_str())
        .unwrap_or("")
}

fn is_public(lang: Lang, name: &str, decl_line: &str) -> bool {
    match lang {
        Lang::Rust => decl_line.trim_start().starts_with("pub"),
        Lang::Go => name.chars().next().is_some_and(|c| c.is_ascii_uppercase()),
        Lang::Js => decl_line.contains("export"),
        Lang::Python => !name.starts_with('_'),
        Lang::JavaLike => decl_line.contains("public"),
    }
}

/// True when the line(s) directly above `idx` form a doc/comment.
fn has_doc_above(lang: Lang, lines: &[&str], idx: usize) -> bool {
    if idx == 0 {
        return false;
    }
    let prev = lines[idx - 1].trim();
    matches!(lang, Lang::Python) && prev.starts_with('#')
        || prev.starts_with("//")
        || prev.starts_with("///")
        || prev.starts_with('*')
        || prev.ends_with("*/")
        || prev.starts_with("\"\"\"") // python docstring just below handled loosely
}

/// Span of a brace-delimited body: from the opening `{` until depth returns to 0.
fn brace_span(lines: &[&str], start: usize) -> usize {
    let mut depth: i32 = 0;
    let mut opened = false;
    for (n, line) in lines.iter().enumerate().skip(start) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        if opened && depth <= 0 {
            return n - start + 1;
        }
    }
    1
}

/// Span of a Python def by indentation: lines more-indented than the `def`.
fn indent_span(lines: &[&str], start: usize) -> usize {
    let base = indent_of(lines[start]);
    let mut end = start;
    for (n, line) in lines.iter().enumerate().skip(start + 1) {
        if line.trim().is_empty() {
            continue;
        }
        if indent_of(line) <= base {
            break;
        }
        end = n;
    }
    end - start + 1
}

fn indent_of(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ' || *c == '\t').count()
}

/// Count branch/decision points across a function body (cyclomatic-ish).
fn decision_count(lines: &[&str], start: usize, span: usize) -> usize {
    let kw = ["if ", "for ", "while ", "case ", "catch", "elif ", "match "];
    let mut count = 0;
    for line in lines.iter().skip(start).take(span) {
        let t = line.trim_start();
        for k in kw {
            if t.starts_with(k) {
                count += 1;
            }
        }
        count += line.matches("&&").count();
        count += line.matches("||").count();
    }
    count
}

/// MyEditor's long-function policy: UI components get a high bar; otherwise a
/// hard span cap, or a soft span with enough decision points.
fn flag_long_fn(rel: &str, name: &str, span: usize, decisions: usize) -> bool {
    let lower = rel.to_lowercase();
    let ui = lower.ends_with(".tsx")
        || lower.ends_with(".jsx")
        || name.ends_with("Panel")
        || name.ends_with("Screen")
        || name.ends_with("View");
    if ui {
        return span >= 700 && decisions >= 70;
    }
    span >= LONG_FN_HARD || (span >= LONG_FN_SOFT && decisions >= LONG_FN_DECISIONS)
}

/// Flag contiguous code runs >= LARGE_BLOCK lines with no comment inside.
fn large_uncommented_blocks(rel: &str, lang: Lang, lines: &[&str], out: &mut Vec<Issue>) {
    let line_comment = match lang {
        Lang::Python => "#",
        _ => "//",
    };
    let mut start = 0usize;
    let mut run = 0usize;
    let mut has_comment = false;
    let flush = |start: usize, run: usize, has_comment: bool, out: &mut Vec<Issue>| {
        if run >= LARGE_BLOCK && !has_comment {
            out.push(Issue {
                severity: Sev::Medium,
                category: "large-block-without-comment",
                file: rel.to_string(),
                line: start + 1,
                message: format!("Large code block ({run} lines) has no guiding comments."),
                suggestion: "Split into smaller helpers and annotate non-obvious intent.",
            });
        }
    };
    for (idx, raw) in lines.iter().enumerate() {
        let t = raw.trim();
        if t.is_empty() {
            flush(start, run, has_comment, out);
            run = 0;
            has_comment = false;
            start = idx + 1;
            continue;
        }
        if run == 0 {
            start = idx;
        }
        if t.starts_with(line_comment) || t.starts_with("/*") || t.starts_with('*') {
            has_comment = true;
        }
        run += 1;
    }
    flush(start, run, has_comment, out);
}

/// Error-handling smells: empty catch, ignored Go errors, empty Python except.
fn error_handling(rel: &str, lang: Lang, lines: &[&str], out: &mut Vec<Issue>) {
    let empty_catch = Regex::new(r"catch\s*\([^)]*\)\s*\{\s*\}").unwrap();
    for (idx, raw) in lines.iter().enumerate() {
        match lang {
            Lang::Js | Lang::JavaLike => {
                if empty_catch.is_match(raw) {
                    out.push(Issue {
                        severity: Sev::High,
                        category: "cs-principle",
                        file: rel.to_string(),
                        line: idx + 1,
                        message: "Empty catch block swallows errors silently.".into(),
                        suggestion: "Handle, log, or rethrow the error — never swallow it.",
                    });
                }
            }
            Lang::Go => {
                let t = raw.trim();
                if t.starts_with("_ =") && t.contains("err") {
                    out.push(Issue {
                        severity: Sev::Medium,
                        category: "cs-principle",
                        file: rel.to_string(),
                        line: idx + 1,
                        message: "Error assigned to `_` is ignored.".into(),
                        suggestion: "Check and handle the error instead of discarding it.",
                    });
                }
            }
            Lang::Python => {
                let t = raw.trim();
                if t.starts_with("except") && t.ends_with(':') {
                    // Empty body when the next non-blank line is `pass`.
                    if let Some(next) = lines.get(idx + 1) {
                        if next.trim() == "pass" {
                            out.push(Issue {
                                severity: Sev::High,
                                category: "cs-principle",
                                file: rel.to_string(),
                                line: idx + 1,
                                message: "`except: pass` silently swallows exceptions.".into(),
                                suggestion: "Handle or log the exception; narrow the except type.",
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// TypeScript ambient declaration files (`.d.ts`) declare external API surface,
/// not project implementation, so the principle checks (docs, single
/// responsibility, error handling) do not apply to them.
fn is_declaration_file(rel: &str) -> bool {
    rel.ends_with(".d.ts")
}

fn is_test_path(p: &str) -> bool {
    let pl = p.to_lowercase();
    pl.contains("/test")
        || pl.contains("test/")
        || pl.ends_with("_test.go")
        || pl.ends_with(".test.ts")
        || pl.ends_with(".test.js")
        || pl.ends_with(".spec.ts")
        || pl.ends_with("_test.py")
}

// ── rendering ────────────────────────────────────────────────────────────────

fn render(issues: &[Issue], max: usize) -> String {
    if issues.is_empty() {
        return "✓ No CS2420/CS3500 principle violations found.\n\nClean: single responsibility, documentation, error handling, and maintainability all pass.".into();
    }
    let (mut hi, mut med, mut lo) = (0, 0, 0);
    for i in issues {
        match i.severity {
            Sev::High => hi += 1,
            Sev::Medium => med += 1,
            Sev::Low => lo += 1,
        }
    }
    let mut s = String::new();
    s.push_str(&format!(
        "# CS2420/CS3500 principle review — {} issue(s): {hi} high, {med} medium, {lo} low\n\n",
        issues.len()
    ));
    s.push_str("_Fix high first. Each line is a concrete, deterministic violation; re-run `cs_lint` to watch the count drop. Pair with `helpers grade` for the rubric._\n\n");

    for i in issues.iter().take(max) {
        s.push_str(&format!(
            "- [{}] {}:{} — {} ({})\n    → {}\n",
            i.severity.label(),
            i.file,
            i.line,
            i.message,
            i.category,
            i.suggestion
        ));
    }
    if issues.len() > max {
        s.push_str(&format!(
            "\n…and {} more (raise `max`).\n",
            issues.len() - max
        ));
    }
    s
}

// ── schema ───────────────────────────────────────────────────────────────────

/// MCP schema for the `cs_lint` tool.
pub fn schema() -> Value {
    json!({
        "name": "cs_lint",
        "description": "Scan the project for CS2420/CS3500 software-principle violations and return one clean, prioritized list (severity, file:line, message, fix suggestion). Covers single responsibility (long functions), documentation gaps (undocumented public functions), error handling (empty catch / ignored errors), and maintainability (long files, large uncommented blocks). Deterministic, no AI. Complements `helpers grade`: grade gives the rubric, cs_lint gives the exact lines to fix. Re-run to track progress as the count drops.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "root": { "type": "string", "description": "Project root. Defaults to the current workspace." },
                "max": { "type": "integer", "description": "Max issues to list (1-500). Default 80." }
            },
            "required": []
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_long_function_and_clean_passes() {
        assert!(flag_long_fn("src/x.rs", "f", 330, 2));
        assert!(flag_long_fn("src/x.rs", "f", 210, 25));
        assert!(!flag_long_fn("src/x.rs", "f", 150, 5));
        // UI components get a much higher bar.
        assert!(!flag_long_fn("ui/Panel.tsx", "MyPanel", 400, 30));
    }

    #[test]
    fn brace_span_counts_body_lines() {
        let src = ["fn a() {", "  let x = 1;", "  x + 1", "}"];
        assert_eq!(brace_span(&src, 0), 4);
    }

    #[test]
    fn detects_empty_catch_and_doc_gap() {
        let lines = vec![
            "export function doThing() {",
            "  try { risky(); } catch (e) {}",
            "}",
        ];
        let mut out = Vec::new();
        scan_file("a.ts", Lang::Js, &lines, false, false, false, &mut out);
        assert!(out
            .iter()
            .any(|i| i.category == "cs-principle" && i.message.contains("Empty catch")));
        assert!(out.iter().any(|i| i.category == "documentation-gap"));
    }

    #[test]
    fn declaration_files_are_skipped() {
        assert!(is_declaration_file("vscode.proposed.foo.d.ts"));
        assert!(is_declaration_file("types/index.d.ts"));
        assert!(!is_declaration_file("src/index.ts"));
        assert!(!is_declaration_file("src/app.js"));
    }

    #[test]
    fn js_const_arrow_name_is_captured() {
        let re = fn_pattern(Lang::Js);
        let caps = re
            .captures("export const handler = async (req) => {")
            .unwrap();
        assert_eq!(captured_name(&caps), "handler");
    }

    #[test]
    fn dominant_lang_picks_most_files() {
        let mut counts: HashMap<&'static str, usize> = HashMap::new();
        counts.insert("rust", 5);
        counts.insert("python", 2);
        assert_eq!(dominant_lang(&counts), Some("rust"));
        assert_eq!(dominant_lang(&HashMap::new()), None);
    }

    #[test]
    fn lang_tool_maps_known_languages_only() {
        assert_eq!(lang_tool("rust").map(|(t, _)| t), Some("clippy"));
        assert_eq!(lang_tool("python").map(|(t, _)| t), Some("ruff"));
        assert!(lang_tool("java").is_none());
    }
}
