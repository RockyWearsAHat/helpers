//! `lint_docs` — learn a language's lint rules directly from its **official documentation**, given
//! only a link. No manual scraping step, no pre-built corpus: the engine fetches the docs the
//! maintainers publish, extracts rule candidates (an anti-pattern, its fix, and the English lesson),
//! and returns [`Knowledge`] the [`crate::linter::LintModule`]/[`crate::linter::Reasoner`] train
//! from. The packing fit abstains on anything it can't separate cleanly, so even a rough crawl
//! yields a precise module.
//!
//! Two extraction paths, chosen by what the link serves:
//!   * **Structured rules JSON** (clippy's `lints.json`) → a dedicated, high-precision parse that
//!     keeps each lint's real id, level, and the bad/good code from its `docs` markdown.
//!   * **Anything else** (an HTML/Markdown rules site) → generic `(prose, code)` sections from
//!     [`crate::doc_crawler`], each labelled bad/good by the imperative/deprecation signal in its
//!     prose. Lower precision, but the packing fit drops what doesn't ground.
//!
//! Only [`learn_from_url`] touches the network (behind the `crawl` feature). Everything else is a
//! pure function over already-fetched text, so the extraction is unit-tested offline.

use crate::linter::{Knowledge, LearnedRule};

/// A resolved documentation source for a language: the URL to learn from, whether it is a single
/// structured file (fetched once) or a docs site to crawl, and the tool it belongs to (provenance
/// and the stable module id).
#[derive(Clone, Debug)]
pub struct DocsSource {
    /// The documentation URL to fetch or crawl.
    pub url: String,
    /// `true` → graph-crawl the site from this URL; `false` → fetch this single file.
    pub crawl: bool,
    /// The linter the docs belong to (e.g. `clippy`), used for the module id and provenance.
    pub tool: String,
}

/// The known per-version docs URL for `lang`, or `None` when we have no pattern — in which case the
/// calling agent is asked to supply one (via `search_web`). Version-pinned where the docs are
/// (clippy ships per Rust version); "latest" otherwise, which staleness re-checks track by toolchain
/// version. See [`clippy_url_candidates`] for the pinned→stable→master fallback clippy uses.
pub fn known_docs_url(lang: &str, version: &str) -> Option<DocsSource> {
    let (url, crawl, tool) = match lang {
        // Clippy renders every lint inline in one HTML page (no standalone lints.json anymore), so
        // it is fetched once (not crawled) and parsed by [`rules_from_clippy_html`].
        "rust" => (clippy_url_candidates(version).remove(0), false, "clippy"),
        "python" => ("https://docs.astral.sh/ruff/rules/".to_string(), true, "ruff"),
        "javascript" | "typescript" => {
            ("https://eslint.org/docs/latest/rules/".to_string(), true, "eslint")
        }
        "go" => ("https://staticcheck.dev/docs/checks/".to_string(), true, "staticcheck"),
        _ => return None,
    };
    Some(DocsSource { url, crawl, tool: tool.to_string() })
}

/// The clippy lint-list page URLs to try, in order: the exact Rust version, then `stable`, then
/// `master`. A clean dev toolchain matches the first version-pinned page (`rust-<version>/`); an
/// unreleased/nightly version falls back so learning still succeeds rather than 404-ing. Each page
/// embeds the full rule set as HTML — parsed by [`rules_from_clippy_html`].
pub fn clippy_url_candidates(version: &str) -> Vec<String> {
    let base = "https://rust-lang.github.io/rust-clippy";
    let mut v = Vec::new();
    if !version.is_empty() {
        v.push(format!("{base}/rust-{version}/"));
    }
    v.push(format!("{base}/stable/"));
    v.push(format!("{base}/master/"));
    v
}

