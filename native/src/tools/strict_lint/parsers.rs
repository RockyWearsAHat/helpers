//! Pure linter-output parsers — each converts one tool's raw stdout/stderr into
//! the shared [`Diag`] model. These are deterministic and unit-tested without the
//! linters installed.

use std::path::Path;

use serde_json::Value;

use super::{Diag, Sev};

/// Build a `Diag`, trimming the message. Shared by every parser below.
fn diag(
    file: String,
    line: u32,
    col: u32,
    severity: Sev,
    rule: &str,
    message: &str,
    tool: &'static str,
) -> Diag {
    Diag {
        file,
        line,
        col,
        severity,
        rule: rule.to_string(),
        message: message.trim().to_string(),
        tool,
    }
}

/// Resolve a possibly-relative path reported by a linter against `base`.
fn resolve(base: &Path, rel: &str) -> String {
    let p = Path::new(rel);
    if p.is_absolute() {
        rel.to_string()
    } else {
        base.join(rel).to_string_lossy().to_string()
    }
}

/// Parse ESLint `--format json` output into unified diagnostics.
pub fn parse_eslint(stdout: &str) -> Vec<Diag> {
    let mut diags = Vec::new();
    let files: Value = serde_json::from_str(stdout.trim()).unwrap_or(Value::Null);
    if let Some(arr) = files.as_array() {
        for f in arr {
            let path = f.get("filePath").and_then(Value::as_str).unwrap_or("");
            for m in f
                .get("messages")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let sev = if m.get("severity").and_then(Value::as_i64) == Some(2) {
                    Sev::Error
                } else {
                    Sev::Warning
                };
                diags.push(diag(
                    path.to_string(),
                    m.get("line").and_then(Value::as_u64).unwrap_or(0) as u32,
                    m.get("column").and_then(Value::as_u64).unwrap_or(0) as u32,
                    sev,
                    m.get("ruleId").and_then(Value::as_str).unwrap_or(""),
                    m.get("message").and_then(Value::as_str).unwrap_or(""),
                    "eslint",
                ));
            }
        }
    }
    diags
}

/// Parse `cargo clippy --message-format=json` lines (one JSON value per line)
/// into unified diagnostics, resolving span paths relative to `base`.
pub fn parse_clippy(stdout: &str, base: &Path) -> Vec<Diag> {
    let mut diags = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let m: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if m.get("reason").and_then(Value::as_str) != Some("compiler-message") {
            continue;
        }
        let msg = match m.get("message") {
            Some(v) => v,
            None => continue,
        };
        let level = msg.get("level").and_then(Value::as_str).unwrap_or("");
        let sev = match level {
            "error" => Sev::Error,
            "warning" => Sev::Warning,
            _ => continue,
        };
        let spans = msg
            .get("spans")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let span = spans
            .iter()
            .find(|s| s.get("is_primary").and_then(Value::as_bool) == Some(true))
            .or_else(|| spans.first());
        let span = match span {
            Some(s) => s,
            None => continue,
        };
        let code = msg
            .get("code")
            .and_then(|c| c.get("code"))
            .and_then(Value::as_str)
            .unwrap_or("");
        diags.push(diag(
            resolve(
                base,
                span.get("file_name").and_then(Value::as_str).unwrap_or(""),
            ),
            span.get("line_start").and_then(Value::as_u64).unwrap_or(0) as u32,
            span.get("column_start")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            sev,
            code,
            msg.get("message").and_then(Value::as_str).unwrap_or(""),
            "clippy",
        ));
    }
    diags
}

/// Parse ShellCheck `-f json` output into unified diagnostics.
pub fn parse_shellcheck(stdout: &str) -> Vec<Diag> {
    let mut diags = Vec::new();
    let arr: Value = serde_json::from_str(stdout.trim()).unwrap_or(Value::Null);
    for v in arr.as_array().into_iter().flatten() {
        let sev = match v.get("level").and_then(Value::as_str).unwrap_or("") {
            "error" => Sev::Error,
            "warning" => Sev::Warning,
            _ => Sev::Hint,
        };
        let code = format!("SC{}", v.get("code").and_then(Value::as_u64).unwrap_or(0));
        diags.push(diag(
            v.get("file")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            v.get("line").and_then(Value::as_u64).unwrap_or(0) as u32,
            v.get("column").and_then(Value::as_u64).unwrap_or(0) as u32,
            sev,
            &code,
            v.get("message").and_then(Value::as_str).unwrap_or(""),
            "shellcheck",
        ));
    }
    diags
}

