//! `lint_toolchain` — ground the reader's understanding against the INSTALLED toolchain.
//!
//! The docs make claims ("this is deprecated", "this is the correct form"); the only honest way to
//! know which code is bad is to test it against reality. For every code example a crawl reads, this
//! module runs the language's toolchain in **check mode only** — parse / compile / deprecation
//! checks, never execution — and returns a [`Verdict`]. A flagged example's surrounding prose feeds
//! the BAD polarity prototype; a clean example's feeds GOOD. That is the whole grounding loop.
//!
//! The check commands are DATA ([`lint-index/toolchains.json`]), so adding a toolchain is a data
//! edit, not a code change. A language with no template, or whose tool is not installed, yields
//! [`Verdict::Unknown`] and simply does not contribute grounding.

use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use serde::Deserialize;

/// The committed check templates, embedded so an installed binary far from the checkout can still
/// ground against a local toolchain. The on-disk copy is preferred so editing it takes effect.
const EMBEDDED_TOOLCHAINS: &str = include_str!("../../lint-index/toolchains.json");

/// What the toolchain said about one code example.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    /// The toolchain accepted the snippet (parsed/compiled clean, no deprecation) — endorsement.
    Clean,
    /// The toolchain rejected the snippet (syntax/compile error or deprecation) — prohibition.
    Flagged,
    /// No template for this language, or its tool is not installed — no grounding signal.
    Unknown,
}

/// One language's check-mode probe template, loaded from the toolchains data file.
#[derive(Clone, Deserialize)]
struct Toolchain {
    language: String,
    ext: String,
    /// Argv with `{file}` (and optional `{devnull}`) placeholders; the first element is the binary.
    check: Vec<String>,
    /// Line-lead prompts (e.g. a REPL `>>> `) stripped before checking, so a doctest is judged as
    /// the code it illustrates rather than failing as a bare prompt.
    #[serde(default)]
    strip_prompts: Vec<String>,
    /// Substrings in the tool's stderr that mark a prohibition even on a success exit (rustc prints
    /// deprecation warnings and still exits 0). Knowledge about the TOOL, so it lives in the data
    /// file — expanding it is a data edit, never a code change. Empty → exit status alone decides.
    #[serde(default)]
    flag_markers: Vec<String>,
    /// Fragment harness: `{code}` templates retried IN ORDER when the snippet fails as
    /// written (LINTER.md, open problems — fragment examples). Documentation quotes
    /// FRAGMENTS (`#[expect(…)]` on a bare line; go statements without a `package`
    /// clause), and different fragment shapes need different scaffolds (go: statement
    /// wrap vs top-level-declaration wrap). A snippet that passes under ANY wrap is
    /// valid code shown without scaffolding — CLEAN exposure. Tool knowledge, so it is
    /// DATA. Empty → no retry.
    #[serde(default)]
    wraps: Vec<String>,
    /// Substrings in a FAILING run's stderr that mean "missing surrounding context", not
    /// "demonstrated violation" (rustc's `cannot find`/`unresolved import` on a snippet
    /// that references names its page defined elsewhere). Such failures judge the
    /// SCAFFOLDING, so the verdict is Unknown. Tool knowledge — data, like `flag_markers`.
    #[serde(default)]
    context_markers: Vec<String>,
}

/// The verdict for one check run: a non-success exit is always a prohibition; on success, the
/// toolchain's own configured `flag_markers` decide whether the output still flags the snippet.
fn verdict_from_output(success: bool, stderr: &str, flag_markers: &[String]) -> Verdict {
    let s = stderr.to_lowercase();
    if !success || flag_markers.iter().any(|m| s.contains(m.as_str())) {
        Verdict::Flagged
    } else {
        Verdict::Clean
    }
}

/// The parsed toolchains file (on-disk under `data_root` preferred, embedded fallback).
fn toolchains(data_root: &Path) -> &'static Vec<Toolchain> {
    static CACHE: OnceLock<Vec<Toolchain>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let raw = std::fs::read_to_string(data_root.join("lint-index/toolchains.json"))
            .unwrap_or_else(|_| EMBEDDED_TOOLCHAINS.to_string());
        serde_json::from_str::<serde_json::Value>(&raw)
            .ok()
            .and_then(|v| serde_json::from_value(v["toolchains"].clone()).ok())
            .unwrap_or_default()
    })
}

/// Whether a binary is runnable, memoized so a whole crawl probes each tool's presence once.
fn tool_present(bin: &str) -> bool {
    static CACHE: OnceLock<Mutex<std::collections::HashMap<String, bool>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    let mut map = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(hit) = map.get(bin) {
        return *hit;
    }
    // A bare spawn with no args is enough to learn whether the binary exists on PATH; we do not
    // care about its exit status here, only that it could be launched.
    let present = Command::new(bin)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok();
    map.insert(bin.to_string(), present);
    present
}

