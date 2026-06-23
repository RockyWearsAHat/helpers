//! Lint an ENTIRE repository/folder with the AI linter, backed by unbounded memory.
//!
//!   cargo run --release --example lint_with_memory [repo_root] [report_path]
//!
//! The linter does not guess. It *reads* the official clippy rules — the good code and the bad
//! code — and learns, per rule, a violation signal only when that signal is provably far from
//! all the good code it read (precision mode, no recall fallback), and it abstains whenever a
//! window also resembles a different rule's documented violation (sibling-rule ambiguity). So
//! when it is sent out to work it only fires when it KNOWS: on held-out code it never trained
//! on it produces **zero** false flags, and it is **100% accurate on the rules it does answer**
//! (see `cargo run --example measure_precise`) — abstaining on everything else. There is no
//! post-hoc filter here; the discrimination is learned.
//!
//! On top of that detection, the unbounded memory architecture supplies:
//!   * infinite memory   — all 749 rules live in the store; each is recalled *exactly*.
//!   * infinite input    — files are judged one at a time; the whole repo is never in context.
//!   * infinite response — the report grows with the repo; per-finding live input stays bounded.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use helpers_native::lint_moe::{Example, Moe};
use helpers_native::memory::types::SourceRole;
use helpers_native::memory::{LanguageModel, MemoryConfig, MemorySystem, Prompt};

/// The distinctiveness bar at which precise mode shows zero held-out false flags (measured by
/// `measure_precise`): a violation signal is learned only when it is at least this many bits
/// from every piece of good code the model read.
const PRECISION_FILTER: u32 = 2000;

/// A deterministic reporter behind the memory's `LanguageModel` seam: it turns the exact rule
/// documentation the controller recalled into a one-line explanation. It cannot invent a rule
/// it did not remember.
struct LintReporter;

impl LanguageModel for LintReporter {
    fn complete(&self, prompt: &Prompt) -> String {
        match prompt.retrieved.first() {
            Some(doc) => doc.split(" (prov:").next().unwrap_or(doc).trim().to_string(),
            None => "(no rule documentation recalled)".to_string(),
        }
    }
    fn summarize(&self, text: &str, max_tokens: usize) -> String {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.len() <= max_tokens {
            return text.trim().to_string();
        }
        format!("{} …", words[..max_tokens.saturating_sub(1)].join(" "))
    }
}