/// Learn rules for `lang` from a documentation `source` over the network: fetch (or crawl) it and
/// extract [`LearnedRule`]s. Returns empty [`Knowledge`] if nothing could be fetched/extracted —
/// the caller then degrades gracefully (cache, or an agent docs request). Network-only, so it is
/// gated behind the `crawl` feature.
#[cfg(feature = "crawl")]
pub fn learn_from_url(lang: &str, source: &DocsSource, max_pages: usize) -> Knowledge {
    use crate::doc_crawler::{crawl, extract, fetch};

    let mut rules = Vec::new();
    let mut reference = Vec::new();
    if source.crawl {
        let pages = crawl(&[&source.url], max_pages, 50);
        // Structure-aware path: most rules sites lay out ONE rule per page (ruff, eslint), so each
        // page yields one clean bad→good pair — exactly what the fit needs to ground a rule. This is
        // how the AI learns from the live docs: read every rule page, keep its anti-pattern and fix.
        rules.extend(rules_from_pages(lang, &source.url, &pages));
        // Fallback for sites with an unusual layout (rules not one-per-page): the flattened
        // prose-signalled sections, so we still learn something rather than nothing.
        if rules.is_empty() {
            let mut sections = Vec::new();
            for p in &pages {
                sections.extend(p.sections.clone());
            }
            rules.extend(rules_from_sections(lang, &source.tool, &sections));
        }
        // Every code block the crawl read is a sample of real, normal code in this language — the
        // corpus the fit calibrates "rare ⇒ distinctive" against, so a feature common in real code
        // never grounds a rule. Learned live from the same docs, no static artifact.
        reference = collect_reference(&pages);
    } else if let Some((ct, body)) = fetch(&source.url) {
        if ct.contains("json") {
            rules.extend(rules_from_clippy_json(lang, &body));
        }
        if rules.is_empty() {
            // Not the expected structured shape — fall back to generic section extraction.
            rules.extend(rules_from_sections(lang, &source.tool, &extract(&ct, &body)));
        }
    }
    Knowledge { rules, reference }
}

/// Gather a deduplicated, bounded sample of NORMAL real code the crawl read — the "what's normal in
/// this language" corpus the fit calibrates distinctiveness against. Each page's documented
/// anti-pattern is EXCLUDED: a rule must not see its own violation as "normal", or it would be
/// forced to abstain on the very shape it is meant to flag. Capped so packing a whole-site crawl
/// stays fast; tiny inline spans are dropped as too thin to be a useful negative.
#[cfg(feature = "crawl")]
fn collect_reference(pages: &[crate::doc_crawler::Page]) -> Vec<String> {
    use std::collections::HashSet;
    const MAX_REFERENCE: usize = 1500;
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for p in pages {
        let blocks = pre_blocks(&p.html);
        let (bad, _good) = bad_good_from_blocks(&p.html, &blocks);
        for (_, c) in &blocks {
            if *c == bad {
                continue; // the anti-pattern — not normal code
            }
            if c.len() >= 8 && seen.insert(c.clone()) {
                out.push(c.clone());
                if out.len() >= MAX_REFERENCE {
                    return out;
                }
            }
        }
    }
    out
}

/// For Rust, fetch the first clippy lint-list page that responds (version-pinned → stable → master)
/// and parse its inline rules, so a missing per-version page doesn't abort learning. Handles both
/// the current HTML page and the legacy `lints.json` shape. Network-only.
#[cfg(feature = "crawl")]
pub fn learn_clippy(lang: &str, version: &str, _max_pages: usize) -> Knowledge {
    use crate::doc_crawler::fetch;
    for url in clippy_url_candidates(version) {
        let Some((ct, body)) = fetch(&url) else { continue };
        let rules = if ct.contains("json") {
            rules_from_clippy_json(lang, &body)
        } else {
            rules_from_clippy_html(lang, &body)
        };
        if !rules.is_empty() {
            // Every code block on the lint-list page is real Rust — the "normal" corpus the fit
            // calibrates distinctiveness against (empty for the legacy JSON shape, which has no HTML).
            let reference = pre_blocks(&body).into_iter().map(|(_, c)| c).filter(|c| c.len() >= 8).collect();
            return Knowledge { rules, reference };
        }
    }
    Knowledge { rules: Vec::new(), reference: Vec::new() }
}

