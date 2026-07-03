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

use crate::lint_read::Memory;
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

/// Learn rules for `lang` from one documentation `source` over the network: READ it into a
/// [`Memory`] and QUERY the rules out. Returns empty [`Knowledge`] if nothing could be read — the
/// caller then degrades gracefully (cache, or an agent docs request). Network-only (`crawl` feature).
#[cfg(feature = "crawl")]
pub fn learn_from_url(lang: &str, source: &DocsSource, max_pages: usize, data_root: &Path) -> Knowledge {
    let memory = read_language(lang, std::slice::from_ref(source), max_pages, data_root);
    Knowledge {
        rules: rules_from_memory(lang, &memory).into_iter().map(|(r, _)| r).collect(),
        reference: memory.reference,
    }
}

/// READ a language's documentation into an association [`Memory`] — this is the whole learning step.
///
/// 1. Crawl every source (or fetch single files) and let the reader read the prose.
/// 2. GROUND a bounded sample of code examples against the installed toolchain (check mode only):
///    flagged examples' governing prose feeds the prohibition prototype, clean examples' the
///    endorsement one — the docs' claims tested against reality, no authored labels anywhere.
/// 3. BIND each (governing prose, code example) pair into the memory (prose ⊗ code, with url/slug),
///    and keep the non-violation code blocks as the "what's normal" reference corpus.
///
/// The returned memory is what "the model read the docs" means; rules are a query over it
/// ([`rules_from_memory`]), so expanding what the linter understands is more reading, never code.
#[cfg(feature = "crawl")]
pub fn read_language(lang: &str, sources: &[DocsSource], max_pages: usize, data_root: &Path) -> Memory {
    use crate::doc_crawler::{crawl, extract, fetch};
    use crate::lint_read::{Binding, PolarityBuilder, Reader};

    // Every read unit, uniformly: (page url, slug, governing prose, code example).
    let mut units: Vec<(String, String, String, String)> = Vec::new();
    let mut reader = Reader::new();
    let mut pages_read = 0usize;
    for src in sources {
        if src.crawl {
            for p in crawl(&[&src.url], max_pages, 50) {
                if pages_read < MAX_READ_PAGES {
                    reader.learn_span(&p.prose);
                    pages_read += 1;
                }
                let page_slug = rule_slug_under(&src.url, &p.url);
                let blocks = pre_blocks(&p.html);
                for (prose, code) in block_contexts(&p.html, &blocks) {
                    let s = page_slug.clone().unwrap_or_else(|| slug(&prose));
                    units.push((p.url.clone(), s, prose, code));
                }
            }
        } else if let Some((ct, body)) = fetch(&src.url) {
            for (prose, code) in extract(&ct, &body) {
                reader.learn_span(&prose);
                units.push((src.url.clone(), slug(&prose), prose, code));
            }
        }
    }

    // Ground: test a sample of examples against the toolchain; their prose shapes the prototypes.
    let mut builder = PolarityBuilder::new(reader);
    let mut checked = 0usize;
    for (_, _, prose, code) in &units {
        if prose.split_whitespace().count() < 3 {
            continue;
        }
        match crate::lint_toolchain::check(lang, code, data_root) {
            crate::lint_toolchain::Verdict::Flagged => builder.accumulate(prose, true),
            crate::lint_toolchain::Verdict::Clean => builder.accumulate(prose, false),
            crate::lint_toolchain::Verdict::Unknown => continue,
        }
        checked += 1;
        if checked >= MAX_GROUND_CHECKS {
            break;
        }
    }
    let polarity = builder.build();

    // Bind: every read unit becomes a prose⊗code association; non-violation code is the reference.
    let mut bindings = Vec::new();
    let mut reference = Vec::new();
    let mut seen_ref = std::collections::HashSet::new();
    for (url, s, prose, code) in units {
        if code.len() < 3 {
            continue;
        }
        let is_bad = polarity.classify(&prose) == Some(true);
        if !is_bad && code.len() >= 8 && reference.len() < MAX_REFERENCE && seen_ref.insert(code.clone()) {
            reference.push(code.clone());
        }
        if bindings.len() < MAX_BINDINGS {
            if let Some(b) = Binding::form(lang, &url, &s, &prose, &code, &polarity) {
                bindings.push(b);
            }
        }
    }
    Memory { bindings, reference, polarity: polarity.is_ready().then_some(polarity) }
}

