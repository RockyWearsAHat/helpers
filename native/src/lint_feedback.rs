//! Human/AI feedback loop for the linter.
//!
//! Two kinds of feedback close the loop between what the linter reports and what the developer
//! actually wants:
//!
//! * **`false_positive`** — a reported finding was wrong. Recorded per rule/file/line. Once a rule
//!   collects [`SUPPRESS_THRESHOLD`] *distinct* false-positive flags in a project, it is
//!   auto-suppressed for that project via a [`crate::tools::lint::LintConfig`] merge (same effect as
//!   adding it to `ignore_rules`), so the noise stops without the developer editing config by hand.
//! * **`missed`** — something that should have been flagged was not. Surfaced in lint output as a
//!   "pending rule" prompting the developer to formalize it with `lint_rule`. Missed flags never
//!   auto-invent a firing pattern from a single report — that is a deliberate policy (see the tool
//!   `lint_flag`), because one example is not enough signal to compile a reliable rule.
//!
//! Storage is an append-only JSONL log at `.helpers/lint-feedback.jsonl` in the project root: one
//! [`FeedbackRecord`] per line. Append-only means concurrent flags never corrupt each other and the
//! full history stays auditable and shareable (see `lint_submit`).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Distinct false-positive flags a single rule must collect before it is auto-suppressed for the
/// project. "Distinct" counts unique `(file, line)` sites, so re-flagging the exact same finding
/// does not inflate the count — two genuinely different wrong hits are required.
pub const SUPPRESS_THRESHOLD: usize = 2;

/// The `false_positive` action string: a reported finding was wrong.
pub const ACTION_FALSE_POSITIVE: &str = "false_positive";
/// The `missed` action string: something that should have been flagged was not.
pub const ACTION_MISSED: &str = "missed";

/// One appended feedback event. `false_positive` records carry `rule`/`file`/`line`; `missed`
/// records carry `file`/`line`/`description` plus optional rule/severity/snippet hints. Optional
/// fields are omitted from the JSON when absent to keep each line small and self-describing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedbackRecord {
    /// ISO-8601 UTC time the flag was recorded (from [`crate::util::now_iso`]).
    pub timestamp: String,
    /// Either [`ACTION_FALSE_POSITIVE`] or [`ACTION_MISSED`].
    pub action: String,
    /// The rule id: the wrongly-fired rule (false_positive) or a suggested id (missed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    /// Repo-relative (or absolute) path the flag concerns.
    pub file: String,
    /// 1-based source line the flag concerns, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    /// Free-text reason a finding was a false positive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// What should have been flagged (required for `missed`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Suggested severity for a missed finding (`high`/`medium`/`low`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    /// Language of a missed finding, used to route a seeded draft rule to the right file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// A code snippet showing the pattern that should have been caught.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bad: Option<String>,
    /// A code snippet showing the correct form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub good: Option<String>,
}

/// The feedback log path for a project root: `<root>/.helpers/lint-feedback.jsonl`.
pub fn feedback_path(project_root: &Path) -> PathBuf {
    project_root.join(".helpers/lint-feedback.jsonl")
}

/// Append one record to the project's feedback log, creating `.helpers/` if needed.
///
/// Serializes to a single JSON line (no embedded newlines) so the file stays valid JSONL.
pub fn append(project_root: &Path, record: &FeedbackRecord) -> Result<(), String> {
    let path = feedback_path(project_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("lint_feedback: {e}"))?;
    }
    let line = serde_json::to_string(record).map_err(|e| format!("lint_feedback: {e}"))?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("lint_feedback: {e}"))?;
    writeln!(f, "{line}").map_err(|e| format!("lint_feedback: {e}"))
}

/// Read every well-formed record from the project's feedback log. Missing file ⇒ empty vec;
/// malformed lines are skipped rather than aborting, so one bad append never blinds the reader.
pub fn read_all(project_root: &Path) -> Vec<FeedbackRecord> {
    let path = feedback_path(project_root);
    let Ok(raw) = std::fs::read_to_string(&path) else { return Vec::new() };
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<FeedbackRecord>(l).ok())
        .collect()
}

/// Rule ids that have earned auto-suppression: those with at least `threshold` distinct
/// `(file, line)` false-positive sites. Deterministic ordering via `BTreeSet`.
pub fn auto_suppressed(records: &[FeedbackRecord], threshold: usize) -> BTreeSet<String> {
    let mut sites: BTreeMap<String, BTreeSet<(String, Option<u64>)>> = BTreeMap::new();
    for r in records.iter().filter(|r| r.action == ACTION_FALSE_POSITIVE) {
        if let Some(rule) = r.rule.clone() {
            sites.entry(rule).or_default().insert((r.file.clone(), r.line));
        }
    }
    sites
        .into_iter()
        .filter(|(_, s)| s.len() >= threshold)
        .map(|(rule, _)| rule)
        .collect()
}

/// The `missed` records to surface as pending rules, in insertion order.
pub fn pending_missed(records: &[FeedbackRecord]) -> Vec<&FeedbackRecord> {
    records.iter().filter(|r| r.action == ACTION_MISSED).collect()
}