// ── extraction (pure, offline-testable) ──────────────────────────────────────

/// Parse clippy's `lints.json` into rules. Each lint contributes its real id, a severity mapped
/// from its `level`, the lesson (the "What it does" prose), the bad example (the first fenced Rust
/// block in its `docs`), and the fix (the first block after a "Use instead" marker, if any). Lints
/// with no example are skipped — without a bad form there is nothing to ground.
pub fn rules_from_clippy_json(lang: &str, body: &str) -> Vec<LearnedRule> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    // The file is either a bare array or `{ "lints": [...] }` depending on version.
    let lints = v
        .as_array()
        .cloned()
        .or_else(|| v.get("lints").and_then(|x| x.as_array()).cloned())
        .unwrap_or_default();

    let mut out = Vec::new();
    for lint in &lints {
        let id = lint["id"].as_str().unwrap_or("").trim().to_string();
        if id.is_empty() {
            continue;
        }
        let docs = lint["docs"].as_str().unwrap_or("");
        let blocks = fenced_blocks(docs);
        let Some((_, bad)) = blocks.first().cloned() else {
            continue; // no example → cannot ground
        };
        // The fix is the first fenced block that starts after a "Use instead"/"Good" marker.
        let good = good_marker_index(docs)
            .and_then(|m| blocks.iter().find(|(pos, _)| *pos > m).map(|(_, c)| c.clone()))
            .unwrap_or_default();
        out.push(LearnedRule {
            language: lang.to_string(),
            id,
            severity: severity_from_level(lint["level"].as_str().unwrap_or("warn")),
            description: first_paragraph(docs),
            bad,
            good,
        });
    }
    out
}

/// Parse clippy's lint-list HTML page into rules. Each lint is an `<article id="…">` section
/// carrying its level (`level-deny`/`level-warn`/…), a "What it does" lesson, an example, and
/// often a "Use instead" fix. We key on the article id (the real lint name), map the level class to
/// severity, take the first `<pre>` as the bad example and the first `<pre>` after a "Use instead"
/// marker as the fix. Sections with no code are skipped. This is the structured replacement for the
/// retired `lints.json`.
pub fn rules_from_clippy_html(lang: &str, html: &str) -> Vec<LearnedRule> {
    const MARK: &str = "<article id=\"";
    // Article start offsets, so each lint's segment runs up to the next article.
    let mut starts: Vec<usize> = Vec::new();
    let mut from = 0;
    while let Some(rel) = html[from..].find(MARK) {
        let pos = from + rel;
        starts.push(pos);
        from = pos + MARK.len();
    }
    let mut out = Vec::new();
    for (n, &start) in starts.iter().enumerate() {
        let end = starts.get(n + 1).copied().unwrap_or(html.len());
        let seg = &html[start..end];
        let id_start = start + MARK.len();
        let Some(idq) = html[id_start..end].find('"') else { continue };
        let id = html[id_start..id_start + idq].trim().to_string();
        if id.is_empty() || !seg.contains("lint-doc") {
            continue; // not a lint article (nav/other anchors)
        }
        let blocks = pre_blocks(seg);
        let Some((_, bad)) = blocks.first().cloned() else {
            continue; // no example → cannot ground
        };
        let good = seg
            .to_lowercase()
            .find("use instead")
            .and_then(|m| blocks.iter().find(|(pos, _)| *pos > m).map(|(_, c)| c.clone()))
            .unwrap_or_default();
        out.push(LearnedRule {
            language: lang.to_string(),
            id,
            severity: clippy_level_severity(seg),
            description: clippy_what_it_does(seg),
            bad,
            good,
        });
    }
    out
}

