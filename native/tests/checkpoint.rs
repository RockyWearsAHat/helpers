//! Integration test for the checkpoint tool's git logic (manual-message path,
//! so no AI/Copilot dependency). Exercises commit, the clean-tree no-op, and the
//! branch assertion.

use std::fs;
use std::process::Command;

use helpers_native::tools::checkpoint;
use serde_json::json;

fn git(args: &[&str], cwd: &std::path::Path) {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
}

#[test]
fn commits_with_manual_message_then_noops_when_clean() {
    let dir = std::env::temp_dir().join(format!("helpers-cp-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    git(&["init", "-q"], &dir);
    git(&["config", "user.email", "t@t.t"], &dir);
    git(&["config", "user.name", "Tester"], &dir);
    git(&["commit", "--allow-empty", "-q", "-m", "root"], &dir);
    fs::write(dir.join("a.txt"), "hello\n").unwrap();

    // Commit with a manual message.
    let res = checkpoint::run(&json!({
        "cwd": dir.to_string_lossy(),
        "message": "add a.txt",
    }))
    .expect("checkpoint ok");
    let text = &res[0].text;
    assert!(text.starts_with("Committed "), "got: {text}");
    assert!(
        text.contains("add a.txt") || text.contains("a.txt"),
        "got: {text}"
    );

    // Tree is now clean -> no-op.
    let res2 = checkpoint::run(&json!({ "cwd": dir.to_string_lossy() })).unwrap();
    assert!(
        res2[0].text.contains("Nothing to commit"),
        "got: {}",
        res2[0].text
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn branch_assertion_fails_on_mismatch() {
    let dir = std::env::temp_dir().join(format!("helpers-cp2-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    git(&["init", "-q", "-b", "main"], &dir);
    git(&["config", "user.email", "t@t.t"], &dir);
    git(&["config", "user.name", "Tester"], &dir);
    git(&["commit", "--allow-empty", "-q", "-m", "root"], &dir);
    fs::write(dir.join("a.txt"), "x\n").unwrap();

    let err = checkpoint::run(&json!({
        "cwd": dir.to_string_lossy(),
        "branch": "feature/nope",
        "message": "should not commit",
    }))
    .unwrap_err();
    assert!(err.contains("Branch assertion failed"), "got: {err}");

    let _ = fs::remove_dir_all(&dir);
}
