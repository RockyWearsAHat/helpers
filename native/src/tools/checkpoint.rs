//! `checkpoint` — stage changes, commit (with a provided or deterministic
//! message), and optionally push. Fully deterministic: no AI, no model calls.
//!
//! When no message is given it builds one from the staged diff stat (changed
//! files + insertion/deletion counts), so an agent can checkpoint repeatedly
//! without writing a message or burning tokens on generation.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::git::{exec_git, home, resolve_repo_root};
use crate::proto::{text, ToolResult};

pub fn schema() -> Value {
    json!({
        "name": "checkpoint",
        "description": "Create a local git commit, optionally pushing. Stages changes (git add -A by default), then commits with your message — or, if you omit one, a deterministic message derived from the staged diff (changed files + line counts). No AI is involved. Use this to checkpoint progress frequently and cheaply.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "message": { "type": "string", "description": "Commit message. If omitted, a deterministic message is built from the staged diff stat." },
                "all": { "type": "boolean", "description": "Stage all changes including untracked files (git add -A) before committing. Default: true." },
                "push": { "type": "boolean", "description": "Push to remote after committing. Default: false." },
                "force": { "type": "boolean", "description": "Override a mid-session disable. Only use this when the user explicitly asked for a checkpoint and the previous call returned [no-op]. Never set force on automatic checkpoints." },
                "cwd": { "type": "string", "description": "Absolute path to the git repository to commit in. Auto-detected from the workspace root when omitted. Pass explicitly for a multi-root workspace, a git worktree, or a target repo that differs from the server's working directory." },
                "branch": { "type": "string", "description": "Assert that HEAD is on this branch before committing. If the current branch does not match, the commit is aborted with an error." }
            },
            "required": []
        }
    })
}

fn str_arg<'a>(args: &'a Value, key: &str) -> &'a str {
    args.get(key).and_then(Value::as_str).unwrap_or("").trim()
}

fn worktree_path(branch: &str) -> PathBuf {
    let safe: String = branch
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect();
    home()
        .join(".cache")
        .join("gsh")
        .join("worktrees")
        .join(safe)
}

pub fn run(args: &Value) -> ToolResult {
    // ── Resolve the target repo ──────────────────────────────────────────────
    let cwd: PathBuf = if !str_arg(args, "cwd").is_empty() {
        let given = str_arg(args, "cwd");
        match exec_git(&["rev-parse", "--show-toplevel"], Path::new(given)) {
            Ok(top) => PathBuf::from(top),
            Err(_) => {
                return Err(format!(
                    "Specified cwd is not inside a git repository: {given}"
                ))
            }
        }
    } else if !str_arg(args, "branch").is_empty() {
        let branch = str_arg(args, "branch");
        let wt = worktree_path(branch);
        let from_wt = if wt.exists() {
            match exec_git(&["symbolic-ref", "--short", "HEAD"], &wt) {
                Ok(b) if b == branch => exec_git(&["rev-parse", "--show-toplevel"], &wt).ok(),
                _ => None,
            }
        } else {
            None
        };
        match from_wt {
            Some(top) => PathBuf::from(top),
            None => resolve_repo_root()?,
        }
    } else {
        resolve_repo_root()?
    };

    let current_branch = exec_git(&["symbolic-ref", "--short", "HEAD"], &cwd).unwrap_or_default();

    // ── Branch assertion ─────────────────────────────────────────────────────
    let branch_arg = str_arg(args, "branch");
    if !branch_arg.is_empty() {
        if current_branch.is_empty() {
            return Err(format!(
                "Branch assertion failed: HEAD is detached, expected branch '{branch_arg}'. Switch to the correct branch before checkpointing."
            ));
        }
        if current_branch != branch_arg {
            return Err(format!(
                "Branch assertion failed: HEAD is on '{current_branch}', expected '{branch_arg}'. Switch to the correct branch before checkpointing."
            ));
        }
    }

    // ── Per-repo enable switch ───────────────────────────────────────────────
    let enabled = exec_git(&["config", "--get", "checkpoint.enabled"], &cwd)
        .unwrap_or_else(|_| "true".to_string());
    if enabled == "false" {
        return Ok(vec![text(
            "[no-op] Checkpoint is disabled for this repo. Use `git checkpoint --enable` to turn on.",
        )]);
    }

    // ── Stage + detect changes ───────────────────────────────────────────────
    if args.get("all").and_then(Value::as_bool) != Some(false) {
        exec_git(&["add", "-A"], &cwd).map_err(|e| format!("git add failed: {e}"))?;
    }
    // `--quiet` exits 0 when there is nothing staged.
    if exec_git(&["diff", "--cached", "--quiet"], &cwd).is_ok() {
        return Ok(vec![text("Nothing to commit — working tree clean.")]);
    }

    // ── Commit message (provided, or deterministic from the staged diff) ─────
    let message = {
        let m = str_arg(args, "message");
        if !m.is_empty() {
            m.to_string()
        } else {
            deterministic_message(&cwd)
        }
    };

    let mut commit_args = vec!["commit", "-m", &message];
    let sign = exec_git(&["config", "--get", "checkpoint.sign"], &cwd).unwrap_or_default();
    if sign == "true" {
        commit_args.push("-S");
    }
    exec_git(&commit_args, &cwd).map_err(|e| format!("git commit failed: {e}"))?;

    let hash = exec_git(&["rev-parse", "--short", "HEAD"], &cwd).unwrap_or_default();
    let oneline = exec_git(&["log", "--oneline", "-1"], &cwd).unwrap_or_default();
    let stat = exec_git(&["show", "--stat", "--format=", "HEAD"], &cwd).unwrap_or_default();

    // ── Optional push ────────────────────────────────────────────────────────
    let push_default = exec_git(&["config", "--get", "checkpoint.push"], &cwd).unwrap_or_default();
    let mut push_result = String::new();
    if args.get("push").and_then(Value::as_bool) == Some(true) || push_default == "true" {
        match exec_git(&["push"], &cwd) {
            Ok(_) => push_result = "\nPushed to remote.".to_string(),
            Err(e) => push_result = format!("\nPush failed: {e}"),
        }
    }

    let branch_info = if current_branch.is_empty() {
        " (detached HEAD)".to_string()
    } else {
        format!(" on branch '{current_branch}'")
    };

    Ok(vec![text(format!(
        "Committed {hash}{branch_info}\n{oneline}\n\n{stat}{push_result}"
    ))])
}

// ─── Deterministic commit message ───────────────────────────────────────────

/// Build a commit message from the staged diff: a subject naming the changed
/// files and a body with the per-file stat. Fully deterministic, no AI.
fn deterministic_message(cwd: &Path) -> String {
    let files: Vec<String> = exec_git(&["diff", "--cached", "--name-only"], cwd)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    let stat = exec_git(&["diff", "--cached", "--stat"], cwd).unwrap_or_default();

    let subject = match files.len() {
        0 => "checkpoint".to_string(),
        1 => format!("checkpoint: update {}", files[0]),
        n => {
            let names: Vec<&str> = files
                .iter()
                .take(3)
                .map(|f| f.rsplit('/').next().unwrap_or(f))
                .collect();
            let more = if n > 3 {
                format!(", +{} more", n - 3)
            } else {
                String::new()
            };
            format!("checkpoint: update {n} files ({}{more})", names.join(", "))
        }
    };

    if stat.trim().is_empty() {
        subject
    } else {
        format!("{subject}\n\n{}", stat.trim())
    }
}
