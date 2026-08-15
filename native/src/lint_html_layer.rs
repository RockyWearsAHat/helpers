//! The HTML layer — HTML constructs as orthogonal ISM states, keyed STRUCTURALLY from the markup.
//!
//! Contract: `native/history.dx` → north-star section → "The HTML layer — structural construct-keying that
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
//! Why structural keying is the fix (measured, see `examples/htmlgrad.rs` and the native/architecture.dx
//! subsection): the dictionary knows `strong` only as an adjective, `em` as "them", and does not know
//! `b`/`dfn`/`kbd` at all, so different constructs collapse onto shared English predicates and a
//! `<em>` sentence corroborates a `<strong>` importance-truth. The subject key — page-of-origin, an
//! opaque markup symbol — discriminates them BEFORE the comparator ever runs.
//!
//! Witnesses are pooled CROSS-SOURCE, keyed by the SAME structural subject: one reference page states a
//! construct's meaning only a handful of independent ways, short of the owner's ≥15, so a construct's
//! governing prose is gathered from every documentation SOURCE that publishes a page-of-origin unit for
//! it — MDN's per-element reference page ([`page_witnesses`]), the WHATWG spec's per-element section
//! ([`whatwg_witnesses`]), and W3Schools' per-tag page ([`w3schools_witnesses`]). Each source reduces
//! its own governing region to witnesses through the one shared reader ([`witnesses_from_paragraphs`]),
//! so the sibling-exclusion and example-stripping that killed the leak hold identically across sources,
//! and a witness's subject is always its page-of-origin — never a substring tag-match. Fifteen
//! corroborations drawn from three independent sources is STRONGER independence than fifteen from one.

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
/// (native/architecture.dx HTML-layer subsection). INTERIM, like the other structural windows in this file — the
/// principled end state reads the section's role rather than matching its slug.
const NON_GOVERNING_ANCHORS: &[&str] =
    &["try_it", "examples", "technical_summary", "specifications", "browser_compatibility", "see_also", "feedback"];

/// PASS 39 — whether `anchor` names one of this module's stable CHROME sections
/// ([`NON_GOVERNING_ANCHORS`]). Exposed read-only for the label exporter
/// ([`crate::lint_labels`]) to ground its CHROME training label from [`sections`]' own
/// segmentation; no new judgment — the list stays exactly what it always was.
pub(crate) fn is_chrome_anchor(anchor: &str) -> bool {
    NON_GOVERNING_ANCHORS.contains(&anchor)
}

/// PASS 39 — every heading span in `body` whose own markup carries a prohibition STATUS token,
/// as raw byte spans `(open, close)` of the heading element itself. A measuring-instrument
/// sibling of [`status_notice_binding`]: identical heading walk and identical judgment
/// ([`crate::lint_attest::class_carries_prohibition`]), but every status-classed heading rather
/// than the one URL-subject-matched winner — the label exporter ([`crate::lint_labels`]) grounds
/// its STATUS-MARKER training label here. Kept as its own small walk (not a refactor of the
/// frozen `status_notice_binding`) so the PASS 37 production arm stays untouched byte-for-byte.
pub(crate) fn status_marker_spans(body: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while let Some(rel) = body[at..].find("<h") {
        let open = at + rel;
        let Some(level) = body[open + 2..].chars().next().filter(|c| c.is_ascii_digit()) else {
            at = open + 2;
            continue;
        };
        let close_tag = format!("</h{level}>");
        if body[open..].find('>').is_none() {
            at = open + 2;
            continue;
        }
        let Some(end_rel) = body[open..].find(&close_tag) else {
            at = open + 2;
            continue;
        };
        let heading_end = open + end_rel + close_tag.len();
        let heading = &body[open..heading_end];
        at = heading_end;
        if crate::lint_attest::class_carries_prohibition(heading) {
            out.push((open, heading_end));
        }
    }
    out
}

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
        // Split at the EARLIEST heading of either level — take the MINIMUM position across both
        // patterns, never the first pattern in array order. `find_map` returning `<h2 id="`'s position
        // whenever any h2 exists SKIPPED every `<h3 id="` before a later h2, welding those subsections
        // (and their heading text — e.g. a "Never use …!" command heading) into the preceding region.
        let hit = ["<h2 id=", "<h3 id="]
            .iter()
            .filter_map(|pat| rest.find(pat).map(|i| (at + i, pat.len())))
            .min_by_key(|(pos, _)| *pos);
        let Some((open, pat_len)) = hit else { break };
        // Read the anchor id per the HTML attribute grammar — the value may be quoted or
        // UNQUOTED (the WHATWG serializer writes `<h3 id=non-conforming-features>`; demanding a
        // quote silently welded that spec's every section into one anchorless lead).
        let id_start = open + pat_len;
        let (val_start, val_end) = match body[id_start..].chars().next() {
            Some(q @ ('"' | '\'')) => {
                let Some(close) = body[id_start + 1..].find(q) else { break };
                (id_start + 1, id_start + 1 + close)
            }
            _ => {
                let close = body[id_start..]
                    .find(|c: char| c.is_ascii_whitespace() || c == '>')
                    .map(|i| id_start + i)
                    .unwrap_or(body.len());
                (id_start, close)
            }
        };
        let next_anchor = body[val_start..val_end].to_string();
        out.push((anchor, body[start..open].to_string()));
        anchor = next_anchor;
        start = open;
        at = val_end;
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
///
/// PASS 36 — the recall census: an anchor-skipped section is no longer a silent drop. When the
/// skipped region carries SIGNAL (a forbidding sentence by the frozen English, or a rendered
/// attestation marker — [`region_carries_signal`], never a regex or word list), the skip is
/// returned as a named withhold row `(read-<element>, "read gate (non-governing anchor: <slug>)")`
/// alongside the witnesses, deduped by the pair, for the caller to funnel into the one
/// conservation ledger. A signal-free furniture skip records nothing (the ledger discipline).
pub fn page_witnesses(element: &str, body: &str) -> (Vec<KeyedWitness>, Vec<(String, String)>) {
    let element = element.to_lowercase();
    // Drop <script>/<style>/nav/header/footer/aside first — their raw text is NOT documentation and,
    // left in, survives tag-stripping as pathological run-on "sentences" (inline JS, chrome) that are
    // both noise and a cost blow-up for the comparator's per-concept BFS.
    let body = drop_script_style(body);
    let mut out = Vec::new();
    let mut withheld: Vec<(String, String)> = Vec::new();
    for (anchor, region) in sections(&body) {
        if NON_GOVERNING_ANCHORS.contains(&anchor.as_str()) {
            if region_carries_signal(&region) {
                let row =
                    (format!("read-{element}"), format!("read gate (non-governing anchor: {anchor})"));
                if !withheld.contains(&row) {
                    withheld.push(row);
                }
            }
            continue;
        }
        // Governing prose lives in the section's PARAGRAPHS. The page title (`<h1>`), the baseline-
        // availability indicator, breadcrumbs, and the in-page table of contents are structurally
        // NOT `<p>` — reading paragraphs drops that chrome without a phrase list, keeping only the
        // definition and usage statements.
        out.extend(witnesses_from_paragraphs(&element, paragraphs(&region)));
    }
    (out, withheld)
}

/// Whether a section region carries governing SIGNAL worth a conservation-ledger row (PASS 36):
/// its prose STATES a prohibition through the frozen meaning network
/// ([`crate::lint_english::English::states_prohibition`] — a forbidding sentence), or its markup
/// carries a rendered author attestation status token
/// ([`crate::lint_attest::prohibition_class_tokens`], the same one hand datum the attester joins).
/// Existing english/attestation faculties only — never a regex or word list. `false` when no
/// English brain is loaded and no marker is present, so plain furniture never bloats the ledger.
fn region_carries_signal(region: &str) -> bool {
    if let Some(en) = crate::lint_english::brain() {
        if en.states_prohibition(&strip_tags(&strip_pre_blocks(region))) {
            return true;
        }
    }
    crate::lint_attest::Attestation::from_class_markers(crate::lint_attest::prohibition_class_tokens())
        .attests(region)
}

