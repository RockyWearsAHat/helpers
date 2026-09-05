//! Toolchain doctor — reports available development tools.

use std::io::Write;
use std::process::{Command, ExitCode};

pub fn run() -> ExitCode {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    let _ = writeln!(handle, "Toolchain Status:");

    let tools = [
        ("Rust", "rustc --version"),
        ("Node.js", "node --version"),
        ("Git", "git --version"),
        ("Go", "go version"),
    ];

    for (name, cmd_str) in &tools {
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.is_empty() {
            let _ = writeln!(handle, "{name}: not found");
            continue;
        }

        match Command::new(parts[0])
            .args(&parts[1..])
            .output()
        {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if version.is_empty() {
                    let _ = writeln!(handle, "{name}: found");
                } else {
                    let _ = writeln!(handle, "{name}: found ({})", version);
                }
            }
            _ => {
                let _ = writeln!(handle, "{name}: not found");
            }
        }
    }

    ExitCode::SUCCESS
}
