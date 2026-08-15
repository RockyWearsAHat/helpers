//! `linter` — document READING: the single shape lint knowledge is ingested in.
//!
//! Knowledge enters the AI linter from two places — the official docs (crawled by
//! [`crate::lint_docs`]) and local documents (the `extraDocs/` principles folder, the project's
//! `.helpers/lint-rules/` files, a root-level `lintPref.md`/`.txt`) — and both arrive here as a
//! [`Knowledge`] bag of [`LearnedRule`]s. [`crate::lint_train`] compiles those rules into the
//! engine's training examples; nothing else in the system cares which source a rule came from.
//!
//! [`Knowledge::read_document`] is how ANY plain text or markdown document becomes trainable
//! rules — **no required format**. The document is only *segmented* (paragraphs and fenced code
//! blocks — typography, not a rule grammar); everything semantic is READ by the learned
//! [`crate::lint_read::Polarity`] classifier:
//!
//!   * every prose span is a rule candidate — a heading names it when one exists, otherwise the
//!     span's own first words do;
//!   * whether an attached code example shows the *violation* or the *fix* is decided by
//!     classifying the words around it (its lead-in line, its fence's info words, the rule's own
//!     description) — prohibition ⇒ bad example, endorsement ⇒ good example — with document order
//!     (violation shown first, fix after) as the neutral fallback;
//!   * a span with no code at all is still a rule CANDIDATE: the engine derives the pattern from
//!     the English description downstream. Project law compiles by location (the rule file is the
//!     label); a LEARNED source's candidate must carry a forbidding sentence to compile at all
//!     ([`crate::lint_match`]) — concept and guidance prose is read, never fired.
//!
//! So `"Never call eval anywhere; parse the input explicitly."` in a bare `lintPref.txt` is a
//! complete, enforceable rule — headings, fences, and tags are optional enrichment the reader
//! understands when present, never a format it expects.

use serde::{Deserialize, Serialize};

use crate::lint_read::Polarity;

/// One documented rule learned from a doc or corpus: a language, an id, the bad/good examples, an
/// English description, and a severity. This is the atom every layer trains from.
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
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
    /// The code CONSTRUCT this rule's understanding forbids, when the rule carries its own proven
    /// plan (a graduated construct-module rule — `native/architecture.dx`, "The modular rebuild"). `Some(c)` means
    /// the rule IS the understood prohibition `uses_construct(c)`, compiled and fired DIRECTLY by
    /// `run_plan` in the one AST walk — never re-derived from the bad/good example diff. `None` is a
    /// legacy example/token-miner rule that keeps the example-diff detector path. Optional and
    /// defaulted so no other rule source changes.
    #[serde(default)]
    pub construct: Option<String>,
}

/// A body of knowledge to learn from. Built from a crawled docs site or a text document; the rest
/// of the system never cares which — it only sees [`LearnedRule`]s.
#[derive(Clone, Debug, Default)]
pub struct Knowledge {
    /// Every rule-candidate this knowledge carries.
    pub rules: Vec<LearnedRule>,
    /// Real code the source served alongside the rules (code blocks no rule claimed) — the
    /// "what's normal in this language" sample. Empty for a prose-only document.
    pub reference: Vec<String>,
}

/// One structural piece of a document: a prose paragraph or a fenced code block. Segmentation is
/// pure typography (blank lines, ``` fences) — it carries no rule semantics.
enum Seg {
    /// A paragraph of prose (consecutive non-blank lines outside fences).
    Prose(String),
    /// A fenced code block with its raw info string (the text after the opening ```).
    Code { info: String, body: String },
}

/// Split a document into ordered prose/code segments. An unclosed trailing fence still yields its
/// buffered code, so a truncated or broken document degrades instead of losing content.
fn segments(doc: &str) -> Vec<Seg> {
    let mut segs = Vec::new();
    let mut prose = String::new();
    let mut code = String::new();
    let mut info = String::new();
    let mut in_fence = false;
    let flush_prose = |buf: &mut String, segs: &mut Vec<Seg>| {
        if !buf.trim().is_empty() {
            segs.push(Seg::Prose(std::mem::take(buf)));
        } else {
            buf.clear();
        }
    };
    for line in doc.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("```") {
            if in_fence {
                segs.push(Seg::Code { info: std::mem::take(&mut info), body: code.trim_end().to_string() });
                code.clear();
                in_fence = false;
            } else {
                flush_prose(&mut prose, &mut segs);
                info = rest.trim().to_string();
                in_fence = true;
            }
            continue;
        }
        if in_fence {
            code.push_str(line);
            code.push('\n');
        } else if trimmed.is_empty() {
            flush_prose(&mut prose, &mut segs);
        } else {
            prose.push_str(line);
            prose.push('\n');
        }
    }
    if in_fence && !code.trim().is_empty() {
        segs.push(Seg::Code { info, body: code.trim_end().to_string() });
    } else {
        flush_prose(&mut prose, &mut segs);
    }
    segs
}