/// `<pre>…</pre>` blocks of an HTML fragment as `(byte_offset, code_text)`, tags stripped. Offsets
/// let the caller tell which block follows a "Use instead" marker.
fn pre_blocks(html: &str) -> Vec<(usize, String)> {
    use crate::doc_crawler::strip_code;
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = html[from..].find("<pre") {
        let open = from + rel;
        let Some(gt) = html[open..].find('>') else { break };
        let body_start = open + gt + 1;
        let Some(crel) = html[body_start..].find("</pre>") else { break };
        let code = strip_code(&html[body_start..body_start + crel]);
        if code.len() >= 3 {
            out.push((open, code));
        }
        from = body_start + crel + 6;
    }
    out
}

/// Map a clippy lint section's `level-*` CSS class to our severity bucket.
fn clippy_level_severity(seg: &str) -> String {
    if seg.contains("level-deny") || seg.contains("level-forbid") {
        "high"
    } else if seg.contains("level-warn") {
        "medium"
    } else {
        "low"
    }
    .to_string()
}

/// The "What it does" lesson of a clippy lint section: the text between that heading and the next.
fn clippy_what_it_does(seg: &str) -> String {
    const HEADING: &str = "what it does";
    let lower = seg.to_lowercase();
    let Some(h) = lower.find(HEADING) else {
        return String::new();
    };
    // Start just past the heading text itself (ASCII, so byte math is char-safe).
    let after = &seg[h + HEADING.len()..];
    // Stop at the next section heading.
    let stop = ["why is this bad", "<h3", "</div>"]
        .iter()
        .filter_map(|m| after.to_lowercase().find(m))
        .min()
        .unwrap_or_else(|| after.len().min(600));
    crate::doc_crawler::strip_tags(&after[..stop]).chars().take(240).collect()
}

/// Turn generic `(prose, code)` documentation sections into rule candidates by reading the
/// imperative/deprecation signal in each section's prose: a prohibitive lead-in marks its code as a
/// bad example, a positive one marks a fix. A fix is attached ONLY when the positive section is the
/// one IMMEDIATELY following the anti-pattern — adjacency is the page asserting "this fixes that".
/// Pairing a bad with a far-off positive section (as a persistent `pending_bad` once did) manufactured
/// fixes across unrelated rules; a fabricated `good` trains the engine on a lie. So the pending bad is
/// cleared the moment the next section is not its labeled fix. Sections with no signal are ignored.
pub fn rules_from_sections(lang: &str, tool: &str, sections: &[(String, String)]) -> Vec<LearnedRule> {
    let mut out: Vec<LearnedRule> = Vec::new();
    let mut pending_bad: Option<usize> = None; // index into `out` awaiting its IMMEDIATELY-following fix
    let mut seq = 0usize;
    for (prose, code) in sections {
        match prose_signal(prose) {
            Some(true) => {
                seq += 1;
                out.push(LearnedRule {
                    language: lang.to_string(),
                    id: format!("{tool}_{}_{seq}", slug(prose)),
                    severity: "medium".to_string(),
                    description: trim_prose(prose),
                    bad: code.clone(),
                    good: String::new(),
                });
                pending_bad = Some(out.len() - 1);
            }
            // A fix only counts as this bad's fix when it directly follows it.
            Some(false) => {
                if let Some(i) = pending_bad.take() {
                    out[i].good = code.clone();
                }
            }
            // Any unlabeled section between a bad and a candidate fix breaks the adjacency: the next
            // positive section is no longer THIS bad's fix, so stop waiting rather than guess.
            None => pending_bad = None,
        }
    }
    out
}

/// Extract one [`LearnedRule`] per crawled **rule page** — the structure-aware path for docs laid
/// out one-rule-per-page (ruff, eslint). It is how the AI learns from the live site: every page
/// that is an individual rule (exactly one path segment below the crawl `seed`) becomes a rule whose
/// id is the URL slug, whose lesson is the page's "what it does" prose, and whose bad/good code come
/// from the page's ordered `<pre>` blocks via [`bad_good_from_blocks`] (bad = the lead/incorrect-
/// labeled block; good ONLY from an explicit correct-label, never positional). Pages with no code are
/// skipped — without a bad form there is nothing to ground.
/// This recovers the clean bad→good pair the flattened-section path loses, so the fit grounds rather
/// than abstains.
#[cfg(feature = "crawl")]
pub fn rules_from_pages(lang: &str, seed: &str, pages: &[crate::doc_crawler::Page]) -> Vec<LearnedRule> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for p in pages {
        let Some(id) = rule_slug_under(seed, &p.url) else {
            continue;
        };
        if !seen.insert(id.clone()) {
            continue;
        }
        let blocks = pre_blocks(&p.html);
        let (bad, good) = bad_good_from_blocks(&p.html, &blocks);
        if bad.len() < 3 {
            continue;
        }
        out.push(LearnedRule {
            language: lang.to_string(),
            id,
            severity: "medium".to_string(),
            description: page_lesson(&p.prose),
            bad,
            good,
        });
    }
    out
}

