//! MCP tools for managing the linter's language and doc sources.
//!
//! **Adding a new language — 3-step workflow for an agent:**
//!
//! 1. Call `lint_languages` — see what's already supported and what's missing.
//! 2. If the language has built-in docs knowledge (listed by `lint_languages`), call
//!    `lint_learn language="<lang>"` — it crawls, compiles, and saves the model. Done.
//! 3. If it is a custom or lesser-known language, call `lint_add_source` first to register
//!    its docs URL, then `lint_learn`, then `lint_submit` to open a PR.
//!
//! Tools:
//! * `lint_languages` — full status: docs-knowledge, grammar, and trained state per language.
//! * `lint_add_source` — register a custom language's docs URL in `sources.json`.
//! * `lint_learn`      — force-crawl a language's docs now and save a committed model.
//! * `lint_submit`     — commit trained models + corpus, push, and open a GitHub PR.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::proto::{text, ToolResult};

// ── lint_languages ────────────────────────────────────────────────────────────

/// Every language the linter knows about, with grammar support and docs-knowledge status.
/// Grammar = can parse and match tree patterns. Docs = built-in URL knowledge or custom entry.
/// Trained = a compiled model file exists on disk right now.
static BUILTIN_LANGUAGES: &[(&str, bool, &str, &str)] = &[
    // (name, grammar_support, docs_tool, docs_url_hint)
    ("rust",       true,  "clippy",          "docs.rust-lang.org/clippy"),
    ("python",     true,  "ruff",            "docs.astral.sh/ruff/rules"),
    ("javascript", true,  "eslint",          "eslint.org/docs/latest/rules"),
    ("typescript", true,  "typescript-eslint","typescript-eslint.io/rules"),
    ("go",         true,  "staticcheck",     "staticcheck.dev/docs/checks"),
    ("java",       true,  "pmd",             "pmd.github.io"),
    ("ruby",       true,  "rubocop",         "docs.rubocop.org"),
    ("c",          true,  "clang-tidy",      "clang.llvm.org/extra/clang-tidy"),
    ("bash",       true,  "shellcheck",      "shellcheck.net/wiki/Checks"),
    ("cpp",        false, "clang-tidy",      "clang.llvm.org/extra/clang-tidy"),
    ("kotlin",     false, "detekt",          "detekt.dev/docs/rules"),
    ("swift",      false, "swiftlint",       "realm.github.io/SwiftLint"),
    ("php",        false, "phpstan",         "phpstan.org"),
    ("csharp",     false, "roslyn",          "learn.microsoft.com/dotnet/csharp/roslyn"),
];

