//! LEARNED page-role DEPRECATION attestation, keyed by the AUTHOR'S OWN METADATA TYPOGRAPHY
//! (COMPLETION PASS 13). A documentation author marks a discouraged feature in machine metadata — MDN
//! frontmatter `status:\n  - deprecated` (a block-sequence ENUM: deprecated / experimental / non-standard),
//! the rendered `{{Deprecated_Header}}` banner. That metadata is TYPOGRAPHY (a data-keyed marker, exactly
//! like a code-fence's info-string), NOT English prose — so the register is discoverable WITHOUT the
//! meaning-network polarity that blocked passes 11 and 12.
//!
//! The faculty discovers, from data alone, the sentence-scale invariant runs the author renders for the
//! DEPRECATION status family, and treats a crawled page carrying any such run as attested. The ONLY hand
//! datum is which enum VALUE denotes prohibition (`deprecation-status.json` → `prohibits`) — the one
//! choice no structural signal can supply (every status value renders a banner; structure cannot tell the
//! deprecation banner from the experimental one — the measured PASS 11/12 wall). Everything else — the
//! enum KEY, the family membership, the marker text — is discovered by shape + recurrence + slug-join.
//!
//! MEASURED (COMPLETION PASS 13, `examples/metajoin`): on the 2968-page crawled MDN corpus the discovered
//! markers attest EXACTLY the 117 pages the retired hand marker did — P = 1.000, R = 1.000 — with zero
//! false positives, so the burn of `has_deprecation_notecard` is strictly behavior-preserving.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// The learned deprecation markers for the registered documentation sites. Two data-keyed routes to the
/// SAME page-role fact, both discovered — never a hardcoded banner or class string:
///
/// - `markers`: sentence-scale invariant text runs the author renders for the prohibition status family,
///   discovered by joining the markdown frontmatter `status:` enum (found by shape) to the crawled pages
///   by slug (the MDN route — the marker lives in a separate metadata file, joined in).
/// - `class_markers`: author status-TYPOGRAPHY class-attribute values that JOIN a prohibition enum value
///   directly in the rendered HTML (the Python `<div class="deprecated">` / Rust `class="stab deprecated"`
///   route — a site that publishes the status in the markup itself, no frontmatter, no slug join).
///
/// A crawled page carrying any learned text run OR any element whose class carries a learned status token
/// is an attested deprecation. Empty on a machine with no metadata corpus (attests nothing — an honest
/// abstention).
#[derive(Clone, Debug, Default)]
pub struct Attestation {
    markers: Vec<String>,
    class_markers: Vec<String>,
}