/// QUERY rule candidates out of a read [`Memory`]: every binding whose prose the learned classifier
/// calls a prohibition becomes a rule — id from the binding's slug, description the docs' own prose,
/// bad the bound code; the fix is the next binding on the SAME page whose prose classifies as an
/// endorsement (docs put "use instead" right after the anti-pattern; neutral output blocks between
/// them are skipped, but another page's code is never borrowed). One rule per slug. Returns each rule
/// with its source url for citation. Pure over the memory — offline, deterministic, testable.
pub fn rules_from_memory(lang: &str, memory: &Memory) -> Vec<(LearnedRule, String)> {
    let Some(polarity) = &memory.polarity else { return Vec::new() };
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (i, b) in memory.bindings.iter().enumerate() {
        if b.slug.len() < 2 || polarity.classify(&b.prose) != Some(true) {
            continue;
        }
        if !seen.insert(b.slug.clone()) {
            continue;
        }
        let good = memory.bindings[i + 1..]
            .iter()
            .take_while(|nb| nb.url == b.url)
            .find(|nb| polarity.classify(&nb.prose) == Some(false))
            .map(|nb| nb.code.clone())
            .filter(|g| g != &b.code)
            .unwrap_or_default();
        out.push((
            LearnedRule {
                language: lang.to_string(),
                id: b.slug.clone(),
                severity: "medium".to_string(),
                description: trim_prose(&b.prose),
                bad: b.code.clone(),
                good,
            },
            b.url.clone(),
        ));
    }
    out
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

/// Cap on stored associations per language, bounding the serialized memory.
#[cfg(feature = "crawl")]
const MAX_BINDINGS: usize = 4000;

/// Cap on the "what's normal" reference corpus, keeping whole-site crawls fast to pack.
#[cfg(feature = "crawl")]
const MAX_REFERENCE: usize = 1500;

/// The governing prose (tag-stripped) of each `<pre>` block: the [`GOVERNING_CTX`] bytes before the
/// block, clipped to the previous block so an example's label is read from its OWN words. This is the
/// prose side of every binding the reader stores.
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
    use crate::lint_read::Polarity;

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

    /// Build a [`Memory`] from `(url, slug, prose, code)` read units over the test classifier —
    /// exactly the shape the live crawl stores, minus the network.
    fn memory_from(units: &[(&str, &str, &str, &str)]) -> Memory {
        let polarity = test_polarity();
        let bindings = units
            .iter()
            .filter_map(|(url, slug, prose, code)| {
                crate::lint_read::Binding::form("rust", url, slug, prose, code, &polarity)
            })
            .collect();
        Memory { bindings, reference: Vec::new(), polarity: Some(polarity) }
    }

    #[test]
    fn memory_query_pairs_bad_with_the_pages_good() {
        let memory = memory_from(&[
            ("https://d/rules/r1", "r1", "Avoid indexing with an inclusive range to len", "for i in 0..=xs.len() {}"),
            ("https://d/rules/r1", "r1", "Prefer iterating directly instead", "for x in xs {}"),
            ("https://d/rules/r2", "r2", "The language has three built-in numeric widths", "let y = 1;"),
        ]);
        let rules = rules_from_memory("rust", &memory);
        assert_eq!(rules.len(), 1, "only the prohibition binding becomes a rule");
        let (rule, url) = &rules[0];
        assert_eq!(url, "https://d/rules/r1", "the rule cites the page it was read from");
        assert!(rule.bad.contains("0..=xs.len()"));
        assert!(rule.good.contains("for x in xs"), "the page's endorsement binding is paired as the fix");
        assert!(rule.description.to_lowercase().contains("avoid indexing"), "description is the docs' own prose: {:?}", rule.description);
    }

    #[test]
    fn memory_query_abstains_without_a_grounded_classifier() {
        // No toolchain grounded the language ⇒ no polarity ⇒ no rule is invented from the memory.
        let mut memory = memory_from(&[("https://d/r", "r", "this code is incorrect and will fail", "x = [1]")]);
        memory.polarity = None;
        assert!(rules_from_memory("rust", &memory).is_empty(), "ungrounded memory yields no rules");
    }

    #[test]
    fn good_is_never_fabricated_from_position_or_another_page() {
        // A page whose only later block is a neutral output dump gets NO fix, and a good-classified
        // binding on a DIFFERENT page is never borrowed as this rule's fix.
        let memory = memory_from(&[
            ("https://d/rules/hd", "hd", "this code is incorrect and will fail", "h := http.Header{}"),
            ("https://d/rules/hd", "hd", "the program prints the following output", "// map[Etag]"),
            ("https://d/rules/other", "other", "this is the correct and idiomatic form", "ok()"),
        ]);
        let rules = rules_from_memory("go", &memory);
        assert_eq!(rules.len(), 1);
        assert!(rules[0].0.good.is_empty(), "no same-page fix ⇒ empty good, got: {:?}", rules[0].0.good);
    }

    #[test]
    fn memory_query_skips_neutral_blocks_to_reach_the_pages_fix() {
        // Docs often put an output block between the anti-pattern and its "use instead" — the fix is
        // still the same page's endorsement, with neutral blocks skipped, never guessed by position.
        let memory = memory_from(&[
            ("https://d/rules/xi", "xi", "This indexing is incorrect and unsafe", "xs[xs.len()]"),
            ("https://d/rules/xi", "xi", "the program prints the following output", "panic!"),
            ("https://d/rules/xi", "xi", "Prefer this correct idiomatic form instead", "xs.last()"),
        ]);
        let rules = rules_from_memory("go", &memory);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].0.good, "xs.last()", "the page's own fix is found past the neutral block");
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
        let contexts = block_contexts(&html, &blocks);
        assert!(contexts.iter().any(|(p, c)| c.contains("x = 1") && p.contains("incorrect")),
                "extraction still works around multibyte text: {contexts:?}");
    }

    #[cfg(feature = "crawl")]
    #[test]
    fn url_codec_round_trips() {
        assert_eq!(url_encode("zig language docs"), "zig+language+docs");
        assert_eq!(url_decode("https%3A%2F%2Fziglang.org%2Fdocs").unwrap(), "https://ziglang.org/docs");
        assert!(url_decode("%zz").is_none(), "malformed escape is rejected, not garbled");
    }
}
