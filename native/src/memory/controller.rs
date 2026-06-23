//! `memory::controller` — the finite, inspectable decision layer, plus [`MemorySystem`], the
//! orchestrator that wires every component into one usable API.
//!
//! The controller holds no long-term knowledge of its own — that lives in the store. Each
//! cycle it picks exactly one action from the closed [`Action`] set, records a
//! plain-language [`ControllerDecision`] with the budget snapshot around it, and writes a
//! matching audit entry. Intelligence comes from the model plus retrieval; the controller
//! stays small on purpose.
//!
//! [`MemorySystem`] is what callers use: `ingest` statements (raw is stored immutably,
//! salient facts become recallable items, an over-full window is compacted), `ask` questions
//! (capped retrieval → bounded prompt → model), and `long_answer` (segmented output with
//! bounded input). Every one of these holds the invariants by construction.

use super::audit::AuditLog;
use super::compactor::{self, CompactionReport};
use super::concept_index::ConceptIndex;
use super::embed::extract_facts;
use super::model::{LanguageModel, MockModel};
use super::output_streamer::{OutputStreamer, SegmentEmission};
use super::retriever::{self, RetrieverConfig};
use super::store::MemoryStore;
use super::types::{
    Action, ControllerDecision, EventType, ItemStatus, RetrievalResult, SourceRole,
};
use super::working_set::WorkingSet;

/// Configuration for a [`MemorySystem`]. Defaults are demo-friendly; every field is a policy
/// knob, never a hidden constant buried in the code.
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Session identifier.
    pub session_id: String,
    /// Token budget for the live working set — the bounded-input guarantee.
    pub working_budget: usize,
    /// System preamble kept in every prompt.
    pub system_preamble: String,
    /// Retrieval ranking/caps.
    pub retriever: RetrieverConfig,
    /// Max tokens for a compaction summary.
    pub summary_tokens: usize,
    /// Max tokens for a long-output running summary.
    pub output_summary_tokens: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            session_id: "session".into(),
            working_budget: 80,
            system_preamble: "You are a careful assistant. Use only provided memory and context.".into(),
            retriever: RetrieverConfig::default(),
            summary_tokens: 24,
            output_summary_tokens: 16,
        }
    }
}

/// What an `ingest` did, for callers/tests that want to assert on it.
#[derive(Debug, Default)]
pub struct IngestReport {
    /// The immutable raw span id created.
    pub raw_id: String,
    /// The recallable memory item written from salient facts, if any.
    pub item_id: Option<String>,
    /// How many spans were evicted from the live window this ingest.
    pub evicted: usize,
    /// The compaction produced from evicted spans, if eviction happened.
    pub compaction: Option<CompactionReport>,
}

/// An answer plus everything needed to trust it: the capped retrieval set it was grounded
/// in, the provenance to cite, and the bounded prompt size used.
#[derive(Debug)]
pub struct Answer {
    /// The model's text.
    pub text: String,
    /// The capped, ranked memory the answer was grounded in.
    pub retrieval: Vec<RetrievalResult>,
    /// Flattened provenance (raw span ids) behind the retrieved memory.
    pub provenance: Vec<String>,
    /// Token size of the bounded prompt sent to the model.
    pub prompt_tokens: usize,
}

/// The orchestrator: owns the store, indexes, working set, audit log, and model seam, and
/// exposes the small set of operations a caller needs.
pub struct MemorySystem {
    cfg: MemoryConfig,
    store: MemoryStore,
    concepts: ConceptIndex,
    working: WorkingSet,
    audit: AuditLog,
    model: Box<dyn LanguageModel>,
    decisions: Vec<ControllerDecision>,
    cycle: u64,
}

impl MemorySystem {
    /// Build a system with the default deterministic [`MockModel`]. Real deployments call
    /// [`MemorySystem::with_model`] to swap in a live model behind the same trait.
    pub fn new(cfg: MemoryConfig) -> Self {
        Self::with_model(cfg, Box::new(MockModel))
    }

    /// Build a system with an explicit model implementation.
    pub fn with_model(cfg: MemoryConfig, model: Box<dyn LanguageModel>) -> Self {
        let working = WorkingSet::new(cfg.working_budget, &cfg.system_preamble);
        Self {
            store: MemoryStore::new(&cfg.session_id),
            concepts: ConceptIndex::new(),
            working,
            audit: AuditLog::new(),
            model,
            decisions: Vec::new(),
            cfg,
            cycle: 0,
        }
    }

