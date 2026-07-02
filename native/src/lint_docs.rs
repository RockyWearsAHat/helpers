//! `lint_docs` — learn a language directly from its **official language documentation** (the
//! reference, the manual, the style guide), given only a link. No manual scraping step, no
//! pre-built rule corpus: the engine crawls the docs the language maintainers publish and reads
//! them the way the engine trains — normative sections (deprecations, warnings, avoid/instead
//! guidance) become rule candidates; every code example feeds the "what is normal in this
//! language" reference corpus. Which languages exist is never hardcoded: known sources live in
//! the `sources.json` data registry, and a language nobody registered is **discovered on the
//! fly** ([`discover_docs`]) — web search, probe the candidates, keep the first that actually
//! reads like documentation, cache the answer (positive or negative) per user.
//!
//! Only [`learn_from_url`] and [`discover_docs`] touch the network (behind the `crawl` feature).
//! Everything else is a pure function over already-fetched text, unit-tested offline.

#[cfg(feature = "crawl")]
use std::path::Path;

use crate::lint_read::Polarity;
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

/// Learn rules for `lang` from a documentation `source` over the network: fetch (or crawl) it and
/// extract [`LearnedRule`]s. Returns empty [`Knowledge`] if nothing could be fetched/extracted —
/// the caller then degrades gracefully (cache, or an agent docs request). Network-only, so it is
/// gated behind the `crawl` feature.
#[cfg(feature = "crawl")]
pub fn learn_from_url(lang: &str, source: &DocsSource, max_pages: usize, data_root: &Path) -> Knowledge {
    use crate::doc_crawler::{crawl, extract, fetch};

    let mut rules = Vec::new();
    let mut reference = Vec::new();
    if source.crawl {
        let pages = crawl(&[&source.url], max_pages, 50);
        // Read + test: build the polarity classifier by READING every page and GROUNDING a sample of
        // its code against the installed toolchain — no keyword list, no authored corpus. The learned
        // classifier then decides which blocks the docs treat as violations.
        let polarity = ground_polarity(lang, data_root, &pages);
        // Structure-aware path: most rules sites lay out ONE rule per page (ruff, eslint), so each
        // page yields one clean bad→good pair — exactly what the fit needs to ground a rule. This is
        // how the AI learns from the live docs: read every rule page, keep its anti-pattern and fix.
        rules.extend(rules_from_pages(lang, &source.url, &pages, &polarity));
        // Fallback for sites with an unusual layout (rules not one-per-page): the flattened
        // classified sections, so we still learn something rather than nothing.
        if rules.is_empty() {
            let mut sections = Vec::new();
            for p in &pages {
                sections.extend(p.sections.clone());
            }
            rules.extend(rules_from_sections(lang, &source.tool, &sections, &polarity));
        }
        // Every code block the crawl read is a sample of real, normal code in this language — the
        // corpus the fit calibrates "rare ⇒ distinctive" against, so a feature common in real code
        // never grounds a rule. Learned live from the same docs, no static artifact.
        reference = collect_reference(&pages, &polarity);
    } else if let Some((ct, body)) = fetch(&source.url) {
        let sections = extract(&ct, &body);
        let polarity = ground_polarity_sections(lang, data_root, &sections);
        rules.extend(rules_from_sections(lang, &source.tool, &sections, &polarity));
    }
    Knowledge { rules, reference }
}

/// How many code examples to ground against the toolchain per crawl. Enough grounded prose to shape
/// stable polarity prototypes, capped so the check-mode probes stay a small fraction of crawl time.
#[cfg(feature = "crawl")]
const MAX_GROUND_CHECKS: usize = 120;

/// How many pages the reader reads to learn the corpus's common-word stop-list before grounding. A
/// broad but bounded sample — the stop-list converges quickly, so reading every page would only add
/// CPU without changing which words count as common.
#[cfg(feature = "crawl")]
const MAX_READ_PAGES: usize = 200;

