//! Project-index MCP tools — the centerpiece that replaces session memory.
//!
//! `index_project` builds the map; `project_map` returns a token-cheap overview
//! so the model orients in one call; `lookup` answers "where is X / what touches
//! X" from the graph instead of grep sweeps.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::git::workspace_root;
use crate::index::build::build_index;
use crate::index::dx::{project_map_dx, write_docs};
use crate::index::model::{Dir, ProjectIndex};
use crate::index::store::{graph_path, load, save};
use crate::proto::{text, ToolResult};

fn root_arg(args: &Value) -> PathBuf {
    match args.get("root").and_then(Value::as_str) {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => workspace_root(),
    }
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .to_string()
}

// ─── index_project ──────────────────────────────────────────────────────────

/// Build and persist the project index (graph + optional `.dx` docs) for `root`,
/// returning a summary of files/symbols/edges indexed and the top-ranked modules.
pub fn run_index(args: &Value) -> ToolResult {
    let root = root_arg(args);
    if !root.exists() {
        return Err(format!("index_project: path not found: {}", root.display()));
    }
    let index = build_index(&root);
    save(&root, &index).map_err(|e| format!("failed to write index: {e}"))?;
    let docs = if args.get("write_docs").and_then(Value::as_bool) == Some(false) {
        0
    } else {
        write_docs(&root, &index).map_err(|e| format!("failed to write .dx docs: {e}"))?
    };

    let top: Vec<String> = index
        .ranked()
        .into_iter()
        .take(8)
        .map(|f| f.path.clone())
        .collect();
    let mut lines = vec![format!(
        "Indexed {}: {} files, {} symbols, {} edges{}.",
        Path::new(&index.root)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default(),
        index.file_count,
        index.symbol_count,
        index.edges.len(),
        index
            .commit
            .as_ref()
            .map(|c| format!(" (commit {c})"))
            .unwrap_or_default()
    )];
    lines.push(format!(
        "Wrote {} + {} .dx doc(s) under .gsh/index/.",
        rel(&root, &graph_path(&root)),
        docs
    ));
    if !top.is_empty() {
        lines.push(format!("Top modules: {}", top.join(", ")));
    }
    Ok(vec![text(lines.join("\n"))])
}

// ─── project_map ────────────────────────────────────────────────────────────

/// Return the token-cheap ranked project overview, building the index on demand
/// if none is cached.
pub fn run_map(args: &Value) -> ToolResult {
    let root = root_arg(args);
    let index = load_or_build(&root)?;
    if index.file_count == 0 {
        return Ok(vec![text(
            "No indexable files found. Run index_project after adding source files.",
        )]);
    }
    Ok(vec![text(project_map_dx(&index))])
}

// ─── lookup ─────────────────────────────────────────────────────────────────

