//! `lint` — a deterministic CS2420/CS3500 software-principle scanner.
//!
//! Ported from the MyEditor quality engine: it reports principle *violations*
//! (single responsibility, documentation, error handling, maintainability) as
//! one clean, prioritized list with `file:line`, a message, and a concrete
//! suggestion — so an agent can see exactly what to fix and track progress.
//!
//! It complements `git-cs-grade` (which produces the rubric grade): `helpers grade`
//! tells you *where you stand*; `lint` tells you *the specific lines to fix*.
//! Fully deterministic, no AI.

use std::collections::HashMap;
use std::path::PathBuf;

use regex::Regex;
use serde_json::{json, Value};

use crate::git::workspace_root;
use crate::index::walk::walk_repo;
use crate::proto::{text, ToolResult};
use crate::{lint_checkers, lint_index, lint_metrics, lint_moe};

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

/// Map an index severity label (`high`/`medium`/`low`) to the internal bucket.
fn sev_from_label(s: &str) -> Sev {
    match s {
        "high" => Sev::High,
        "low" => Sev::Low,
        _ => Sev::Medium,
    }
}

fn root_arg(args: &Value) -> PathBuf {
    match args.get("root").and_then(Value::as_str) {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => workspace_root(),
    }
}

/// Which lint modules to run, parsed from the `modules` arg. The selection has two
/// independent axes: which *kinds* of check (`cs` principles vs `official` rules) and
/// which *languages*. Both default to "everything" so an empty/absent `modules`
/// preserves the original behaviour.
struct Selection {
    /// Run the CS2420/CS3500 principle checks (single responsibility, docs, error
    /// handling, maintainability).
    cs: bool,
    /// Run the official-doc rules (the MoE "AI tree-sitter" + metric thresholds).
    official: bool,
    /// Restrict to these canonical language ids; `None` ⇒ all languages.
    langs: Option<std::collections::HashSet<&'static str>>,
}

/// Fold a user module token (case-insensitive) to a canonical language id, or `None`
/// if it isn't a language token. Aliases mirror [`Lang::from_ext`] so `js`/`ts` etc.
/// all resolve, matching what the rest of the tool keys on.
fn canon_lang(tok: &str) -> Option<&'static str> {
    // Note: `cs` is deliberately NOT a language token — it names the CS-principle kind.
    Some(match tok {
        "rust" | "rs" => "rust",
        "go" | "golang" => "go",
        "js" | "javascript" | "jsx" | "ts" | "typescript" | "tsx" => "js",
        "python" | "py" => "python",
        "java" | "kotlin" | "kt" | "swift" | "cpp" | "c" => "java",
        _ => return None,
    })
}

/// Parse the `modules` arg into a [`Selection`].
///
/// Recognized tokens: `cs` (principle checks), `official` (official-doc rules), `all`
/// (everything), and language ids (`rust`, `go`, `js`/`ts`, `python`, `java`, …).
/// Rules: absent/empty/`all` ⇒ everything. Listing only languages keeps BOTH check
/// kinds for those languages. Listing a kind (`cs`/`official`) restricts to the kinds
/// named. Unknown tokens are ignored so a typo degrades to "run nothing extra" rather
/// than erroring mid-scan.
fn parse_selection(args: &Value) -> Selection {
    let all = Selection { cs: true, official: true, langs: None };
    let Some(arr) = args.get("modules").and_then(Value::as_array) else {
        return all;
    };
    let toks: Vec<String> = arr
        .iter()
        .filter_map(Value::as_str)
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if toks.is_empty() || toks.iter().any(|t| t == "all") {
        return all;
    }
    let langs: std::collections::HashSet<&'static str> =
        toks.iter().filter_map(|t| canon_lang(t)).collect();
    let wants_cs = toks.iter().any(|t| t == "cs");
    let wants_official = toks.iter().any(|t| t == "official");
    // Languages alone (no kind named) ⇒ run both kinds for them.
    let (cs, official) = if wants_cs || wants_official {
        (wants_cs, wants_official)
    } else {
        (true, true)
    };
    Selection {
        cs,
        official,
        langs: if langs.is_empty() { None } else { Some(langs) },
    }
}

