//! Understanding-class defect detection, tested through the REAL engine (the built binary,
//! invoked exactly as an agent invokes it). This is the acceptance test for the owner's north
//! star: the linter flags code that is *bad by CS principles* — not because a rule string was
//! hand-written for it, but because it READ a principle described in prose (the machine-global
//! `corpus/`) and understood it well enough to recognise the construct the principle forbids.
//!
//! The mechanism under test (LINTER.md, "Rules from understanding — the probe bridge"): a
//! structural AST *probe* is coded machinery (dead-code-after-return, unwrap-on-fallible, an
//! over-long function, a pub item with no doc, an unexplained numeric literal, a single-letter
//! binding, a secret-shaped literal). WHICH probes are live, and their advice, come entirely
//! from reading the corpus: a principle's prose is understood in the HDC substrate and bound to
//! the probe whose concept it means. Delete the corpus and every probe goes dark — the checks
//! are learned, the tree-walk is programmed.
//!
//! The contract asserted here:
//!   * a genuinely TERRIBLE (but valid) Rust file is flagged for every understanding-class
//!     defect below — none of which has a hand-written rule string anywhere;
//!   * a genuinely EXCELLENT Rust file has ZERO findings (no false positives).
//!
//! Hermetic: caches/home redirected into the test's temp dir, network learning disabled, and the
//! corpus + an empty `lint-index/` planted in the project so `data_root` resolves locally.

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// A throwaway project with the isolated cache/home a hermetic lint run needs.
struct TestProject {
    root: PathBuf,
}

