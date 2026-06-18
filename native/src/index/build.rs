//! Build a [`ProjectIndex`] from a repository: walk → extract symbols →
//! connect files by shared symbol references → rank.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;

use crate::git::exec_git;
use crate::index::lang::{lang_for_ext, TagExtractor};
use crate::index::model::{Edge, FileEntry, ProjectIndex, SymbolDef, INDEX_VERSION};
use crate::index::rank::pagerank;
use crate::index::walk::walk_repo;
use crate::util::{now_iso, round_to};

/// Build the full project index rooted at `root`.
pub fn build_index(root: &Path) -> ProjectIndex {
    let files = walk_repo(root);
    let mut extractor = TagExtractor::new();
    let mut entries: Vec<FileEntry> = Vec::with_capacity(files.len());
    let mut refs_per_file: Vec<Vec<String>> = Vec::with_capacity(files.len());

    for f in &files {
        let bytes = match std::fs::read(&f.abs) {
            Ok(b) => b,
            Err(_) => continue,
        };
        if is_binary(&bytes) {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        let loc = text.lines().count();
        let mut defs: Vec<SymbolDef> = Vec::new();
        let mut refs: Vec<String> = Vec::new();
        let mut headings: Vec<String> = Vec::new();

        let lang_label = if let Some(lang) = lang_for_ext(&f.ext) {
            for t in extractor.extract(lang, &bytes) {
                if t.is_def {
                    defs.push(SymbolDef {
                        name: t.name,
                        kind: t.kind,
                        line: t.line,
                    });
                } else {
                    refs.push(t.name);
                }
            }
            lang.to_string()
        } else {
            match f.ext.as_str() {
                "sh" | "bash" | "zsh" => shell_defs(&text, &mut defs),
                "md" | "markdown" | "mdx" | "dx" => md_headings(&text, &mut headings),
                _ => {}
            }
            label_for_ext(&f.ext).to_string()
        };

        dedup_defs(&mut defs);
        entries.push(FileEntry {
            path: f.rel.clone(),
            lang: lang_label,
            loc,
            rank: 0.0,
            defs,
            headings,
        });
        refs_per_file.push(refs);
    }

    let (edges, pr_edges) = build_edges(&entries, &refs_per_file);
    let ranks = pagerank(entries.len(), &pr_edges, 0.85, 40);
    // Tests exercise the API but are rarely what you want a project map to
    // foreground, so they get a rank penalty that keeps the impl above them.
    for (i, e) in entries.iter_mut().enumerate() {
        let factor = if is_test_path(&e.path) { 0.3 } else { 1.0 };
        e.rank = round_to(ranks[i] * factor, 6);
    }

    let symbol_count = entries.iter().map(|e| e.defs.len()).sum();
    ProjectIndex {
        version: INDEX_VERSION,
        built_at: now_iso(),
        root: root.to_string_lossy().to_string(),
        commit: exec_git(&["rev-parse", "--short", "HEAD"], root).ok(),
        file_count: entries.len(),
        symbol_count,
        files: entries,
        edges,
    }
}

/// Connect files: an edge `i -> j` means file `i` references a symbol that file
/// `j` defines. Each reference is scaled by `1 / (files defining that name)` so
/// common names (`main`, `run`, `test`) barely connect anything, while a symbol
/// defined in one place and referenced widely makes that file important. Symbols
/// defined in too many files are dropped as noise. Returns the display edges
/// (raw counts) and the rarity-scaled weights used for PageRank.
type Edges = (Vec<Edge>, Vec<(usize, usize, f64)>);

fn build_edges(entries: &[FileEntry], refs_per_file: &[Vec<String>]) -> Edges {
    let mut def_map: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        for d in &e.defs {
            if d.name.len() >= 3 {
                def_map.entry(d.name.as_str()).or_default().push(i);
            }
        }
    }

    // How many distinct files reference each name — used for IDF weighting so
    // generic names (`get`, `parse`, `has`, captured from method calls) that
    // appear everywhere carry almost no connecting weight.
    let mut ref_df: HashMap<&str, usize> = HashMap::new();
    for refs in refs_per_file {
        let uniq: std::collections::HashSet<&str> = refs.iter().map(String::as_str).collect();
        for name in uniq {
            *ref_df.entry(name).or_insert(0) += 1;
        }
    }
    let n = entries.len().max(1) as f64;

    // A symbol defined in more than this many files is too generic to connect.
    let common_threshold = 20usize;
    // A name referenced across more than this many files is a generic token
    // (`get`, `parse`, `has`, captured from method calls) — never an edge.
    let ref_common_threshold = (n * 0.02).max(12.0) as usize;

    // (i,j) -> (raw_count, scaled_weight, via names)
    let mut edge_map: HashMap<(usize, usize), (u32, f64, Vec<String>)> = HashMap::new();
    for (i, refs) in refs_per_file.iter().enumerate() {
        let mut counts: HashMap<&str, u32> = HashMap::new();
        for r in refs {
            *counts.entry(r.as_str()).or_insert(0) += 1;
        }
        for (name, cnt) in counts {
            let Some(def_files) = def_map.get(name) else {
                continue;
            };
            let degree = def_files.len();
            if degree > common_threshold {
                continue;
            }
            let df_count = *ref_df.get(name).unwrap_or(&1);
            if df_count > ref_common_threshold {
                continue;
            }
            // Rarity-scaled influence: sqrt(refs), spread across the definers,
            // weighted by IDF over how many files reference the name.
            let df = df_count as f64;
            let idf = (n / (1.0 + df)).ln().max(0.1);
            let scaled = (cnt as f64).sqrt() / degree as f64 * idf;
            for &j in def_files {
                if j == i {
                    continue;
                }
                let ent = edge_map.entry((i, j)).or_insert((0, 0.0, Vec::new()));
                ent.0 += cnt;
                ent.1 += scaled;
                if ent.2.len() < 5 && !ent.2.iter().any(|s| s == name) {
                    ent.2.push(name.to_string());
                }
            }
        }
    }

    let mut edges: Vec<Edge> = edge_map
        .iter()
        .map(|(&(from, to), (weight, _, via))| Edge {
            from,
            to,
            weight: *weight,
            via: via.clone(),
        })
        .collect();
    edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));

    let pr_edges: Vec<(usize, usize, f64)> = edge_map
        .into_iter()
        .map(|((from, to), (_, scaled, _))| (from, to, scaled))
        .collect();
    (edges, pr_edges)
}

