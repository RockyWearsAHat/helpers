//! `lint_index` — the packed, version-matched, official-doc-sourced rule catalog.
//!
//! Helpers ships a committed rule catalog per tool under `lint-index/<tool>.json`
//! (see `lint-index/SCHEMA.md`). This module is the **fast-path resolver**: given a
//! tool, its language, and a detected toolchain version, it reads the packed index,
//! verifies its checksum, and decides whether the snapshot covers that toolchain.
//!
//! The tiered resolution contract (fast-path checksum/version match → poll/pull →
//! crawl-on-miss) lives in `SCHEMA.md`. This module implements **only the decision**:
//! [`resolve`] returns [`Resolution::Packed`] on a verified fast-path hit, or
//! [`Resolution::NeedsCrawl`] with the trigger reason otherwise. It never fetches,
//! polls, or crawls — callers own those slower tiers.
//!
//! Determinism: the checksum is the sha256 of the canonical (id-sorted, compact)
//! serialization of the `rules` array, so a packed index is byte-stable across
//! machines and [`checksum_ok`] can certify "this index is intact and current"
//! without refetching.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::git::workspace_root;

/// A single lint rule sourced directly from official documentation.
///
/// Mirrors the `rules[]` element in `SCHEMA.md`. `id`/`category`/`severity`/
/// `description`/`source` are required; `example_bad`/`example_good` are optional.
/// Field order and naming match the on-disk camelCase JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// Stable rule id (e.g. `unwrap_used`); the canonical sort key.
    pub id: String,
    /// Rule category (e.g. `correctness`).
    pub category: String,
    /// Severity bucket: `high`, `medium`, or `low`.
    pub severity: String,
    /// Human-readable description, taken verbatim from the official docs.
    pub description: String,
    /// Optional snippet that triggers the rule.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "exampleBad")]
    pub example_bad: Option<String>,
    /// Optional snippet that satisfies the rule.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "exampleGood"
    )]
    pub example_good: Option<String>,
    /// Direct URL to the rule's official documentation.
    pub source: String,
}

/// A packed lint index file: one tool's version-matched rule catalog.
///
/// Deserialized from `lint-index/<tool>.json`. Provenance fields
/// (`docs_base`, `fetched_at`) are optional so older or hand-trimmed snapshots
/// still load. `checksum` is `sha256:<hex>` of the canonical rules JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Index {
    /// Stable tool id and filename stem (e.g. `clippy`).
    pub tool: String,
    /// Lowercase language the tool lints (e.g. `rust`).
    pub language: String,
    /// Toolchain version this snapshot targets (e.g. `1.95.0`).
    #[serde(rename = "toolchainVersion")]
    pub toolchain_version: String,
    /// Official-docs version the rules were sourced from; may lag the toolchain.
    #[serde(rename = "docsVersion")]
    pub docs_version: String,
    /// Provenance: the crawl/source id (e.g. `rust-clippy`).
    pub source: String,
    /// Provenance: base URL the rules were crawled from.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "docsBase")]
    pub docs_base: Option<String>,
    /// Provenance: ISO-8601 timestamp of the crawl.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "fetchedAt")]
    pub fetched_at: Option<String>,
    /// `sha256:<hex>` of the canonical (id-sorted, compact) `rules` JSON.
    pub checksum: String,
    /// Number of rules; must equal `rules.len()`.
    #[serde(rename = "ruleCount")]
    pub rule_count: usize,
    /// The rule catalog.
    pub rules: Vec<Rule>,
}

/// The outcome of a fast-path resolution.
///
/// `Packed` is a verified hit: the file loaded, its checksum matched, and its
/// `docsVersion` covers the detected toolchain. `NeedsCrawl` carries the trigger
/// for the slower tiers (poll/pull, then crawl) — this module does not act on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// Fast-path hit — use these official rules directly.
    Packed(Index),
    /// Fast-path miss — the named tool/version is uncovered for `reason`.
    NeedsCrawl {
        /// Tool whose index is missing or stale.
        tool: String,
        /// Detected toolchain version that must be covered.
        version: String,
        /// Why the fast path failed (missing file / bad checksum / version mismatch).
        reason: String,
    },
}

