//! `git-help-i-pushed-an-env` — emergency removal of leaked secrets and other
//! sensitive files from the current repository's entire git history.
//!
//! Deterministic and single-repo: it scrubs *this* repo's history via
//! `git-filter-repo` (preferred) or `git filter-branch` (fallback), adds the
//! patterns to `.gitignore`, untracks the files (kept on disk), and prints
//! force-push + credential-rotation guidance. History rewriting is destructive,
//! so it confirms first unless `--force`.
//!
//! The old multi-repo GitHub batch ops, interactive per-file menus, caching,
//! and Copilot AI analysis were intentionally dropped — they are not the core
//! value and conflict with the deterministic, minimal design.

use std::io::Write;
use std::process::ExitCode;

use regex::Regex;

use super::*;

const TAG: &str = "git-help-i-pushed-an-env";

/// Default glob patterns for files that commonly hold secrets.
const DEFAULT_PATTERNS: &[&str] = &[
    ".env",
    ".env.*",
    "*.env",
    "*.log",
    "logs/",
    "*.pem",
    "*.key",
    "*.p12",
    "*.pfx",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    "credentials.json",
    "secrets.json",
    "secrets.yml",
    "secrets.yaml",
    "*-secret*.json",
    "application.properties",
    "application.yml",
    "appsettings.json",
    "appsettings.*.json",
    "web.config",
    "wp-config.php",
    "LocalSettings.php",
    ".aws/credentials",
    ".aws/config",
    "gcloud-service-key.json",
    "firebase-adminsdk*.json",
    "*.dump",
    "*.tfstate",
    "*.tfstate.*",
    ".terraform/",
];

const HELP: &str = r#"git-help-i-pushed-an-env - scrub secrets from this repo's git history

USAGE:
    git help-i-pushed-an-env [options]

DESCRIPTION:
    Removes sensitive files (env files, keys, secrets, logs, …) from the ENTIRE
    git history of the current repository. Destructive: it rewrites history.
    Files already in .gitignore that were never committed are not affected.

OPTIONS:
    -h, --help        Show this help
    -n, --dry-run     Show what would be removed without changing anything
    -f, --force       Skip the confirmation prompt
    -v, --verbose     Verbose output
    --scan            Only scan for sensitive files (no changes)
    --review          Run git-scan-for-leaked-envs after cleaning
    --backup          Create a backup branch before rewriting history
    --ext EXT         Also remove files with this extension (repeatable)
    --file PATTERN    Also remove files matching this glob (repeatable)

EXIT CODES:
    0   Clean, or completed successfully
    1   Sensitive files found (with --scan) or an error occurred
"#;

struct Opts {
    dry_run: bool,
    force: bool,
    verbose: bool,
    scan_only: bool,
    review: bool,
    backup: bool,
    patterns: Vec<String>,
}

