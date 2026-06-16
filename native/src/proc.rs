//! Shared subprocess helper: run a command with a bounded timeout, draining
//! stdout/stderr on threads so a child can't deadlock on a full pipe.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use wait_timeout::ChildExt;

/// Run `cmd args` (optional `cwd`, extra `env`), killing it after `timeout_s`.
/// Returns `(success, stdout, stderr)`; a spawn failure is `(false, "", "")`.
pub fn run_capture(
    cmd: &str,
    args: &[&str],
    cwd: Option<&Path>,
    env: &[(&str, &str)],
    timeout_s: u64,
) -> (bool, String, String) {
    let mut command = Command::new(cmd);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    for (k, v) in env {
        command.env(k, v);
    }
    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(_) => return (false, String::new(), String::new()),
    };
    let mut out = child.stdout.take().unwrap();
    let mut err = child.stderr.take().unwrap();
    let oh = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let eh = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });
    let success = match child.wait_timeout(Duration::from_secs(timeout_s)) {
        Ok(Some(status)) => status.success(),
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            false
        }
        Err(_) => false,
    };
    (
        success,
        oh.join().unwrap_or_default(),
        eh.join().unwrap_or_default(),
    )
}