/// Strip markdown decoration from one prose line (list bullets, blockquote marks, emphasis
/// asterisks/underscores at the edges) so the reader sees words, not typography. A plain-text
/// line passes through unchanged.
fn clean_line(line: &str) -> String {
    let mut t = line.trim();
    for marker in ["- ", "* ", "+ ", "> "] {
        if let Some(rest) = t.strip_prefix(marker) {
            t = rest.trim_start();
            break;
        }
    }
    t.trim_matches(|c| c == '*' || c == '_').trim().to_string()
}

/// A paragraph's `(heading, body)` split: when its first line is a markdown ATX heading the
/// heading text names the rule and the remaining lines are its description; otherwise there is no
/// heading and the whole paragraph is body. Body lines are joined with `\n`, NOT a space, so each
/// source line stays a SENTENCE boundary ([`crate::lint_read::sentences`] breaks on `\n`): a canon
/// section's terminal-less markdown bullet ("Missing documentation on public APIs — write it")
/// surfaces as its OWN sentence for the understanding gate instead of welding into one blob with
/// its neighbours (native/architecture.dx, "REAL-CANON coverage" — the description assembler fix).
fn split_heading(paragraph: &str) -> (Option<String>, String) {
    let mut lines = paragraph.lines();
    let first = lines.next().unwrap_or("").trim();
    let stripped = first.trim_start_matches('#');
    let body_rest: Vec<String> = lines.map(clean_line).filter(|l| !l.is_empty()).collect();
    if stripped.len() < first.len() {
        (Some(stripped.trim_matches('#').trim().to_string()), body_rest.join("\n"))
    } else {
        let mut all = vec![clean_line(first)];
        all.extend(body_rest);
        (None, all.into_iter().filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n"))
    }
}

/// The building rule during a fold plus how it started — a heading-started rule absorbs the
/// paragraphs that follow it; a paragraph-started rule treats the next paragraph as a NEW rule, so
/// a bare list of English instructions yields one rule per instruction. Code blocks collect with
/// their most-local words (`(local words, body)`) and are oriented bad/good when the rule commits,
/// so a labeled pair can be compared against itself instead of each label read absolutely.
struct Building {
    rule: LearnedRule,
    from_heading: bool,
    /// True while the rule's description is ONLY its heading's own words: a heading that never
    /// gains body prose or a code block is a document TITLE, not a law (`# Rust law for this
    /// repo` once minted a trusted rule watching `rust` — native/architecture.dx ledger #14).
    heading_only: bool,
    blocks: Vec<(String, String)>,
}

impl Knowledge {
    /// READ a plain **text / markdown document** into rules. The document IS the training input —
    /// no structured format required (see the module doc for the full contract). `polarity` is the
    /// learned prohibition/endorsement classifier local documents are read through
    /// ([`crate::lint_docs::document_polarity`]); `None` degrades every semantic decision to the
    /// document-order fallback, never to an error.
    ///
    /// `default_lang` is the language rules belong to when the document names none (project files
    /// use their filename stem; `"any"` means every language). A fence info word that names the
    /// default language or a bundled grammar sets that rule's language; every other info word is
    /// read as prose and helps classify the block's polarity.
    pub fn read_document(default_lang: &str, doc: &str, polarity: Option<&Polarity>) -> Knowledge {
        let mut rules: Vec<LearnedRule> = Vec::new();
        let mut reference: Vec<String> = Vec::new();
        let mut cur: Option<Building> = None;
        let mut lead = String::new();

        // Prohibition (true) / endorsement (false) of a snippet of surrounding words — the
        // learned classifier's call, `None` when it abstains or no classifier is available.
        let read_polarity = |text: &str| -> Option<bool> {
            let t = text.trim();
            if t.is_empty() {
                return None;
            }
            polarity.and_then(|p| p.classify(t))
        };
        // Commit the building rule, orienting its collected code blocks. ONE block is read
        // absolutely (its local words → the description → violation by default). A PAIR keeps
        // document order (violation first, fix after — how rules are written) unless there is
        // POSITIVE evidence to swap: the trailing block's local words decisively classify
        // prohibition and the leading block's do not. Only decisive, margin-gated prohibition
        // may override the order convention, so classifier drift on label vocabulary degrades
        // to the convention — never to swapped examples (a drifted classifier once read the
        // fence word `bad` as endorsement and inverted a rule). Blocks past the pair are
        // reference knowledge.
        let commit = |cur: &mut Option<Building>, rules: &mut Vec<LearnedRule>, reference: &mut Vec<String>| {
            if let Some(b) = cur.take() {
                let heading_only = b.heading_only;
                let mut r = b.rule;
                let blocks = b.blocks;
                match blocks.len() {
                    0 => {}
                    1 => {
                        let (local, body) = &blocks[0];
                        let is_bad = read_polarity(local)
                            .or_else(|| read_polarity(&r.description))
                            .unwrap_or(true);
                        if is_bad {
                            r.bad = body.clone();
                        } else {
                            r.good = body.clone();
                        }
                    }
                    _ => {
                        let swapped = blocks.len() == 2
                            && read_polarity(&blocks[1].0) == Some(true)
                            && read_polarity(&blocks[0].0) != Some(true);
                        let (bad_i, good_i) = if swapped { (1, 0) } else { (0, 1) };
                        r.bad = blocks[bad_i].1.clone();
                        r.good = blocks[good_i].1.clone();
                        for (_, body) in blocks.iter().skip(2) {
                            reference.push(body.clone());
                        }
                    }
                }
                if r.language.is_empty() {
                    r.language = default_lang.to_string();
                }
                // A heading that never gained body prose or a code block is a TITLE, not a law.
                let title_only = heading_only && blocks.is_empty();
                if !title_only && (!r.bad.is_empty() || r.description.split_whitespace().count() >= 3) {
                    rules.push(r);
                }
            }
        };

        let segs = segments(doc);
        for (i, seg) in segs.iter().enumerate() {
            match seg {
                Seg::Prose(p) => {
                    let (heading, body) = split_heading(p);
                    if let Some(h) = heading {
                        commit(&mut cur, &mut rules, &mut reference);
                        let (severity, title) = split_severity(&h);
                        let body_was_empty = body.is_empty();
                        // A bare-id heading (`## no_eval [high]`) contributes nothing to the
                        // English advice — the prose IS the advice; a descriptive heading's words
                        // stay in it.
                        let description = if slug(title) == title.trim() && !body.is_empty() {
                            body
                        } else {
                            format!("{title} {body}").trim().to_string()
                        };
                        cur = Some(Building {
                            rule: LearnedRule {
                                language: String::new(),
                                id: slug(title),
                                severity,
                                description,
                                bad: String::new(),
                                good: String::new(),
                                construct: None,
                            },
                            from_heading: true,
                            heading_only: body_was_empty,
                            blocks: Vec::new(),
                        });
                        lead.clear();
                        continue;
                    }
                    let clean = body;
                    let next_is_code = matches!(segs.get(i + 1), Some(Seg::Code { .. }));
                    // A short line right before code ("Bad:", "Instead, write:") is the block's
                    // lead-in — words that orient the block, not a rule of their own.
                    if next_is_code && clean.split_whitespace().count() <= 6 && cur.is_some() {
                        lead = clean;
                        continue;
                    }
                    match cur.as_mut() {
                        // A heading-started rule absorbs its body paragraphs until code arrives.
                        Some(b) if b.from_heading && b.blocks.is_empty() => {
                            b.heading_only = false;
                            if b.rule.description == b.rule.id {
                                // The heading was the bare rule id — this prose IS the advice.
                                b.rule.description = clean;
                            } else {
                                if !b.rule.description.is_empty() {
                                    b.rule.description.push('\n');
                                }
                                b.rule.description.push_str(&clean);
                            }
                        }
                        // A plain paragraph after a finished/paragraph rule is the NEXT rule.
                        _ => {
                            commit(&mut cur, &mut rules, &mut reference);
                            let (severity, text) = split_severity(&clean);
                            let id = slug(&text.split_whitespace().take(8).collect::<Vec<_>>().join(" "));
                            cur = Some(Building {
                                rule: LearnedRule {
                                    language: String::new(),
                                    id,
                                    severity,
                                    description: text.to_string(),
                                    bad: String::new(),
                                    good: String::new(),
                                    construct: None,
                                },
                                from_heading: false,
                                heading_only: false,
                                blocks: Vec::new(),
                            });
                        }
                    }
                    lead.clear();
                }
                Seg::Code { info, body } => {
                    let Some(b) = cur.as_mut() else {
                        // Code no rule claims is still knowledge: the "what's normal" reference.
                        reference.push(body.clone());
                        lead.clear();
                        continue;
                    };
                    // Split the fence info into a language hint vs orientation words: a word is a
                    // language when it names the document's default language or a bundled grammar;
                    // everything else is prose the classifier reads.
                    let mut lang_hint: Option<String> = None;
                    let mut orient_words = String::new();
                    for tok in info.split(|c: char| !c.is_ascii_alphanumeric()).filter(|t| !t.is_empty()) {
                        // A fence hint aliases like a filename stem does: ```js names javascript
                        // (ledger #16 — the walker and the law must agree on language names).
                        let low = tok.to_lowercase();
                        let low = crate::lint_train::resolve_language(&low);
                        if lang_hint.is_none() && (low == default_lang || crate::lint_match::bundled_language(&low)) {
                            lang_hint = Some(low);
                        } else {
                            orient_words.push_str(tok);
                            orient_words.push(' ');
                        }
                    }
                    if let Some(l) = lang_hint {
                        if b.rule.language.is_empty() {
                            b.rule.language = l;
                        }
                    }
                    // The block joins its rule with its most-local words (lead-in + fence words);
                    // orientation is resolved when the rule COMMITS, so a labeled pair is compared
                    // against itself instead of each label being read absolutely.
                    b.blocks.push((format!("{lead} {orient_words}"), body.clone()));
                    lead.clear();
                }
            }
        }
        commit(&mut cur, &mut rules, &mut reference);
        Knowledge { rules, reference }
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

    /// A small trained classifier whose vocabulary covers the orientation words documents
    /// actually use — built from labeled prose exactly like the grounded path builds one.
    fn polarity() -> Polarity {
        Polarity::from_labeled(&[
            ("bad wrong avoid never do this it is deprecated and unsafe", true),
            ("never use this forbidden dangerous broken pattern anywhere", true),
            ("do not call this discouraged brittle unsafe function", true),
            ("this fragile obsolete style leaks and must not be used", true),
            ("good correct prefer this recommended safe form instead", false),
            ("always use the idiomatic clean supported approach here", false),
            ("this canonical fix is right and well tested", false),
            ("the proper maintainable way handles errors explicitly", false),
        ])
    }

    #[test]
    fn heading_less_plain_sentences_each_become_a_rule() {
        let doc = "Never call eval anywhere in this project; parse the input explicitly.\n\n\
                   Do not use list literals in configuration code; use tuples instead.\n";
        let k = Knowledge::read_document("any", doc, None);
        assert_eq!(k.rules.len(), 2, "one rule per instruction: {:?}", k.rules);
        assert_eq!(k.rules[0].id, "never_call_eval_anywhere_in_this_project_parse");
        assert!(k.rules[0].description.contains("parse the input explicitly"));
        assert_eq!(k.rules[0].language, "any");
        assert!(k.rules[1].description.contains("use tuples instead"));
    }

    #[test]
    fn heading_names_the_rule_and_prose_is_the_advice() {
        let doc = "## no_eval [high]\nNever call `eval` anywhere; parse the input explicitly.\n";
        let k = Knowledge::read_document("python", doc, None);
        assert_eq!(k.rules.len(), 1);
        let r = &k.rules[0];
        assert_eq!(r.id, "no_eval");
        assert_eq!(r.severity, "high");
        assert_eq!(r.description, "Never call `eval` anywhere; parse the input explicitly.");
        assert_eq!(r.language, "python");
    }

    #[test]
    fn descriptive_heading_keeps_its_words_in_the_advice() {
        let doc = "## Comments Explain Why, Not What\n\nCode already says what it does.\n";
        let k = Knowledge::read_document("any", doc, None);
        assert_eq!(k.rules[0].id, "comments_explain_why_not_what");
        assert!(k.rules[0].description.contains("Comments Explain Why"));
        assert!(k.rules[0].description.contains("Code already says"));
    }

    #[test]
    fn prohibition_prose_marks_its_code_block_as_the_violation() {
        let doc = "Never spawn raw threads for background work.\n\n```\nthread::spawn(work)\n```\n";
        let p = polarity();
        let k = Knowledge::read_document("rust", doc, Some(&p));
        assert_eq!(k.rules.len(), 1);
        assert_eq!(k.rules[0].bad, "thread::spawn(work)");
        assert!(k.rules[0].good.is_empty());
    }

    #[test]
    fn endorsement_prose_marks_its_code_block_as_the_fix() {
        let doc = "Prefer the recommended safe form for path handling, always use it.\n\n\
                   ```\nPath::new(p).join(q)\n```\n";
        let p = polarity();
        let k = Knowledge::read_document("rust", doc, Some(&p));
        assert_eq!(k.rules.len(), 1, "{:?}", k.rules);
        assert_eq!(k.rules[0].good, "Path::new(p).join(q)", "endorsed example is the fix, not the violation");
        assert!(k.rules[0].bad.is_empty());
    }

    #[test]
    fn bad_then_good_lead_ins_orient_both_blocks() {
        let doc = "## no_string_paths\nNever build paths by string concatenation, it is wrong and unsafe.\n\n\
                   Bad:\n\n```\np = a + \"/\" + b\n```\n\nGood:\n\n```\np = join(a, b)\n```\n";
        let p = polarity();
        let k = Knowledge::read_document("any", doc, Some(&p));
        assert_eq!(k.rules.len(), 1, "{:?}", k.rules);
        assert_eq!(k.rules[0].bad, "p = a + \"/\" + b");
        assert_eq!(k.rules[0].good, "p = join(a, b)");
    }

    #[test]
    fn document_order_breaks_ties_without_a_classifier() {
        let doc = "## q_no_containers\nxyzzy qwerty plugh zork.\n\n```\nxs = [1]\n```\n\n```\nxs = (1,)\n```\n";
        let k = Knowledge::read_document("any", doc, None);
        assert_eq!(k.rules[0].bad, "xs = [1]", "first block defaults to the violation");
        assert_eq!(k.rules[0].good, "xs = (1,)", "second block defaults to the fix");
    }

    #[test]
    fn fence_words_resolve_to_language_or_orientation_by_reading() {
        // `qx` matches the document's default language; `bad`/`good` classify as orientation.
        let doc = "## no_brackets\nDo not use bracket literals, never.\n\n\
                   ```qx:bad\nxs = [1]\n```\n\n```qx:good\nxs = tuple(1)\n```\n";
        let p = polarity();
        let k = Knowledge::read_document("qx", doc, Some(&p));
        let r = &k.rules[0];
        assert_eq!(r.language, "qx");
        assert_eq!(r.bad, "xs = [1]");
        assert_eq!(r.good, "xs = tuple(1)");
    }

    #[test]
    fn bundled_grammar_name_in_a_fence_sets_the_rule_language() {
        let doc = "## q_no_containers\nNever use list literals anywhere.\n\n```python\nxs = [1, 2]\n```\n";
        let p = polarity();
        let k = Knowledge::read_document("any", doc, Some(&p));
        assert_eq!(k.rules[0].language, "python");
        assert_eq!(k.rules[0].bad, "xs = [1, 2]");
    }

    #[test]
    fn orphan_code_is_reference_not_a_rule() {
        let doc = "```\nlet x = 1;\n```\n";
        let k = Knowledge::read_document("rust", doc, None);
        assert!(k.rules.is_empty());
        assert_eq!(k.reference, vec!["let x = 1;".to_string()]);
    }

    #[test]
    fn broken_markdown_degrades_instead_of_crashing() {
        let doc = "## broken [high\nunclosed severity, no fences\n``` \n,,,\n";
        let k = Knowledge::read_document("any", doc, None);
        assert!(!k.rules.is_empty(), "still yields the readable part");
        assert_eq!(k.rules[0].severity, "medium", "malformed tag falls back, never panics");
    }

    #[test]
    fn heading_started_rules_absorb_their_paragraphs_but_plain_rules_do_not_merge() {
        let doc = "## single_responsibility\nEach unit does one thing.\n\nSplit it when it grows.\n";
        let k = Knowledge::read_document("any", doc, None);
        assert_eq!(k.rules.len(), 1, "paragraphs under a heading are one rule");
        assert!(k.rules[0].description.contains("Split it when it grows"));
    }
}
