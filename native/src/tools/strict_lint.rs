//! `strict_lint` — port of `lib/mcp-strict-lint.js` + `-standalone.js`.
//!
//! Primary path: VS Code's live diagnostics over a unix socket (every installed
//! language server). Fallback: run the project's own tooling (eslint, tsc, ruff,
//! mypy, cargo clippy, go vet/staticcheck, shellcheck) and unify the output, so
//! non-VS-Code agents still get each provider's best-practice diagnostics.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

use crate::git::home;
use crate::proto::{text, ToolResult};

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sev {
    Error,
    Warning,
    Hint,
}

impl Sev {
    fn order(self) -> u8 {
        match self {
            Sev::Error => 0,
            Sev::Warning => 1,
            Sev::Hint => 2,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Sev::Error => "ERRORS",
            Sev::Warning => "WARNINGS",
            Sev::Hint => "HINTS / RECOMMENDATIONS",
        }
    }
}

fn diag(
    file: String,
    line: u32,
    col: u32,
    severity: Sev,
    rule: &str,
    message: &str,
    tool: &'static str,
) -> Diag {
    Diag {
        file,
        line,
        col,
        severity,
        rule: rule.to_string(),
        message: message.trim().to_string(),
        tool,
    }
}

struct LinterResult {
    tools: Vec<&'static str>,
    skipped: Option<String>,
    diagnostics: Vec<Diag>,
}

// ─── tool/process helpers ───────────────────────────────────────────────────

/// Whether `cmd` is on PATH.
fn on_path(cmd: &str) -> bool {
    run_capture("sh", &["-c", &format!("command -v {cmd}")], None, 5).0
}

/// Run a linter subprocess with a bounded timeout (delegates to the shared
/// process helper). Returns `(success, stdout, stderr)`.
fn run_capture(
    cmd: &str,
    args: &[&str],
    cwd: Option<&Path>,
    timeout_s: u64,
) -> (bool, String, String) {
    crate::proc::run_capture(cmd, args, cwd, &[], timeout_s)
}

const IGNORE_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "build",
    "out",
    "bin",
    "dist",
    ".venv",
    "venv",
    "__pycache__",
    ".gradle",
    ".idea",
    ".cache",
];

/// Files under `target` whose lowercased name ends with one of `exts`.
fn list_files(target: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let matches = |p: &Path| {
        let name = p.to_string_lossy().to_lowercase();
        exts.iter().any(|e| name.ends_with(e))
    };
    let meta = match std::fs::metadata(target) {
        Ok(m) => m,
        Err(_) => return out,
    };
    if meta.is_file() {
        if matches(target) {
            out.push(target.to_path_buf());
        }
        return out;
    }
    let mut stack = vec![target.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let p = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                if IGNORE_DIRS.contains(&name.as_str()) || name.starts_with('.') {
                    continue;
                }
                stack.push(p);
            } else if matches(&p) {
                out.push(p);
            }
        }
    }
    out
}

/// Nearest ancestor of `start` (inclusive) containing any of `names`.
fn find_up(start: &Path, names: &[&str]) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    for _ in 0..40 {
        if names.iter().any(|n| dir.join(n).exists()) {
            return Some(dir);
        }
        match dir.parent() {
            Some(p) if p != dir => dir = p.to_path_buf(),
            _ => break,
        }
    }
    None
}

/// Nearest `node_modules/.bin/<bin>` walking up from `root`.
fn local_bin(root: &Path, bin: &str) -> Option<PathBuf> {
    let mut dir = root.to_path_buf();
    for _ in 0..40 {
        let p = dir.join("node_modules").join(".bin").join(bin);
        if p.exists() {
            return Some(p);
        }
        match dir.parent() {
            Some(parent) if parent != dir => dir = parent.to_path_buf(),
            _ => break,
        }
    }
    None
}

fn resolve(base: &Path, rel: &str) -> String {
    let p = Path::new(rel);
    if p.is_absolute() {
        rel.to_string()
    } else {
        base.join(rel).to_string_lossy().to_string()
    }
}

// ─── pure output parsers (unit-tested without the linters installed) ─────────

