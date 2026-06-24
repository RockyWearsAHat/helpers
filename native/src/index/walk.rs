//! Gitignore-aware repository walk (built on ripgrep's `ignore` crate).

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

/// A file found during the walk.
pub struct WalkedFile {
    /// Repo-relative path with forward slashes.
    pub rel: String,
    pub abs: PathBuf,
    /// Lowercase extension (no dot), or empty.
    pub ext: String,
}

/// Directories we never index even when not gitignored (build output, vcs, our own index, and —
/// crucially — dependency trees, which are NOT the project's own code and would swamp a review with
/// thousands of third-party findings). Shared so every walker (index + review) prunes identically.
pub const SKIP_DIRS: &[&str] = &[
    ".git",
    ".helpers",
    // dependency trees (JS / Python / general)
    "node_modules",
    "vendor",
    ".venv",
    "venv",
    "env",
    ".env",
    "site-packages",
    "bower_components",
    "Pods",
    // build / generated output
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".svelte-kit",
    // caches / tooling
    "__pycache__",
    ".cache",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".gradle",
    "coverage",
    ".idea",
];

/// Skip files larger than this — they are almost never source worth indexing.
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// Walk `root`, honoring `.gitignore`, returning indexable files sorted by path.
pub fn walk_repo(root: &Path) -> Vec<WalkedFile> {
    let mut out = Vec::new();
    let walker = WalkBuilder::new(root)
        .hidden(false) // keep dotfiles like .github/*; SKIP_DIRS handles noise
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .parents(true)
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !SKIP_DIRS.contains(&name.as_ref())
        })
        .build();

    for entry in walker.flatten() {
        let ft = match entry.file_type() {
            Some(ft) => ft,
            None => continue,
        };
        if !ft.is_file() {
            continue;
        }
        if entry
            .metadata()
            .map(|m| m.len() > MAX_FILE_BYTES)
            .unwrap_or(false)
        {
            continue;
        }
        let abs = entry.path();
        let rel = abs
            .strip_prefix(root)
            .unwrap_or(abs)
            .to_string_lossy()
            .replace('\\', "/");
        let ext = abs
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        out.push(WalkedFile {
            rel,
            abs: abs.to_path_buf(),
            ext,
        });
    }
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    out
}