/// Show every language the linter knows — docs URL, grammar support, and trained status —
/// so an agent immediately knows what to call next.
pub fn run_languages(args: &Value) -> ToolResult {
    let data_root = crate::tools::lint::data_root_pub();
    let models_dir = data_root.join("lint-models");

    // Check which models are actually trained on disk.
    let trained: BTreeMap<String, bool> = BUILTIN_LANGUAGES.iter().map(|(lang, _, _, _)| {
        let path = models_dir.join(format!("{lang}.learned.json"));
        (lang.to_string(), path.exists())
    }).collect();

    // Load custom sources from sources.json.
    let sources_path = data_root.join("lint-index/sources.json");
    let custom_langs: Vec<String> = std::fs::read_to_string(&sources_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v["sources"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|e| e["language"].as_str().map(str::to_string))
        .filter(|l| !BUILTIN_LANGUAGES.iter().any(|(b, _, _, _)| *b == l))
        .collect();

    let filter = args.get("language").and_then(Value::as_str).map(str::to_ascii_lowercase);

    let mut out = String::from("Language support status:\n");
    out.push_str(&format!("  {:<14} {:<8} {:<18} {:<8} {}\n", "LANGUAGE", "GRAMMAR", "DOCS TOOL", "TRAINED", "ACTION"));
    out.push_str(&format!("  {}\n", "-".repeat(72)));

    let mut can_learn: Vec<&str> = Vec::new();
    let mut needs_grammar: Vec<&str> = Vec::new();

    for (lang, grammar, tool, hint) in BUILTIN_LANGUAGES {
        if let Some(ref f) = filter { if f != lang { continue; } }
        let is_trained = trained.get(*lang).copied().unwrap_or(false);
        let action = if is_trained {
            "ready"
        } else if *grammar {
            can_learn.push(lang);
            "→ call lint_learn"
        } else {
            needs_grammar.push(lang);
            "→ needs grammar crate first"
        };
        out.push_str(&format!(
            "  {:<14} {:<8} {:<18} {:<8} {}  ({})\n",
            lang,
            if *grammar { "yes" } else { "no" },
            tool,
            if is_trained { "yes" } else { "no" },
            action,
            hint,
        ));
    }

    if !custom_langs.is_empty() {
        out.push_str("\nCustom languages (from sources.json):\n");
        for lang in &custom_langs {
            let is_trained = models_dir.join(format!("{lang}.learned.json")).exists();
            out.push_str(&format!(
                "  {:<14} trained={}\n",
                lang, if is_trained { "yes" } else { "no → call lint_learn" }
            ));
        }
    }

    if filter.is_none() {
        if !can_learn.is_empty() {
            out.push_str(&format!(
                "\nTo train all ready languages now:\n  call lint_learn once per language: {}\n",
                can_learn.join(", ")
            ));
        }
        if !needs_grammar.is_empty() {
            out.push_str(&format!(
                "\nLanguages needing a grammar crate before they can be trained: {}\n\
                 Add tree-sitter-<lang> to Cargo.toml and wire it into lint_match.rs + lint_ast.rs,\n\
                 then lint_learn will work for them too.\n",
                needs_grammar.join(", ")
            ));
        }
        out.push_str("\nFull workflow for a NEW language:\n\
            1. lint_languages                        — check if it's already known\n\
            2. lint_add_source language=X url=<docs> — register it (skip if already in list above)\n\
            3. lint_learn language=X                 — crawl docs, compile model, save to lint-models/\n\
            4. lint_submit description=\"add X\"       — commit, push, open PR\n");
    }

    Ok(vec![text(out)])
}

pub fn schema_languages() -> Value {
    json!({
        "name": "lint_languages",
        "description": "List every language the linter knows about, with grammar support (can parse files), \
                        built-in docs-URL knowledge, and trained status (model on disk). \
                        Start here when adding a new language — it tells you exactly what to do next. \
                        Full workflow: lint_languages → lint_learn (for built-ins) or lint_add_source → lint_learn → lint_submit.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "language": {
                    "type": "string",
                    "description": "Filter to one specific language. Omit to see all."
                }
            },
            "required": []
        }
    })
}

// ── lint_add_source ───────────────────────────────────────────────────────────

