//! Project-local tool registry — agent-agnostic reusable flows.
//!
//! An agent registers a named command once; afterwards any agent can run it
//! with a single `tools/call` instead of repeating the steps (and the context)
//! every time. Definitions live in `<workspace>/.gsh/tools/manifest.json`, so
//! they are scoped to the project and shared across agents. No editor, no AI.
//!
//! `register_workspace_tool` / `unregister_workspace_tool` / `list_workspace_tools`
//! are the static meta-tools; the registered flows are surfaced dynamically in
//! `schemas()` and executed via `dispatch()`.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::git::workspace_root;
use crate::proc::run_capture;
use crate::proto::{text, ToolResult};

const NAME_RE: &str = r"^[a-z][a-z0-9_-]{0,63}$";

#[derive(Serialize, Deserialize, Clone, Default)]
struct ProjectTool {
    name: String,
    #[serde(default)]
    description: String,
    /// Shell command/flow run via `sh -c` in the workspace root.
    #[serde(default)]
    command: String,
    /// Optional MCP input schema; defaults to a free-form object.
    #[serde(
        rename = "inputSchema",
        default,
        skip_serializing_if = "Value::is_null"
    )]
    input_schema: Value,
}

#[derive(Serialize, Deserialize, Default)]
struct Manifest {
    #[serde(default)]
    tools: Vec<ProjectTool>,
}

fn manifest_path() -> std::path::PathBuf {
    workspace_root()
        .join(".gsh")
        .join("tools")
        .join("manifest.json")
}

fn load_tools() -> Vec<ProjectTool> {
    std::fs::read_to_string(manifest_path())
        .ok()
        .and_then(|s| serde_json::from_str::<Manifest>(&s).ok())
        .map(|m| m.tools)
        .unwrap_or_default()
}

fn save_tools(tools: &[ProjectTool]) -> Result<(), String> {
    let path = manifest_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&Manifest {
        tools: tools.to_vec(),
    })
    .map_err(|e| e.to_string())?;
    std::fs::write(path, json + "\n").map_err(|e| e.to_string())
}

/// Dynamic schemas for the registered flows (runnable ones only).
pub fn schemas() -> Vec<Value> {
    load_tools()
        .into_iter()
        .filter(|t| !t.command.trim().is_empty())
        .map(|t| {
            let input = if t.input_schema.is_null() {
                json!({ "type": "object", "properties": {} })
            } else {
                t.input_schema
            };
            let desc = if t.description.is_empty() {
                format!("Project flow: {}", t.name)
            } else {
                t.description
            };
            json!({ "name": t.name, "description": desc, "inputSchema": input })
        })
        .collect()
}

/// Execute a registered flow by name. `None` if `name` is not a project tool.
pub fn dispatch(name: &str, args: &Value) -> Option<ToolResult> {
    let tool = load_tools().into_iter().find(|t| t.name == name)?;
    if tool.command.trim().is_empty() {
        return Some(Err(format!(
            "Project tool \"{name}\" has no command. Re-register it with a command."
        )));
    }
    let args_json = serde_json::to_string(args).unwrap_or_else(|_| "{}".into());
    let (ok, out, err) = run_capture(
        "sh",
        &["-c", &tool.command],
        Some(&workspace_root()),
        &[("GSH_TOOL_ARGS", args_json.as_str())],
        120,
    );
    let body = {
        let o = out.trim();
        let e = err.trim();
        if !o.is_empty() {
            o.to_string()
        } else if !e.is_empty() {
            e.to_string()
        } else {
            "(no output)".to_string()
        }
    };
    Some(Ok(vec![text(if ok {
        body
    } else {
        format!("Project tool \"{name}\" failed:\n{body}")
    })]))
}

// ─── meta-tools: register / unregister / list ───────────────────────────────

