//! `checkpoint` — port of `lib/mcp-checkpoint.js`. Stage changes, generate (or
//! accept) a commit message, commit, and optionally push. AI message generation
//! shells out to the Copilot CLI exactly as the JS version did.
//!
//! The VS Code branch-commit notification is intentionally NOT done here — the
//! Node daemon re-emits it after a successful native checkpoint (see
//! git-shell-helpers-mcp.js), keeping the extension integration without coupling
//! this binary to the editor IPC socket.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

use crate::git::{exec_git, home, resolve_repo_root};
use crate::model_utils::{detect_cheap_model, load_available_models, resolve_model_id};
use crate::proto::{text, ToolResult};

pub fn schema() -> Value {
    json!({
        "name": "checkpoint",
        "description": "Create a local git commit with an AI-generated message. Stages changes, generates a commit message from the diff, commits, and optionally pushes. Pass context for extra AI hints. Optionally override with a manual message.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "context": { "type": "string", "description": "Optional extra context to include in the AI prompt (e.g. 'fixes the login bug introduced in last PR'). Ignored when message is provided." },
                "message": { "type": "string", "description": "Optional manual override for the commit message. If omitted, the message is AI-generated from the staged diff." },
                "all": { "type": "boolean", "description": "Stage all changes including untracked files (git add -A) before committing. Default: true." },
                "push": { "type": "boolean", "description": "Push to remote after committing. Default: false." },
                "force": { "type": "boolean", "description": "Override a mid-session disable. Only use this when the user explicitly asked for a checkpoint and the previous call returned [no-op]. Never set force on automatic checkpoints." },
                "cwd": { "type": "string", "description": "Absolute path to the git repository to commit in. Auto-detected from the workspace root when omitted. Pass explicitly when working in a multi-root workspace, a git worktree, or when the target repo differs from the server's working directory." },
                "branch": { "type": "string", "description": "Assert that HEAD is on this branch before committing. If the current branch does not match, the commit is aborted with an error." },
                "model": { "type": "string", "description": "Model to use for AI commit message generation. Accepts a model id, display name, or shorthand (e.g. 'haiku'). When omitted, the cheapest available model is selected automatically." }
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

/// Run git, returning trimmed stdout or empty string on any failure (the JS
/// `gitOut` helper used for read-only queries feeding the AI prompt).
fn git_out(args: &[&str], cwd: &Path) -> String {
    exec_git(args, cwd).unwrap_or_default()
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

    // ── Commit message ───────────────────────────────────────────────────────
    let message = {
        let m = str_arg(args, "message");
        if !m.is_empty() {
            m.to_string()
        } else {
            generate_ai_commit_message(&cwd, str_arg(args, "context"), str_arg(args, "model"))?
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

// ─── AI commit message generation ───────────────────────────────────────────

fn generate_ai_commit_message(
    cwd: &Path,
    extra_context: &str,
    preferred_model: &str,
) -> Result<String, String> {
    let changed_files = git_out(&["diff", "--cached", "--name-only"], cwd);
    let stat_summary = git_out(&["diff", "--cached", "--stat"], cwd);
    let mut actual_diff = git_out(&["diff", "--cached", "--unified=3"], cwd);
    let diff_lines = actual_diff.lines().count();
    if diff_lines > 2000 {
        let head: Vec<&str> = actual_diff.lines().take(2000).collect();
        actual_diff = format!(
            "{}\n(truncated — showing first 2000 of {diff_lines} lines)",
            head.join("\n")
        );
    }
    let recent_history = git_out(&["log", "--oneline", "-10", "--no-decorate"], cwd);
    let detailed_recent = git_out(
        &["log", "-3", "--pretty=format:--- %h (%ar) ---%n%s%n%n%b"],
        cwd,
    );

    let mut repo_guidance = String::new();
    for file in [
        ".github/COMMIT_GUIDELINES.md",
        ".github/commit_guidelines.md",
        ".github/COMMIT_MESSAGE.md",
        ".github/commit_message.md",
        ".github/copilot-instructions.md",
        "AGENTS.md",
        "CLAUDE.md",
        "CONTRIBUTING.md",
    ] {
        if let Ok(raw) = std::fs::read_to_string(cwd.join(file)) {
            let snippet: Vec<&str> = raw.lines().take(120).collect();
            let snippet = snippet.join("\n");
            let snippet = snippet.trim();
            if !snippet.is_empty() {
                repo_guidance.push_str(&format!("\n--- {file} ---\n{snippet}\n"));
            }
        }
    }

    let prompt = "You are a commit message generator. Your ONLY job is to describe\nthe staged diff below. You have no knowledge of any other project, conversation,\nor task. Every word in your message must come from what you see in the diff and\nthe commit history of THIS repository. Do not infer, hallucinate, or borrow\ncontext from outside this prompt.\n\nThis is a CHECKPOINT commit — the developer is marking a meaningful moment:\nsomething works now that did not before, a logical unit of work is complete,\nor they are about to switch context. Frame the message accordingly.\n\nCONTEXT MATTERS MOST:\nRead the recent commit history below. Each commit is part of an ongoing thread.\nFrame yours as the next step in that story. If the diff and the history look\nunrelated to each other, trust the diff — it is the ground truth.\n\nSUBJECT LINE:\nOne line, <= 72 chars. Say what the commit DOES or FIXES, not what\nfiles it touches.\n\nBODY:\nDescribe the situation, what you did, and why. Someone reading git blame\nshould understand the reasoning without opening the diff.\n\nDo NOT use section headers like 'What changed:', 'Why this matters:', etc.\nFor a tiny fix: one sentence or no body. For a real change: a short paragraph.\n\nNever anthropomorphize code. Never restate the subject in different words.\n\nOUTPUT FORMAT — output ONLY the commit between these markers:\nCOMMIT_BEGIN\n<commit message>\nCOMMIT_END\n";

    let mut full_prompt = format!(
        "{prompt}\nRECENT COMMIT HISTORY:\n{recent_history}\n\nDETAILED RECENT COMMITS (last 3):\n{detailed_recent}"
    );
    if !repo_guidance.is_empty() {
        full_prompt.push_str(&format!("\n\nREPOSITORY GUIDANCE:{repo_guidance}"));
    }
    let files_block = changed_files
        .lines()
        .map(|f| format!("  - {f}"))
        .collect::<Vec<_>>()
        .join("\n");
    let stat_last = stat_summary.lines().last().unwrap_or("");
    full_prompt.push_str(&format!(
        "\n\n---\n\nChanged files:\n{files_block}\n\nGit stat: {stat_last}\n\nDIFF ({diff_lines} lines):\n```diff\n{actual_diff}\n```"
    ));
    if !extra_context.is_empty() {
        full_prompt.push_str(&format!(
            "\n\nAdditional context from developer: {extra_context}"
        ));
    }

    // Resolve the AI command (explicit override, or a Copilot invocation).
    let ai_cmd = match std::env::var("GIT_UPLOAD_AI_CMD") {
        Ok(c) if !c.is_empty() => c,
        _ => {
            let models = load_available_models();
            let model_id = if !preferred_model.is_empty() {
                resolve_model_id(preferred_model, &models)
                    .unwrap_or_else(|| preferred_model.to_string())
            } else {
                detect_cheap_model(&models)
            };
            format!("copilot -s --model {model_id} --deny-tool write --deny-tool shell -p \"$GIT_UPLOAD_AI_PROMPT\"")
        }
    };

    let output = Command::new("sh")
        .arg("-c")
        .arg(&ai_cmd)
        .env("GIT_UPLOAD_AI_PROMPT", &full_prompt)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("AI message generation failed to start: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() || stdout.trim().is_empty() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "AI message generation failed (exit {}): {}",
            output.status.code().unwrap_or(-1),
            err.trim()
        ));
    }

    let mut capturing = false;
    let mut lines = Vec::new();
    for line in stdout.lines() {
        if line.trim() == "COMMIT_BEGIN" {
            capturing = true;
            continue;
        }
        if line.trim() == "COMMIT_END" {
            break;
        }
        if capturing {
            lines.push(line);
        }
    }
    let message = lines.join("\n");
    let message = message.trim();
    if message.is_empty() {
        return Err(
            "Could not parse AI output. Use the message parameter to provide a manual message."
                .to_string(),
        );
    }
    Ok(message.to_string())
}
