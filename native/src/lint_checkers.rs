//! `lint_checkers` — PRECISE, data-driven rule checkers (no fuzzy matching, no
//! false positives). The AI reads a rule's official docs in English and emits a
//! compact **checker spec** — a generic primitive (regex / self-binop / tail-return
//! / banned-keyword) plus parameters — that decides exactly whether a line violates
//! the rule. Specs live per language (and version) in `lint-checkers/<lang>.json`,
//! are pulled from the repo when present, and generated + PR'd on a miss.
//!
//! This is the engine that reaches "any language, any version" without a hardcoded
//! rule list: the *content* is generated data (from the docs), the *interpreter*
//! here is a small fixed set of precise primitives. Adding a rule is adding a spec
//! line; adding a language is adding a spec file — never recompiling the binary.

use std::collections::HashMap;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use regex::Regex;
use serde::Deserialize;

use crate::git::workspace_root;

/// One generated checker: a rule id, severity, and a primitive `kind` + params.
#[derive(Debug, Clone, Deserialize)]
pub struct Spec {
    /// Official rule id this checker implements (e.g. `bool_comparison`).
    pub rule: String,
    /// Severity bucket from the docs (`high`/`medium`/`low`).
    pub severity: String,
    /// Primitive: `regex`, `self_binop`, `tail_return`, or `banned_keyword`.
    pub kind: String,
    /// `regex`: the line pattern to flag (exact, word-bounded).
    #[serde(default)]
    pub pattern: String,
    /// `self_binop`: operators flagged when both operands are the identical token.
    #[serde(default)]
    pub ops: Vec<String>,
    /// `banned_keyword`: the keyword whose presence is a violation.
    #[serde(default)]
    pub keyword: String,
}

/// A language's generated checker set, loaded from `lint-checkers/<lang>.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct CheckerSet {
    /// Language these checkers lint.
    pub language: String,
    /// Doc/toolchain version the checkers were generated from.
    #[serde(default)]
    pub version: String,
    /// The generated checkers.
    pub checkers: Vec<Spec>,
}

/// A precise violation.
pub struct CheckHit {
    /// 1-based source line.
    pub line: usize,
    /// The rule id violated.
    pub rule_id: String,
    /// Severity from the spec.
    pub severity: String,
}

/// Load the generated checker set for `lang`, or `None` when none exists yet (the
/// caller then asks the AI to generate it). Convenience wrapper for [`resolve`].
pub fn load(lang: &str) -> Option<CheckerSet> {
    resolve(lang, None)
}

/// Resolve the checker set for `lang` at `version`: prefer the version-pinned
/// `lint-checkers/<lang>@<version>.json` (so each toolchain version gets its own
/// docs-accurate checkers), falling back to the unpinned `<lang>.json`. This is the
/// "any version" path — a new version is a new spec file, generated from that
/// version's docs and PR'd.
pub fn resolve(lang: &str, version: Option<&str>) -> Option<CheckerSet> {
    let dir = workspace_root().join("lint-checkers");
    if let Some(v) = version {
        if let Ok(raw) = std::fs::read_to_string(dir.join(format!("{lang}@{v}.json"))) {
            if let Ok(cs) = serde_json::from_str::<CheckerSet>(&raw) {
                return Some(cs);
            }
        }
    }
    serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{lang}.json"))).ok()?).ok()
}

