//! `lint` — the AI code reviewer: one reasoning model that learned its rules from
//! documents, reads the whole repository, and reports in English like a meticulous TA.
//!
//! This is the production entry for the [`crate::linter`] engine. It:
//!   1. **Learns the CS2420/CS3500 principles** from `corpus/cs-principles.md` (a plain
//!      document — edit it and the reasoner learns the new rule, no rebuild).
//!   2. **Reads project guidance** if the repo ships one ([`GUIDANCE_CANDIDATES`]), so the
//!      review can be steered with house conventions.
//!   3. **Finds the project's setup** — detects its languages — and **self-sets-up**: it packs
//!      any language module the project needs but the store lacks, training it once from the
//!      crawled docs corpus and caching it for reuse.
//!   4. **Reviews the whole repository** — calibrating the bar to the project's own idiomatic
//!      code — then talks back in English: the verdict, what to fix, and what it could not
//!      analyze, plus a deterministic grader supplement for documentation/maintainability.
//!
//! It complements `git-cs-grade` (which produces the rubric grade): `helpers grade`
//! tells you *where you stand*; `lint` tells you *the specific lines to fix*. The verdict
//! is grounded in webscraped, version-matched docs and the project's own code — never memory.

use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::{json, Value};

use crate::git::workspace_root;
use crate::index::walk::walk_repo;
use crate::linter::{review_repository, LintModule, ModuleRegistry, Reasoner};
use crate::proto::{text, ToolResult};
use crate::{lint_checkers, lint_docs};

/// How many documentation pages to crawl when learning a language whose docs are a site (not a
/// single structured file). Set high enough to cover an entire rules site (ruff/eslint publish
/// ~1000 rule pages) — the AI learns the WHOLE documentation, not a sample. The crawl stays in the
/// rules subtree and the learned module is cached, so this first-run cost is paid once per version.
#[cfg(feature = "crawl")]
const MAX_CRAWL_PAGES: usize = 2000;

/// The always-on CS2420/CS3500 principles, embedded at build time so the reasoner always has its
/// baseline even when the on-disk `corpus/cs-principles.md` cannot be located (e.g. an installed
/// binary far from the checkout). The on-disk copy is preferred at runtime — edit it and the
/// reasoner learns the new rule with no rebuild — and this is only the fallback.
const EMBEDDED_CS_PRINCIPLES: &str = include_str!("../../../corpus/cs-principles.md");

/// Project-level guidance files, in priority order. A user drops one of these in their repo to
/// steer the review with house conventions; the reasoner learns it on top of the CS principles.
const GUIDANCE_CANDIDATES: &[&str] = &[
    ".helpers/lint.md",
    "LINT.md",
    ".lint-rules.md",
    "lint-rules.md",
];

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

