//! The HTML layer — HTML constructs as orthogonal ISM states, keyed STRUCTURALLY from the markup.
//!
//! Contract: `LINTER.md` → north-star section → "The HTML layer — structural construct-keying that
//! unblocks graduation". This is the first curriculum layer above the proven English bedrock, and its
//! one job is to supply the per-construct SUBJECT the English dictionary cannot key, so real HTML
//! construct truths ("`<strong>` means importance") corroborate ONLY from their own construct's prose.
//!
//! The frozen pieces are untouched — this module NEVER judges English and NEVER writes a tag name into
//! the meaning graph. It only READS a documentation page's structure to (a) key each witness sentence
//! by the construct whose reference page it belongs to and (b) drop page furniture, then hands a
//! subject-GATED witness stream to the frozen [`crate::lint_ism::graduate`]. Every English judgement
//! and the ≥15-witness law stay exactly the engine's.
//!
//! Why structural keying is the fix (measured, see `examples/htmlgrad.rs` and the LINTER.md
//! subsection): the dictionary knows `strong` only as an adjective, `em` as "them", and does not know
//! `b`/`dfn`/`kbd` at all, so different constructs collapse onto shared English predicates and a
//! `<em>` sentence corroborates a `<strong>` importance-truth. The subject key — page-of-origin, an
//! opaque markup symbol — discriminates them BEFORE the comparator ever runs.

use crate::doc_crawler::{drop_script_style, strip_tags};
use crate::lint_char::MeaningNetwork;
use crate::lint_english::English;
use crate::lint_ism::{graduate, Candidate, Verdict};

/// Stable MDN section anchors whose prose is page FURNITURE, not governing definition/usage text: the
/// interactive `try_it` demo, worked `examples`, the `technical_summary`/`specifications`/
/// `browser_compatibility` reference tables, the `see_also` link list, and `feedback` chrome. These
/// ids are URL-fragment slugs the page publishes for itself (`<h2 id="examples">`), so excluding by
/// them is a STRUCTURAL page-role filter, never a judgement of construct MEANING. The measurement
/// showed this prose asserts sibling constructs and example content that trip false contradictions
/// (LINTER.md HTML-layer subsection). INTERIM, like the other structural windows in this file — the
/// principled end state reads the section's role rather than matching its slug.
const NON_GOVERNING_ANCHORS: &[&str] =
    &["try_it", "examples", "technical_summary", "specifications", "browser_compatibility", "see_also", "feedback"];

/// A witness sentence carrying its STRUCTURAL subject key — the construct whose reference PAGE the
/// sentence belongs to (page-of-origin), an opaque markup symbol (the element/tag name), NEVER a
/// dictionary word. This is the HTML layer's own jargon; it gates which witnesses a construct's truth
/// ever sees and never enters the frozen English comparator's judgement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyedWitness {
    /// The construct this sentence is governing prose ABOUT — the element name of its origin page.
    pub subject: String,
    /// The English sentence, tag-stripped, ready for the frozen comparator's predicate judgement.
    pub sentence: String,
}

/// Split a documentation page body at its stable section anchors (`<h2 id="…">` / `<h3 id="…">`) into
/// `(anchor_id, region_html)`. The lead region — everything before the first anchor, where MDN states
/// the construct's definition — carries the empty anchor `""`. The ids are URL-fragment slugs the page
/// publishes for itself, so this is a STRUCTURAL segmentation by the page's own landmarks, exactly as
/// the URL path names the element. Region html is retained (not yet tag-stripped) so callers can read
/// both prose and the code/tag structure inside a region.
pub fn sections(body: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut anchor = String::new(); // lead region has no anchor
    let mut start = 0usize;
    let mut at = 0usize;
    while at < body.len() {
        let rest = &body[at..];
        let hit = ["<h2 id=\"", "<h3 id=\""].iter().find_map(|pat| rest.find(pat).map(|i| (at + i, pat.len())));
        let Some((open, pat_len)) = hit else { break };
        // Read the anchor id up to the closing quote.
        let id_start = open + pat_len;
        let Some(q) = body[id_start..].find('"') else { break };
        let next_anchor = body[id_start..id_start + q].to_string();
        out.push((anchor, body[start..open].to_string()));
        anchor = next_anchor;
        start = open;
        at = id_start + q;
    }
    out.push((anchor, body[start..].to_string()));
    out
}