/// Remove leading REPL/shell prompts from a snippet so a doctest is checked as its underlying code.
fn strip_prompts(code: &str, prompts: &[String]) -> String {
    code.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            for p in prompts {
                if let Some(rest) = trimmed.strip_prefix(p.as_str()) {
                    return rest.to_string();
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Check one code example against `lang`'s installed toolchain in check mode only.
///
/// Returns [`Verdict::Unknown`] when there is no template for the language or its tool is missing —
/// grounding is then simply skipped. Otherwise writes the (prompt-stripped) snippet to a temp file
/// and runs the template: a non-success exit, or a `deprecat`/`warning` mention in its output, is a
/// [`Verdict::Flagged`] prohibition; a clean run is [`Verdict::Clean`] endorsement. Never executes
/// the snippet — the templates are parse/compile/format checks.
pub fn check(lang: &str, code: &str, data_root: &Path) -> Verdict {
    let Some(tc) = toolchains(data_root).iter().find(|t| t.language == lang) else {
        return Verdict::Unknown;
    };
    let Some(bin) = tc.check.first() else { return Verdict::Unknown };
    if !tool_present(bin) {
        return Verdict::Unknown;
    }
    let body = strip_prompts(code, &tc.strip_prompts);
    if body.trim().is_empty() {
        return Verdict::Unknown;
    }
    // Isolated temp file per check so concurrent probes never collide — the sequence number
    // keeps two parallel checks of the SAME snippet apart (checks now run on a thread pool).
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "helpers-check-{}-{:x}-{}",
        std::process::id(),
        crate::lint_ai::token_seed(code),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    if std::fs::create_dir_all(&dir).is_err() {
        return Verdict::Unknown;
    }
    let file = dir.join(format!("snippet.{}", tc.ext));
    if std::fs::write(&file, &body).is_err() {
        let _ = std::fs::remove_dir_all(&dir);
        return Verdict::Unknown;
    }
    let args: Vec<String> = tc.check[1..]
        .iter()
        .map(|a| a.replace("{file}", &file.to_string_lossy()).replace("{devnull}", "/dev/null"))
        .collect();
    let output = Command::new(bin).args(&args).current_dir(&dir).output();
    let mut verdict = match output {
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
            if !out.status.success()
                && tc.context_markers.iter().any(|m| stderr.contains(m.as_str()))
            {
                // The failure names missing CONTEXT, not a demonstrated violation: the
                // snippet references what its page defined elsewhere. Reality was asked
                // the wrong question — no label either way.
                Verdict::Unknown
            } else {
                verdict_from_output(out.status.success(), &stderr, &tc.flag_markers)
            }
        }
        Err(_) => Verdict::Unknown,
    };
    // A failing FRAGMENT retried under the toolchain's wrap harnesses: a snippet that
    // PASSES under any wrap was valid code shown without scaffolding — that is reality's
    // "parses", i.e. CLEAN exposure (never Flagged, and not Unknown either: zero clean
    // verdicts would leave the language's tallies bad-saturated). A wrapped failure that
    // names missing context downgrades to Unknown; anything else keeps Flagged.
    if verdict == Verdict::Flagged {
        for wrap in &tc.wraps {
            let wrapped = wrap.replace("{code}", &body);
            if std::fs::write(&file, &wrapped).is_err() {
                break;
            }
            let Ok(out) = Command::new(bin).args(&args).current_dir(&dir).output() else {
                break;
            };
            let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
            if out.status.success() && !tc.flag_markers.iter().any(|m| stderr.contains(m.as_str())) {
                verdict = Verdict::Clean;
                break;
            }
            if tc.context_markers.iter().any(|m| stderr.contains(m.as_str())) {
                verdict = Verdict::Unknown;
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    verdict
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    #[test]
    fn verdict_reads_exit_status_and_the_toolchains_own_flag_markers() {
        let markers = vec!["deprecat".to_string(), "warning".to_string()];
        // Non-success exit is always a prohibition, whatever the output says.
        assert_eq!(verdict_from_output(false, "", &markers), Verdict::Flagged);
        // Success + a marker the DATA file lists for this toolchain → prohibition (rustc prints
        // deprecation warnings on a success exit).
        assert_eq!(verdict_from_output(true, "warning: use of deprecated function", &markers), Verdict::Flagged);
        // Success + no listed marker → endorsement.
        assert_eq!(verdict_from_output(true, "note: compiled fine", &markers), Verdict::Clean);
        // No markers configured → exit status alone decides.
        assert_eq!(verdict_from_output(true, "warning: whatever", &[]), Verdict::Clean);
    }

    #[test]
    fn rust_template_carries_its_flag_markers_as_data() {
        // The markers are knowledge about the TOOL, so they live in toolchains.json — adding or
        // fixing one is a data edit, never a code change.
        let tc = toolchains(&data_root());
        let rust = tc.iter().find(|t| t.language == "rust").expect("rust template");
        assert!(rust.flag_markers.iter().any(|m| m.contains("deprecat")), "rust flags deprecation output: {:?}", rust.flag_markers);
    }

    #[test]
    fn strips_repl_prompts() {
        let out = strip_prompts(">>> x = 1\n... y = 2\nz = 3", &[">>> ".into(), "... ".into()]);
        assert_eq!(out, "x = 1\ny = 2\nz = 3");
    }

    #[test]
    fn embedded_templates_parse() {
        // The data file must always be loadable; python/javascript must be covered.
        let tcs = toolchains(&data_root());
        assert!(tcs.iter().any(|t| t.language == "python"));
        assert!(tcs.iter().any(|t| t.language == "javascript"));
    }

    #[test]
    fn unknown_language_yields_unknown() {
        assert_eq!(check("no-such-language", "print(1)", &data_root()), Verdict::Unknown);
    }

    #[test]
    fn python_grounding_separates_clean_from_flagged() {
        // Only asserts when python3 is installed; otherwise the check abstains (Unknown) and the
        // test degrades gracefully — determinism does not depend on the toolchain being present.
        if !tool_present("python3") {
            return;
        }
        assert_eq!(check("python", "x = 1\ny = x + 1\n", &data_root()), Verdict::Clean);
        // `nonlocal` at module scope is a real SyntaxError — the toolchain flags it.
        assert_eq!(check("python", "nonlocal q\n", &data_root()), Verdict::Flagged);
    }
}