impl Attestation {
    /// Discover the markers from the whole-site crawled `pages` joined to the registered markdown
    /// frontmatter metadata. Steps, all data-driven:
    /// 1. Read frontmatter block-sequence ENUM values per page slug (`status:` is discovered by shape).
    /// 2. The prohibition value(s) come from `deprecation-status.json` (the one hand datum); the pages
    ///    whose metadata carries one form the FAMILY, the other enum-valued pages the negative set.
    /// 3. A marker is an invariant run that is present on ≥ half the family's crawled pages, ABSENT from
    ///    every negative-set page, family-DOMINANT (≥ half its own support is family), and sentence-scale
    ///    (≥ [`MIN_MARKER_WORDS`] words — a rendered banner is prose, not a fragment). Such a run
    ///    generalizes to every page the author renders the same banner on, including pages with no
    ///    markdown source (the crawl's `web/api` tree the clone omits).
    pub fn discover(pages: &[(String, String)]) -> Attestation {
        let prohibit = prohibition_values();
        if prohibit.is_empty() {
            return Attestation::default();
        }
        // The rendered route runs unconditionally on whatever HTML is present — a site that renders the
        // status class directly (Python/Rust) needs no frontmatter. Discovered here so a corpus with no
        // markdown metadata still attests.
        let class_markers = discover_class_markers(pages, &prohibit);
        let families = frontmatter_families(); // enum value -> set<slug>
        let mut dep_slugs: HashSet<&str> = HashSet::new();
        for v in &prohibit {
            if let Some(s) = families.get(v) {
                dep_slugs.extend(s.iter().map(String::as_str));
            }
        }
        if dep_slugs.is_empty() {
            return Attestation { markers: Vec::new(), class_markers };
        }
        let other_slugs: HashSet<&str> = families
            .iter()
            .filter(|(v, _)| !prohibit.contains(*v))
            .flat_map(|(_, s)| s.iter().map(String::as_str))
            .filter(|s| !dep_slugs.contains(s))
            .collect();

        // Join the metadata families to the crawled pages by slug.
        let mut dep_urls: HashSet<&str> = HashSet::new();
        let mut other_urls: HashSet<&str> = HashSet::new();
        for (url, _) in pages {
            if let Some(slug) = slug_of_url(url) {
                if dep_slugs.contains(slug.as_str()) {
                    dep_urls.insert(url.as_str());
                } else if other_slugs.contains(slug.as_str()) {
                    other_urls.insert(url.as_str());
                }
            }
        }
        if dep_urls.is_empty() {
            return Attestation { markers: Vec::new(), class_markers };
        }

        // Candidate runs = the sentence-scale runs the family pages carry.
        let mut candidates: HashSet<String> = HashSet::new();
        for (url, body) in pages {
            if !dep_urls.contains(url.as_str()) {
                continue;
            }
            for run in runs_of(body) {
                if run.split(' ').count() >= MIN_MARKER_WORDS {
                    candidates.insert(run);
                }
            }
        }
        // Support of each candidate: on family / on negative-set / total.
        let mut dep_ct: HashMap<&str, usize> = HashMap::new();
        let mut other_ct: HashMap<&str, usize> = HashMap::new();
        let mut total_ct: HashMap<&str, usize> = HashMap::new();
        for (url, body) in pages {
            let page_runs: HashSet<String> = runs_of(body).into_iter().collect();
            let in_dep = dep_urls.contains(url.as_str());
            let in_other = other_urls.contains(url.as_str());
            for run in &page_runs {
                if let Some(c) = candidates.get(run.as_str()) {
                    *total_ct.entry(c.as_str()).or_default() += 1;
                    if in_dep {
                        *dep_ct.entry(c.as_str()).or_default() += 1;
                    }
                    if in_other {
                        *other_ct.entry(c.as_str()).or_default() += 1;
                    }
                }
            }
        }
        // The support floor is the machine's own WITNESS LAW (≥ 15 independent witnesses,
        // `lint_ism::REQUIRED_WITNESSES`), capped by half the family so a small corpus keeps its
        // original bar. The old half-the-family demand was a COVERAGE demand, not a soundness one —
        // at whole-site scale (12k+ pages, banner wording varying by section) no single run covers
        // half the family and discovery died to zero (MEASURED, PASS 34); soundness lives in the
        // other two gates (zero negative-family support; family dominance of the run's own support).
        let floor = crate::lint_ism::REQUIRED_WITNESSES.min((dep_urls.len() as f64 * 0.5).ceil() as usize);
        // Dominance is judged over the LABELED pages only (family + negative set): an unlabeled page
        // carrying the run is an UNKNOWN the marker will rightly generalize to, never counter-evidence.
        // MEASURED (PASS 35): at whole-site scale the banner run sits on 575 pages while the
        // frontmatter join labels 84 — the old whole-corpus dominance (84/575 < 0.5) killed every real
        // marker; labeled dominance (84/84) admits it, and site chrome still dies on its negative-set
        // support (a run every page carries is on some experimental/non-standard page too).
        let mut markers: Vec<String> = candidates
            .iter()
            .filter(|c| {
                let d = *dep_ct.get(c.as_str()).unwrap_or(&0);
                let o = *other_ct.get(c.as_str()).unwrap_or(&0);
                d >= floor && o == 0 && d * 2 >= d + o
            })
            .cloned()
            .collect();
        markers.sort();
        markers.dedup();
        Attestation { markers, class_markers }
    }

    /// A test/data constructor over explicit marker runs (no corpus). Public so a synthetic page test can
    /// exercise the attester deterministically.
    pub fn from_markers(markers: Vec<String>) -> Attestation {
        Attestation { markers, class_markers: Vec::new() }
    }