/// Scan the project and return the prioritized CS2420/CS3500 violation list.
pub fn run(args: &Value) -> ToolResult {
    let root = root_arg(args);
    if !root.exists() {
        return Err(format!("lint: path not found: {}", root.display()));
    }
    let max = args
        .get("max")
        .and_then(Value::as_u64)
        .unwrap_or(80)
        .clamp(1, 500) as usize;
    let sel = parse_selection(args);

    let mut issues: Vec<Issue> = Vec::new();
    let mut lang_counts: HashMap<&'static str, usize> = HashMap::new();
    // Metric models, loaded lazily per language.
    let mut metric_models: HashMap<&'static str, Option<lint_metrics::Metrics>> = HashMap::new();
    // Trained MoE linters (the "AI tree-sitter"), loaded per language from disk — never
    // trained here. Absent model ⇒ no AI judgments for that language (train_lint first).
    let mut ai_models: HashMap<&'static str, Option<lint_moe::Moe>> = HashMap::new();
    for f in walk_repo(&root) {
        let Some(lang) = Lang::from_ext(&f.ext) else {
            continue;
        };
        // Skip languages the caller didn't select (None ⇒ all languages).
        if let Some(langs) = &sel.langs {
            if !langs.contains(lang.id()) {
                continue;
            }
        }
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
        if sel.cs {
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
        if !sel.official {
            continue; // CS-only selection: skip the official-doc rule passes below.
        }
        // AI judgment: delegate the file to its language's trained MoE ("AI tree-sitter").
        // It reasons in 1-bit signal space and flags only what it is sure of via the
        // causation gate. Loaded from disk (trained once), never trained here; if no model
        // exists for this language, this is silently skipped.
        let model = ai_models
            .entry(lang.id())
            .or_insert_with(|| lint_moe::Moe::load(&lint_moe::Moe::model_path(lang.id())));
        if let Some(m) = model.as_ref() {
            for (line, rule) in m.judge_located(&content) {
                issues.push(Issue {
                    severity: Sev::Medium,
                    category: "official-rule",
                    file: f.rel.clone(),
                    line,
                    message: format!(
                        "violates `{}` (AI-judged from the official docs)",
                        m.rule_name(rule)
                    ),
                    suggestion: "rewrite to the rule's documented good form",
                });
            }
        }
        // Metric rules: exact measurements (too many args/branches/nesting), with
        // each rule's threshold read from its own docs. Precise — never fuzzy.
        let metrics = metric_models
            .entry(lang.id())
            .or_insert_with(|| lint_index::for_language(lang.id()).map(|idx| lint_metrics::Metrics::build(&idx)));
        if let Some(mm) = metrics.as_ref() {
            for hit in mm.scan(&lines) {
                issues.push(Issue {
                    severity: sev_from_label(&hit.severity),
                    category: "official-rule",
                    file: f.rel.clone(),
                    line: hit.line,
                    message: format!("exceeds official rule `{}` — {}", hit.rule_id, hit.detail),
                    suggestion: "reduce the measure below the rule's documented threshold",
                });
            }
        }
    }

    issues.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });

    // Fast-path note: if a packed lint index covers the dominant language's
    // toolchain, surface that its official rules are active. The built-in checks
    // above remain the fallback and run unconditionally. Both notes describe the
    // official-rule layer, so they are suppressed for a CS-only selection.
    let official_note = if sel.official {
        dominant_lang(&lang_counts).and_then(official_rules_note)
    } else {
        None
    };

    let mut out = render(&issues, max);
    if let Some(note) = official_note {
        out.push('\n');
        out.push_str(&note);
    }
    // Program → AI seam: hand the rules that need understanding to the AI reviewer.
    if sel.official {
        if let Some(note) = dominant_lang(&lang_counts).and_then(ai_review_note) {
            out.push('\n');
            out.push_str(&note);
        }
    }
    Ok(vec![text(out)])
}

