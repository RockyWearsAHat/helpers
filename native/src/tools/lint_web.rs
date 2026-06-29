//! `lint_web` — project-graph MCP tools.
//!
//! These tools give agents and the linter itself whole-codebase awareness:
//!
//! * **`lint_build_web`** — parse every source file, assemble the call graph,
//!   and cache it to `.helpers/lint-web.json`. Subsequent tool calls load the
//!   graph in milliseconds.
//! * **`lint_probe`** — run a named lint rule as a structural probe across the
//!   whole project; each match is annotated with its containing function and
//!   one step of forward + backward trace context.
//! * **`lint_trace`** — trace forward or backward from any file:line, following
//!   call edges through the graph up to a requested depth.

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::git::workspace_root;
use crate::lint_graph::{file_lang, ProjectWeb};
use crate::lint_train;
use crate::proto::{text, ToolResult};

// ── shared helpers ────────────────────────────────────────────────────────────

/// Resolve the `root` argument (defaults to the workspace root when absent).
fn root_arg(args: &Value) -> PathBuf {
    match args.get("root").and_then(Value::as_str) {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => workspace_root(),
    }
}

/// Load the cached project web; build it on-the-fly if the cache is missing.
fn load_or_build(root: &std::path::Path) -> ProjectWeb {
    ProjectWeb::load(root).unwrap_or_else(|| ProjectWeb::build(root))
}

// ── lint_build_web ────────────────────────────────────────────────────────────

/// Build (or rebuild) the project call-graph web for `root` and cache it to
/// `.helpers/lint-web.json`. Agents and the linter use this to get trace context
/// (containing function, callers, callees) for any lint hit in the project.
pub fn run_build_web(args: &Value) -> ToolResult {
    let root = root_arg(args);
    if !root.exists() {
        return Err(format!("lint_build_web: path not found: {}", root.display()));
    }

    let web = ProjectWeb::build(&root);
    let fns  = web.fn_count();
    let edges = web.edge_count();
    web.save(&root).map_err(|e| format!("lint_build_web: {e}"))?;

    Ok(vec![text(format!(
        "Project web built and cached.\n\
         Functions : {fns}\n\
         Call edges: {edges}\n\
         Saved to  : {}",
        root.join(".helpers/lint-web.json").display()
    ))])
}

/// MCP schema for the `lint_build_web` tool.
pub fn schema_build_web() -> Value {
    json!({
        "name": "lint_build_web",
        "description": "Build the project call-graph web by parsing all source files. \
            Caches the result to .helpers/lint-web.json for use by lint_probe and lint_trace. \
            Run once per project; re-run after large refactors.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "root": {
                    "type": "string",
                    "description": "Project root directory to parse (default: workspace root)."
                }
            }
        }
    })
}

// ── lint_probe ────────────────────────────────────────────────────────────────

/// Run a named lint rule as a structural probe across the whole project.
/// Each match is reported with its containing function and one hop of
/// forward + backward call-graph trace.
pub fn run_probe(args: &Value) -> ToolResult {
    let root = root_arg(args);
    if !root.exists() {
        return Err(format!("lint_probe: path not found: {}", root.display()));
    }

    let rule_id = args
        .get("rule")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if rule_id.is_empty() {
        return Err("lint_probe: `rule` is required".to_string());
    }

    let web = load_or_build(&root);

    // Walk all source files and apply the probe rule to each.
    use crate::index::walk::walk_repo;
    let files = walk_repo(&root);

    let mut out = format!("Probe: {rule_id}\n\n");
    let mut total_hits = 0usize;

    for f in &files {
        let Some(lang) = file_lang(&f.ext) else { continue };
        let Some(model) = lint_train::load_patterns(lang) else { continue };
        let Ok(code) = std::fs::read_to_string(&f.abs) else { continue };

        let findings: Vec<_> = model
            .flag(&code)
            .into_iter()
            .filter(|fd| fd.rule == rule_id)
            .collect();

        if findings.is_empty() { continue; }

        out.push_str(&format!("── {} ──\n", f.rel));
        for fd in findings {
            total_hits += 1;
            out.push_str(&format!("  line {}: [{}] ", fd.line, fd.severity));

            // Containing function.
            let container = web.containing_fn(&f.abs, fd.line);
            if let Some(fn_name) = container {
                out.push_str(&format!("in `{fn_name}`"));

                // Backward trace (who calls this).
                let callers = web.direct_callers(fn_name);
                if !callers.is_empty() {
                    let names: Vec<&str> = callers.iter().map(|c| c.name.as_str()).collect();
                    out.push_str(&format!("  ← called by: {}", names.join(", ")));
                }

                // Forward trace (what this calls).
                let callees = web.direct_callees(fn_name);
                if !callees.is_empty() {
                    let names: Vec<&str> = callees.iter().map(|c| c.name.as_str()).collect();
                    out.push_str(&format!("  → calls: {}", names.join(", ")));
                }
            }
            out.push('\n');
        }
        out.push('\n');
    }

    if total_hits == 0 {
        out.push_str("No matches found.\n");
    } else {
        out.push_str(&format!("Total: {total_hits} match(es)\n"));
    }

    Ok(vec![text(out)])
}