/// The rule id for a page that sits exactly one path segment below `seed` (a per-rule page like
/// `…/ruff/rules/<name>/` or `…/eslint/rules/<name>`), or `None` for the index itself or anything
/// deeper/elsewhere. The id is the slug, lowercased and sanitized to `[a-z0-9_-]`.
#[cfg(feature = "crawl")]
fn rule_slug_under(seed: &str, url: &str) -> Option<String> {
    let seed = seed.split(['?', '#']).next().unwrap_or(seed).trim_end_matches('/');
    let url = url.split(['?', '#']).next().unwrap_or(url).trim_end_matches('/');
    let rest = url.strip_prefix(seed)?.trim_start_matches('/');
    if rest.is_empty() || rest.contains('/') {
        return None; // the index page (== seed) or something deeper than one segment
    }
    let slug: String = rest
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    (slug.len() >= 2).then_some(slug)
}

/// How much page text immediately before a code block counts as the prose that GOVERNS it — the
/// label/heading a docs page puts right above its example. Wide enough to catch a short heading or a
/// `class="incorrect"` wrapper, narrow enough not to bleed into the previous example's discussion.
#[cfg(feature = "crawl")]
const GOVERNING_CTX: usize = 320;

/// Read the polarity a page asserts ABOUT a code block from the words that govern it — the text just
/// before the block, wherever the page wrote the signal: heading prose ("Examples of incorrect
/// code"), inline guidance ("use instead"), or the structural class a page tags the example with
/// (`class="incorrect"`). This is general English comprehension, not a per-site marker table: any
/// docs site that says, in English, whether code is wrong or right is understood. Positive ⇒ the page
/// calls this code a FIX; negative ⇒ a VIOLATION; zero ⇒ the page does not say, so we will not guess.
/// Substring-safe: "incorrect" is not double-counted as "correct".
#[cfg(feature = "crawl")]
fn governed_polarity(ctx: &str) -> i32 {
    let c = ctx.to_lowercase();
    let n = |needle: &str| c.matches(needle).count() as i32;
    let incorrect = n("incorrect");
    let not_recommended = n("not recommended");
    // "correct" that is not the tail of "incorrect"; "recommended" that is not "not recommended".
    let correct = (n("correct") - incorrect).max(0);
    let recommended = (n("recommended") - not_recommended).max(0);
    let neg = incorrect
        + not_recommended
        + n("avoid")
        + n("anti-pattern")
        + n("problematic")
        + n("deprecated")
        + n("don't")
        + n("do not")
        + n("will be flagged")
        + n("bad example");
    let pos = correct
        + recommended
        + n("use instead")
        + n("instead:")
        + n("do this")
        + n("fixed")
        + n("good example")
        + n("prefer");
    pos - neg
}

