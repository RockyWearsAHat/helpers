//! Serializable data model for the project index (persisted as
//! `.gsh/index/graph.json`).

use serde::{Deserialize, Serialize};

pub const INDEX_VERSION: u32 = 1;

/// The whole project index: ranked files, their definitions, and the
/// symbol-reference graph between them.
#[derive(Serialize, Deserialize, Default)]
pub struct ProjectIndex {
    pub version: u32,
    pub built_at: String,
    pub root: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub commit: Option<String>,
    pub file_count: usize,
    pub symbol_count: usize,
    pub files: Vec<FileEntry>,
    pub edges: Vec<Edge>,
}

/// One indexed file: its language, size, importance rank, top-level definitions,
/// and (for docs) any markdown headings.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct FileEntry {
    pub path: String,
    pub lang: String,
    pub loc: usize,
    pub rank: f64,
    pub defs: Vec<SymbolDef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub headings: Vec<String>,
}

/// A single symbol definition (function, class, struct, …).
#[derive(Serialize, Deserialize, Clone)]
pub struct SymbolDef {
    pub name: String,
    pub kind: String,
    pub line: usize,
}

/// A directed edge: `from` references a symbol defined in `to`. `via` lists a
/// few of the connecting symbol names; `weight` is the reference count.
#[derive(Serialize, Deserialize, Clone)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub weight: u32,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub via: Vec<String>,
}

impl ProjectIndex {
    /// Files sorted by descending rank (ties broken by path for determinism).
    pub fn ranked(&self) -> Vec<&FileEntry> {
        let mut v: Vec<&FileEntry> = self.files.iter().collect();
        v.sort_by(|a, b| {
            b.rank
                .partial_cmp(&a.rank)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.path.cmp(&b.path))
        });
        v
    }

    /// Outgoing + incoming neighbors of a file index, deduped, each with the
    /// connecting symbols. Used to render a node's local graph.
    pub fn neighbors(&self, idx: usize) -> Vec<Neighbor> {
        use std::collections::HashMap;
        let mut map: HashMap<usize, Neighbor> = HashMap::new();
        for e in &self.edges {
            if e.from == idx {
                let n = map
                    .entry(e.to)
                    .or_insert_with(|| Neighbor::new(e.to, Dir::Out));
                n.merge(e);
            } else if e.to == idx {
                let n = map
                    .entry(e.from)
                    .or_insert_with(|| Neighbor::new(e.from, Dir::In));
                n.merge(e);
            }
        }
        let mut v: Vec<Neighbor> = map.into_values().collect();
        v.sort_by(|a, b| b.weight.cmp(&a.weight).then_with(|| a.file.cmp(&b.file)));
        v
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Dir {
    In,
    Out,
}

pub struct Neighbor {
    pub file: usize,
    pub dir: Dir,
    pub weight: u32,
    pub via: Vec<String>,
}

impl Neighbor {
    fn new(file: usize, dir: Dir) -> Self {
        Neighbor {
            file,
            dir,
            weight: 0,
            via: Vec::new(),
        }
    }
    fn merge(&mut self, e: &Edge) {
        self.weight += e.weight;
        for v in &e.via {
            if self.via.len() < 5 && !self.via.contains(v) {
                self.via.push(v.clone());
            }
        }
    }
}