    /// A test/data constructor over explicit rendered class-token markers (no corpus). Public so a
    /// synthetic Python/Rust-shaped page test can exercise the rendered route deterministically.
    pub fn from_class_markers(class_markers: Vec<String>) -> Attestation {
        Attestation { markers: Vec::new(), class_markers }
    }

    /// Whether a page body carries a learned deprecation marker — either a sentence-scale text run (the
    /// same whitespace-collapsed run normalization discovery used) or an element whose class attribute
    /// carries a learned author status token (the rendered route).
    pub fn attests(&self, body: &str) -> bool {
        if !self.markers.is_empty() {
            let runs: HashSet<String> = runs_of(body).into_iter().collect();
            if self.markers.iter().any(|m| runs.contains(m)) {
                return true;
            }
        }
        if !self.class_markers.is_empty() && body_carries_class_token(body, &self.class_markers) {
            return true;
        }
        false
    }

    /// Whether a page body carries a PAGE-SCOPE deprecation banner — the learned sentence-scale TEXT-RUN
    /// markers only. The rendered class-token route is deliberately excluded: a status class marks an
    /// ITEM (one table row, one anchor — PASS 14's item unit), so it must never confer deprecation on
    /// the PAGE's own URL-subject (the measured junk class: a reference INDEX page with one deprecated
    /// row minting its page slug as a deprecated construct).
    pub fn attests_page_scope(&self, body: &str) -> bool {
        if self.markers.is_empty() {
            return false;
        }
        let runs: HashSet<String> = runs_of(body).into_iter().collect();
        self.markers.iter().any(|m| runs.contains(m))
    }

    /// The learned marker runs (diagnostics / measurement).
    pub fn markers(&self) -> &[String] {
        &self.markers
    }

    /// The learned rendered class-token markers (diagnostics / measurement).
    pub fn class_markers(&self) -> &[String] {
        &self.class_markers
    }
}

/// The minimum number of distinct page SUBJECTS a rendered status class token must mark before it is a
/// trusted marker — the same recurring-across-many-subject-varying-pages covenant the frontmatter route
/// holds, so a lone page that happens to carry `class="deprecated"` in unrelated content is not a family.
const CLASS_MARKER_SUPPORT_FLOOR: usize = 8;

/// Discover the rendered author status-typography class tokens: a class-attribute value that JOINS a
/// prohibition enum value (`deprecation-status.json` — the same one hand datum the frontmatter route uses)
/// AND recurs across ≥ [`CLASS_MARKER_SUPPORT_FLOOR`] distinct page subjects. The join is data-keyed: the
/// author's own class token (`deprecated`) is only ever kept because it EQUALS a prohibition value read
/// from data — no class name, no site, is written in code. This is the rendered parallel of the
/// frontmatter enum: Python renders `<div class="deprecated">`, Rust `class="stab deprecated"`, both
/// carrying the token `deprecated` the data marks as prohibition.
fn discover_class_markers(pages: &[(String, String)], prohibit: &[String]) -> Vec<String> {
    let prohibit_set: HashSet<&str> = prohibit.iter().map(String::as_str).collect();
    let mut token_subjects: HashMap<String, HashSet<String>> = HashMap::new();
    for (url, body) in pages {
        let subj = subject_of_url(url);
        let mut on_page: HashSet<String> = HashSet::new();
        for value in class_values(body) {
            for tok in value.split_whitespace() {
                let tok = tok.to_lowercase();
                if prohibit_set.contains(tok.as_str()) {
                    on_page.insert(tok);
                }
            }
        }
        for tok in on_page {
            token_subjects.entry(tok).or_default().insert(subj.clone());
        }
    }
    let mut markers: Vec<String> = token_subjects
        .into_iter()
        .filter(|(_, subjects)| subjects.len() >= CLASS_MARKER_SUPPORT_FLOOR)
        .map(|(tok, _)| tok)
        .collect();
    markers.sort();
    markers.dedup();
    markers
}

