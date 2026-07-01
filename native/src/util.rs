//! Small shared helpers ported from `lib/mcp-utils.js`.

use chrono::{SecondsFormat, Utc};
use serde_json::Value;

/// Port of `tokenizeQuery`: lowercase, split on chars outside `[a-z0-9_.-]`,
/// keep tokens of length >= 2.
pub fn tokenize_query(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split(|c: char| {
            !(c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-'))
        })
        .map(|t| t.trim())
        .filter(|t| t.len() >= 2)
        .map(str::to_string)
        .collect()
}

/// Port of `getMarkdownTitle`: the first `# heading` text, or `fallback`.
pub fn get_markdown_title(text: &str, fallback: &str) -> String {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# ") {
            let t = rest.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
    }
    fallback.to_string()
}

/// Port of `summarizeText`: truncate to `max_chars`, preferring a clean break
/// (paragraph or sentence) past 60% of the limit, then append an ellipsis.
pub fn summarize_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let clipped: String = text.chars().take(max_chars).collect();
    let para = clipped.rfind("\n\n").map(|i| i as i64).unwrap_or(-1);
    let sentence = clipped.rfind(". ").map(|i| i as i64).unwrap_or(-1);
    let last_break = para.max(sentence);
    if last_break > (max_chars as f64 * 0.6) as i64 {
        let cut = &clipped[..last_break as usize];
        format!("{}...", cut.trim())
    } else {
        format!("{}...", clipped.trim())
    }
}

/// Current UTC time formatted exactly like JavaScript's `Date.toISOString()`
/// ("YYYY-MM-DDTHH:MM:SS.sssZ").
pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Current time in milliseconds since the Unix epoch (like `Date.now()`).
pub fn now_millis() -> i64 {
    Utc::now().timestamp_millis()
}

/// Parse an ISO-8601 timestamp to epoch millis (like `new Date(s).getTime()`).
pub fn parse_iso_millis(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

/// Port of `toPositiveInt`: `parseInt` the value (number or string), fall back
/// to `fallback` when not finite, then clamp to `[min, max]`.
pub fn to_positive_int(value: &Value, fallback: i64, min: i64, max: i64) -> i64 {
    let s = match value {
        Value::Null => return fallback,
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => return fallback,
    };
    match js_parse_int(&s) {
        Some(n) => n.clamp(min, max),
        None => fallback,
    }
}

/// JavaScript `parseInt(s, 10)` semantics: skip leading whitespace, optional
/// sign, then take consecutive ASCII digits. Returns `None` for no digits (NaN).
fn js_parse_int(s: &str) -> Option<i64> {
    let bytes = s.trim_start().as_bytes();
    let mut i = 0;
    let mut sign: i64 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let start = i;
    let mut acc: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        acc = acc
            .saturating_mul(10)
            .saturating_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        None
    } else {
        Some(sign * acc)
    }
}

/// `Math.round(x * 10^p) / 10^p` with JS half-up rounding (values here are
/// non-negative, so `f64::round` matches).
pub fn round_to(x: f64, places: i32) -> f64 {
    let factor = 10f64.powi(places);
    (x * factor).round() / factor
}

/// Map a file extension to a canonical language name. The single source of truth — every
/// other module imports this rather than maintaining its own copy.
/// Returns `None` for unrecognized extensions; callers decide whether to skip or surface
/// that as an error.
pub fn file_lang(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "rs"                          => "rust",
        "py"                          => "python",
        "js" | "mjs" | "cjs" | "jsx" => "javascript",
        "ts" | "tsx"                  => "typescript",
        "go"                          => "go",
        "java"                        => "java",
        "rb"                          => "ruby",
        "c"  | "h"                    => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "sh" | "bash"                 => "bash",
        "kt" | "kts"                  => "kotlin",
        "swift"                       => "swift",
        "cs"                          => "csharp",
        "php"                         => "php",
        _                             => return None,
    })
}