/// The shared governing-prose reader every source funnels through: reduce a list of already-extracted
/// paragraph HTML fragments to subject-keyed witnesses. Each paragraph has its `<pre>` example code
/// stripped, is tag-stripped to prose, split into sentences, and any sentence that names a FOREIGN
/// construct tag (a sibling cross-reference like `<b>`, surviving tag-stripping as the literal token)
/// is dropped — a sentence about a sibling is not governing prose about this subject. Source-agnostic:
/// MDN, WHATWG, and W3Schools differ only in HOW they carve a governing region into paragraphs; the
/// leak-killing rules (sibling exclusion, example-code stripping) are identical here for all of them.
fn witnesses_from_paragraphs(subject: &str, paras: Vec<String>) -> Vec<KeyedWitness> {
    let mut out = Vec::new();
    for para in paras {
        let prose = strip_tags(&strip_pre_blocks(&para));
        for sentence in split_sentences(&prose) {
            if referenced_constructs(&sentence).iter().any(|&t| t != subject) {
                continue;
            }
            out.push(KeyedWitness { subject: subject.to_string(), sentence });
        }
    }
    out
}

/// Read subject-keyed governing witnesses from the WHATWG single-page spec (the `text-level-semantics`
/// section documents `<strong>/<em>/<b>/<i>/<mark>/…`). Each element is its own page-of-origin unit: a
/// section opened by `<h4 id=the-<name>-element>`, so the subject is read STRUCTURALLY from that anchor
/// (the spec's own landmark for the element), never from a tag the prose contains. Only bare `<p>`
/// paragraphs are read: WHATWG authors normative definitional prose as attribute-less `<p>`, while the
/// browser-support annotation and worked examples carry a `class`, so [`bare_paragraphs`] drops that
/// furniture by page-role. INTERIM, like the MDN anchor filter — the principled end state reads the
/// region's role rather than its markup shape.
pub fn whatwg_witnesses(body: &str) -> Vec<KeyedWitness> {
    let body = drop_script_style(body);
    let mut out = Vec::new();
    for (subject, region) in whatwg_element_sections(&body) {
        // Normative DEFINITIONAL prose precedes the element's worked examples; everything after the
        // first worked-example block is example NARRATION ("Here, the word...", "In this example…"),
        // not a statement of the construct's meaning — truncate there structurally (the spec's own
        // `class=example` markup), the WHATWG analog of MDN's furniture-anchor cut.
        let governing = &region[..whatwg_examples_start(&region)];
        let paras = bare_paragraphs(governing)
            .into_iter()
            // A paragraph that hyperlinks a FOREIGN element section (`#the-<name>-element`, name ≠
            // subject) is a sibling cross-reference / see-also — not governing prose about this
            // construct, and a measured foil-assertion source. The spec links every element mention,
            // so this is a precise STRUCTURAL sibling filter, needing no construct vocabulary.
            .filter(|p| !whatwg_links_foreign_element(p, &subject))
            .collect();
        out.extend(witnesses_from_paragraphs(&subject, paras));
    }
    out
}

/// Byte offset of the element section's first worked-example block (`<div class=example>` or
/// `<pre class=example>`), or the region length when it has none. Prose before this offset is the
/// element's normative definition/usage; prose after it narrates worked examples.
fn whatwg_examples_start(region: &str) -> usize {
    ["<div class=example>", "<pre class=example"]
        .iter()
        .filter_map(|p| region.find(p))
        .min()
        .unwrap_or(region.len())
}

/// Whether a WHATWG paragraph hyperlinks a FOREIGN element's section — an anchor `#the-<name>-element`
/// whose `<name>` differs from `subject`. Such a paragraph is a sibling cross-reference (the spec
/// hyperlinks every element mention to its section), excluded like MDN's `<b>`-token sibling sentences.
fn whatwg_links_foreign_element(para: &str, subject: &str) -> bool {
    let mut at = 0usize;
    while let Some(rel) = para[at..].find("#the-") {
        let name_start = at + rel + "#the-".len();
        if let Some(end) = para[name_start..].find("-element") {
            let name = &para[name_start..name_start + end];
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) && name != subject
            {
                return true;
            }
        }
        at = name_start;
    }
    false
}

/// Split the WHATWG spec body into `(element_name, section_html)` at the per-element headings
/// `<h4 id=the-<name>-element>` (quoted or unquoted id). The section runs to the next such heading. The
/// element name between `the-` and `-element` is the structural subject — the spec's own page-of-origin
/// key for that construct.
fn whatwg_element_sections(body: &str) -> Vec<(String, String)> {
    let mut heads: Vec<(usize, String)> = Vec::new();
    for pat in ["<h4 id=the-", "<h4 id=\"the-"] {
        let mut at = 0usize;
        while let Some(rel) = body[at..].find(pat) {
            let open = at + rel;
            let name_start = open + pat.len();
            if let Some(end) = body[name_start..].find("-element") {
                let name = &body[name_start..name_start + end];
                if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase()) {
                    heads.push((open, name.to_string()));
                }
            }
            at = name_start;
        }
    }
    heads.sort_by_key(|(pos, _)| *pos);
    let mut out = Vec::new();
    for i in 0..heads.len() {
        let (start, name) = &heads[i];
        let end = heads.get(i + 1).map(|(p, _)| *p).unwrap_or(body.len());
        out.push((name.clone(), body[*start..end].to_string()));
    }
    out
}

/// Paragraphs whose opening tag carries NO attributes — exactly `<p>`. Used for the WHATWG spec, whose
/// normative definitional prose is authored as bare `<p>` while browser-support and worked-example
/// chrome carry a `class` attribute; reading only bare paragraphs keeps the governing statements and
/// drops that furniture structurally, without a phrase list.
fn bare_paragraphs(region: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while let Some(rel) = region[at..].find("<p>") {
        let inner_start = at + rel + "<p>".len();
        let inner_end =
            region[inner_start..].find("</p>").map(|i| inner_start + i).unwrap_or(region.len());
        out.push(region[inner_start..inner_end].to_string());
        at = (inner_end + "</p>".len()).min(region.len());
    }
    out
}

/// Read subject-keyed governing witnesses from a W3Schools per-tag reference page (`tag_<name>.asp`).
/// The page-of-origin unit is the whole page (subject = `element`, from the URL), and its governing
/// prose is the "Definition and Usage" region — the section from that heading to the next horizontal
/// rule / heading. The worked `Example` block above it and the browser-support / attributes tables
/// below are structurally outside that region and never read. The "Definition and Usage" landmark is
/// the source's own section label (a page-role filter, INTERIM, like the MDN anchor slugs) — never a
/// judgement of construct meaning.
pub fn w3schools_witnesses(element: &str, body: &str) -> Vec<KeyedWitness> {
    let body = drop_script_style(body);
    let Some(region) = w3schools_definition_region(&body) else { return Vec::new() };
    witnesses_from_paragraphs(&element.to_lowercase(), paragraphs(&region))
}

/// The "Definition and Usage" governing region of a W3Schools tag page: from that section heading to
/// the next `<hr` or `<h2` landmark, or `None` when the page has no such section. A structural slice by
/// the page's own section labels, never by meaning.
fn w3schools_definition_region(body: &str) -> Option<String> {
    let marker = body.find(">Definition and Usage<")?;
    let after = body[marker..].find("</h2>").map(|i| marker + i + "</h2>".len()).unwrap_or(marker);
    let end = ["<hr", "<h2"]
        .iter()
        .filter_map(|p| body[after..].find(p).map(|i| after + i))
        .min()
        .unwrap_or(body.len());
    Some(body[after..end].to_string())
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

// ── PASS 37 — the attribute dimension: three attestation SHAPES the reader learns ────────────────
//
// Contract: native/history.dx → "COMPLETION PASS 37", implementation subsection. Every judgement below is
// an existing frozen faculty (the English prohibition read) or the one-hand-datum status join
// ([`crate::lint_attest::class_carries_prohibition`]) — never an icon list, class list, or word
// list. All three readers are structural page readers: they run over the PRE-chrome-strip body
// (the banner-attester precedent — a recurring notice run is exactly what the chrome filter
// strips) and return `(subject, governing prose)` pairs for the module's attestation read.

/// LAW 1 — the per-attribute Deprecated badge inside an element page's attribute definition list
/// (the MDN dt/dd shape). A `<dt …>` entry whose OWN markup (open tag + interior) carries a class
/// value joining a prohibition status token attests its attribute deprecated; the attribute name
/// is the dt's own `id` (else its first interior text token, byte-preserved), and the governing
/// prose is the following `<dd>`'s tag-stripped text. The HOST element is the CALLER's fact (the
/// page's URL subject under the element-hood proof) — this reader only reads the entries.
///
/// PASS 37 production closure (class 2): an INTERFACE page writes its members in the DOTTED
/// member typography (`id="htmldivelement.align"`) — the attribute name is the member half
/// exactly where the owner half names `subject` (the page's URL subject, the PASS-35 member law,
/// never a split heuristic); and a dt whose own text writes CALL typography (`…()`) is a METHOD,
/// not an attribute — excluded by the author's own parentheses.
pub fn attribute_badges(subject: &str, body: &str) -> Vec<(String, String)> {
    let body = drop_script_style(body);
    let subject = subject.to_lowercase();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut at = 0usize;
    while let Some(rel) = body[at..].find("<dt") {
        let open = at + rel;
        // Confirm a real <dt> tag (next char ends the name), not <dtxyz…>.
        if !matches!(body[open + 3..].chars().next(), Some('>') | Some(' ') | Some('\t') | Some('\n') | Some('/')) {
            at = open + 3;
            continue;
        }
        let Some(gt) = body[open..].find('>') else { break };
        let entry_end = body[open..].find("</dt>").map(|i| open + i).unwrap_or(body.len());
        let entry = &body[open..entry_end];
        at = entry_end;
        if !crate::lint_attest::class_carries_prohibition(entry) {
            continue;
        }
        // The attribute name: the dt's own id, else the first text token of its interior.
        let open_tag = &body[open..open + gt + 1];
        let interior = &body[open + gt + 1..entry_end];
        let text = strip_tags(interior);
        // The author's CALL typography names a method — never an attribute.
        if text.split_whitespace().next().is_some_and(|t| t.contains('(')) {
            continue;
        }
        let name = tag_attr(open_tag, "id")
            .or_else(|| text.split_whitespace().next().map(str::to_string))
            .unwrap_or_default();
        // The dotted member typography: the member half, where the owner half is the subject.
        let name = match name.split_once('.') {
            Some((owner, member)) if owner.eq_ignore_ascii_case(&subject) => member.to_string(),
            _ => name,
        };
        if name.len() < 2 || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            continue;
        }
        // Governing prose: the following <dd>'s tag-stripped text.
        let governing = body[entry_end..]
            .find("<dd")
            .map(|i| entry_end + i)
            .and_then(|dd_open| {
                let dd_gt = body[dd_open..].find('>')?;
                let dd_end = body[dd_open..].find("</dd>").map(|i| dd_open + i).unwrap_or(body.len());
                Some(strip_tags(&body[dd_open + dd_gt + 1..dd_end]))
            })
            .unwrap_or_default();
        if !out.iter().any(|(n, _)| n == &name) {
            out.push((name, governing));
        }
    }
    out
}