/// Parse Ruff `--output-format json` output into unified diagnostics.
pub fn parse_ruff(stdout: &str) -> Vec<Diag> {
    let mut diags = Vec::new();
    let arr: Value = serde_json::from_str(stdout.trim()).unwrap_or(Value::Null);
    for v in arr.as_array().into_iter().flatten() {
        diags.push(diag(
            v.get("filename")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            v.get("location")
                .and_then(|l| l.get("row"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            v.get("location")
                .and_then(|l| l.get("column"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            Sev::Warning,
            v.get("code").and_then(Value::as_str).unwrap_or(""),
            v.get("message").and_then(Value::as_str).unwrap_or(""),
            "ruff",
        ));
    }
    diags
}

/// Parse `tsc --noEmit --pretty false` text output into unified diagnostics,
/// resolving file paths relative to `base`.
pub fn parse_tsc(stdout: &str, base: &Path) -> Vec<Diag> {
    let re =
        regex::Regex::new(r"^(.+?)\((\d+),(\d+)\):\s+(error|warning)\s+(TS\d+):\s+(.*)$").unwrap();
    let mut diags = Vec::new();
    for line in stdout.lines() {
        if let Some(c) = re.captures(line) {
            let sev = if &c[4] == "error" {
                Sev::Error
            } else {
                Sev::Warning
            };
            diags.push(diag(
                resolve(base, &c[1]),
                c[2].parse().unwrap_or(0),
                c[3].parse().unwrap_or(0),
                sev,
                &c[5],
                &c[6],
                "tsc",
            ));
        }
    }
    diags
}

/// Parse mypy `--show-error-codes` text output into unified diagnostics,
/// resolving file paths relative to `base`.
pub fn parse_mypy(stdout: &str, base: &Path) -> Vec<Diag> {
    let re = regex::Regex::new(
        r"^(.+?):(\d+):(?:(\d+):)?\s+(error|note|warning):\s+(.*?)(?:\s+\[([\w-]+)\])?$",
    )
    .unwrap();
    let mut diags = Vec::new();
    for line in stdout.lines() {
        if let Some(c) = re.captures(line) {
            let sev = match &c[4] {
                "error" => Sev::Error,
                "warning" => Sev::Warning,
                _ => Sev::Hint,
            };
            diags.push(diag(
                resolve(base, &c[1]),
                c[2].parse().unwrap_or(0),
                c.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
                sev,
                c.get(6).map(|m| m.as_str()).unwrap_or(""),
                c.get(5).map(|m| m.as_str()).unwrap_or(""),
                "mypy",
            ));
        }
    }
    diags
}

/// Parse `go vet` stderr text into unified diagnostics, resolving file paths
/// relative to `base`.
pub fn parse_go_vet(stderr: &str, base: &Path) -> Vec<Diag> {
    let re = regex::Regex::new(r"^(.+?\.go):(\d+):(?:(\d+):)?\s+(.*)$").unwrap();
    let mut diags = Vec::new();
    for line in stderr.lines() {
        if let Some(c) = re.captures(line.trim()) {
            diags.push(diag(
                resolve(base, &c[1]),
                c[2].parse().unwrap_or(0),
                c.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
                Sev::Warning,
                "vet",
                &c[4],
                "go vet",
            ));
        }
    }
    diags
}

/// Parse `staticcheck -f json` output (one JSON value per line) into diagnostics.
pub fn parse_staticcheck(stdout: &str) -> Vec<Diag> {
    let mut diags = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let m: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let loc = match m.get("location") {
            Some(l) => l,
            None => continue,
        };
        let sev = if m.get("severity").and_then(Value::as_str) == Some("error") {
            Sev::Error
        } else {
            Sev::Warning
        };
        diags.push(diag(
            loc.get("file")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            loc.get("line").and_then(Value::as_u64).unwrap_or(0) as u32,
            loc.get("column").and_then(Value::as_u64).unwrap_or(0) as u32,
            sev,
            m.get("code").and_then(Value::as_str).unwrap_or(""),
            m.get("message").and_then(Value::as_str).unwrap_or(""),
            "staticcheck",
        ));
    }
    diags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_eslint_json() {
        let json = r#"[{"filePath":"/p/a.js","messages":[{"severity":2,"line":3,"column":5,"ruleId":"no-unused-vars","message":"x is unused"},{"severity":1,"line":7,"column":1,"ruleId":"eqeqeq","message":"use ==="}]}]"#;
        let d = parse_eslint(json);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].severity, Sev::Error);
        assert_eq!(d[0].rule, "no-unused-vars");
        assert_eq!(d[1].severity, Sev::Warning);
    }

    #[test]
    fn parses_clippy_json() {
        let line = r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused variable","code":{"code":"unused_variables"},"spans":[{"is_primary":true,"file_name":"src/x.rs","line_start":4,"column_start":9}]}}"#;
        let d = parse_clippy(line, Path::new("/proj"));
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].severity, Sev::Warning);
        assert_eq!(d[0].rule, "unused_variables");
        assert_eq!(d[0].file, "/proj/src/x.rs");
        assert_eq!(d[0].line, 4);
    }

    #[test]
    fn parses_shellcheck_and_tsc() {
        let sc = r#"[{"file":"a.sh","line":2,"column":1,"level":"warning","code":2086,"message":"Double quote"}]"#;
        let d = parse_shellcheck(sc);
        assert_eq!(d[0].rule, "SC2086");
        let tsc = "src/x.ts(10,5): error TS2304: Cannot find name 'foo'.";
        let d2 = parse_tsc(tsc, Path::new("/proj"));
        assert_eq!(d2.len(), 1);
        assert_eq!(d2[0].severity, Sev::Error);
        assert_eq!(d2[0].rule, "TS2304");
        assert_eq!(d2[0].file, "/proj/src/x.ts");
    }
}
