//! `git-cs-grade` — objective structural rubric for CS2420 / CS3500 projects.
//!
//! Ported into `helpers-native` as a busybox-dispatched CLI so the single shipped
//! binary also grades projects (this was previously a separate `git-cs-grade`
//! crate/binary). The grading logic itself lives in the `cs_grade` library crate.
//!
//!   git-cs-grade [path] [--course cs2420|cs3500|auto] [--json]
//!
//! Without `--json` it writes `GRADE.md` in the project root and prints the same
//! report. Exit code is 0 when the grade is A+ (≥97), 2 otherwise — matching the
//! original so the `helpers grade` fix-loop can detect completion.

use std::path::Path;
use std::process::ExitCode;

use cs_grade::{evaluate, paths, project_label, report};

const USAGE: &str = "usage: git-cs-grade [path] [--course cs2420|cs3500|auto] [--json]";

/// Parsed command-line arguments for a grading run.
struct Args {
    root: String,
    course: String,
    as_json: bool,
}

/// Parse the post-program arguments, or `Err(code)` to exit early (help → 0).
fn parse(argv: &[String]) -> Result<Args, ExitCode> {
    let mut root = ".".to_string();
    let mut course = "auto".to_string();
    let mut as_json = false;

    let mut i = 0;
    while i < argv.len() {
        let a = &argv[i];
        match a.as_str() {
            "--course" => {
                i += 1;
                course = argv
                    .get(i)
                    .map(|s| s.to_lowercase())
                    .unwrap_or_else(|| "auto".into());
            }
            "--json" => as_json = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Err(ExitCode::SUCCESS);
            }
            other if !other.starts_with('-') => root = other.to_string(),
            _ => {}
        }
        i += 1;
    }
    Ok(Args {
        root,
        course,
        as_json,
    })
}

/// Run `git-cs-grade` against `args` (the arguments after the program name).
/// Returns exit code 0 for an A+ project, 2 otherwise, 1 on a usage/IO error.
pub fn run(args: &[String]) -> ExitCode {
    let parsed = match parse(args) {
        Ok(a) => a,
        Err(code) => return code,
    };

    let root = paths::resolve(&parsed.root);
    if !root.exists() {
        eprintln!("git-cs-grade: path not found: {}", root.display());
        return ExitCode::from(1);
    }

    let (grade, src_files, test_files) = evaluate(&root, &parsed.course);
    let pass = grade.pct >= 97.0;

    if parsed.as_json {
        println!("{}", report::json(&grade));
        return exit_code(pass);
    }

    let report_body = report::markdown(&grade, &project_label(&root), src_files, test_files);
    let out_path = root.join("GRADE.md");
    if let Err(e) = std::fs::write(&out_path, format!("{report_body}\n")) {
        eprintln!("git-cs-grade: failed to write {}: {e}", out_path.display());
        return ExitCode::from(1);
    }

    println!("{report_body}");
    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let wrote = paths::relative(&cwd, &out_path);
    let wrote = if wrote.is_empty() {
        out_path.display().to_string()
    } else {
        wrote
    };
    println!("\nWrote {wrote}");
    exit_code(pass)
}

/// Map the A+ pass/fail flag to the documented exit code (0 = A+, 2 = below).
fn exit_code(pass: bool) -> ExitCode {
    if pass {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    }
}
