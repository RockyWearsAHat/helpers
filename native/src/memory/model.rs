//! `memory::model` — the single replaceable seam between the architecture and any real
//! language model. Everything the system feeds a model goes through [`Prompt`], which is
//! assembled by the working set and budget-checked *before* it ever reaches [`LanguageModel`].
//!
//! The test suite runs entirely against [`MockModel`], a deterministic stand-in. The MVP
//! proves the *architecture* — bounded input, recall, capped retrieval, continuation,
//! audit — which must hold regardless of how good a real model's prose is. A real model
//! swaps in behind this same trait and changes none of the invariants.

use super::types::OutputRun;

/// Approximate token count: whitespace-delimited words. Deterministic and dependency-free
/// so the budget math is identical on every platform and in every test. A real deployment
/// can swap in a true tokenizer; the working set only needs a monotonic size estimate.
pub fn count_tokens(text: &str) -> usize {
    text.split_whitespace().count()
}

/// The bounded, assembled input handed to the model on a single call. The working set is
/// the only thing allowed to construct one for a real call, and it checks
/// [`Prompt::token_count`] against the budget first — so no model call can ever exceed the
/// live working-set budget, no matter how much total history is stored.
#[derive(Debug, Clone, Default)]
pub struct Prompt {
    /// Standing instructions / role.
    pub system: String,
    /// Retrieved memory lines, each already tagged with its provenance.
    pub retrieved: Vec<String>,
    /// The most recent raw spans kept verbatim in the live window.
    pub recent: Vec<String>,
    /// For long output: a bounded summary of what has already been produced.
    pub running_summary: String,
    /// The concrete instruction for this call (a question, or a segment directive).
    pub instruction: String,
}

impl Prompt {
    /// Total token cost of the assembled prompt — the number the working set bounds.
    pub fn token_count(&self) -> usize {
        let mut n = count_tokens(&self.system)
            + count_tokens(&self.running_summary)
            + count_tokens(&self.instruction);
        for r in &self.retrieved {
            n += count_tokens(r);
        }
        for r in &self.recent {
            n += count_tokens(r);
        }
        n
    }
}

/// The one interface a real model implements. Two narrow capabilities are all the
/// architecture needs: turn a bounded prompt into an answer, and compress text into a
/// shorter summary. Keeping the surface this small is what makes the seam truly
/// replaceable.
pub trait LanguageModel {
    /// Produce an answer for the assembled, already-budgeted prompt.
    fn complete(&self, prompt: &Prompt) -> String;

    /// Compress `text` into at most `max_tokens` tokens. Used by the compactor for the
    /// natural-language part of a summary (the concrete facts are extracted
    /// deterministically elsewhere, never trusted to the model).
    fn summarize(&self, text: &str, max_tokens: usize) -> String;

    /// Produce the next segment of a long output from the bounded run state plus the
    /// section directive. Default impl delegates to [`LanguageModel::complete`] so simple
    /// models need not special-case streaming.
    fn continue_output(&self, run: &OutputRun, section: &str) -> String {
        let prompt = Prompt {
            system: "Continue the answer; stay on plan and in style.".into(),
            retrieved: Vec::new(),
            recent: Vec::new(),
            running_summary: run.running_summary.clone(),
            instruction: section.into(),
        };
        self.complete(&prompt)
    }
}

/// A deterministic, model-free implementation used by the entire test suite and the demo.
///
/// It never calls out to anything. Its answers are mechanical but faithful to the
/// architecture's needs: when memory has been retrieved into the prompt, [`MockModel`]
/// surfaces that retrieved content (so "recall a fact from the beginning" is observable),
/// and its summaries are a deterministic head-truncation (so recall-gate and continuity
/// tests are reproducible).
#[derive(Debug, Default, Clone)]
pub struct MockModel;

impl LanguageModel for MockModel {
    fn complete(&self, prompt: &Prompt) -> String {
        // Faithfully reflect retrieved memory so the answer demonstrably uses recall.
        if !prompt.retrieved.is_empty() {
            let body = prompt.retrieved.join(" | ");
            return format!("{} [grounded in: {}]", prompt.instruction.trim(), body);
        }
        if !prompt.recent.is_empty() {
            return format!(
                "{} [from recent context]",
                prompt.instruction.trim()
            );
        }
        format!("{} [no memory available]", prompt.instruction.trim())
    }

    fn summarize(&self, text: &str, max_tokens: usize) -> String {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.len() <= max_tokens {
            return text.trim().to_string();
        }
        // Honor the bound exactly: the ellipsis occupies one of the `max_tokens` slots, so
        // the result never exceeds `max_tokens` whitespace tokens.
        let keep = max_tokens.saturating_sub(1);
        format!("{} …", words[..keep].join(" "))
    }

    fn continue_output(&self, run: &OutputRun, section: &str) -> String {
        // Deterministic, on-plan segment text that visibly carries the running summary
        // forward without ever consuming the full prior output.
        format!(
            "## {}\nContinuing \"{}\". So far: {}.",
            section,
            run.user_request.trim(),
            if run.running_summary.is_empty() {
                "(opening)".to_string()
            } else {
                run.running_summary.clone()
            }
        )
    }
}
