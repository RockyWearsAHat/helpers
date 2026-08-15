//! `lint_labels` — PASS 39 phase one, item 1 (native/history.dx "PASS 39"): label export.
//!
//! Walks every cached documentation page this machine holds ([`crate::lint_docs::all_cached_pages`])
//! and emits grounded `(url, byte-span, label)` triples from facts the FROZEN PASS 35-37 read arms
//! already prove:
//!
//! - **Code** / **governing prose** — [`crate::lint_graph::read_page`], the char-substrate unit
//!   former: a unit's code span and the raw span its governing prose was sliced from.
//! - **Chrome** — [`crate::lint_html_layer::sections`] + [`crate::lint_html_layer::is_chrome_anchor`],
//!   the module's own `NON_GOVERNING_ANCHORS` list (demo widgets, spec tables, see-also).
//! - **Status marker** — [`crate::lint_html_layer::status_marker_spans`], PASS 37's
//!   prohibition-status predicate.
//! - **Term** — the page's own URL-subject element written in its own markup (PASS 35's element
//!   typography, `<name`/`&lt;name&gt;`), literal occurrences only.
//!
//! No new judgment is made here — every label is an offset into a span some other frozen
//! function already decided; this module only walks the corpus and records where. Persisted as
//! the HLM1 sidecar `labels.html.bin` ([`crate::lint_codec::kind::LABELS`]) — the training data
//! for the one-bit predictive reader ([`crate::lint_coder`]).
//!
//! SHADOW ONLY (THE NO-NEW-READER-CODE LAW, native/laws.dx): nothing here mints a finding or touches
//! the live lint path.

use crate::lint_codec::{Bin, Dec, Enc};

/// The register one byte-span of one page is grounded to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LabelKind {
    /// A code example span (`PageUnit::at..PageUnit::end`).
    Code = 0,
    /// The prose sentences governing an adjacent code span (`PageUnit::prose_from..PageUnit::at`).
    GoverningProse = 1,
    /// A stable page-furniture section (demo widgets, spec tables, see-also).
    Chrome = 2,
    /// A heading whose own markup carries a prohibition status token.
    StatusMarker = 3,
    /// The page's own URL-subject element written in its own markup.
    Term = 4,
}

impl LabelKind {
    fn from_u8(v: u8) -> Option<LabelKind> {
        Some(match v {
            0 => LabelKind::Code,
            1 => LabelKind::GoverningProse,
            2 => LabelKind::Chrome,
            3 => LabelKind::StatusMarker,
            4 => LabelKind::Term,
            _ => return None,
        })
    }
}

/// One grounded training label: a raw byte span `[start, end)` in one crawled page's body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Label {
    pub url: String,
    pub start: u32,
    pub end: u32,
    pub kind: LabelKind,
}

impl Bin for Label {
    fn enc(&self, e: &mut Enc) {
        e.str(&self.url);
        e.u(self.start as u64);
        e.u(self.end as u64);
        e.u(self.kind as u64);
    }
    fn dec(d: &mut Dec) -> Option<Label> {
        let url = d.str()?;
        let start = d.u()? as u32;
        let end = d.u()? as u32;
        let kind = LabelKind::from_u8(d.u()? as u8)?;
        Some(Label { url, start, end, kind })
    }
}

/// Per-kind tallies from one export run — what the train ack reports (native/history.dx "PASS 39").
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct LabelCounts {
    pub pages: usize,
    pub code: usize,
    pub governing_prose: usize,
    pub chrome: usize,
    pub status_marker: usize,
    pub term: usize,
}

impl LabelCounts {
    pub(crate) fn total(&self) -> usize {
        self.code + self.governing_prose + self.chrome + self.status_marker + self.term
    }
}

/// The element this page's OWN URL names as its subject — the last non-empty path segment,
/// lowercased (PASS 35's URL-subject law; the same test
/// [`crate::lint_html_layer::not_supported_subject`] applies before ever reading the page).
/// `None` for a root/query-only URL or a segment that is not plain typography (a UUID, a hash).
fn url_subject(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next().unwrap_or(url).trim_end_matches('/');
    let seg = path.rsplit('/').next()?;
    // No minimum length: real single-letter elements exist (`<a>`, `<b>`, `<i>`).
    if seg.is_empty() || !seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return None;
    }
    Some(seg.to_ascii_lowercase())
}

/// Every literal occurrence of `name` written in the page's own ELEMENT typography (PASS 35:
/// `<name` tag-open, or its tag-stripped rendering `&lt;name&gt;`) — whole-token, case-
/// insensitive byte spans. A span-FINDER, not a judgment: the true/false call ("is this an
/// element construct") is PASS 35's own frozen law; this only locates where the page already
/// writes it.
fn element_typography_spans(body: &str, name: &str) -> Vec<(usize, usize)> {
    let lower = body.to_ascii_lowercase();
    let mut out = Vec::new();
    for pat in [format!("<{name}"), format!("&lt;{name}&gt;")] {
        let plen = pat.len();
        let mut at = 0usize;
        while let Some(rel) = lower[at..].find(pat.as_str()) {
            let start = at + rel;
            let end = (start + plen).min(lower.len());
            let boundary =
                lower.as_bytes().get(end).map(|b| !b.is_ascii_alphanumeric()).unwrap_or(true);
            if boundary {
                out.push((start, end));
            }
            at = start + 1;
        }
    }
    out
}