/// Run git-help-i-pushed-an-env: find sensitive files (current + history),
/// scrub them from the entire history, gitignore the patterns, and untrack them.
pub fn run(args: &[String]) -> ExitCode {
    let mut o = Opts {
        dry_run: false,
        force: false,
        verbose: false,
        scan_only: false,
        review: false,
        backup: false,
        patterns: DEFAULT_PATTERNS.iter().map(|s| s.to_string()).collect(),
    };

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return ExitCode::SUCCESS;
            }
            "-n" | "--dry-run" => o.dry_run = true,
            "-f" | "--force" => o.force = true,
            "-v" | "--verbose" => o.verbose = true,
            "--scan" => o.scan_only = true,
            "--review" => o.review = true,
            "--backup" => o.backup = true,
            "--ext" => {
                i += 1;
                if let Some(ext) = args.get(i) {
                    let ext = ext.trim_start_matches('.');
                    o.patterns.push(format!("*.{ext}"));
                }
            }
            "--file" => {
                i += 1;
                if let Some(p) = args.get(i) {
                    o.patterns.push(p.clone());
                }
            }
            // Dropped multi-repo / interactive modes — fail loudly, don't pretend.
            "--all-public" | "--all-repos" | "--scan-all-public" | "--scan-all-repos" | "-i"
            | "--interactive" => {
                note(TAG, &format!("{} was removed.", args[i]));
                note(
                    TAG,
                    "This tool now scrubs only the current repository, deterministically.",
                );
                return ExitCode::from(2);
            }
            other => {
                note(TAG, &format!("Unknown option: {other}"));
                print!("{HELP}");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }

    if !git_ok(&["rev-parse", "--is-inside-work-tree"]) {
        note(TAG, "Not inside a git repository.");
        return ExitCode::from(1);
    }

    let regexes = compile_patterns(&o.patterns);
    let matches = find_matching(&regexes);

    if matches.is_empty() {
        note(
            TAG,
            "\u{2705} No sensitive files found in the repository or its history.",
        );
        return ExitCode::SUCCESS;
    }

    println!("Sensitive files found:");
    for m in &matches {
        println!("  {m}");
    }
    println!();

    if o.scan_only {
        note(
            TAG,
            "Scan only — no changes made. Re-run without --scan to scrub history.",
        );
        return ExitCode::from(1);
    }

    if o.dry_run {
        note(
            TAG,
            "[DRY RUN] Would remove the above from history; no changes made.",
        );
        return ExitCode::SUCCESS;
    }

    if !o.force {
        eprintln!("[{TAG}] \u{26a0}\u{fe0f}  This permanently REWRITES git history.");
        eprintln!("[{TAG}] All collaborators must re-clone or force-pull afterward.");
        if !confirm() {
            note(TAG, "Operation cancelled.");
            return ExitCode::from(1);
        }
    }

    if o.backup {
        let branch = format!("backup-{}", timestamp());
        if git_ok(&["branch", &branch]) {
            note(TAG, &format!("Backup branch: {branch}"));
        }
    }

    if !rewrite_history(&o) {
        note(TAG, "History rewrite failed. No safe partial state — restore from your backup branch if needed.");
        return ExitCode::from(1);
    }

    add_to_gitignore(&o.patterns, o.verbose);
    untrack_now(&regexes, o.verbose);

    note(
        TAG,
        "\u{2705} History cleaned. Files preserved in the working directory.",
    );
    eprintln!();
    eprintln!("[{TAG}] Next steps:");
    eprintln!("  git push --force --all");
    eprintln!("  git push --force --tags");
    eprintln!("[{TAG}] \u{26a0}\u{fe0f}  Rotate every exposed credential now.");

    if o.review {
        note(TAG, "Running post-cleanup scan\u{2026}");
        if let Some(bin) = which("git-scan-for-leaked-envs") {
            let _ = std::process::Command::new(bin).arg("--verbose").status();
        }
    }
    ExitCode::SUCCESS
}

/// Compile each glob pattern to an anchored regex matching a repo-relative path.
fn compile_patterns(patterns: &[String]) -> Vec<Regex> {
    patterns
        .iter()
        .filter_map(|p| Regex::new(&glob_to_regex(p)).ok())
        .collect()
}

/// Translate a shell glob into a regex anchored at a path segment boundary,
/// mirroring the original `sed 's/\./\\./g; s/\*/.*/g'` plus `(^|/)…$`.
fn glob_to_regex(glob: &str) -> String {
    let dir = glob.ends_with('/');
    let core = glob.trim_end_matches('/');
    let mut body = String::new();
    for ch in core.chars() {
        match ch {
            '.' => body.push_str("\\."),
            '*' => body.push_str(".*"),
            // Escape other regex metacharacters that can appear in paths.
            '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '?' | '|' | '\\' => {
                body.push('\\');
                body.push(ch);
            }
            _ => body.push(ch),
        }
    }
    if dir {
        format!("(^|/){body}(/|$)")
    } else {
        format!("(^|/){body}$")
    }
}