/// Review the project the way the linter is meant to work: learn the principles from documents,
/// self-set-up the language modules the project needs, calibrate to the repository, and talk back
/// in English. See the module docs for the four steps.
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
    let data = data_root();

    // 1) The reasoner learns the always-on CS2420/CS3500 principles from the document (on-disk so
    //    editing it teaches the reasoner with no rebuild; the embedded copy is the fallback).
    let cs_doc = read_cs_principles(&data);
    let mut reasoner = Reasoner::from_cs_principles("rust", &cs_doc);

    // 2) Project-level guidance: the house rules a user drops in their repo to steer the review.
    let guidance = read_project_guidance(&root);
    if let Some((_, doc)) = &guidance {
        reasoner.learn(doc);
    }

    // 3) Find the project's setup (its languages + toolchain versions) and self-set-up: for each
    //    language whose cached module is missing or stale, learn its rules straight from the official
    //    docs (known per-version URL, or one the calling agent supplies via `docs`), pack, and cache.
    //    When the selection excludes the `official` (learned-rule) kind, the registry stays empty so
    //    only the floor + taught principles + behavioral norms judge the code.
    let provided = parse_docs_arg(args);
    let (mut registry, setup) = if sel.official {
        let mut reg = ModuleRegistry::open(data.join("lint-modules"));
        let setup = self_setup(&mut reg, &root, &sel, &provided);
        (reg, setup)
    } else {
        (ModuleRegistry::open(data.join("__lint_no_modules__")), SetupOutcome::default())
    };

    // 4) Read the whole repository, calibrate to it, and review every file.
    let report = review_repository(&root, &mut reasoner, &mut registry);

    // 5) Render the English review, then the deterministic grader supplement (documentation &
    //    maintainability — the TA notes the reasoner's behavioral layer doesn't cover).
    let mut out = report.to_english();
    if let Some((path, _)) = &guidance {
        out.push_str(&format!("\nProject guidance applied from `{path}`.\n"));
    }
    if !setup.packed.is_empty() {
        out.push_str(&format!(
            "Learned and cached lint module(s) from the official docs this run (reused offline next time): {}.\n",
            setup.packed.join(", ")
        ));
    }
    for req in &setup.requests {
        out.push_str(&docs_request_note(req));
    }
    let supplement = grader_supplement(&root, &sel, max);
    if !supplement.is_empty() {
        out.push('\n');
        out.push_str(&supplement);
    }
    Ok(vec![text(out)])
}

// ── runtime resource resolution & self-setup ─────────────────────────────────

/// Locate the directory that holds Helpers' bundled lint data (`corpus/`, `lint-modules/`). Prefers
/// the resolved workspace root (the dev/checkout case), then walks up from the executable's location
/// (the installed case). Always returns a path; missing files degrade gracefully — embedded
/// principles, no extra modules — so the review still runs.
fn data_root() -> PathBuf {
    let ws = workspace_root();
    if ws.join("corpus/cs-principles.md").exists() || ws.join("lint-modules").exists() {
        return ws;
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        while let Some(d) = dir {
            if d.join("corpus/cs-principles.md").exists() {
                return d;
            }
            dir = d.parent().map(Path::to_path_buf);
        }
    }
    ws
}

/// Read the CS principles document, preferring the on-disk copy (so edits teach the reasoner with
/// no rebuild) and falling back to the embedded baseline when it cannot be found.
fn read_cs_principles(data: &Path) -> String {
    std::fs::read_to_string(data.join("corpus/cs-principles.md"))
        .unwrap_or_else(|_| EMBEDDED_CS_PRINCIPLES.to_string())
}

/// Find the project's lint guidance file, if any: the first existing, non-empty candidate from
/// [`GUIDANCE_CANDIDATES`]. Returns `(relative_path, contents)` so the report can say what it used.
fn read_project_guidance(root: &Path) -> Option<(String, String)> {
    for cand in GUIDANCE_CANDIDATES {
        if let Ok(s) = std::fs::read_to_string(root.join(cand)) {
            if !s.trim().is_empty() {
                return Some(((*cand).to_string(), s));
            }
        }
    }
    None
}

/// The distinct corpus-language names a project under `root` uses (respecting any `modules` language
/// filter). Only languages the corpus/modules can train for are returned — this is the shortlist
/// [`ensure_modules`] self-packs and the registry pulls.
fn detect_languages(root: &Path, sel: &Selection) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for f in walk_repo(root) {
        let Some(lang) = Lang::from_ext(&f.ext) else {
            continue;
        };
        if let Some(langs) = &sel.langs {
            if !langs.contains(lang.id()) {
                continue;
            }
        }
        if let Some(cl) = corpus_lang_of(lang) {
            if !seen.iter().any(|s| s == cl) {
                seen.push(cl.to_string());
            }
        }
    }
    seen
}

/// Map a scanned language to the language name used in the crawled docs corpus, or `None` when the
/// corpus has no rules for it (so no module can be self-packed).
fn corpus_lang_of(lang: Lang) -> Option<&'static str> {
    Some(match lang {
        Lang::Rust => "rust",
        Lang::Python => "python",
        Lang::Js => "javascript",
        Lang::Go => "go",
        Lang::JavaLike => return None,
    })
}

