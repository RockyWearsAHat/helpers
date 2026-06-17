//! `git-upload` — stage, commit, and push with safe recovery from broken or
//! in-progress git operations.
//!
//! Deterministic by default: the commit message is what you pass, or a
//! generated summary of the staged change. AI commit messages are opt-in via
//! `-ai` and run through Claude or Copilot — never required, never the default.
//!
//! Usage:
//!   git-upload [message]
//!   git-upload -ai [ai-context] [fallback-message]
//!   git-upload --provider claude|copilot -ai
//!   git-upload --auto-resolve            (take "theirs" on conflicts)

use std::process::ExitCode;

use super::*;

const TAG: &str = "git-upload";

struct Opts {
    use_ai: bool,
    provider: Option<String>,
    auto_resolve: bool,
    ai_context: String,
    user_msg: String,
}

pub fn run(args: &[String]) -> ExitCode {
    let mut o = Opts {
        use_ai: false,
        provider: None,
        auto_resolve: false,
        ai_context: String::new(),
        user_msg: String::new(),
    };

    let mut expect_provider = false;
    let mut expect_ai_context = false;
    for arg in args {
        if expect_provider {
            o.provider = Some(arg.clone());
            expect_provider = false;
            continue;
        }
        match arg.as_str() {
            "--aiDiffCommitMsg" | "-ai" => {
                o.use_ai = true;
                expect_ai_context = true;
            }
            "--provider" => expect_provider = true,
            "--auto-resolve" => o.auto_resolve = true,
            s if s.starts_with("--") => { /* ignore unknown flags */ }
            s => {
                if expect_ai_context {
                    o.ai_context = s.to_string();
                    expect_ai_context = false;
                } else if o.user_msg.is_empty() {
                    o.user_msg = s.to_string();
                }
            }
        }
    }

    if !in_repo() {
        note(TAG, "Not inside a git repository.");
        return ExitCode::from(1);
    }

    if let Err(code) = recover_in_progress(&o) {
        return code;
    }
    let current_branch = match resolve_branch(&o) {
        Ok(b) => b,
        Err(code) => return code,
    };
    if let Err(code) = sync_upstream(&current_branch) {
        return code;
    }

    git_ok(&["add", "-A"]);

    if let Some(repo_root) = git_out(&["rev-parse", "--show-toplevel"]) {
        if let Err(code) = ensure_release_notes(&repo_root) {
            return code;
        }
    }

    warn_sensitive();

    // Ensure there is something to commit.
    if git_ok(&["diff", "--cached", "--quiet"]) {
        let dirty = git_out(&["status", "--porcelain"]).unwrap_or_default();
        if dirty.is_empty() {
            note(TAG, "Nothing to commit \u{2013} working tree is clean.");
            return ExitCode::from(1);
        }
        git_ok(&["add", "-A"]);
    }

    let commit_msg = resolve_message(&o);
    note(TAG, &format!("Using commit message: {commit_msg}"));

    let signed = git_out(&["config", "--get", "checkpoint.sign"]).as_deref() == Some("true");
    let commit_ok = if signed {
        git_inherit(&["commit", "-S", "-m", &commit_msg])
    } else {
        git_inherit(&["commit", "-m", &commit_msg])
    };
    if !commit_ok {
        note(TAG, "git commit failed.");
        return ExitCode::from(1);
    }

    let push_ok = if git_ok(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]) {
        git_inherit(&["push"])
    } else {
        git_inherit(&["push", "-u", "origin", &current_branch])
    };
    if !push_ok {
        note(TAG, "git push failed.");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

/// Phase 0 — detect and safely recover leftover in-progress operations.
fn recover_in_progress(o: &Opts) -> Result<(), ExitCode> {
    // Bisect.
    if git_dir_has("BISECT_LOG") {
        note(TAG, "\u{26a0}\u{fe0f}  A bisect session is in progress.");
        note(TAG, "Resetting bisect to return to your branch\u{2026}");
        if !git_ok(&["bisect", "reset"]) {
            note(
                TAG,
                "Failed to reset bisect. Run 'git bisect reset' manually.",
            );
            return Err(ExitCode::from(1));
        }
    }
    // git am (check before rebase: am also uses rebase-apply/).
    if git_dir_has("rebase-apply/applying") {
        note(
            TAG,
            "\u{26a0}\u{fe0f}  A 'git am' is in progress. Aborting\u{2026}",
        );
        git_ok(&["am", "--abort"]);
    }
    // Rebase.
    let rebasing = git_dir_has("rebase-merge")
        || (git_dir_has("rebase-apply") && !git_dir_has("rebase-apply/applying"));
    if rebasing {
        note(TAG, "\u{26a0}\u{fe0f}  A rebase is in progress.");
        if git_ok(&["diff", "--quiet"]) && git_ok(&["diff", "--cached", "--quiet"]) {
            if git_ok(&["rebase", "--continue"]) {
                note(TAG, "\u{2705} Rebase completed successfully.");
            } else {
                git_ok(&["rebase", "--abort"]);
                note(
                    TAG,
                    "\u{2705} Rebase aborted; your branch is back to its pre-rebase state.",
                );
            }
        } else {
            recover_conflicts(o, "rebase");
        }
    }
    // Merge.
    if git_dir_has("MERGE_HEAD") {
        note(TAG, "\u{26a0}\u{fe0f}  A merge is in progress.");
        recover_conflicts(o, "merge");
    }
    // Cherry-pick.
    if git_dir_has("CHERRY_PICK_HEAD") {
        note(TAG, "\u{26a0}\u{fe0f}  A cherry-pick is in progress.");
        recover_conflicts(o, "cherry-pick");
    }
    // Revert.
    if git_dir_has("REVERT_HEAD") {
        note(TAG, "\u{26a0}\u{fe0f}  A revert is in progress.");
        let conflicts = git_out(&["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
        if conflicts.is_empty() {
            if !git_ok(&["revert", "--continue"]) {
                git_ok(&["revert", "--abort"]);
            }
        } else {
            git_ok(&["revert", "--abort"]);
            note(TAG, "Revert had conflicts; aborted to preserve work.");
        }
    }
    Ok(())
}

/// Resolve conflicts for an in-progress `op`. With `--auto-resolve`, take
/// "theirs" for each conflicted file and continue; otherwise abort to preserve
/// work (the safe, deterministic default).
fn recover_conflicts(o: &Opts, op: &str) {
    let conflicts = git_out(&["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
    if conflicts.is_empty() {
        note(TAG, &format!("{op} appears resolved. Continuing\u{2026}"));
        git_ok(&[op, "--continue"]);
        return;
    }
    if o.auto_resolve {
        note(
            TAG,
            &format!("{op} conflicts; auto-resolving (taking theirs)\u{2026}"),
        );
        for cf in conflicts.lines() {
            if git_ok(&["checkout", "--theirs", "--", cf]) {
                git_ok(&["add", cf]);
            }
        }
        let remaining = git_out(&["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
        if remaining.is_empty() && git_ok(&[op, "--continue"]) {
            note(
                TAG,
                &format!("\u{2705} {op} completed after auto-resolving conflicts."),
            );
        } else {
            git_ok(&[op, "--abort"]);
            note(TAG, &format!("\u{2705} {op} aborted safely."));
        }
    } else {
        git_ok(&[op, "--abort"]);
        note(
            TAG,
            &format!("\u{2705} {op} aborted. Pass --auto-resolve to take theirs automatically."),
        );
    }
}

/// Phase 1 — ensure HEAD is on a branch, returning the branch name.
fn resolve_branch(o: &Opts) -> Result<String, ExitCode> {
    let current = current_branch();
    if !current.is_empty() {
        return Ok(current);
    }
    // Detached HEAD.
    let head_commit = git_out(&["rev-parse", "-q", "--verify", "HEAD"]).unwrap_or_default();
    if head_commit.is_empty() {
        note(
            TAG,
            "HEAD is not on a branch (and there are no commits yet).",
        );
        note(
            TAG,
            "Fix: create/switch to a branch first (e.g. 'git switch -c main').",
        );
        return Err(ExitCode::from(1));
    }
    let containing = git_out(&[
        "branch",
        "--contains",
        &head_commit,
        "--format=%(refname:short)",
    ])
    .unwrap_or_default();
    let branches: Vec<&str> = containing
        .lines()
        .map(str::trim)
        .filter(|b| !b.is_empty() && !b.starts_with('('))
        .collect();

    match branches.len() {
        1 => {
            let b = branches[0].to_string();
            note(
                TAG,
                &format!("Detached HEAD is contained by '{b}'; switching\u{2026}"),
            );
            if git_ok(&["switch", &b]) {
                Ok(b)
            } else {
                note(TAG, &format!("Failed to switch to '{b}'."));
                Err(ExitCode::from(1))
            }
        }
        0 => {
            let short =
                git_out(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
            let new_branch = format!("detached-work-{short}-{}", timestamp());
            note(
                TAG,
                &format!(
                    "Detached HEAD with commits not on any branch; creating '{new_branch}'\u{2026}"
                ),
            );
            if git_ok(&["switch", "-c", &new_branch]) {
                Ok(new_branch)
            } else {
                note(TAG, &format!("Failed to create branch '{new_branch}'."));
                Err(ExitCode::from(1))
            }
        }
        _ => {
            if o.auto_resolve {
                let best = branches
                    .iter()
                    .find(|b| matches!(***b, _ if ["main", "master", "develop"].contains(*b)))
                    .copied()
                    .unwrap_or(branches[0]);
                note(
                    TAG,
                    &format!("Detached HEAD on multiple branches; picking '{best}'."),
                );
                if git_ok(&["switch", best]) {
                    Ok(best.to_string())
                } else {
                    note(TAG, &format!("Failed to switch to '{best}'."));
                    Err(ExitCode::from(1))
                }
            } else {
                note(
                    TAG,
                    "Detached HEAD; the commit exists on multiple branches:",
                );
                for b in &branches {
                    eprintln!("  - {b}");
                }
                note(
                    TAG,
                    "Fix: 'git switch <branch>' first, or pass --auto-resolve.",
                );
                Err(ExitCode::from(1))
            }
        }
    }
}

/// Rebase onto upstream when behind, then dry-run the push to catch protection
/// or permission errors before committing.
fn sync_upstream(current_branch: &str) -> Result<(), ExitCode> {
    let Some(upstream) = git_out(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
    else {
        return Ok(()); // no upstream — push will set one later
    };
    git_ok(&["fetch"]);

    let ahead_behind = git_out(&[
        "rev-list",
        "--left-right",
        "--count",
        &format!("{upstream}...HEAD"),
    ])
    .unwrap_or_default();
    let behind: u32 = ahead_behind
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if behind > 0 {
        note(
            TAG,
            &format!(
                "Branch '{current_branch}' is {behind} commit(s) behind upstream. Rebasing..."
            ),
        );
        if !git_ok(&["pull", "--rebase", "--autostash"]) {
            note(
                TAG,
                "git pull --rebase --autostash stopped (conflicts or local changes).",
            );
            note(
                TAG,
                "Run 'git resolve' for a safe backup and guidance, then rerun git-upload.",
            );
            return Err(ExitCode::from(1));
        }
    }
    if !git_ok(&["push", "--dry-run"]) {
        note(
            TAG,
            &format!("Unable to push to the upstream for '{current_branch}'."),
        );
        note(
            TAG,
            "This branch may be protected or you may lack direct-push permission.",
        );
        return Err(ExitCode::from(1));
    }
    Ok(())
}

/// Deterministic guard: if VERSION is staged-changed and the matching release
/// notes file is missing, require the user to create it first.
fn ensure_release_notes(repo_root: &str) -> Result<(), ExitCode> {
    let version_path = std::path::Path::new(repo_root).join("VERSION");
    if !version_path.is_file() {
        return Ok(());
    }
    let staged = git_out(&["diff", "--cached", "--name-only"]).unwrap_or_default();
    if !staged.lines().any(|l| l == "VERSION") {
        return Ok(());
    }
    let new_version = std::fs::read_to_string(&version_path)
        .unwrap_or_default()
        .trim()
        .to_string();
    if new_version.is_empty() {
        return Ok(());
    }
    let old_version = git_out(&["show", "HEAD:VERSION"]).unwrap_or_default();
    if old_version == new_version {
        return Ok(());
    }
    let notes_rel = format!("release-notes/v{new_version}.md");
    let notes_path = std::path::Path::new(repo_root).join(&notes_rel);
    if notes_path.is_file() {
        git_ok(&["add", notes_path.to_str().unwrap_or(&notes_rel)]);
        return Ok(());
    }
    note(
        TAG,
        &format!("VERSION changed to {new_version} but {notes_rel} is missing."),
    );
    note(
        TAG,
        "Create that release notes file before running git-upload.",
    );
    Err(ExitCode::from(1))
}

/// Warn (don't block) when sensitive files are staged.
fn warn_sensitive() {
    let staged = git_out(&["diff", "--cached", "--name-only"]).unwrap_or_default();
    let hits: Vec<&str> = staged
        .lines()
        .filter(|f| {
            let lf = f.to_lowercase();
            lf.ends_with(".env")
                || lf.ends_with(".pem")
                || lf.ends_with(".key")
                || lf.ends_with(".p12")
                || lf.ends_with(".pfx")
                || lf.ends_with(".log")
                || lf.contains("id_rsa")
                || lf.contains("id_ed25519")
                || lf.contains("credentials")
                || lf.contains(".npmrc")
                || lf.contains(".netrc")
        })
        .collect();
    if !hits.is_empty() {
        eprintln!();
        note(
            TAG,
            "\u{26a0}\u{fe0f}  SENSITIVE FILES DETECTED IN STAGED CHANGES:",
        );
        for f in &hits {
            eprintln!("[{TAG}]   \u{2022} {f}");
        }
        note(TAG, "If accidental, Ctrl+C now and run: git reset HEAD");
        eprintln!();
    }
}

/// Resolve the commit message: AI when requested and available, else the user
/// message, else a deterministic summary of the staged change.
fn resolve_message(o: &Opts) -> String {
    if o.use_ai {
        if let Some(msg) = ai_commit_message(o.provider.as_deref(), &o.ai_context, TAG) {
            if !msg.trim().is_empty() {
                return msg;
            }
        }
        if !o.user_msg.is_empty() {
            return o.user_msg.clone();
        }
        note(TAG, "AI message unavailable; using a generated summary.");
    } else if !o.user_msg.is_empty() {
        return o.user_msg.clone();
    }
    staged_summary_message("update")
}
