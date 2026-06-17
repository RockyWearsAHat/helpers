//! `checkpoint` — stage changes, commit (with a provided or deterministic
//! message), and optionally push. Fully deterministic: no AI, no model calls.
//!
//! When no message is given it builds one from the staged diff stat (changed
//! files + insertion/deletion counts), so an agent can checkpoint repeatedly
//! without writing a message or burning tokens on generation.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::git::{exec_git, exec_git_stdin, home, resolve_repo_root};
use crate::proto::{text, ToolResult};

/// MCP schema for the `checkpoint` tool.
pub fn schema() -> Value {
    json!({
        "name": "checkpoint",
        "description": "Create a local git commit, optionally pushing. By default stages everything (git add -A), but you can checkpoint a precise subset: pass `paths` to stage only specific files, or `lines` to stage only specific line ranges within files (so a focused checkpoint never sweeps in unrelated edits). Commits with your message — or, if you omit one, a deterministic message derived from the staged diff. No AI. Cheaper and less error-prone than composing raw git add/commit yourself.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "message": { "type": "string", "description": "Commit message. If omitted, a deterministic message is built from the staged diff stat." },
                "all": { "type": "boolean", "description": "Stage all changes including untracked files (git add -A) before committing. Default: true, but ignored when `paths` or `lines` is given." },
                "paths": { "type": "array", "items": { "type": "string" }, "description": "Stage only these file paths (relative to the repo root) instead of everything. Overrides `all`." },
                "lines": { "type": "array", "description": "Stage only specific line ranges within files (line-level checkpoint). Each item is { \"file\": \"path\", \"ranges\": [[start,end], …] } with 1-based, inclusive line numbers in the current file. Only diff hunks touching those lines are staged. Overrides `all`.", "items": { "type": "object", "properties": { "file": { "type": "string" }, "ranges": { "type": "array", "items": { "type": "array", "items": { "type": "integer" } } } }, "required": ["file", "ranges"] } },
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

/// Stage the requested subset (lines/paths/all), commit, and optionally push.
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

    // ── Stage: precise subset (lines > paths) or everything ─────────────────
    let lines_spec = args.get("lines").and_then(Value::as_array);
    let paths_spec = args.get("paths").and_then(Value::as_array);
    if let Some(specs) = lines_spec.filter(|a| !a.is_empty()) {
        stage_lines(&cwd, specs)?;
    } else if let Some(paths) = paths_spec.filter(|a| !a.is_empty()) {
        let files: Vec<String> = paths
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        let mut add_args = vec!["add", "--"];
        add_args.extend(files.iter().map(String::as_str));
        exec_git(&add_args, &cwd).map_err(|e| format!("git add failed: {e}"))?;
    } else if args.get("all").and_then(Value::as_bool) != Some(false) {
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

// ─── Line-level staging (stage only hunks touching requested ranges) ─────────

/// Stage only the diff hunks that touch the requested line ranges. For each
/// `{ file, ranges }` spec it diffs the file, keeps hunks whose new-side range
/// intersects a requested range, and pipes the filtered patch to
/// `git apply --cached`. Inclusive, 1-based line numbers in the current file.
fn stage_lines(cwd: &Path, specs: &[Value]) -> Result<(), String> {
    for spec in specs {
        let file = spec
            .get("file")
            .and_then(Value::as_str)
            .ok_or("checkpoint: each `lines` entry needs a string `file`")?;
        let ranges = parse_ranges(spec.get("ranges"))?;
        if ranges.is_empty() {
            continue;
        }
        // Zero context so each change is its own hunk and can be isolated by line.
        let diff = exec_git(&["diff", "-U0", "--", file], cwd)
            .map_err(|e| format!("git diff failed for {file}: {e}"))?;
        if diff.trim().is_empty() {
            continue; // nothing unstaged here
        }
        let patch = filter_hunks(&diff, &ranges);
        if patch.is_none() {
            continue; // no hunk overlapped the requested lines
        }
        exec_git_stdin(
            &["apply", "--cached", "--recount", "--unidiff-zero"],
            cwd,
            &patch.unwrap(),
        )
        .map_err(|e| format!("staging lines for {file} failed: {e}"))?;
    }
    Ok(())
}

/// Parse `[[start,end], …]` into inclusive (start,end) pairs.
fn parse_ranges(v: Option<&Value>) -> Result<Vec<(usize, usize)>, String> {
    let arr = v
        .and_then(Value::as_array)
        .ok_or("checkpoint: `ranges` must be an array of [start,end] pairs")?;
    let mut out = Vec::new();
    for pair in arr {
        let nums = pair
            .as_array()
            .ok_or("checkpoint: each range must be a [start,end] array")?;
        let start = nums.first().and_then(Value::as_u64).unwrap_or(0) as usize;
        let end = nums.get(1).and_then(Value::as_u64).unwrap_or(start as u64) as usize;
        if start >= 1 && end >= start {
            out.push((start, end));
        }
    }
    Ok(out)
}

/// Keep the file header plus the hunks whose new-side line span intersects any
/// requested range; returns `None` when nothing matches.
fn filter_hunks(diff: &str, ranges: &[(usize, usize)]) -> Option<String> {
    let mut header = String::new();
    let mut kept: Vec<String> = Vec::new();
    let mut current: Option<(usize, usize, String)> = None; // (new_start, new_count, text)
    let mut seen_hunk = false;

    let flush = |cur: Option<(usize, usize, String)>, kept: &mut Vec<String>| {
        if let Some((start, count, text)) = cur {
            let hunk_end = start + count.saturating_sub(1);
            let hit = ranges
                .iter()
                .any(|&(a, b)| start <= b && a <= hunk_end.max(start));
            if hit {
                kept.push(text);
            }
        }
    };

    for line in diff.lines() {
        if line.starts_with("@@") {
            flush(current.take(), &mut kept);
            seen_hunk = true;
            let (new_start, new_count) = parse_hunk_header(line);
            current = Some((new_start, new_count, format!("{line}\n")));
        } else if seen_hunk {
            if let Some((_, _, text)) = current.as_mut() {
                text.push_str(line);
                text.push('\n');
            }
        } else {
            header.push_str(line);
            header.push('\n');
        }
    }
    flush(current.take(), &mut kept);

    if kept.is_empty() {
        return None;
    }
    Some(format!("{header}{}", kept.concat()))
}

/// Parse the new-side `+start,count` from a `@@ -a,b +c,d @@` hunk header.
fn parse_hunk_header(line: &str) -> (usize, usize) {
    // Find the "+c,d" token.
    for tok in line.split_whitespace() {
        if let Some(rest) = tok.strip_prefix('+') {
            let mut it = rest.split(',');
            let start = it.next().and_then(|s| s.parse().ok()).unwrap_or(1);
            let count = it.next().and_then(|s| s.parse().ok()).unwrap_or(1);
            return (start, count);
        }
    }
    (1, 1)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hunk_header_reads_new_side() {
        assert_eq!(parse_hunk_header("@@ -10,3 +12,5 @@ fn x()"), (12, 5));
        assert_eq!(parse_hunk_header("@@ -1 +1 @@"), (1, 1)); // single-line, no count
    }

    #[test]
    fn filter_hunks_keeps_only_overlapping_ranges() {
        // Zero-context diff: two separate hunks at new lines 1 and 5.
        let diff = "diff --git a/f b/f\n--- a/f\n+++ b/f\n\
                    @@ -1 +1 @@\n-line1\n+CHANGED1\n\
                    @@ -5 +5 @@\n-line5\n+CHANGED5\n";
        // Requesting line 1 keeps only the first hunk.
        let patch = filter_hunks(diff, &[(1, 1)]).expect("a hunk should match");
        assert!(patch.contains("CHANGED1"));
        assert!(!patch.contains("CHANGED5"));
        // Requesting a line no hunk touches yields nothing.
        assert!(filter_hunks(diff, &[(3, 3)]).is_none());
    }
}
