//! CLI linter runners — locate each language's tool, run it under `target`, and
//! return a [`LinterResult`] (or a skip note when the tool or its config is
//! absent). Parsing of each tool's output is delegated to [`super::parsers`].

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::parsers::{
    parse_clippy, parse_eslint, parse_go_vet, parse_mypy, parse_ruff, parse_shellcheck,
    parse_staticcheck, parse_tsc,
};
use super::{Diag, LinterResult};

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

// ─── per-language runners ────────────────────────────────────────────────────

/// Lint JS/TS under `target` with the nearest ESLint (local `node_modules` bin
/// preferred). `None` when no JS/TS files are present; a skip note when ESLint or
/// its config is missing.
pub(super) fn lint_eslint(target: &Path, root: &Path) -> Option<LinterResult> {
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

/// Lint Rust under `target` with `cargo clippy` from the nearest crate root.
pub(super) fn lint_clippy(target: &Path, root: &Path) -> Option<LinterResult> {
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

/// Lint Python under `target` with `ruff check`.
pub(super) fn lint_ruff(target: &Path, root: &Path) -> Option<LinterResult> {
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

/// Lint shell scripts under `target` with ShellCheck (capped at 500 files).
pub(super) fn lint_shellcheck(target: &Path) -> Option<LinterResult> {
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

/// Type-check TS under `target` with `tsc --noEmit`; skipped for file-scoped
/// runs (tsc needs the whole project).
pub(super) fn lint_tsc(target: &Path, root: &Path, scope_is_file: bool) -> Option<LinterResult> {
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

/// Type-check Python under `target` with mypy, only when mypy is configured.
pub(super) fn lint_mypy(target: &Path, root: &Path) -> Option<LinterResult> {
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

/// Lint Go under `target` with `go vet` and (when present) `staticcheck`.
pub(super) fn lint_go(target: &Path, root: &Path) -> Option<LinterResult> {
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

/// A `LinterResult` carrying only a skip note (tool or config absent).
fn skipped(msg: &str) -> LinterResult {
    LinterResult {
        tools: Vec::new(),
        skipped: Some(msg.to_string()),
        diagnostics: Vec::new(),
    }
}

/// A `LinterResult` for a tool that ran and produced `diagnostics`.
fn ran(tool: &'static str, diagnostics: Vec<Diag>) -> LinterResult {
    LinterResult {
        tools: vec![tool],
        skipped: None,
        diagnostics,
    }
}
