//! Portable `.dxbundle` export/install — package a project's index so another
//! project can reference its map ("easy reference of other projects").
//!
//! A bundle is a single self-contained JSON file: the machine graph plus every
//! `.dx` doc. Installing it drops the bundle under the host project's
//! `.gsh/index/refs/<name>/`, where `project_map`/`lookup` can consult it.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::index::build::build_index;
use crate::index::dx::write_docs;
use crate::index::model::ProjectIndex;
use crate::index::store::{index_dir, load, save};

const BUNDLE_FORMAT: &str = "dxbundle/1";

#[derive(Serialize, Deserialize)]
pub struct Bundle {
    pub format: String,
    pub name: String,
    pub built_at: String,
    pub file_count: usize,
    pub symbol_count: usize,
    pub graph: ProjectIndex,
    /// index-dir-relative path (`map.dx`, `nodes/x.dx`) → contents.
    pub docs: BTreeMap<String, String>,
}

fn repo_name(root: &Path) -> String {
    root.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string())
}

/// Collect every `.dx` doc under the index dir into a map.
fn collect_docs(dir: &Path) -> BTreeMap<String, String> {
    let mut docs = BTreeMap::new();
    if let Ok(content) = std::fs::read_to_string(dir.join("map.dx")) {
        docs.insert("map.dx".to_string(), content);
    }
    let nodes = dir.join("nodes");
    if let Ok(entries) = std::fs::read_dir(&nodes) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("dx") {
                if let (Some(name), Ok(content)) = (
                    p.file_name().and_then(|s| s.to_str()),
                    std::fs::read_to_string(&p),
                ) {
                    docs.insert(format!("nodes/{name}"), content);
                }
            }
        }
    }
    docs
}

/// Export the project's index (building it fresh if absent) to a `.dxbundle`.
pub fn export_bundle(root: &Path, out: &Path) -> Result<Bundle, String> {
    let graph = match load(root) {
        Some(g) => g,
        None => {
            let g = build_index(root);
            save(root, &g).map_err(|e| e.to_string())?;
            write_docs(root, &g).map_err(|e| e.to_string())?;
            g
        }
    };
    let docs = collect_docs(&index_dir(root));
    let bundle = Bundle {
        format: BUNDLE_FORMAT.to_string(),
        name: repo_name(root),
        built_at: graph.built_at.clone(),
        file_count: graph.file_count,
        symbol_count: graph.symbol_count,
        graph,
        docs,
    };
    let json = serde_json::to_string(&bundle).map_err(|e| e.to_string())?;
    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(out, json).map_err(|e| e.to_string())?;
    Ok(bundle)
}

/// Install a `.dxbundle` into `root/.gsh/index/refs/<name>/`. Returns the
/// installed reference name.
pub fn install_bundle(root: &Path, bundle_path: &Path) -> Result<String, String> {
    let raw = std::fs::read_to_string(bundle_path)
        .map_err(|e| format!("cannot read bundle {}: {e}", bundle_path.display()))?;
    let bundle: Bundle =
        serde_json::from_str(&raw).map_err(|e| format!("invalid .dxbundle: {e}"))?;
    if bundle.format != BUNDLE_FORMAT {
        return Err(format!("unsupported bundle format: {}", bundle.format));
    }
    let dest = index_dir(root).join("refs").join(&bundle.name);
    std::fs::create_dir_all(dest.join("nodes")).map_err(|e| e.to_string())?;
    let graph_json = serde_json::to_string_pretty(&bundle.graph).map_err(|e| e.to_string())?;
    std::fs::write(dest.join("graph.json"), graph_json).map_err(|e| e.to_string())?;
    for (rel, content) in &bundle.docs {
        let p = dest.join(rel);
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(p, content).map_err(|e| e.to_string())?;
    }
    Ok(bundle.name)
}

/// Names of installed reference indexes under `.gsh/index/refs/`.
pub fn list_refs(root: &Path) -> Vec<String> {
    let refs_dir = index_dir(root).join("refs");
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&refs_dir) {
        for e in entries.flatten() {
            if e.path().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    names.push(name.to_string());
                }
            }
        }
    }
    names.sort();
    names
}