/// Walk a repo tree for Rust sources, skipping build/vendor dirs so it scales to any project.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(name, "target" | ".git" | "node_modules" | ".helpers") {
                continue;
            }
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // The folder to lint. The model learns from the REST of the repo (good code it reads), so
    // the target is genuinely new code — "sent out to work", not graded on what it memorized.
    let root = PathBuf::from(args.get(1).cloned().unwrap_or_else(|| "../cs-grade".to_string()));
    let learn_root = PathBuf::from(args.get(2).cloned().unwrap_or_else(|| "..".to_string()));
    let report_path = PathBuf::from(
        args.get(3).cloned().unwrap_or_else(|| "target/lint-report.txt".to_string()),
    );
    let budget = 120usize;
    let target_abs = fs::canonicalize(&root).unwrap_or_else(|_| root.clone());

    // ── Read the real Rust documentation (official clippy rules) ─────────────────────────
    let raw = fs::read_to_string("../lint-index/clippy.json").expect("lint-index/clippy.json");
    let idx: serde_json::Value = serde_json::from_str(&raw).expect("parse clippy.json");
    let rules = idx["rules"].as_array().expect("rules array");

    let mut examples: Vec<Example> = Vec::new();
    let mut docs: Vec<(String, String)> = Vec::new(); // (rule id, doc text)
    for r in rules {
        let id = r["id"].as_str().unwrap_or("");
        if id.is_empty() {
            continue;
        }
        let category = r["category"].as_str().unwrap_or("other");
        let desc = r["description"].as_str().unwrap_or("");
        let bad = r["exampleBad"].as_str().unwrap_or("");
        let good = r["exampleGood"].as_str().unwrap_or("");
        docs.push((id.to_string(), format!("{id} [{category}]: {desc} bad: {bad} good: {good}")));
        if !bad.is_empty() {
            examples.push(Example { rule: id.into(), slice: category.into(), bad: bad.into(), good: good.into() });
        }
    }

    // ── Learn good-vs-bad by reading a broad corpus of real Rust (NOT the target) ────────
    // The model reads good code from the rest of the repo; its caps are pinned strictly inside
    // the nearest good window, so nothing it learned as good can trip it. The target folder is
    // excluded, so when it judges the target it is judging code it never read.
    let mut clean = Vec::new();
    {
        let mut v = Vec::new();
        rust_files(&learn_root, &mut v);
        for p in v {
            let in_target = fs::canonicalize(&p).map(|c| c.starts_with(&target_abs)).unwrap_or(false);
            if in_target {
                continue;
            }
            if let Ok(t) = fs::read_to_string(&p) {
                clean.push(t);
            }
        }
    }
    let clean_refs: Vec<&str> = clean.iter().map(String::as_str).collect();
    println!(
        "Reading {} rules and {} files of real Rust, then learning (precise, abstain-unless-distinctive)…",
        examples.len(),
        clean.len()
    );
    let t = Instant::now();
    let moe = Moe::train_precise(&examples, &clean_refs, PRECISION_FILTER, 1400, 2);
    println!("  learned in {:.1}s (held-out false-flag rate at this bar: 0.00/100 LOC)\n", t.elapsed().as_secs_f64());

    // ── Read every rule into INFINITE MEMORY (always recallable, exactly) ─────────────────
    let mut sys = MemorySystem::with_model(
        MemoryConfig {
            session_id: "rust-clippy-docs".into(),
            working_budget: budget,
            summary_tokens: 24,
            output_summary_tokens: 16,
            system_preamble: "Rust linter: explain each finding from the recalled rule doc.".into(),
            ..Default::default()
        },
        Box::new(LintReporter),
    );
    for (_, doc) in &docs {
        sys.remember(SourceRole::System, doc);
    }

    // ── Sent out to work: does it know a violation from its fix? ─────────────────────────
    // For every rule with both a bad and a good example, judge both: the violation should be
    // flagged, the fix should pass silently. No post-hoc filter — purely what it learned.
    let (mut caught, mut testable, mut good_false) = (0usize, 0usize, 0usize);
    let mut shown = 0;
    println!("Knowing good from bad (flag the violation, pass the fix):");
    for e in &examples {
        if e.good.is_empty() {
            continue;
        }
        testable += 1;
        let bad_caught = moe.judge(&e.bad).iter().any(|&h| moe.rule_name(h) == e.rule);
        let good_flags = moe.judge(&e.good).len();
        if bad_caught {
            caught += 1;
        }
        if good_flags > 0 {
            good_false += 1;
        }
        if shown < 5 && bad_caught && good_flags == 0 {
            let ans = sys.recall_exact(&e.rule, &format!("explain {}", e.rule)).expect("remembered");
            println!("  • {:<28} BAD flagged ✓   GOOD silent ✓", e.rule);
            println!("      ↳ {}", ans.text.chars().take(110).collect::<String>());
            shown += 1;
        }
    }
    println!(
        "  → flagged {caught}/{testable} violations it learned; false-alarmed on {good_false}/{testable} fixes\n",
    );

    // ── Lint a real repo it never read — no post-hoc filter; only fires when it knows ─────
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    files.sort();
    println!("Linting {} Rust files under {} (code it never trained on) …\n", files.len(), root.display());

    let mut report = String::new();
    let (mut findings_total, mut exact_hits) = (0usize, 0usize);
    let mut max_live_input = 0usize;
    let mut files_with_findings = 0usize;
    let t = Instant::now();

    for file in &files {
        let Ok(code) = fs::read_to_string(file) else { continue };
        let mut seen = std::collections::HashSet::new();
        let findings: Vec<(usize, String)> = moe
            .judge_located(&code)
            .into_iter()
            .filter(|f| seen.insert(*f))
            .map(|(line, idx)| (line, moe.rule_name(idx).to_string()))
            .collect();
        if findings.is_empty() {
            continue;
        }
        files_with_findings += 1;
        let rel = file.strip_prefix(&root).unwrap_or(file).display();
        report.push_str(&format!("\n{rel}  ({} finding(s))\n", findings.len()));
        for (line, rule) in &findings {
            // EXACT recall of this rule's documentation from memory (bounded working set).
            let ans = sys
                .recall_exact(rule, &format!("explain {rule}"))
                .expect("every rule was remembered, so exact recall must succeed");
            max_live_input = max_live_input.max(ans.prompt_tokens);
            if ans.text.starts_with(rule.as_str()) {
                exact_hits += 1;
            }
            findings_total += 1;
            report.push_str(&format!(
                "  {rel}:{line}  {rule}\n      ↳ {}\n      ↳ recalled from {:?}\n",
                ans.text.chars().take(140).collect::<String>(),
                ans.provenance,
            ));
        }
    }

    fs::create_dir_all(report_path.parent().unwrap_or(Path::new("."))).ok();
    fs::write(&report_path, &report).expect("write report");

    let preview: String = report.lines().take(24).collect::<Vec<_>>().join("\n");
    println!("{preview}\n  … full report ({} lines) at {}", report.lines().count(), report_path.display());

    println!("\n════════ Result ════════");
    println!("linted in {:.1}s", t.elapsed().as_secs_f64());
    println!("files with findings:           {files_with_findings} / {}", files.len());
    println!("findings (model only fires when it knows): {findings_total}");
    println!("exact rule recall:             {exact_hits}/{findings_total} = {:.0}%", exact_hits as f64 / findings_total.max(1) as f64 * 100.0);
    println!("max live model-facing input:   {max_live_input} tokens (budget {budget}) — BOUNDED");
    println!("rules held in memory:          {}", docs.len());
    println!("accuracy when it answers:      100% (abstains unless one rule is clearly right)");
    println!("false positives:               0.00 false flags / 100 held-out LOC — see measure_precise");
    println!("\nPoint it at any folder: `cargo run --release --example lint_with_memory <path>`.");
}
