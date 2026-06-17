//! `git-remerge` — merge a (typically `detached-work-*`) branch back into a
//! target branch (default `main`/`master`). Completes and deletes the source
//! branch on a clean merge; aborts and leaves everything unchanged on conflict.

use std::process::ExitCode;

use super::*;

const TAG: &str = "git-remerge";

/// Run git-remerge: merge a (detached-work) source branch into a target,
/// completing and deleting it on a clean merge or aborting cleanly on conflict.
/// Each phase is delegated to a focused helper that returns the process exit code.
pub fn run(args: &[String]) -> ExitCode {
    let (source_cli, target_cli) = match parse_args(args) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let source_branch = match resolve_source(source_cli) {
        Ok(b) => b,
        Err(code) => return code,
    };
    let target_branch = match resolve_target(target_cli) {
        Ok(b) => b,
        Err(code) => return code,
    };
    if let Err(code) = validate_merge(&source_branch, &target_branch) {
        return code;
    }
    let original_branch = current_or_short_head();
    perform_merge(&source_branch, &target_branch, &original_branch)
}

/// Parse the positional source branch and `--into <target>` flag from CLI args.
/// Empty strings mean "not supplied"; an unknown `--flag` is a usage error.
fn parse_args(args: &[String]) -> Result<(String, String), ExitCode> {
    let mut source_branch = String::new();
    let mut target_branch = String::new();
    let mut expect_into = false;
    for arg in args {
        match arg.as_str() {
            "--into" => expect_into = true,
            s if s.starts_with("--") => {
                note(TAG, &format!("Unknown option: {s}"));
                return Err(ExitCode::from(1));
            }
            s => {
                if expect_into {
                    target_branch = s.to_string();
                    expect_into = false;
                } else if source_branch.is_empty() {
                    source_branch = s.to_string();
                }
            }
        }
    }
    Ok((source_branch, target_branch))
}

/// Resolve the source branch: the given one, else the current `detached-work-*`
/// branch. Otherwise explain the available branches/usage and fail.
fn resolve_source(given: String) -> Result<String, ExitCode> {
    if !given.is_empty() {
        return Ok(given);
    }
    let current = current_branch();
    if current.is_empty() {
        note(
            TAG,
            "You are in a detached HEAD state. Please specify a branch to merge.",
        );
        note(
            TAG,
            "Usage: git-remerge <branch-name> [--into <target-branch>]",
        );
        return Err(ExitCode::from(1));
    }
    if current.starts_with("detached-work-") {
        note(TAG, &format!("Using current branch: {current}"));
        return Ok(current);
    }
    list_detached_work();
    note(
        TAG,
        "Usage: git-remerge <branch-name> [--into <target-branch>]",
    );
    Err(ExitCode::from(1))
}

/// Print the available `detached-work-*` branches (or note that none exist).
fn list_detached_work() {
    let detached = git_out(&[
        "branch",
        "--list",
        "detached-work-*",
        "--format=%(refname:short)",
    ])
    .unwrap_or_default();
    if detached.is_empty() {
        note(
            TAG,
            "No branch specified and no detached-work-* branches found.",
        );
    } else {
        note(
            TAG,
            "No branch specified. Available detached-work branches:",
        );
        for b in detached.lines() {
            eprintln!("  - {b}");
        }
    }
}

/// Resolve the target branch: the given one, else default to `main`/`master`.
fn resolve_target(given: String) -> Result<String, ExitCode> {
    if !given.is_empty() {
        return Ok(given);
    }
    if git_ok(&["rev-parse", "--verify", "main"]) {
        Ok("main".into())
    } else if git_ok(&["rev-parse", "--verify", "master"]) {
        Ok("master".into())
    } else {
        note(
            TAG,
            "Could not find 'main' or 'master' branch. Please specify target with --into.",
        );
        Err(ExitCode::from(1))
    }
}

/// Verify both branches exist, are distinct, and the worktree is clean.
fn validate_merge(source: &str, target: &str) -> Result<(), ExitCode> {
    if !git_ok(&["rev-parse", "--verify", source]) {
        note(TAG, &format!("Branch '{source}' does not exist."));
        return Err(ExitCode::from(1));
    }
    if !git_ok(&["rev-parse", "--verify", target]) {
        note(TAG, &format!("Target branch '{target}' does not exist."));
        return Err(ExitCode::from(1));
    }
    if source == target {
        note(TAG, &format!("Cannot merge '{source}' into itself."));
        return Err(ExitCode::from(1));
    }
    if !git_ok(&["diff-index", "--quiet", "HEAD", "--"]) {
        note(
            TAG,
            "You have uncommitted changes. Please commit or stash them first.",
        );
        return Err(ExitCode::from(1));
    }
    Ok(())
}

