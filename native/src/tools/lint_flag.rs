//! `lint_flag` — the linter's feedback tool.
//!
//! Closes the loop between what `lint` reports and what the developer actually wants, by recording
//! two kinds of correction to `.helpers/lint-feedback.jsonl` (see [`crate::lint_feedback`]):
//!
//! * `action="false_positive"` — a reported finding was wrong. After [`SUPPRESS_THRESHOLD`]
//!   distinct false-positive flags for the same rule, that rule is auto-suppressed for the project
//!   on the next `lint` run (a `LintConfig` `ignore_rules` merge).
//! * `action="missed"` — something that should have been flagged was not. It is surfaced in `lint`
//!   output as a "pending rule". If the flag carries a `bad` snippet and a `language`, a rule
//!   *draft* is seeded under `.helpers/lint-rules/drafts/<lang>.md` for the developer to review and
//!   promote with `lint_rule`. Drafts deliberately live outside the trained rule set so a single
//!   flag never auto-invents a firing pattern.
//!
//! Feedback is local until shared: the tool result reminds the developer they can push it upstream
//! with `lint_submit include_feedback=true`.

use serde_json::{json, Value};

use crate::lint_feedback::{
    self, FeedbackRecord, ACTION_FALSE_POSITIVE, ACTION_MISSED, SUPPRESS_THRESHOLD,
};
use crate::proto::{text, ToolResult};

/// Resolve the project root from the `root` arg, defaulting to the current workspace.
fn root_arg(args: &Value) -> std::path::PathBuf {
    args.get("root")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(crate::git::workspace_root)
}

/// Record a feedback flag and report its effect. Dispatches on `action`.
pub fn run(args: &Value) -> ToolResult {
    match args.get("action").and_then(Value::as_str) {
        Some(ACTION_FALSE_POSITIVE) => run_false_positive(args),
        Some(ACTION_MISSED) => run_missed(args),
        other => Err(format!(
            "lint_flag: unknown action {:?}. Valid: `false_positive` | `missed`.",
            other.unwrap_or("")
        )),
    }
}

/// Record that a reported finding was wrong, and report how close the rule is to auto-suppression.
fn run_false_positive(args: &Value) -> ToolResult {
    let rule = args["rule"].as_str().ok_or("lint_flag: `rule` is required for false_positive")?;
    let file = args["file"].as_str().ok_or("lint_flag: `file` is required for false_positive")?;
    let line = args["line"].as_u64().ok_or("lint_flag: `line` is required for false_positive")?;
    let reason = args["reason"].as_str().map(str::to_string);
    let root = root_arg(args);

    lint_feedback::append(&root, &FeedbackRecord {
        timestamp: crate::util::now_iso(),
        action: ACTION_FALSE_POSITIVE.into(),
        train: Some(crate::lint_train::train_version().to_string()),
        rule: Some(rule.to_string()),
        file: file.to_string(),
        line: Some(line),
        reason,
        description: None,
        severity: None,
        language: None,
        bad: None,
        good: None,
    })?;

    // Count distinct sites now on record for this rule to tell the developer where it stands.
    let records = lint_feedback::read_all(&root);
    let suppressed = lint_feedback::auto_suppressed(&records, SUPPRESS_THRESHOLD);
    let mut out = format!(
        "Recorded false positive: rule `{rule}` at {file}:{line}.\n\
         Logged to .helpers/lint-feedback.jsonl.\n"
    );
    if suppressed.contains(rule) {
        out.push_str(&format!(
            "\nRule `{rule}` now has {SUPPRESS_THRESHOLD}+ distinct false-positive flags — it is \
             auto-suppressed for this project and will not appear on the next `lint` run.\n\
             Re-enable any time with `lint_config action=unignore rule={rule}`.\n"
        ));
    } else {
        out.push_str(&format!(
            "\nOne more distinct false-positive flag for `{rule}` (reaching {SUPPRESS_THRESHOLD}) \
             will auto-suppress it for this project.\n"
        ));
    }
    out.push_str(SHARE_HINT);
    Ok(vec![text(out)])
}