/// Read a file to a string, retrying briefly on a transient OS error (e.g. descriptor exhaustion
/// when many reads race). A linter must not drop a whole rule bank over one momentary hiccup; a
/// genuinely-absent file still resolves to `None` quickly (the first attempt's `NotFound`).
fn read_resilient(path: &std::path::Path) -> Option<String> {
    for attempt in 0..6 {
        match std::fs::read_to_string(path) {
            Ok(s) => return Some(s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
            Err(_) if attempt < 5 => std::thread::sleep(std::time::Duration::from_millis(3)),
            Err(_) => return None,
        }
    }
    None
}

/// Read and parse one bank file, or `None` if absent/invalid.
fn read_bank(path: std::path::PathBuf) -> Option<CheckerSet> {
    serde_json::from_str(&read_resilient(&path)?).ok()
}

/// Detect the project's runtime version for `lang` by asking its toolchain
/// (`rustc --version`, `python3 --version`, `node --version`, `go version`). Used
/// to pull the right version supplement. `None` when the toolchain isn't installed.
pub fn detect_version(lang: &str) -> Option<String> {
    // Memoize per language: this spawns the toolchain, and on a busy review run dozens of callers
    // would otherwise spawn `rustc --version` at once — the descriptor pressure that made bank
    // reads fail intermittently. Compute once (serialized on the lock) and reuse.
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

/// Assemble the effective checker bank for `lang`: the **hard base** (`<lang>.json`)
/// merged with the **version supplement** for the detected runtime
/// (`<lang>@<version>.json`), supplement rules taking precedence. This is the one
/// resolution an agent needs — it auto-detects the version and pulls/merges the
/// right banks; CS principles are always applied separately by the built-in scanner.
pub fn assemble(lang: &str) -> Option<CheckerSet> {
    // Memoize per language: assembling reads bank files and spawns the toolchain (`rustc
    // --version` …) to detect the version. Doing that on every review wastes work and, under
    // parallel load, the subprocess/FD pressure can make a file read transiently fail and drop the
    // whole bank. The bank is stable within a run, so compute it once and clone thereafter.
    static CACHE: OnceLock<Mutex<HashMap<String, Option<CheckerSet>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(map) = cache.lock() {
        if let Some(hit) = map.get(lang) {
            return hit.clone();
        }
    }
    let built = assemble_uncached(lang);
    // Only cache a real bank — never a `None`, which may be a transient read failure under load
    // rather than a genuinely-absent bank; caching it would make the failure stick for the run.
    if built.is_some() {
        if let Ok(mut map) = cache.lock() {
            map.insert(lang.to_string(), built.clone());
        }
    }
    built
}

/// The actual assembly (see [`assemble`], which memoizes this).
fn assemble_uncached(lang: &str) -> Option<CheckerSet> {
    let version = detect_version(lang);
    let dir = workspace_root().join("lint-checkers");
    let base = read_bank(dir.join(format!("{lang}.json")));
    let supp = version
        .as_deref()
        .and_then(|v| read_bank(dir.join(format!("{lang}@{v}.json"))));
    let mut set = match (base, supp.clone()) {
        (Some(b), _) => b,
        (None, Some(s)) => s,
        (None, None) => return None,
    };
    if let Some(s) = supp {
        let have: std::collections::HashSet<&str> =
            set.checkers.iter().map(|c| c.rule.as_str()).collect::<std::collections::HashSet<_>>();
        let extra: Vec<Spec> = s.checkers.into_iter().filter(|c| !have.contains(c.rule.as_str())).collect();
        set.checkers.extend(extra);
        if !s.version.is_empty() {
            set.version = s.version;
        }
    } else if let Some(v) = version {
        set.version = v;
    }
    Some(set)
}

/// Reduce a line to the code a checker may match: strip a `//` comment AND blank
/// out string-literal contents, so a checker never fires on code that only appears
/// inside a comment or a string (a doc example or test snippet stored as a string).
/// Essential for zero false positives. A `#` outside a string ends a line comment.
fn code_of(line: &str) -> String {
    let no_slashes = line.split("//").next().unwrap_or("");
    let mut out = String::with_capacity(no_slashes.len());
    let mut in_str = false;
    let mut wrote = false; // wrote the placeholder for the current string yet?
    let mut chars = no_slashes.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push('"');
                in_str = !in_str;
                if in_str {
                    wrote = false;
                }
            }
            '\\' if in_str => {
                chars.next(); // skip the escaped char
                if !wrote {
                    out.push('s');
                    wrote = true;
                }
            }
            '#' if !in_str => break, // line comment (python/shell)
            // Collapse non-empty string content to a single placeholder `s` so an
            // empty `""` stays empty (for `comparison_to_empty`) while no checker
            // matches code-looking text inside a string.
            _ if in_str => {
                if !wrote {
                    out.push('s');
                    wrote = true;
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// `\b<kw>\b` matcher cache for the banned-keyword primitive.
fn word_re(kw: &str) -> Option<Regex> {
    Regex::new(&format!(r"\b{}\b", regex::escape(kw))).ok()
}

impl CheckerSet {
    /// Run every checker in this set over the file's lines, returning precise hits.
    pub fn run(&self, lines: &[&str]) -> Vec<CheckHit> {
        let mut hits = Vec::new();
        for c in &self.checkers {
            match c.kind.as_str() {
                "regex" => {
                    if let Ok(re) = Regex::new(&c.pattern) {
                        for (i, l) in lines.iter().enumerate() {
                            if re.is_match(&code_of(l)) {
                                hits.push(self.mk(i + 1, c));
                            }
                        }
                    }
                }
                "self_binop" => self.self_binop(&c.ops, lines, c, &mut hits),
                "tail_return" => self.tail_return(lines, c, &mut hits),
                "banned_keyword" => {
                    if let Some(re) = word_re(&c.keyword) {
                        for (i, l) in lines.iter().enumerate() {
                            if re.is_match(&code_of(l)) {
                                hits.push(self.mk(i + 1, c));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        hits
    }

    fn mk(&self, line: usize, c: &Spec) -> CheckHit {
        CheckHit { line, rule_id: c.rule.clone(), severity: c.severity.clone() }
    }

    /// A binary op whose two operands are the identical identifier (`a == a`).
    fn self_binop(&self, ops: &[String], lines: &[&str], c: &Spec, hits: &mut Vec<CheckHit>) {
        // Operands capture the FULL expression incl. `.` (so `t.name == name` reads
        // as `t.name` vs `name` — not equal — instead of a spurious `name == name`).
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| {
            Regex::new(r"([a-zA-Z_][\w.]*)\s*(==|!=|&&|\|\||<=|>=)\s*([a-zA-Z_][\w.]*)").unwrap()
        });
        for (i, l) in lines.iter().enumerate() {
            let cl = code_of(l);
            for cap in re.captures_iter(&cl) {
                let (a, op, b) = (&cap[1], &cap[2], &cap[3]);
                if a == b && a != "true" && a != "false" && ops.iter().any(|o| o == op) {
                    hits.push(self.mk(i + 1, c));
                    break;
                }
            }
        }
    }

    /// A `return` that is the LAST statement of a function body (position-aware, so
    /// an early return is never flagged).
    fn tail_return(&self, lines: &[&str], c: &Spec, hits: &mut Vec<CheckHit>) {
        let mut i = 0;
        while i < lines.len() {
            let t = lines[i].trim_start();
            if (t.starts_with("fn ") || t.contains(" fn ") || t.starts_with("def ")) && (lines[i].contains('{') || t.starts_with("def ")) {
                let braces = lines[i].contains('{');
                let (mut depth, mut end) = (0i32, lines.len() - 1);
                if braces {
                    for (k, l) in lines.iter().enumerate().skip(i) {
                        depth += code_of(l).matches('{').count() as i32 - code_of(l).matches('}').count() as i32;
                        if depth <= 0 {
                            end = k;
                            break;
                        }
                    }
                }
                for k in (i + 1..=end).rev() {
                    let cl = code_of(lines[k]);
                    let s = cl.trim().trim_end_matches(';').trim();
                    if s.is_empty() || s == "}" {
                        continue;
                    }
                    if s == "return" || s.starts_with("return ") {
                        hits.push(self.mk(k + 1, c));
                    }
                    break;
                }
                i = end + 1;
            } else {
                i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(json: &str) -> CheckerSet {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn regex_primitive_is_exact() {
        let s = set(r#"{"language":"rust","checkers":[{"rule":"bool_comparison","severity":"medium","kind":"regex","pattern":"(==|!=)\\s*(true|false)\\b"}]}"#);
        assert_eq!(s.run(&["if flag == true {"]).len(), 1);
        assert_eq!(s.run(&["if flag {"]).len(), 0);
        assert_eq!(s.run(&["let truest = 1;"]).len(), 0);
    }

    #[test]
    fn self_binop_needs_identical_operands() {
        let s = set(r#"{"language":"rust","checkers":[{"rule":"eq_op","severity":"high","kind":"self_binop","ops":["==","!="]}]}"#);
        assert_eq!(s.run(&["if a == a {"]).len(), 1);
        assert_eq!(s.run(&["if a == b {"]).len(), 0);
    }

    #[test]
    fn tail_return_is_position_aware() {
        let s = set(r#"{"language":"rust","checkers":[{"rule":"needless_return","severity":"low","kind":"tail_return"}]}"#);
        assert_eq!(s.run(&["fn f() -> i32 {", "    let x = c();", "    return x;", "}"]).len(), 1);
        assert_eq!(s.run(&["fn g(c: bool) {", "    if c { return; }", "    work();", "}"]).len(), 0);
    }
}
