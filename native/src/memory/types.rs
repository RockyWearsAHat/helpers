//! `memory::types` — the typed data model for the unbounded memory architecture.
//!
//! Every structure here is a plain, serializable record. The architecture's guarantees
//! (bounded live input, immutable raw history, capped retrieval, auditable decisions)
//! are enforced by the components that operate on these records — the records themselves
//! are deliberately dumb data so the whole system state can be inspected and serialized.
//!
//! Identifiers are simple monotonic strings (`raw-0`, `item-3`, …) minted by the store.
//! They are stable for the lifetime of a run, which is all the provenance pointers need.

use serde::{Deserialize, Serialize};

/// Who produced a span of raw history. Kept explicit so provenance never has to guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    /// Text supplied by the human user.
    User,
    /// Text produced by the model (an answer or a generated segment).
    Assistant,
    /// Text injected by the system (instructions, tool results).
    System,
}

/// An original, immutable span of conversation history. Once written it is never deleted
/// or mutated — compactions summarize spans but always keep [`RawSpan::id`] pointers back
/// here, so any fact can be rehydrated from the untouched source. This immutability is
/// what turns "no relevant fact is lost" into a structural guarantee.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawSpan {
    /// Stable identifier, e.g. `raw-7`.
    pub id: String,
    /// The session this span belongs to.
    pub session_id: String,
    /// Who said it.
    pub source_role: SourceRole,
    /// The verbatim text. Never altered after creation.
    pub text: String,
    /// ISO-8601 creation time.
    pub created_at: String,
    /// Concepts this span was indexed under.
    pub concept_ids: Vec<String>,
}

/// The lifecycle status of a [`MemoryItem`]. Contradictions and staleness are expressed
/// by transitioning status and down-ranking — never by silent deletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    /// Current, trusted knowledge.
    Active,
    /// Old but not wrong; down-ranked during retrieval.
    Stale,
    /// Directly contradicted by a newer item; retained for audit, heavily down-ranked.
    Contradicted,
    /// Replaced by a newer version of the same fact.
    Superseded,
}

/// A unit of recallable knowledge distilled from one or more raw spans. Items carry
/// pointers back to their source spans so retrieval can always cite provenance and, if a
/// summary proves insufficient, the raw text can be rehydrated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryItem {
    /// Stable identifier, e.g. `item-3`.
    pub id: String,
    /// Concepts this item is filed under.
    pub concept_ids: Vec<String>,
    /// The recallable text (a fact or a compacted summary line).
    pub text: String,
    /// The raw spans this knowledge was derived from (provenance).
    pub source_span_ids: Vec<String>,
    /// Confidence in correctness, 0..=1.
    pub confidence: f32,
    /// Importance for ranking, 0..=1.
    pub importance: f32,
    /// ISO-8601 creation time.
    pub created_at: String,
    /// ISO-8601 time of last retrieval — feeds recency ranking.
    pub last_accessed_at: String,
    /// Lifecycle status.
    pub status: ItemStatus,
    /// Monotonic version; bumped when a fact is superseded.
    pub version: u32,
}

/// A topic/entity bucket that memory items and raw spans are filed under. The concept
/// index uses these to retrieve by relevance without dumping all vaguely related memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Concept {
    /// Stable identifier (the normalized name, e.g. `deadline`).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Alternate spellings/synonyms that map onto this concept.
    pub aliases: Vec<String>,
    /// Optional description.
    pub description: String,
}

/// A versioned, replaceable summary of a set of raw spans. Unlike raw spans, compactions
/// can be regenerated — but each one records the deterministic facts it preserved and the
/// recall-gate verdict, so a compaction that drops a required fact is rejected outright.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compaction {
    /// Stable identifier, e.g. `compaction-1`.
    pub id: String,
    /// The raw spans summarized (provenance pointers).
    pub source_span_ids: Vec<String>,
    /// The natural-language summary.
    pub summary_text: String,
    /// Concrete facts extracted and verified to survive (names, dates, numbers, …).
    pub extracted_facts: Vec<String>,
    /// The recall-gate verdict for this compaction.
    pub recall_gate_result: RecallGateResult,
    /// ISO-8601 creation time.
    pub created_at: String,
    /// Monotonic version.
    pub version: u32,
}

