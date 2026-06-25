//! `linter` — the documentation rule parser: the single shape lint knowledge is ingested in.
//!
//! Knowledge enters the AI linter from two sources — the official docs (crawled by
//! [`crate::lint_docs`]) and the CS-principles folder document — and both arrive here as a
//! [`Knowledge`] bag of [`LearnedRule`]s. [`crate::lint_train`] compiles those rules into the engine's
//! training examples; nothing else in the system cares which source a rule came from.
//!
//! [`Knowledge::from_text`] is how a plain text/markdown document (the CS2420/CS3500 principles, a
//! house style guide) becomes trainable rules with no code change: a heading starts a rule, its
//! fenced `bad`/`good` blocks are its examples.

use serde::{Deserialize, Serialize};

/// One documented rule learned from a doc or corpus: a language, an id, the bad/good examples, an
/// English description, and a severity. This is the atom every layer trains from.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LearnedRule {
    /// Language the examples are written in.
    pub language: String,
    /// Stable rule id.
    pub id: String,
    /// Severity bucket (`high`/`medium`/`low`); defaults to `medium`.
    pub severity: String,
    /// English description / the advice to show.
    pub description: String,
    /// Code the rule considers wrong.
    pub bad: String,
    /// The corrected form (may be empty).
    pub good: String,
}

/// A body of knowledge to learn from. Built from a crawled docs site or a text document; the rest
/// of the system never cares which — it only sees [`LearnedRule`]s.
#[derive(Clone, Debug, Default)]
pub struct Knowledge {
    /// Every rule-candidate this knowledge carries.
    pub rules: Vec<LearnedRule>,
    /// Real code the source served alongside the rules (every code block on every crawled doc
    /// page) — the "what's normal in this language" sample. Empty for a plain text document.
    pub reference: Vec<String>,
}

impl Knowledge {
    /// Learn from a plain **text / markdown document**. This is how a user hands the system their
    /// own rules — the curated CS2420/CS3500 principles, a house style guide — and it becomes
    /// trainable knowledge with no code changes. The grammar is deliberately simple:
    ///
    /// * A heading (`#`/`##`/…) starts a rule. Its text is the description; an `[high|medium|low]`
    ///   suffix sets severity; the id is the heading slugified.
    /// * Fenced code blocks under a heading are its examples. The info string's tag decides which:
    ///   `bad`/`wrong`/`avoid` ⇒ the bad example, `good`/`right`/`correct`/`fix` ⇒ the good one
    ///   (an untagged first block is treated as bad, a second as good). The fence's language word
    ///   (` ```rust `) sets the example language, else `default_lang`.
    pub fn from_text(default_lang: &str, doc: &str) -> Knowledge {
        let mut rules: Vec<LearnedRule> = Vec::new();
        let mut cur: Option<LearnedRule> = None;
        let mut in_fence = false;
        let mut fence_lang = String::new();
        let mut fence_tag = String::new();
        let mut fence_buf = String::new();

        // Commit a finished fenced block to the current rule's bad/good slot.
        fn place(rule: &mut LearnedRule, tag: &str, code: String) {
            let is_good = matches!(tag, "good" | "right" | "correct" | "fix" | "after");
            let is_bad = matches!(tag, "bad" | "wrong" | "avoid" | "dont" | "before");
            if is_good || (!is_bad && !rule.bad.is_empty() && rule.good.is_empty()) {
                rule.good = code;
            } else {
                rule.bad = code;
            }
        }

        for line in doc.lines() {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("```") {
                if in_fence {
                    // Closing fence: commit the block.
                    if let Some(r) = cur.as_mut() {
                        if !r.language.is_empty() && !fence_lang.is_empty() {
                            r.language = fence_lang.clone();
                        } else if r.language.is_empty() {
                            r.language = if fence_lang.is_empty() { default_lang.to_string() } else { fence_lang.clone() };
                        }
                        place(r, &fence_tag, fence_buf.trim_end().to_string());
                    }
                    in_fence = false;
                    fence_buf.clear();
                } else {
                    // Opening fence: parse `lang` and/or `:tag` (e.g. `rust:bad`, `bad`, `rust`).
                    in_fence = true;
                    let info = rest.trim();
                    let (l, t) = info.split_once(':').unwrap_or((info, ""));
                    fence_lang = l.trim().to_string();
                    fence_tag = if t.is_empty() { l.trim().to_string() } else { t.trim().to_string() };
                    // If the single word is itself a tag (untagged-language case), treat it so.
                    if t.is_empty() && !matches!(l.trim(), "bad" | "wrong" | "avoid" | "dont" | "before" | "good" | "right" | "correct" | "fix" | "after") {
                        fence_tag = String::new();
                        fence_lang = l.trim().to_string();
                    } else if t.is_empty() {
                        fence_lang = String::new();
                    }
                }
                continue;
            }
            if in_fence {
                fence_buf.push_str(line);
                fence_buf.push('\n');
                continue;
            }
            if let Some(h) = heading(trimmed) {
                if let Some(r) = cur.take() {
                    if !r.bad.is_empty() {
                        rules.push(r);
                    }
                }
                let (sev, title) = split_severity(h);
                cur = Some(LearnedRule {
                    language: String::new(),
                    id: slug(title),
                    severity: sev,
                    description: title.to_string(),
                    bad: String::new(),
                    good: String::new(),
                });
            } else if let Some(r) = cur.as_mut() {
                // Prose between the heading and the first fence extends the description.
                let t = line.trim();
                if !t.is_empty() && r.bad.is_empty() {
                    if !r.description.is_empty() {
                        r.description.push(' ');
                    }
                    r.description.push_str(t);
                }
            }
        }
        if let Some(r) = cur.take() {
            if !r.bad.is_empty() {
                rules.push(r);
            }
        }
        Knowledge { rules, reference: Vec::new() }
    }
}