/// Whether `token` is a construct-tag reference — the literal form `<name>` a sibling cross-reference
/// leaves in the prose after tag-stripping (MDN renders `<code>&lt;b&gt;</code>`, which decodes to the
/// bare token `<b>`). Name is one or more ASCII lowercase letters; `<name>` with anything else inside
/// is not a construct reference. Used to detect a FOREIGN construct mentioned in a candidate witness.
fn construct_tag(token: &str) -> Option<&str> {
    let inner = token.strip_prefix('<')?.strip_suffix('>')?;
    (!inner.is_empty() && inner.chars().all(|c| c.is_ascii_lowercase())).then_some(inner)
}

/// Every construct tag `<name>` referenced in `sentence` (tag-stripped prose), lowercased, in order.
/// Empty when the sentence names no construct — pure governing prose.
fn referenced_constructs(sentence: &str) -> Vec<&str> {
    sentence.split_whitespace().filter_map(|w| construct_tag(w.trim_matches(|c: char| c == '.' || c == ',' || c == ';'))).collect()
}

/// Split governing prose into sentences at `.!?` + whitespace/end, keeping sentences of at least
/// `MIN_SENTENCE_WORDS` words (drops nav fragments, headings, and code echoes). A rough splitter over
/// already-tag-stripped prose — the substrate's own sentence former is the principled replacement.
fn split_sentences(prose: &str) -> Vec<String> {
    const MIN_SENTENCE_WORDS: usize = 4;
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = prose.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        cur.push(c);
        if matches!(c, '.' | '!' | '?') && chars.get(i + 1).is_none_or(|n| n.is_whitespace()) {
            let s = cur.trim().to_string();
            if s.split_whitespace().count() >= MIN_SENTENCE_WORDS {
                out.push(s);
            }
            cur.clear();
        }
    }
    let tail = cur.trim();
    if tail.split_whitespace().count() >= MIN_SENTENCE_WORDS {
        out.push(tail.to_string());
    }
    out
}

/// Read the GOVERNING witnesses for `element` from its reference page `body`: sentences of the lead
/// definition and usage prose, keyed with `element` as their structural subject. Furniture sections
/// ([`NON_GOVERNING_ANCHORS`]) are dropped by their stable anchor, `<pre>` code/example blocks are
/// stripped, and any sentence that names a FOREIGN construct tag (a sibling cross-reference such as
/// "use the `<b>` element…") is excluded — a sentence about a sibling is not governing prose about
/// this subject, and those were a measured contradiction source. The construct's OWN tag in a sentence
/// is fine (that is the definition naming itself).
pub fn page_witnesses(element: &str, body: &str) -> Vec<KeyedWitness> {
    let element = element.to_lowercase();
    // Drop <script>/<style>/nav/header/footer/aside first — their raw text is NOT documentation and,
    // left in, survives tag-stripping as pathological run-on "sentences" (inline JS, chrome) that are
    // both noise and a cost blow-up for the comparator's per-concept BFS.
    let body = drop_script_style(body);
    let mut out = Vec::new();
    for (anchor, region) in sections(&body) {
        if NON_GOVERNING_ANCHORS.contains(&anchor.as_str()) {
            continue;
        }
        // Governing prose lives in the section's PARAGRAPHS. The page title (`<h1>`), the baseline-
        // availability indicator, breadcrumbs, and the in-page table of contents are structurally
        // NOT `<p>` — reading paragraphs drops that chrome without a phrase list, keeping only the
        // definition and usage statements.
        for para in paragraphs(&region) {
            let prose = strip_tags(&strip_pre_blocks(&para));
            for sentence in split_sentences(&prose) {
                if referenced_constructs(&sentence).iter().any(|&t| t != element) {
                    continue; // names a sibling construct — not governing prose about this subject
                }
                out.push(KeyedWitness { subject: element.clone(), sentence });
            }
        }
    }
    out
}

