//! Three MCP tools that manage the linter's language/doc sources:
//!
//! * `lint_add_source` — register a new language by pointing it at its official docs URL;
//!   the linter then learns from those docs automatically on next use.
//! * `lint_learn` — force-crawl a language's registered docs right now, compile the model,
//!   and save it as a committed module (`lint-models/<lang>.learned.json`) so `git pull`
//!   ships the learned rules to everyone without a per-machine crawl.
//! * `lint_submit` — stage the trained modules + corpus changes, commit, push, and open a
//!   GitHub PR so others get the improvements on their next pull.

use serde_json::{json, Value};

use crate::proto::{text, ToolResult};

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
        "Registered `{tool}` for `{lang}` ({kind}).\n\
         URL: {url}\n\n\
         The linter will crawl this URL and train automatically on the next `lint` run \
         (or immediately via `lint_learn` with language=\"{lang}\").\n\n\
         Note: to analyze `{lang}` files, a tree-sitter grammar crate must also be wired in \
         (Cargo.toml + lint_match.rs). Languages already wired: rust, python, javascript, typescript, go."
    ))])
}

pub fn schema_add_source() -> Value {
    json!({
        "name": "lint_add_source",
        "description": "Register a language's official docs URL so the linter knows where to learn its rules. \
                        The linter crawls the URL automatically on the next lint run, or immediately when you call lint_learn. \
                        This is the whole workflow for adding a new language: point it at its linter docs, train, submit as PR.",
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
