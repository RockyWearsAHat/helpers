//! `lint_graph` — the project web: a call-graph model of the whole codebase.
//!
//! Every source file is parsed once with tree-sitter. Function definitions and
//! call sites are extracted and linked into a bidirectional call graph. This
//! enables lint hits to carry trace context: the containing function, who calls
//! it (backward trace), and what it reaches next (forward trace).
//!
//! Persistence: `save` / `load` checkpoint the web to `.helpers/lint-web.json`
//! so the build cost is paid once; subsequent runs load in milliseconds.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tree_sitter::Parser;

use crate::index::walk::walk_repo;
use crate::lint_match::language;

// ── public types ─────────────────────────────────────────────────────────────

/// A function or method definition found in the codebase.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FnNode {
    /// Simple name (e.g. `parse_config`, not a qualified path).
    pub name: String,
    /// Absolute source file path.
    pub file: PathBuf,
    /// 1-based first line of the function body.
    pub start_line: usize,
    /// 1-based last line of the function body.
    pub end_line: usize,
    /// Language name (e.g. `rust`, `python`).
    pub lang: String,
}

/// One hop in a forward or backward trace.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceHop {
    pub name: String,
    pub file: PathBuf,
    pub line: usize,
}

/// The project web: bidirectional call graph assembled from all source files.
///
/// `build(root)` creates it; `save` / `load` checkpoint it to disk.
#[derive(Default, Serialize, Deserialize)]
pub struct ProjectWeb {
    /// All known function definitions, keyed by name.
    /// When multiple definitions share a name (e.g. overloads in different files),
    /// the last one encountered is kept — sufficient for trace lookups.
    pub functions: HashMap<String, FnNode>,
    /// Forward call edges: caller name → distinct callee names.
    pub out_calls: HashMap<String, Vec<String>>,
    /// Backward call edges: callee name → distinct caller names.
    pub in_calls: HashMap<String, Vec<String>>,
}

impl ProjectWeb {
    /// Build the project web by walking all source files under `root`.
    /// Parsing is parallelized across files; edges are merged sequentially.
    pub fn build(root: &Path) -> Self {
        let files = walk_repo(root);
        let slices: Vec<WebSlice> = files
            .par_iter()
            .filter_map(|f| {
                let lang = file_lang(&f.ext)?;
                parse_file_slice(&f.abs, lang)
            })
            .collect();

        let mut web = ProjectWeb::default();
        for slice in slices {
            for fn_node in slice.functions {
                web.functions.insert(fn_node.name.clone(), fn_node);
            }
            for (caller, callee) in slice.calls {
                web.out_calls.entry(caller.clone()).or_default().push(callee.clone());
                web.in_calls.entry(callee).or_default().push(caller);
            }
        }
        for v in web.out_calls.values_mut() { v.sort(); v.dedup(); }
        for v in web.in_calls.values_mut() { v.sort(); v.dedup(); }
        web
    }

    /// The name of the innermost function containing `(file, line)`, if known.
    pub fn containing_fn(&self, file: &Path, line: usize) -> Option<&str> {
        self.functions
            .values()
            .filter(|f| f.file == file && f.start_line <= line && line <= f.end_line)
            .min_by_key(|f| f.end_line - f.start_line) // innermost = tightest range
            .map(|f| f.name.as_str())
    }

    /// All functions that `fn_name` directly calls (one hop forward).
    pub fn direct_callees(&self, fn_name: &str) -> Vec<&FnNode> {
        self.out_calls
            .get(fn_name)
            .map(|names| names.iter().filter_map(|n| self.functions.get(n)).collect())
            .unwrap_or_default()
    }

    /// All functions that directly call `fn_name` (one hop backward).
    pub fn direct_callers(&self, fn_name: &str) -> Vec<&FnNode> {
        self.in_calls
            .get(fn_name)
            .map(|names| names.iter().filter_map(|n| self.functions.get(n)).collect())
            .unwrap_or_default()
    }

    /// BFS forward from `fn_name` up to `depth` hops (what this function reaches).
    pub fn forward_trace(&self, fn_name: &str, depth: usize) -> Vec<TraceHop> {
        self.bfs_trace(fn_name, depth, &self.out_calls)
    }

