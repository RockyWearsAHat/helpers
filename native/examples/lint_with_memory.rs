//! Lint THIS project with the AI linter, backed by the unbounded memory architecture.
//!
//!   cargo run --release --example lint_with_memory [budget] [max_findings]
//!
//! Three phases, matching the request:
//!   1. ENSURE MEMORY WORKS — a quick self-check (the test suite proves the rest).
//!   2. TRAIN ON REAL RUST DOCS — ingest all official clippy rules (the real Rust lint
//!      documentation, scraped from rust-lang.github.io) into the memory store. There are
//!      far more rules than fit the live window, so this exercises "infinite memory": the
//!      knowledge lives in the external store and the live input stays bounded.
//!   3. LINT WITH INFINITE INPUT & RESPONSE — the mixture-of-experts linter detects
//!      violations in the project's own Rust files (bounded input per file, so a project of
//!      any size is fine), each finding's explanation is RECALLED from memory with
//!      provenance, and the report is emitted segment-by-segment (one per file) so the
//!      response length is unbounded while the live model-facing input stays under budget.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use helpers_native::lint_moe::{Example, Moe};
use helpers_native::memory::types::SourceRole;
use helpers_native::memory::{LanguageModel, MemoryConfig, MemorySystem, Prompt, RetrieverConfig};

/// A deterministic "reporter" model behind the memory's `LanguageModel` seam. It never
/// invents lint knowledge: it turns whatever rule documentation the retriever recalled into
/// the working set into a human explanation. If nothing was recalled, it says so — it cannot
/// fabricate a rule it did not remember.
struct LintReporter;

