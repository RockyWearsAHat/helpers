//! `strict_lint` — port of `lib/mcp-strict-lint.js` + `-standalone.js`.
//!
//! Primary path: VS Code's live diagnostics over a unix socket (every installed
//! language server). Fallback: run the project's own tooling (eslint, tsc, ruff,
//! mypy, cargo clippy, go vet/staticcheck, shellcheck) and unify the output, so
//! non-VS-Code agents still get each provider's best-practice diagnostics.
//!
//! The pure output parsers live in [`parsers`]; the CLI runners that locate and
//! invoke each tool live in [`runners`]. This module owns the shared diagnostic
//! model, the standalone orchestration, the report formatter, and the VS Code
//! IPC path.

mod parsers;
mod runners;

#[cfg(unix)]
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::Duration;

use serde_json::{json, Value};

#[cfg(unix)]
use crate::git::home;
use crate::proto::{text, ToolResult};

use runners::{
    lint_clippy, lint_eslint, lint_go, lint_mypy, lint_ruff, lint_shellcheck, lint_tsc,
};

/// MCP tool schema for `strict_lint`.
pub fn schema() -> Value {
    json!({
        "name": "strict_lint",
        "description": "Run strict diagnostics on a file, folder, or the whole workspace and report errors, warnings, AND best-practice hints. Inside VS Code it returns the live Problems panel (every installed language server). Elsewhere it runs the project's own tooling — eslint + tsc, ruff + mypy, cargo clippy, go vet + staticcheck, shellcheck — so you get each language provider's current best-practice recommendations with their rule ids. Call after every edit before declaring work complete; fix reported issues (or document why a warning is acceptable), and treat each rule as a principle to apply going forward.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "filePath": { "type": "string", "description": "Absolute path to a specific file to check. Omit to check the whole workspace." },
                "folderPath": { "type": "string", "description": "Absolute path to a folder to check. Omit to check the whole workspace." },
                "severityFilter": { "type": "string", "enum": ["all", "errors-only", "warnings-and-above"], "description": "Which severity levels to include. 'all' includes hint/style recommendations. Defaults to 'all'." }
            },
            "required": []
        }
    })
}

// ─── Diagnostic model ───────────────────────────────────────────────────────

/// One unified diagnostic from any provider (file:line:col + severity/rule).
#[derive(Clone)]
pub struct Diag {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub severity: Sev,
    pub rule: String,
    pub message: String,
    pub tool: &'static str,
}

/// Diagnostic severity, ordered Error → Warning → Hint for sorting/grouping.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sev {
    Error,
    Warning,
    Hint,
}

impl Sev {
    /// Sort key (0 = Error, highest priority).
    fn order(self) -> u8 {
        match self {
            Sev::Error => 0,
            Sev::Warning => 1,
            Sev::Hint => 2,
        }
    }
    /// Section heading used in the rendered report.
    fn label(self) -> &'static str {
        match self {
            Sev::Error => "ERRORS",
            Sev::Warning => "WARNINGS",
            Sev::Hint => "HINTS / RECOMMENDATIONS",
        }
    }
}

/// The outcome of running one provider: which tool(s) ran, an optional skip
/// note, and the diagnostics produced.
struct LinterResult {
    tools: Vec<&'static str>,
    skipped: Option<String>,
    diagnostics: Vec<Diag>,
}

// ─── standalone orchestration ───────────────────────────────────────────────