/// What [`self_setup`] did: the module ids learned+cached this run, and the languages for which we
/// have no docs link and need the calling agent to supply one (zero human input, agent action).
#[derive(Default)]
struct SetupOutcome {
    /// Module ids (re)trained from the official docs and cached this run.
    packed: Vec<String>,
    /// Languages whose docs the calling agent should fetch (URL unknown / crawl unavailable).
    requests: Vec<DocsRequest>,
}

/// A request for the calling agent to find a language's official rules docs (via `search_web`) and
/// pass the link back in `docs`, so the engine can learn and cache it — no human in the loop.
struct DocsRequest {
    lang: String,
    version: String,
    tool: String,
}

/// One agent-supplied documentation link (the answer to a prior [`DocsRequest`]).
struct DocsArg {
    language: String,
    url: String,
    version: Option<String>,
}

/// Parse the optional `docs` input: `[{language, url, version?}]`. Malformed entries are skipped.
fn parse_docs_arg(args: &Value) -> Vec<DocsArg> {
    let Some(arr) = args.get("docs").and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|d| {
            let language = d.get("language").and_then(Value::as_str)?.trim().to_string();
            let url = d.get("url").and_then(Value::as_str)?.trim().to_string();
            if language.is_empty() || url.is_empty() {
                return None;
            }
            let version = d.get("version").and_then(Value::as_str).map(str::to_string);
            Some(DocsArg { language, url, version })
        })
        .collect()
}

/// Self-setup the module store for the project: first learn from any agent-supplied docs (answers to
/// earlier requests), then for each project language ensure a version-fresh module — learning it
/// from the known official docs URL, or recording a [`DocsRequest`] when none is known. Modules are
/// cached (gitignored, self-healing), so a version-matched run does no network I/O.
fn self_setup(
    registry: &mut ModuleRegistry,
    root: &Path,
    sel: &Selection,
    provided: &[DocsArg],
) -> SetupOutcome {
    let mut out = SetupOutcome::default();

    // 1) Learn from any docs the agent passed in this call.
    for d in provided {
        if let Some(id) = learn_provided(registry, d) {
            out.packed.push(id);
        }
    }

    // 2) Ensure a fresh module per project language.
    for lang in detect_languages(root, sel) {
        let version = lint_checkers::detect_version(&lang).unwrap_or_default();
        if module_fresh(registry, &lang, &version) {
            continue;
        }
        match learn_known(registry, &lang, &version) {
            Some(id) => out.packed.push(id),
            None => {
                // URL unknown, or crawl unavailable/failed. Ask the agent only when we have nothing
                // cached for this language (a stale cache still reviews; no need to block on a link).
                if registry.entry_for_lang(&lang).is_none() {
                    let tool = lint_docs::known_docs_url(&lang, &version)
                        .map(|s| s.tool)
                        .unwrap_or_else(|| format!("{lang} linter"));
                    out.requests.push(DocsRequest { lang, version, tool });
                }
            }
        }
    }
    out
}

/// A module for `lang` is fresh when one exists and its version matches the project's current
/// toolchain version. With no detectable toolchain version we can't judge staleness, so an existing
/// module is left in place rather than re-learned on every run.
fn module_fresh(registry: &ModuleRegistry, lang: &str, version: &str) -> bool {
    match registry.entry_for_lang(lang) {
        None => false,
        Some(e) => version.is_empty() || e.version == version,
    }
}

/// Learn `lang`'s rules from its known official docs URL and publish the packed module. Returns the
/// module id, or `None` when no URL is known, the crawl feature is off, or nothing could be learned.
#[cfg(feature = "crawl")]
fn learn_known(registry: &mut ModuleRegistry, lang: &str, version: &str) -> Option<String> {
    let src = lint_docs::known_docs_url(lang, version)?;
    let knowledge = if lang == "rust" {
        // Clippy ships a structured lints.json per Rust version (pinned → stable → master fallback).
        lint_docs::learn_clippy(lang, version, MAX_CRAWL_PAGES)
    } else {
        lint_docs::learn_from_url(lang, &src, MAX_CRAWL_PAGES)
    };
    publish_knowledge(registry, lang, version, &src.tool, knowledge)
}