/// MCP schema for the `lint_probe` tool.
pub fn schema_probe() -> Value {
    json!({
        "name": "lint_probe",
        "description": "Run a named lint rule as a structural probe across the whole project. \
            Each match is annotated with its containing function and one hop of call-graph \
            trace (callers + callees). Requires lint rules to be trained (run lint first, \
            or ensure lint-models/ exists).",
        "inputSchema": {
            "type": "object",
            "properties": {
                "root": {
                    "type": "string",
                    "description": "Project root (default: workspace root)."
                },
                "rule": {
                    "type": "string",
                    "description": "Rule id to probe for (e.g. 'bare_unwrap', 'off_by_one_indexing')."
                }
            },
            "required": ["rule"]
        }
    })
}

// ── lint_trace ────────────────────────────────────────────────────────────────

/// Trace forward or backward through the call graph from a specific location.
/// `file` and `line` identify the starting point; `direction` is "forward",
/// "backward", or "both". `depth` limits the number of hops (default 3).
pub fn run_trace(args: &Value) -> ToolResult {
    let root = root_arg(args);

    let file_str = args.get("file").and_then(Value::as_str).unwrap_or("");
    if file_str.is_empty() {
        return Err("lint_trace: `file` is required".to_string());
    }
    let file_path = if std::path::Path::new(file_str).is_absolute() {
        PathBuf::from(file_str)
    } else {
        root.join(file_str)
    };

    let line = args.get("line").and_then(Value::as_u64).unwrap_or(1) as usize;
    let depth = args.get("depth").and_then(Value::as_u64).unwrap_or(3).clamp(1, 10) as usize;
    let direction = args.get("direction").and_then(Value::as_str).unwrap_or("both");

    let web = load_or_build(&root);

    let container = web.containing_fn(&file_path, line);
    let Some(fn_name) = container else {
        return Ok(vec![text(format!(
            "No function found at {}:{line} in the project web.\n\
             (Run lint_build_web if you haven't yet.)",
            file_path.display()
        ))]);
    };

    let mut out = format!(
        "Trace from `{fn_name}` ({}:{line})\n\n",
        file_path.display()
    );

    if matches!(direction, "backward" | "both") {
        let hops = web.backward_trace(fn_name, depth);
        if hops.is_empty() {
            out.push_str("← backward: (no callers found)\n");
        } else {
            out.push_str("← backward (callers):\n");
            for h in &hops {
                out.push_str(&format!("  {} ({}:{})\n", h.name, h.file.display(), h.line));
            }
        }
        out.push('\n');
    }

    if matches!(direction, "forward" | "both") {
        let hops = web.forward_trace(fn_name, depth);
        if hops.is_empty() {
            out.push_str("→ forward: (no callees found)\n");
        } else {
            out.push_str("→ forward (callees):\n");
            for h in &hops {
                out.push_str(&format!("  {} ({}:{})\n", h.name, h.file.display(), h.line));
            }
        }
    }

    Ok(vec![text(out)])
}

/// MCP schema for the `lint_trace` tool.
pub fn schema_trace() -> Value {
    json!({
        "name": "lint_trace",
        "description": "Trace forward or backward through the call graph from a file:line. \
            Shows which functions call into this location (backward) and which functions \
            this location calls (forward). Useful for understanding the blast radius of a \
            violation or confirming a rule applies in context.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "root": {
                    "type": "string",
                    "description": "Project root (default: workspace root)."
                },
                "file": {
                    "type": "string",
                    "description": "Source file path (absolute, or relative to root)."
                },
                "line": {
                    "type": "integer",
                    "description": "1-based line number to trace from."
                },
                "direction": {
                    "type": "string",
                    "enum": ["forward", "backward", "both"],
                    "description": "Trace direction (default: both)."
                },
                "depth": {
                    "type": "integer",
                    "description": "Maximum call-graph hops to follow (default: 3, max: 10)."
                }
            },
            "required": ["file"]
        }
    })
}

