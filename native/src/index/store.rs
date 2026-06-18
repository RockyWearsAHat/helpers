//! On-disk layout for the index under `<workspace>/.gsh/index/`.

use std::path::{Path, PathBuf};

use crate::index::model::ProjectIndex;

/// `<root>/.gsh/index` — where the machine index and `.dx` docs live.
pub fn index_dir(root: &Path) -> PathBuf {
    root.join(".gsh").join("index")
}

/// Machine-readable index: `<root>/.gsh/index/graph.json`.
pub fn graph_path(root: &Path) -> PathBuf {
    index_dir(root).join("graph.json")
}

/// Persist the index as pretty JSON. Creates the directory if needed.
pub fn save(root: &Path, index: &ProjectIndex) -> std::io::Result<()> {
    std::fs::create_dir_all(index_dir(root))?;
    let json = serde_json::to_string_pretty(index).map_err(std::io::Error::other)?;
    std::fs::write(graph_path(root), json)
}

/// Load a previously-built index, if present and parseable.
pub fn load(root: &Path) -> Option<ProjectIndex> {
    let raw = std::fs::read_to_string(graph_path(root)).ok()?;
    serde_json::from_str(&raw).ok()
}
