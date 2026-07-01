//! `memory` — an unbounded speaking & memory architecture built *around* a fixed-capability
//! language model, so neither conversation length, output length, nor accumulated knowledge
//! ever forces the model to forget, truncate, or degrade.
//!
//! It is not a model. It is the layer that lets a model behave as though its ability to
//! **remember** and to **speak** is without practical limit, at roughly constant cost per
//! step. The single load-bearing invariant is that the live model input is always bounded
//! ([`working_set`]); everything else grows outward from it:
//!
//! * [`store`] — immutable raw history + versioned, never-overwritten knowledge.
//! * [`embed`] — training-free hypervector fingerprints (hashing, not learned weights).
//! * [`gpu`] — the popcount similarity kernel that makes retrieval fast without training.
//! * [`concept_index`] / [`retriever`] — concept-scoped, capped, ranked recall with provenance.
//! * [`compactor`] / [`recall_gate`] — lossy summaries that provably never drop a concrete fact.
//! * [`output_streamer`] — arbitrarily long output with bounded per-segment input.
//! * [`audit`] — a plain-language, append-only record of every decision and state change.
//! * [`controller`] — a small finite-state decision layer and the [`MemorySystem`] orchestrator.
//!
//! The model sits behind one replaceable seam ([`model::LanguageModel`]); the whole test
//! suite runs against a deterministic [`model::MockModel`], because the architecture's
//! guarantees must hold independently of any real model's quality.

pub mod audit;
pub mod compactor;
pub mod concept_index;
pub mod controller;
pub mod embed;
pub mod gpu;
pub mod model;
pub mod output_streamer;
pub mod recall_gate;
pub mod retriever;
pub mod store;
pub mod types;
pub mod util;
pub mod working_set;

pub use controller::{Answer, IngestReport, MemoryConfig, MemorySystem};
pub use model::{LanguageModel, MockModel, Prompt};
pub use retriever::RetrieverConfig;
pub use types::*;
