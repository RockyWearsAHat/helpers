//! `git-scan-for-leaked-envs` — scan the repository for leaked secrets,
//! credentials, and sensitive files using deterministic pattern matching.
//!
//! Fully deterministic (no AI). Reports sensitive tracked files and regex
//! matches across tracked content, optionally including history, in human or
//! JSON form. Exit codes: 0 = clean, 1 = secrets found, 2 = error.

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use regex::RegexBuilder;
use serde_json::json;

use super::*;

/// Common secret/credential regex patterns (case-insensitive).
const SECRET_PATTERNS: &[&str] = &[
    r"AKIA[0-9A-Z]{16}",
    r"[a-zA-Z0-9_-]*api[_-]?key[a-zA-Z0-9_-]*\s*[:=]",
    r"[a-zA-Z0-9_-]*apikey[a-zA-Z0-9_-]*\s*[:=]",
    r"ghp_[a-zA-Z0-9]{36}",
    r"gho_[a-zA-Z0-9]{36}",
    r"ghu_[a-zA-Z0-9]{36}",
    r"ghs_[a-zA-Z0-9]{36}",
    r"github_pat_[a-zA-Z0-9]{22}_[a-zA-Z0-9]{59}",
    r"xox[baprs]-[0-9]{10,13}-[0-9]{10,13}[a-zA-Z0-9-]*",
    r"sk-[a-zA-Z0-9]{48}",
    r"sk-proj-[a-zA-Z0-9_-]{80,}",
    r"[a-zA-Z0-9_-]*password[a-zA-Z0-9_-]*\s*[:=]",
    r"[a-zA-Z0-9_-]*secret[a-zA-Z0-9_-]*\s*[:=]",
    r"[a-zA-Z0-9_-]*private[_-]?key[a-zA-Z0-9_-]*\s*[:=]",
    r"postgres://[^:]+:[^@]+@",
    r"mysql://[^:]+:[^@]+@",
    r"mongodb(\+srv)?://[^:]+:[^@]+@",
    r"redis://[^:]+:[^@]+@",
    r"AZURE[_-]?[A-Z_]*[_-]?(KEY|SECRET|TOKEN|PASSWORD)",
    r"[a-zA-Z0-9_-]*AWS[_-]?SECRET[a-zA-Z0-9_-]*",
    r"-----BEGIN (RSA |DSA |EC |OPENSSH |PGP )?PRIVATE KEY-----",
    r"-----BEGIN CERTIFICATE-----",
    r"bearer\s+[a-zA-Z0-9_\-.]+",
    r"authorization:\s*(bearer|basic)\s+[a-zA-Z0-9_\-.]+",
];

/// Exact basenames that commonly hold secrets.
const SENSITIVE_NAMES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.development",
    ".env.production",
    ".env.staging",
    ".env.test",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    ".npmrc",
    ".pypirc",
    ".netrc",
    ".htpasswd",
    "credentials",
    "secrets.json",
    "secrets.yml",
    "secrets.yaml",
    "config.json",
    "config.yml",
];

/// Suffixes that commonly hold secrets/keys.
const SENSITIVE_SUFFIXES: &[&str] = &[".pem", ".key", ".p12", ".pfx", ".jks", ".log"];

const HELP: &str = r#"git-scan-for-leaked-envs - Scan repository for leaked secrets and credentials

USAGE:
    git-scan-for-leaked-envs [options]

DESCRIPTION:
    Scans the current repository for potentially leaked environment variables,
    API keys, secrets, and other sensitive data using deterministic pattern
    matching. Suggests git-help-i-pushed-an-env to clean up history.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Show detailed scan output
    --json                  Output results in JSON format
    --no-recommend          Skip recommendations section
    --include-history       Scan git history (not just current files)
    --output FILE           Write results to file

EXIT CODES:
    0   No secrets found
    1   Secrets found (action required)
    2   Error during scan
"#;

struct Opts {
    verbose: bool,
    json: bool,
    recommend: bool,
    include_history: bool,
    output: Option<String>,
}