impl TestProject {
    /// Create an empty git project under a unique temp dir.
    fn new(name: &str) -> TestProject {
        let root = std::env::temp_dir().join(format!("understand-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp project dir");
        let ok = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .status()
            .expect("git available")
            .success();
        assert!(ok, "git init failed");
        TestProject { root }
    }

    /// Write a file (creating parents) relative to the project root.
    fn write(&self, rel: &str, content: &str) -> &Self {
        let path = self.root.join(rel);
        fs::create_dir_all(path.parent().expect("has parent")).expect("mkdir");
        fs::write(path, content).expect("write fixture file");
        self
    }

    /// Run `helpers-native call lint` on this project and return the rendered verdict text.
    fn lint(&self) -> String {
        let request = format!(r#"{{"root":{:?}}}"#, self.root.to_string_lossy());
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_helpers-native"));
        cmd.args(["call", "lint"])
            .current_dir(&self.root)
            .env("HELPERS_LINT_MODELS", self.root.join(".test-models"))
            .env("HOME", self.root.join(".test-home"))
            .env("HELPERS_LINT_OFFLINE", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().expect("binary runs");
        child
            .stdin
            .as_mut()
            .expect("stdin piped")
            .write_all(request.as_bytes())
            .expect("request written");
        let out = child.wait_with_output().expect("binary exits");
        assert!(
            out.status.success(),
            "`call lint` must exit cleanly.\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        let raw = String::from_utf8_lossy(&out.stdout).into_owned();
        let json: serde_json::Value = serde_json::from_str(raw.trim())
            .unwrap_or_else(|e| panic!("tool output is protocol JSON ({e}): {raw}"));
        json["content"][0]["text"].as_str().expect("text content").to_string()
    }
}

/// The machine-global principles corpus (language-agnostic prose the AI reads). This is DATA —
/// the same file that ships in `<repo>/corpus/`. Planted into the hermetic project so the run
/// reads the exact principles under test.
const PRINCIPLES: &str = include_str!("../../corpus/principles.md");

/// True when `rule` is flagged within `file`'s section of the verdict.
fn flagged_in(verdict: &str, file: &str, rule: &str) -> bool {
    verdict
        .lines()
        .skip_while(|l| !l.contains(file))
        .skip(1)
        .take_while(|l| !l.trim().is_empty())
        .any(|l| l.to_lowercase().contains(&rule.to_lowercase()))
}

/// A deliberately awful — but syntactically valid — Rust file, one planted defect per class.
const TERRIBLE: &str = r#"use std::process::Command;

pub fn handle(x: i32) -> i32 {
    let n = 8675309;
    if x > n {
        return x + x;
        let leftover = x * 3;
        drop(leftover);
    }
    let d = std::fs::read_to_string("config.txt");
    let _ = d;
    let parsed: i32 = "12".parse().unwrap();
    x + parsed
}

pub fn run_shell(user_input: &str) -> std::io::Result<()> {
    Command::new("sh").arg("-c").arg(format!("ls {}", user_input)).status()?;
    Ok(())
}

pub fn connect() -> String {
    let api_key = "sk_live_9wQ2Zx7Lm4Rf8Tv1Nb6Hc3Yd0Ke5Pj";
    api_key.to_string()
}

pub fn summarize_first(values: &[i32]) -> i32 {
    let mut total = 0;
    total += values.len() as i32;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total += 1;
    total
}
"#;

/// A genuinely clean Rust file: documented public items, named constants, descriptive names,
/// explicit error handling, small functions, no secrets, no dead code.
const EXCELLENT: &str = r#"//! A small, clean configuration helper.

/// Maximum number of connection attempts before giving up.
const MAX_ATTEMPTS: u32 = 5;

/// Read the trimmed contents of the file at `path`, or an I/O error if it cannot be read.
pub fn read_config(path: &str) -> std::io::Result<String> {
    let contents = std::fs::read_to_string(path)?;
    Ok(contents.trim().to_string())
}

/// Call `attempt` up to [`MAX_ATTEMPTS`] times, returning `true` on the first success.
pub fn with_retries<F: Fn() -> bool>(attempt: F) -> bool {
    let mut remaining = MAX_ATTEMPTS;
    while remaining > 0 {
        if attempt() {
            return true;
        }
        remaining -= 1;
    }
    false
}
"#;

/// Build a hermetic project carrying the principles corpus and the two fixture files.
fn project(name: &str) -> TestProject {
    let p = TestProject::new(name);
    // An empty sources registry anchors `data_root` at the project and keeps the run off the
    // real network (exactly as the behavior suite does).
    p.write("lint-index/sources.json", r#"{"version": 3, "sources": []}"#);
    p.write("corpus/principles.md", PRINCIPLES);
    p
}

/// The TERRIBLE file is flagged for every understanding-class defect — each derived from reading
/// the corpus, none hand-written. This is the required floor for this cycle.
#[test]
fn terrible_file_is_flagged_for_every_understanding_class_defect() {
    let p = project("terrible");
    p.write("terrible.rs", TERRIBLE);
    let v = p.lint();

    // The seven structural classes + secret (the required floor).
    assert!(flagged_in(&v, "terrible.rs", "dead_code"), "dead code after return:\n{v}");
    assert!(flagged_in(&v, "terrible.rs", "swallow"), "swallowed error (let _ = fallible):\n{v}");
    assert!(flagged_in(&v, "terrible.rs", "unwrap") || flagged_in(&v, "terrible.rs", "swallow"),
        "unwrap on a fallible call:\n{v}");
    assert!(flagged_in(&v, "terrible.rs", "god_function") || flagged_in(&v, "terrible.rs", "long_function"),
        "over-long god function:\n{v}");
    assert!(flagged_in(&v, "terrible.rs", "undocumented") || flagged_in(&v, "terrible.rs", "document"),
        "undocumented public item:\n{v}");
    assert!(flagged_in(&v, "terrible.rs", "magic"), "magic number:\n{v}");
    assert!(flagged_in(&v, "terrible.rs", "descriptive") || flagged_in(&v, "terrible.rs", "single_letter"),
        "non-descriptive single-letter name:\n{v}");
    assert!(flagged_in(&v, "terrible.rs", "secret"), "hardcoded secret:\n{v}");
}

/// The EXCELLENT file has ZERO findings — no false positives from any probe.
#[test]
fn excellent_file_has_no_findings() {
    let p = project("excellent");
    p.write("excellent.rs", EXCELLENT);
    let v = p.lint();
    let excellent_section: String = v
        .lines()
        .skip_while(|l| !l.contains("excellent.rs"))
        .skip(1)
        .take_while(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        excellent_section.trim().is_empty(),
        "the clean file must produce zero findings, got:\n{excellent_section}\n---full---\n{v}"
    );
}
