//! `git-get` — initialize a local repository from a remote and pull a branch.
//!
//!   git-get <remote-url> [branch=main]

use std::process::ExitCode;

use super::*;

const TAG: &str = "git-get";

pub fn run(args: &[String]) -> ExitCode {
    let repo = args.first().map(String::as_str).unwrap_or("");
    let branch = args.get(1).map(String::as_str).unwrap_or("main");

    if repo.is_empty() {
        note(
            TAG,
            "No remote URL provided. Usage: git-get <remote-url> [branch]",
        );
        return ExitCode::from(1);
    }

    git_inherit(&["init"]);
    git_inherit(&["remote", "add", "origin", repo]);
    if !git_inherit(&["pull", "origin", branch]) {
        note(TAG, &format!("Failed to pull '{branch}' from {repo}."));
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