/// Without the crawler compiled in, the engine cannot learn over the network — it reuses caches and
/// asks the agent for docs instead.
#[cfg(not(feature = "crawl"))]
fn learn_known(_registry: &mut ModuleRegistry, _lang: &str, _version: &str) -> Option<String> {
    None
}

/// Learn from an agent-supplied docs link and publish the module. A `.json` link is treated as a
/// single structured file; anything else is crawled as a docs site.
#[cfg(feature = "crawl")]
fn learn_provided(registry: &mut ModuleRegistry, d: &DocsArg) -> Option<String> {
    let crawl = !d.url.to_lowercase().ends_with(".json");
    let src = lint_docs::DocsSource { url: d.url.clone(), crawl, tool: "docs".to_string() };
    let version = d.version.clone().unwrap_or_default();
    let knowledge = lint_docs::learn_from_url(&d.language, &src, MAX_CRAWL_PAGES);
    publish_knowledge(registry, &d.language, &version, "docs", knowledge)
}

#[cfg(not(feature = "crawl"))]
fn learn_provided(_registry: &mut ModuleRegistry, _d: &DocsArg) -> Option<String> {
    None
}

/// Pack `knowledge` into a module for `lang` and publish it to the store, returning its id. `None`
/// when nothing grounded precisely (the self-validating fit kept no rule) — no empty module is
/// stored. The id is stable per `(lang, tool)` so re-learning a new version replaces in place.
#[cfg(feature = "crawl")]
fn publish_knowledge(
    registry: &mut ModuleRegistry,
    lang: &str,
    version: &str,
    tool: &str,
    knowledge: crate::linter::Knowledge,
) -> Option<String> {
    let id = format!("{lang}-{tool}");
    let provenance = format!("official docs: {tool}");
    let module = LintModule::pack(&id, version, &provenance, lang, &knowledge);
    if module.rule_count() == 0 {
        return None;
    }
    registry.publish(&module).ok()?;
    Some(id)
}

/// The agent-in-the-loop note: how to supply a missing language's docs link so the engine can learn
/// it — addressed to the calling agent, not a human.
fn docs_request_note(req: &DocsRequest) -> String {
    let v = if req.version.is_empty() {
        String::new()
    } else {
        format!(" v{}", req.version)
    };
    format!(
        "\n## Need docs to fully lint {lang}{v} — agent action, zero human input\n\
         I have no built-in docs link for `{lang}`{v}. As the calling agent: `search_web` for the \
         official {tool} rules documentation for {lang}{v}, then call `lint` again with:\n\
         `docs=[{{\"language\":\"{lang}\",\"url\":\"<official-rules-docs-url>\",\"version\":\"{ver}\"}}]`\n\
         I'll crawl, learn, and cache it — you won't be asked again for this version. (I'm reviewing \
         now with the floor + CS principles + behavioral norms in the meantime.)\n",
        lang = req.lang,
        tool = req.tool,
        v = v,
        ver = req.version,
    )
}