/// The verdict of the recall gate: did the compaction preserve every required fact?
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecallGateResult {
    /// True only if every required fact is recoverable from the compaction (or raw).
    pub passed: bool,
    /// Facts the gate confirmed are present.
    pub preserved_facts: Vec<String>,
    /// Facts the gate found missing (forces rejection / rehydrate-from-raw).
    pub missing_facts: Vec<String>,
}

/// One ranked retrieval hit, always paired with the provenance needed to cite it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalResult {
    /// The memory item retrieved.
    pub memory_item_id: String,
    /// Fused relevance score (higher is more relevant).
    pub relevance_score: f32,
    /// Plain-language reason this item was selected.
    pub reason_selected: String,
    /// Raw span ids backing this item — printed when the answer cites memory.
    pub provenance: Vec<String>,
}

/// The action the controller may take on a single cycle. The set is deliberately small
/// and closed so the controller stays an inspectable finite state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Answer the user from the current bounded working set.
    Answer,
    /// Pull relevant memory into the working set (capped).
    Retrieve,
    /// Compress old working-set material into concept-linked memory.
    Compact,
    /// Persist an extracted fact as a memory item.
    WriteMemory,
    /// Emit the next segment of a long answer.
    ContinueOutput,
    /// Ask the user to disambiguate.
    Clarify,
    /// Refuse on safety grounds.
    Refuse,
}

/// A single controller decision, recorded with the budget snapshot around it so the
/// bounded-input invariant is visible per cycle, not merely asserted in tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerDecision {
    /// Monotonic cycle counter.
    pub cycle_id: u64,
    /// The chosen action.
    pub action: Action,
    /// Plain-language plan for this cycle.
    pub plain_language_plan: String,
    /// Working-set token count before the action.
    pub budget_before: usize,
    /// Working-set token count after the action.
    pub budget_after: usize,
    /// Why this action was chosen.
    pub reason: String,
}

/// The state of a long, segmented output run. The model is never shown the whole prior
/// output — only the running summary, the outline, and the open promises — so arbitrarily
/// long answers stay on-thread with bounded input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputRun {
    /// Stable identifier.
    pub id: String,
    /// The request that started the run.
    pub user_request: String,
    /// The planned sections, in order.
    pub outline: Vec<String>,
    /// Segments already emitted (the full text, retained outside the model input).
    pub completed_segments: Vec<String>,
    /// A bounded summary of what has been said so far.
    pub running_summary: String,
    /// Requirements/promises not yet fulfilled.
    pub unresolved_items: Vec<String>,
    /// Style/register constraints to preserve across segments.
    pub style_constraints: Vec<String>,
}

/// The category of an audit event. Every state change in the system maps to one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// A raw span was ingested.
    Ingest,
    /// A memory item was written.
    WriteMemory,
    /// Old material was compacted.
    Compact,
    /// Memory was retrieved into the working set.
    Retrieve,
    /// A working-set item was evicted.
    Evict,
    /// A contradiction was versioned.
    Contradiction,
    /// A long-output segment was emitted.
    OutputContinuation,
    /// The controller chose an action.
    Decision,
}

/// One human-readable audit record with full provenance. The audit log is append-only;
/// reading it back must explain, in plain language, everything the system did and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Monotonic identifier.
    pub id: u64,
    /// What kind of event this is.
    pub event_type: EventType,
    /// Plain-language description a human can read without decoding internals.
    pub plain_language_description: String,
    /// The source/component that produced the event.
    pub source: String,
    /// Concepts touched by the event.
    pub concept_ids: Vec<String>,
    /// Confidence associated with the event, where meaningful.
    pub confidence: f32,
    /// ISO-8601 timestamp.
    pub timestamp: String,
    /// Provenance pointers (raw span ids / item ids) relevant to the event.
    pub provenance: Vec<String>,
}