/// One attribute value of a single HTML open tag (`id="…"`/`id='…'`), or `None`.
fn tag_attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=");
    let i = tag.find(&key)?;
    let rest = &tag[i + key.len()..];
    let q = rest.chars().next()?;
    if q != '"' && q != '\'' {
        return None;
    }
    let rest = &rest[1..];
    Some(rest[..rest.find(q)?].to_string())
}

/// Every element name a markup fragment writes in the page's own ESCAPED element typography
/// (`&lt;name&gt;`, ascii-lowercase name) — the `<x>` proof the PASS-35 element tier reads,
/// scanned off the raw markup (entities are the author's typography for "this is a tag").
fn escaped_element_names(fragment: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut at = 0usize;
    while let Some(rel) = fragment[at..].find("&lt;") {
        let start = at + rel + "&lt;".len();
        let name: String =
            fragment[start..].chars().take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit()).collect();
        at = start + name.len();
        if !name.is_empty() && fragment[at..].starts_with("&gt;") && !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

/// THE INTERFACE-HOST LAW (PASS 37 production closure, class 2): the HOST element(s) of an
/// INTERFACE page (`Web/API/HTMLDivElement`) are the page's OWN documented mapping — never a
/// name-splitting heuristic:
/// (a) THE MAPPING SENTENCE — a lead sentence naming the URL-subject interface as a whole token
///     and writing element(s) in `<x>` typography ("The HTMLDivElement interface … for
///     manipulating `<div>` elements."): every element it writes is a host. Where the sentence's
///     typography writes two same-stem numeric endpoints (`<h1>` through `<h6>`), the page
///     states the inclusive SPAN — the endpoints' own stem+numerals derive the intermediates.
/// (b) THE REFERENCES ARM, only when no sentence binds — a link whose anchor wraps `&lt;x&gt;`
///     typography AND whose href's last segment names `x` (the URL-subject law applied to the
///     LINK: HTMLMediaElement's "References `<video>` and `<audio>` HTML elements").
/// Empty when the page publishes no mapping (honest abstention). The second return is TRUE
/// when the hosts came from the REFERENCES arm — a link-list's entries are an ENUMERATION
/// (every entry an equal claim), while a mapping sentence's elements beyond the first can be
/// CONTEXT (the primary-claim law's yield applies only to mapping-sentence hosts).
pub fn interface_hosts(subject: &str, body: &str) -> (Vec<String>, bool) {
    let body = drop_script_style(body);
    // Arm (a) — the mapping sentence, over the lead's paragraphs.
    let lead = sections(&body)
        .into_iter()
        .find(|(a, _)| a.is_empty())
        .map(|(_, r)| r)
        .unwrap_or_default();
    let mut hosts: Vec<String> = Vec::new();
    for s in region_sentences(&lead) {
        // The subject must stand in the author's OWN interface typography — the URL segment's
        // exact-case spelling as its own word (whitespace-delimited, punctuation-trimmed).
        // MEASURED: case-folded matching admitted "… the old HTML event handler attributes"
        // (the WORD event) as a mapping sentence for the Event interface, and hyphen-compound
        // splitting admitted "Event-handlers … (such as <button>, <div>, …)".
        let names_subject = s
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_ascii_alphanumeric()))
            .any(|t| t == subject);
        if !names_subject {
            continue;
        }
        for name in sentence_element_tokens(&s) {
            if !hosts.contains(&name) {
                hosts.push(name);
            }
        }
    }
    expand_numeric_span(&mut hosts);
    if !hosts.is_empty() {
        return (hosts, false);
    }
    // Arm (b) — the references link-list: an anchor wrapping the element's own typography whose
    // href's last segment names the element (the URL-subject law on the link), read ONLY from
    // the page's `see_also` reference section (the same stable anchor slug the witness reader
    // already treats as a structural landmark — INTERIM like every slug landmark; measured: a
    // whole-body scan admitted a mid-page events link to `<source>` as a junk host).
    let mut see_also = String::new();
    let mut in_tail = false;
    for (anchor, region) in sections(&body) {
        // The see-also block is the page's TERMINAL reference matter; its own sub-headings
        // (`<h3 id="references">`) split into follow-on sections, so take the whole tail.
        in_tail = in_tail || anchor == "see_also";
        if in_tail {
            see_also.push_str(&region);
        }
    }
    let body = see_also;
    let mut at = 0usize;
    while let Some(rel) = body[at..].find("<a ") {
        let open = at + rel;
        let end = body[open..].find("</a>").map(|i| open + i).unwrap_or(body.len());
        let anchor = &body[open..end];
        at = end + 1;
        let Some(href) = tag_attr(&anchor[..anchor.find('>').map(|i| i + 1).unwrap_or(anchor.len())], "href")
        else {
            continue;
        };
        let seg = href
            .split(['?', '#'])
            .next()
            .unwrap_or(&href)
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_lowercase();
        for name in escaped_element_names(anchor) {
            if name == seg && !hosts.contains(&name) {
                hosts.push(name);
            }
        }
    }
    (hosts, true)
}

/// Every element name a tag-stripped SENTENCE writes in the decoded `<x>` typography (ascii
/// lowercase + digits), reading each whitespace token trimmed of surrounding punctuation — the
/// prose-side twin of [`escaped_element_names`].
fn sentence_element_tokens(sentence: &str) -> Vec<String> {
    let mut out = Vec::new();
    for tok in sentence.split_whitespace() {
        let tok = tok.trim_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '<' || c == '>'));
        let Some(inner) = tok.strip_prefix('<').and_then(|t| t.strip_suffix('>')) else { continue };
        if !inner.is_empty()
            && inner.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            && !out.contains(&inner.to_string())
        {
            out.push(inner.to_string());
        }
    }
    out
}