/// The AI-reviewer hand-off: the official rules NOT covered by a deterministic
/// checker — the ones that need understanding (type/semantic/intent), not pattern
/// matching. This is the program→AI seam of the hybrid: the program decides what it
/// can precisely, then tells the AI reviewer exactly what's left to judge by reading
/// the code. `None` when every indexed rule is already deterministically covered.
fn ai_review_note(lang: &str) -> Option<String> {
    let idx = lint_index::for_language(lang)?;
    let covered: std::collections::HashSet<String> = lint_checkers::assemble(lang)
        .map(|b| b.checkers.into_iter().map(|c| c.rule).collect())
        .unwrap_or_default();
    let uncovered: Vec<&lint_index::Rule> = idx.rules.iter().filter(|r| !covered.contains(&r.id)).collect();
    if uncovered.is_empty() {
        return None;
    }
    let mut s = format!(
        "\n## Grounded AI-review pass — the \"unslop\" check ({lang} {})\n\
         The deterministic checks above are exact (zero false positives) but only decide *syntactic* \
         rules. The {} rules below need understanding — and the rule is: **verify against the docs, \
         never from memory.** Ground every judgment in `lint-index/{}.json` (these webscraped, \
         version-matched official rules for v{}) — that is how this stays current and never \
         hallucinates a deprecated API. As the reviewer, read the compacted repo and flag what's \
         wrong, deprecated-as-of-this-version, or against CS principles; emit each finding for a \
         fixing agent (file:line + why + the doc it violates):\n",
        idx.docs_version, uncovered.len(), idx.tool, idx.docs_version
    );
    for r in uncovered.iter().take(20) {
        let desc: String = r.description.chars().take(90).collect();
        s.push_str(&format!("- `{}` — {desc}\n", r.id));
    }
    if uncovered.len() > 20 {
        s.push_str(&format!(
            "- …and {} more (full version-matched set in `lint-index/{}.json` — the grounding source)\n",
            uncovered.len() - 20,
            idx.tool
        ));
    }
    Some(s)
}