/// The inner HTML of every `<p …>…</p>` paragraph in `region`, in order. Governing documentation prose
/// is authored as paragraphs; the page's non-`<p>` furniture (title, baseline indicator, breadcrumbs,
/// table of contents) is left behind. A `<p>` with no close runs to the region end (tolerant of the
/// section-split cutting mid-element).
fn paragraphs(region: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while let Some(rel) = region[at..].find("<p") {
        let open = at + rel;
        // Confirm this is a <p> tag (next char ends the name: '>' or whitespace or '/'), not <pre>.
        let after = region[open + 2..].chars().next();
        if !matches!(after, Some('>') | Some(' ') | Some('\t') | Some('\n') | Some('/')) {
            at = open + 2;
            continue;
        }
        let Some(gt) = region[open..].find('>') else { break };
        let inner_start = open + gt + 1;
        let inner_end = region[inner_start..].find("</p>").map(|i| inner_start + i).unwrap_or(region.len());
        out.push(region[inner_start..inner_end].to_string());
        at = (inner_end + 4).min(region.len());
    }
    out
}

/// Discard cross-page-invariant CHROME from a MULTI-page witness pool, per the north-star rule
/// "Cross-page invariance = chrome, discarded." A sentence whose exact text recurs under MORE THAN ONE
/// distinct construct subject is site furniture — the Baseline availability banner, "Skip to main
/// content", the "This element only includes the global attributes" boilerplate — not governing prose
/// about any single construct; content UNIQUE to a page carries that page's meaning. The cut is
/// "appears for a single subject" (not a tuned frequency): a construct's definition names the
/// construct, so it is unique to that subject and kept, while chrome repeats across subjects and drops.
pub fn discard_chrome(witnesses: Vec<KeyedWitness>) -> Vec<KeyedWitness> {
    use std::collections::{BTreeSet, HashMap};
    let mut subjects_of: HashMap<&str, BTreeSet<&str>> = HashMap::new();
    for w in &witnesses {
        subjects_of.entry(w.sentence.as_str()).or_default().insert(w.subject.as_str());
    }
    let invariant: BTreeSet<String> = subjects_of
        .iter()
        .filter(|(_, subjects)| subjects.len() > 1)
        .map(|(sentence, _)| (*sentence).to_string())
        .collect();
    witnesses.into_iter().filter(|w| !invariant.contains(&w.sentence)).collect()
}

/// Remove `<pre …>…</pre>` blocks (worked code examples) from a region, replacing each with a space so
/// surrounding prose does not weld across the excision. Example CODE is structurally not governing
/// prose; keeping it injected example strings that tripped false contradictions (measured).
fn strip_pre_blocks(html: &str) -> String {
    let mut out = html.to_string();
    while let Some(s) = out.find("<pre") {
        match out[s..].find("</pre>") {
            Some(e) => out.replace_range(s..s + e + "</pre>".len(), " "),
            None => break,
        }
    }
    out
}

