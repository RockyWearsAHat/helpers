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

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::git::{find_repo_root, workspace_root};
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

/// The `.gsh/tools/manifest.json` path for a given workspace root.
fn manifest_path_in(ws: &Path) -> PathBuf {
    ws.join(".gsh").join("tools").join("manifest.json")
}

/// The repo containing the running `gsh-native` binary — i.e. the GSH install
/// itself. Project flows almost never belong here, so a write landing here is
/// the signal that the workspace was misresolved (see [`writable_workspace`]).
fn install_repo_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe = std::fs::canonicalize(&exe).unwrap_or(exe);
    find_repo_root(exe.parent()?)
}

/// Whether two paths point at the same location, comparing canonical forms when
/// they resolve and falling back to a literal comparison.
fn same_path(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// Resolve the workspace a manifest write should target, refusing to contaminate
/// the GSH install repo. A misresolved workspace (the MCP host's cwd left
/// pointing at the GSH source tree) used to silently register project flows into
/// GSH's own `.gsh/tools/manifest.json`; now that fails loudly unless `force` is
/// set, so a stray registration is impossible to miss instead of polluting an
/// unrelated repo.
fn writable_workspace(force: bool) -> Result<PathBuf, String> {
    let ws = workspace_root();
    if !force {
        if let Some(install) = install_repo_root() {
            if same_path(&ws, &install) {
                return Err(format!(
                    "Refusing to write project flows into the GSH install repo ({}). \
                     The workspace was not resolved to your project — open the intended \
                     project (or set GSH_WORKSPACE_ROOTS to it) and retry. Pass \
                     {{\"force\": true}} only to register a flow for GSH itself.",
                    ws.display()
                ));
            }
        }
    }
    Ok(ws)
}

fn load_tools_at(path: &Path) -> Vec<ProjectTool> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<Manifest>(&s).ok())
        .map(|m| m.tools)
        .unwrap_or_default()
}

fn save_tools_at(path: &Path, tools: &[ProjectTool]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&Manifest {
        tools: tools.to_vec(),
    })
    .map_err(|e| e.to_string())?;
    std::fs::write(path, json + "\n").map_err(|e| e.to_string())
}

/// Tools registered in the current workspace, for read-only surfacing
/// (`schemas`, `list`). Reads never trigger the install-repo guard.
fn load_tools() -> Vec<ProjectTool> {
    load_tools_at(&manifest_path_in(&workspace_root()))
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

fn bool_arg(args: &Value, key: &str) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// Register (or update) a named project flow from `name`/`description`/`command`
/// args, persisting it to the workspace manifest so it becomes callable by name.
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

    let path = manifest_path_in(&writable_workspace(bool_arg(args, "force"))?);
    let mut tools = load_tools_at(&path);
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
    save_tools_at(&path, &tools)?;

    Ok(vec![text(format!(
        "Project tool \"{name}\" {}.\nStored in .gsh/tools/manifest.json (scoped to this project).\n\nIt is now live in tools/list — call it directly:\n  tools/call {{ \"name\": \"{name}\", \"arguments\": {{ ... }} }}",
        if existed { "updated" } else { "registered" }
    ))])
}

/// Remove the named project flow from the workspace manifest; errors if absent.
pub fn run_unregister(args: &Value) -> ToolResult {
    let name = str_arg(args, "name");
    if name.is_empty() {
        return Err("unregister_workspace_tool: 'name' is required.".into());
    }
    let path = manifest_path_in(&writable_workspace(bool_arg(args, "force"))?);
    let mut tools = load_tools_at(&path);
    match tools.iter().position(|t| t.name == name) {
        Some(i) => {
            tools.remove(i);
            save_tools_at(&path, &tools)?;
            Ok(vec![text(format!(
                "Project tool \"{name}\" removed. It no longer appears in tools/list."
            ))])
        }
        None => Err(format!(
            "unregister_workspace_tool: tool \"{name}\" not found."
        )),
    }
}

/// List every registered project flow (name, description, first command line).
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

/// MCP tool schema for `register_workspace_tool`.
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
                "inputSchema": { "type": "object", "description": "Optional JSON schema for the tool's arguments. Defaults to a free-form object." },
                "force": { "type": "boolean", "description": "Override the safety guard that refuses to write into the GSH install repo itself. Only set this to register a flow for GSH's own development." }
            },
            "required": ["name", "description", "command"]
        }
    })
}

/// MCP tool schema for `unregister_workspace_tool`.
pub fn schema_unregister() -> Value {
    json!({
        "name": "unregister_workspace_tool",
        "description": "Remove a registered project flow. It immediately disappears from tools/list.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Exact tool name to remove." },
                "force": { "type": "boolean", "description": "Override the guard that refuses to operate on the GSH install repo itself." }
            },
            "required": ["name"]
        }
    })
}

/// MCP tool schema for `list_workspace_tools`.
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
    use std::sync::Mutex;

    // Both tests mutate the process-global GSH_WORKSPACE_ROOTS; serialize them so
    // they don't race under cargo's parallel test runner.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn register_list_dispatch_unregister_cycle() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

    #[test]
    fn register_refuses_to_write_into_gsh_install_repo() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Point the workspace at the GSH install repo itself (the repo holding
        // the running binary). Registering there must fail loudly rather than
        // contaminate GSH's own manifest — the bug this guard prevents.
        let install = install_repo_root().expect("test binary lives in a repo");
        std::env::set_var(
            "GSH_WORKSPACE_ROOTS",
            format!("[{:?}]", install.to_string_lossy()),
        );

        let blocked = run_register(&json!({
            "name": "stray-flow",
            "description": "should never be written",
            "command": "echo nope"
        }));
        assert!(blocked.is_err(), "expected the install-repo guard to fire");
        assert!(blocked.unwrap_err().contains("GSH install repo"));

        // The manifest in the install repo must be untouched by the refusal.
        let manifest = manifest_path_in(&install);
        assert!(
            !load_tools_at(&manifest).iter().any(|t| t.name == "stray-flow"),
            "guarded write must not persist the flow"
        );

        // force:true is the documented escape hatch.
        assert!(writable_workspace(true).is_ok());

        std::env::remove_var("GSH_WORKSPACE_ROOTS");
    }
}
