//! `git-resolve` — safe helper for resolving in-progress merge/rebase conflicts.
//!
//! Never destroys history: it always creates backup branches before you
//! continue or abort, shows what operation is in progress and which files are
//! conflicted, and leaves the actual resolution to you.

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use super::*;

const TAG: &str = "git-resolve";

pub fn run(_args: &[String]) -> ExitCode {
    if !in_repo() {
        note(TAG, "Not inside a git repository.");
        return ExitCode::from(1);
    }
    let git_dir = git_dir();
    let gd = Path::new(&git_dir);

    let has_merge = gd.join("MERGE_HEAD").is_file();
    let has_rebase = gd.join("rebase-merge").is_dir() || gd.join("rebase-apply").is_dir();
    let worktree_status = git_out(&["status", "--porcelain"]).unwrap_or_default();
    let has_dirty = !worktree_status.is_empty();

    if !has_merge && !has_rebase && !has_dirty {
        note(
            TAG,
            "No merge, rebase, or local changes; nothing to resolve.",
        );
        return ExitCode::SUCCESS;
    }

    // Safety backups so the previous state is always recoverable.
    let ts = timestamp();
    let current_head = match git_out(&["rev-parse", "HEAD"]) {
        Some(h) => h,
        None => {
            note(TAG, "Could not read HEAD.");
            return ExitCode::from(1);
        }
    };
    let backup_branch = format!("git-resolve-backup-{ts}");
    git_ok(&["branch", &backup_branch, &current_head]);
    note(
        TAG,
        &format!("Created backup branch: {backup_branch} (points at current HEAD)"),
    );

    if let Some(orig) = git_out(&["rev-parse", "ORIG_HEAD"]) {
        let orig_backup = format!("git-resolve-orig-{ts}");
        git_ok(&["branch", &orig_backup, &orig]);
        note(
            TAG,
            &format!("Created backup branch: {orig_backup} (points at ORIG_HEAD)"),
        );
    }

    if has_merge {
        if let Ok(merge_head) = fs::read_to_string(gd.join("MERGE_HEAD")) {
            let merge_head = merge_head.trim();
            if !merge_head.is_empty() {
                let merge_backup = format!("git-resolve-merge-{ts}");
                git_ok(&["branch", &merge_backup, merge_head]);
                note(
                    TAG,
                    &format!("Created backup branch: {merge_backup} (points at MERGE_HEAD)"),
                );
            }
        }
    }

    eprintln!();

    if has_rebase || has_merge {
        if has_rebase {
            println!("[{TAG}] A rebase is currently in progress.");
        }
        if has_merge {
            println!("[{TAG}] A merge is currently in progress.");
        }
        let conflicted = git_out(&["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
        if conflicted.is_empty() {
            note(
                TAG,
                "No files reported as conflicted, but an operation is in progress.",
            );
        } else {
            println!();
            println!("[{TAG}] Conflicted files:");
            for line in conflicted.lines() {
                println!("  - {line}");
            }
        }
        println!();
        print!("{}", REBASE_MERGE_STEPS);
    } else if has_dirty {
        note(
            TAG,
            "No merge or rebase is in progress, but you have local changes.",
        );
        eprintln!();
        note(TAG, "git status summary:");
        for line in worktree_status.lines() {
            eprintln!("  {line}");
        }
        eprintln!();
        print!("{}", DIRTY_STEPS);
    }

    ExitCode::SUCCESS
}

const REBASE_MERGE_STEPS: &str = r#"Next steps (manual but safe):

1. Open each conflicted file, look for conflict markers (<<<<<<<, =======, >>>>>>>),
   and edit them to the desired final content.
   - You can also use: git mergetool

2. When a file is resolved, stage it:
     git add <file>

3. When all conflicts are resolved and staged:
   - If you were rebasing: run
       git rebase --continue
   - If you were merging: run
       git commit

4. If you decide to cancel the in-progress operation entirely:
   - For a rebase:
       git rebase --abort
   - For a merge:
       git merge --abort

Because git-resolve created backup branches, you can always recover the
previous state with, for example:
   git switch git-resolve-backup-<timestamp>
"#;

const DIRTY_STEPS: &str = r#"Next steps to make 'git pull --rebase' or 'git upload' succeed:

1. Decide whether to COMMIT or STASH your local changes.

   To commit them now on this branch:
     git add <files>
     git commit -m "your commit message"

   To stash them temporarily (so the branch can rebase cleanly):
     git stash push -u -m "before rebase"

2. After your working tree is clean, update the branch:
   - Either run:
       git pull --rebase
   - Or simply rerun:
       git upload -ai

3. If you stashed changes, re-apply them once the branch is updated:
     git stash pop

Because git-resolve created backup branches, you can always recover the
previous state with, for example:
   git switch git-resolve-backup-<timestamp>
"#;