/// Build the polarity classifier for a crawl by READING then TESTING (the correction's grounding
/// loop): first read a broad sample of pages so the reader learns the corpus's common vocabulary, then
/// run a bounded sample of code examples through the installed toolchain in check mode — each flagged
/// example's governing prose feeds the prohibition prototype, each clean example's the endorsement
/// one. No authored labels. A language with no toolchain simply yields an unready classifier (which
/// abstains), so nothing is invented for it.
#[cfg(feature = "crawl")]
fn ground_polarity(lang: &str, data_root: &Path, pages: &[crate::doc_crawler::Page]) -> Polarity {
    use crate::lint_read::{PolarityBuilder, Reader};
    let mut reader = Reader::new();
    for p in pages.iter().take(MAX_READ_PAGES) {
        reader.learn_span(&p.prose);
    }
    let mut builder = PolarityBuilder::new(reader);
    let mut checked = 0usize;
    'pages: for p in pages {
        let blocks = pre_blocks(&p.html);
        for (prose, code) in block_contexts(&p.html, &blocks) {
            if prose.split_whitespace().count() < 3 {
                continue;
            }
            match crate::lint_toolchain::check(lang, &code, data_root) {
                crate::lint_toolchain::Verdict::Flagged => builder.accumulate(&prose, true),
                crate::lint_toolchain::Verdict::Clean => builder.accumulate(&prose, false),
                crate::lint_toolchain::Verdict::Unknown => continue,
            }
            checked += 1;
            if checked >= MAX_GROUND_CHECKS {
                break 'pages;
            }
        }
    }
    builder.build()
}

/// Ground polarity from flat `(prose, code)` sections (the single-file source path) the same way
/// [`ground_polarity`] does for crawled pages.
#[cfg(feature = "crawl")]
fn ground_polarity_sections(lang: &str, data_root: &Path, sections: &[(String, String)]) -> Polarity {
    use crate::lint_read::{PolarityBuilder, Reader};
    let mut reader = Reader::new();
    for (prose, _) in sections {
        reader.learn_span(prose);
    }
    let mut builder = PolarityBuilder::new(reader);
    for (prose, code) in sections.iter().take(MAX_GROUND_CHECKS) {
        if prose.split_whitespace().count() < 3 {
            continue;
        }
        match crate::lint_toolchain::check(lang, code, data_root) {
            crate::lint_toolchain::Verdict::Flagged => builder.accumulate(prose, true),
            crate::lint_toolchain::Verdict::Clean => builder.accumulate(prose, false),
            crate::lint_toolchain::Verdict::Unknown => continue,
        }
    }
    builder.build()
}

/// The governing prose (tag-stripped) of each `<pre>` block: the [`GOVERNING_CTX`] bytes before the
/// block, clipped to the previous block so an example's label is read from its OWN words. Shared by
/// grounding and rule extraction so both read the same context an example sits in.
#[cfg(feature = "crawl")]
fn block_contexts(html: &str, blocks: &[(usize, String)]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (i, (off, code)) in blocks.iter().enumerate() {
        let prev_end = if i == 0 { 0 } else { blocks[i - 1].0 };
        let mut start = off.saturating_sub(GOVERNING_CTX).max(prev_end);
        while !html.is_char_boundary(start) {
            start += 1;
        }
        out.push((crate::doc_crawler::strip_tags(&html[start..*off]), code.clone()));
    }
    out
}