pub fn parse_eslint(stdout: &str) -> Vec<Diag> {
    let mut diags = Vec::new();
    let files: Value = serde_json::from_str(stdout.trim()).unwrap_or(Value::Null);
    if let Some(arr) = files.as_array() {
        for f in arr {
            let path = f.get("filePath").and_then(Value::as_str).unwrap_or("");
            for m in f
                .get("messages")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let sev = if m.get("severity").and_then(Value::as_i64) == Some(2) {
                    Sev::Error
                } else {
                    Sev::Warning
                };
                diags.push(diag(
                    path.to_string(),
                    m.get("line").and_then(Value::as_u64).unwrap_or(0) as u32,
                    m.get("column").and_then(Value::as_u64).unwrap_or(0) as u32,
                    sev,
                    m.get("ruleId").and_then(Value::as_str).unwrap_or(""),
                    m.get("message").and_then(Value::as_str).unwrap_or(""),
                    "eslint",
                ));
            }
        }
    }
    diags
}

pub fn parse_clippy(stdout: &str, base: &Path) -> Vec<Diag> {
    let mut diags = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let m: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if m.get("reason").and_then(Value::as_str) != Some("compiler-message") {
            continue;
        }
        let msg = match m.get("message") {
            Some(v) => v,
            None => continue,
        };
        let level = msg.get("level").and_then(Value::as_str).unwrap_or("");
        let sev = match level {
            "error" => Sev::Error,
            "warning" => Sev::Warning,
            _ => continue,
        };
        let spans = msg
            .get("spans")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let span = spans
            .iter()
            .find(|s| s.get("is_primary").and_then(Value::as_bool) == Some(true))
            .or_else(|| spans.first());
        let span = match span {
            Some(s) => s,
            None => continue,
        };
        let code = msg
            .get("code")
            .and_then(|c| c.get("code"))
            .and_then(Value::as_str)
            .unwrap_or("");
        diags.push(diag(
            resolve(
                base,
                span.get("file_name").and_then(Value::as_str).unwrap_or(""),
            ),
            span.get("line_start").and_then(Value::as_u64).unwrap_or(0) as u32,
            span.get("column_start")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            sev,
            code,
            msg.get("message").and_then(Value::as_str).unwrap_or(""),
            "clippy",
        ));
    }
    diags
}

pub fn parse_shellcheck(stdout: &str) -> Vec<Diag> {
    let mut diags = Vec::new();
    let arr: Value = serde_json::from_str(stdout.trim()).unwrap_or(Value::Null);
    for v in arr.as_array().into_iter().flatten() {
        let sev = match v.get("level").and_then(Value::as_str).unwrap_or("") {
            "error" => Sev::Error,
            "warning" => Sev::Warning,
            _ => Sev::Hint,
        };
        let code = format!("SC{}", v.get("code").and_then(Value::as_u64).unwrap_or(0));
        diags.push(diag(
            v.get("file")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            v.get("line").and_then(Value::as_u64).unwrap_or(0) as u32,
            v.get("column").and_then(Value::as_u64).unwrap_or(0) as u32,
            sev,
            &code,
            v.get("message").and_then(Value::as_str).unwrap_or(""),
            "shellcheck",
        ));
    }
    diags
}