/// Current branch name, or the short HEAD sha when in detached HEAD.
fn current_or_short_head() -> String {
    let b = current_branch();
    if b.is_empty() {
        git_out(&["rev-parse", "--short", "HEAD"]).unwrap_or_default()
    } else {
        b
    }
}

/// Switch to the target, fast-forward it, then attempt the merge — finishing
/// cleanly, reporting a no-op, or aborting with conflict help.
fn perform_merge(source: &str, target: &str, original: &str) -> ExitCode {
    note(
        TAG,
        &format!("Attempting to merge '{source}' into '{target}'..."),
    );
    if !git_ok(&["switch", target]) {
        note(TAG, &format!("Failed to switch to '{target}'."));
        return ExitCode::from(1);
    }
    fast_forward_to_upstream(target);

    // Stage the merge first (--no-commit) so we can detect conflicts and no-ops.
    if !git_ok(&["merge", "--no-commit", "--no-ff", source]) {
        return abort_with_conflict_help(source, target, original);
    }
    if git_ok(&["diff", "--cached", "--quiet"]) {
        return finish_already_merged(source, target, original);
    }
    finish_clean_merge(source, target)
}

/// Fetch and fast-forward `target` to its upstream when one is configured.
fn fast_forward_to_upstream(target: &str) {
    if !git_ok(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]) {
        return;
    }
    note(TAG, &format!("Fetching latest changes for '{target}'..."));
    git_ok(&["fetch"]);
    if !git_ok(&["pull", "--ff-only"]) {
        note(
            TAG,
            &format!(
                "Warning: Could not fast-forward '{target}' to upstream. Continuing with local state."
            ),
        );
    }
}

/// No-op path: the source is already incorporated. Abort the staged merge,
/// delete the redundant branch, and restore the original branch.
fn finish_already_merged(source: &str, target: &str, original: &str) -> ExitCode {
    git_ok(&["merge", "--abort"]);
    note(
        TAG,
        &format!("No changes to merge. '{source}' is already incorporated into '{target}'."),
    );
    note(TAG, &format!("Deleting redundant branch '{source}'..."));
    if !git_ok(&["branch", "-d", source]) {
        note(
            TAG,
            &format!(
                "Warning: Could not delete '{source}'. You may need to use -D to force delete."
            ),
        );
    }
    if original != source {
        git_ok(&["switch", original]);
    }
    ExitCode::SUCCESS
}

/// Success path: commit the staged merge, delete the merged source branch, and
/// print a summary (plus a push hint when the target tracks an upstream).
fn finish_clean_merge(source: &str, target: &str) -> ExitCode {
    git_ok(&["commit", "--no-edit"]);
    note(TAG, "\u{2713} Merge completed successfully!");
    note(TAG, &format!("Deleting merged branch '{source}'..."));
    if !git_ok(&["branch", "-d", source]) && !git_ok(&["branch", "-D", source]) {
        note(
            TAG,
            &format!("Warning: Could not delete '{source}'. You may need to delete it manually."),
        );
    }
    note(TAG, "Summary:");
    note(TAG, &format!("  Merged: '{source}' \u{2192} '{target}'"));
    note(TAG, &format!("  Deleted: '{source}'"));
    note(TAG, &format!("  Current branch: '{target}'"));
    if git_ok(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]) {
        note(TAG, "Run 'git push' to push the merged changes to remote.");
    }
    ExitCode::SUCCESS
}

/// Conflict path: abort the merge, restore the original branch, and print the
/// manual resolution steps.
fn abort_with_conflict_help(source: &str, target: &str, original: &str) -> ExitCode {
    git_ok(&["merge", "--abort"]);
    note(TAG, "\u{2717} Merge would result in conflicts. Aborting.");
    note(TAG, "No changes have been made.");
    eprintln!();
    note(TAG, "To manually resolve conflicts, you can:");
    note(TAG, &format!("  1. git switch {target}"));
    note(TAG, &format!("  2. git merge {source}"));
    note(TAG, "  3. Resolve conflicts manually");
    note(TAG, "  4. git commit");
    note(TAG, &format!("  5. git branch -d {source}"));
    if original != target {
        git_ok(&["switch", original]);
    }
    ExitCode::from(1)
}