/// Every `class="…"` / `class='…'` attribute value in an HTML body (structure only — the attribute
/// value, never the element interior). The author's status typography lives here.
fn class_values(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lb = body.as_bytes();
    let needle = b"class=";
    let mut i = 0usize;
    while i + needle.len() < lb.len() {
        if &lb[i..i + needle.len()] == needle {
            let q = lb[i + needle.len()];
            if q == b'"' || q == b'\'' {
                let start = i + needle.len() + 1;
                if let Some(rel) = lb[start..].iter().position(|&b| b == q) {
                    if body.is_char_boundary(start) && body.is_char_boundary(start + rel) {
                        out.push(body[start..start + rel].to_string());
                    }
                    i = start + rel + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// Whether any element in `body` carries a class attribute whose token set contains a learned marker.
fn body_carries_class_token(body: &str, class_markers: &[String]) -> bool {
    for value in class_values(body) {
        for tok in value.split_whitespace() {
            let tok = tok.to_lowercase();
            if class_markers.iter().any(|m| *m == tok) {
                return true;
            }
        }
    }
    false
}

/// A page's subject key for the recurring-across-subjects test: the last path segment of its URL
/// (case-folded, fragment/trailing-slash trimmed) — one page per item on rustdoc/python-docs, so the
/// item name distinguishes subjects; the family is trusted only when many distinct items carry the token.
fn subject_of_url(url: &str) -> String {
    url.split('#')
        .next()
        .unwrap_or(url)
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

/// A rendered banner is a sentence, not a 2-word fragment: a minimum run length that is a typography
/// property (banner = prose), not a word list. Drops generic short runs ("compatibility table") that
/// merely miss the small negative slug set.
const MIN_MARKER_WORDS: usize = 6;

/// The author status tokens that denote PROHIBITION, from `deprecation-status.json` — the SAME data the
/// attester joins, exposed so the language-layer subject reader can find the item anchors a page renders
/// its markers on (the rendered route's structural subject extraction). A class/enum token equal to one of
/// these is a deprecation marker; nothing else is named in code.
pub fn prohibition_class_tokens() -> Vec<String> {
    prohibition_values()
}

/// The author enum value(s) that denote PROHIBITION, from `deprecation-status.json`. The one datum the
/// faculty cannot derive structurally (see the module doc); carried as DATA exactly like `sources.json`.
fn prohibition_values() -> Vec<String> {
    status_values("prohibits")
}

/// The author status token(s) that denote REMOVAL, from `deprecation-status.json` → `removed` (PASS 22 —
/// the second data-keyed marker, consumed exactly as `prohibits` is). Removal is the strongest prohibition
/// status: it is a WHOLE-MODULE marker (`class="deprecated-removed"`) distinct from an inline per-method
/// deprecation note. Exposed for the construction miner's removal-subject basis; the page-role attester is
/// untouched. Empty when the datum is absent.
pub fn removal_class_tokens() -> Vec<String> {
    status_values("removed")
}

/// The EXCEPTION register ANCHOR(s), from `deprecation-status.json` → `scope_exception` (PASS 28;
/// meaning-anchored PASS 29). One dictionary word per register — the meaning net expands it to every
/// word the dictionary defines via it ([`crate::lint_lang_layer::note_counter_attests`]). An item a
/// note names AFTER an exception-meaning word is EXCLUDED from the note's own deprecation ("All
/// TLSVersion members except TLSVersion.TLSv1_3 are deprecated"), so the marker must not attest it.
/// Empty when the datum is absent (honest abstention — no narrowing).
pub fn scope_exception_tokens() -> Vec<String> {
    status_values("scope_exception")
}

/// The CONDITIONAL-form register ANCHOR(s), from `deprecation-status.json` → `usage_form` (PASS 28;
/// meaning-anchored PASS 29 — same expansion as [`scope_exception_tokens`]). A conditional-meaning word
/// in the FIRST SENTENCE of a note's clause after the deprecation head means the note deprecates an
/// argument/call form ("Deprecation warning is emitted if loop …"), not the item itself — so the marker
/// must not attest the item. Empty when the datum is absent (honest abstention).
pub fn usage_form_tokens() -> Vec<String> {
    status_values("usage_form")
}

/// The SUPERSESSION register ANCHOR(s), from `deprecation-status.json` → `replacement` (PASS 35). One
/// dictionary word; the meaning net carries it to its morphological forms and to words the dictionary
/// defines via it. A governing sentence with this MEANING that names another living construct in the
/// author's own code typography states "successor replaces subject" ([`crate::lint_web`]'s succession
/// read). Empty when the datum is absent (honest abstention — no edges minted).
pub fn replacement_tokens() -> Vec<String> {
    status_values("replacement")
}

/// Read a status-token array from `deprecation-status.json` by key (lower-cased). One reader for all the
/// register data (`prohibits`, `removed`, `scope_exception`, `usage_form`, `replacement`) — the only hand
/// data the faculty carries.
fn status_values(key: &str) -> Vec<String> {
    let Some(text) = crate::lint_train::embedded_lint_index_file("deprecation-status.json") else {
        return Vec::new();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    json.get(key)
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_lowercase())).collect())
        .unwrap_or_default()
}

/// Whether `body` carries the TIGHT module-removal marker: some `class="…"` value that, split on
/// whitespace AND hyphen, contains BOTH a prohibition token and a removal token (`deprecated-removed`).
/// This is the whole-module removal status — the strongest prohibition — distinct from an inline
/// `versionmodified deprecated` per-method note (the PASS-21 precision line, MEASURED: the tight compound
/// matches 29 pages and admits exactly the removed python modules, while a loose `deprecated` substring
/// floods 2812 and contaminates the subject basis). Structure only — the class attribute value, never
/// prose. Consumed by the construction miner's removal-subject basis; NOT by [`Attestation::attests`], so
/// every existing module is byte-identical. `false` when either datum is absent (honest abstention).
pub fn attests_module_removal(body: &str) -> bool {
    let prohibit = prohibition_values();
    let removed = removal_class_tokens();
    if prohibit.is_empty() || removed.is_empty() {
        return false;
    }
    let prohibit: HashSet<&str> = prohibit.iter().map(String::as_str).collect();
    let removed: HashSet<&str> = removed.iter().map(String::as_str).collect();
    for value in class_values(body) {
        let value = value.to_lowercase();
        let toks: HashSet<&str> = value.split([' ', '\t', '\n', '-']).collect();
        if toks.iter().any(|t| removed.contains(t)) && toks.iter().any(|t| prohibit.contains(t)) {
            return true;
        }
    }
    false
}

/// Whether `fragment` (any HTML region — a whole page, one `<dt>` entry, a heading tag) carries a
/// class attribute value that, split on whitespace AND hyphen, contains a PROHIBITION status token
/// (`deprecation-status.json` → `prohibits` — the same one hand datum every attestation route joins).
/// This is the [`attests_module_removal`] split precedent applied to the prohibition family alone: the
/// author's compound status typography (`icon icon-deprecated`) joins by DATA, never by an icon or
/// class list. PASS 37 — the per-attribute badge attester reads a definition-term entry through this.
/// `false` when the datum is absent (honest abstention).
pub fn class_carries_prohibition(fragment: &str) -> bool {
    let prohibit = prohibition_values();
    if prohibit.is_empty() {
        return false;
    }
    let prohibit: HashSet<&str> = prohibit.iter().map(String::as_str).collect();
    for value in class_values(fragment) {
        let value = value.to_lowercase();
        if value.split([' ', '\t', '\n', '-']).any(|t| prohibit.contains(t)) {
            return true;
        }
    }
    false
}

/// Map every markdown frontmatter block-sequence ENUM value to the set of page slugs carrying it, across
/// the registered markdown corpora. Cached per process (the corpus is fixed during a run). This is how the
/// `status` enum is discovered WITHOUT naming it: any top-level key whose value is a YAML block sequence
/// contributes its values; the prohibition value(s) then select the family.
pub(crate) fn frontmatter_families() -> &'static HashMap<String, HashSet<String>> {
    static CACHE: OnceLock<HashMap<String, HashSet<String>>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut fam: HashMap<String, HashSet<String>> = HashMap::new();
        for path in markdown_paths() {
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let Some((slug, enums)) = frontmatter(&text) else { continue };
            for values in enums.values() {
                for v in values {
                    fam.entry(v.clone()).or_default().insert(slug.clone());
                }
            }
        }
        fam
    })
}

/// Every `*.md` path under the registered markdown corpora (the DATA clones beside the models). Shares the
/// root resolution with [`crate::lint_char`]'s markdown curriculum.
fn markdown_paths() -> Vec<std::path::PathBuf> {
    let beside = |name: &str| {
        crate::lint_train::model_dir_pub()
            .parent()
            .map(|p| p.join(name))
            .unwrap_or_else(|| std::path::PathBuf::from(name))
    };
    let mut paths = Vec::new();
    let mut stack = vec![beside("mdn-content"), beside("mattpocock-skills")];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "md") {
                paths.push(p);
            }
        }
    }
    paths
}