pub fn parse_ruff(stdout: &str) -> Vec<Diag> {
    let mut diags = Vec::new();
    let arr: Value = serde_json::from_str(stdout.trim()).unwrap_or(Value::Null);
    for v in arr.as_array().into_iter().flatten() {
        diags.push(diag(
            v.get("filename")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            v.get("location")
                .and_then(|l| l.get("row"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            v.get("location")
                .and_then(|l| l.get("column"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            Sev::Warning,
            v.get("code").and_then(Value::as_str).unwrap_or(""),
            v.get("message").and_then(Value::as_str).unwrap_or(""),
            "ruff",
        ));
    }
    diags
}

pub fn parse_tsc(stdout: &str, base: &Path) -> Vec<Diag> {
    let re =
        regex::Regex::new(r"^(.+?)\((\d+),(\d+)\):\s+(error|warning)\s+(TS\d+):\s+(.*)$").unwrap();
    let mut diags = Vec::new();
    for line in stdout.lines() {
        if let Some(c) = re.captures(line) {
            let sev = if &c[4] == "error" {
                Sev::Error
            } else {
                Sev::Warning
            };
            diags.push(diag(
                resolve(base, &c[1]),
                c[2].parse().unwrap_or(0),
                c[3].parse().unwrap_or(0),
                sev,
                &c[5],
                &c[6],
                "tsc",
            ));
        }
    }
    diags
}

pub fn parse_mypy(stdout: &str, base: &Path) -> Vec<Diag> {
    let re = regex::Regex::new(
        r"^(.+?):(\d+):(?:(\d+):)?\s+(error|note|warning):\s+(.*?)(?:\s+\[([\w-]+)\])?$",
    )
    .unwrap();
    let mut diags = Vec::new();
    for line in stdout.lines() {
        if let Some(c) = re.captures(line) {
            let sev = match &c[4] {
                "error" => Sev::Error,
                "warning" => Sev::Warning,
                _ => Sev::Hint,
            };
            diags.push(diag(
                resolve(base, &c[1]),
                c[2].parse().unwrap_or(0),
                c.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
                sev,
                c.get(6).map(|m| m.as_str()).unwrap_or(""),
                c.get(5).map(|m| m.as_str()).unwrap_or(""),
                "mypy",
            ));
        }
    }
    diags
}

pub fn parse_go_vet(stderr: &str, base: &Path) -> Vec<Diag> {
    let re = regex::Regex::new(r"^(.+?\.go):(\d+):(?:(\d+):)?\s+(.*)$").unwrap();
    let mut diags = Vec::new();
    for line in stderr.lines() {
        if let Some(c) = re.captures(line.trim()) {
            diags.push(diag(
                resolve(base, &c[1]),
                c[2].parse().unwrap_or(0),
                c.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
                Sev::Warning,
                "vet",
                &c[4],
                "go vet",
            ));
        }
    }
    diags
}

pub fn parse_staticcheck(stdout: &str) -> Vec<Diag> {
    let mut diags = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let m: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let loc = match m.get("location") {
            Some(l) => l,
            None => continue,
        };
        let sev = if m.get("severity").and_then(Value::as_str) == Some("error") {
            Sev::Error
        } else {
            Sev::Warning
        };
        diags.push(diag(
            loc.get("file")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            loc.get("line").and_then(Value::as_u64).unwrap_or(0) as u32,
            loc.get("column").and_then(Value::as_u64).unwrap_or(0) as u32,
            sev,
            m.get("code").and_then(Value::as_str).unwrap_or(""),
            m.get("message").and_then(Value::as_str).unwrap_or(""),
            "staticcheck",
        ));
    }
    diags
}

// ─── linter runners ─────────────────────────────────────────────────────────

fn lint_eslint(target: &Path, root: &Path) -> Option<LinterResult> {
    let exts = [".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs"];
    if list_files(target, &exts).is_empty() {
        return None;
    }
    let config_dir = find_up(
        root,
        &[
            "eslint.config.js",
            "eslint.config.mjs",
            "eslint.config.cjs",
            "eslint.config.ts",
            ".eslintrc.js",
            ".eslintrc.cjs",
            ".eslintrc.json",
            ".eslintrc.yml",
            ".eslintrc.yaml",
            ".eslintrc",
        ],
    );
    let bin = local_bin(root, "eslint")
        .map(|p| p.to_string_lossy().to_string())
        .or_else(|| on_path("eslint").then(|| "eslint".to_string()));
    let bin = match bin {
        Some(b) => b,
        None => return Some(skipped("eslint (not installed)")),
    };
    let config_dir = match config_dir {
        Some(d) => d,
        None => return Some(skipped("eslint (no config found)")),
    };
    let rel_target = target
        .strip_prefix(&config_dir)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());
    let rel_target = if rel_target.is_empty() {
        ".".to_string()
    } else {
        rel_target
    };
    let (_, stdout, stderr) = run_capture(
        &bin,
        &[
            "--format",
            "json",
            "--no-error-on-unmatched-pattern",
            &rel_target,
        ],
        Some(&config_dir),
        60,
    );
    if serde_json::from_str::<Value>(stdout.trim()).is_err() && !stderr.trim().is_empty() {
        return Some(skipped(&format!(
            "eslint (error: {})",
            stderr.trim().lines().next().unwrap_or("")
        )));
    }
    Some(ran("eslint", parse_eslint(&stdout)))
}

fn lint_clippy(target: &Path, root: &Path) -> Option<LinterResult> {
    let cargo_dir = find_up(root, &["Cargo.toml"])?;
    if list_files(target, &[".rs"]).is_empty() {
        return None;
    }
    if !on_path("cargo") {
        return Some(skipped("clippy (cargo not installed)"));
    }
    let (_, stdout, _) = run_capture(
        "cargo",
        &["clippy", "--message-format=json", "-q"],
        Some(&cargo_dir),
        180,
    );
    Some(ran("clippy", parse_clippy(&stdout, &cargo_dir)))
}

fn lint_ruff(target: &Path, root: &Path) -> Option<LinterResult> {
    if list_files(target, &[".py", ".pyi"]).is_empty() {
        return None;
    }
    if !on_path("ruff") {
        return Some(skipped("ruff (not installed — `pip install ruff`)"));
    }
    let (_, stdout, _) = run_capture(
        "ruff",
        &[
            "check",
            "--output-format",
            "json",
            "--force-exclude",
            &target.to_string_lossy(),
        ],
        Some(root),
        60,
    );
    Some(ran("ruff", parse_ruff(&stdout)))
}

fn lint_shellcheck(target: &Path) -> Option<LinterResult> {
    let files = list_files(target, &[".sh", ".bash"]);
    if files.is_empty() {
        return None;
    }
    if !on_path("shellcheck") {
        return Some(skipped("shellcheck (not installed)"));
    }
    let mut args = vec!["-f".to_string(), "json".to_string()];
    for f in files.iter().take(500) {
        args.push(f.to_string_lossy().to_string());
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (_, stdout, _) = run_capture("shellcheck", &arg_refs, None, 60);
    Some(ran("shellcheck", parse_shellcheck(&stdout)))
}

fn lint_tsc(target: &Path, root: &Path, scope_is_file: bool) -> Option<LinterResult> {
    if scope_is_file {
        return None;
    }
    let ts_root = find_up(root, &["tsconfig.json"])?;
    if list_files(target, &[".ts", ".tsx"]).is_empty() {
        return None;
    }
    let bin = local_bin(root, "tsc")
        .map(|p| p.to_string_lossy().to_string())
        .or_else(|| on_path("tsc").then(|| "tsc".to_string()));
    let bin = match bin {
        Some(b) => b,
        None => return Some(skipped("tsc (not installed)")),
    };
    let (_, stdout, _) = run_capture(
        &bin,
        &["--noEmit", "--pretty", "false"],
        Some(&ts_root),
        120,
    );
    Some(ran("tsc", parse_tsc(&stdout, &ts_root)))
}

fn lint_mypy(target: &Path, root: &Path) -> Option<LinterResult> {
    if list_files(target, &[".py"]).is_empty() {
        return None;
    }
    // Only run when explicitly configured (avoids noise).
    let cfg_dir = find_up(root, &["mypy.ini", ".mypy.ini"]).or_else(|| {
        find_up(root, &["pyproject.toml"]).filter(|d| {
            std::fs::read_to_string(d.join("pyproject.toml"))
                .map(|s| s.contains("[tool.mypy]"))
                .unwrap_or(false)
        })
    })?;
    if !on_path("mypy") {
        return Some(skipped("mypy (not installed)"));
    }
    let (_, stdout, _) = run_capture(
        "mypy",
        &[
            "--no-error-summary",
            "--show-error-codes",
            "--no-color-output",
            &target.to_string_lossy(),
        ],
        Some(&cfg_dir),
        120,
    );
    Some(ran("mypy", parse_mypy(&stdout, &cfg_dir)))
}

fn lint_go(target: &Path, root: &Path) -> Option<LinterResult> {
    let go_root = find_up(root, &["go.mod"])?;
    if list_files(target, &[".go"]).is_empty() {
        return None;
    }
    let mut tools: Vec<&'static str> = Vec::new();
    let mut skipped_msg = None;
    let mut diagnostics = Vec::new();
    if on_path("go") {
        let (_, _, stderr) = run_capture("go", &["vet", "./..."], Some(&go_root), 120);
        tools.push("go vet");
        diagnostics.extend(parse_go_vet(&stderr, &go_root));
    } else {
        skipped_msg = Some("go vet (go not installed)".to_string());
    }
    if on_path("staticcheck") {
        let (_, stdout, _) =
            run_capture("staticcheck", &["-f", "json", "./..."], Some(&go_root), 120);
        tools.push("staticcheck");
        diagnostics.extend(parse_staticcheck(&stdout));
    }
    Some(LinterResult {
        tools,
        skipped: skipped_msg,
        diagnostics,
    })
}

fn skipped(msg: &str) -> LinterResult {
    LinterResult {
        tools: Vec::new(),
        skipped: Some(msg.to_string()),
        diagnostics: Vec::new(),
    }
}

fn ran(tool: &'static str, diagnostics: Vec<Diag>) -> LinterResult {
    LinterResult {
        tools: vec![tool],
        skipped: None,
        diagnostics,
    }
}

// ─── standalone orchestration ───────────────────────────────────────────────

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

fn shorten(file: &str) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(rel) = Path::new(file).strip_prefix(&cwd) {
            return rel.to_string_lossy().to_string();
        }
    }
    file.to_string()
}

// ─── VS Code IPC primary path ───────────────────────────────────────────────

fn ipc_info_path() -> PathBuf {
    home()
        .join(".cache")
        .join("gsh")
        .join("strict-lint-ipc.json")
}

enum Ipc {
    Ok(String),
    Err {
        provider_inactive: bool,
        text: String,
    },
}

/// Try VS Code's diagnostics over its unix socket; `None` if unreachable.
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
    fn parses_eslint_json() {
        let json = r#"[{"filePath":"/p/a.js","messages":[{"severity":2,"line":3,"column":5,"ruleId":"no-unused-vars","message":"x is unused"},{"severity":1,"line":7,"column":1,"ruleId":"eqeqeq","message":"use ==="}]}]"#;
        let d = parse_eslint(json);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].severity, Sev::Error);
        assert_eq!(d[0].rule, "no-unused-vars");
        assert_eq!(d[1].severity, Sev::Warning);
    }

    #[test]
    fn parses_clippy_json() {
        let line = r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused variable","code":{"code":"unused_variables"},"spans":[{"is_primary":true,"file_name":"src/x.rs","line_start":4,"column_start":9}]}}"#;
        let d = parse_clippy(line, Path::new("/proj"));
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].severity, Sev::Warning);
        assert_eq!(d[0].rule, "unused_variables");
        assert_eq!(d[0].file, "/proj/src/x.rs");
        assert_eq!(d[0].line, 4);
    }

    #[test]
    fn parses_shellcheck_and_tsc() {
        let sc = r#"[{"file":"a.sh","line":2,"column":1,"level":"warning","code":2086,"message":"Double quote"}]"#;
        let d = parse_shellcheck(sc);
        assert_eq!(d[0].rule, "SC2086");
        let tsc = "src/x.ts(10,5): error TS2304: Cannot find name 'foo'.";
        let d2 = parse_tsc(tsc, Path::new("/proj"));
        assert_eq!(d2.len(), 1);
        assert_eq!(d2[0].severity, Sev::Error);
        assert_eq!(d2[0].rule, "TS2304");
        assert_eq!(d2[0].file, "/proj/src/x.ts");
    }

    #[test]
    fn formats_clean_report() {
        let mut diags: Vec<Diag> = Vec::new();
        let out = format_report(Path::new("/p"), &["eslint"], &[], &mut diags, "all");
        assert!(out.contains("✓ Clean"));
        assert!(out.contains("providers run: eslint"));
    }
}