fn str_arg(args: &Value, key: &str) -> String {
    args.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

pub fn run_register(args: &Value) -> ToolResult {
    let name = str_arg(args, "name");
    let description = str_arg(args, "description");
    let command = str_arg(args, "command");
    if name.is_empty() {
        return Err("register_workspace_tool: 'name' is required.".into());
    }
    if description.is_empty() {
        return Err("register_workspace_tool: 'description' is required.".into());
    }
    if !regex::Regex::new(NAME_RE).unwrap().is_match(&name) {
        return Err(format!(
            "register_workspace_tool: invalid name \"{name}\". Use lowercase letters, digits, hyphens, underscores; must start with a letter."
        ));
    }
    if command.is_empty() {
        return Err(
            "register_workspace_tool: 'command' is required (the shell command/flow to run)."
                .into(),
        );
    }

    let mut tools = load_tools();
    let input_schema = args.get("inputSchema").cloned().unwrap_or(Value::Null);
    let record = ProjectTool {
        name: name.clone(),
        description,
        command,
        input_schema,
    };
    let existed = match tools.iter().position(|t| t.name == name) {
        Some(i) => {
            tools[i] = record;
            true
        }
        None => {
            tools.push(record);
            false
        }
    };
    save_tools(&tools)?;

    Ok(vec![text(format!(
        "Project tool \"{name}\" {}.\nStored in .gsh/tools/manifest.json (scoped to this project).\n\nIt is now live in tools/list — call it directly:\n  tools/call {{ \"name\": \"{name}\", \"arguments\": {{ ... }} }}",
        if existed { "updated" } else { "registered" }
    ))])
}

pub fn run_unregister(args: &Value) -> ToolResult {
    let name = str_arg(args, "name");
    if name.is_empty() {
        return Err("unregister_workspace_tool: 'name' is required.".into());
    }
    let mut tools = load_tools();
    match tools.iter().position(|t| t.name == name) {
        Some(i) => {
            tools.remove(i);
            save_tools(&tools)?;
            Ok(vec![text(format!(
                "Project tool \"{name}\" removed. It no longer appears in tools/list."
            ))])
        }
        None => Err(format!(
            "unregister_workspace_tool: tool \"{name}\" not found."
        )),
    }
}

pub fn run_list(_args: &Value) -> ToolResult {
    let tools = load_tools();
    if tools.is_empty() {
        return Ok(vec![text(
            "No project tools registered. Use register_workspace_tool to add a reusable flow.",
        )]);
    }
    let mut lines = vec![format!("{} project tool(s):", tools.len()), String::new()];
    for t in &tools {
        lines.push(format!(
            "- {} — {}",
            t.name,
            if t.description.is_empty() {
                "(no description)"
            } else {
                &t.description
            }
        ));
        if !t.command.trim().is_empty() {
            lines.push(format!("    $ {}", t.command.lines().next().unwrap_or("")));
        }
    }
    Ok(vec![text(lines.join("\n"))])
}

// ─── schemas for the meta-tools ─────────────────────────────────────────────

pub fn schema_register() -> Value {
    json!({
        "name": "register_workspace_tool",
        "description": "Register a reusable project flow as a callable MCP tool. Give it a name, a description, and a shell command (the flow). It is written to .gsh/tools/manifest.json (scoped to this project, shared across agents) and becomes immediately callable via tools/call — so a repetitive multi-step task becomes one tool call instead of repeated context. No editor or AI required. The command runs in the project root with the call's arguments available as JSON in $GSH_TOOL_ARGS.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Tool name (lowercase letters/digits/-/_, starts with a letter). Becomes the tools/call name." },
                "description": { "type": "string", "description": "What the flow does — agents use this to decide when to call it." },
                "command": { "type": "string", "description": "Shell command/flow to run (may be multi-line). Arguments to the tool call are provided as JSON in $GSH_TOOL_ARGS." },
                "inputSchema": { "type": "object", "description": "Optional JSON schema for the tool's arguments. Defaults to a free-form object." }
            },
            "required": ["name", "description", "command"]
        }
    })
}

pub fn schema_unregister() -> Value {
    json!({
        "name": "unregister_workspace_tool",
        "description": "Remove a registered project flow. It immediately disappears from tools/list.",
        "inputSchema": {
            "type": "object",
            "properties": { "name": { "type": "string", "description": "Exact tool name to remove." } },
            "required": ["name"]
        }
    })
}

pub fn schema_list() -> Value {
    json!({
        "name": "list_workspace_tools",
        "description": "List the project flows registered for this workspace (name, description, command). Use this to discover reusable tools before re-implementing a task.",
        "inputSchema": { "type": "object", "properties": {}, "required": [] }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_list_dispatch_unregister_cycle() {
        let tmp = std::env::temp_dir().join(format!("gsh-pt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var(
            "GSH_WORKSPACE_ROOTS",
            format!("[{:?}]", tmp.to_string_lossy()),
        );

        let reg = run_register(&json!({
            "name": "say-hi",
            "description": "echo a greeting",
            "command": "echo hello-from-flow"
        }));
        assert!(reg.is_ok());

        // appears in schemas + list
        assert!(schemas().iter().any(|s| s["name"] == "say-hi"));
        let listed = run_list(&Value::Null).unwrap();
        assert!(listed[0].text.contains("say-hi"));

        // dispatch runs the command
        let out = dispatch("say-hi", &json!({})).unwrap().unwrap();
        assert!(
            out[0].text.contains("hello-from-flow"),
            "got: {}",
            out[0].text
        );

        // unregister removes it
        run_unregister(&json!({ "name": "say-hi" })).unwrap();
        assert!(dispatch("say-hi", &json!({})).is_none());

        std::env::remove_var("GSH_WORKSPACE_ROOTS");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
