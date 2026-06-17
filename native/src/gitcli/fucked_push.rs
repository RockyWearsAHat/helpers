//! `git-fucked-the-push` — recover from "I just pushed a bad last commit".
//!
//! Soft-resets HEAD back one commit (keeping changes staged) and, if that
//! commit is on the upstream, force-pushes with lease to drop it from the
//! remote branch. History-rewriting and destructive when the commit was pushed.

use std::io::Write;
use std::process::ExitCode;

use super::*;

const TAG: &str = "git-fucked-the-push";

const USAGE: &str = r#"Usage:
  git fucked-the-push [--yes]

Destructive recovery helper for when you accidentally pushed a bad last commit.

What it will do:
  - Move your branch back one commit (soft reset), keeping the changes STAGED.
  - If that last commit is on your upstream, rewrite the remote branch using
    `git push --force-with-lease` to drop the bad commit.

Flags:
  --yes   Skip the interactive confirmation prompt.

Examples:
  git fucked-the-push
  git fucked-the-push --yes
"#;

pub fn run(args: &[String]) -> ExitCode {
    let mut assume_yes = false;
    for arg in args {
        match arg.as_str() {
            "--yes" => assume_yes = true,
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other => {
                note(TAG, &format!("Unknown argument: {other}"));
                eprint!("{USAGE}");
                return ExitCode::from(2);
            }
        }
    }

    if !in_repo() {
        note(TAG, "Not a git repository.");
        return ExitCode::from(1);
    }

    let current_branch = current_branch();
    if current_branch.is_empty() {
        note(TAG, "HEAD is detached. Checkout a branch first.");
        return ExitCode::from(1);
    }

    let upstream_ref =
        git_out(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]).unwrap_or_default();

    let head_commit = git_out(&["rev-parse", "HEAD"]).unwrap_or_default();
    let subject = git_out(&["log", "-1", "--pretty=%s"]).unwrap_or_default();
    let head_parent = git_out(&["rev-parse", "-q", "--verify", "HEAD~1"]).unwrap_or_default();
    if head_parent.is_empty() {
        note(TAG, "Your branch has no parent commit (initial commit).");
        note(TAG, "Safer fix for a bad message: git commit --amend");
        note(
            TAG,
            "(If you already pushed the initial commit, history rewriting is riskier.)",
        );
        return ExitCode::from(1);
    }

    let pushed_to_upstream = !upstream_ref.is_empty()
        && git_ok(&["merge-base", "--is-ancestor", &head_commit, &upstream_ref]);

    note(TAG, &format!("Branch:   {current_branch}"));
    if upstream_ref.is_empty() {
        note(TAG, "Upstream: (none set)");
    } else {
        note(TAG, &format!("Upstream: {upstream_ref}"));
    }
    note(TAG, "Last commit:");
    eprintln!("  {head_commit}  {subject}");
    eprintln!();
    note(TAG, "WARNING: This may be destructive.");
    if pushed_to_upstream {
        note(TAG, "The last commit appears to be on your upstream.");
        note(
            TAG,
            "This will rewrite the remote branch using --force-with-lease.",
        );
        note(TAG, "Do NOT proceed if others may have pulled this branch.");
    } else {
        note(
            TAG,
            "The last commit does not appear on the upstream (or no upstream is set).",
        );
        note(
            TAG,
            "This will only move your local branch back one commit.",
        );
    }
    eprintln!();

    if !assume_yes && !confirm() {
        note(TAG, "Aborted.");
        return ExitCode::from(1);
    }

    // Step 1: uncommit but keep changes staged.
    if !git_inherit(&["reset", "--soft", "HEAD~1"]) {
        note(TAG, "Soft reset failed.");
        return ExitCode::from(1);
    }

    // Step 2: prune the bad commit from the remote if it was pushed.
    if !upstream_ref.is_empty() && pushed_to_upstream {
        git_ok(&["fetch"]);
        note(
            TAG,
            "Dropping the bad commit from the remote via --force-with-lease\u{2026}",
        );
        if !git_inherit(&["push", "--force-with-lease"]) {
            note(TAG, "Force-push failed.");
            note(
                TAG,
                "Possible causes: branch protection, permissions, or the remote moved.",
            );
            note(
                TAG,
                "Your changes are still staged locally; you can recommit with a better message.",
            );
            return ExitCode::from(1);
        }
    }

    note(TAG, "Done. Your changes are staged and ready to recommit.");
    git_inherit(&["status", "-sb"]);
    eprintln!("---");
    git_inherit(&["diff", "--cached", "--name-status"]);
    note(TAG, "Next: create a new commit, then push.");
    ExitCode::SUCCESS
}

/// Prompt on stderr and require the user to type `yes`.
fn confirm() -> bool {
    eprint!("[{TAG}] Type 'yes' to proceed: ");
    let _ = std::io::stderr().flush();
    let mut reply = String::new();
    if std::io::stdin().read_line(&mut reply).is_err() {
        return false;
    }
    reply.trim() == "yes"
}
