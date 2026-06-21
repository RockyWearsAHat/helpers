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
    use tools::{
        checkpoint as cp, knowledge as kn, lint as ln, project_index as pi, setup as su,
    };
    vec![
        Tool {
            name: "checkpoint",
            schema: cp::schema,
            handler: cp::run,
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
        Tool {
            name: "project_setup",
            schema: su::schema,
            handler: su::run,
        },
        Tool {
            name: "lint",
            schema: ln::schema,
            handler: ln::run,
        },
        Tool {
            name: "build_knowledge_index",
            schema: kn::schema_build_index,
            handler: kn::run_build_index,
        },
        Tool {
            name: "search_knowledge_index",
            schema: kn::schema_search_index,
            handler: kn::run_search_index,
        },
        Tool {
            name: "search_knowledge_cache",
            schema: kn::schema_search_cache,
            handler: kn::run_search_cache,
        },
        Tool {
            name: "read_knowledge_note",
            schema: kn::schema_read_note,
            handler: kn::run_read_note,
        },
        Tool {
            name: "write_knowledge_note",
            schema: kn::schema_write_note,
            handler: kn::run_write_note,
        },
        Tool {
            name: "update_knowledge_note",
            schema: kn::schema_update_note,
            handler: kn::run_update_note,
        },
        Tool {
            name: "append_to_knowledge_note",
            schema: kn::schema_append_note,
            handler: kn::run_append_note,
        },
        Tool {
            name: "submit_community_research",
            schema: kn::schema_submit,
            handler: kn::run_submit,
        },
        // Project-local tool registry (agent-agnostic reusable flows).
        Tool {
            name: "register_workspace_tool",
            schema: tools::project_tools::schema_register,
            handler: tools::project_tools::run_register,
        },
        Tool {
            name: "unregister_workspace_tool",
            schema: tools::project_tools::schema_unregister,
            handler: tools::project_tools::run_unregister,
        },
        Tool {
            name: "list_workspace_tools",
            schema: tools::project_tools::schema_list,
            handler: tools::project_tools::run_list,
        },
    ]
}

/// The `schemas` subcommand payload: the built-in tools plus any project-local
/// flows registered in this workspace's `.helpers/tools/manifest.json`.
pub fn schemas() -> Vec<Value> {
    let mut out: Vec<Value> = all_tools().iter().map(|t| (t.schema)()).collect();
    out.extend(tools::project_tools::schemas());
    out
}

/// Run a tool by name. Built-in tools take precedence; an unknown name falls
/// through to the registered project flows. `None` means no such native tool.
pub fn dispatch(name: &str, args: &Value) -> Option<ToolResult> {
    if let Some(t) = all_tools().into_iter().find(|t| t.name == name) {
        return Some((t.handler)(args));
    }
    tools::project_tools::dispatch(name, args)
}
