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
    /// The returned [`Prompt`] is **unconditionally** guaranteed to satisfy
    /// `token_count() <= budget` — even if a single ingested span is itself larger than the
    /// whole budget (an input bigger than the window is the one genuine hardware-style limit;
    /// it is truncated, never allowed to blow the bound).
    ///
    /// It is a greedy fit by priority: the system preamble is always kept, then the
    /// instruction (truncated if need be), then the deliberately-retrieved memory (the
    /// grounding for this call), then the verbatim recent window newest-first, truncating the
    /// last admitted span so the total lands exactly within budget.
    pub fn assemble(&self, instruction: &str) -> Prompt {
        let sys_tokens = count_tokens(&self.system);

        // The instruction comes after the system preamble; never let it exceed the budget.
        let instr_budget = self.budget.saturating_sub(sys_tokens);
        let instruction = truncate_tokens(instruction, instr_budget);
        let mut remaining = instr_budget.saturating_sub(count_tokens(&instruction));

        // Retrieved grounding fills next, line by line, until the budget is spent. A line that
        // does not fit is truncated (not dropped) when it would otherwise leave the grounding
        // empty, so a recalled rule doc larger than the whole window still surfaces — bounded.
        let mut retrieved = Vec::new();
        for line in &self.retrieved {
            if remaining == 0 {
                break;
            }
            let t = count_tokens(line);
            if t <= remaining {
                retrieved.push(line.clone());
                remaining -= t;
            } else if retrieved.is_empty() {
                retrieved.push(truncate_tokens(line, remaining));
                remaining = 0;
            }
        }

        // The verbatim recent window fills whatever is left, newest-first, but is rendered
        // oldest-first for chronological order; the last admitted span is truncated to fit.
        let mut recent_rev: Vec<String> = Vec::new();
        for span in self.recent.iter().rev() {
            if remaining == 0 {
                break;
            }
            if span.tokens <= remaining {
                recent_rev.push(span.text.clone());
                remaining -= span.tokens;
            } else {
                recent_rev.push(truncate_tokens(&span.text, remaining));
                remaining = 0;
            }
        }
        recent_rev.reverse();

        let prompt = Prompt {
            system: self.system.clone(),
            retrieved,
            recent: recent_rev,
            running_summary: String::new(),
            instruction,
        };
        debug_assert!(
            prompt.token_count() <= self.budget,
            "working-set invariant violated: {} > {}",
            prompt.token_count(),
            self.budget
        );
        prompt
    }
}

/// Keep at most `max` whitespace tokens of `text`. The single primitive that lets
/// [`WorkingSet::assemble`] guarantee its bound even against an oversized atomic span.
fn truncate_tokens(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max {
        return text.trim().to_string();
    }
    words[..max].join(" ")
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
    fn a_single_span_larger_than_budget_is_still_bounded() {
        // The one genuine limit: an atomic input bigger than the whole window. It must be
        // truncated into the prompt, never allowed to exceed the budget.
        let mut ws = WorkingSet::new(20, "sys");
        let huge = "word ".repeat(500);
        ws.ingest(&span("raw-big", huge.trim()));
        let prompt = ws.assemble("question here");
        assert!(prompt.token_count() <= ws.budget(), "oversized span must not blow the bound");
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