/// Graduate a construct truth via the FROZEN engine, gating the witness stream by construct-key
/// IDENTITY. Only witnesses whose `subject == subject` are offered to [`graduate`]; every English
/// judgement and the ≥15-independent-witness law are the engine's, unchanged. `truth` is the
/// construct's doc-grounded definition predicate; `foil` is a confusable SIBLING construct's
/// definition (a discriminating competing meaning). Returns the engine's [`Verdict`] plus the count of
/// subject-keyed witnesses actually offered (for honest reporting of witness scarcity).
pub fn graduate_construct(
    m: &MeaningNetwork,
    en: &English,
    subject: &str,
    truth: &str,
    foil: &str,
    witnesses: &[KeyedWitness],
) -> (Verdict, usize) {
    let subject = subject.to_lowercase();
    let gated: Vec<&str> =
        witnesses.iter().filter(|w| w.subject == subject).map(|w| w.sentence.as_str()).collect();
    let candidate = Candidate::new(truth, foil);
    (graduate(m, en, &candidate, gated.iter().copied()), gated.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_split_at_stable_anchors_lead_first() {
        let body = r#"<h1>Title</h1><p>The lead definition.</p>
            <h2 id="usage_notes" class="heading">Usage notes</h2><p>Use it well.</p>
            <h2 id="examples" class="heading">Examples</h2><pre>code</pre>"#;
        let secs = sections(body);
        assert_eq!(secs[0].0, "", "lead region carries the empty anchor");
        assert!(secs[0].1.contains("lead definition"));
        let anchors: Vec<&str> = secs.iter().map(|(a, _)| a.as_str()).collect();
        assert_eq!(anchors, vec!["", "usage_notes", "examples"]);
    }

    #[test]
    fn furniture_sections_and_sibling_sentences_are_excluded() {
        // Lead + usage govern; the examples section is furniture; the sibling sentence names <b>.
        let body = r#"<h1>x</h1><p>The strong element indicates that contents have strong importance here.</p>
            <h2 id="usage_notes">Usage notes</h2>
            <p>This element gives its contents great importance in the document.</p>
            <p>Use the <code>&lt;b&gt;</code> element to draw attention without importance.</p>
            <h2 id="examples">Examples</h2><p>Never feed him after midnight and never forget the rule.</p>"#;
        let ws = page_witnesses("strong", body);
        let sentences: Vec<&str> = ws.iter().map(|w| w.sentence.as_str()).collect();
        assert!(ws.iter().all(|w| w.subject == "strong"), "subject is page-of-origin");
        assert!(sentences.iter().any(|s| s.contains("indicates that contents have strong importance")));
        assert!(sentences.iter().any(|s| s.contains("great importance in the document")));
        assert!(
            !sentences.iter().any(|s| s.contains("draw attention")),
            "a sentence naming the sibling <b> is not governing prose about <strong>"
        );
        assert!(
            !sentences.iter().any(|s| s.contains("Never feed him")),
            "the examples section is furniture, excluded by its stable anchor"
        );
    }

    #[test]
    fn chrome_recurring_across_subjects_is_discarded() {
        // A sentence recurring under two DISTINCT subjects is site furniture; one unique to a subject
        // is that construct's own governing prose and is kept.
        let ws = vec![
            KeyedWitness { subject: "strong".into(), sentence: "This feature is widely available.".into() },
            KeyedWitness { subject: "em".into(), sentence: "This feature is widely available.".into() },
            KeyedWitness { subject: "strong".into(), sentence: "It indicates strong importance.".into() },
        ];
        let kept = discard_chrome(ws);
        assert_eq!(kept.len(), 1, "the cross-subject chrome sentence is dropped");
        assert_eq!(kept[0].sentence, "It indicates strong importance.");
    }

    #[test]
    fn subject_key_gate_admits_only_the_matching_construct() {
        // The leak-gone property at the unit level: graduating construct `strong` offers ONLY
        // strong-subject witnesses to the frozen engine — em/b-subject sentences never reach it,
        // however their English predicate would read. Brains-gated (defined only over the real
        // bedrock); skips honestly when no artifact is on disk.
        let (Some(br), Some(en)) = (crate::lint_char::brain(), crate::lint_english::brain()) else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        let ws = vec![
            KeyedWitness { subject: "strong".into(), sentence: "The element indicates strong importance.".into() },
            KeyedWitness { subject: "em".into(), sentence: "The element marks stress emphasis.".into() },
            KeyedWitness { subject: "b".into(), sentence: "The element draws attention to text.".into() },
        ];
        let (_verdict, offered) = graduate_construct(
            m,
            en,
            "strong",
            "the element indicates strong importance",
            "the element draws attention to text",
            &ws,
        );
        assert_eq!(offered, 1, "only the strong-subject witness is offered — foreign subjects gated out");
    }

    #[test]
    fn construct_tag_detection() {
        assert_eq!(construct_tag("<b>"), Some("b"));
        assert_eq!(construct_tag("<strong>"), Some("strong"));
        assert_eq!(construct_tag("<b"), None);
        assert_eq!(construct_tag("<b3>"), None);
        assert_eq!(construct_tag("bold"), None);
    }
}