/// Merge feedback-driven auto-suppressions into a loaded [`crate::tools::lint::LintConfig`],
/// returning the rule ids that feedback added (those not already in the user's `ignore_rules`).
///
/// This is the "config merge" the trainer relies on each run: reading the append-only log and
/// folding earned suppressions into the same `ignore_rules` list the user could have written by
/// hand — so the lint pass suppresses them through its normal path, with no special-casing.
pub fn merge_auto_suppressed(
    config: &mut crate::tools::lint::LintConfig,
    project_root: &Path,
) -> BTreeSet<String> {
    let suppressed = auto_suppressed(&read_all(project_root), SUPPRESS_THRESHOLD);
    let already: BTreeSet<&str> = config.ignore_rules.iter().map(String::as_str).collect();
    let added: BTreeSet<String> = suppressed
        .iter()
        .filter(|r| !already.contains(r.as_str()))
        .cloned()
        .collect();
    for rule in &added {
        config.ignore_rules.push(rule.clone());
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(action: &str, rule: Option<&str>, file: &str, line: Option<u64>) -> FeedbackRecord {
        FeedbackRecord {
            timestamp: "2026-07-01T00:00:00.000Z".into(),
            action: action.into(),
            rule: rule.map(str::to_string),
            file: file.into(),
            line,
            reason: None,
            description: None,
            severity: None,
            language: None,
            bad: None,
            good: None,
        }
    }

    #[test]
    fn append_and_read_roundtrip() {
        let dir = std::env::temp_dir().join(format!("lf-rt-{}", crate::util::now_millis()));
        let r = rec(ACTION_FALSE_POSITIVE, Some("no_x"), "a.py", Some(10));
        append(&dir, &r).unwrap();
        append(&dir, &rec(ACTION_MISSED, None, "b.py", Some(5))).unwrap();
        let back = read_all(&dir);
        assert_eq!(back.len(), 2);
        assert_eq!(back[0], r, "first record roundtrips exactly");
        assert_eq!(back[1].action, ACTION_MISSED);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_lines_are_skipped() {
        let dir = std::env::temp_dir().join(format!("lf-bad-{}", crate::util::now_millis()));
        std::fs::create_dir_all(dir.join(".helpers")).unwrap();
        std::fs::write(
            feedback_path(&dir),
            "not json\n{\"timestamp\":\"t\",\"action\":\"missed\",\"file\":\"x\"}\n\n",
        )
        .unwrap();
        let back = read_all(&dir);
        assert_eq!(back.len(), 1, "only the valid line parses");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn auto_suppress_after_threshold_distinct_sites() {
        // Same rule at two distinct sites → suppressed; a rule flagged once → not.
        let records = vec![
            rec(ACTION_FALSE_POSITIVE, Some("no_x"), "a.py", Some(10)),
            rec(ACTION_FALSE_POSITIVE, Some("no_x"), "a.py", Some(20)),
            rec(ACTION_FALSE_POSITIVE, Some("no_y"), "b.py", Some(1)),
        ];
        let s = auto_suppressed(&records, SUPPRESS_THRESHOLD);
        assert!(s.contains("no_x"), "two distinct FP sites suppresses: {s:?}");
        assert!(!s.contains("no_y"), "one FP site does not suppress: {s:?}");
    }

    #[test]
    fn identical_repeated_flag_does_not_reach_threshold() {
        // Re-flagging the exact same (file, line) counts once.
        let records = vec![
            rec(ACTION_FALSE_POSITIVE, Some("no_x"), "a.py", Some(10)),
            rec(ACTION_FALSE_POSITIVE, Some("no_x"), "a.py", Some(10)),
        ];
        assert!(!auto_suppressed(&records, SUPPRESS_THRESHOLD).contains("no_x"));
    }

    #[test]
    fn pending_missed_surfaces_only_missed() {
        let records = vec![
            rec(ACTION_FALSE_POSITIVE, Some("no_x"), "a.py", Some(10)),
            rec(ACTION_MISSED, None, "b.py", Some(5)),
        ];
        let p = pending_missed(&records);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].file, "b.py");
    }

    #[test]
    fn merge_adds_suppressed_and_reports_new() {
        let dir = std::env::temp_dir().join(format!("lf-merge-{}", crate::util::now_millis()));
        append(&dir, &rec(ACTION_FALSE_POSITIVE, Some("no_x"), "a.py", Some(10))).unwrap();
        append(&dir, &rec(ACTION_FALSE_POSITIVE, Some("no_x"), "a.py", Some(20))).unwrap();
        let mut cfg = crate::tools::lint::LintConfig::default();
        let added = merge_auto_suppressed(&mut cfg, &dir);
        assert!(added.contains("no_x"));
        assert!(cfg.ignore_rules.iter().any(|r| r == "no_x"), "merged into ignore_rules");
        // Second merge reports nothing new (already present) but keeps it suppressed.
        let added2 = merge_auto_suppressed(&mut cfg, &dir);
        assert!(added2.is_empty(), "no duplicate additions: {added2:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
