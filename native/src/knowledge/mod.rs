//! Knowledge subsystem — port of `lib/mcp-knowledge-index.js` +
//! `lib/mcp-knowledge-rw.js`. Local TF-IDF index/search over markdown notes,
//! note CRUD, a keyword-search fallback, and the GitHub-hosted community index
//! (fetched via `curl` with ETag caching).

pub mod community;
pub mod index;
pub mod notes;

use std::path::{Path, PathBuf};

use crate::git::{home, workspace_root};

/// Raw-content base for the community knowledge repo.
pub const GITHUB_RAW_BASE: &str =
    "https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/dev";
/// Re-check the community index at most this often (10 minutes).
pub const INDEX_MAX_AGE_MS: i64 = 10 * 60 * 1000;

/// Resolved paths for the knowledge tools, mirroring `mcp-research.js`.
pub struct KnowledgeConfig {
    pub workspace_root: PathBuf,
    pub repo_root: PathBuf,
    pub knowledge_root: PathBuf,
    pub repo_knowledge_root: PathBuf,
    pub local_index_path: PathBuf,
    pub github_cache_dir: PathBuf,
    pub cache_meta_path: PathBuf,
}

impl KnowledgeConfig {
    /// Resolve all knowledge paths from the current workspace and install dir:
    /// the active knowledge root (workspace `knowledge/` if it holds markdown,
    /// else `.github/knowledge/`), the repo-bundled root, and the GitHub cache.
    pub fn resolve() -> Self {
        let workspace_root = workspace_root();
        // The GSH install dir is this binary's parent (REPO_ROOT in the JS).
        let repo_root = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| workspace_root.clone());

        // knowledge/ if it holds markdown, else .github/knowledge/.
        let at_root = workspace_root.join("knowledge");
        let knowledge_root = if dir_has_markdown(&at_root) {
            at_root
        } else {
            workspace_root.join(".github").join("knowledge")
        };

        let github_cache_dir = home().join(".cache").join("gsh");
        KnowledgeConfig {
            local_index_path: knowledge_root.join("_index.json"),
            repo_knowledge_root: repo_root.join("knowledge"),
            cache_meta_path: github_cache_dir.join("_cache_meta.json"),
            knowledge_root,
            repo_root,
            workspace_root,
            github_cache_dir,
        }
    }
}

fn dir_has_markdown(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                e.file_name()
                    .to_str()
                    .map(|n| n.ends_with(".md"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}