/// Run every applicable CLI linter for `args`' target and render the report.
fn run_standalone(args: &Value) -> String {
    let file_path = args
        .get("filePath")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    let folder_path = args
        .get("folderPath")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    let target_str = file_path
        .or(folder_path)
        .map(|s| s.to_string())
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| ".".to_string());
    let target = PathBuf::from(&target_str);
    if !target.exists() {
        return format!(
            "strict_lint (standalone): target not found: {}",
            target.display()
        );
    }
    let scope_is_file = file_path.is_some() && target.is_file();
    let root = if scope_is_file {
        target
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| target.clone())
    } else {
        target.clone()
    };
    let filter = args
        .get("severityFilter")
        .and_then(Value::as_str)
        .unwrap_or("all");

    let mut tools_ran: Vec<&'static str> = Vec::new();
    let mut skipped_list: Vec<String> = Vec::new();
    let mut diagnostics: Vec<Diag> = Vec::new();

    let results = [
        lint_eslint(&target, &root),
        lint_tsc(&target, &root, scope_is_file),
        lint_ruff(&target, &root),
        lint_mypy(&target, &root),
        lint_clippy(&target, &root),
        lint_go(&target, &root),
        lint_shellcheck(&target),
    ];
    for res in results.into_iter().flatten() {
        for t in res.tools {
            if !tools_ran.contains(&t) {
                tools_ran.push(t);
            }
        }
        if let Some(s) = res.skipped {
            skipped_list.push(s);
        }
        diagnostics.extend(res.diagnostics);
    }

    match filter {
        "errors-only" => diagnostics.retain(|d| d.severity == Sev::Error),
        "warnings-and-above" => diagnostics.retain(|d| d.severity != Sev::Hint),
        _ => {}
    }

    format_report(&target, &tools_ran, &skipped_list, &mut diagnostics, filter)
}

/// Render diagnostics grouped by severity, with provider/skip summary lines.
fn format_report(
    target: &Path,
    ran: &[&str],
    skipped: &[String],
    diagnostics: &mut [Diag],
    filter: &str,
) -> String {
    let mut counts = [0usize; 3];
    for d in diagnostics.iter() {
        counts[d.severity.order() as usize] += 1;
    }
    let mut lines = vec![
        format!("strict_lint (standalone) — {}", target.display()),
        format!(
            "providers run: {}{}",
            if ran.is_empty() {
                "none".to_string()
            } else {
                ran.join(", ")
            },
            if skipped.is_empty() {
                String::new()
            } else {
                format!("  |  skipped: {}", skipped.join(", "))
            }
        ),
    ];

    if ran.is_empty() && skipped.is_empty() {
        lines.push(String::new());
        lines.push(
            "No language tooling matched this target. Install a provider to lint here:".to_string(),
        );
        lines.push("  JS/TS → eslint + typescript | Python → ruff | Rust → clippy | Go → staticcheck | Shell → shellcheck".to_string());
        return lines.join("\n");
    }

    if diagnostics.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "✓ Clean — 0 {} from: {}.",
            if filter == "all" {
                "errors/warnings/hints"
            } else {
                filter
            },
            ran.join(", ")
        ));
        if !skipped.is_empty() {
            lines.push(format!(
                "(Some providers were skipped: {}.)",
                skipped.join(", ")
            ));
        }
        return lines.join("\n");
    }

    diagnostics.sort_by(|a, b| {
        a.severity
            .order()
            .cmp(&b.severity.order())
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });

    let mut current: Option<Sev> = None;
    for d in diagnostics.iter() {
        if current != Some(d.severity) {
            current = Some(d.severity);
            lines.push(String::new());
            lines.push(format!(
                "{} ({})",
                d.severity.label(),
                counts[d.severity.order() as usize]
            ));
        }
        let rule = if d.rule.is_empty() {
            format!(" [{}]", d.tool)
        } else {
            format!(" [{}:{}]", d.tool, d.rule)
        };
        lines.push(format!(
            "  {}:{}:{}{} {}",
            shorten(&d.file),
            d.line,
            d.col,
            rule,
            d.message
        ));
    }

    lines.push(String::new());
    lines.push(format!(
        "Summary: {} error(s), {} warning(s), {} hint(s).",
        counts[0], counts[1], counts[2]
    ));
    lines.push("Each rule id is a best-practice principle from the language's own tooling — fix it and apply the principle going forward, don't just silence it.".to_string());
    lines.join("\n")
}

/// Shorten a path to be workspace-relative for display when possible.
fn shorten(file: &str) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(rel) = Path::new(file).strip_prefix(&cwd) {
            return rel.to_string_lossy().to_string();
        }
    }
    file.to_string()
}

// ─── VS Code IPC primary path ───────────────────────────────────────────────