/// Read and parse the packed index for `tool` from `lint-index/<tool>.json`
/// under the workspace root.
///
/// Returns `None` when the file is absent or fails to parse — callers treat a
/// `None` as "no packed index", falling back to the slower tiers. Does **not**
/// verify the checksum or version coverage; see [`checksum_ok`] / [`covers`].
pub fn load(tool: &str) -> Option<Index> {
    let path = index_path(tool);
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<Index>(&raw).ok()
}

/// Path to the packed index for `tool`: `<workspace>/lint-index/<tool>.json`.
fn index_path(tool: &str) -> PathBuf {
    workspace_root().join("lint-index").join(format!("{tool}.json"))
}

/// Canonical serialization of an index's rules: id-sorted, compact (no
/// whitespace) JSON of the `rules` array — the exact bytes the checksum covers.
fn canonical_rules_json(idx: &Index) -> String {
    let mut sorted = idx.rules.clone();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));
    // Compact form (serde_json default) is whitespace-free; matches SCHEMA.md.
    serde_json::to_string(&sorted).unwrap_or_default()
}

/// Recompute the canonical rules checksum and compare it to `idx.checksum`.
///
/// True only when `idx.checksum` is `sha256:<hex>` and `<hex>` equals the sha256
/// of [`canonical_rules_json`]. This is the fast-path integrity check: a match
/// certifies the packed rules are intact and current without any refetch.
pub fn checksum_ok(idx: &Index) -> bool {
    let Some(want) = idx.checksum.strip_prefix("sha256:") else {
        return false;
    };
    let mut hasher = Sha256::new();
    hasher.update(canonical_rules_json(idx).as_bytes());
    let got = hex_lower(&hasher.finalize());
    got.eq_ignore_ascii_case(want)
}

/// Lowercase hex encoding of a byte slice.
fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Whether the packed snapshot covers a detected toolchain version.
///
/// True when `idx.docs_version <= toolchain_version` under a relaxed semver
/// compare: the docs the rules were sourced from must not be *newer* than the
/// running toolchain (a newer toolchain can use older-docs rules; older docs
/// describing a newer toolchain would be a coverage gap). Unparsable versions
/// compare as equal segments, so a clean numeric match still resolves.
pub fn covers(idx: &Index, toolchain_version: &str) -> bool {
    semver_cmp(&idx.docs_version, toolchain_version) != std::cmp::Ordering::Greater
}

