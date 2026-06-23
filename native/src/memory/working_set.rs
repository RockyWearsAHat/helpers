//! `memory::working_set` — the bounded live context. This is the load-bearing invariant of
//! the whole architecture: whatever the model sees on any cycle is held under a fixed token
//! budget, and stays under it no matter how long the session runs or how much history is
//! stored. Everything else grows outward from this guarantee.
//!
//! The working set holds three things: a small system preamble, a sliding window of the most
//! recent raw spans (verbatim), and the currently retrieved memory lines. When ingesting a
//! span would push the recent window past its share of the budget, the oldest spans are
//! *evicted and returned* so the controller can compact them into long-term memory — they
//! leave the live context but are never lost (raw is immutable). [`WorkingSet::assemble`] is
//! the only place a model-facing [`Prompt`] is built, and it enforces the budget before
//! handing anything to a model.

use super::model::{count_tokens, Prompt};
use super::types::RawSpan;

/// One span living in the bounded recent window, with its token cost cached.
#[derive(Debug, Clone)]
struct LiveSpan {
    id: String,
    text: String,
    tokens: usize,
}

/// The bounded live context. Construct with a token `budget`; nothing assembled here ever
/// exceeds it.
pub struct WorkingSet {
    budget: usize,
    system: String,
    recent: Vec<LiveSpan>,
    retrieved: Vec<String>,
}

impl WorkingSet {
    /// A working set with the given total token `budget` and a fixed system preamble.
    pub fn new(budget: usize, system: &str) -> Self {
        Self {
            budget,
            system: system.to_string(),
            recent: Vec::new(),
            retrieved: Vec::new(),
        }
    }

    /// The configured total token budget.
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// The share of the budget reserved for the verbatim recent window (the rest is left
    /// for the system preamble, retrieved memory, and the instruction). Keeping the recent
    /// window under this share is what makes [`WorkingSet::assemble`] rarely need to trim.
    fn recent_budget(&self) -> usize {
        self.budget * 6 / 10
    }

    /// Current token cost of the recent window.
    fn recent_tokens(&self) -> usize {
        self.recent.iter().map(|s| s.tokens).sum()
    }

    /// Total live footprint (system + recent + retrieved), excluding the per-call instruction.
    pub fn footprint(&self) -> usize {
        count_tokens(&self.system)
            + self.recent_tokens()
            + self.retrieved.iter().map(|r| count_tokens(r)).sum::<usize>()
    }

    /// Ingest a raw span into the recent window and return any spans evicted to keep the
    /// window under its budget share. Evicted spans are handed back (oldest first) so the
    /// caller can compact them; they are gone from the live context but not from the store.
    pub fn ingest(&mut self, span: &RawSpan) -> Vec<EvictedSpan> {
        self.recent.push(LiveSpan {
            id: span.id.clone(),
            text: span.text.clone(),
            tokens: count_tokens(&span.text),
        });
        let mut evicted = Vec::new();
        // Always keep at least the most recent span; evict from the front (oldest) otherwise.
        while self.recent_tokens() > self.recent_budget() && self.recent.len() > 1 {
            let old = self.recent.remove(0);
            evicted.push(EvictedSpan { id: old.id, text: old.text });
        }
        evicted
    }

    /// Replace the retrieved-memory block (the retriever has already capped it).
    pub fn load_retrieved(&mut self, lines: Vec<String>) {
        self.retrieved = lines;
    }

    /// Clear the retrieved block (e.g., after answering).
    pub fn clear_retrieved(&mut self) {
        self.retrieved.clear();
    }

    /// Assemble the bounded model-facing prompt for `instruction`, enforcing the budget.
    ///
    /// The returned [`Prompt`] is **guaranteed** to satisfy `token_count() <= budget`. If the
    /// raw assembly would exceed the budget, retrieved lines are dropped first (they are
    /// transient and re-derivable), then, only as a last resort, the oldest recent spans —
    /// and finally the instruction itself is truncated. In normal operation `ingest` keeps
    /// the recent window small enough that only retrieved trimming is ever needed.
    pub fn assemble(&self, instruction: &str) -> Prompt {
        let mut retrieved = self.retrieved.clone();
        let mut recent: Vec<String> = self.recent.iter().map(|s| s.text.clone()).collect();
        let mut instruction = instruction.to_string();

        let build = |retrieved: &[String], recent: &[String], instruction: &str| Prompt {
            system: self.system.clone(),
            retrieved: retrieved.to_vec(),
            recent: recent.to_vec(),
            running_summary: String::new(),
            instruction: instruction.to_string(),
        };

        // 1) Drop the oldest recent spans first: retrieved memory is the deliberately
        //    chosen grounding for this call and outranks incidental recent chatter. Keep at
        //    least the newest recent span for local continuity.
        while build(&retrieved, &recent, &instruction).token_count() > self.budget
            && recent.len() > 1
        {
            recent.remove(0);
        }
        // 2) Only if still over budget, drop retrieved lines from the least-relevant end.
        while build(&retrieved, &recent, &instruction).token_count() > self.budget
            && !retrieved.is_empty()
        {
            retrieved.pop();
        }
        // 3) Final guarantee: truncate the instruction itself to fit the budget.
        let fixed = count_tokens(&self.system)
            + recent.iter().map(|r| count_tokens(r)).sum::<usize>();
        if fixed + count_tokens(&instruction) > self.budget {
            let allow = self.budget.saturating_sub(fixed);
            let words: Vec<&str> = instruction.split_whitespace().take(allow).collect();
            instruction = words.join(" ");
        }

        let prompt = build(&retrieved, &recent, &instruction);
        debug_assert!(
            prompt.token_count() <= self.budget,
            "working-set invariant violated: {} > {}",
            prompt.token_count(),
            self.budget
        );
        prompt
    }
}

/// A span evicted from the live window, handed to the controller for compaction.
#[derive(Debug, Clone)]
pub struct EvictedSpan {
    /// The raw span id (its text is also carried so no extra store lookup is needed).
    pub id: String,
    /// The verbatim text of the evicted span.
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::SourceRole;
    use crate::memory::util::now_iso;

    fn span(id: &str, text: &str) -> RawSpan {
        RawSpan {
            id: id.into(),
            session_id: "sess".into(),
            source_role: SourceRole::User,
            text: text.into(),
            created_at: now_iso(),
            concept_ids: vec![],
        }
    }

    #[test]
    fn assembled_prompt_never_exceeds_budget() {
        let mut ws = WorkingSet::new(40, "system preamble here");
        // Ingest far more text than the budget; the window must stay bounded.
        for i in 0..200 {
            let s = span(&format!("raw-{i}"), "this is a reasonably wordy span of conversation text");
            ws.ingest(&s);
            let prompt = ws.assemble("answer the user question now please");
            assert!(
                prompt.token_count() <= ws.budget(),
                "cycle {i}: {} > {}",
                prompt.token_count(),
                ws.budget()
            );
        }
    }

    #[test]
    fn ingest_evicts_oldest_and_returns_them() {
        let mut ws = WorkingSet::new(30, "sys");
        let mut all_evicted = 0;
        for i in 0..50 {
            let s = span(&format!("raw-{i}"), "alpha beta gamma delta epsilon zeta eta theta");
            all_evicted += ws.ingest(&s).len();
        }
        assert!(all_evicted > 0, "a long session must evict old spans");
    }
}