/// The language with the most scanned files, if any — the one whose packed lint
/// index (if present) is most relevant to surface.
fn dominant_lang(counts: &HashMap<&'static str, usize>) -> Option<&'static str> {
    counts
        .iter()
        .max_by_key(|(lang, n)| (**n, **lang))
        .map(|(lang, _)| *lang)
}

/// Build the "official rules active" note from Helpers' **webscraped lint index**
/// for `lang` — our own always-current rule catalog, sourced directly from the
/// official docs and refreshed via `helpers lint-index pull`. Deliberately does
/// NOT run or depend on the project's toolchain: the index is the source of
/// truth, so linting works even where clippy/ruff/eslint aren't installed.
///
/// Discovery is data-driven — [`lint_index::for_language`] matches the project's
/// language against each packed index's own `language` field, so adding a tool
/// is just dropping in its `lint-index/<tool>.json`. `None` when none serves the
/// language (or its checksum doesn't verify).
fn official_rules_note(lang: &str) -> Option<String> {
    match lint_index::for_language(lang) {
        // A packed index serves this language → report the active official rules.
        Some(idx) => Some(format!(
            "\n_Official rules: {} {} rules active — webscraped from the official docs (v{}), \
             kept current via `helpers lint-index pull`; no local toolchain required. \
             These augment the built-in checks above._\n",
            idx.rule_count, idx.tool, idx.docs_version
        )),
        // No index for a language we just scanned → ask the agent to supply the
        // linter's official rules-docs page so the index self-expands (the new
        // source is crawled, packed, and submitted back for everyone).
        None => Some(format!(
            "\n_No official lint index for `{lang}` yet. As the agent (you are the reader): \
             scrape that linter's official rules-docs page (e.g. `scrape_webpage`), extract its \
             rules as JSON `[{{id, category, severity, description, source}}]`, then run \
             `node scripts/build-lint-index.mjs --add-rules <tool> {lang} <rules.json> --docs <url>` \
             to pack + share them. Built-in checks above still apply._\n"
        )),
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

/// True when the declaration at `idx` is documented.
///
/// Walks upward past blank lines and any annotations/attributes/decorators that
/// legitimately sit between a doc comment and the declaration — Rust
/// `#[must_use]` / `#[wasm_bindgen]` (including multi-line attributes), Java/JS
/// `@Override`, Python `@staticmethod` — then checks for a doc/comment line.
/// For Python it also accepts a docstring on the first line after the `def`
/// (the idiomatic placement). Skipping attributes is the fix for a common false
/// positive: an item is documented, but an attribute between the `///` and the
/// `fn` previously hid the doc comment from this check.
fn has_doc_above(lang: Lang, lines: &[&str], idx: usize) -> bool {
    if matches!(lang, Lang::Python) && python_has_docstring_below(lines, idx) {
        return true;
    }
    let mut i = idx;
    while i > 0 {
        let prev = lines[i - 1].trim();
        if prev.is_empty() || is_annotation_line(lang, prev) {
            i -= 1;
            continue;
        }
        // Rust multi-line attribute (`#[cfg(\n  …\n)]`): its closing line ends
        // with `]` but doesn't start with `#`; skip up to the `#[`/`#![` opener.
        if matches!(lang, Lang::Rust) && prev.ends_with(']') && !prev.starts_with("//") {
            let mut k = i - 1;
            while k > 0 && !lines[k].trim_start().starts_with('#') {
                k -= 1;
            }
            if lines.get(k).is_some_and(|l| l.trim_start().starts_with('#')) {
                i = k;
                continue;
            }
        }
        return is_doc_line(lang, prev);
    }
    false
}

/// True when `line` is an annotation/attribute/decorator that may separate a
/// doc comment from the declaration it documents (and so must be skipped).
fn is_annotation_line(lang: Lang, line: &str) -> bool {
    match lang {
        Lang::Rust => line.starts_with("#[") || line.starts_with("#!["),
        // Java/C# annotations and JS/TS decorators, e.g. `@Override`, `@Component`.
        Lang::JavaLike | Lang::Js | Lang::Python => line.starts_with('@'),
        Lang::Go => false,
    }
}

/// True when `line` opens a documentation/comment for `lang`.
fn is_doc_line(lang: Lang, line: &str) -> bool {
    if matches!(lang, Lang::Python) {
        return line.starts_with('#') || line.starts_with("\"\"\"") || line.starts_with("'''");
    }
    line.starts_with("//")      // //, ///, //!
        || line.starts_with("/*") // /* or /**
        || line.starts_with('*')  // continuation line inside a block comment
        || line.ends_with("*/") // closing line of a block comment
}

/// True when the first non-blank line after a Python `def` opens a docstring —
/// the idiomatic place Python documents a function, which lives *inside* the
/// body rather than above the declaration.
fn python_has_docstring_below(lines: &[&str], idx: usize) -> bool {
    // A signature can span lines until the `:`; find the line that ends it.
    let mut j = idx;
    while j < lines.len() && !lines[j].trim_end().ends_with(':') {
        if j - idx > 8 {
            return false; // pathological signature; give up rather than misread
        }
        j += 1;
    }
    for line in lines.iter().skip(j + 1) {
        let t = line.trim_start();
        if t.is_empty() {
            continue;
        }
        return t.starts_with("\"\"\"")
            || t.starts_with("'''")
            || t.starts_with("r\"\"\"")
            || t.starts_with("r'''");
    }
    false
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
            Lang::Js | Lang::JavaLike if empty_catch.is_match(raw) => {
                out.push(Issue {
                    severity: Sev::High,
                    category: "cs-principle",
                    file: rel.to_string(),
                    line: idx + 1,
                    message: "Empty catch block swallows errors silently.".into(),
                    suggestion: "Handle, log, or rethrow the error — never swallow it.",
                });
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
    s.push_str("_Fix high first. Each line is a concrete, deterministic violation; re-run `lint` to watch the count drop. Pair with `helpers grade` for the rubric._\n\n");

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

/// MCP schema for the unified `lint` tool (supersedes the former cs_lint + strict_lint).
pub fn schema() -> Value {
    json!({
        "name": "lint",
        "description": "Lint the project and return one clean, prioritized list (severity, file:line, message, fix). Combines deterministic CS2420/CS3500 software-principle checks (single responsibility, documentation gaps, error handling, maintainability) with the official rules from Helpers' webscraped, always-current lint index (clippy/ruff/eslint/staticcheck), sourced directly from the official docs and refreshed via `helpers lint-index pull`. No local toolchain required — works even where clippy/ruff/eslint aren't installed. Deterministic. Pair with `helpers grade` for the rubric; re-run to track the count down.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "root": { "type": "string", "description": "Project root. Defaults to the current workspace." },
                "max": { "type": "integer", "description": "Max issues to list (1-500). Default 80." },
                "modules": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Selectable lint modules. Two axes: KIND — `cs` (CS2420/CS3500 principle checks) and/or `official` (webscraped official-doc rules); and LANGUAGE — `rust`, `go`, `js`/`ts`, `python`, `java`. Listing only languages runs both kinds for them; listing a kind restricts to it; `all` or omitted runs everything. Example: [\"cs\",\"rust\"] = principle checks on Rust only."
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
    fn doc_above_skips_attributes_and_decorators() {
        // Rust: `///` doc separated from `pub fn` by attributes (the reported
        // false positive) must still count as documented.
        let rust = vec![
            "/// Adds two numbers.",
            "#[must_use]",
            "#[wasm_bindgen(js_name = add)]",
            "pub fn add(a: i32, b: i32) -> i32 { a + b }",
        ];
        assert!(has_doc_above(Lang::Rust, &rust, 3));

        // Rust: multi-line attribute between doc and fn.
        let rust_multiline = vec![
            "/// Builds it.",
            "#[cfg(",
            "    feature = \"x\"",
            ")]",
            "pub fn build() {}",
        ];
        assert!(has_doc_above(Lang::Rust, &rust_multiline, 4));

        // Rust: genuinely undocumented (only an attribute, no doc) stays flagged.
        let undocumented = vec!["#[must_use]", "pub fn lonely() {}"];
        assert!(!has_doc_above(Lang::Rust, &undocumented, 1));

        // Python: docstring below the `def` is documentation.
        let py = vec!["def greet(name):", "    \"\"\"Greet someone.\"\"\"", "    pass"];
        assert!(has_doc_above(Lang::Python, &py, 0));

        // Python: decorator between comment and def.
        let py_decorated = vec!["# helper", "@staticmethod", "def util():", "    return 1"];
        assert!(has_doc_above(Lang::Python, &py_decorated, 2));
    }

    #[test]
    fn rust_attribute_only_function_is_flagged_as_doc_gap() {
        let lines = vec!["#[no_mangle]", "pub fn entry() {}"];
        let mut out = Vec::new();
        scan_file("src/lib.rs", Lang::Rust, &lines, false, false, false, &mut out);
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
    fn selection_defaults_to_everything() {
        let s = parse_selection(&json!({}));
        assert!(s.cs && s.official && s.langs.is_none());
        // Explicit `all` is the same as omitting.
        let s = parse_selection(&json!({ "modules": ["all"] }));
        assert!(s.cs && s.official && s.langs.is_none());
        // Empty list degrades to everything, not "nothing".
        let s = parse_selection(&json!({ "modules": [] }));
        assert!(s.cs && s.official && s.langs.is_none());
    }

    #[test]
    fn selection_by_kind_and_language() {
        // A kind alone restricts to that kind, all languages.
        let s = parse_selection(&json!({ "modules": ["cs"] }));
        assert!(s.cs && !s.official && s.langs.is_none());
        // A language alone keeps BOTH kinds, restricted to that language.
        let s = parse_selection(&json!({ "modules": ["rust"] }));
        assert!(s.cs && s.official);
        assert_eq!(s.langs.as_ref().unwrap().len(), 1);
        assert!(s.langs.as_ref().unwrap().contains("rust"));
        // Kind + language together: just that kind, just that language. Aliases fold.
        let s = parse_selection(&json!({ "modules": ["official", "ts", "py"] }));
        assert!(!s.cs && s.official);
        let langs = s.langs.unwrap();
        assert!(langs.contains("js") && langs.contains("python"));
        // Unknown tokens are ignored (no language restriction emerges from a typo).
        let s = parse_selection(&json!({ "modules": ["cs", "cobol"] }));
        assert!(s.cs && !s.official && s.langs.is_none());
    }

    #[test]
    fn dominant_lang_picks_most_files() {
        let mut counts: HashMap<&'static str, usize> = HashMap::new();
        counts.insert("rust", 5);
        counts.insert("python", 2);
        assert_eq!(dominant_lang(&counts), Some("rust"));
        assert_eq!(dominant_lang(&HashMap::new()), None);
    }

}
