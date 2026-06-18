//! `.dx` document generation — portable, agent- and human-readable index units
//! with Obsidian-style `[[links]]` and an embedded Mermaid graph of each node's
//! neighborhood (the "graphify" view, auto-derived from the code).

use std::collections::HashMap;
use std::path::Path;

use crate::index::model::{Dir, FileEntry, ProjectIndex};
use crate::index::store::index_dir;
use crate::util::now_iso;

/// How many top-ranked nodes to draw in the project-wide mermaid graph.
const MAP_GRAPH_NODES: usize = 24;
/// How many top-ranked files get their own `.dx` node doc.
const MAX_NODE_DOCS: usize = 80;

/// Repo basename for titles.
fn repo_name(index: &ProjectIndex) -> String {
    Path::new(&index.root)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string())
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Filesystem-safe slug for a repo-relative path (`src/core.rs` → `src-core-rs`).
pub fn slug(path: &str) -> String {
    let s: String = path
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    s.trim_matches('-').to_string()
}

/// Distinct, meaningful definition names for display (drops module-system
/// noise like `exports`/`module` and de-duplicates repeated names).
fn top_symbol_names(file: &FileEntry, n: usize) -> Vec<&str> {
    let mut seen = std::collections::HashSet::new();
    file.defs
        .iter()
        .map(|d| d.name.as_str())
        .filter(|name| !matches!(*name, "exports" | "module" | "require" | "default"))
        .filter(|name| seen.insert(*name))
        .take(n)
        .collect()
}

// ─── Project map ────────────────────────────────────────────────────────────

/// The root `map.dx`: a compact, token-cheap overview the model reads first.
pub fn project_map_dx(index: &ProjectIndex) -> String {
    let name = repo_name(index);
    let ranked = index.ranked();
    let mut out = String::new();

    out.push_str("---\n");
    out.push_str("id: __project_map__\n");
    out.push_str(&format!("title: Project Map — {name}\n"));
    out.push_str("kind: project-map\n");
    if let Some(commit) = &index.commit {
        out.push_str(&format!("commit: {commit}\n"));
    }
    out.push_str(&format!("generated_at: {}\n", index.built_at));
    out.push_str("---\n\n");

    out.push_str(&format!("# Project Map — {name}\n\n"));
    out.push_str(&format!(
        "{} files · {} symbols{}\n\n",
        index.file_count,
        index.symbol_count,
        index
            .commit
            .as_ref()
            .map(|c| format!(" · commit {c}"))
            .unwrap_or_default()
    ));

    out.push_str("## Top modules (by importance)\n\n");
    for (i, f) in ranked.iter().take(30).enumerate() {
        let syms = top_symbol_names(f, 4);
        let sym_str = if syms.is_empty() {
            String::new()
        } else {
            format!(" — `{}`", syms.join("`, `"))
        };
        out.push_str(&format!(
            "{}. [[{}]] · {} · {} LOC{}\n",
            i + 1,
            f.path,
            f.lang,
            f.loc,
            sym_str
        ));
    }
    out.push('\n');

    out.push_str("## Project graph\n\n");
    out.push_str(&project_mermaid(index));
    out.push('\n');
    out
}

/// Mermaid graph of the top-ranked nodes and the edges among them.
fn project_mermaid(index: &ProjectIndex) -> String {
    let ranked = index.ranked();
    // Map the top N file paths to stable mermaid ids.
    let mut id_of: HashMap<&str, String> = HashMap::new();
    let mut lines = vec!["```mermaid".to_string(), "graph LR".to_string()];
    for (i, f) in ranked.iter().take(MAP_GRAPH_NODES).enumerate() {
        let id = format!("n{i}");
        lines.push(format!("  {id}[\"{}\"]", basename(&f.path)));
        id_of.insert(f.path.as_str(), id);
    }
    // Resolve indices for the chosen files, then draw edges among them.
    let path_of_index: HashMap<usize, &str> = index
        .files
        .iter()
        .enumerate()
        .map(|(i, f)| (i, f.path.as_str()))
        .collect();
    let mut drawn = 0;
    for e in &index.edges {
        let (Some(fp), Some(tp)) = (path_of_index.get(&e.from), path_of_index.get(&e.to)) else {
            continue;
        };
        if let (Some(fid), Some(tid)) = (id_of.get(fp), id_of.get(tp)) {
            lines.push(format!("  {fid} --> {tid}"));
            drawn += 1;
            if drawn >= 60 {
                break;
            }
        }
    }
    lines.push("```".to_string());
    lines.join("\n") + "\n"
}