/// Gather a deduplicated, bounded sample of NORMAL real code the crawl read — the "what's normal in
/// this language" corpus the fit calibrates distinctiveness against. Each page's documented
/// anti-pattern is EXCLUDED: a rule must not see its own violation as "normal", or it would be
/// forced to abstain on the very shape it is meant to flag. Capped so packing a whole-site crawl
/// stays fast; tiny inline spans are dropped as too thin to be a useful negative.
#[cfg(feature = "crawl")]
fn collect_reference(pages: &[crate::doc_crawler::Page], polarity: &Polarity) -> Vec<String> {
    use std::collections::HashSet;
    const MAX_REFERENCE: usize = 1500;
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for p in pages {
        let blocks = pre_blocks(&p.html);
        let (bad, _good, _explicit, _ctx) = bad_good_from_blocks(&p.html, &blocks, polarity);
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

/// Turn generic `(prose, code)` documentation sections into rule candidates by reading the
/// imperative/deprecation signal in each section's prose: a prohibitive lead-in marks its code as a
/// bad example, a positive one marks a fix. A fix is attached ONLY when the positive section is the
/// one IMMEDIATELY following the anti-pattern — adjacency is the page asserting "this fixes that".
/// Pairing a bad with a far-off positive section (as a persistent `pending_bad` once did) manufactured
/// fixes across unrelated rules; a fabricated `good` trains the engine on a lie. So the pending bad is
/// cleared the moment the next section is not its labeled fix. Sections with no signal are ignored.
pub fn rules_from_sections(
    lang: &str,
    tool: &str,
    sections: &[(String, String)],
    polarity: &Polarity,
) -> Vec<LearnedRule> {
    let mut out: Vec<LearnedRule> = Vec::new();
    let mut pending_bad: Option<usize> = None; // index into `out` awaiting its IMMEDIATELY-following fix
    let mut seq = 0usize;
    for (prose, code) in sections {
        match polarity.classify(prose) {
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
pub fn rules_from_pages(
    lang: &str,
    seed: &str,
    pages: &[crate::doc_crawler::Page],
    polarity: &Polarity,
) -> Vec<LearnedRule> {
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
        let (bad, good, explicit, label_ctx) = bad_good_from_blocks(&p.html, &blocks, polarity);
        // Language documentation mostly shows CORRECT code. A page yields a rule only when it
        // EXPLICITLY labels a block wrong (deprecated/incorrect/avoid … immediately governing
        // the block); every other page still teaches — its examples feed the reference corpus,
        // and normative prose beside code is handled per-section by `rules_from_sections`.
        if !explicit {
            continue;
        }
        if bad.len() < 3 {
            continue;
        }
        // Prefer the docs' own labeling sentence; fall back to the page lesson.
        let description = if label_ctx.len() >= 20 { label_ctx.chars().take(240).collect() } else { page_lesson(&p.prose) };
        out.push(LearnedRule {
            language: lang.to_string(),
            id,
            severity: "medium".to_string(),
            description,
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

/// Map the learned classifier's verdict on a block's governing prose to a polarity score: `-1`
/// violation, `+1` fix, `0` the classifier abstained (no confident call). This is the one place the
/// keyword table used to live; the decision is now the read + toolchain-grounded [`Polarity`].
#[cfg(feature = "crawl")]
fn classified_score(polarity: &Polarity, prose: &str) -> i32 {
    match polarity.classify(prose) {
        Some(true) => -1,
        Some(false) => 1,
        None => 0,
    }
}

/// Pick the (bad, good) code from a rule page's ordered `<pre>` blocks by READING the page, not by
/// position. Each block is judged by the learned [`Polarity`] classifier on the prose that governs it
/// — the page's own words, understood by a model that learned prohibition vs endorsement from reading
/// and toolchain grounding, not a keyword table. The anti-pattern is the first block classified a
/// violation (or, when none is and the page is itself a rule page, its first code block — a rule page
/// leads with the offending code); the fix is the first LATER block classified correct. A `good` is
/// only ever a block positively classified as a fix — never a positional guess.
#[cfg(feature = "crawl")]
fn bad_good_from_blocks(
    html: &str,
    blocks: &[(usize, String)],
    polarity: &Polarity,
) -> (String, String, bool, String) {
    if blocks.is_empty() {
        return (String::new(), String::new(), false, String::new());
    }
    // The governing context of each block: the page text from the previous block's start up to this
    // block (capped), where the docs put the example's label — keeping each example bound to its own
    // prose so polarity is read from THIS rule's words, not a neighbour's.
    let polarity_scores: Vec<i32> = blocks
        .iter()
        .enumerate()
        .map(|(i, (off, _))| {
            let prev_end = if i == 0 { 0 } else { blocks[i - 1].0 };
            let mut start = (*off).saturating_sub(GOVERNING_CTX).max(prev_end);
            // The cap is byte arithmetic; snap forward to a char boundary so multibyte
            // text (docs quote every human language) can never split a character.
            while !html.is_char_boundary(start) {
                start += 1;
            }
            classified_score(polarity, &crate::doc_crawler::strip_tags(&html[start..*off]))
        })
        .collect();

    // The violation: the first block the page calls wrong; else the lead block. `explicit`
    // records which case this was — language-doc pages mostly show NORMAL code, so a rule is
    // only minted when the page itself labels a block wrong (deprecated/incorrect/avoid …).
    let explicit = polarity_scores.iter().any(|&p| p < 0);
    let bad_i = polarity_scores.iter().position(|&p| p < 0).unwrap_or(0);
    let bad = blocks[bad_i].1.clone();
    // The words that did the labeling — the docs' own sentence about WHY this code is wrong —
    // is the truest description a minted rule can carry.
    let label_ctx = if explicit {
        let (off, _) = blocks[bad_i];
        let prev_end = if bad_i == 0 { 0 } else { blocks[bad_i - 1].0 };
        let mut start = off.saturating_sub(GOVERNING_CTX).max(prev_end);
        while !html.is_char_boundary(start) {
            start += 1;
        }
        crate::doc_crawler::strip_tags(&html[start..off]).split_whitespace().collect::<Vec<_>>().join(" ")
    } else {
        String::new()
    };
    // The fix: the first LATER block the page positively labels correct. No positive label ⇒ no fix.
    let good = blocks
        .iter()
        .zip(&polarity_scores)
        .skip(bad_i + 1)
        .find(|((_, _), &p)| p > 0)
        .map(|((_, c), _)| c.clone())
        .filter(|g| g != &bad)
        .unwrap_or_default();
    (bad, good, explicit, label_ctx)
}

/// The rule's English lesson: the "what it does" prose, trimmed to a short summary. Falls back to
/// the page's leading prose when the heading is absent.
#[cfg(feature = "crawl")]
fn page_lesson(prose: &str) -> String {
    let lower = prose.to_lowercase();
    let start = lower.find("what it does").map(|i| i + "what it does".len()).unwrap_or(0);
    let tail = &prose[start.min(prose.len())..];
    let tail_lower = tail.to_lowercase();
    let mut end = ["why is this bad", "example", "details", "references", "options"]
        .iter()
        .filter_map(|m| tail_lower.find(m))
        .filter(|&e| e > 0)
        .min()
        .unwrap_or(tail.len().min(240))
        .min(tail.len());
    // Byte caps may land inside a multibyte char; snap back to a boundary before slicing.
    while !tail.is_char_boundary(end) {
        end -= 1;
    }
    tail[..end].split_whitespace().collect::<Vec<_>>().join(" ").chars().take(240).collect()
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


// ── On-the-fly language discovery ─────────────────────────────────────────────

/// Minimum rule candidates a probed site must yield to be accepted as a language's documentation.
#[cfg(feature = "crawl")]
const MIN_DISCOVERED_RULES: usize = 5;

/// Crawl budget for probing one discovery candidate (a cheap taste, not the full learn).
#[cfg(feature = "crawl")]
const DISCOVERY_PROBE_PAGES: usize = 20;

/// The per-user learned source registry: where discovery caches what it found
/// (`~/.cache/helpers/lint-index/sources.json`, same shape as the committed seed). A negative
/// result is stored as `kind:"none"` so an unknown language is searched at most once per cache.
fn learned_sources_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join(".cache/helpers/lint-index/sources.json")
}

/// The cached discovery answer for `lang`: `Some(Some(src))` found, `Some(None)` searched and
/// negative-cached, `None` never searched.
pub fn learned_source(lang: &str) -> Option<Option<DocsSource>> {
    let raw = std::fs::read_to_string(learned_sources_path()).ok()?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    for e in json["sources"].as_array()? {
        if e["language"].as_str() != Some(lang) {
            continue;
        }
        return match e["kind"].as_str() {
            Some("none") => Some(None),
            _ => Some(Some(DocsSource {
                url: e["seed"].as_str().unwrap_or("").to_string(),
                crawl: true,
                tool: e["tool"].as_str().unwrap_or(lang).to_string(),
            })),
        };
    }
    None
}

/// Persist a discovery answer (found source, or a negative marker) into the learned registry.
fn remember_source(lang: &str, found: Option<&DocsSource>) {
    let path = learned_sources_path();
    let mut json: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({ "version": 1, "sources": [] }));
    let entry = match found {
        Some(src) => serde_json::json!({
            "tool": src.tool, "language": lang, "kind": "crawl", "seed": src.url, "discovered": true
        }),
        None => serde_json::json!({ "tool": lang, "language": lang, "kind": "none", "discovered": true }),
    };
    if let Some(arr) = json["sources"].as_array_mut() {
        arr.retain(|e| e["language"].as_str() != Some(lang));
        arr.push(entry);
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap_or_default());
}

/// Discover official documentation for a language no registry knows — assembled on the fly, no
/// built-in language list: ask the web, probe the top candidate sites, accept the first whose
/// crawl actually yields normative documentation (≥ [`MIN_DISCOVERED_RULES`] rule candidates),
/// and remember the answer either way so the search runs at most once per language.
#[cfg(feature = "crawl")]
pub fn discover_docs(lang: &str, data_root: &Path) -> Option<DocsSource> {
    if let Some(cached) = learned_source(lang) {
        return cached;
    }
    let found = search_and_probe(lang, data_root);
    remember_source(lang, found.as_ref());
    found
}

/// One discovery pass: web-search the language's official docs, rank candidate URLs by how much
/// they look like documentation, and probe the best few by actually crawling them.
#[cfg(feature = "crawl")]
fn search_and_probe(lang: &str, data_root: &Path) -> Option<DocsSource> {
    use crate::doc_crawler::fetch;
    let query = format!("{lang} programming language official documentation reference");
    let url = format!("https://html.duckduckgo.com/html/?q={}", url_encode(&query));
    let (_, html) = fetch(&url)?;
    let lang_lc = lang.to_lowercase();
    let mut seen_hosts = std::collections::HashSet::new();
    let mut candidates: Vec<(i32, String)> = Vec::new();
    for cand in result_urls(&html) {
        let lc = cand.to_lowercase();
        let Some(host) = lc.strip_prefix("https://").and_then(|r| r.split('/').next()) else {
            continue; // http or malformed — official docs serve https
        };
        if host.contains("duckduckgo") || !seen_hosts.insert(host.to_string()) {
            continue;
        }
        let mut score = 0;
        if lc.contains(&lang_lc) {
            score += 3;
        }
        for hint in ["doc", "reference", "manual", "spec", "lang"] {
            if lc.contains(hint) {
                score += 1;
            }
        }
        candidates.push((score, cand));
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, url) in candidates.into_iter().take(4) {
        let tool = url
            .strip_prefix("https://")
            .and_then(|r| r.split('/').next())
            .unwrap_or(lang)
            .trim_start_matches("www.")
            .to_string();
        let src = DocsSource { url: url.clone(), crawl: true, tool };
        if learn_from_url(lang, &src, DISCOVERY_PROBE_PAGES, data_root).rules.len() >= MIN_DISCOVERED_RULES {
            return Some(src);
        }
    }
    None
}

/// Result URLs from a DuckDuckGo static-HTML results page: each result link carries the real
/// destination percent-encoded in its `uddg=` parameter.
#[cfg(feature = "crawl")]
fn result_urls(html: &str) -> Vec<String> {
    let re = regex::Regex::new(r#"uddg=([^&"]+)"#).expect("static");
    re.captures_iter(html).filter_map(|c| url_decode(&c[1])).collect()
}

/// Percent-encode a search query (RFC 3986 unreserved kept, space → `+`).
#[cfg(feature = "crawl")]
fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Percent-decode a URL; `None` when the encoding is malformed.
#[cfg(feature = "crawl")]
fn url_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hex = s.get(i + 1..i + 3)?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                i += 3;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A polarity classifier LEARNED from labeled prose (no keyword table) — the offline stand-in for
    /// the read + toolchain-grounded classifier the live crawl builds. Each prohibition/endorsement
    /// word appears in its own sentence so it stays a distinctive (non-common) token.
    fn test_polarity() -> Polarity {
        Polarity::from_labeled(&[
            ("avoid indexing past the end of the range", true),
            ("this code is incorrect and will fail", true),
            ("this api is deprecated and was removed", true),
            ("never use a global mutable variable here", true),
            ("this pattern is discouraged as fragile", true),
            ("this obsolete call slowly leaks memory", true),
            ("doing this is dangerous and simply wrong", true),
            ("passing a raw pointer here is unsafe", true),
            ("this blocks the thread and deadlocks", true),
            ("reusing that buffer triggers a data race", true),
            ("swallowing the exception hides real bugs", true),
            ("hard coding the path is brittle", true),
            ("prefer iterating directly over the sequence", false),
            ("this is the correct and idiomatic form", false),
            ("this is the recommended supported approach", false),
            ("use this instead for readable clarity", false),
            ("this canonical shape is thoroughly tested", false),
            ("keep helpers small explicit and clean", false),
            ("validate input then return typed errors", false),
            ("this scales gracefully and stays maintainable", false),
            ("document every public function plainly", false),
            ("favor composition and descriptive names", false),
            ("handle the result and close resources cleanly", false),
            ("this efficient path is clear and safe", false),
        ])
    }

    #[test]
    fn sections_classified_pair_bad_then_good() {
        let sections = vec![
            ("Avoid indexing with an inclusive range to len".to_string(), "for i in 0..=xs.len() {}".to_string()),
            ("Prefer iterating directly instead".to_string(), "for x in xs {}".to_string()),
            ("The language has three built-in numeric widths".to_string(), "let y = 1;".to_string()),
        ];
        let rules = rules_from_sections("rust", "clippy", &sections, &test_polarity());
        assert_eq!(rules.len(), 1, "only the prohibition section becomes a rule");
        assert!(rules[0].bad.contains("0..=xs.len()"));
        assert!(rules[0].good.contains("for x in xs"), "the next endorsement section is paired as the fix");
    }

    #[cfg(feature = "crawl")]
    #[test]
    fn good_is_never_fabricated_from_position() {
        // Two code blocks, NO "correct/use instead" label. The old code grabbed the second block as
        // the fix — a fabrication. Faithful behavior: bad is the first block, good is EMPTY.
        let html = "<p>this is incorrect and will fail:</p><pre>h := http.Header{}\nh[\"etag\"] = x</pre><pre>// Output:\n// map[Etag]</pre>";
        let blocks = pre_blocks(html);
        let (bad, good, _explicit, _ctx) = bad_good_from_blocks(html, &blocks, &test_polarity());
        assert!(bad.contains("http.Header"), "bad is the lead block: {bad:?}");
        assert!(good.is_empty(), "no labeled fix ⇒ no fabricated good, got: {good:?}");
    }

    #[cfg(feature = "crawl")]
    #[test]
    fn good_is_taken_only_from_an_explicit_correct_label() {
        let html = "<p>this code is incorrect and will fail:</p><pre>if x == true {}</pre>\
                    <p>the correct idiomatic form:</p><pre>if x {}</pre>";
        let blocks = pre_blocks(html);
        let (bad, good, explicit, ctx) = bad_good_from_blocks(html, &blocks, &test_polarity());
        assert!(explicit, "an 'incorrect' context is classified an explicit violation marker");
        assert!(ctx.to_lowercase().contains("incorrect"), "the labeling sentence is the description: {ctx:?}");
        assert!(bad.contains("== true"), "bad captured: {bad:?}");
        assert!(good.contains("if x {}"), "classified fix captured: {good:?}");
    }

    #[test]
    fn sections_do_not_pair_a_fix_across_an_unrelated_section() {
        // bad, then an UNLABELED section, then a positive one. Adjacency is broken, so the far-off
        // positive section is NOT this bad's fix — pairing it would be a manufactured good.
        let sections = vec![
            ("This indexing is incorrect and unsafe".to_string(), "xs[xs.len()]".to_string()),
            ("the following diagram shows a nested layout".to_string(), "let y = 1;".to_string()),
            ("Prefer this correct idiomatic form instead".to_string(), "xs.last()".to_string()),
        ];
        let rules = rules_from_sections("go", "staticcheck", &sections, &test_polarity());
        assert_eq!(rules.len(), 1, "only the prohibition section becomes a rule");
        assert!(rules[0].good.is_empty(), "no adjacent fix ⇒ empty good, got: {:?}", rules[0].good);
    }

    #[test]
    fn discovery_registry_round_trips_and_negative_caches() {
        // Discovery must remember both answers so a language is searched at most once.
        let dir = std::env::temp_dir().join(format!("lint-docs-test-{}", std::process::id()));
        std::env::set_var("HOME", &dir); // learned registry lives under $HOME
        remember_source("zig", Some(&DocsSource { url: "https://ziglang.org/documentation/".into(), crawl: true, tool: "ziglang.org".into() }));
        assert_eq!(learned_source("zig").unwrap().unwrap().url, "https://ziglang.org/documentation/");
        remember_source("brainfuck", None);
        assert!(learned_source("brainfuck").unwrap().is_none(), "negative answer is cached");
        assert!(learned_source("cobol").is_none(), "never-searched language has no cached answer");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "crawl")]
    #[test]
    fn multibyte_context_never_splits_a_char() {
        // The governing-context cap is byte arithmetic; docs quote every human language, so the
        // cap must snap to a char boundary instead of panicking inside a multibyte character.
        let html = format!("<p>{}this code is incorrect and will fail:</p><pre>x = 1</pre>", "文".repeat(600));
        let blocks = pre_blocks(&html);
        let (bad, _good, _explicit, _ctx) = bad_good_from_blocks(&html, &blocks, &test_polarity());
        assert!(bad.contains("x = 1"), "extraction still works around multibyte text: {bad:?}");
    }

    #[cfg(feature = "crawl")]
    #[test]
    fn url_codec_round_trips() {
        assert_eq!(url_encode("zig language docs"), "zig+language+docs");
        assert_eq!(url_decode("https%3A%2F%2Fziglang.org%2Fdocs").unwrap(), "https://ziglang.org/docs");
        assert!(url_decode("%zz").is_none(), "malformed escape is rejected, not garbled");
    }
}