/// Record a missed finding, optionally seeding a rule draft, and report next steps.
fn run_missed(args: &Value) -> ToolResult {
    let file = args["file"].as_str().ok_or("lint_flag: `file` is required for missed")?;
    let description = args["description"]
        .as_str()
        .ok_or("lint_flag: `description` is required for missed")?;
    let line = args["line"].as_u64();
    let rule = args["rule"].as_str().map(str::to_string);
    let severity = args["severity"].as_str().map(str::to_string);
    let language = args["language"].as_str().map(str::to_string);
    let bad = args["bad"].as_str().map(str::to_string);
    let good = args["good"].as_str().map(str::to_string);
    let root = root_arg(args);

    lint_feedback::append(&root, &FeedbackRecord {
        timestamp: crate::util::now_iso(),
        action: ACTION_MISSED.into(),
        train: Some(crate::lint_train::train_version().to_string()),
        rule: rule.clone(),
        file: file.to_string(),
        line,
        reason: None,
        description: Some(description.to_string()),
        severity: severity.clone(),
        language: language.clone(),
        bad: bad.clone(),
        good: good.clone(),
    })?;

    let loc = line.map(|l| format!("{file}:{l}")).unwrap_or_else(|| file.to_string());
    let mut out = format!(
        "Recorded missed finding at {loc}: {description}\n\
         Logged to .helpers/lint-feedback.jsonl.\n\
         It will surface as a PENDING rule in `lint` output until formalized.\n"
    );

    // Only seed a draft when there is enough signal (a bad snippet + a language). A draft is a
    // scaffold, never a firing rule: it lives outside the trained set so no pattern is auto-invented.
    if let (Some(bad_code), Some(lang)) = (bad.as_ref(), language.as_ref()) {
        let draft_path = seed_draft(&root, lang, rule.as_deref(), description, severity.as_deref(), bad_code, good.as_deref())?;
        out.push_str(&format!(
            "\nSeeded a rule DRAFT at {}.\n\
             Review it, then promote it to a live rule with `lint_rule` \
             (language={lang}, add the bad/good pair). Drafts do not fire until promoted.\n",
            draft_path.display()
        ));
    } else {
        out.push_str(
            "\nTo turn this into an enforced rule, call `lint_rule` with a bad/good example pair \
             (or re-flag with `language` + `bad` to seed an editable draft).\n",
        );
    }
    out.push_str(SHARE_HINT);
    Ok(vec![text(out)])
}

/// Reminder shown on every flag: feedback is local until explicitly shared.
const SHARE_HINT: &str =
    "\nShare this feedback upstream (opt-in): `lint_submit include_feedback=true`.\n";

/// Append a rule draft to `.helpers/lint-rules/drafts/<lang>.md` and return the file path.
///
/// The `drafts/` subdirectory is intentional: the trainer reads only top-level
/// `.helpers/lint-rules/*.md`, so drafts stay out of the trained model until a developer promotes
/// one with `lint_rule`. This honors the policy that one missed flag never auto-invents a pattern.
fn seed_draft(
    root: &std::path::Path,
    lang: &str,
    rule: Option<&str>,
    description: &str,
    severity: Option<&str>,
    bad: &str,
    good: Option<&str>,
) -> Result<std::path::PathBuf, String> {
    let dir = root.join(".helpers/lint-rules/drafts");
    std::fs::create_dir_all(&dir).map_err(|e| format!("lint_flag: {e}"))?;
    let path = dir.join(format!("{lang}.md"));

    let id = rule.unwrap_or("draft_rule");
    let sev = severity.unwrap_or("medium");
    let block = format!(
        "\n## {id} [{sev}] [draft]\n\n{description}\n\n\
         ```{lang}:bad\n{bad}\n```\n\n\
         ```{lang}:good\n{}\n```\n",
        good.unwrap_or("")
    );
    let mut content = std::fs::read_to_string(&path).unwrap_or_default();
    content.push_str(&block);
    std::fs::write(&path, &content).map_err(|e| format!("lint_flag: {e}"))?;
    Ok(path)
}