/// Pick the (bad, good) code from a rule page's ordered `<pre>` blocks by READING the page, not by
/// position. Each block is judged by the polarity of the prose that governs it ([`governed_polarity`])
/// — the page's own English label. The anti-pattern is the first block the page calls a violation
/// (or, when the page labels none but is itself a rule page, its first code block — a rule page leads
/// with the offending code); the fix is the first LATER block the page calls correct. A `good` is
/// only ever a block the page positively labels as a fix — never a positional guess — so a violation
/// is never paired with an unrelated snippet. The page asserts the pairing or we emit none of it.
#[cfg(feature = "crawl")]
fn bad_good_from_blocks(html: &str, blocks: &[(usize, String)]) -> (String, String) {
    if blocks.is_empty() {
        return (String::new(), String::new());
    }
    // The governing context of each block: the page text from the previous block's start up to this
    // block (capped), where the docs put the example's label — keeping each example bound to its own
    // prose so polarity is read from THIS rule's words, not a neighbour's.
    let polarity: Vec<i32> = blocks
        .iter()
        .enumerate()
        .map(|(i, (off, _))| {
            let prev_end = if i == 0 { 0 } else { blocks[i - 1].0 };
            let start = (*off).saturating_sub(GOVERNING_CTX).max(prev_end);
            governed_polarity(&html[start..*off])
        })
        .collect();

    // The violation: the first block the page calls wrong; else the lead block (a rule page opens
    // with the code it flags).
    let bad_i = polarity.iter().position(|&p| p < 0).unwrap_or(0);
    let bad = blocks[bad_i].1.clone();
    // The fix: the first LATER block the page positively labels correct. No positive label ⇒ no fix.
    let good = blocks
        .iter()
        .zip(&polarity)
        .skip(bad_i + 1)
        .find(|((_, _), &p)| p > 0)
        .map(|((_, c), _)| c.clone())
        .filter(|g| g != &bad)
        .unwrap_or_default();
    (bad, good)
}

/// The rule's English lesson: the "what it does" prose, trimmed to a short summary. Falls back to
/// the page's leading prose when the heading is absent.
#[cfg(feature = "crawl")]
fn page_lesson(prose: &str) -> String {
    let lower = prose.to_lowercase();
    let start = lower.find("what it does").map(|i| i + "what it does".len()).unwrap_or(0);
    let tail = &prose[start.min(prose.len())..];
    let tail_lower = tail.to_lowercase();
    let end = ["why is this bad", "example", "details", "references", "options"]
        .iter()
        .filter_map(|m| tail_lower.find(m))
        .filter(|&e| e > 0)
        .min()
        .unwrap_or(tail.len().min(240));
    tail[..end.min(tail.len())].split_whitespace().collect::<Vec<_>>().join(" ").chars().take(240).collect()
}

/// Map a clippy lint `level` to our severity bucket: `deny`/`forbid` → high, `warn` → medium,
/// everything else (`allow`, pedantic/nursery groups) → low.
fn severity_from_level(level: &str) -> String {
    match level.trim().to_lowercase().as_str() {
        "deny" | "forbid" => "high",
        "warn" => "medium",
        _ => "low",
    }
    .to_string()
}

/// Every fenced ```code``` block in a Markdown body as `(byte_offset_of_block, code)`, with the
/// info string (the word after the opening fence) dropped. Offsets let a caller tell which block
/// comes after a marker like "Use instead".
fn fenced_blocks(md: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let bytes = md.as_bytes();
    let mut i = 0;
    while let Some(rel) = md[i..].find("```") {
        let open = i + rel;
        let after = open + 3;
        // Skip the info string up to the newline.
        let body_start = md[after..].find('\n').map(|n| after + n + 1).unwrap_or(bytes.len());
        let Some(crel) = md[body_start..].find("```") else { break };
        let code = md[body_start..body_start + crel].trim().to_string();
        if code.len() >= 3 {
            out.push((open, code));
        }
        i = body_start + crel + 3;
    }
    out
}

/// Byte offset of the first "use instead" / "good" / "correct" marker in a docs body, if present —
/// the boundary after which a fenced block is the documented fix rather than the anti-pattern.
fn good_marker_index(md: &str) -> Option<usize> {
    let lower = md.to_lowercase();
    ["use instead", "instead:", "good:", "correct:", "do this"]
        .iter()
        .filter_map(|m| lower.find(m))
        .min()
}

