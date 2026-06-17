//! `git-remerge` — merge a (typically `detached-work-*`) branch back into a
//! target branch (default `main`/`master`). Completes and deletes the source
//! branch on a clean merge; aborts and leaves everything unchanged on conflict.

use std::process::ExitCode;

use super::*;

const TAG: &str = "git-remerge";

pub fn run(args: &[String]) -> ExitCode {
    let mut source_branch = String::new();
    let mut target_branch = String::new();
    let mut expect_into = false;

    for arg in args {
        match arg.as_str() {
            "--into" => expect_into = true,
            s if s.starts_with("--") => {
                note(TAG, &format!("Unknown option: {s}"));
                return ExitCode::from(1);
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

    // Determine the source branch when not given.
    if source_branch.is_empty() {
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
            return ExitCode::from(1);
        }
        if current.starts_with("detached-work-") {
            source_branch = current.clone();
            note(TAG, &format!("Using current branch: {source_branch}"));
        } else {
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
            note(
                TAG,
                "Usage: git-remerge <branch-name> [--into <target-branch>]",
            );
            return ExitCode::from(1);
        }
    }

    if !git_ok(&["rev-parse", "--verify", &source_branch]) {
        note(TAG, &format!("Branch '{source_branch}' does not exist."));
        return ExitCode::from(1);
    }

    // Determine the target branch when not given.
    if target_branch.is_empty() {
        if git_ok(&["rev-parse", "--verify", "main"]) {
            target_branch = "main".into();
        } else if git_ok(&["rev-parse", "--verify", "master"]) {
            target_branch = "master".into();
        } else {
            note(
                TAG,
                "Could not find 'main' or 'master' branch. Please specify target with --into.",
            );
            return ExitCode::from(1);
        }
    }

    if !git_ok(&["rev-parse", "--verify", &target_branch]) {
        note(
            TAG,
            &format!("Target branch '{target_branch}' does not exist."),
        );
        return ExitCode::from(1);
    }
    if source_branch == target_branch {
        note(TAG, &format!("Cannot merge '{source_branch}' into itself."));
        return ExitCode::from(1);
    }
    if !git_ok(&["diff-index", "--quiet", "HEAD", "--"]) {
        note(
            TAG,
            "You have uncommitted changes. Please commit or stash them first.",
        );
        return ExitCode::from(1);
    }

    let original_branch = {
        let b = current_branch();
        if b.is_empty() {
            git_out(&["rev-parse", "--short", "HEAD"]).unwrap_or_default()
        } else {
            b
        }
    };

    note(
        TAG,
        &format!("Attempting to merge '{source_branch}' into '{target_branch}'..."),
    );
    if !git_ok(&["switch", &target_branch]) {
        note(TAG, &format!("Failed to switch to '{target_branch}'."));
        return ExitCode::from(1);
    }

    // Fast-forward the target to upstream when possible.
    if git_ok(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]) {
        note(
            TAG,
            &format!("Fetching latest changes for '{target_branch}'..."),
        );
        git_ok(&["fetch"]);
        if !git_ok(&["pull", "--ff-only"]) {
            note(
                TAG,
                &format!(
                    "Warning: Could not fast-forward '{target_branch}' to upstream. Continuing with local state."
                ),
            );
        }
    }

    // Try the merge with --no-commit to check for conflicts first.
    if git_ok(&["merge", "--no-commit", "--no-ff", &source_branch]) {
        if git_ok(&["diff", "--cached", "--quiet"]) {
            // Nothing to merge — source already incorporated.
            git_ok(&["merge", "--abort"]);
            note(
                TAG,
                &format!(
                    "No changes to merge. '{source_branch}' is already incorporated into '{target_branch}'."
                ),
            );
            note(
                TAG,
                &format!("Deleting redundant branch '{source_branch}'..."),
            );
            if !git_ok(&["branch", "-d", &source_branch]) {
                note(
                    TAG,
                    &format!(
                        "Warning: Could not delete '{source_branch}'. You may need to use -D to force delete."
                    ),
                );
            }
            if original_branch != source_branch {
                git_ok(&["switch", &original_branch]);
            }
            return ExitCode::SUCCESS;
        }

        git_ok(&["commit", "--no-edit"]);
        note(TAG, "\u{2713} Merge completed successfully!");
        note(TAG, &format!("Deleting merged branch '{source_branch}'..."));
        if !git_ok(&["branch", "-d", &source_branch]) && !git_ok(&["branch", "-D", &source_branch])
        {
            note(
                TAG,
                &format!("Warning: Could not delete '{source_branch}'. You may need to delete it manually."),
            );
        }
        note(TAG, "Summary:");
        note(
            TAG,
            &format!("  Merged: '{source_branch}' \u{2192} '{target_branch}'"),
        );
        note(TAG, &format!("  Deleted: '{source_branch}'"));
        note(TAG, &format!("  Current branch: '{target_branch}'"));
        if git_ok(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]) {
            note(TAG, "Run 'git push' to push the merged changes to remote.");
        }
        ExitCode::SUCCESS
    } else {
        git_ok(&["merge", "--abort"]);
        note(TAG, "\u{2717} Merge would result in conflicts. Aborting.");
        note(TAG, "No changes have been made.");
        eprintln!();
        note(TAG, "To manually resolve conflicts, you can:");
        note(TAG, &format!("  1. git switch {target_branch}"));
        note(TAG, &format!("  2. git merge {source_branch}"));
        note(TAG, "  3. Resolve conflicts manually");
        note(TAG, "  4. git commit");
        note(TAG, &format!("  5. git branch -d {source_branch}"));
        if original_branch != target_branch {
            git_ok(&["switch", &original_branch]);
        }
        ExitCode::from(1)
    }
}