/// THE SPAN of two same-stem numeric endpoints (`h1` + `h6` → `h2..h5` added): when a mapping
/// sentence's typography writes exactly two names sharing one alphabetic stem with integer
/// suffixes a < b, the page states the inclusive range — the intermediates are derived from the
/// endpoints' own stem+numerals, never a list.
fn expand_numeric_span(hosts: &mut Vec<String>) {
    let parse = |n: &str| -> Option<(String, u32)> {
        let stem: String = n.chars().take_while(|c| !c.is_ascii_digit()).collect();
        let digits = &n[stem.len()..];
        (!stem.is_empty() && !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
            .then(|| digits.parse().ok().map(|d| (stem, d)))
            .flatten()
    };
    let numbered: Vec<(String, u32)> = hosts.iter().filter_map(|h| parse(h)).collect();
    if let [(s1, a), (s2, b)] = numbered.as_slice() {
        if s1 == s2 && a < b {
            for k in *a + 1..*b {
                let name = format!("{s1}{k}");
                if !hosts.contains(&name) {
                    hosts.push(name);
                }
            }
        }
    }
}

/// The MEMBER concept axis (native/history.dx PASS 41) — the owner's named seeds for "an element's
/// property": a prohibition-status section headed by one of these lists members, never elements.
const MEMBER_ANCHORS: &[&str] = &["attribute", "property"];
/// The CONSTRUCT concept axis (native/history.dx PASS 41) — the owner's named seeds for "a language
/// construct": a section headed by one of these (or by a noun the net cannot key) lists elements.
const CONSTRUCT_ANCHORS: &[&str] = &["element", "tag", "feature"];

/// THE SECTION-SUBJECT LAW (PASS 41): whether this prohibition-status section's HEADING names a
/// MEMBER section, so the element-roster read must ABSTAIN. The head noun is read HEAD-FINAL — the
/// rightmost heading word that binds a dictionary meaning (prohibition status tokens dropped, each
/// candidate lemmatised through the shared English suffix morphology so a plural `attributes`
/// resolves to its keyed singular `attribute`) — the English noun-phrase head, not any word
/// present ("Deprecated HTML Element Interfaces" heads on `interfaces`, not the `element`
/// modifier). It is a member section iff the trained meaning net places that head nearer a
/// [`MEMBER_ANCHORS`] word than any [`CONSTRUCT_ANCHORS`] word ([`MeaningNetwork::related`]).
/// NEGATIVE by construction: a head the net cannot key, or one nearer a construct anchor, is NOT a
/// member section (the read proceeds), so a true roster is never suppressed.
fn heading_names_members(heading: &str, m: &crate::lint_char::MeaningNetwork) -> bool {
    let tokens = crate::lint_attest::prohibition_class_tokens();
    let is_status = |w: &str| {
        tokens.iter().any(|t| {
            t.split(|c: char| !c.is_ascii_alphanumeric())
                .any(|part| !part.is_empty() && part == w)
        })
    };
    let head = heading
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() >= 3 && !is_status(w))
        .filter_map(|w| {
            crate::lint_graph::english_lemma(|q| m.has(q), w, crate::lint_graph::READ_SUFFIXES)
        })
        .last();
    let Some(head) = head else { return false };
    let member = MEMBER_ANCHORS.iter().map(|a| m.related(&head, a)).min().unwrap_or(u32::MAX);
    let construct = CONSTRUCT_ANCHORS.iter().map(|a| m.related(&head, a)).min().unwrap_or(u32::MAX);
    member < construct
}

/// LAW 2a — the Elements-index obsolete section (the MDN `content`/`image` shape). An ANCHORED
/// section (never the lead) whose own prose STATES a prohibition (the frozen English) or whose
/// heading text carries a prohibition status token by the one-hand-datum join attests every
/// element its `<dt>` entries name in the page's own `&lt;x&gt;` typography. Governing = the
/// entry's own `<dd>` prose, else the section's stating sentence.
pub fn obsolete_index_entries(
    body: &str,
    en: &crate::lint_english::English,
    m: &crate::lint_char::MeaningNetwork,
) -> Vec<(String, String)> {
    let body = drop_script_style(body);
    let mut out: Vec<(String, String)> = Vec::new();
    for (anchor, region) in sections(&body) {
        if anchor.is_empty() {
            continue; // the lead is never an index section
        }
        // The prohibition read runs per PARAGRAPH (the authored prose unit): whole-region prose
        // welds the heading text onto the first sentence ("Old elements Never use…"), which
        // defeats the frozen English's command-position read.
        let para_sentences: Vec<String> = paragraphs(&strip_pre_blocks(&region))
            .iter()
            .flat_map(|p| {
                let prose = strip_tags(p);
                crate::lint_read::sentences(&prose)
                    .iter()
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<String>>()
            })
            .collect();
        // The join compares each status value as its WORD SEQUENCE contiguously contained in
        // the heading's words, so a hyphenated status value ("non-conforming" — WHATWG's own
        // vocabulary for "authors must not use") joins its heading exactly as a single-word
        // value does; single-word membership alone broke every hyphenated status.
        let heading = heading_text(&region);
        let heading_words_join = heading
            .as_deref()
            .map(|h| {
                let words: Vec<String> = h
                    .to_lowercase()
                    .split(|c: char| !c.is_ascii_alphanumeric())
                    .filter(|w| !w.is_empty())
                    .map(str::to_string)
                    .collect();
                crate::lint_attest::prohibition_class_tokens().iter().any(|p| {
                    let seq: Vec<&str> =
                        p.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).collect();
                    !seq.is_empty()
                        && words
                            .windows(seq.len())
                            .any(|w| w.iter().map(String::as_str).eq(seq.iter().copied()))
                })
            })
            .unwrap_or(false);
        let stating = para_sentences
            .iter()
            .find(|s| en.sentence_states_prohibition(s))
            .cloned()
            .unwrap_or_default();
        // The GATE is the heading's own status typography (the one-hand-datum join). An
        // English-read stating sentence alone was MEASURED over the whole corpus to admit 43
        // junk sections (Content_categories, CSS gradients, HTTP headers — arbitrary prose the
        // frozen English over-reads); the section's stated prohibition remains the governing
        // FALLBACK for entries without their own description, never the gate.
        if !heading_words_join {
            continue;
        }
        // THE SECTION-SUBJECT LAW (PASS 41): the reader defers to English on the section's HEAD
        // NOUN. A prohibition-status section headed by a MEMBER noun ("Deprecated attributes",
        // "…Interfaces") lists members — Law 1's `host@attr` owns them — so the roster read
        // ABSTAINS; the same rows were minting junk `<abbr>`/`<img>` element rules. Only a
        // construct-headed (or unkeyable) section proceeds. Negative by construction — a true
        // roster ("Obsolete and deprecated elements") is never suppressed.
        if heading.as_deref().is_some_and(|h| heading_names_members(h, m)) {
            continue;
        }
        // THE REPEATING-ENTRY LAW (PASS 37 production closure, class 3): the section's entries
        // are whatever repeating child structure carries the `<x>` typography — the real MDN
        // index authors a TABLE (23 `<tr>` rows, zero `<dt>`), the fixture a definition list.
        for (element, governing) in repeating_entries(&region) {
            let governing = if governing.trim().is_empty() { stating.clone() } else { governing };
            if element.len() >= 2 && !out.iter().any(|(e, _)| e == &element) {
                out.push((element, governing));
            }
        }
    }
    out
}