    /// Pre-register a known concept (optional; the index also self-organizes during ingest).
    pub fn register_concept(&mut self, name: &str, aliases: &[&str], description: &str) -> String {
        self.concepts.register(name, aliases, description)
    }

    /// Record one controller cycle: append the decision and a matching audit entry. `budget`
    /// is the `(before, after)` working-set token snapshot so the bounded-input invariant is
    /// visible per cycle.
    fn decide(
        &mut self,
        action: Action,
        plan: impl Into<String>,
        reason: impl Into<String>,
        budget: (usize, usize),
        concept_ids: Vec<String>,
        provenance: Vec<String>,
    ) {
        let (before, after) = budget;
        let plan = plan.into();
        let reason = reason.into();
        self.decisions.push(ControllerDecision {
            cycle_id: self.cycle,
            action,
            plain_language_plan: plan.clone(),
            budget_before: before,
            budget_after: after,
            reason: reason.clone(),
        });
        self.audit.record(
            EventType::Decision,
            format!("cycle {}: {:?} — {}", self.cycle, action, plan),
            "controller",
            concept_ids,
            1.0,
            provenance,
        );
        self.cycle += 1;
    }

    /// Ingest a statement: store it immutably, file it under concepts, persist its salient
    /// facts as recallable memory, and compact the live window if ingest overflowed it.
    pub fn ingest(&mut self, role: SourceRole, text: &str) -> IngestReport {
        let concept_ids = self.concepts.assign_or_create(text, 4);
        let raw_id = self.store.add_raw(role, text, concept_ids.clone());
        self.audit.record(
            EventType::Ingest,
            format!("ingested {role:?} span: \"{}\"", truncate(text, 60)),
            "ingest",
            concept_ids.clone(),
            1.0,
            vec![raw_id.clone()],
        );
        let mut report = IngestReport {
            raw_id: raw_id.clone(),
            ..Default::default()
        };

        // Persist salient facts so they are recallable even before any eviction. Only spans
        // that carry concrete facts become items, to avoid polluting memory with chatter.
        if !extract_facts(text).is_empty() {
            let before = self.working.footprint();
            let outcome = self.store.write_memory(text, concept_ids.clone(), vec![raw_id.clone()], 0.95, 0.8);
            if !outcome.deduped {
                report.item_id = Some(outcome.id.clone());
                self.audit.record(
                    EventType::WriteMemory,
                    format!("wrote recallable fact: \"{}\"", truncate(text, 60)),
                    "controller",
                    concept_ids.clone(),
                    0.95,
                    vec![raw_id.clone()],
                );
            }
            for sup in &outcome.superseded {
                self.audit.record(
                    EventType::Contradiction,
                    format!("fact superseded prior item {sup} (versioned, not overwritten)"),
                    "store",
                    concept_ids.clone(),
                    1.0,
                    vec![sup.clone(), outcome.id.clone()],
                );
            }
            self.decide(
                Action::WriteMemory,
                "persist salient fact from the statement",
                "statement contains concrete facts worth recalling",
                (before, self.working.footprint()),
                concept_ids.clone(),
                vec![raw_id.clone()],
            );
        }

        // Push into the live window; compact anything evicted.
        let raw_span = self.store.get_raw(&raw_id).cloned();
        if let Some(span) = raw_span {
            let evicted = self.working.ingest(&span);
            report.evicted = evicted.len();
            if !evicted.is_empty() {
                let before = self.working.footprint();
                let spans: Vec<_> = evicted
                    .iter()
                    .filter_map(|e| self.store.get_raw(&e.id).cloned())
                    .collect();
                let compaction = compactor::compact(
                    &spans,
                    &mut self.store,
                    &mut self.concepts,
                    self.model.as_ref(),
                    self.cfg.summary_tokens,
                );
                self.audit.record(
                    EventType::Compact,
                    format!(
                        "compacted {} evicted span(s) → summary; recall gate {}; facts: [{}]",
                        spans.len(),
                        if compaction.passed { "passed" } else { "FAILED → kept per-fact items" },
                        compaction.facts.join(", ")
                    ),
                    "compactor",
                    compaction.concept_ids.clone(),
                    if compaction.passed { 1.0 } else { 0.5 },
                    compaction.source_span_ids.clone(),
                );
                for e in &evicted {
                    self.audit.record(
                        EventType::Evict,
                        format!("evicted span {} from live window (compacted, raw preserved)", e.id),
                        "working_set",
                        vec![],
                        1.0,
                        vec![e.id.clone()],
                    );
                }
                self.decide(
                    Action::Compact,
                    "compact evicted spans into concept-linked memory",
                    "live window exceeded its budget share",
                    (before, self.working.footprint()),
                    compaction.concept_ids.clone(),
                    compaction.source_span_ids.clone(),
                );
                report.compaction = Some(compaction);
            }
        }
        report
    }

