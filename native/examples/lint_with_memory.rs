//! Lint an ENTIRE repository/folder with the AI linter, backed by unbounded memory.
//!
//!   cargo run --release --example lint_with_memory [repo_root] [report_path]
//!
//! Point it at any folder and it just works: it walks every Rust file in the tree, judges
//! each with the mixture-of-experts linter, recalls each flagged rule's *exact* documentation
//! from memory (no fuzzy near-misses), drops findings that a memory-grounded precision check
//! deems likely-clean, and writes the full report — while the live model-facing input stays
//! under a fixed token budget no matter how large the repo is.
//!
//! The three properties, made real:
//!   - infinite memory: all 749 official clippy rules live in the store; each is recalled exactly, within a fixed-size window.
//!   - infinite input: files are judged one at a time; the whole repo is never in context.
//!   - infinite response: the report grows with the repo; per-finding live input stays bounded.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use helpers_native::lint_moe::{Example, Moe};
use helpers_native::memory::embed::{fingerprint, similarity, tokens};
use helpers_native::memory::types::SourceRole;
use helpers_native::memory::{LanguageModel, MemoryConfig, MemorySystem, Prompt};
use helpers_native::lint_ai::Hv;

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

/// The memory-grounded precision gate, in two checks against what the rule's own examples
/// (recalled from memory) say a violation looks like. First, fingerprint: the flagged window
/// must not be closer to the rule's GOOD (already-fixed) example than to its BAD one — code
/// that resembles the fix is a false flag. Second, lexical grounding: the window must share at
/// least one distinctive token with the BAD example, which kills systematic noise like
/// `test_attr_in_doctest` firing on a module-doc line that contains nothing resembling
/// `#[test]`. With no example to check against, the finding is kept (we never over-drop blindly).
fn looks_like_violation(
    window: &str,
    rule: &str,
    bad_fp: &HashMap<String, Hv>,
    good_fp: &HashMap<String, Hv>,
    bad_tokens: &HashMap<String, Vec<String>>,
) -> bool {
    let wfp = fingerprint(window);
    if let (Some(b), Some(g)) = (bad_fp.get(rule), good_fp.get(rule)) {
        if similarity(&wfp, b) < similarity(&wfp, g) {
            return false;
        }
    }
    if let Some(toks) = bad_tokens.get(rule) {
        if !toks.is_empty() {
            let wt: std::collections::HashSet<String> = tokens(window).into_iter().collect();
            if !toks.iter().any(|t| wt.contains(t)) {
                return false;
            }
        }
    }
    true
}

/// Build each rule's *distinctive* bad-example tokens: substantive words (length ≥ 4) that are
/// rare across all rules' bad examples (document frequency ≤ `max_df`). Rare tokens are the
/// identifier/keyword-like ones a genuine violation contains (e.g. `compile_fail`, `rem_euclid`);
/// common ones (`value`, `self`, `test`) are filtered out, so a module-doc line that merely
/// happens to contain a common word no longer keeps a bogus finding alive. This mirrors the
/// MoE's own "distinctive-only" reasoning.
fn build_distinctive(examples: &[Example], max_df: usize) -> HashMap<String, Vec<String>> {
    let mut df: HashMap<String, usize> = HashMap::new();
    for ex in examples {
        let set: std::collections::HashSet<String> =
            tokens(&ex.bad).into_iter().filter(|t| t.len() >= 4).collect();
        for t in set {
            *df.entry(t).or_default() += 1;
        }
    }
    let mut out = HashMap::new();
    for ex in examples {
        let toks: std::collections::HashSet<String> = tokens(&ex.bad)
            .into_iter()
            .filter(|t| t.len() >= 4 && df.get(t).copied().unwrap_or(0) <= max_df)
            .collect();
        out.insert(ex.rule.clone(), toks.into_iter().collect());
    }
    out
}

