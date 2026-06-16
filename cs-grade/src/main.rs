//! Command-line front end for the cs-grade library.
//!
//!   git-cs-grade [path] [--course cs2420|cs3500|auto] [--json]
//!
//! Without `--json` it writes GRADE.md in the project root and prints the same
//! report. Exit code is 0 when the grade is A+ (≥97), 2 otherwise — matching the
//! original so the `gsh grade` fix-loop can detect completion.

use std::path::Path;
use std::process::ExitCode;

use cs_grade::{evaluate, paths, project_label, report};

const USAGE: &str = "usage: git-cs-grade [path] [--course cs2420|cs3500|auto] [--json]";

struct Args {
    root: String,
    course: String,
    as_json: bool,
}

/// Parse argv, or `Err(code)` to exit early (help → 0, bad path handled later).
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

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse(&argv) {
        Ok(a) => a,
        Err(code) => return code,
    };

    let root = paths::resolve(&args.root);
    if !root.exists() {
        eprintln!("git-cs-grade: path not found: {}", root.display());
        return ExitCode::from(1);
    }

    let (grade, src_files, test_files) = evaluate(&root, &args.course);
    let pass = grade.pct >= 97.0;

    if args.as_json {
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

fn exit_code(pass: bool) -> ExitCode {
    if pass {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    }
}
