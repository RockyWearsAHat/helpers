//! `memory::output_streamer` — produces arbitrarily long output in planned segments while
//! keeping the model's input bounded the whole way.
//!
//! The trick mirrors the memory side: the model is never shown its entire prior output. For
//! each segment it sees only a bounded *running summary* of what has been said, the section
//! directive from the outline, and the style constraints to preserve. The full text of
//! every completed segment is retained outside the model input (in the [`OutputRun`]), so a
//! 100-section answer costs the same per step as a 2-section one.

use super::model::{count_tokens, LanguageModel};
use super::types::OutputRun;

/// One emitted segment plus the bounded size of the input that produced it — exposed so the
/// demo can show, per segment, that model input stayed flat.
#[derive(Debug, Clone)]
pub struct SegmentEmission {
    /// 0-based index of this segment in the outline.
    pub index: usize,
    /// The outline section this segment fulfills.
    pub section: String,
    /// The generated text.
    pub text: String,
    /// Tokens of model-facing input used to generate it (running summary + directive).
    pub input_tokens: usize,
}

/// Drives a long output run segment by segment with a bounded running summary.
pub struct OutputStreamer {
    run: OutputRun,
    /// Hard cap on the running-summary token count — the bound on per-segment input.
    max_summary_tokens: usize,
}

impl OutputStreamer {
    /// Start a run: the request, the planned `outline` sections, and `style` constraints to
    /// carry across every segment. `max_summary_tokens` bounds the running summary.
    pub fn start(
        id: &str,
        request: &str,
        outline: Vec<String>,
        style: Vec<String>,
        max_summary_tokens: usize,
    ) -> Self {
        Self {
            run: OutputRun {
                id: id.to_string(),
                user_request: request.to_string(),
                outline: outline.clone(),
                completed_segments: Vec::new(),
                running_summary: String::new(),
                unresolved_items: outline,
                style_constraints: style,
            },
            max_summary_tokens,
        }
    }

    /// True while outline sections remain to emit.
    pub fn has_next(&self) -> bool {
        self.run.completed_segments.len() < self.run.outline.len()
    }

    /// The current run state (full completed text lives here, outside model input).
    pub fn run(&self) -> &OutputRun {
        &self.run
    }

    /// Emit the next segment. The model sees only the bounded running summary and the
    /// section directive — never the accumulated output — so input stays flat. Returns
    /// `None` when the outline is exhausted.
    pub fn next_segment(&mut self, model: &dyn LanguageModel) -> Option<SegmentEmission> {
        if !self.has_next() {
            return None;
        }
        let index = self.run.completed_segments.len();
        let section = self.run.outline[index].clone();

        // The bounded model-facing input for this segment.
        let input_tokens = count_tokens(&self.run.running_summary) + count_tokens(&section);
        debug_assert!(
            count_tokens(&self.run.running_summary) <= self.max_summary_tokens,
            "running summary exceeded its bound before a segment call"
        );

        let text = model.continue_output(&self.run, &section);

        // Record the full segment outside the model input, then re-bound the running summary.
        self.run.completed_segments.push(text.clone());
        let combined = format!("{} {}", self.run.running_summary, section);
        self.run.running_summary = model.summarize(&combined, self.max_summary_tokens);
        // Mark this section's promise resolved.
        self.run.unresolved_items.retain(|u| u != &section);

        Some(SegmentEmission {
            index,
            section,
            text,
            input_tokens,
        })
    }

    /// The full assembled output (all segments joined) — produced without ever holding it
    /// all in a model prompt.
    pub fn full_output(&self) -> String {
        self.run.completed_segments.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::model::MockModel;

    #[test]
    fn long_output_stays_bounded_and_on_plan() {
        let outline: Vec<String> = (0..40).map(|i| format!("Section {i}")).collect();
        let mut streamer = OutputStreamer::start(
            "run-0",
            "write a long structured report",
            outline.clone(),
            vec!["formal register".into()],
            12,
        );
        let mut max_input = 0;
        let mut emitted = 0;
        while let Some(seg) = streamer.next_segment(&MockModel) {
            max_input = max_input.max(seg.input_tokens);
            emitted += 1;
        }
        assert_eq!(emitted, outline.len(), "every section must be emitted");
        // Input never grows with output length: bounded by summary cap + a short directive.
        assert!(max_input <= 12 + 8, "per-segment input must stay bounded, got {max_input}");
        assert!(streamer.run().unresolved_items.is_empty(), "all promises resolved");
    }
}