/// Register a language's official docs URL in `sources.json`. The linter crawls
/// the URL on first use (or when `lint_learn` is called) and learns the rules itself.
pub fn run_add_source(args: &Value) -> ToolResult {
    let lang = args["language"].as_str().ok_or("lint_add_source: `language` is required")?;
    let url = args["url"].as_str().ok_or("lint_add_source: `url` is required")?;
    let tool = args["tool"].as_str().unwrap_or(lang);
    let kind = args["kind"].as_str().unwrap_or("crawl");

    let data_root = crate::tools::lint::data_root_pub();
    let sources_path = data_root.join("lint-index/sources.json");

    let raw = std::fs::read_to_string(&sources_path)
        .unwrap_or_else(|_| r#"{"version":1,"sources":[]}"#.to_string());
    let mut cfg: Value =
        serde_json::from_str(&raw).map_err(|e| format!("sources.json parse error: {e}"))?;

    // Reject duplicates (same language+tool pair).
    if let Some(arr) = cfg["sources"].as_array() {
        if arr.iter().any(|e| e["language"].as_str() == Some(lang) && e["tool"].as_str() == Some(tool)) {
            return Ok(vec![text(format!(
                "`{tool}` for `{lang}` is already registered in sources.json.\n\
                 Run `lint_learn` with language=\"{lang}\" to force an immediate crawl."
            ))]);
        }
    }

    let entry = match kind {
        "agent" => json!({ "tool": tool, "language": lang, "kind": "agent", "docsBase": url }),
        _ => json!({ "tool": tool, "language": lang, "kind": "crawl", "seed": url, "docsBase": url }),
    };

    cfg["sources"]
        .as_array_mut()
        .ok_or("sources.json: missing `sources` array")?
        .push(entry);

    let out = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    if let Some(parent) = sources_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&sources_path, out).map_err(|e| format!("could not write sources.json: {e}"))?;

    Ok(vec![text(format!(
        "Registered `{tool}` for `{lang}` ({kind}).\nURL: {url}\n\n\
         Next: call lint_learn language=\"{lang}\" to crawl the docs and compile the model now.\n\
         Then lint_submit to share it via PR.\n\n\
         Note: to analyze `{lang}` source files, a tree-sitter grammar must exist in Cargo.toml \
         and be wired into lint_match.rs + lint_ast.rs. \
         Call lint_languages to see which languages already have grammar support."
    ))])
}

pub fn schema_add_source() -> Value {
    json!({
        "name": "lint_add_source",
        "description": "Register a CUSTOM or lesser-known language's docs URL in sources.json. \
                        NOT needed for built-in languages (rust, python, javascript, typescript, go, java, ruby, c, bash, \
                        cpp, kotlin, swift, php, csharp) — those already have docs-URL knowledge built in, just call lint_learn. \
                        Use lint_add_source only when lint_languages shows a language is missing entirely. \
                        After registering: call lint_learn to train, then lint_submit to PR it.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "language": {
                    "type": "string",
                    "description": "Language name (e.g. rust, python, javascript, typescript, go, java, ruby, bash, c, cpp)"
                },
                "url": {
                    "type": "string",
                    "description": "URL of the linter's rules index page (e.g. https://docs.rubocop.org/rubocop/cops.html)"
                },
                "tool": {
                    "type": "string",
                    "description": "Linter tool name (e.g. rubocop, checkstyle, shellcheck). Defaults to the language name."
                },
                "kind": {
                    "type": "string",
                    "enum": ["crawl", "agent"],
                    "description": "`crawl` (default) follows links from the seed URL. `agent` uses the URL as a docs base for the AI reader."
                }
            },
            "required": ["language", "url"]
        }
    })
}

// ── lint_learn ────────────────────────────────────────────────────────────────

/// Force-train the linter for one language: crawl its registered docs now, compile the
/// pattern model, and save it as `lint-models/<lang>.learned.json` (a committed module).
pub fn run_learn(args: &Value) -> ToolResult {
    let lang = args["language"].as_str().ok_or("lint_learn: `language` is required")?;
    let data_root = crate::tools::lint::data_root_pub();
    match crate::lint_train::learn_and_commit(lang, &data_root) {
        Ok(r) => Ok(vec![text(format!(
            "Trained `{}`: {} rules from docs → {} compiled patterns.\n\
             Module: {}\n\n\
             Run `lint_submit` to share this with others via a PR.",
            r.lang, r.rule_count, r.pattern_count, r.module_path.display()
        ))]),
        Err(e) => Err(format!("lint_learn failed for `{lang}`: {e}")),
    }
}

pub fn schema_learn() -> Value {
    json!({
        "name": "lint_learn",
        "description": "Force-train the linter for a language right now: crawls its registered docs URL (from sources.json), \
                        compiles the tree-pattern model, and saves the result as lint-models/<lang>.learned.json — a committed \
                        module that git pull ships to everyone. Use lint_add_source first if the language is not registered yet.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "language": {
                    "type": "string",
                    "description": "Language to train (must be registered in lint-index/sources.json)"
                }
            },
            "required": ["language"]
        }
    })
}