/// Path to the JSON file advertising VS Code's strict-lint IPC socket.
/// Unix-only: it is consulted solely by the `#[cfg(unix)]` `try_ipc`.
#[cfg(unix)]
fn ipc_info_path() -> PathBuf {
    home()
        .join(".cache")
        .join("helpers")
        .join("strict-lint-ipc.json")
}

/// VS Code IPC outcome: a rendered result, or an error (with whether the error
/// indicates no diagnostics provider was active). The type is referenced by
/// `run` on every platform, but its variants are only constructed by the
/// `#[cfg(unix)]` `try_ipc`, so non-Unix builds never instantiate them.
#[cfg_attr(not(unix), allow(dead_code))]
enum Ipc {
    Ok(String),
    Err {
        provider_inactive: bool,
        text: String,
    },
}

/// VS Code's diagnostics bridge is a unix-domain socket, so on non-Unix targets
/// (e.g. Windows) there is no IPC path: return `None` so `run` cleanly falls
/// back to the standalone CLI linters. This stub also keeps the crate free of
/// any `std::os::unix` reference on those targets.
#[cfg(not(unix))]
fn try_ipc(_args: &Value) -> Option<Ipc> {
    None
}

/// Try VS Code's diagnostics over its unix socket; `None` if unreachable.
///
/// Unix-only: it connects to the advertised `UnixStream`. The non-Unix build
/// uses the [`#[cfg(not(unix))]`](try_ipc) stub above, which always returns
/// `None`.
#[cfg(unix)]
fn try_ipc(args: &Value) -> Option<Ipc> {
    let info = std::fs::read_to_string(ipc_info_path()).ok()?;
    let socket_path = serde_json::from_str::<Value>(&info)
        .ok()?
        .get("socketPath")?
        .as_str()?
        .to_string();
    if socket_path.is_empty() {
        return None;
    }
    let mut stream = std::os::unix::net::UnixStream::connect(&socket_path).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(15)));
    let req = json!({ "arguments": args }).to_string() + "\n";
    stream.write_all(req.as_bytes()).ok()?;
    let mut buf = String::new();
    stream.read_to_string(&mut buf).ok()?;
    // VS Code reports this when it has no linter/language server active for the
    // target — in that case the CLI providers are strictly better than nothing.
    let provider_inactive_re = regex::Regex::new(
        r"(?i)no diagnostics provider|requires an active|provider .*not active|no .*provider activity",
    )
    .unwrap();
    for line in buf.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(resp) = serde_json::from_str::<Value>(line) {
            if resp.get("ok").and_then(Value::as_bool) == Some(true) {
                return Some(Ipc::Ok(
                    resp.get("result")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                ));
            }
            let err = resp.get("error").and_then(Value::as_str).unwrap_or("");
            return Some(Ipc::Err {
                provider_inactive: provider_inactive_re.is_match(err),
                text: format!("strict_lint error: {err}"),
            });
        }
    }
    None
}

/// Run `strict_lint`: prefer VS Code's live diagnostics over IPC, falling back to
/// the standalone CLI linters (and noting when VS Code had no active provider).
pub fn run(args: &Value) -> ToolResult {
    let ipc = try_ipc(args);
    if let Some(Ipc::Ok(t)) = &ipc {
        return Ok(vec![text(t.clone())]);
    }

    let cli_text = run_standalone(args);
    let cli_found = !cli_text.contains("providers run: none");
    if cli_found {
        let note = matches!(&ipc, Some(Ipc::Err { provider_inactive: true, .. }))
            .then(|| "[VS Code had no active diagnostics provider for this target — used the language's CLI tooling instead]\n\n".to_string())
            .unwrap_or_default();
        return Ok(vec![text(format!("{note}{cli_text}"))]);
    }
    if let Some(Ipc::Err { text: t, .. }) = ipc {
        return Ok(vec![text(t)]);
    }
    Ok(vec![text(cli_text)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_clean_report() {
        let mut diags: Vec<Diag> = Vec::new();
        let out = format_report(Path::new("/p"), &["eslint"], &[], &mut diags, "all");
        assert!(out.contains("✓ Clean"));
        assert!(out.contains("providers run: eslint"));
    }
}
