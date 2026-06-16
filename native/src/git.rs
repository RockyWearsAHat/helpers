//! Shared git + workspace helpers — the Rust counterpart of `lib/mcp-git.js`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// `~/.cache/gsh/worktrees` — base dir for branch-session worktrees.
pub fn worktree_base() -> PathBuf {
    home().join(".cache").join("gsh").join("worktrees")
}

/// Resolve the user's home directory the same way the JS helpers do.
pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// Run `git <args>` in `cwd`. Returns trimmed stdout on success, or trimmed
/// stderr (falling back to a generic message) on failure — mirrors `execGit`.
pub fn exec_git(args: &[&str], cwd: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("git {} failed", args.join(" "))
        } else {
            stderr
        })
    }
}

/// Workspace roots from `$GSH_WORKSPACE_ROOTS` (a JSON array), or an empty vec
/// so the caller can apply its own fallback (cwd, etc.).
pub fn workspace_roots() -> Vec<PathBuf> {
    if let Ok(raw) = std::env::var("GSH_WORKSPACE_ROOTS") {
        if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str(&raw) {
            let roots: Vec<PathBuf> = arr
                .iter()
                .filter_map(|v| v.as_str())
                .map(PathBuf::from)
                .collect();
            if !roots.is_empty() {
                return roots;
            }
        }
    }
    Vec::new()
}

/// Walk up from `start` looking for a `.git` or `.github` entry. Mirrors
/// `findRepoRoot` — returns the first matching ancestor, or `None`.
pub fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() || current.join(".github").exists() {
            return Some(current);
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent.to_path_buf(),
            _ => return None,
        }
    }
}

/// The workspace root used by the session/knowledge tools. Mirrors the detection
/// in `mcp-research.js`: first `$GSH_WORKSPACE_ROOTS` entry, else the repo root
/// containing cwd, else the GSH install dir (this binary's parent).
pub fn workspace_root() -> PathBuf {
    if let Some(first) = workspace_roots().into_iter().next() {
        return first;
    }
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(root) = find_repo_root(&cwd) {
            return root;
        }
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Resolve the repository root: the first `$GSH_WORKSPACE_ROOTS` entry / cwd
/// whose `git rev-parse --show-toplevel` succeeds. Mirrors `resolveRepoRoot`.
pub fn resolve_repo_root() -> Result<PathBuf, String> {
    let mut candidates = workspace_roots();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.to_path_buf());
        }
    }
    for dir in &candidates {
        if let Ok(top) = exec_git(&["rev-parse", "--show-toplevel"], dir) {
            return Ok(PathBuf::from(top));
        }
    }
    Err("Not inside a git repository.".into())
}
