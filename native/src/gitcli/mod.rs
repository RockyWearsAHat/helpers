//! Native Rust ports of the standalone `git-*` shell CLIs.
//!
//! These are invoked busybox-style: the `helpers-native` binary is symlinked to
//! each CLI name (`git-resolve`, `git-remerge`, …) and dispatches on the
//! basename of `argv[0]`. They can also be run explicitly as
//! `helpers-native gitcli <name> [args…]`.
//!
//! Every CLI here is deterministic (no AI) except `git-upload`, which is
//! deterministic by default and offers an opt-in AI commit-message path.

use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

pub mod checkpoint;
pub mod fucked_push;
pub mod get;
pub mod initialize;
pub mod pushed_env;
pub mod remerge;
pub mod resolve;
pub mod scan_envs;
pub mod upload;

/// The CLI names this multiplexer recognises by `argv[0]` basename.
pub const CLI_NAMES: &[&str] = &[
    "git-resolve",
    "git-remerge",
    "git-fucked-the-push",
    "git-initialize",
    "git-get",
    "git-scan-for-leaked-envs",
    "git-upload",
    "git-checkpoint",
    "git-help-i-pushed-an-env",
];

/// Returns `true` when `basename` names one of the ported CLIs.
pub fn is_cli(basename: &str) -> bool {
    CLI_NAMES.contains(&basename)
}

/// Dispatch a ported CLI by name. `args` are the arguments after the program
/// name. Unknown names return exit code 2.
pub fn dispatch(name: &str, args: &[String]) -> ExitCode {
    match name {
        "git-resolve" => resolve::run(args),
        "git-remerge" => remerge::run(args),
        "git-fucked-the-push" => fucked_push::run(args),
        "git-initialize" => initialize::run(args),
        "git-get" => get::run(args),
        "git-scan-for-leaked-envs" => scan_envs::run(args),
        "git-upload" => upload::run(args),
        "git-checkpoint" => checkpoint::run(args),
        "git-help-i-pushed-an-env" => pushed_env::run(args),
        other => {
            eprintln!("helpers-native gitcli: unknown CLI: {other}");
            ExitCode::from(2)
        }
    }
}

// ── Shared command helpers ───────────────────────────────────────────────

/// Run `git <args>` inheriting the parent's stdio (for interactive/streamed
/// output). Returns `true` on a zero exit status.
pub fn git_inherit(args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run `git <args>` silently (stdout+stderr discarded). Returns success.
pub fn git_ok(args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run `git <args>` capturing stdout. Returns trimmed stdout, or `None` when
/// the command fails or cannot be spawned.
pub fn git_out(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

/// Run any command capturing `(success, stdout, stderr)`, all trimmed.
pub fn run_capture(cmd: &str, args: &[&str]) -> (bool, String, String) {
    match Command::new(cmd).args(args).output() {
        Ok(o) => (
            o.status.success(),
            String::from_utf8_lossy(&o.stdout).trim().to_string(),
            String::from_utf8_lossy(&o.stderr).trim().to_string(),
        ),
        Err(e) => (false, String::new(), e.to_string()),
    }
}

/// True when the current directory is inside a git work tree.
pub fn in_repo() -> bool {
    git_ok(&["rev-parse", "--git-dir"])
}

/// The absolute path to the resolved `--git-dir`, defaulting to `.git`.
pub fn git_dir() -> String {
    git_out(&["rev-parse", "--git-dir"]).unwrap_or_else(|| ".git".to_string())
}

/// Current branch name via `symbolic-ref` (empty string when detached).
pub fn current_branch() -> String {
    git_out(&["symbolic-ref", "-q", "--short", "HEAD"]).unwrap_or_default()
}

/// A `YYYYMMDD-HHMMSS` local timestamp for backup-branch naming.
pub fn timestamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

/// Print one `[tag] message` line to stderr, matching the shell CLIs' style.
pub fn note(tag: &str, msg: &str) {
    eprintln!("[{tag}] {msg}");
}

/// Whether `path` exists (file or dir) under the git dir.
pub fn git_dir_has(rel: &str) -> bool {
    Path::new(&git_dir()).join(rel).exists()
}

// ── Shared commit-message helpers (used by git-upload and git-checkpoint) ──

/// Deterministic "update N files (a, b, c)" summary of the staged change, plus
/// a short stat. No AI — mirrors the `checkpoint` MCP tool.
pub fn staged_summary_message(verb: &str) -> String {
    let names = git_out(&["diff", "--cached", "--name-only"]).unwrap_or_default();
    let files: Vec<&str> = names.lines().filter(|l| !l.is_empty()).collect();
    if files.is_empty() {
        return verb.to_string();
    }
    let shown: Vec<&str> = files.iter().take(3).copied().collect();
    let mut list = shown.join(", ");
    if files.len() > shown.len() {
        list.push_str(&format!(", +{} more", files.len() - shown.len()));
    }
    let stat = git_out(&["diff", "--cached", "--shortstat"]).unwrap_or_default();
    let plural = if files.len() == 1 { "" } else { "s" };
    let mut msg = format!("{verb} {} file{plural} ({list})", files.len());
    if !stat.is_empty() {
        msg.push_str(&format!("\n\n{}", stat.trim()));
    }
    msg
}

/// Generate a commit message via Claude or Copilot from the staged diff.
/// Returns the first non-empty lines of output, or `None` when the provider is
/// missing or fails. `tag` is the CLI name for log lines.
pub fn ai_commit_message(provider: Option<&str>, extra_context: &str, tag: &str) -> Option<String> {
    let stat = git_out(&["diff", "--cached", "--stat"]).unwrap_or_default();
    let names = git_out(&["diff", "--cached", "--name-only"]).unwrap_or_default();
    let mut prompt = String::from(
        "Write a clean, concise one-to-three line git commit message for this staged diff. \
         Output only the message, no quotes or preamble.\n\n",
    );
    if !extra_context.is_empty() {
        prompt.push_str(&format!("Extra context: {extra_context}\n\n"));
    }
    prompt.push_str(&format!("Changed files:\n{names}\n\nDiff stat:\n{stat}\n"));

    let provider = provider.map(str::to_string).unwrap_or_else(detect_provider);
    let (bin, args): (&str, Vec<&str>) = match provider.as_str() {
        "claude" => ("claude", vec!["-p", &prompt]),
        "copilot" => (
            "copilot",
            vec![
                "-s",
                "--deny-tool",
                "write",
                "--deny-tool",
                "shell",
                "-p",
                &prompt,
            ],
        ),
        other => {
            note(
                tag,
                &format!("Unknown AI provider '{other}'. Use claude or copilot."),
            );
            return None;
        }
    };
    if which(bin).is_none() {
        note(tag, &format!("AI provider '{bin}' not found on PATH."));
        return None;
    }

    note(tag, &format!("Generating commit message via {bin}\u{2026}"));
    let out = Command::new(bin)
        .args(&args)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        None
    } else {
        Some(lines.into_iter().take(3).collect::<Vec<_>>().join("\n"))
    }
}

/// Prefer `claude`, then `copilot`, defaulting to claude when neither is found.
pub fn detect_provider() -> String {
    if which("claude").is_some() {
        "claude".into()
    } else if which("copilot").is_some() {
        "copilot".into()
    } else {
        "claude".into()
    }
}

/// Minimal `which`: the first PATH entry containing an executable `name`.
pub fn which(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