/// The section's ATTESTING ENTRIES, read off its own repetition (no shape enumeration): a
/// candidate unit is any tag kind repeated in the region; a unit ATTESTS when its tag-stripped
/// text BEGINS with the decoded `<x>` element typography (an entry leads with its own name — a
/// cross-reference inside a description never leads its unit, so `<slot>` in the `content` row's
/// prose stays unattested), or — the CODE-TYPOGRAPHY ARM (PASS 37 WHATWG re-land) — when its
/// definition term is EXACTLY a bare name the author wraps in code font and a description
/// follows (WHATWG's obsolete list writes `bgsound` without brackets; its attribute rows read
/// "charset on a elements" and are structurally excluded); the entry kind is the OUTERMOST
/// repeated kind with at least two attesting units — the one whose first occurrence opens
/// EARLIEST (`tr` opens before the `td`/`a`/`code` it wraps; `dt` before its inner `code`),
/// ties to fewest units — and each attesting unit's governing prose is its own remainder text
/// (the row's description cell / the dt's following dd).
fn repeating_entries(region: &str) -> Vec<(String, String)> {
    use std::collections::BTreeMap;
    // Every opening-tag kind's occurrence positions, in document order.
    let mut positions: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut i = 0usize;
    while let Some(rel) = region[i..].find('<') {
        let p = i + rel;
        let name: String = region[p + 1..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        i = p + 1 + name.len().max(1);
        if !name.is_empty() {
            positions.entry(name).or_default().push(p);
        }
    }
    // A unit attests when its stripped text BEGINS with `<x>` typography; its governing prose is
    // the remainder. Choose the kind whose first ATTESTING unit opens earliest (the outermost
    // entry structure: `<tr>` opens before the `<td>`/`<a>`/`<code>` it wraps — an incidental
    // earlier occurrence that attests nothing, like a heading's own anchor link, never ranks),
    // ties to fewest units.
    let attesting = |starts: &[usize]| -> (usize, usize, Vec<(String, String)>) {
        let mut entries = Vec::new();
        let mut definitional = 0usize;
        let mut first_attesting = region.len();
        for (j, &p) in starts.iter().enumerate() {
            let end = starts.get(j + 1).copied().unwrap_or(region.len());
            let text = strip_tags(&region[p..end]);
            let text = text.trim_start();
            if let Some(rest) = text.strip_prefix('<') {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                    .collect();
                if name.is_empty() || !rest[name.len()..].starts_with('>') {
                    continue;
                }
                // An ENTRY is subject + description: its leading name is followed by PROSE. A
                // name-RUN ("<circle>, <ellipse>, …" — a feature table's element list) and a
                // name-only unit are references, not entries (measured junk sources).
                let governing = rest[name.len() + 1..]
                    .trim_start_matches(|c: char| !c.is_ascii_alphabetic() && c != '<');
                if !governing.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
                    continue;
                }
                definitional += 1;
                first_attesting = first_attesting.min(p);
                entries.push((name, governing.trim().to_string()));
                continue;
            }
            // THE CODE-TYPOGRAPHY ARM (PASS 37, WHATWG re-land): an entry may lead with the
            // author's code-font term instead of `<x>` bracket typography, under three
            // structural conditions. (1) The definition term's OWN text — the unit up to its
            // first `<dd` sibling — is EXACTLY the bare name: WHATWG's element rows define
            // "bgsound" alone, while its attribute rows define "charset on a elements" and are
            // thereby structurally excluded from element-hood. (2) The raw term wraps the name
            // in the page's own code typography (`<code>name<`). (3) Prose follows in the
            // remainder (the `<dd>` description), same as the bracket arm. Element-hood then
            // closes by construction at the caller's demo gate (PASS 35). The section-heading
            // status gate upstream is unchanged — this arm never runs outside a
            // prohibition-status section.
            let raw = &region[p..end];
            let (term_raw, desc_raw) = match raw.find("<dd") {
                Some(dd) => (&raw[..dd], &raw[dd..]),
                None => {
                    // A term with no `<dd>` of its own sits in a SHARED-DESCRIPTION RUN — the
                    // dt+dd list grammar runs several terms into one description ("big",
                    // "blink", … "tt" → "Use appropriate elements or CSS instead."), and only
                    // the run's last term sees the `<dd>` inside its own span. The shared
                    // description governs EVERY term of its run, so a qualifying bare
                    // code-typography term attests with the run's `<dd>` text; a short term
                    // that does not qualify still counts as definition STRUCTURE (the spec's
                    // attribute runs), keeping the majority honest. Prose units — the measured
                    // junk class — remain non-definitional by their own sentence shape.
                    let t = strip_tags(raw);
                    let t = t.trim();
                    if t.is_empty() || t.len() > 80 || t.contains('.') {
                        continue;
                    }
                    definitional += 1;
                    let name = t.to_string();
                    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                        || !raw.to_lowercase().contains(&format!("<code>{name}<"))
                    {
                        continue;
                    }
                    // The run's shared `<dd>`: the next one after this unit, provided the list
                    // has not closed in between (a run-ending term at a list boundary must not
                    // steal a later list's description).
                    let ahead = &region[end..];
                    let Some(dd_rel) = ahead.find("<dd") else { continue };
                    if ahead[..dd_rel].contains("</dl") {
                        continue;
                    }
                    let dd = &ahead[dd_rel..];
                    let dd_end =
                        dd.find("<dt").or_else(|| dd.find("</dl")).unwrap_or(dd.len());
                    let governing = strip_tags(&dd[..dd_end]);
                    let governing = governing.trim();
                    if governing.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
                        first_attesting = first_attesting.min(p);
                        entries.push((name, governing.to_string()));
                    }
                    continue;
                }
            };
            // Term + description IS definition structure, whatever the term names — WHATWG's
            // obsolete-attribute rows ("charset on a elements") are real entries of the same
            // list, structurally excluded from ELEMENT-hood below but counted toward the
            // kind's definitional majority (the majority filter rejects prose-junk kinds, and
            // a sibling definition row is not prose junk — measured: 17 element rows among 172
            // dt entries in the spec's non-conforming section).
            definitional += 1;
            let name = strip_tags(term_raw).trim().to_string();
            if name.is_empty()
                || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            {
                continue;
            }
            if !term_raw.to_lowercase().contains(&format!("<code>{name}<")) {
                continue;
            }
            let governing = strip_tags(desc_raw);
            let governing = governing.trim();
            if !governing.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
                continue;
            }
            first_attesting = first_attesting.min(p);
            entries.push((name, governing.to_string()));
        }
        (first_attesting, definitional, entries)
    };
    positions
        .iter()
        .filter(|(_, starts)| starts.len() >= 2)
        .map(|(_, starts)| (starts.len(), attesting(starts)))
        // The entry kind LISTS entries: at least two attest, and DEFINITION-STRUCTURED units
        // (attesting entries plus their structurally-excluded sibling rows — e.g. the spec's
        // obsolete-attribute definitions sharing the element list) are the MAJORITY of the
        // kind's units. A kind whose units are mostly non-definitional is prose structure that
        // happens to contain cross-references — the measured junk the filter exists to reject;
        // a sibling definition row is not junk and must not veto its own list.
        .filter(|(units, (_, definitional, entries))| {
            entries.len() >= 2 && definitional * 2 > *units
        })
        .min_by_key(|&(units, (first_attesting, _, _))| (first_attesting, units))
        .map(|(_, (_, _, entries))| entries)
        .unwrap_or_default()
}

/// The text of the first heading element (`<h1…6`) in a region, tag-stripped. `None` when the
/// region opens with no heading.
fn heading_text(region: &str) -> Option<String> {
    let open = region.find("<h")?;
    let level = region[open + 2..].chars().next().filter(|c| c.is_ascii_digit())?;
    let gt = region[open..].find('>')? + open;
    let close = format!("</h{level}>");
    let end = region[gt..].find(&close)? + gt;
    Some(strip_tags(&region[gt + 1..end]))
}

/// LAW 2b — the "Not Supported" page-role notice (the w3schools `applet` shape), under THE
/// SUBJECT-BINDING LAW (PASS 37 production closure, law 1): the notice is a STATUS-CLASSED
/// NOTICE HEADING — a heading whose own markup carries a prohibition status token by the
/// one-hand-datum join ([`crate::lint_attest::class_carries_prohibition`]; w3schools authors
/// the real notice as `<h2><span class="deprecated">Not Supported in HTML5.</span></h2>`),
/// whose governed region (heading → next heading / `<hr`) writes the URL-subject element in
/// the page's own `&lt;x&gt;` typography.
///
/// A bare PROSE arm (an English-read prohibition sentence, even one self-naming the subject)
/// was built and MEASURED over the whole corpus: it contributed only junk ("Different browsers
/// may use different default types for the `<button>` element" self-names `<button>`) and zero
/// truths — every real notice binds through the status typography — so a page-scope element
/// attestation requires the page's own STATUS TYPOGRAPHY (the PASS-35 banner-truth doctrine),
/// never a prose sentence. The frozen-English parameter stays: the caller's read of the
/// governed region is language, not this gate. `None` when no notice binds (honest abstention).
pub fn not_supported_subject(
    url: &str,
    body: &str,
    _en: &crate::lint_english::English,
) -> Option<(String, String)> {
    let body = drop_script_style(body);
    // The URL-subject law: the last path segment names the element as a whole token.
    let seg = url.split(['?', '#']).next().unwrap_or(url).trim_end_matches('/').rsplit('/').next()?;
    let seg_tokens: Vec<String> = seg
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .collect();
    status_notice_binding(&body, &seg_tokens)
}

