//! Native Rust ports of the standalone `git-*` shell CLIs.
//!
//! These are invoked busybox-style: the `gsh-native` binary is symlinked to
//! each CLI name (`git-resolve`, `git-remerge`, …) and dispatches on the
//! basename of `argv[0]`. They can also be run explicitly as
//! `gsh-native gitcli <name> [args…]`.
//!
//! Every CLI here is deterministic (no AI) except `git-upload`, which is
//! deterministic by default and offers an opt-in AI commit-message path.

use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

pub mod fucked_push;
pub mod get;
pub mod initialize;
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
        other => {
            eprintln!("gsh-native gitcli: unknown CLI: {other}");
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