    /// BFS backward from `fn_name` up to `depth` hops (what calls into this function).
    pub fn backward_trace(&self, fn_name: &str, depth: usize) -> Vec<TraceHop> {
        self.bfs_trace(fn_name, depth, &self.in_calls)
    }

    fn bfs_trace(&self, start: &str, depth: usize, edges: &HashMap<String, Vec<String>>) -> Vec<TraceHop> {
        let mut result = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        visited.insert(start.to_string());
        queue.push_back((start.to_string(), 0));
        while let Some((name, d)) = queue.pop_front() {
            if d >= depth { continue; }
            if let Some(neighbors) = edges.get(&name) {
                for nb in neighbors {
                    if visited.insert(nb.clone()) {
                        if let Some(f) = self.functions.get(nb) {
                            result.push(TraceHop {
                                name: nb.clone(),
                                file: f.file.clone(),
                                line: f.start_line,
                            });
                        }
                        queue.push_back((nb.clone(), d + 1));
                    }
                }
            }
        }
        result
    }

    /// Total number of function definitions in the web.
    pub fn fn_count(&self) -> usize { self.functions.len() }

    /// Total number of directed call edges in the web.
    pub fn edge_count(&self) -> usize {
        self.out_calls.values().map(Vec::len).sum()
    }

    /// Persist the web to `.helpers/lint-web.json` under `root`.
    pub fn save(&self, root: &Path) -> Result<(), String> {
        let path = root.join(".helpers/lint-web.json");
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let json = serde_json::to_string(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }

    /// Load from `.helpers/lint-web.json` under `root`. Returns `None` on any error.
    pub fn load(root: &Path) -> Option<Self> {
        let path = root.join(".helpers/lint-web.json");
        let text = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&text).ok()
    }
}

// ── internal: per-file parsing ────────────────────────────────────────────────

/// Raw output from parsing one source file.
struct WebSlice {
    functions: Vec<FnNode>,
    /// (caller_name, callee_name) pairs from this file.
    calls: Vec<(String, String)>,
}

/// Parse one source file and extract function defs + call sites.
fn parse_file_slice(path: &Path, lang: &str) -> Option<WebSlice> {
    let grammar = language(lang)?;
    let src = std::fs::read(path).ok()?;
    let mut parser = Parser::new();
    parser.set_language(&grammar).ok()?;
    let tree = parser.parse(&src, None)?;

    let mut fns: Vec<FnNode> = Vec::new();
    let mut raw_calls: Vec<(usize, String)> = Vec::new(); // (line, callee)

    // Collect function defs and call sites in one tree walk.
    collect_nodes(tree.root_node(), &src, path, lang, &mut fns, &mut raw_calls);

    // Attribute each call site to the innermost enclosing function.
    let calls: Vec<(String, String)> = raw_calls
        .into_iter()
        .filter_map(|(line, callee)| {
            let caller = fns
                .iter()
                .filter(|f| f.start_line <= line && line <= f.end_line)
                .min_by_key(|f| f.end_line - f.start_line)?
                .name
                .clone();
            Some((caller, callee))
        })
        .filter(|(caller, callee)| caller != callee) // drop trivial self-calls
        .collect();

    Some(WebSlice { functions: fns, calls })
}

/// Recursive tree walk — collects function definitions and call site names.
fn collect_nodes(
    node: tree_sitter::Node,
    src: &[u8],
    path: &Path,
    lang: &str,
    fns: &mut Vec<FnNode>,
    calls: &mut Vec<(usize, String)>,
) {
    if is_fn_def(node, lang) {
        if let Some(name) = fn_name(node, src, lang) {
            fns.push(FnNode {
                name,
                file: path.to_path_buf(),
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
                lang: lang.to_string(),
            });
        }
    }

    if is_call_site(node, lang) {
        if let Some(callee) = callee_name(node, src, lang) {
            calls.push((node.start_position().row + 1, callee));
        }
    }

    // Collect children eagerly to avoid borrow issues, then recurse.
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    for child in children {
        collect_nodes(child, src, path, lang, fns, calls);
    }
}