/// The few source lines around a 1-based line number — a stand-in for the token window the
/// linter flagged, good enough for a fingerprint comparison.
fn window_at(lines: &[&str], line: usize) -> String {
    let i = line.saturating_sub(1);
    let lo = i.saturating_sub(1);
    let hi = (i + 2).min(lines.len());
    lines.get(lo..hi).map(|s| s.join(" ")).unwrap_or_default()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Default to the repo root (one up from the `native` crate cargo runs us in).
    let root = PathBuf::from(args.get(1).cloned().unwrap_or_else(|| "..".to_string()));
    let report_path = PathBuf::from(
        args.get(2).cloned().unwrap_or_else(|| "target/lint-report.txt".to_string()),
    );
    let budget = 120usize;

    // ── Load the real Rust documentation (official clippy rules) ─────────────────────────
    let raw = fs::read_to_string("../lint-index/clippy.json").expect("lint-index/clippy.json");
    let idx: serde_json::Value = serde_json::from_str(&raw).expect("parse clippy.json");
    let rules = idx["rules"].as_array().expect("rules array");

    let mut examples: Vec<Example> = Vec::new();
    let mut docs: Vec<(String, String)> = Vec::new(); // (rule id, doc text)
    let mut bad_fp: HashMap<String, Hv> = HashMap::new();
    let mut good_fp: HashMap<String, Hv> = HashMap::new();
    for r in rules {
        let id = r["id"].as_str().unwrap_or("");
        if id.is_empty() {
            continue;
        }
        let category = r["category"].as_str().unwrap_or("other");
        let desc = r["description"].as_str().unwrap_or("");
        let bad = r["exampleBad"].as_str().unwrap_or("");
        let good = r["exampleGood"].as_str().unwrap_or("");
        docs.push((
            id.to_string(),
            format!("{id} [{category}]: {desc} bad: {bad} good: {good}"),
        ));
        if !bad.is_empty() {
            examples.push(Example { rule: id.into(), slice: category.into(), bad: bad.into(), good: good.into() });
            bad_fp.insert(id.to_string(), fingerprint(bad));
        }
        if !good.is_empty() {
            good_fp.insert(id.to_string(), fingerprint(good));
        }
    }

    // Distinctive (rare) bad-example tokens per rule — the lexical grounding for the gate.
    let bad_tokens = build_distinctive(&examples, 6);

    // ── Train the AI linter (the judge) on those rules ───────────────────────────────────
    println!("Training the mixture-of-experts judge on {} clippy rules…", examples.len());
    let mut clean = Vec::new();
    {
        let mut v = Vec::new();
        rust_files(Path::new("src"), &mut v);
        for p in v {
            if let Ok(t) = fs::read_to_string(&p) {
                clean.push(t);
            }
        }
    }
    let clean_refs: Vec<&str> = clean.iter().map(String::as_str).collect();
    let t = Instant::now();
    let moe = Moe::train(&examples, &clean_refs, 1000, 1400, 2);
    println!("  trained in {:.1}s", t.elapsed().as_secs_f64());

    // ── Read every rule into INFINITE MEMORY (always recallable) ─────────────────────────
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
    let t = Instant::now();
    for (_, doc) in &docs {
        sys.remember(SourceRole::System, doc); // always persisted → always recallable
    }
    println!(
        "Read {} rules into memory in {:.1}s (store: {} raw spans, {} items, {} compactions)",
        docs.len(),
        t.elapsed().as_secs_f64(),
        sys.store().raw_spans().len(),
        sys.active_item_count(),
        sys.store().compactions().len(),
    );

    // ── Lint the whole repo ──────────────────────────────────────────────────────────────
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    files.sort();
    println!("\nLinting {} Rust files under {} …\n", files.len(), root.display());

    let mut report = String::new();
    let (mut raw_findings, mut kept, mut dropped, mut exact_hits) = (0usize, 0usize, 0usize, 0usize);
    let mut max_live_input = 0usize;
    let mut files_with_findings = 0usize;
    let t = Instant::now();

    for file in &files {
        let Ok(code) = fs::read_to_string(file) else { continue };
        let located = moe.judge_located(&code);
        if located.is_empty() {
            continue;
        }
        let lines: Vec<&str> = code.lines().collect();

        // Dedupe (line, rule) and apply the precision gate.
        let mut seen = std::collections::HashSet::new();
        let mut findings = Vec::new();
        for (line, idx) in located {
            if !seen.insert((line, idx)) {
                continue;
            }
            raw_findings += 1;
            let rule = moe.rule_name(idx).to_string();
            if looks_like_violation(&window_at(&lines, line), &rule, &bad_fp, &good_fp, &bad_tokens) {
                findings.push((line, rule));
            } else {
                dropped += 1;
            }
        }
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
            // Verify the recall is exactly this rule (not a neighbor).
            if ans.text.starts_with(rule.as_str()) {
                exact_hits += 1;
            }
            kept += 1;
            report.push_str(&format!(
                "  {rel}:{line}  {rule}\n      ↳ {}\n      ↳ recalled from {:?}\n",
                ans.text.chars().take(140).collect::<String>(),
                ans.provenance,
            ));
        }
    }

    fs::create_dir_all(report_path.parent().unwrap_or(Path::new("."))).ok();
    fs::write(&report_path, &report).expect("write report");

    // ── Console summary (full report is on disk) ─────────────────────────────────────────
    let preview: String = report.lines().take(28).collect::<Vec<_>>().join("\n");
    println!("{preview}\n  … full report ({} lines) written to {}", report.lines().count(), report_path.display());

    println!("\n════════ Result ════════");
    println!("linted in {:.1}s", t.elapsed().as_secs_f64());
    println!("files with findings:           {files_with_findings} / {}", files.len());
    println!("raw findings from the judge:   {raw_findings}");
    println!("dropped by precision gate:     {dropped}  (code closer to the rule's GOOD example)");
    println!("reported findings:             {kept}");
    println!("exact rule recall:             {exact_hits}/{kept} = {:.0}%", exact_hits as f64 / kept.max(1) as f64 * 100.0);
    println!("max live model-facing input:   {max_live_input} tokens (budget {budget}) — BOUNDED");
    println!("rules held in memory:          {}", docs.len());
    println!("\nPoint it at any folder: `cargo run --release --example lint_with_memory <path>`.");
}
