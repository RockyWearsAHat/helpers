//! The native tool table: maps tool names to their MCP schema and handler.
//! `lib/mcp-native.js` keeps a matching allowlist; the two must stay in sync.

use serde_json::Value;

use crate::proto::ToolResult;
use crate::tools;

/// One native MCP tool: its name, a thunk producing its `{name, description,
/// inputSchema}` schema, and its handler.
pub struct Tool {
    pub name: &'static str,
    pub schema: fn() -> Value,
    pub handler: fn(&Value) -> ToolResult,
}

/// Every tool the native binary owns. Adding a tool here (and to the JS
/// allowlist) is all that's needed to route it to Rust.
pub fn all_tools() -> Vec<Tool> {
    use tools::{project_index as pi, workspace_context as wc};
    vec![
        Tool {
            name: "workspace_context",
            schema: wc::schema,
            handler: wc::run,
        },
        Tool {
            name: "index_project",
            schema: pi::schema_index,
            handler: pi::run_index,
        },
        Tool {
            name: "project_map",
            schema: pi::schema_map,
            handler: pi::run_map,
        },
        Tool {
            name: "lookup",
            schema: pi::schema_lookup,
            handler: pi::run_lookup,
        },
    ]
}

/// The `schemas` subcommand payload: every tool's schema as a JSON array.
pub fn schemas() -> Vec<Value> {
    all_tools().iter().map(|t| (t.schema)()).collect()
}

/// Run a tool by name. `None` means the name isn't a native tool.
pub fn dispatch(name: &str, args: &Value) -> Option<ToolResult> {
    all_tools()
        .into_iter()
        .find(|t| t.name == name)
        .map(|t| (t.handler)(args))
}
