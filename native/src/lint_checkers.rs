//! `lint_checkers` — toolchain version detection for the AI linter.
//!
//! The linter learns each language's rules from its official docs, version-matched: it asks the
//! installed toolchain what version it is, so the crawl pulls (and the cache keys on) the rules for
//! exactly that version. That single capability is all this module provides now — the former
//! deterministic checker "floor" was retired in favour of the tree-pattern engine, which learns
//! every rule from the docs rather than from a hand-written primitive bank.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use regex::Regex;

/// The toolchain binary that answers for `lang`, and the probe argument sets to try. The alias
/// map only covers toolchains whose binary is not named after the language (rustc, python3,
/// node); everything else is probed as `<lang> --version` / `<lang> version`.
fn toolchain_binary(lang: &str) -> Option<(&str, &[&[&str]])> {
    match lang {
        "rust" => Some(("rustc", &[&["--version"]])),
        "python" => Some(("python3", &[&["--version"]])),
        "javascript" | "typescript" => Some(("node", &[&["--version"]])),
        "go" => Some(("go", &[&["version"]])),
        _ => None,
    }
}

/// The installed toolchain version for `lang` (`rustc --version`, `python3 --version`,
/// `node --version`, `go version`), or `None` when the toolchain isn't installed — used to pull
/// and cache the right version of the rules. Answers come from a MACHINE cache
/// (`<models>/toolchains.json`) keyed by the resolved binary's `(path, mtime, len)` — absence
/// keyed by a PATH fingerprint — so a warm run spawns NOTHING: seventeen parallel version
/// spawns were the single largest stage of every warm lint (native/architecture.dx, "Warm runs replay
/// per-file verdicts"). Installing, upgrading, or removing a toolchain changes the key and
/// re-probes exactly the affected language.
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
    let v = detect_version_machine_cached(lang);
    map.insert(lang.to_string(), v.clone());
    v
}

/// Where detected toolchain versions persist — a small data file beside the models, like the
/// extension map (model artifacts are `HLM1`; this is metadata).
fn version_cache_path() -> PathBuf {
    crate::lint_train::model_dir_pub().join("toolchains.json")
}

/// One remembered probe: the state key it was made under, and the answer.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct VersionEntry {
    key: String,
    version: Option<String>,
}

/// `lang`'s version through the machine cache: resolve the binary in PATH (a few stats), and
/// only when its state key differs from the remembered probe does a process actually spawn.
fn detect_version_machine_cached(lang: &str) -> Option<String> {
    let (cmd, arg_sets): (String, Vec<Vec<String>>) = match toolchain_binary(lang) {
        Some((cmd, sets)) => (
            cmd.to_string(),
            sets.iter().map(|args| args.iter().map(|a| a.to_string()).collect()).collect(),
        ),
        None => {
            if lang.is_empty() || !lang.chars().all(|c| c.is_ascii_alphanumeric()) {
                return None; // a language name is a bare word; anything else is not a binary
            }
            (lang.to_string(), vec![vec!["--version".to_string()], vec!["version".to_string()]])
        }
    };
    let resolved = resolve_in_path(&cmd);
    let key = match &resolved {
        Some((path, state)) => format!("{}@{state:x}", path.display()),
        None => format!("absent@{:x}", path_env_fingerprint()),
    };
    static DISK: Mutex<Option<HashMap<String, VersionEntry>>> = Mutex::new(None);
    let mut disk = DISK.lock().unwrap_or_else(|e| e.into_inner());
    let cache = disk.get_or_insert_with(|| {
        std::fs::read_to_string(version_cache_path())
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    });
    if let Some(entry) = cache.get(lang) {
        if entry.key == key {
            return entry.version.clone();
        }
    }
    // The binary is genuinely new to this machine (or changed): probe it once and remember.
    let version = match resolved {
        Some((path, _)) => probe_version(&path, &arg_sets),
        None => None,
    };
    cache.insert(lang.to_string(), VersionEntry { key, version: version.clone() });
    if let Ok(json) = serde_json::to_string(&*cache) {
        if let Some(dir) = version_cache_path().parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(version_cache_path(), json);
    }
    version
}

/// Find `cmd` in PATH and fold its `(mtime, len)` into a state key — the cheap witness that
/// the toolchain changed. A relative or bare name searches PATH exactly like the shell would.
fn resolve_in_path(cmd: &str) -> Option<(PathBuf, u128)> {
    let state = |p: &PathBuf| -> Option<u128> {
        let m = std::fs::metadata(p).ok()?;
        if !m.is_file() {
            return None;
        }
        let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
        Some(t.map(|d| d.as_nanos()).unwrap_or(0) ^ ((m.len() as u128) << 64))
    };
    for dir in std::env::split_paths(&std::env::var_os("PATH")?) {
        let candidate = dir.join(cmd);
        if let Some(st) = state(&candidate) {
            return Some((candidate, st));
        }
    }
    None
}

/// Fingerprint of the PATH environment: the value itself plus each directory's mtime, so a
/// cached "not installed" answer expires the moment anything lands in a PATH directory.
fn path_env_fingerprint() -> u64 {
    let path = std::env::var("PATH").unwrap_or_default();
    let mut h = crate::lint_ai::token_seed(&path);
    for dir in std::env::split_paths(&path) {
        let mtime = std::fs::metadata(&dir)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        h = h.rotate_left(7) ^ mtime;
    }
    h
}

/// Spawn the resolved binary with each probe argument set until one yields a version.
fn probe_version(path: &PathBuf, arg_sets: &[Vec<String>]) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(\d+\.\d+(?:\.\d+)?)").unwrap());
    for args in arg_sets {
        if let Ok(out) = Command::new(path).args(args).output() {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(c) = re.captures(&text) {
                return Some(c[1].to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_language_has_no_version() {
        assert!(detect_version("cobol").is_none());
    }
}