/// The first non-empty, non-heading paragraph of a Markdown body — the rule's English lesson.
fn first_paragraph(md: &str) -> String {
    for para in md.split("\n\n") {
        let t: String = para
            .lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join(" ");
        let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
        if t.len() >= 12 && !t.starts_with("```") {
            return t.chars().take(240).collect();
        }
    }
    String::new()
}

/// Classify a section's prose: `Some(true)` = anti-pattern, `Some(false)` = recommended fix,
/// `None` = neutral. The vocabulary mirrors how docs flag good vs bad ("avoid", "deprecated",
/// "prefer", "use instead").
fn prose_signal(prose: &str) -> Option<bool> {
    let p = prose.to_lowercase();
    const BAD: &[&str] = &[
        "avoid", "never", "don't", "do not", "deprecated", "unsound", "undefined behavior",
        "incorrect", "anti-pattern", "not recommended", "bad:", "instead of", "warning",
    ];
    const GOOD: &[&str] = &[
        "prefer", "use instead", "instead:", "correct", "recommended", "good:", "do this",
        "better",
    ];
    if BAD.iter().any(|w| p.contains(w)) {
        Some(true)
    } else if GOOD.iter().any(|w| p.contains(w)) {
        Some(false)
    } else {
        None
    }
}

/// Trim section prose to a short lesson for the advice message.
fn trim_prose(prose: &str) -> String {
    prose.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(160).collect()
}