pub fn run(args: &[String]) -> ExitCode {
    let mut o = Opts {
        verbose: false,
        json: false,
        recommend: true,
        include_history: false,
        output: None,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return ExitCode::SUCCESS;
            }
            "-v" | "--verbose" => o.verbose = true,
            "--json" => o.json = true,
            "--no-recommend" => o.recommend = false,
            "--include-history" => o.include_history = true,
            "--output" => {
                i += 1;
                o.output = args.get(i).cloned();
            }
            other => {
                eprintln!("[ERROR] Unknown option: {other}");
                print!("{HELP}");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }

    if !git_ok(&["rev-parse", "--is-inside-work-tree"]) {
        eprintln!("[ERROR] Not inside a git repository");
        return ExitCode::from(2);
    }
    let repo_root = match git_out(&["rev-parse", "--show-toplevel"]) {
        Some(r) => r,
        None => {
            eprintln!("[ERROR] Could not resolve repository root");
            return ExitCode::from(2);
        }
    };

    let tracked = git_out(&["ls-files"]).unwrap_or_default();
    let files: Vec<&str> = tracked.lines().filter(|l| !l.is_empty()).collect();

    let sensitive_files = scan_sensitive_files(&files);
    let pattern_matches = scan_patterns(Path::new(&repo_root), &files, o.verbose);
    let history_matches = if o.include_history {
        scan_history()
    } else {
        Vec::new()
    };

    let found =
        !sensitive_files.is_empty() || !pattern_matches.is_empty() || !history_matches.is_empty();

    let report = render(
        &o,
        &repo_root,
        &sensitive_files,
        &pattern_matches,
        &history_matches,
        found,
    );

    match &o.output {
        Some(path) => {
            if let Err(e) = fs::write(path, &report) {
                eprintln!("[ERROR] Could not write {path}: {e}");
                return ExitCode::from(2);
            }
            eprintln!("[INFO] Results written to: {path}");
        }
        None => print!("{report}"),
    }

    if found {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Tracked files whose basename matches a sensitive name/suffix.
fn scan_sensitive_files(files: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = files
        .iter()
        .filter(|f| {
            let base = Path::new(f)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            SENSITIVE_NAMES.contains(&base)
                || base.starts_with(".env")
                || SENSITIVE_SUFFIXES.iter().any(|s| base.ends_with(s))
        })
        .map(|f| f.to_string())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// `file:line:match` hits of any secret pattern across tracked content.
fn scan_patterns(repo_root: &Path, files: &[&str], verbose: bool) -> Vec<String> {
    let regexes: Vec<regex::Regex> = SECRET_PATTERNS
        .iter()
        .filter_map(|p| RegexBuilder::new(p).case_insensitive(true).build().ok())
        .collect();

    let mut results: Vec<String> = Vec::new();
    for f in files {
        let path = repo_root.join(f);
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        if meta.len() > 2_000_000 {
            continue; // skip very large files
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue; // skip binary/non-UTF8
        };
        for (lineno, line) in content.lines().enumerate() {
            if regexes.iter().any(|re| re.is_match(line)) {
                results.push(format!("{f}:{}:{}", lineno + 1, line.trim()));
                if results.len() >= 100 {
                    break;
                }
            }
        }
        if verbose {
            eprintln!("[DEBUG] scanned {f}");
        }
        if results.len() >= 100 {
            break;
        }
    }
    results.sort();
    results.dedup();
    results
}

/// Secret hits across the diffs of recent commits (bounded for speed).
fn scan_history() -> Vec<String> {
    let regexes: Vec<regex::Regex> = SECRET_PATTERNS
        .iter()
        .filter_map(|p| RegexBuilder::new(p).case_insensitive(true).build().ok())
        .collect();

    let commits = git_out(&["log", "--all", "--pretty=format:%H"]).unwrap_or_default();
    let mut results: Vec<String> = Vec::new();
    for commit in commits.lines().take(50) {
        let Some(diff) = git_out(&["show", commit]) else {
            continue;
        };
        let mut hits = 0;
        for line in diff.lines() {
            if regexes.iter().any(|re| re.is_match(line)) {
                results.push(format!("commit:{commit} - {}", line.trim()));
                hits += 1;
                if hits >= 5 {
                    break;
                }
            }
        }
        if results.len() >= 50 {
            break;
        }
    }
    results.sort();
    results.dedup();
    results
}

fn render(
    o: &Opts,
    repo_root: &str,
    sensitive: &[String],
    patterns: &[String],
    history: &[String],
    found: bool,
) -> String {
    if o.json {
        let v = json!({
            "repository": repo_root,
            "scan_time": chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "sensitive_files": sensitive,
            "pattern_matches": patterns,
            "history_matches": history,
        });
        return format!("{}\n", serde_json::to_string_pretty(&v).unwrap_or_default());
    }

    let mut s = String::new();
    if !found {
        s.push_str(&format!(
            "\n\u{2713} Clean - no secrets detected in {repo_root}\n\n"
        ));
        return s;
    }
    s.push_str(&format!("\n\u{26a0} Secrets found in {repo_root}\n\n"));

    if !sensitive.is_empty() {
        s.push_str("Sensitive files:\n");
        for f in sensitive {
            s.push_str(&format!("  {f}\n"));
        }
        s.push('\n');
    }
    if !patterns.is_empty() {
        s.push_str("Pattern matches:\n");
        for m in patterns.iter().take(10) {
            s.push_str(&format!("  {m}\n"));
        }
        if patterns.len() > 10 {
            s.push_str(&format!("  ({} more)\n", patterns.len() - 10));
        }
        s.push('\n');
    }
    if o.include_history && !history.is_empty() {
        s.push_str("In git history:\n");
        for m in history.iter().take(5) {
            s.push_str(&format!("  {m}\n"));
        }
        s.push('\n');
    }
    if o.recommend {
        s.push_str("Next: Rotate any real credentials above, then run: git help-i-pushed-an-env\n");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_files_match_names_globs_and_suffixes() {
        let files = vec![
            "src/main.rs",
            ".env",
            "config/.env.production",
            "deploy/server.pem",
            "keys/id_ed25519",
            "README.md",
        ];
        let found = scan_sensitive_files(&files);
        assert!(found.contains(&".env".to_string()));
        assert!(found.contains(&"config/.env.production".to_string()));
        assert!(found.contains(&"deploy/server.pem".to_string()));
        assert!(found.contains(&"keys/id_ed25519".to_string()));
        assert!(!found.contains(&"src/main.rs".to_string()));
        assert!(!found.contains(&"README.md".to_string()));
    }

    #[test]
    fn secret_patterns_compile_and_catch_known_tokens() {
        let regexes: Vec<regex::Regex> = SECRET_PATTERNS
            .iter()
            .map(|p| RegexBuilder::new(p).case_insensitive(true).build().unwrap())
            .collect();
        let hit = |line: &str| regexes.iter().any(|re| re.is_match(line));
        assert!(hit("AKIAIOSFODNN7EXAMPLE"));
        assert!(hit("ghp_0123456789abcdef0123456789abcdef0123"));
        assert!(hit("password = hunter2"));
        assert!(hit("postgres://user:pass@host/db"));
        assert!(hit("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(!hit("let total = sum + count;"));
        assert!(!hit("plain prose with no secrets"));
    }
}