// ─── Fallback extractors ────────────────────────────────────────────────────

fn shell_defs(text: &str, defs: &mut Vec<SymbolDef>) {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?m)^\s*(?:function\s+)?([A-Za-z_][A-Za-z0-9_-]*)\s*\(\)\s*\{").unwrap()
    });
    for (line_no, line) in text.lines().enumerate() {
        if let Some(c) = re.captures(line) {
            defs.push(SymbolDef {
                name: c[1].to_string(),
                kind: "function".to_string(),
                line: line_no + 1,
            });
        }
    }
}

fn md_headings(text: &str, headings: &mut Vec<String>) {
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix('#') {
            let title = rest.trim_start_matches('#').trim();
            if !title.is_empty() && headings.len() < 40 {
                headings.push(title.to_string());
            }
        }
    }
}

fn label_for_ext(ext: &str) -> &str {
    match ext {
        "sh" | "bash" | "zsh" => "shell",
        "md" | "markdown" => "markdown",
        "mdx" => "mdx",
        "dx" => "dx",
        "json" => "json",
        "toml" => "toml",
        "yml" | "yaml" => "yaml",
        "c" => "c",
        "h" => "c-header",
        "java" => "java",
        "rb" => "ruby",
        "" => "text",
        other => other,
    }
}

fn dedup_defs(defs: &mut Vec<SymbolDef>) {
    let mut seen = std::collections::HashSet::new();
    defs.retain(|d| seen.insert((d.name.clone(), d.line)));
}

/// Heuristic binary check: a NUL byte in the first 8 KiB.
fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|&b| b == 0)
}

/// Whether a repo-relative path looks like a test file (directory or filename
/// convention across common ecosystems).
fn is_test_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    let base = lower.rsplit('/').next().unwrap_or(&lower);
    lower.contains("/test/")
        || lower.contains("/tests/")
        || lower.contains("/__tests__/")
        || lower.contains("/spec/")
        || base.starts_with("test-")
        || base.starts_with("test_")
        || base.starts_with("test.")
        || base.contains(".test.")
        || base.contains(".spec.")
        || base.contains("_test.")
        || base.ends_with("test.java")
        || base.ends_with("tests.java")
}