/// The status-classed notice heading (subject-binding law, arm a): scan every heading
/// (`<hN>…</hN>`) whose own markup carries a prohibition status token by the one-hand-datum
/// join; its governed region runs to the next heading or `<hr`. The notice attests the
/// URL-subject element the region writes in `&lt;x&gt;` typography; governing = the notice text
/// plus the region's own longest sentence naming the element.
fn status_notice_binding(body: &str, seg_tokens: &[String]) -> Option<(String, String)> {
    let mut at = 0usize;
    while let Some(rel) = body[at..].find("<h") {
        let open = at + rel;
        let Some(level) = body[open + 2..].chars().next().filter(|c| c.is_ascii_digit()) else {
            at = open + 2;
            continue;
        };
        let close = format!("</h{level}>");
        let (Some(gt), Some(end_rel)) = (body[open..].find('>'), body[open..].find(&close)) else {
            at = open + 2;
            continue;
        };
        let heading = &body[open..open + end_rel + close.len()];
        at = open + end_rel + close.len();
        if !crate::lint_attest::class_carries_prohibition(heading) {
            continue;
        }
        let notice = strip_tags(&heading[gt + 1..heading.len().min(end_rel)]);
        // The governed region: from the heading's close to the next heading or rule.
        let region_end = ["<h", "<hr"]
            .iter()
            .filter_map(|p| body[at..].find(p).map(|i| at + i))
            .min()
            .unwrap_or(body.len());
        let region = &body[at..region_end];
        let Some(element) =
            escaped_element_names(region).into_iter().find(|e| seg_tokens.iter().any(|t| t == e))
        else {
            continue;
        };
        let naming = region_sentences(region)
            .into_iter()
            .filter(|s| {
                s.split(|c: char| !c.is_ascii_alphanumeric())
                    .any(|t| t.eq_ignore_ascii_case(&element))
            })
            .max_by_key(|s| s.len())
            .unwrap_or_default();
        let governing = if naming.is_empty() || naming == notice {
            notice
        } else {
            format!("{notice} {naming}")
        };
        return Some((element, governing));
    }
    None
}