/// A markdown ATX heading's text, or `None`.
fn heading(line: &str) -> Option<&str> {
    let h = line.trim_start_matches('#');
    if h.len() < line.len() && line.starts_with('#') {
        Some(h.trim())
    } else {
        None
    }
}

/// Split a trailing `[high|medium|low]` severity tag off a heading; default `medium`.
fn split_severity(title: &str) -> (String, &str) {
    let t = title.trim();
    if let Some(stripped) = t.strip_suffix(']') {
        if let Some(idx) = stripped.rfind('[') {
            let sev = stripped[idx + 1..].trim().to_lowercase();
            if matches!(sev.as_str(), "high" | "medium" | "low") {
                return (sev, stripped[..idx].trim());
            }
        }
    }
    ("medium".to_string(), t)
}

/// Slugify a heading into a stable id: lowercase, non-alphanumerics to `_`, collapsed.
fn slug(title: &str) -> String {
    let mut out = String::new();
    let mut last_us = false;
    for c in title.trim().chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            last_us = false;
        } else if !last_us && !out.is_empty() {
            out.push('_');
            last_us = true;
        }
    }
    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CS_DOC: &str = r#"
# Off by one indexing [high]
Indexing a collection with an inclusive range up to its length reads one element past the end.
```rust:bad
fn sum(xs: &[i32]) -> i32 { let mut t = 0; for i in 0..=xs.len() { t += xs[i]; } t }
```
```rust:good
fn sum(xs: &[i32]) -> i32 { let mut t = 0; for i in 0..xs.len() { t += xs[i]; } t }
```
"#;

    #[test]
    fn from_text_parses_rules_with_examples() {
        let k = Knowledge::from_text("rust", CS_DOC);
        assert_eq!(k.rules.len(), 1);
        let r = &k.rules[0];
        assert_eq!(r.id, "off_by_one_indexing");
        assert_eq!(r.severity, "high");
        assert_eq!(r.language, "rust");
        assert!(r.bad.contains("0..=xs.len()") && r.good.contains("0..xs.len()"));
    }

    #[test]
    fn narrative_only_section_yields_no_rule() {
        // A heading with no code block teaches nothing detectable (it has no bad example).
        let k = Knowledge::from_text("rust", "# Single responsibility\nA function should do one thing.\n");
        assert!(k.rules.is_empty());
    }
}