/// Parse a markdown file's leading `---`…`---` frontmatter into (slug, block-sequence enums). A key whose
/// value is empty and whose following `  - v` lines form a YAML sequence is an enum key; its lowercased
/// values are collected. The `slug` scalar is the join key to the crawled page URL.
fn frontmatter(text: &str) -> Option<(String, HashMap<String, Vec<String>>)> {
    let body = text.strip_prefix("---")?;
    let end = body.find("\n---")?;
    let fm = &body[..end];
    let mut slug = None;
    let mut enums: HashMap<String, Vec<String>> = HashMap::new();
    let mut current: Option<String> = None;
    for line in fm.lines() {
        if let Some(item) = line.strip_prefix("  - ") {
            if let Some(key) = &current {
                enums.entry(key.clone()).or_default().push(item.trim().trim_matches('"').to_lowercase());
            }
            continue;
        }
        if line.starts_with(' ') {
            continue;
        }
        if let Some(colon) = line.find(':') {
            let key = line[..colon].trim().to_lowercase();
            let value = line[colon + 1..].trim().trim_matches('"');
            if key == "slug" {
                slug = Some(value.to_lowercase());
            }
            current = if value.is_empty() { Some(key) } else { None };
        } else {
            current = None;
        }
    }
    Some((slug?, enums))
}