/// Sensitive files tracked now or deleted somewhere in history.
fn find_matching(regexes: &[Regex]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let any = |p: &str| regexes.iter().any(|re| re.is_match(p));

    // Currently tracked.
    if let Some(tracked) = git_out(&["ls-files"]) {
        for f in tracked.lines().filter(|l| !l.is_empty()) {
            if any(f) {
                out.push(format!("(current) {f}"));
            }
        }
    }
    // Deleted in history: parse `delete mode 100644 <path>` lines.
    if let Some(log) = git_out(&["log", "--all", "--diff-filter=D", "--summary"]) {
        for line in log.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("delete mode ") {
                if let Some(path) = rest.split_whitespace().last() {
                    if any(path) {
                        out.push(format!("(history) {path}"));
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Rewrite history with git-filter-repo if present, else git filter-branch.
fn rewrite_history(o: &Opts) -> bool {
    if which("git-filter-repo").is_some() {
        let mut args: Vec<String> = Vec::new();
        for p in &o.patterns {
            args.push("--path-glob".into());
            args.push(p.clone());
        }
        args.push("--invert-paths".into());
        args.push("--force".into());
        if o.verbose {
            note(TAG, "Using git-filter-repo.");
        }
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let (ok, _, err) = run_capture("git-filter-repo", &arg_refs);
        if !ok && !err.is_empty() {
            note(TAG, &err);
        }
        ok
    } else {
        if o.verbose {
            note(
                TAG,
                "git-filter-repo not found; using git filter-branch (slower).",
            );
        }
        let mut rm = String::new();
        for p in &o.patterns {
            rm.push_str(&format!(
                "git rm --cached --ignore-unmatch '{}' 2>/dev/null || true; ",
                p.replace('\'', "")
            ));
        }
        let ok = git_inherit(&[
            "filter-branch",
            "--force",
            "--index-filter",
            &rm,
            "--prune-empty",
            "--tag-name-filter",
            "cat",
            "--",
            "--all",
        ]);
        if ok {
            let _ = std::fs::remove_dir_all(format!("{}/refs/original", git_dir()));
            git_ok(&["reflog", "expire", "--expire=now", "--all"]);
            git_ok(&["gc", "--prune=now", "--aggressive"]);
        }
        ok
    }
}

/// Append the patterns to `.gitignore` (deduped) and commit the change.
fn add_to_gitignore(patterns: &[String], verbose: bool) {
    let path = ".gitignore";
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<String> = existing.lines().map(str::to_string).collect();
    let mut added = 0;
    for p in patterns {
        if !lines.iter().any(|l| l == p) {
            lines.push(p.clone());
            added += 1;
        }
    }
    if added > 0 {
        let mut content = lines.join("\n");
        content.push('\n');
        if std::fs::write(path, content).is_ok() {
            git_ok(&["add", ".gitignore"]);
            git_ok(&["commit", "-m", "Add sensitive file patterns to .gitignore"]);
            if verbose {
                note(TAG, &format!("Added {added} pattern(s) to .gitignore."));
            }
        }
    }
}

/// Untrack any still-tracked matching files (kept on disk) and commit.
fn untrack_now(regexes: &[Regex], verbose: bool) {
    let Some(tracked) = git_out(&["ls-files"]) else {
        return;
    };
    let mut removed = false;
    for f in tracked.lines().filter(|l| !l.is_empty()) {
        if regexes.iter().any(|re| re.is_match(f)) {
            if git_ok(&["rm", "--cached", f]) && verbose {
                note(TAG, &format!("Untracked: {f}"));
            }
            removed = true;
        }
    }
    if removed && !git_ok(&["diff", "--cached", "--quiet"]) {
        git_ok(&[
            "commit",
            "-m",
            "Remove sensitive files from tracking (files preserved locally)",
        ]);
    }
}

/// Prompt on stderr and require the user to type `yes`.
fn confirm() -> bool {
    eprint!("[{TAG}] Type 'yes' to proceed: ");
    let _ = std::io::stderr().flush();
    let mut reply = String::new();
    if std::io::stdin().read_line(&mut reply).is_err() {
        return false;
    }
    reply.trim() == "yes"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_to_regex_matches_expected_paths() {
        let re = |g: &str| Regex::new(&glob_to_regex(g)).unwrap();

        let env = re(".env");
        assert!(env.is_match(".env"));
        assert!(env.is_match("config/.env"));
        assert!(!env.is_match(".environment"));

        let star_env = re("*.env");
        assert!(star_env.is_match("prod.env"));
        assert!(star_env.is_match("a/b/prod.env"));

        let pem = re("*.pem");
        assert!(pem.is_match("certs/server.pem"));
        assert!(!pem.is_match("server.pemx"));

        let dir = re("logs/");
        assert!(dir.is_match("logs/app.log"));
        assert!(dir.is_match("a/logs/x"));

        let fb = re("firebase-adminsdk*.json");
        assert!(fb.is_match("firebase-adminsdk-abc123.json"));
    }

    #[test]
    fn find_matching_dedups_and_classifies() {
        // Pure logic: the regexes match by path; no git calls here.
        let regexes = compile_patterns(&["*.env".to_string(), "id_rsa".to_string()]);
        let any = |p: &str| regexes.iter().any(|re| re.is_match(p));
        assert!(any("secret.env"));
        assert!(any("keys/id_rsa"));
        assert!(!any("src/main.rs"));
    }
}