/// Walk the whole cached documentation corpus and emit every grounded label the frozen arms
/// prove. A page qualifies as "HTML" by the SAME test [`crate::lint_docs::read_crawled_page`]
/// already applies (`body.contains("</")`) — non-HTML docs (Markdown, plain text) are outside
/// this pass's scope and skipped. Requires a trained char-substrate brain
/// ([`crate::lint_char::brain`]) for the code/governing-prose facts; chrome, status-marker, and
/// term facts need no brain and still export when one is absent.
#[cfg(feature = "crawl")]
pub(crate) fn export_labels() -> (Vec<Label>, LabelCounts) {
    let mut labels = Vec::new();
    let mut counts = LabelCounts::default();
    let brain = crate::lint_char::brain();
    for (url, body) in crate::lint_docs::all_cached_pages() {
        if !body.contains("</") {
            continue;
        }
        counts.pages += 1;
        let dropped = crate::doc_crawler::drop_script_style(&body);

        if let Some(brain) = brain {
            for u in crate::lint_graph::read_page(&dropped, brain) {
                let end = u.end.min(dropped.len());
                if end > u.at {
                    labels.push(Label { url: url.clone(), start: u.at as u32, end: end as u32, kind: LabelKind::Code });
                    counts.code += 1;
                }
                if u.prose_from < u.at {
                    labels.push(Label {
                        url: url.clone(),
                        start: u.prose_from as u32,
                        end: u.at as u32,
                        kind: LabelKind::GoverningProse,
                    });
                    counts.governing_prose += 1;
                }
            }
        }

        let mut off = 0usize;
        for (anchor, region) in crate::lint_html_layer::sections(&dropped) {
            let end = off + region.len();
            if crate::lint_html_layer::is_chrome_anchor(&anchor) {
                labels.push(Label { url: url.clone(), start: off as u32, end: end as u32, kind: LabelKind::Chrome });
                counts.chrome += 1;
            }
            off = end;
        }

        for (start, end) in crate::lint_html_layer::status_marker_spans(&dropped) {
            labels.push(Label { url: url.clone(), start: start as u32, end: end as u32, kind: LabelKind::StatusMarker });
            counts.status_marker += 1;
        }

        if let Some(name) = url_subject(&url) {
            for (start, end) in element_typography_spans(&dropped, &name) {
                labels.push(Label { url: url.clone(), start: start as u32, end: end as u32, kind: LabelKind::Term });
                counts.term += 1;
            }
        }
    }
    (labels, counts)
}

/// The machine-global label sidecar's path (`labels.html.bin`, beside the per-language models).
fn labels_path() -> std::path::PathBuf {
    crate::lint_train::model_dir_pub().join("labels.html.bin")
}

/// Persist `labels` as the HLM1 sidecar, stamped with the current train version (native/architecture.dx
/// "PASS 39"). A logic change to the export bumps [`crate::lint_train::TRAIN_VERSION`] like
/// every other artifact, invalidating stale labels honestly rather than silently.
pub(crate) fn save_labels(labels: &[Label]) {
    let mut e = Enc::new();
    e.u(labels.len() as u64);
    for l in labels {
        l.enc(&mut e);
    }
    let path = labels_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, e.finish(crate::lint_codec::kind::LABELS, crate::lint_train::TRAIN_VERSION));
}

/// Load the persisted label sidecar, or `None` when absent/unreadable.
pub(crate) fn load_labels() -> Option<Vec<Label>> {
    let bytes = std::fs::read(labels_path()).ok()?;
    let (_, mut d) = Dec::open(&bytes, crate::lint_codec::kind::LABELS)?;
    let n = d.u()? as usize;
    let mut out = Vec::with_capacity(n.min(1_000_000));
    for _ in 0..n {
        out.push(Label::dec(&mut d)?);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_subject_reads_last_path_segment() {
        assert_eq!(url_subject("https://developer.mozilla.org/en-US/docs/Web/HTML/Element/button"), Some("button".to_string()));
        assert_eq!(url_subject("https://site.example/"), None);
        assert_eq!(url_subject("https://site.example/docs/a?x=1"), Some("a".to_string()));
    }

    #[test]
    fn element_typography_spans_finds_tag_open_and_escaped_form() {
        let body = "<p>Use &lt;button&gt; not <buttongroup>. <button type=\"submit\">";
        let spans = element_typography_spans(body, "button");
        // The escaped reference, the real tag-open — but never inside `<buttongroup>` (boundary guard).
        assert!(spans.iter().any(|&(s, e)| &body[s..e] == "&lt;button&gt;"));
        assert!(spans.iter().any(|&(s, e)| &body[s..e] == "<button"));
        assert!(!spans.iter().any(|&(s, _)| body[s..].starts_with("<buttongroup")));
    }

    #[test]
    fn save_and_load_round_trip_through_the_sidecar_file() {
        let _env = crate::test_env_lock();
        let dir = std::env::temp_dir().join(format!("lint_labels_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::env::set_var("HELPERS_LINT_MODELS", &dir);
        let labels = vec![
            Label { url: "https://x/y".into(), start: 1, end: 5, kind: LabelKind::Chrome },
            Label { url: "https://x/z".into(), start: 10, end: 40, kind: LabelKind::Code },
        ];
        save_labels(&labels);
        let loaded = load_labels().expect("loads");
        std::env::remove_var("HELPERS_LINT_MODELS");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(loaded, labels);
    }

    #[test]
    fn label_round_trips_through_bin() {
        let l = Label { url: "https://x/y".into(), start: 3, end: 9, kind: LabelKind::StatusMarker };
        let mut e = Enc::new();
        l.enc(&mut e);
        let bytes = e.finish(crate::lint_codec::kind::LABELS, "t");
        let (_, mut d) = Dec::open(&bytes, crate::lint_codec::kind::LABELS).expect("opens");
        let back = Label::dec(&mut d).expect("decodes");
        assert_eq!(back, l);
    }
}