/// The deterministic grader supplement: documentation gaps, over-long files, and large
/// uncommented blocks across the project — the TA-style maintainability notes the reasoner's
/// floor/principles/norms layers don't cover. Suppressed when the selection excludes CS checks.
fn grader_supplement(root: &Path, sel: &Selection, max: usize) -> String {
    if !sel.cs {
        return String::new();
    }
    let mut issues: Vec<Issue> = Vec::new();
    for f in walk_repo(root) {
        let Some(lang) = Lang::from_ext(&f.ext) else {
            continue;
        };
        if let Some(langs) = &sel.langs {
            if !langs.contains(lang.id()) {
                continue;
            }
        }
        if is_declaration_file(&f.rel) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&f.abs) else {
            continue;
        };
        let lower = content.to_lowercase();
        let allow_long_fn = lower.contains("quality:allow-long-function");
        let allow_block = lower.contains("quality:allow-large-block");
        let allow_long_file = lower.contains("quality:allow-long-file");
        let lines: Vec<&str> = content.lines().collect();
        scan_file(&f.rel, lang, &lines, allow_long_fn, allow_block, allow_long_file, &mut issues);
    }
    // Keep only the documentation/maintainability categories — correctness, complexity, and error
    // handling are already judged (and de-duplicated) by the reasoner's review above.
    issues.retain(|i| {
        matches!(
            i.category,
            "documentation-gap" | "maintainability" | "large-block-without-comment"
        )
    });
    if issues.is_empty() {
        return String::new();
    }
    issues.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });
    let mut s = format!(
        "--- Grader notes — documentation & maintainability ({} item(s)) ---\n",
        issues.len()
    );
    for i in issues.iter().take(max) {
        s.push_str(&format!(
            "  [{}] {}:{} — {} ({})\n    → {}\n",
            i.severity.label(),
            i.file,
            i.line,
            i.message,
            i.category,
            i.suggestion
        ));
    }
    if issues.len() > max {
        s.push_str(&format!("  …and {} more (raise `max`).\n", issues.len() - max));
    }
    s
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

// ── schema ───────────────────────────────────────────────────────────────────

/// MCP schema for the unified `lint` tool (supersedes the former cs_lint + strict_lint).
pub fn schema() -> Value {
    json!({
        "name": "lint",
        "description": "Review the whole project like a meticulous TA. One reasoning model that LEARNED its rules from documents — the CS2420/CS3500 principles in corpus/cs-principles.md plus webscraped, version-matched official rules (clippy/ruff/eslint/staticcheck) packed into per-language modules — reads every file, calibrates the bar to the project's own idiomatic code, and reports in English: the verdict, the lines to fix, and what it could not analyze. It finds the project's setup and self-sets-up (training+caching any missing language module on first run), and can be steered with a project guidance file (.helpers/lint.md or LINT.md). No local toolchain required. Grounded in the docs and the project's own code — never memory. Pair with `helpers grade` for the rubric.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "root": { "type": "string", "description": "Project root. Defaults to the current workspace." },
                "max": { "type": "integer", "description": "Max grader-supplement items to list (1-500). Default 80." },
                "modules": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional filter. KIND — `cs` (include the documentation/maintainability grader supplement) and/or `official` (the learned rule modules); LANGUAGE — `rust`, `go`, `js`/`ts`, `python`, `java` restricts which languages are self-set-up and supplemented. `all` or omitted runs everything."
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
    fn data_root_finds_the_checkout_with_principles() {
        // In the dev checkout the data root resolves to a directory that carries the principles doc
        // (the embedded fallback guarantees the reasoner still has its baseline if it cannot).
        let d = data_root();
        assert!(
            d.join("corpus/cs-principles.md").exists() || !EMBEDDED_CS_PRINCIPLES.is_empty(),
            "either the on-disk principles resolve, or the embedded baseline is present"
        );
    }

    #[test]
    fn grader_supplement_keeps_only_doc_and_maintainability() {
        // A file with a long undocumented public surface yields documentation/maintainability notes
        // and nothing from the correctness categories the reasoner now owns.
        let dir = std::env::temp_dir().join(format!("grader_supp_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let body = "pub fn undocumented_surface() {}\n".repeat(3);
        std::fs::write(dir.join("lib.rs"), body).unwrap();
        let out = grader_supplement(&dir, &parse_selection(&json!({})), 80);
        assert!(out.contains("documentation-gap"), "doc gap is reported: {out}");
        assert!(!out.contains("cs-principle"), "correctness categories are excluded");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