// ── lint_submit ───────────────────────────────────────────────────────────────

/// Commit newly-trained models + corpus changes and open a GitHub PR so others get them.
pub fn run_submit(args: &Value) -> ToolResult {
    let desc = args["description"]
        .as_str()
        .unwrap_or("Add trained language models from official docs");
    let data_root = crate::tools::lint::data_root_pub();
    let repo_root = crate::git::workspace_root();

    let paths: Vec<std::path::PathBuf> = ["lint-models", "lint-index/sources.json", "corpus"]
        .iter()
        .map(|p| data_root.join(p))
        .filter(|p| p.exists())
        .collect();

    if paths.is_empty() {
        return Err("lint_submit: nothing to submit — no lint-models, sources, or corpus found".into());
    }

    let result = commit_and_pr(&repo_root, &paths, desc)?;
    Ok(vec![text(result)])
}

fn commit_and_pr(root: &std::path::Path, paths: &[std::path::PathBuf], desc: &str) -> Result<String, String> {
    use std::process::Command;

    // Stage the paths.
    let mut add = Command::new("git");
    add.current_dir(root).arg("add");
    for p in paths { add.arg(p); }
    let out = add.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("git add failed: {}", String::from_utf8_lossy(&out.stderr)));
    }

    // Check what was staged.
    let diff = Command::new("git")
        .current_dir(root)
        .args(["diff", "--cached", "--name-only"])
        .output().map_err(|e| e.to_string())?;
    let staged = String::from_utf8_lossy(&diff.stdout);
    if staged.trim().is_empty() {
        return Ok("Nothing new to submit — all models are already committed.".into());
    }

    // Commit.
    let msg = format!("feat(lint): {desc}");
    let commit = Command::new("git")
        .current_dir(root)
        .args(["commit", "-m", &msg])
        .output().map_err(|e| e.to_string())?;
    if !commit.status.success() {
        return Err(format!("git commit failed: {}", String::from_utf8_lossy(&commit.stderr)));
    }

    // Push (best-effort; continue even if remote is not set).
    let push = Command::new("git")
        .current_dir(root)
        .args(["push", "origin", "HEAD"])
        .output();
    let pushed = push.map(|o| o.status.success()).unwrap_or(false);

    // Open a PR if gh is available.
    let pr_url = if pushed {
        let pr = Command::new("gh")
            .current_dir(root)
            .args([
                "pr", "create",
                "--title", &format!("feat(lint): {desc}"),
                "--body",
                "Adds newly trained language model(s) crawled from official docs.\n\n\
                 Every rule has a bad/good example sourced from the official linter docs; \
                 patterns were compiled with the lossless tree-pattern engine.\n\n\
                 Reviewers: load this branch and run `helpers lint` on a project of the \
                 trained language to verify 0 false positives.\n\n\
                 Generated by `lint_submit`.",
            ])
            .output()
            .ok();
        pr.and_then(|o| if o.status.success() {
            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
        } else {
            None
        })
    } else {
        None
    };

    let mut msg = format!("Committed.\nFiles:\n{staged}");
    if pushed {
        msg.push_str("Pushed to origin.\n");
    } else {
        msg.push_str("Could not push (no remote or no credentials) — push manually.\n");
    }
    if let Some(url) = pr_url {
        msg.push_str(&format!("PR: {url}\n"));
    } else if pushed {
        msg.push_str("(Install `gh` and authenticate to auto-open PRs.)\n");
    }
    Ok(msg)
}

pub fn schema_submit() -> Value {
    json!({
        "name": "lint_submit",
        "description": "Commit newly-trained lint-models and corpus/sources changes, push, and open a GitHub PR so others get the trained language models on git pull. Run lint_learn first to train a language.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "Short description for the commit message and PR title. Default: 'Add trained language models from official docs'."
                }
            },
            "required": []
        }
    })
}