/// Resolve a symbol or file query from the index graph: returns matching symbol
/// definitions (with their referencing files) and file-path matches, ranked.
pub fn run_lookup(args: &Value) -> ToolResult {
    let query = args
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if query.is_empty() {
        return Err("lookup requires a non-empty query (a symbol or file name).".into());
    }
    let max = args
        .get("max_results")
        .and_then(Value::as_i64)
        .unwrap_or(10)
        .clamp(1, 50) as usize;

    let root = root_arg(args);
    let index = load_or_build(&root)?;
    let ql = query.to_lowercase();

    // Symbol definition matches, ranked by exact-name then file rank.
    let mut defs: Vec<(f64, bool, String)> = Vec::new();
    for (i, f) in index.files.iter().enumerate() {
        for d in &f.defs {
            let nl = d.name.to_lowercase();
            let exact = nl == ql;
            if exact || nl.contains(&ql) {
                let neigh = index.neighbors(i);
                let refs: Vec<String> = neigh
                    .iter()
                    .filter(|n| n.dir == Dir::In)
                    .take(4)
                    .map(|n| index.files[n.file].path.clone())
                    .collect();
                let refs_str = if refs.is_empty() {
                    String::new()
                } else {
                    format!("   ← referenced by {}", refs.join(", "))
                };
                defs.push((
                    f.rank,
                    exact,
                    format!(
                        "- `{}` ({}) — {}:{}{}",
                        d.name, d.kind, f.path, d.line, refs_str
                    ),
                ));
            }
        }
    }
    defs.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then(b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal))
    });

    // File path matches.
    let mut files: Vec<(f64, String)> = index
        .files
        .iter()
        .filter(|f| f.path.to_lowercase().contains(&ql))
        .map(|f| {
            (
                f.rank,
                format!("- {} ({}, {} LOC, rank {})", f.path, f.lang, f.loc, f.rank),
            )
        })
        .collect();
    files.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    if defs.is_empty() && files.is_empty() {
        return Ok(vec![text(format!(
            "No symbol or file matches for \"{query}\". Try index_project to refresh, or a different term."
        ))]);
    }

    let mut out = vec![format!(
        "Lookup \"{query}\": {} definition(s), {} file match(es).",
        defs.len(),
        files.len()
    )];
    if !defs.is_empty() {
        out.push(String::new());
        out.push("Definitions:".to_string());
        out.extend(defs.into_iter().take(max).map(|(_, _, s)| s));
    }
    if !files.is_empty() {
        out.push(String::new());
        out.push("Files:".to_string());
        out.extend(files.into_iter().take(max).map(|(_, s)| s));
    }
    Ok(vec![text(out.join("\n"))])
}

/// Load the cached index, building (and persisting) one if none exists.
fn load_or_build(root: &Path) -> Result<ProjectIndex, String> {
    if !root.exists() {
        return Err(format!("path not found: {}", root.display()));
    }
    if let Some(index) = load(root) {
        return Ok(index);
    }
    let index = build_index(root);
    let _ = save(root, &index);
    let _ = write_docs(root, &index);
    Ok(index)
}

// ─── Schemas ────────────────────────────────────────────────────────────────

/// MCP tool schema for `index_project`.
pub fn schema_index() -> Value {
    json!({
        "name": "index_project",
        "description": "Build (or refresh) the GSH project index: a cheap, static map of the repository's files, symbols (functions/classes/types), and the reference graph between them, ranked by importance. Writes .gsh/index/graph.json plus portable .dx documents (with embedded Mermaid graphs). Run this once per session or after structural changes so project_map and lookup stay current — it lets the model orient without expensive file exploration.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "root": { "type": "string", "description": "Optional path to the project root. Defaults to the current workspace." },
                "write_docs": { "type": "boolean", "description": "Write per-file .dx node docs in addition to graph.json. Default true." }
            },
            "required": []
        }
    })
}

/// MCP tool schema for `project_map`.
pub fn schema_map() -> Value {
    json!({
        "name": "project_map",
        "description": "Return a compact, token-cheap overview of the project: the most important modules (ranked) with their key symbols, plus a Mermaid graph of how files connect. Call this FIRST when starting work in an unfamiliar area instead of reading or grepping many files — it orients you in one call. Builds the index automatically if it does not exist yet.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "root": { "type": "string", "description": "Optional path to the project root. Defaults to the current workspace." }
            },
            "required": []
        }
    })
}

/// MCP tool schema for `lookup`.
pub fn schema_lookup() -> Value {
    json!({
        "name": "lookup",
        "description": "Find where a symbol is defined and what references it, or locate files by name — answered from the project index graph instead of a grep sweep. Returns file:line for matching definitions, the files that reference them, and matching file paths, ranked by importance. Much cheaper than reading files to find something.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "A symbol name (function/class/type) or a substring of a file path." },
                "max_results": { "type": "integer", "description": "Max matches per section (1-50). Default 10." },
                "root": { "type": "string", "description": "Optional path to the project root. Defaults to the current workspace." }
            },
            "required": ["query"]
        }
    })
}