// ── language dispatch ─────────────────────────────────────────────────────────

/// `true` when `node` is a function or method definition in `lang`.
fn is_fn_def(node: tree_sitter::Node, lang: &str) -> bool {
    let k = node.kind();
    match lang {
        "rust"                             => k == "function_item",
        "python"                           => k == "function_definition",
        "javascript" | "typescript" | "tsx" => matches!(k, "function_declaration" | "method_definition" | "function"),
        "go"                               => matches!(k, "function_declaration" | "method_declaration"),
        "java"                             => k == "method_declaration",
        "ruby"                             => matches!(k, "method" | "singleton_method"),
        "c" | "cpp"                        => k == "function_definition",
        "bash"                             => k == "function_definition",
        _                                  => false,
    }
}

/// Extract the function name from a definition node. Returns `None` when the
/// definition is anonymous or the name field is absent.
fn fn_name(node: tree_sitter::Node, src: &[u8], lang: &str) -> Option<String> {
    let name_node = match lang {
        "c" | "cpp" => {
            // C/C++: name is nested inside the declarator.
            let decl = node.child_by_field_name("declarator")?;
            // `function_declarator { declarator: identifier }`
            decl.child_by_field_name("declarator")
                .or_else(|| Some(decl))
        }
        _ => node.child_by_field_name("name"),
    }?;
    name_node.utf8_text(src).ok().map(|s| s.trim().to_string())
}

/// `true` when `node` is a call expression in `lang`.
fn is_call_site(node: tree_sitter::Node, lang: &str) -> bool {
    let k = node.kind();
    match lang {
        "rust"                             => k == "call_expression",
        "python"                           => k == "call",
        "javascript" | "typescript" | "tsx" => k == "call_expression",
        "go"                               => k == "call_expression",
        "java"                             => k == "method_invocation",
        "ruby"                             => k == "call",
        "c" | "cpp"                        => k == "call_expression",
        "bash"                             => k == "command",
        _                                  => false,
    }
}

/// Extract the callee name from a call site. Returns `None` for complex
/// call targets (closures, computed values) that don't reduce to a simple name.
fn callee_name(node: tree_sitter::Node, src: &[u8], lang: &str) -> Option<String> {
    // Java uses `name` field directly on method_invocation.
    if matches!(lang, "java") {
        return node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(src).ok())
            .map(|s| s.trim().to_string());
    }

    // Bash: command name is first child.
    if lang == "bash" {
        return node
            .child(0)
            .and_then(|n| n.utf8_text(src).ok())
            .map(|s| s.trim().to_string());
    }

    let func = node.child_by_field_name("function")?;

    // Direct call: `foo(…)`
    if matches!(func.kind(), "identifier" | "simple_identifier") {
        return func.utf8_text(src).ok().map(|s| s.trim().to_string());
    }

    // Method call: `obj.method(…)` — the field name varies by grammar.
    let field_key = match lang {
        "rust"                             => "field",
        "python"                           => "attribute",
        "javascript" | "typescript" | "tsx" => "property",
        "go"                               => "field",
        _                                  => "field",
    };
    func.child_by_field_name(field_key)
        .and_then(|f| f.utf8_text(src).ok())
        .map(|s| s.trim().to_string())
}

// ── file → language mapping ───────────────────────────────────────────────────

/// Map a file extension to a language name. Mirrors the table in `tools/lint.rs`
/// (shared logic, kept local to avoid a cross-module dependency).
pub fn file_lang(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "rs"                            => "rust",
        "py"                            => "python",
        "js" | "mjs" | "cjs" | "jsx"   => "javascript",
        "ts" | "tsx"                    => "typescript",
        "go"                            => "go",
        "java"                          => "java",
        "rb"                            => "ruby",
        "c"  | "h"                      => "c",
        "cpp" | "cc" | "cxx" | "hpp"   => "cpp",
        "sh" | "bash"                   => "bash",
        "kt" | "kts"                    => "kotlin",
        "swift"                         => "swift",
        "cs"                            => "csharp",
        "php"                           => "php",
        _                               => return None,
    })
}