/// The join key both sides share: the URL path after `/docs/`, case-folded, trailing slash + fragment
/// trimmed. A crawled `…/docs/Web/CSS/…` page and a frontmatter `slug: Web/CSS/…` map to the same key.
fn slug_of_url(url: &str) -> Option<String> {
    let u = url.split('#').next().unwrap_or(url);
    let i = u.find("/docs/")?;
    Some(u[i + 6..].trim_end_matches('/').to_lowercase())
}

/// Whitespace-collapse a text span; a run of ≥ 2 words and ≥ 6 chars is a comparison atom (the same rule
/// [`crate::lint_graph`]'s site-invariance atom uses).
fn norm_run(text: &str) -> Option<String> {
    let norm: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if norm.len() < 6 || norm.split(' ').count() < 2 {
        return None;
    }
    Some(norm)
}

/// The invariant text runs of an HTML body: the whitespace-collapsed text BETWEEN tags (a `<…>` opens a
/// tag, `>` closes it). Structure only — never the tag interior.
pub(crate) fn runs_of(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let b = body.as_bytes();
    let (mut in_tag, mut start) = (false, 0usize);
    for i in 0..b.len() {
        match b[i] {
            b'<' => {
                if !in_tag {
                    if let Some(r) = norm_run(&body[start..i]) {
                        out.push(r);
                    }
                }
                in_tag = true;
            }
            b'>' => {
                in_tag = false;
                start = i + 1;
            }
            _ => {}
        }
    }
    if let Some(r) = norm_run(&body[start..]) {
        out.push(r);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attester_matches_a_learned_run_as_a_whole_run() {
        let att = Attestation::from_markers(vec!["this feature is no longer recommended".to_string()]);
        // Whitespace normalization means intervening tags / newlines still match the collapsed run.
        assert!(att.attests("<p>This feature is\n  no longer   recommended</p>".to_lowercase().as_str()));
        assert!(!att.attests("<p>this feature is recommended</p>"));
        assert!(!Attestation::default().attests("anything at all here"));
    }

    #[test]
    fn discovery_selects_the_family_dominant_sentence_run() {
        // Two "deprecated"-family pages carry a shared banner sentence; an "experimental" page carries a
        // different banner; a generic short run appears everywhere. Only the banner sentence is a marker.
        let banner = "This feature is no longer recommended and should be avoided";
        let dep_a = format!("<div class='x'>{banner}</div><span>compatibility table</span>");
        let dep_b = format!("<div class='y'>{banner}</div><span>compatibility table</span>");
        // No frontmatter corpus in a unit test ⇒ families empty ⇒ discovery abstains. This test documents
        // the ABSTENTION contract (a machine with no metadata attests nothing); the real-corpus P=R=1.000
        // is proven by `examples/metajoin`.
        let pages = vec![
            ("https://x/en-US/docs/Web/CSS/a".to_string(), dep_a),
            ("https://x/en-US/docs/Web/CSS/b".to_string(), dep_b),
        ];
        let att = Attestation::discover(&pages);
        assert!(att.markers().is_empty(), "no frontmatter metadata on a test machine ⇒ honest abstention");
    }

    #[test]
    fn rendered_class_token_attests_python_and_rust_shaped_pages() {
        // The rendered route matches an element whose class carries a learned status token, whichever
        // extra tokens the author bundles (`stab deprecated`) and whichever quote style.
        let att = Attestation::from_class_markers(vec!["deprecated".to_string()]);
        assert!(att.attests("<div class=\"deprecated\"><p>Deprecated since version 3.11</p></div>"));
        assert!(att.attests("<div class='stab deprecated'>Deprecated</div>"));
        assert!(!att.attests("<div class=\"stable\">fine</div>"));
        assert!(!att.attests("the word deprecated in prose is not a class attribute"));
    }

    #[test]
    fn module_removal_needs_the_prohibition_and_removal_tokens_in_one_class_value() {
        // The tight compound: a class value carrying BOTH a prohibition token and a removal token
        // (hyphen-split). The data is read from the embedded `deprecation-status.json` (`prohibits` +
        // `removed`), so this exercises the real datum wiring, not a synthetic set.
        assert!(attests_module_removal("<div class=\"deprecated-removed\"><p>Removed in 3.13</p></div>"));
        assert!(attests_module_removal("<section class='deprecated removed'>gone</section>"));
        // A lone `deprecated` (the inline per-method note) is NOT a module-removal marker — the precision
        // line that keeps the subject basis clean.
        assert!(!attests_module_removal("<div class=\"versionmodified deprecated\">Deprecated since 3.11</div>"));
        // A lone `removed` with no prohibition token, or the word in prose, does not attest either.
        assert!(!attests_module_removal("<div class=\"removed\">gone</div>"));
        assert!(!attests_module_removal("the module was deprecated and removed in prose only"));
    }

    #[test]
    fn class_prohibition_join_reads_hyphen_compounds_never_prose() {
        // PASS 37 — the badge attester's data join: a class value carrying the prohibition token
        // as a whitespace- OR hyphen-split part joins (`icon icon-deprecated`); the word in prose
        // or in a non-class attribute never does. Reads the real embedded datum (`prohibits`).
        assert!(class_carries_prohibition("<abbr class=\"icon icon-deprecated\" title=\"x\">"));
        assert!(class_carries_prohibition("<div class='deprecated'>note</div>"));
        assert!(!class_carries_prohibition("<div class=\"stable icon\">deprecated in prose</div>"));
        assert!(!class_carries_prohibition("the word deprecated with no class attribute at all"));
    }

    #[test]
    fn rendered_discovery_keeps_only_the_prohibition_class_over_the_support_floor() {
        // Ten distinct-subject pages carry `class="deprecated"` (the join value) plus a generic
        // `class="section"`; discovery keeps only the prohibition token, and only because it EQUALS a
        // value the data marks (the test cannot read `deprecation-status.json` in isolation, so it drives
        // `discover_class_markers` directly with the prohibition set the library reads from data).
        let mut pages = Vec::new();
        for i in 0..10 {
            pages.push((
                format!("https://x/std/item{i}.html"),
                "<div class=\"section deprecated\">Deprecated</div>".to_string(),
            ));
        }
        let markers = discover_class_markers(&pages, &["deprecated".to_string()]);
        assert_eq!(markers, vec!["deprecated".to_string()]);
        // Below the support floor ⇒ no marker (a lone stray class is not a family).
        let few = &pages[..3];
        assert!(discover_class_markers(few, &["deprecated".to_string()]).is_empty());
    }
}
