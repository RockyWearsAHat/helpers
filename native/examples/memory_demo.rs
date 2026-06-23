//! End-to-end demo of the unbounded speaking & memory architecture.
//!
//! Run with: `cargo run --example memory_demo`
//!
//! It plays out the required scenario and prints, per cycle, the things that make the
//! invariants *visible* to a human watching it run — not merely asserted in tests:
//!   * the assembled model-facing input size (always under budget),
//!   * the retrieved items with their provenance,
//!   * the long-output segment numbers (bounded input throughout),
//!   * and the plain-language audit entries for each major action.

use helpers_native::memory::types::SourceRole;
use helpers_native::memory::{MemoryConfig, MemorySystem, RetrieverConfig};

fn rule(title: &str) {
    println!("\n──────── {title} ────────");
}

fn main() {
    // A deliberately tiny live budget so compaction/eviction happen within a short demo —
    // the architecture behaves identically at any scale, this just makes it observable.
    let mut sys = MemorySystem::new(MemoryConfig {
        working_budget: 60,
        summary_tokens: 18,
        output_summary_tokens: 12,
        retriever: RetrieverConfig { cap: 3, ..Default::default() },
        ..Default::default()
    });

    rule("1-2. A conversation longer than the live window (older turns compacted)");
    println!("live budget = {} tokens\n", sys.budget());

    // The fact we will ask about later goes in FIRST, then gets buried under chatter.
    let planted = sys.ingest(
        SourceRole::User,
        "The launch deadline is 2026-08-01 for the Acme account; ship version v0.3.8.",
    );
    println!(
        "ingest {:>6}: footprint={:>3}/{:<3}  (planted the deadline fact as {})",
        planted.raw_id,
        sys.working_footprint(),
        sys.budget(),
        planted.item_id.as_deref().unwrap_or("-"),
    );

    let chatter = [
        "I grabbed a coffee this morning before the standup.",
        "The weather has been unusually grey all week here.",
        "We adopted a small grey cat and named her Mochi.",
        "My commute took almost an hour because of roadwork.",
        "Someone brought donuts to the office on Tuesday.",
        "I rewatched an old movie over the weekend, it held up.",
        "The plants on my desk finally need repotting.",
        "Lunch options near the office have gotten repetitive.",
    ];
    for (i, line) in chatter.iter().enumerate() {
        let r = sys.ingest(SourceRole::User, line);
        let note = match &r.compaction {
            Some(c) => format!(
                "EVICTED {} span(s) → compacted (gate {}); facts kept: [{}]",
                r.evicted,
                if c.passed { "PASS" } else { "FAIL" },
                c.facts.join(", ")
            ),
            None => "in live window".to_string(),
        };
        println!(
            "ingest {:>6}: footprint={:>3}/{:<3}  {}",
            r.raw_id,
            sys.working_footprint(),
            sys.budget(),
            note
        );
        let _ = i;
    }

    println!(
        "\nstored: {} raw spans (immutable), {} compaction(s), {} active memory items",
        sys.store().raw_spans().len(),
        sys.store().compactions().len(),
        sys.active_item_count(),
    );

    rule("3-5. A later question about the FIRST fact — recalled, not the chatter");
    let answer = sys.ask("what is the launch deadline date and version for the Acme account?");
    println!("assembled model-facing input = {} tokens (budget {})", answer.prompt_tokens, sys.budget());
    println!("retrieved {} item(s), capped at {}:", answer.retrieval.len(), 3);
    for hit in &answer.retrieval {
        let text = sys.store().get_item(&hit.memory_item_id).map(|i| i.text.clone()).unwrap_or_default();
        println!(
            "  • {} | score={:.3} | provenance={:?}\n      {}",
            hit.memory_item_id, hit.relevance_score, hit.provenance, text
        );
    }
    println!("\nanswer (cites provenance {:?}):\n  {}", answer.provenance, answer.text);
    // Prove the rehydration path is intact: the cited raw span is still verbatim.
    if let Some(p) = answer.provenance.first() {
        if let Some(raw) = sys.store().get_raw(p) {
            println!("rehydrate {p}: \"{}\"", raw.text);
        }
    }

    rule("6. A very long output generated across bounded segments");
    let outline: Vec<String> = (0..8).map(|i| format!("Part {i}")).collect();
    let segments = sys.long_answer(
        "Write a detailed multi-part launch plan",
        outline,
        vec!["concise, professional register".into()],
    );
    for seg in &segments {
        println!(
            "segment #{:<2} \"{}\"  model-input={} tokens (bounded by running summary)",
            seg.index, seg.section, seg.input_tokens
        );
    }
    println!(
        "→ produced {} segments; max per-segment model input was {} tokens — flat, never the whole prior output",
        segments.len(),
        segments.iter().map(|s| s.input_tokens).max().unwrap_or(0)
    );

    rule("7. Bounded throughout: every cycle's budget snapshot");
    for d in sys.decisions().iter().rev().take(8).rev() {
        println!(
            "cycle {:>2} {:?}: {} (budget {}→{})",
            d.cycle_id, d.action, d.plain_language_plan, d.budget_before, d.budget_after
        );
    }

    rule("8. The audit log — plain language, with provenance");
    println!("{}", sys.audit().render());
}
