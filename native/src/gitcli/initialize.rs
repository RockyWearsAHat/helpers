//! `git-initialize` — initialize a local repository, commit everything, and
//! push it to a remote, creating/updating the `origin` remote as needed.
//!
//!   git-initialize <remote-url> [branch=main] [message="Initial commit"]

use std::process::ExitCode;

use super::*;

const TAG: &str = "git-initialize";

pub fn run(args: &[String]) -> ExitCode {
    let remote_url = args.first().map(String::as_str).unwrap_or("");
    let branch = args.get(1).map(String::as_str).unwrap_or("main");
    let message = args.get(2).map(String::as_str).unwrap_or("Initial commit");

    if remote_url.is_empty() {
        println!("No remote URL provided. Please provide a remote URL as the first argument.");
        return ExitCode::from(1);
    }

    // `git init` is safe even if the repo already exists.
    if !git_inherit(&["init"]) {
        note(TAG, "git init failed.");
        return ExitCode::from(1);
    }
    git_inherit(&["add", "."]);
    if !git_inherit(&["commit", "-m", message]) {
        note(TAG, "git commit failed (nothing to commit?).");
        return ExitCode::from(1);
    }
    git_inherit(&["branch", "-m", branch]);

    // Add or update the origin remote.
    let remotes = git_out(&["remote"]).unwrap_or_default();
    let has_origin = remotes.lines().any(|l| l == "origin");
    if has_origin {
        git_inherit(&["remote", "set-url", "origin", remote_url]);
    } else {
        git_inherit(&["remote", "add", "origin", remote_url]);
    }

    if !git_inherit(&["push", "-u", "origin", branch]) {
        note(TAG, "git push failed.");
        return ExitCode::from(1);
    }

    // OSC-8 hyperlink so the remote URL is clickable in capable terminals.
    let link = format!("\x1b]8;;{remote_url}\x07Click here to view\x1b]8;;\x07");
    println!("Git repository initialized with message \"{message}\" {link}");
    ExitCode::SUCCESS
}