// ─── Per-file node doc ──────────────────────────────────────────────────────

/// A `.dx` node document for a single file: frontmatter, symbol list, a mermaid
/// neighborhood graph, and `[[links]]` to connected files.
pub fn file_dx(index: &ProjectIndex, idx: usize) -> String {
    let f = &index.files[idx];
    let neighbors = index.neighbors(idx);
    let mut out = String::new();

    // Links list for frontmatter.
    let links: Vec<String> = neighbors
        .iter()
        .take(12)
        .map(|n| format!("[[{}]]", index.files[n.file].path))
        .collect();

    out.push_str("---\n");
    out.push_str(&format!("id: {}\n", f.path));
    out.push_str(&format!("title: {}\n", basename(&f.path)));
    out.push_str("kind: file-index\n");
    out.push_str(&format!("lang: {}\n", f.lang));
    out.push_str(&format!("rank: {}\n", f.rank));
    if !links.is_empty() {
        out.push_str(&format!("links: {}\n", links.join(" ")));
    }
    out.push_str(&format!("generated_at: {}\n", now_iso()));
    out.push_str("---\n\n");

    out.push_str(&format!(
        "# {}  ·  {}  ·  {} LOC\n\n",
        f.path, f.lang, f.loc
    ));

    if !f.defs.is_empty() {
        out.push_str("## Symbols\n\n");
        for d in f.defs.iter().take(60) {
            out.push_str(&format!("- `{}` ({}) — L{}\n", d.name, d.kind, d.line));
        }
        out.push('\n');
    }

    if !f.headings.is_empty() {
        out.push_str("## Headings\n\n");
        for h in f.headings.iter().take(30) {
            out.push_str(&format!("- {h}\n"));
        }
        out.push('\n');
    }

    if !neighbors.is_empty() {
        out.push_str("## Graph\n\n");
        out.push_str(&file_mermaid(index, idx));
        out.push('\n');

        let (mut outs, mut ins) = (Vec::new(), Vec::new());
        for n in &neighbors {
            let line = format!(
                "- [[{}]]{}",
                index.files[n.file].path,
                if n.via.is_empty() {
                    String::new()
                } else {
                    format!(" via `{}`", n.via.join("`, `"))
                }
            );
            match n.dir {
                Dir::Out => outs.push(line),
                Dir::In => ins.push(line),
            }
        }
        if !outs.is_empty() {
            out.push_str("### References\n\n");
            out.push_str(&outs.join("\n"));
            out.push_str("\n\n");
        }
        if !ins.is_empty() {
            out.push_str("### Referenced by\n\n");
            out.push_str(&ins.join("\n"));
            out.push_str("\n\n");
        }
    }
    out
}

/// Mermaid graph of one file and its immediate neighbors.
fn file_mermaid(index: &ProjectIndex, idx: usize) -> String {
    let neighbors = index.neighbors(idx);
    let mut lines = vec!["```mermaid".to_string(), "graph LR".to_string()];
    lines.push(format!("  c[\"{}\"]", basename(&index.files[idx].path)));
    for (i, n) in neighbors.iter().take(14).enumerate() {
        let id = format!("x{i}");
        lines.push(format!(
            "  {id}[\"{}\"]",
            basename(&index.files[n.file].path)
        ));
        match n.dir {
            Dir::Out => lines.push(format!("  c --> {id}")),
            Dir::In => lines.push(format!("  {id} --> c")),
        }
    }
    lines.push("```".to_string());
    lines.join("\n") + "\n"
}

// ─── Writing docs ───────────────────────────────────────────────────────────

/// Write `map.dx` plus per-file node docs under `.gsh/index/`. Returns the
/// number of `.dx` files written.
pub fn write_docs(root: &Path, index: &ProjectIndex) -> std::io::Result<usize> {
    let dir = index_dir(root);
    let nodes_dir = dir.join("nodes");
    std::fs::create_dir_all(&nodes_dir)?;

    std::fs::write(dir.join("map.dx"), project_map_dx(index))?;
    let mut written = 1;

    // Only the top-ranked files get node docs — keeps the index small/cheap.
    for f in index.ranked().into_iter().take(MAX_NODE_DOCS) {
        let idx = index
            .files
            .iter()
            .position(|x| x.path == f.path)
            .expect("ranked file exists");
        let path = nodes_dir.join(format!("{}.dx", slug(&f.path)));
        std::fs::write(path, file_dx(index, idx))?;
        written += 1;
    }
    Ok(written)
}
