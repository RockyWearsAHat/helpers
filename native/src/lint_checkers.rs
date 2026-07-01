//! `lint_checkers` — toolchain version detection for the AI linter.
//!
//! The linter learns each language's rules from its official docs, version-matched: it asks the
//! installed toolchain what version it is, so the crawl pulls (and the cache keys on) the rules for
//! exactly that version. That single capability is all this module provides now — the former
//! deterministic checker "floor" was retired in favour of the tree-pattern engine, which learns
//! every rule from the docs rather than from a hand-written primitive bank.

use std::collections::HashMap;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use regex::Regex;

/// The installed toolchain version for `lang` (`rustc --version`, `python3 --version`,
/// `node --version`, `go version`), or `None` when the toolchain isn't installed — used to pull and
/// cache the right version of the rules. Memoized per language so a whole-repo review spawns the
/// toolchain at most once per language.
pub fn detect_version(lang: &str) -> Option<String> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    // Hold the lock ACROSS the spawn so the toolchain is invoked exactly once per language. A
    // check-then-spawn pattern would let every concurrent first-caller spawn `rustc` at once — the
    // process/descriptor storm that made unrelated file reads fail under parallel load.
    let mut map = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(hit) = map.get(lang) {
        return hit.clone();
    }
    let v = detect_version_uncached(lang);
    map.insert(lang.to_string(), v.clone());
    v
}

/// Spawn the toolchain to read its version (see [`detect_version`], which memoizes this).
fn detect_version_uncached(lang: &str) -> Option<String> {
    let (cmd, args): (&str, &[&str]) = match lang {
        "rust" => ("rustc", &["--version"]),
        "python" => ("python3", &["--version"]),
        "javascript" | "typescript" => ("node", &["--version"]),
        "go" => ("go", &["version"]),
        _ => return None,
    };
    let out = Command::new(cmd).args(args).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(\d+\.\d+\.\d+)").unwrap());
    re.captures(&text).map(|c| c[1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_language_has_no_version() {
        assert!(detect_version("cobol").is_none());
    }
}