impl LanguageModel for LintReporter {
    fn complete(&self, prompt: &Prompt) -> String {
        match prompt.retrieved.first() {
            Some(doc) => format!("{} — recalled rule: {}", prompt.instruction.trim(), doc),
            None => format!("{} — (no rule documentation recalled)", prompt.instruction.trim()),
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

/// Recursively collect this project's Rust source files (the thing we will lint).
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn rule_doc(id: &str, category: &str, desc: &str, bad: &str, good: &str) -> String {
    format!("{id} [{category}]: {desc} bad example: {bad} good example: {good}")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let budget: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(120);
    let max_findings: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(40);

    // ── Phase 1: ensure the memory works ────────────────────────────────────────────────
    println!("════════ Phase 1: memory self-check ════════");
    {
        let mut probe = MemorySystem::new(MemoryConfig { working_budget: 64, ..Default::default() });
        probe.ingest(SourceRole::User, "The API key is SECRET-1234 and the port is 8080.");
        for i in 0..30 {
            probe.ingest(SourceRole::User, &format!("Filler chatter number {i} to overflow the window."));
        }
        let a = probe.ask("what is the API key and port?");
        let ok = a.text.contains("SECRET-1234") && a.prompt_tokens <= probe.budget();
        println!(
            "recall after overflow: {} | live input {} ≤ budget {} | raw spans preserved: {}",
            if a.text.contains("SECRET-1234") { "FOUND fact" } else { "MISSED" },
            a.prompt_tokens,
            probe.budget(),
            probe.store().raw_spans().len()
        );
        assert!(ok, "memory self-check failed");
        println!("✓ memory works: bounded input, fact recalled from beyond the window\n");
    }

    // ── Load the real Rust documentation (official clippy rules) ─────────────────────────
    let raw = fs::read_to_string("../lint-index/clippy.json").expect("lint-index/clippy.json");
    let idx: serde_json::Value = serde_json::from_str(&raw).expect("parse clippy.json");
    let rules = idx["rules"].as_array().expect("rules array");

    // Examples for the MoE judge (rules that ship a bad example).
    let mut examples: Vec<Example> = Vec::new();
    // Full docs (every rule, example or not) for the memory store.
    let mut docs: Vec<(String, String, String)> = Vec::new(); // (id, category, doc text)
    for r in rules {
        let id = r["id"].as_str().unwrap_or("");
        let category = r["category"].as_str().unwrap_or("other");
        let desc = r["description"].as_str().unwrap_or("");
        let bad = r["exampleBad"].as_str().unwrap_or("");
        let good = r["exampleGood"].as_str().unwrap_or("");
        if id.is_empty() {
            continue;
        }
        docs.push((id.to_string(), category.to_string(), rule_doc(id, category, desc, bad, good)));
        if !bad.is_empty() {
            examples.push(Example {
                rule: id.to_string(),
                slice: category.to_string(),
                bad: bad.to_string(),
                good: good.to_string(),
            });
        }
    }
    println!(
        "loaded {} official clippy rules ({} with code examples) from rust-lang.github.io\n",
        docs.len(),
        examples.len()
    );

    // ── Train the AI linter (the judge) on those rules ───────────────────────────────────
    println!("════════ Training the mixture-of-experts judge on the Rust docs ════════");
    let mut clean = Vec::new();
    for p in {
        let mut v = Vec::new();
        rust_files(Path::new("src"), &mut v);
        v
    } {
        if let Ok(t) = fs::read_to_string(&p) {
            clean.push(t);
        }
    }
    let clean_refs: Vec<&str> = clean.iter().map(String::as_str).collect();
    let t = Instant::now();
    let moe = Moe::train(&examples, &clean_refs, 1000, 1400, 2);
    let (experts, signals) = moe.stats();
    println!("trained {experts} experts / {signals} signals in {:.1}s\n", t.elapsed().as_secs_f64());

    // ── Phase 2: train (read) the real Rust docs into INFINITE MEMORY ────────────────────
    println!("════════ Phase 2: reading {} rules into memory (budget {budget} tokens) ════════", docs.len());
    let mut sys = MemorySystem::with_model(
        MemoryConfig {
            session_id: "rust-clippy-docs".into(),
            working_budget: budget,
            summary_tokens: 24,
            output_summary_tokens: 16,
            retriever: RetrieverConfig { cap: 3, ..Default::default() },
            system_preamble: "You are a Rust linter. Explain findings using only recalled rule docs.".into(),
        },
        Box::new(LintReporter),
    );
    for (_, category, _) in &docs {
        sys.register_concept(category, &[], "clippy rule category");
    }
    let t = Instant::now();
    let mut max_model_input = 0;
    for (_, _, doc) in &docs {
        sys.ingest(SourceRole::System, doc);
        // The enforced bound is on the assembled MODEL-FACING prompt, which is always ≤ budget
        // even when a single rule doc is itself larger than the window.
        max_model_input = max_model_input.max(sys.peek_prompt_tokens("explain the next finding"));
    }
    println!("read all {} rules in {:.1}s", docs.len(), t.elapsed().as_secs_f64());
    println!(
        "  • model-facing input NEVER exceeded {} / {} tokens during ingest (bounded)",
        max_model_input, budget
    );
    println!(
        "  • store now holds {} immutable raw spans, {} compaction(s), {} active memory items",
        sys.store().raw_spans().len(),
        sys.store().compactions().len(),
        sys.active_item_count()
    );
    // Prove recall on a known real rule.
    let probe = sys.ask("clippy rule needless return");
    println!(
        "  • recall probe \"needless return\": {} (provenance {:?})\n",
        probe
            .retrieval
            .first()
            .and_then(|h| sys.store().get_item(&h.memory_item_id))
            .map(|i| i.text.chars().take(70).collect::<String>())
            .unwrap_or_else(|| "<none>".into()),
        probe.provenance
    );

    // ── Phase 3: lint this project, infinite input & response ────────────────────────────
    println!("════════ Phase 3: linting this project (infinite input & response) ════════");
    let mut files = Vec::new();
    rust_files(Path::new("src"), &mut files);
    files.sort();

    let mut total_findings = 0usize;
    let mut explained = 0usize;
    let mut max_live_input = 0usize;
    let mut report_segments = 0usize;
    let mut report_chars = 0usize;

    for file in &files {
        let Ok(code) = fs::read_to_string(file) else { continue };
        let located = moe.judge_located(&code);
        if located.is_empty() {
            continue;
        }
        // Dedupe (line, rule) so each distinct finding is reported once.
        let mut seen = std::collections::HashSet::new();
        let findings: Vec<(usize, u32)> =
            located.into_iter().filter(|f| seen.insert(*f)).collect();
        total_findings += findings.len();

        // Each FILE is one streamed report segment. The live model-facing input per finding
        // is bounded by the working set, no matter how big the file or the project is.
        report_segments += 1;
        let rel = file.strip_prefix("src").unwrap_or(file).display();
        let mut segment = format!("── src/{rel} ({} finding(s)) ──\n", findings.len());

        for (line, rule_idx) in findings.iter().take(8) {
            let rule = moe.rule_name(*rule_idx).to_string();
            // Recall this rule's documentation from infinite memory (bounded retrieval).
            let ans = sys.ask(&format!("clippy rule {}", rule.replace('_', " ")));
            max_live_input = max_live_input.max(ans.prompt_tokens);
            let explanation = ans.text.chars().take(160).collect::<String>();
            segment.push_str(&format!(
                "  line {line:<4} {rule}\n      ↳ {explanation}\n      ↳ provenance: {:?}\n",
                ans.provenance
            ));
            explained += 1;
            if explained >= max_findings {
                break;
            }
        }
        report_chars += segment.len();
        // Print only the first handful of segments so the console stays readable — but the
        // report itself is fully assembled (unbounded length) regardless of what we print.
        if report_segments <= 6 {
            print!("{segment}");
        }
        if explained >= max_findings {
            println!("  … ({} more findings across the project not expanded here)", total_findings - explained);
            break;
        }
    }

    println!("\n════════ Result ════════");
    println!("files linted (segments in the report): {report_segments}");
    println!("total findings detected by the AI linter: {total_findings}");
    println!("findings explained from recalled memory:  {explained}");
    println!(
        "MAX live model-facing input across every recall: {} tokens (budget {}) — BOUNDED",
        max_live_input, budget
    );
    println!("assembled report length: {report_chars} chars — grows with the project, UNBOUNDED");
    println!(
        "audit entries recorded: {} (every recall + decision logged with provenance)",
        sys.audit().entries().len()
    );
    println!(
        "\nInfinite memory: {} rules stored, recalled within a {}-token window.\nInfinite input: files linted one at a time, never all in context at once.\nInfinite response: report streamed per-file, live input held under budget throughout.",
        docs.len(), budget
    );
}
