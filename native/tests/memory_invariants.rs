//! End-to-end proof of the eight architectural invariants for the unbounded memory system.
//!
//! Every test runs against the deterministic `MockModel` (no real model anywhere), because
//! these invariants are properties of the *architecture* — bounded input, immutable raw,
//! capped recall, contradiction versioning, long-output continuity, auditability — and must
//! hold regardless of how good any real model is.

use helpers_native::memory::types::{EventType, ItemStatus, SourceRole};
use helpers_native::memory::{MemoryConfig, MemorySystem, RetrieverConfig};

/// A system tuned small so eviction/compaction actually trigger within a short session.
fn small_system() -> MemorySystem {
    MemorySystem::new(MemoryConfig {
        working_budget: 80,
        summary_tokens: 20,
        output_summary_tokens: 14,
        retriever: RetrieverConfig { cap: 4, ..Default::default() },
        ..Default::default()
    })
}

/// Ingest a long run of chatter, returning the system. The early fact is planted first so
/// later tests can prove it is still recallable after it has been compacted out of the live
/// window.
fn long_session(extra_chatter: usize) -> MemorySystem {
    let mut sys = small_system();
    sys.ingest(SourceRole::User, "The launch deadline is 2026-08-01 for the Acme account.");
    for i in 0..extra_chatter {
        sys.ingest(
            SourceRole::User,
            &format!("Note number {i}: we discussed assorted unrelated logistics and weather."),
        );
    }
    sys
}

#[test]
fn invariant_1_and_2_bounded_and_flat_input() {
    // Bounded live input on every cycle, and flat per-step cost as stored history grows by
    // orders of magnitude: prompt size and retrieval count stay under their caps either way.
    for history in [20usize, 600] {
        let mut sys = long_session(history);
        let cfg_cap = 4;
        let answer = sys.ask("when is the launch deadline for Acme?");
        assert!(
            answer.prompt_tokens <= sys.budget(),
            "history={history}: prompt {} exceeded budget {}",
            answer.prompt_tokens,
            sys.budget()
        );
        assert!(
            answer.retrieval.len() <= cfg_cap,
            "history={history}: retrieval {} exceeded cap {cfg_cap}",
            answer.retrieval.len()
        );
        assert!(
            sys.working_footprint() <= sys.budget(),
            "history={history}: live footprint exceeded budget"
        );
    }
}

#[test]
fn invariant_3_raw_source_is_never_mutated_or_deleted() {
    let mut sys = small_system();
    let first = sys.ingest(SourceRole::User, "The launch deadline is 2026-08-01 for the Acme account.");
    let original = sys
        .store()
        .get_raw(&first.raw_id)
        .expect("first span must exist")
        .text
        .clone();
    // Drive enough chatter to force eviction + compaction.
    for i in 0..40 {
        sys.ingest(SourceRole::User, &format!("Filler message {i} with various words to fill the window."));
    }
    // The compaction(s) must have happened, yet the original raw span is byte-for-byte intact.
    assert!(!sys.store().compactions().is_empty(), "session should have compacted");
    let after = sys.store().get_raw(&first.raw_id).expect("raw must still exist").text.clone();
    assert_eq!(original, after, "raw span text must never change");
    // Raw span count equals everything ingested (nothing deleted): 1 + 40.
    assert_eq!(sys.store().raw_spans().len(), 41);
}

#[test]
fn invariant_4_facts_survive_compaction_and_are_recallable() {
    let mut sys = long_session(60); // pushes the early fact out of the live window
    let answer = sys.ask("what is the launch deadline date for Acme?");
    // Recalled from memory, and grounded with provenance back to raw.
    assert!(
        answer.text.contains("2026-08-01"),
        "the early date must be recalled after compaction; got: {}",
        answer.text
    );
    assert!(!answer.provenance.is_empty(), "recall must carry provenance");
    // Rehydration path: the cited raw span is still available verbatim.
    let raw_id = &answer.provenance[0];
    assert!(sys.store().get_raw(raw_id).is_some(), "provenance must point to a live raw span");
}

#[test]
fn invariant_5_retrieval_is_capped_and_precise() {
    let mut sys = small_system();
    sys.ingest(SourceRole::User, "The launch deadline is 2026-08-01 for the Acme account.");
    for noise in ["I like pizza on Fridays.", "The sky was grey today.", "We adopted a cat named Mochi.", "My commute took an hour."] {
        sys.ingest(SourceRole::User, noise);
    }
    let answer = sys.ask("when is the Acme deadline?");
    assert!(!answer.retrieval.is_empty(), "must recall something relevant");
    assert!(answer.retrieval.len() <= 4, "must respect the hard cap, not dump");
    // The top hit is the deadline, not the pizza/cat noise.
    let top = sys.store().get_item(&answer.retrieval[0].memory_item_id).unwrap();
    assert!(top.text.contains("2026-08-01"), "top hit must be the relevant fact, got {top:?}");
}

#[test]
fn invariant_6_contradictions_are_versioned_not_overwritten() {
    let mut sys = small_system();
    let a = sys.ingest(SourceRole::User, "deadline is 2026-08-01");
    let b = sys.ingest(SourceRole::User, "deadline is 2026-09-10");
    let a_item = a.item_id.expect("first fact stored");
    let b_item = b.item_id.expect("second fact stored");
    // Old item retained but superseded; new item active. Nothing was silently overwritten.
    assert_eq!(sys.store().get_item(&a_item).unwrap().status, ItemStatus::Superseded);
    assert_eq!(sys.store().get_item(&b_item).unwrap().status, ItemStatus::Active);
    assert!(sys.audit().count(EventType::Contradiction) >= 1, "the version event must be audited");
}

#[test]
fn invariant_7_long_output_is_continuous_with_bounded_input() {
    let mut sys = small_system();
    let outline: Vec<String> = (0..30).map(|i| format!("Section {i}: detail")).collect();
    let segments = sys.long_answer("Write a long structured report", outline.clone(), vec!["formal".into()]);
    assert_eq!(segments.len(), outline.len(), "every planned section is emitted");
    // Per-segment input never grows with output length.
    let max_input = segments.iter().map(|s| s.input_tokens).max().unwrap();
    assert!(max_input <= 14 + 8, "per-segment input must stay bounded, got {max_input}");
    // Segments follow the plan in order.
    for (i, seg) in segments.iter().enumerate() {
        assert_eq!(seg.index, i, "segments must be emitted in outline order");
    }
}

#[test]
fn invariant_8_every_action_is_audited() {
    let mut sys = long_session(40);
    sys.ask("when is the Acme deadline?");
    sys.long_answer("Summarize", vec!["Intro".into(), "Body".into()], vec![]);
    let audit = sys.audit();
    // Each kind of state change produced at least one readable audit entry.
    for ev in [
        EventType::Ingest,
        EventType::WriteMemory,
        EventType::Compact,
        EventType::Evict,
        EventType::Retrieve,
        EventType::OutputContinuation,
        EventType::Decision,
    ] {
        assert!(audit.count(ev) >= 1, "no audit entry for {ev:?}");
    }
    // The log renders as human-readable plain language.
    let rendered = audit.render();
    assert!(rendered.contains("retrieved"), "audit must read in plain language");
    assert!(rendered.lines().count() >= 10, "audit should narrate the whole run");
}