/// Slugify prose into a short, stable id fragment: lowercase alphanumerics, `_`-separated, capped.
fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut last_us = false;
    for c in s.trim().chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            last_us = false;
        } else if !last_us && !out.is_empty() {
            out.push('_');
            last_us = true;
        }
        if out.len() >= 24 {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // A miniature of clippy's lints.json. Built with serde_json so the embedded markdown (with its
    // `###` headers and ```` ``` ```` fences) needs no fragile raw-string delimiter.
    fn clippy_json() -> String {
        let needless_docs = "### What it does\nChecks for return statements at the end of a block.\n\n### Why is this bad?\nRemoving them makes code more concise.\n\n### Example\n```rust\nfn foo() -> i32 {\n    return 1;\n}\n```\nUse instead:\n```rust\nfn foo() -> i32 {\n    1\n}\n```\n";
        serde_json::json!([
            { "id": "needless_return", "level": "warn", "docs": needless_docs },
            { "id": "no_example", "level": "deny", "docs": "### What it does\nNo code block." }
        ])
        .to_string()
    }

    #[test]
    fn clippy_json_yields_bad_good_pairs() {
        let rules = rules_from_clippy_json("rust", &clippy_json());
        // The lint with no example is dropped; the one with a pair is kept.
        assert_eq!(rules.len(), 1);
        let r = &rules[0];
        assert_eq!(r.id, "needless_return");
        assert_eq!(r.severity, "medium");
        assert!(r.bad.contains("return 1;"), "bad example captured: {:?}", r.bad);
        assert!(r.good.contains("    1"), "good example captured: {:?}", r.good);
        assert!(!r.description.is_empty(), "the lesson is captured");
    }

    #[test]
    fn sections_labelled_by_signal_pair_bad_then_good() {
        let sections = vec![
            ("Avoid indexing with an inclusive range to len".to_string(), "for i in 0..=xs.len() {}".to_string()),
            ("Prefer iterating directly instead".to_string(), "for x in xs {}".to_string()),
            ("Some neutral prose about the language".to_string(), "let y = 1;".to_string()),
        ];
        let rules = rules_from_sections("rust", "clippy", &sections);
        assert_eq!(rules.len(), 1, "only the signalled section becomes a rule");
        assert!(rules[0].bad.contains("0..=xs.len()"));
        assert!(rules[0].good.contains("for x in xs"), "the next good section is paired as the fix");
    }

    #[cfg(feature = "crawl")]
    #[test]
    fn good_is_never_fabricated_from_position() {
        // Two code blocks, NO "correct/use instead" label. The old code grabbed the second block as
        // the fix — a fabrication. Faithful behavior: bad is the first block, good is EMPTY.
        let html = "<p>incorrect:</p><pre>h := http.Header{}\nh[\"etag\"] = x</pre><pre>// Output:\n// map[Etag]</pre>";
        let blocks = pre_blocks(html);
        let (bad, good) = bad_good_from_blocks(html, &blocks);
        assert!(bad.contains("http.Header"), "bad is the lead block: {bad:?}");
        assert!(good.is_empty(), "no labeled fix ⇒ no fabricated good, got: {good:?}");
    }

    #[cfg(feature = "crawl")]
    #[test]
    fn good_is_taken_only_from_an_explicit_correct_label() {
        let html = "<p>Examples of incorrect code:</p><pre>if x == true {}</pre>\
                    <p>Examples of correct code:</p><pre>if x {}</pre>";
        let blocks = pre_blocks(html);
        let (bad, good) = bad_good_from_blocks(html, &blocks);
        assert!(bad.contains("== true"), "bad captured: {bad:?}");
        assert!(good.contains("if x {}"), "labeled fix captured: {good:?}");
    }

    #[test]
    fn sections_do_not_pair_a_fix_across_an_unrelated_section() {
        // bad, then an UNLABELED section, then a positive one. Adjacency is broken, so the far-off
        // positive section is NOT this bad's fix — pairing it would be a manufactured good.
        let sections = vec![
            ("Avoid indexing past len, incorrect".to_string(), "xs[xs.len()]".to_string()),
            ("Some neutral explanation paragraph".to_string(), "let y = 1;".to_string()),
            ("Prefer this correct form instead".to_string(), "xs.last()".to_string()),
        ];
        let rules = rules_from_sections("go", "staticcheck", &sections);
        assert_eq!(rules.len(), 1, "only the bad-signalled section becomes a rule");
        assert!(rules[0].good.is_empty(), "no adjacent fix ⇒ empty good, got: {:?}", rules[0].good);
    }

    #[test]
    fn known_url_is_version_pinned_for_rust() {
        let s = known_docs_url("rust", "1.95.0").unwrap();
        assert!(s.url.contains("rust-1.95.0"), "version-pinned page: {}", s.url);
        assert!(!s.crawl, "the single lint-list page is fetched, not crawled");
        assert_eq!(s.tool, "clippy");
        assert!(known_docs_url("cobol", "0").is_none(), "unknown language → agent supplies the link");
    }

    #[test]
    fn clippy_html_section_yields_a_rule() {
        // A miniature of the clippy lint-list page: one `<article id>` lint with a level, a lesson,
        // an example, and a "Use instead" fix.
        let html = concat!(
            "<article id=\"needless_return\">",
            "<span class=\"label lint-level level-warn\">warn</span>",
            "<div class=\"lint-docs\"><div class=\"lint-doc-md\">",
            "<h3>What it does</h3><p>Checks for return at the end of a block.</p>",
            "<h3>Example</h3><pre>fn f() -> i32 { return 1; }</pre>",
            "<p>Use instead:</p><pre>fn f() -> i32 { 1 }</pre>",
            "</div></div></article>",
            "<article id=\"not_a_lint\"><p>nav, no lint-doc here</p></article>",
        );
        let rules = rules_from_clippy_html("rust", html);
        assert_eq!(rules.len(), 1, "only the real lint article becomes a rule");
        let r = &rules[0];
        assert_eq!(r.id, "needless_return");
        assert_eq!(r.severity, "medium");
        assert!(r.bad.contains("return 1;"), "bad example: {:?}", r.bad);
        assert!(r.good.contains("{ 1 }"), "good example after 'Use instead': {:?}", r.good);
        assert!(r.description.to_lowercase().contains("return at the end"), "lesson: {:?}", r.description);
    }

    #[test]
    fn clippy_candidates_fall_back_to_stable_and_master() {
        let c = clippy_url_candidates("1.95.0");
        assert_eq!(c.len(), 3);
        assert!(c[0].contains("rust-1.95.0"));
        assert!(c[1].contains("/stable/"));
        assert!(c[2].contains("/master/"));
    }
}