/// A region's prose sentences read per PARAGRAPH (the authored prose unit): whole-region prose
/// welds heading text onto the first sentence, defeating the frozen English's command-position
/// read — the shared reader for the subject-binding arms.
fn region_sentences(region: &str) -> Vec<String> {
    paragraphs(&strip_pre_blocks(region))
        .iter()
        .flat_map(|p| {
            let prose = strip_tags(p);
            crate::lint_read::sentences(&prose)
                .iter()
                .map(|s| s.trim().to_string())
                .collect::<Vec<String>>()
        })
        .collect()
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

// Tests prove the MECHANISM with ZERO embedded HTML knowledge (per the native/architecture.dx invariant: no real
// construct names or meanings anywhere, including tests). Subjects are ABSTRACT opaque keys
// (`alpha`/`beta`/`gamma`) and prose is SYNTHETIC (a construct "conveys a widget property", a sibling
// "marks a gadget region"). What is under test is page-of-origin subject-keying, the chrome/sibling/
// example/furniture exclusions, and the leak-gone property — never any fact about HTML.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_split_at_stable_anchors_lead_first() {
        // The lead region (before the first anchor) carries the empty key; each later region carries
        // its own anchor slug, whatever the slug happens to be.
        let body = r#"<h1>Title</h1><p>The lead definition sits here plainly.</p>
            <h2 id="region_two" class="heading">Region two</h2><p>More governing prose follows here.</p>
            <h2 id="region_three" class="heading">Region three</h2><pre>code</pre>"#;
        let secs = sections(body);
        assert_eq!(secs[0].0, "", "lead region carries the empty anchor");
        assert!(secs[0].1.contains("lead definition"));
        let anchors: Vec<&str> = secs.iter().map(|(a, _)| a.as_str()).collect();
        assert_eq!(anchors, vec!["", "region_two", "region_three"]);
    }

    #[test]
    fn sections_split_at_the_earliest_heading_not_the_first_pattern() {
        // An `<h3 id>` subsection that PRECEDES a later `<h2 id>` must open its own region — the split is
        // at the EARLIEST heading of either level, never the first pattern in array order. Before the fix,
        // `find_map` returned the `<h2 id>` position whenever any h2 existed, WELDING the earlier h3 (and
        // its heading text — a "Never use …!" command) into the preceding region.
        let body = r#"<h1>Title</h1><p>Lead prose sits here plainly.</p>
            <h3 id="early_note" class="heading">Never use the thing directly!</h3><p>Because it is unsafe here.</p>
            <h2 id="later_region" class="heading">Later region</h2><p>Closing governing prose follows.</p>"#;
        let anchors: Vec<String> = sections(body).into_iter().map(|(a, _)| a).collect();
        assert_eq!(anchors, vec!["", "early_note", "later_region"], "the h3 before the h2 opens its own region");
        let early = sections(body).into_iter().find(|(a, _)| a == "early_note").unwrap().1;
        assert!(early.contains("Never use the thing directly!"), "the h3 heading text lands in its OWN region, not welded into the lead");
    }

    #[test]
    fn furniture_sections_and_sibling_sentences_are_excluded() {
        // Lead + a governing section define subject `alpha`; a paragraph naming the sibling construct
        // `<beta>` is not governing prose about alpha; a furniture section (keyed by one of the
        // module's OWN non-governing anchors, taken from the constant so no slug is hard-coded here) is
        // dropped whole. No HTML meaning is asserted — only these structural exclusions.
        let furniture_anchor = NON_GOVERNING_ANCHORS[0];
        let body = format!(
            r#"<h1>x</h1><p>The alpha construct conveys a widget property here.</p>
            <h2 id="governing_notes">Governing notes</h2>
            <p>The alpha construct always conveys that widget property clearly.</p>
            <p>Use the <code>&lt;beta&gt;</code> construct to mark a gadget region.</p>
            <h2 id="{furniture_anchor}">Furniture</h2><p>An unrelated furniture sentence lives right here.</p>"#
        );
        let ws = page_witnesses("alpha", &body).0;
        let s: Vec<&str> = ws.iter().map(|w| w.sentence.as_str()).collect();
        assert!(ws.iter().all(|w| w.subject == "alpha"), "subject is page-of-origin — always alpha");
        assert!(s.iter().any(|x| x.contains("conveys a widget property")));
        assert!(s.iter().any(|x| x.contains("always conveys that widget property")));
        assert!(!s.iter().any(|x| x.contains("gadget")), "the sibling <beta> sentence is not governing prose");
        assert!(!s.iter().any(|x| x.contains("furniture sentence")), "furniture section excluded by its anchor");
    }

    #[test]
    fn whatwg_reader_keys_by_section_excludes_examples_and_foreign_links() {
        // A spec-shaped page with two element sections. Witnesses are keyed by the SECTION they belong
        // to (page-of-origin, structural). Within a section: bare `<p>` normative prose is kept; a
        // browser-support `<p class=…>` is dropped; a paragraph HYPERLINKING a foreign element section
        // is a sibling reference, dropped; prose after the first worked example is narration, dropped.
        let body = r#"<h4 id=the-alpha-element>The alpha element</h4>
            <p>The alpha construct conveys a widget property to readers.</p>
            <p class=all-engines-text>Support exists in all current engines here.</p>
            <p>Importance follows because alpha marks the widget clearly.</p>
            <p>Prefer the <a href=#the-beta-element>beta</a> construct for a gadget instead.</p>
            <div class=example><p>Here alpha wraps an example phrase that is shown.</p></div>
            <p>In this example alpha decorates the sample text below.</p>
            <h4 id=the-beta-element>The beta element</h4>
            <p>The beta construct marks a gadget region for readers.</p>"#;
        let ws = whatwg_witnesses(body);
        let alpha: Vec<&str> = ws.iter().filter(|w| w.subject == "alpha").map(|w| w.sentence.as_str()).collect();
        assert!(
            ws.iter().any(|w| w.subject == "beta" && w.sentence.contains("marks a gadget region")),
            "the second section keys its prose to beta, structurally"
        );
        assert!(alpha.iter().any(|x| x.contains("conveys a widget property")), "bare governing <p> kept");
        assert!(alpha.iter().any(|x| x.contains("Importance follows")), "bare governing <p> kept");
        assert!(!alpha.iter().any(|x| x.contains("all current engines")), "classed browser-support <p> dropped");
        assert!(!alpha.iter().any(|x| x.contains("gadget")), "foreign-linking sibling paragraph dropped");
        assert!(!alpha.iter().any(|x| x.contains("decorates the sample")), "post-example narration dropped");
    }

    #[test]
    fn w3schools_reader_reads_only_the_definition_region() {
        // Only the "Definition and Usage" region is governing: the worked example above it and the
        // support table below it are outside that region and never read. No construct meaning asserted.
        let body = r#"<div class="w3-example"><h3>Example</h3><p>An example alpha usage is shown here.</p></div>
            <hr><h2>Definition and Usage</h2>
            <p>The alpha construct conveys a widget property in documents.</p>
            <hr><h2>Browser Support</h2><p>Every browser supports the alpha construct fully.</p>"#;
        let ws = w3schools_witnesses("alpha", body);
        let s: Vec<&str> = ws.iter().map(|w| w.sentence.as_str()).collect();
        assert!(ws.iter().all(|w| w.subject == "alpha"), "subject is the page's own tag, alpha");
        assert!(s.iter().any(|x| x.contains("conveys a widget property")), "the definition region is read");
        assert!(!s.iter().any(|x| x.contains("example alpha usage")), "the pre-definition example region is excluded");
        assert!(!s.iter().any(|x| x.contains("browser supports")), "the post-definition support table is excluded");
    }

    #[test]
    fn chrome_recurring_across_subjects_is_discarded() {
        // A sentence recurring under two DISTINCT subjects is site furniture; one unique to a subject
        // is that construct's own governing prose and is kept.
        let ws = vec![
            KeyedWitness { subject: "alpha".into(), sentence: "This shared banner appears on every page.".into() },
            KeyedWitness { subject: "beta".into(), sentence: "This shared banner appears on every page.".into() },
            KeyedWitness { subject: "alpha".into(), sentence: "The alpha construct conveys a widget property.".into() },
        ];
        let kept = discard_chrome(ws);
        assert_eq!(kept.len(), 1, "the cross-subject chrome sentence is dropped");
        assert_eq!(kept[0].sentence, "The alpha construct conveys a widget property.");
    }

    #[test]
    fn subject_key_gate_admits_only_the_matching_construct() {
        // THE LEAK-GONE PROPERTY, abstractly: graduating construct `alpha` offers ONLY alpha-subject
        // witnesses to the frozen engine — beta/gamma-subject sentences never reach it, however their
        // English predicate would read. Brains-gated (the engine is defined only over the real bedrock);
        // skips honestly when no artifact is on disk. The count is purely the structural gate.
        let (Some(br), Some(en)) = (crate::lint_char::brain(), crate::lint_english::brain()) else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let m = br.meanings();
        let ws = vec![
            KeyedWitness { subject: "alpha".into(), sentence: "The construct conveys a widget property.".into() },
            KeyedWitness { subject: "beta".into(), sentence: "The construct marks a gadget region.".into() },
            KeyedWitness { subject: "gamma".into(), sentence: "The construct draws a doodad boundary.".into() },
        ];
        let (_verdict, offered) = graduate_construct(
            m,
            en,
            "alpha",
            "the construct conveys a widget property",
            "the construct draws a doodad boundary",
            &ws,
        );
        assert_eq!(offered, 1, "only the alpha-subject witness is offered — foreign subjects gated out");
    }

    #[test]
    fn attribute_badges_read_the_marked_dt_and_its_dd_prose() {
        // PASS 37 law 1 — the dt/dd definition-list shape: only the entry whose OWN markup
        // carries the prohibition status join attests; its dd is the governing prose; the
        // healthy sibling attests nothing. Names are opaque (no HTML knowledge asserted).
        let body = r#"<h1>&lt;alpha&gt;</h1><h2 id="attributes">Attributes</h2><dl>
            <dt id="blip">blip <abbr class="icon icon-deprecated" title="x"><span>Deprecated</span></abbr></dt>
            <dd><p>Never use the blip attribute on the alpha element here.</p></dd>
            <dt id="bloop">bloop</dt><dd><p>Sets the bloop level.</p></dd></dl>"#;
        let badges = attribute_badges("alpha", body);
        assert_eq!(badges.len(), 1, "only the marked entry attests: {badges:?}");
        assert_eq!(badges[0].0, "blip");
        assert!(badges[0].1.contains("Never use the blip attribute"), "dd prose is governing: {:?}", badges[0].1);
        // PASS 37 class 2 — the INTERFACE page's dotted member typography: the member half is
        // the attribute name where the owner half is the URL subject; a dt whose own text
        // writes CALL typography is a method, never an attribute; a foreign owner never joins.
        let iface = r#"<dl>
            <dt id="alphawidget.blip"><code>AlphaWidget.blip</code> <span class="icon icon-deprecated"></span></dt>
            <dd><p>A string of the blip alignment.</p></dd>
            <dt id="alphawidget.spin"><code>AlphaWidget.spin()</code> <span class="icon icon-deprecated"></span></dt>
            <dd><p>Spins the widget once.</p></dd>
            <dt id="otherthing.bloop"><code>OtherThing.bloop</code> <span class="icon icon-deprecated"></span></dt>
            <dd><p>A foreign owner's member.</p></dd></dl>"#;
        let iface_badges = attribute_badges("alphawidget", iface);
        assert_eq!(
            iface_badges.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
            vec!["blip"],
            "member half where owner==subject; call typography and foreign owners excluded"
        );
    }

    #[test]
    fn obsolete_index_entries_attest_only_the_prohibition_section() {
        // PASS 37 law 2a — the index shape: an anchored section whose prose states a
        // prohibition attests each dt-listed `&lt;x&gt;` element; the healthy section and the
        // lead attest nothing. Brains-gated (the prohibition read is the frozen English's).
        let (Some(en), Some(br)) = (crate::lint_english::brain(), crate::lint_char::brain()) else {
            eprintln!("skip: no brains on disk");
            return;
        };
        let m = br.meanings();
        let body = r#"<h1>Elements reference</h1><p>This page lists all the elements.</p>
            <h2 id="live_elements">Live elements</h2>
            <dl><dt><code>&lt;alpha&gt;</code></dt><dd>A live element here.</dd>
            <dt><code>&lt;beta&gt;</code></dt><dd>Another live element.</dd></dl>
            <h2 id="old_elements">Obsolete and deprecated elements</h2>
            <p>Never use these elements in new pages; they are obsolete now.</p>
            <dl><dt><code>&lt;omega&gt;</code></dt><dd>Never use the omega element anywhere.</dd>
            <dt><code>&lt;sigma&gt;</code></dt><dd>Replaced by the &lt;alpha&gt; element long ago.</dd></dl>"#;
        let entries = obsolete_index_entries(body, en, m);
        assert_eq!(
            entries.iter().map(|(e, _)| e.as_str()).collect::<Vec<_>>(),
            vec!["omega", "sigma"],
            "only the prohibition section's entries attest: {entries:?}"
        );
        assert!(entries[0].1.contains("omega element"), "dd prose is governing: {:?}", entries[0].1);
        assert!(
            entries[1].1.contains("Replaced by"),
            "a cross-reference inside a description never attests — it stays that entry's prose: {:?}",
            entries[1].1
        );
        // PASS 37 class 3 — the REAL index authors a TABLE: the repeating-entry law reads the
        // same truth off `<tr>` rows (no dt anywhere), and the description cell's own
        // cross-reference (`&lt;alpha&gt;` in the sigma row) still attests nothing.
        let table = r#"<h1>Elements reference</h1><p>This page lists all the elements.</p>
            <h2 id="old_elements">Obsolete and deprecated elements</h2>
            <p>Never use these elements in new pages; they are obsolete now.</p>
            <figure><table><thead><tr><th>Element</th><th>Description</th></tr></thead><tbody>
            <tr><td><a href="/x/omega"><code>&lt;omega&gt;</code></a></td><td>Never use the omega element anywhere.</td></tr>
            <tr><td><code>&lt;sigma&gt;</code></td><td>Replaced by the <a href="/x/alpha"><code>&lt;alpha&gt;</code></a> element long ago.</td></tr>
            </tbody></table></figure>"#;
        let rows = obsolete_index_entries(table, en, m);
        assert_eq!(
            rows.iter().map(|(e, _)| e.as_str()).collect::<Vec<_>>(),
            vec!["omega", "sigma"],
            "the table rows attest exactly the row-leading elements: {rows:?}"
        );
    }

    #[test]
    fn member_headed_section_abstains_the_element_roster() {
        // PASS 41 — the section-subject law: an MDN element page's "Deprecated attributes"
        // section authors bare code-name rows (`<dt><code>abbr</code></dt>`) IDENTICAL in shape
        // to WHATWG's obsolete-element rows; the head noun `attributes` means MEMBER, so the
        // roster read must ABSTAIN (Law 1's `host@attr` owns these) — the measured junk class
        // (`graded-abbr`/`graded-text`/`graded-width`) that fired on valid modern html. The
        // construct-headed section on the SAME page still yields its true elements.
        let (Some(en), Some(br)) = (crate::lint_english::brain(), crate::lint_char::brain()) else {
            eprintln!("skip: no brains on disk");
            return;
        };
        let m = br.meanings();
        let body = r#"<h1>&lt;td&gt;: The Table Data Cell element</h1><p>Docs for the cell.</p>
            <h2 id="deprecated_attributes">Deprecated attributes</h2>
            <p>The following attributes are deprecated; do not use them.</p>
            <dl><dt><code>abbr</code></dt><dd>Use the scope attribute instead.</dd>
            <dt><code>width</code></dt><dd>Use CSS width instead.</dd>
            <dt><code>axis</code></dt><dd>No longer supported.</dd></dl>
            <h2 id="deprecated_elements">Obsolete and deprecated elements</h2>
            <p>Never use these elements in new pages; they are obsolete now.</p>
            <dl><dt><code>&lt;omega&gt;</code></dt><dd>Never use the omega element anywhere.</dd>
            <dt><code>&lt;sigma&gt;</code></dt><dd>Never use the sigma element anywhere.</dd></dl>"#;
        let entries = obsolete_index_entries(body, en, m);
        assert_eq!(
            entries.iter().map(|(e, _)| e.as_str()).collect::<Vec<_>>(),
            vec!["omega", "sigma"],
            "the member-headed section abstains; only the construct-headed roster attests: {entries:?}"
        );
        assert!(
            !entries.iter().any(|(e, _)| ["abbr", "width", "axis"].contains(&e.as_str())),
            "no attribute name may attest as an element: {entries:?}"
        );
    }

    #[test]
    fn whatwg_code_typography_entries_attest_elements_and_exclude_attribute_rows() {
        // PASS 37 WHATWG re-land — the code-typography arm: a hyphenated status value joins the
        // heading by word SEQUENCE ("Non-conforming features"), element rows define a bare
        // code-font name alone, attribute rows ("charset on a elements") and closed-tag
        // omission (`<dt>` with no `</dt>`, `<dd>` following directly) are the real spec's own
        // shapes. Brains-gated (the head-noun read is the meaning net's).
        let (Some(en), Some(br)) = (crate::lint_english::brain(), crate::lint_char::brain()) else {
            eprintln!("skip: no brains on disk");
            return;
        };
        let m = br.meanings();
        let body = r#"<h1>Obsolete features</h1><p>The features described here are obsolete.</p>
            <h2 id=non-conforming-features>Non-conforming features</h2>
            <p>Elements in the following list are entirely obsolete, and must not be used by authors.</p>
            <dl><dt><dfn id=bgsound><code>bgsound</code></dfn><dd><p>Use audio instead.</p>
            <dt><dfn id=isindex><code>isindex</code></dfn><dd><p>Use an explicit form and text control combination instead.</p>
            <dt><code>charset on a</code><dd><p>Use an HTTP Content-Type header on the linked resource instead.</p>
            <dt><code>coords on a</code><dd><p>Use area instead of a for image maps.</p>
            <dt><code>shape on a</code><dd><p>Use area instead of a for image maps.</p>
            <dt><code>methods on a</code><dd><p>Use the HTTP OPTIONS feature instead.</p>
            <dt><dfn id=blorbel><code>blorbel</code></dfn><dt><dfn id=zintel><code>zintel</code></dfn><dd><p>Use appropriate modern elements instead.</p></dl>"#;
        let entries = obsolete_index_entries(body, en, m);
        assert_eq!(
            entries.iter().map(|(e, _)| e.as_str()).collect::<Vec<_>>(),
            vec!["bgsound", "isindex", "blorbel", "zintel"],
            "bare code-font element terms attest — including a SHARED-DESCRIPTION RUN (blorbel \
             + zintel share one dd, the list grammar's own shape) — while multi-word attribute \
             terms are excluded from element-hood yet count as definition structure, so the \
             attribute-majority list still yields its elements: {entries:?}"
        );
        let shared = entries.iter().find(|(e, _)| e == "blorbel").unwrap();
        assert!(
            shared.1.contains("appropriate modern elements"),
            "a run member attests with the run's shared dd as governing: {:?}",
            shared.1
        );
        assert!(entries[0].1.contains("Use audio instead"), "dd prose governs: {:?}", entries[0].1);
    }

    #[test]
    fn not_supported_subject_needs_notice_typography_and_url_subject() {
        // PASS 37 law 2b — the page-role notice: a lead sentence the frozen English reads as a
        // stated prohibition + the element's own `&lt;x&gt;` typography + the URL-subject law.
        // A healthy page (no notice) attests nothing. Brains-gated.
        let Some(en) = crate::lint_english::brain() else {
            eprintln!("skip: no english brain on disk");
            return;
        };
        // The REAL w3schools shape (PASS 37 production closure, class 4): the notice is a
        // STATUS-CLASSED HEADING (arm a), whose governed region names the subject in the
        // page's own typography.
        let body = "<h1>&lt;omega&gt; Tag</h1>\
             <h2><span class=\"deprecated\">Not Supported in HTML5.</span></h2>\
             <p>The &lt;omega&gt; tag was used to embed a gadget in a page.</p><hr>\
             <p>Something else entirely.</p>";
        let got = not_supported_subject("https://site/tags/tag_omega", body, en);
        let (element, governing) = got.expect("status heading + typography + url-subject attest");
        assert_eq!(element, "omega");
        assert!(governing.contains("Not Supported"), "the notice is cited: {governing:?}");
        assert!(governing.contains("omega"), "the naming sentence rides along: {governing:?}");
        // Healthy page: same shape, no notice — attests nothing.
        let healthy = "<h1>&lt;omega&gt; Tag</h1><p>The &lt;omega&gt; tag embeds a gadget.</p>";
        assert!(not_supported_subject("https://site/tags/tag_omega", healthy, en).is_none());
        // URL-subject law: the URL names a different subject — attests nothing.
        assert!(not_supported_subject("https://site/tags/tag_other", body, en).is_none());
        // THE SUBJECT-BINDING LAW (class 1, the tutorial-junk pin's src twin): PROSE never
        // attests page-scope — a lead prohibition sentence on a tutorial page that writes the
        // URL-subject's typography (even one naming the subject) binds nothing; only the
        // status-classed notice heading does (measured: the prose arm yielded only junk).
        let tutorial = "<h1>Omega Tutorial</h1>\
             <p>The &lt;omega&gt; tag structures a page.</p>\
             <p>Do not use hidden markers as a form of security.</p>\
             <p>Never use the omega tag in new pages.</p>";
        assert!(
            not_supported_subject("https://site/html/omega_intro", tutorial, en).is_none(),
            "prose sentences must attest nothing — page scope needs the status typography"
        );
    }

    #[test]
    fn interface_hosts_read_the_mapping_sentence_and_references_arm() {
        // PASS 37 class 2 — the interface-host law. Arm (a): the mapping sentence names the
        // URL-subject interface and writes its element(s) in `<x>` typography; a numeric span
        // derives the intermediates from the endpoints' own stem+numerals; typography OUTSIDE
        // a subject-naming sentence never hosts.
        let mapping = "<h1>AlphaWidget</h1>\
             <p>The <code>AlphaWidget</code> interface provides properties for manipulating \
             <code>&lt;alpha&gt;</code> elements.</p>\
             <p>See the unrelated <code>&lt;gamma&gt;</code> element elsewhere.</p>\
             <h2 id=\"props\">Properties</h2>";
        assert_eq!(interface_hosts("AlphaWidget", mapping), (vec!["alpha".to_string()], false));
        let span = "<h1>RungWidget</h1>\
             <p>The <code>RungWidget</code> interface represents the rung elements, \
             <code>&lt;r1&gt;</code> through <code>&lt;r4&gt;</code>.</p>\
             <h2 id=\"props\">Properties</h2>";
        assert_eq!(
            interface_hosts("RungWidget", span),
            (vec!["r1".to_string(), "r4".to_string(), "r2".to_string(), "r3".to_string()], false),
            "the endpoints' own stem+numerals derive the span"
        );
        // Arm (b): no sentence binds — a link wrapping `<x>` typography whose href's last
        // segment names x hosts; a link whose segment names something else never does.
        let refs = "<h1>MediaWidget</h1>\
             <p>The MediaWidget interface adds media capabilities.</p>\
             <h2 id=\"see_also\">See also</h2><ul>\
             <li><a href=\"/ref/elements/vid\"><code>&lt;vid&gt;</code></a> and \
             <a href=\"/ref/elements/aud\"><code>&lt;aud&gt;</code></a> elements</li>\
             <li><a href=\"/ref/elements/other\"><code>&lt;mismatch&gt;</code></a></li></ul>";
        assert_eq!(
            interface_hosts("MediaWidget", refs),
            (vec!["vid".to_string(), "aud".to_string()], true),
            "the references arm hosts only URL-subject-matching links, marked as an enumeration"
        );
    }

    #[test]
    fn construct_tag_detection() {
        assert_eq!(construct_tag("<beta>"), Some("beta"));
        assert_eq!(construct_tag("<alpha>"), Some("alpha"));
        assert_eq!(construct_tag("<beta"), None);
        assert_eq!(construct_tag("<a3>"), None);
        assert_eq!(construct_tag("gamma"), None);
    }
}