    /// Answer a question: capped retrieval into the bounded working set, then one model call
    /// on a budget-checked prompt. The returned [`Answer`] carries the retrieval set and
    /// provenance so the answer can be cited.
    pub fn ask(&mut self, question: &str) -> Answer {
        // Cycle 1 — Retrieve (capped, ranked).
        let before = self.working.footprint();
        let hits = retriever::retrieve(question, &self.store, &self.concepts, &self.cfg.retriever);
        let mut lines = Vec::new();
        let mut provenance = Vec::new();
        for h in &hits {
            self.store.touch(&h.memory_item_id);
            if let Some(item) = self.store.get_item(&h.memory_item_id) {
                lines.push(format!("{} (prov: {})", item.text, item.source_span_ids.join(",")));
            }
            for p in &h.provenance {
                if !provenance.contains(p) {
                    provenance.push(p.clone());
                }
            }
        }
        self.working.load_retrieved(lines);
        self.audit.record(
            EventType::Retrieve,
            format!(
                "retrieved {} item(s) for \"{}\" (capped at {})",
                hits.len(),
                truncate(question, 50),
                self.cfg.retriever.cap
            ),
            "retriever",
            self.concepts.assign(question),
            1.0,
            provenance.clone(),
        );
        self.decide(
            Action::Retrieve,
            "pull the most relevant memory into the working set",
            "a question needs grounding from long-term memory",
            (before, self.working.footprint()),
            self.concepts.assign(question),
            provenance.clone(),
        );

        // Cycle 2 — Answer from the bounded prompt.
        let prompt = self.working.assemble(question);
        let prompt_tokens = prompt.token_count();
        let before = prompt_tokens;
        let text = self.model.complete(&prompt);
        self.working.clear_retrieved();
        self.decide(
            Action::Answer,
            "answer from the bounded working set",
            "retrieval assembled; prompt within budget",
            (before, self.working.footprint()),
            self.concepts.assign(question),
            provenance.clone(),
        );

        Answer {
            text,
            retrieval: hits,
            provenance,
            prompt_tokens,
        }
    }

    /// Generate a long, segmented answer with bounded per-segment input. Each segment is a
    /// `ContinueOutput` cycle and is audited; the model never sees the accumulated output.
    pub fn long_answer(
        &mut self,
        request: &str,
        outline: Vec<String>,
        style: Vec<String>,
    ) -> Vec<SegmentEmission> {
        let mut streamer = OutputStreamer::start(
            "run-0",
            request,
            outline,
            style,
            self.cfg.output_summary_tokens,
        );
        let mut segments = Vec::new();
        while let Some(seg) = streamer.next_segment(self.model.as_ref()) {
            self.audit.record(
                EventType::OutputContinuation,
                format!(
                    "emitted segment {} (\"{}\") with bounded input of {} tokens",
                    seg.index, seg.section, seg.input_tokens
                ),
                "output_streamer",
                vec![],
                1.0,
                vec![],
            );
            self.decide(
                Action::ContinueOutput,
                format!("emit segment {} of the planned outline", seg.index),
                "long output continues; input held bounded by the running summary",
                (seg.input_tokens, seg.input_tokens),
                vec![],
                vec![],
            );
            segments.push(seg);
        }
        segments
    }

    // --- read-only accessors for the demo and tests ---

    /// The append-only audit log.
    pub fn audit(&self) -> &AuditLog {
        &self.audit
    }
    /// The recorded controller decisions.
    pub fn decisions(&self) -> &[ControllerDecision] {
        &self.decisions
    }
    /// The long-term store.
    pub fn store(&self) -> &MemoryStore {
        &self.store
    }
    /// The current live working-set footprint (tokens), excluding the per-call instruction.
    pub fn working_footprint(&self) -> usize {
        self.working.footprint()
    }
    /// The configured working-set budget.
    pub fn budget(&self) -> usize {
        self.cfg.working_budget
    }
    /// Count of active (current, trusted) memory items.
    pub fn active_item_count(&self) -> usize {
        self.store.items().filter(|i| i.status == ItemStatus::Active).count()
    }
}

/// Truncate text for compact audit messages, adding an ellipsis when cut.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}
