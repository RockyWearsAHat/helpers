//! `git-checkpoint` — commit the current working state at an intentional
//! moment. Unlike `git-upload` it only commits locally (unless `--push`).
//!
//! Deterministic by default: the message is `-m`, or a generated summary of the
//! staged change. AI messages are opt-in via `-ai` through Claude or Copilot.
//! Per-repo gating, signing, and push-after-commit come from git config keys
//! (`checkpoint.enabled` / `checkpoint.sign` / `checkpoint.push`).

use std::process::ExitCode;

use super::*;

const TAG: &str = "git-checkpoint";

const HELP: &str = r#"git-checkpoint - commit the current state at an intentional moment (local only)

USAGE:
    git checkpoint [options] [ai-context]

OPTIONS:
    -m <message>     Use this message instead of generating one
    -a, --all        Stage all tracked changes first (git add -u)
    --push           Also push after committing (via git-upload)
    -ai              Generate the message with AI (Claude or Copilot)
    --provider <p>   AI provider for -ai: claude | copilot
    --dry-run        Show what would be committed without committing
    --no-verify      Skip pre-commit hooks
    --status         Show checkpoint config for this repo
    --enable         Enable checkpoint for this repo
    --disable        Disable checkpoint for this repo
    -h, --help       Show this help

CONFIG (git config):
    checkpoint.enabled  true/false gate for external callers (default true)
    checkpoint.push     true/false always push after commit (default false)
    checkpoint.sign     true/false GPG-sign commits (default false)
"#;

struct Opts {
    manual_msg: Option<String>,
    stage_all: bool,
    do_push: bool,
    dry_run: bool,
    no_verify: bool,
    use_ai: bool,
    provider: Option<String>,
    ai_context: String,
}

pub fn run(args: &[String]) -> ExitCode {
    let mut o = Opts {
        manual_msg: None,
        stage_all: false,
        do_push: false,
        dry_run: false,
        no_verify: false,
        use_ai: false,
        provider: None,
        ai_context: String::new(),
    };

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "-m" => {
                i += 1;
                match args.get(i) {
                    Some(m) => o.manual_msg = Some(m.clone()),
                    None => {
                        note(TAG, "-m requires a message argument.");
                        return ExitCode::from(1);
                    }
                }
            }
            "-a" | "--all" => o.stage_all = true,
            "--push" => o.do_push = true,
            "-ai" | "--aiDiffCommitMsg" => o.use_ai = true,
            "--provider" => {
                i += 1;
                o.provider = args.get(i).cloned();
            }
            "--dry-run" => o.dry_run = true,
            "--no-verify" => o.no_verify = true,
            "--status" => return cmd_status(),
            "--enable" => {
                git_ok(&["config", "checkpoint.enabled", "true"]);
                note(TAG, "Enabled for this repo.");
                return ExitCode::SUCCESS;
            }
            "--disable" => {
                git_ok(&["config", "checkpoint.enabled", "false"]);
                note(TAG, "Disabled for this repo.");
                return ExitCode::SUCCESS;
            }
            "-h" | "--help" => {
                print!("{HELP}");
                return ExitCode::SUCCESS;
            }
            s if s.starts_with("--model") => { /* AI model selection removed; ignore */ }
            s if s.starts_with("--") => {
                note(TAG, &format!("Unknown option: {s}"));
                return ExitCode::from(1);
            }
            s => {
                if o.ai_context.is_empty() {
                    o.ai_context = s.to_string();
                }
            }
        }
        i += 1;
    }

    // Per-repo enable gate.
    if git_out(&["config", "--get", "checkpoint.enabled"]).as_deref() == Some("false") {
        note(
            TAG,
            "Disabled for this repo. Use 'git checkpoint --enable' to turn on.",
        );
        return ExitCode::SUCCESS;
    }
    if !git_ok(&["rev-parse", "--is-inside-work-tree"]) {
        note(TAG, "Not inside a git repository.");
        return ExitCode::from(1);
    }

    if o.stage_all {
        git_ok(&["add", "-u"]);
    }

    // Require staged changes.
    if git_ok(&["diff", "--cached", "--quiet"]) {
        if !git_ok(&["diff", "--quiet"]) {
            note(
                TAG,
                "No staged changes. Use -a to stage tracked changes, or stage manually.",
            );
            return ExitCode::from(1);
        }
        note(TAG, "Nothing to commit — working tree clean.");
        return ExitCode::SUCCESS;
    }

    if o.dry_run {
        note(TAG, "DRY RUN — would commit these changes:");
        git_inherit(&["diff", "--cached", "--stat"]);
        match &o.manual_msg {
            Some(m) => eprintln!("\nMessage: {m}"),
            None => eprintln!(
                "\nMessage: (generated summary{})",
                if o.use_ai { " or AI" } else { "" }
            ),
        }
        return ExitCode::SUCCESS;
    }

    // Build the message.
    let commit_msg = if let Some(m) = &o.manual_msg {
        m.clone()
    } else if o.use_ai {
        ai_commit_message(o.provider.as_deref(), &o.ai_context, TAG)
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| {
                note(TAG, "AI message unavailable; using a generated summary.");
                staged_summary_message("checkpoint: update")
            })
    } else {
        staged_summary_message("checkpoint: update")
    };

    // Commit, honoring --no-verify and checkpoint.sign.
    let mut commit_args: Vec<&str> = vec!["commit"];
    if o.no_verify {
        commit_args.push("--no-verify");
    }
    let signed = git_out(&["config", "--get", "checkpoint.sign"]).as_deref() == Some("true");
    if signed {
        commit_args.push("-S");
    }
    commit_args.push("-m");
    commit_args.push(&commit_msg);
    if !git_inherit(&commit_args) {
        note(TAG, "git commit failed.");
        return ExitCode::from(1);
    }
    note(TAG, "\u{2705} Committed.");
    git_inherit(&["log", "--oneline", "-1"]);

    // Optional push: delegate to git-upload for its remote-tracking logic.
    let push_cfg = git_out(&["config", "--get", "checkpoint.push"]).as_deref() == Some("true");
    if o.do_push || push_cfg {
        note(TAG, "Pushing\u{2026}");
        return super::upload::run(&[commit_msg]);
    }
    ExitCode::SUCCESS
}

/// Print the per-repo checkpoint configuration.
fn cmd_status() -> ExitCode {
    let get = |k: &str, d: &str| git_out(&["config", "--get", k]).unwrap_or_else(|| d.to_string());
    note(
        TAG,
        &format!("enabled: {}", get("checkpoint.enabled", "true")),
    );
    note(
        TAG,
        &format!("push after commit: {}", get("checkpoint.push", "false")),
    );
    note(
        TAG,
        &format!("sign commits: {}", get("checkpoint.sign", "false")),
    );
    ExitCode::SUCCESS
}
