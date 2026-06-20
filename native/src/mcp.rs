//! Pure-Rust MCP stdio server — no Node required.
//!
//! Speaks JSON-RPC (one JSON object per line) over stdin/stdout: `initialize`,
//! `tools/list`, `tools/call`, `ping`, and notifications. It serves exactly the
//! tools in [`crate::registry`], so an agent (Claude Code / Copilot) can register
//! `helpers-native mcp` directly — a single static binary, no Node runtime and no
//! warm daemon. Cold start is ~1ms, so nothing needs to stay warm.
//!
//! Tool handlers resolve their workspace from `$HELPERS_WORKSPACE_ROOTS` (see
//! [`crate::git::workspace_root`]); we set it from the client's `initialize`
//! root so calls run against the right project.

use std::io::{BufRead, Write};

use std::collections::HashSet;

use serde_json::{json, Value};

use crate::git::home;
use crate::registry;
use crate::tools::web;

/// The live control config (`helpers disable` / `helpers tool disable`): the
/// master switch and the set of disabled tool names. Read fresh each request so
/// toggles take effect without restarting the server.
fn tool_config() -> (bool, HashSet<String>) {
    // Diagnostics (e.g. `helpers doctor`) set this to enumerate the full surface
    // regardless of the kill-switch — match the Node server's behavior.
    if std::env::var("HELPERS_FORCE_ENABLE").as_deref() == Ok("1") {
        return (false, HashSet::new());
    }
    let path = home()
        .join(".config")
        .join("helpers-server")
        .join("tools.json");
    let Ok(raw) = std::fs::read_to_string(path) else {
        return (false, HashSet::new());
    };
    let Ok(cfg) = serde_json::from_str::<Value>(&raw) else {
        return (false, HashSet::new());
    };
    let master = cfg.get("disabled").and_then(Value::as_bool).unwrap_or(false);
    let disabled = cfg
        .get("disabledTools")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    (master, disabled)
}

/// Run the MCP server loop until stdin closes (the client disconnected).
pub fn run() -> std::process::ExitCode {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue, // ignore a malformed frame rather than dying
        };
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");

        match method {
            "initialize" => {
                if let Some(roots) = workspace_from_initialize(&msg) {
                    std::env::set_var("HELPERS_WORKSPACE_ROOTS", roots);
                }
                send_result(
                    &mut out,
                    id,
                    json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": { "tools": { "listChanged": true } },
                        "serverInfo": {
                            "name": "helpers",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }),
                );
            }
            // Notifications carry no id and require no response.
            m if m.starts_with("notifications/") => {}
            "tools/list" => {
                // Native registry tools, plus the web tools — which live only in
                // this long-lived server (their human-verified browser reuse needs
                // a persistent process, unlike the per-call `helpers-native call`).
                let mut tools = registry::schemas();
                tools.push(web::schema_search());
                tools.push(web::schema_scrape());
                // Honor the live control surface: master switch hides everything;
                // otherwise drop individually disabled tools.
                let (master, disabled) = tool_config();
                if master {
                    tools.clear();
                } else if !disabled.is_empty() {
                    tools.retain(|t| {
                        t.get("name")
                            .and_then(Value::as_str)
                            .map(|n| !disabled.contains(n))
                            .unwrap_or(true)
                    });
                }
                send_result(&mut out, id, json!({ "tools": tools }));
            }
            "tools/call" => {
                let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
                // A disabled tool (or the master kill-switch) refuses to run unless
                // the caller passes { "force": true } — matching the JS server.
                let forced = args.get("force").and_then(Value::as_bool).unwrap_or(false);
                let (master, disabled) = tool_config();
                if !forced && (master || disabled.contains(name)) {
                    send_error(
                        &mut out,
                        id,
                        -32601,
                        &format!(
                            "tool '{name}' is disabled (helpers {}). Pass {{\"force\": true}} to override.",
                            if master { "is bypassed" } else { "tool disable" }
                        ),
                    );
                    continue;
                }
                let result = match name {
                    "search_web" => Some(web::run_search(&args)),
                    "scrape_webpage" => Some(web::run_scrape(&args)),
                    _ => registry::dispatch(name, &args),
                };
                match result {
                    Some(Ok(content)) => send_result(&mut out, id, json!({ "content": content })),
                    Some(Err(e)) => send_error(&mut out, id, -32603, &e),
                    None => send_error(&mut out, id, -32601, &format!("unknown tool: {name}")),
                }
            }
            "ping" => send_result(&mut out, id, json!({})),
            other => {
                // Only requests (with an id) get an error; ignore unknown notifications.
                if id.is_some() {
                    send_error(&mut out, id, -32601, &format!("method not found: {other}"));
                }
            }
        }
    }
    std::process::ExitCode::SUCCESS
}

/// Extract a filesystem workspace root from an `initialize` message — a
/// VS Code-style `rootUri`/`rootPath`, or the first MCP `roots` entry — and
/// encode it as the JSON array `$HELPERS_WORKSPACE_ROOTS` expects.
fn workspace_from_initialize(msg: &Value) -> Option<String> {
    let params = msg.get("params")?;
    let uri = params
        .get("rootUri")
        .and_then(Value::as_str)
        .or_else(|| params.get("rootPath").and_then(Value::as_str))
        .or_else(|| {
            params
                .get("capabilities")
                .and_then(|c| c.get("roots"))
                .and_then(|r| r.get("0"))
                .and_then(|r| r.get("uri"))
                .and_then(Value::as_str)
        });
    let path = uri?.strip_prefix("file://").unwrap_or(uri?).to_string();
    if path.is_empty() {
        return None;
    }
    Some(json!([path]).to_string())
}

/// Write a JSON-RPC success response.
fn send_result<W: Write>(out: &mut W, id: Option<Value>, result: Value) {
    let msg = json!({ "jsonrpc": "2.0", "id": id, "result": result });
    let _ = writeln!(out, "{}", serde_json::to_string(&msg).unwrap_or_default());
    let _ = out.flush();
}

/// Write a JSON-RPC error response.
fn send_error<W: Write>(out: &mut W, id: Option<Value>, code: i64, message: &str) {
    let msg = json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } });
    let _ = writeln!(out, "{}", serde_json::to_string(&msg).unwrap_or_default());
    let _ = out.flush();
}
