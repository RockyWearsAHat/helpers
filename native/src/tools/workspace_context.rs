//! `workspace_context` — port of `lib/mcp-workspace-context.js`.
//!
//! Reports each workspace git repo (root, branch, worktree?, remote, status)
//! plus any active branch-session worktrees under `~/.cache/gsh/worktrees`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::git::{exec_git, workspace_roots, worktree_base};
use crate::proto::{text, ToolResult};

pub fn schema() -> Value {
    json!({
        "name": "workspace_context",
        "description": "Return the current workspace context: workspace root folders, the active git branch in each, whether each is a worktree, and the remote URL. Call this at the start of a session or before making cross-branch operations to understand which branch and repository you are working in.",
        "inputSchema": { "type": "object", "properties": {}, "required": [] }
    })
}

struct RepoInfo {
    root: String,
    branch: String,
    is_worktree: bool,
    remote: String,
    status: String,
}

struct WorktreeInfo {
    path: String,
    branch: String,
    status: String,
}

pub fn run(_args: &Value) -> ToolResult {
    let mut roots = workspace_roots();
    if roots.is_empty() {
        if let Ok(cwd) = std::env::current_dir() {
            roots.push(cwd);
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                roots.push(dir.to_path_buf());
            }
        }
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut results: Vec<RepoInfo> = Vec::new();
    for root in &roots {
        let toplevel = match exec_git(&["rev-parse", "--show-toplevel"], root) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if !seen.insert(toplevel.clone()) {
            continue;
        }
        let tp = Path::new(&toplevel);
        let branch = exec_git(&["symbolic-ref", "--short", "HEAD"], tp)
            .unwrap_or_else(|_| "(detached HEAD)".to_string());
        let is_worktree = exec_git(&["rev-parse", "--git-dir"], tp)
            .map(|d| d.contains(".git/worktrees"))
            .unwrap_or(false);
        let remote = exec_git(&["remote", "get-url", "origin"], tp).unwrap_or_default();
        let status = exec_git(&["status", "--short", "--branch"], tp).unwrap_or_default();
        results.push(RepoInfo {
            root: toplevel,
            branch,
            is_worktree,
            remote,
            status,
        });
    }

    if results.is_empty() {
        return Ok(vec![text("No git repositories found in workspace.")]);
    }

    let lines: Vec<String> = results
        .iter()
        .map(|r| {
            let mut parts = vec![format!("Root: {}", r.root), format!("Branch: {}", r.branch)];
            if r.is_worktree {
                parts.push("Worktree: yes".to_string());
            }
            if !r.remote.is_empty() {
                parts.push(format!("Remote: {}", r.remote));
            }
            if !r.status.is_empty() {
                parts.push(format!("Status:\n{}", r.status));
            }
            parts.join("\n")
        })
        .collect();

    let active = scan_active_worktrees();

    let mut output = lines.join("\n\n---\n\n");
    if !active.is_empty() {
        output.push_str("\n\n=== Active Branch Sessions ===\n\n");
        let blocks: Vec<String> = active
            .iter()
            .map(|wt| {
                let mut parts = vec![
                    format!("Path: {}", wt.path),
                    format!("Branch: {}", wt.branch),
                ];
                if !wt.status.is_empty() {
                    parts.push(format!("Status:\n{}", wt.status));
                }
                parts.join("\n")
            })
            .collect();
        output.push_str(&blocks.join("\n\n---\n\n"));
    }

    Ok(vec![text(output)])
}

fn scan_active_worktrees() -> Vec<WorktreeInfo> {
    let base: PathBuf = worktree_base();
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&base) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let branch = match exec_git(&["symbolic-ref", "--short", "HEAD"], &path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let status = exec_git(&["status", "--short", "--branch"], &path).unwrap_or_default();
        out.push(WorktreeInfo {
            path: path.to_string_lossy().to_string(),
            branch,
            status,
        });
    }
    out
}