/// Compare two dotted version strings segment-by-segment as integers, ignoring
/// any non-numeric suffix on a segment and treating missing segments as 0.
fn semver_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let pa = version_parts(a);
    let pb = version_parts(b);
    let len = pa.len().max(pb.len());
    for i in 0..len {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// Parse the leading integer of each dot-separated segment (e.g. `1.95.0-beta`
/// → `[1, 95, 0]`). A segment with no leading digits contributes 0.
fn version_parts(v: &str) -> Vec<u64> {
    v.split('.')
        .map(|seg| {
            let digits: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<u64>().unwrap_or(0)
        })
        .collect()
}

/// Resolve a tool/language/version against the packed index — the tiered
/// fast-path decision.
///
/// Returns [`Resolution::Packed`] only on a fully verified hit: the file exists,
/// its `language` matches `lang`, its checksum is intact, and its `docsVersion`
/// covers `toolchain_version`. Any failure yields [`Resolution::NeedsCrawl`] with
/// a precise `reason` (missing file / language mismatch / bad checksum / version
/// mismatch). This function never crawls — it only reports the trigger.
pub fn resolve(tool: &str, lang: &str, toolchain_version: &str) -> Resolution {
    let needs = |reason: String| Resolution::NeedsCrawl {
        tool: tool.to_string(),
        version: toolchain_version.to_string(),
        reason,
    };

    let Some(idx) = load(tool) else {
        return needs(format!("no packed index file lint-index/{tool}.json"));
    };
    if !idx.language.eq_ignore_ascii_case(lang) {
        return needs(format!(
            "index language `{}` does not match `{lang}`",
            idx.language
        ));
    }
    if !checksum_ok(&idx) {
        return needs("checksum mismatch (packed rules altered or stale)".to_string());
    }
    if !covers(&idx, toolchain_version) {
        return needs(format!(
            "docsVersion {} does not cover toolchain {toolchain_version}",
            idx.docs_version
        ));
    }
    Resolution::Packed(idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a checksum-valid sample index conforming to SCHEMA.md. The checksum
    /// is computed from the canonical rules JSON so the fixture is self-consistent.
    fn sample_index() -> Index {
        let rules = vec![
            Rule {
                id: "unwrap_used".into(),
                category: "correctness".into(),
                severity: "high".into(),
                description: "Checks for `.unwrap()` calls on Result/Option.".into(),
                example_bad: Some("x.unwrap()".into()),
                example_good: None,
                source: "https://example.test/#unwrap_used".into(),
            },
            Rule {
                id: "needless_return".into(),
                category: "style".into(),
                severity: "low".into(),
                description: "Checks for return statements at the end of a block.".into(),
                example_bad: None,
                example_good: None,
                source: "https://example.test/#needless_return".into(),
            },
        ];
        let mut idx = Index {
            tool: "clippy".into(),
            language: "rust".into(),
            toolchain_version: "1.95.0".into(),
            docs_version: "1.82.0".into(),
            source: "rust-clippy".into(),
            docs_base: Some("https://example.test/rust-1.82.0".into()),
            fetched_at: Some("2026-06-20T00:00:00.000Z".into()),
            checksum: String::new(),
            rule_count: rules.len(),
            rules,
        };
        // Seal the fixture with its real canonical checksum.
        let mut hasher = Sha256::new();
        hasher.update(canonical_rules_json(&idx).as_bytes());
        idx.checksum = format!("sha256:{}", hex_lower(&hasher.finalize()));
        idx
    }

    #[test]
    fn checksum_round_trips_on_canonical_rules() {
        let idx = sample_index();
        assert!(checksum_ok(&idx));
    }

    #[test]
    fn checksum_is_order_independent() {
        // Reversing rule order must not change the canonical checksum.
        let mut idx = sample_index();
        idx.rules.reverse();
        assert!(checksum_ok(&idx));
    }

    #[test]
    fn checksum_detects_tampering() {
        let mut idx = sample_index();
        idx.rules[0].description = "tampered".into();
        assert!(!checksum_ok(&idx));
    }

    #[test]
    fn checksum_rejects_missing_prefix() {
        let mut idx = sample_index();
        idx.checksum = idx.checksum.trim_start_matches("sha256:").to_string();
        assert!(!checksum_ok(&idx));
    }

    #[test]
    fn covers_when_docs_at_or_below_toolchain() {
        let idx = sample_index(); // docsVersion 1.82.0
        assert!(covers(&idx, "1.95.0"));
        assert!(covers(&idx, "1.82.0"));
        assert!(!covers(&idx, "1.80.0"));
    }

    #[test]
    fn semver_handles_suffixes_and_missing_segments() {
        use std::cmp::Ordering::*;
        assert_eq!(semver_cmp("1.82.0", "1.82"), Equal);
        assert_eq!(semver_cmp("1.95.0-beta", "1.95.0"), Equal);
        assert_eq!(semver_cmp("2.0.0", "1.99.99"), Greater);
        assert_eq!(semver_cmp("1.9.0", "1.10.0"), Less);
    }

    #[test]
    fn resolve_packed_on_verified_hit() {
        // load() reads from disk, so exercise the verification path directly via
        // the same checks resolve() runs, plus the in-memory resolve helpers.
        let idx = sample_index();
        assert!(idx.language.eq_ignore_ascii_case("rust"));
        assert!(checksum_ok(&idx));
        assert!(covers(&idx, &idx.toolchain_version));
    }

    #[test]
    fn resolve_needs_crawl_when_file_missing() {
        // A tool with no packed file must report a crawl trigger, never panic.
        let r = resolve("definitely-not-a-real-tool-xyz", "rust", "1.95.0");
        match r {
            Resolution::NeedsCrawl { tool, version, reason } => {
                assert_eq!(tool, "definitely-not-a-real-tool-xyz");
                assert_eq!(version, "1.95.0");
                assert!(reason.contains("no packed index"));
            }
            Resolution::Packed(_) => panic!("expected NeedsCrawl for a missing tool"),
        }
    }
}