/// MCP schema for the `lint_flag` tool.
pub fn schema() -> Value {
    json!({
        "name": "lint_flag",
        "description": "Give the linter feedback so it learns from mistakes. \
                        action=\"false_positive\": a reported finding was wrong — pass rule, file, line (optional reason). \
                        After 2 distinct false-positive flags, that rule is auto-suppressed for the project on the next lint run. \
                        action=\"missed\": something that should have been flagged was not — pass file, description (optional line, rule id, severity). \
                        Missed flags surface as PENDING rules in lint output; if you also pass language + bad (and optional good), a rule draft is seeded under .helpers/lint-rules/drafts/ for you to promote with lint_rule. \
                        Feedback is stored append-only in .helpers/lint-feedback.jsonl and can be shared with `lint_submit include_feedback=true`.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "action":      { "type": "string", "enum": ["false_positive", "missed"], "description": "false_positive=a finding was wrong | missed=something should have been flagged." },
                "rule":        { "type": "string", "description": "Rule id. Required for false_positive; optional suggested id for missed." },
                "file":        { "type": "string", "description": "File the flag concerns (repo-relative or absolute)." },
                "line":        { "type": "integer", "description": "1-based line. Required for false_positive; optional for missed." },
                "reason":      { "type": "string", "description": "Why the finding was a false positive (false_positive only)." },
                "description": { "type": "string", "description": "What should have been flagged (required for missed)." },
                "severity":    { "type": "string", "enum": ["high", "medium", "low"], "description": "Suggested severity for a missed finding." },
                "language":    { "type": "string", "description": "Language of a missed finding — enables seeding a rule draft (with `bad`)." },
                "bad":         { "type": "string", "description": "Snippet showing the pattern that should have been caught (seeds a draft with `language`)." },
                "good":        { "type": "string", "description": "Snippet showing the correct form (for the seeded draft)." },
                "root":        { "type": "string", "description": "Project root. Defaults to the workspace root." }
            },
            "required": ["action", "file"]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn false_positive_twice_then_lint_suppresses() {
        // Simulated tool-call sequence: flag the same rule at two distinct sites, then confirm the
        // config merge that `lint` performs would suppress it.
        let dir = std::env::temp_dir().join(format!("lflag-fp-{}", crate::util::now_millis()));

        let r1 = run(&json!({ "action": "false_positive", "rule": "no_x", "file": "a.py", "line": 10, "root": dir.to_str().unwrap() }));
        assert!(r1.is_ok(), "first flag ok");

        let r2 = run(&json!({ "action": "false_positive", "rule": "no_x", "file": "a.py", "line": 20, "root": dir.to_str().unwrap() }));
        let msg = r2.unwrap()[0].text.clone();
        assert!(msg.contains("auto-suppressed"), "second flag reports suppression: {msg}");

        // The merge `lint` runs picks it up.
        let mut cfg = crate::tools::lint::LintConfig::default();
        let added = crate::lint_feedback::merge_auto_suppressed(&mut cfg, &dir);
        assert!(added.contains("no_x"), "rule merged into ignore list");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missed_with_snippet_seeds_draft_outside_trained_set() {
        let dir = std::env::temp_dir().join(format!("lflag-miss-{}", crate::util::now_millis()));
        let res = run(&json!({
            "action": "missed", "file": "a.py", "line": 3,
            "description": "eval on user input", "language": "python",
            "rule": "no_eval", "bad": "eval(x)", "good": "ast.literal_eval(x)",
            "root": dir.to_str().unwrap()
        }));
        let msg = res.unwrap()[0].text.clone();
        assert!(msg.contains("DRAFT"), "reports a seeded draft: {msg}");

        // Draft lives under drafts/ so the trainer (top-level *.md only) ignores it.
        let draft = dir.join(".helpers/lint-rules/drafts/python.md");
        assert!(draft.exists(), "draft file written");
        assert!(!dir.join(".helpers/lint-rules/python.md").exists(), "no live rule auto-created");
        let body = std::fs::read_to_string(&draft).unwrap();
        assert!(body.contains("[draft]") && body.contains("eval(x)"));

        // And it is a pending record.
        let records = crate::lint_feedback::read_all(&dir);
        let pending = crate::lint_feedback::pending_missed(&records);
        assert_eq!(pending.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_required_field_errors() {
        assert!(run(&json!({ "action": "false_positive", "file": "a.py", "line": 1 })).is_err());
        assert!(run(&json!({ "action": "bogus", "file": "a.py" })).is_err());
    }
}
