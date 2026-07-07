//! `lint_docs` — learn a language directly from its **official language documentation** (the
//! reference, the manual, the style guide), given only a link. No manual scraping step, no
//! pre-built rule corpus: the engine crawls the docs the language maintainers publish and reads
//! them the way the engine trains — normative sections (deprecations, warnings, avoid/instead
//! guidance) become rule candidates; every code example feeds the "what is normal in this
//! language" reference corpus. Which languages exist is never hardcoded: known sources live in
//! the `sources.json` data registry plus the per-machine added-sources store
//! ([`add_docs_source`]); a language nobody registered is ASKED for at runtime — never
//! web-searched. Setup re-validates page caches older than [`CRAWL_MAX_AGE`] against the live
//! site, so modules always build from current documentation.
//!
//! Only [`learn_from_url`] and [`read_language`] touch the network (behind the `crawl`
//! feature, and only in setup mode). Everything else is a pure function over already-fetched
//! text, unit-tested offline.

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
    // Discovery probes are partial by design (few pages) — never cached, so a later full read
    // cannot mistake a probe's slice for the whole source.
    let memory = read_language(lang, std::slice::from_ref(source), max_pages, data_root, None);
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
pub fn read_language(
    lang: &str,
    sources: &[DocsSource],
    max_pages: usize,
    data_root: &Path,
    cache_version: Option<&str>,
) -> Memory {
    use crate::lint_read::{Binding, PolarityBuilder, Reader};

    // Every read unit, uniformly: (page url, slug, governing prose, code example). Units are
    // INTERLEAVED round-robin across the language's sources so the bounded-memory caps
    // (`MAX_BINDINGS`, `MAX_REFERENCE`, the grounding sample) are source-FAIR: a big source
    // read first must not starve a richer one (measured: MDN filled all 4,000 binding slots
    // and ESLint's 300 rule pages bound NOTHING — read and silently discarded). Page prose
    // feeds the reader the same way, one page per source per turn.
    let mut reader = Reader::new();
    let mut per_source: Vec<Vec<(String, String, String, String)>> = Vec::new();
    let mut prose_queues: Vec<Vec<String>> = Vec::new();
    // Languages this run knows by NAME — the training language plus everything registered —
    // so page attribution can recognize a path segment naming a language no claims cover
    // yet (a brand-new language's own docs site).
    let mut extra_langs: std::collections::HashSet<String> =
        crate::lint_train::registered_languages(data_root).into_iter().collect();
    extra_langs.insert(lang.to_lowercase());
    for src in sources {
        let mut src_units = Vec::new();
        let mut src_prose = Vec::new();
        for page in crawled_source(src, max_pages, cache_version, &extra_langs) {
            src_prose.push(page.prose);
            for (s, prose, code, hint) in page.units {
                // Ledger #18 + "A site is a source": a block's own label wins; an unlabeled
                // block inherits its PAGE's attribution. A block that lands on a DIFFERENT
                // known language (an MDN JavaScript page's `brush: html` example; a css-
                // attributed page's unlabeled blocks while training javascript) is
                // prose-only here — it never binds, never grounds a polarity label, never
                // enters this language's reference corpus. The page's prose was already
                // read above.
                let effective = if hint.is_empty() { page.lang.clone() } else { hint };
                if crate::lint_train::foreign_example(lang, &effective) {
                    continue;
                }
                src_units.push((page.url.clone(), s, prose, code));
            }
        }
        per_source.push(src_units);
        prose_queues.push(src_prose);
    }
    let mut pages_read = 0usize;
    let mut dotted: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    let mut prose_cursors: Vec<std::vec::IntoIter<String>> =
        prose_queues.into_iter().map(Vec::into_iter).collect();
    'reading: loop {
        let mut emitted = false;
        for cursor in &mut prose_cursors {
            if pages_read >= MAX_READ_PAGES {
                break 'reading;
            }
            if let Some(prose) = cursor.next() {
                if !prose.trim().is_empty() {
                    reader.learn_span(&prose);
                    crate::lint_read::tally_dotted_tokens(&prose, &mut dotted);
                    crate::lint_read::tally_name_aliases(lang, &prose, &mut dotted);
                    pages_read += 1;
                }
                emitted = true;
            }
        }
        if !emitted {
            break;
        }
    }
    let mut units: Vec<(String, String, String, String)> = Vec::new();
    {
        let mut cursors: Vec<std::vec::IntoIter<_>> =
            per_source.into_iter().map(Vec::into_iter).collect();
        loop {
            let mut emitted = false;
            for cursor in &mut cursors {
                if let Some(unit) = cursor.next() {
                    units.push(unit);
                    emitted = true;
                }
            }
            if !emitted {
                break;
            }
        }
    }

    // File-extension claims: filenames live in the docs' own commands and examples too
    // (`kotlinc hello.kt`), so unit code feeds the tally alongside the prose.
    for (_, _, _, code) in &units {
        crate::lint_read::tally_dotted_tokens(code, &mut dotted);
    }
    // Teaching material enriches every reading: the shipped principles corpus is prose the
    // reader learns from alongside the docs (never rules).
    for doc in crate::lint_train::corpus_prose(data_root) {
        reader.learn_span(&doc);
    }
    // The claims are the tally as read: call typography already excluded invocations, and a
    // corpus-head filter is deliberately ABSENT — a language's docs say its own extension's
    // name in prose constantly ("js" in MDN, "php" in the manual), so commonness is exactly
    // the wrong reason to drop a claim; cross-language junk loses at RESOLUTION instead
    // (primary rule, counts, the prose guard).
    let extensions: std::collections::BTreeMap<String, u32> = dotted;
    // Ground: test a sample of examples against the toolchain; their prose shapes the prototypes.
    // Grounding verdicts are the ONLY labels anywhere — no curated corpus, no authored labels:
    // the classifier is learned through understanding and tested against reality.
    let mut builder = PolarityBuilder::new(reader);
    // Ground in PARALLEL: each check spawns the toolchain once (an isolated temp file per
    // probe), so the wall-clock cost is process spawns, not the 1-bit math — a serial loop here
    // was the slowest part of learning a language. Verdicts fold into the prototypes serially
    // afterwards, keeping the accumulation deterministic in unit order.
    use rayon::prelude::*;
    let probes: Vec<&(String, String, String, String)> = units
        .iter()
        // Lead units carry no code — there is nothing for the toolchain to test.
        .filter(|(_, _, prose, code)| code.len() >= 3 && prose.split_whitespace().count() >= 3)
        .take(MAX_GROUND_CHECKS)
        .collect();
    let verdicts: Vec<crate::lint_toolchain::Verdict> = probes
        .par_iter()
        .map(|(_, _, _, code)| crate::lint_toolchain::check(lang, code, data_root))
        .collect();
    let mut flagged: std::collections::HashSet<u64> = std::collections::HashSet::new();
    // Honest labels (LINTER.md, "Honest grounding labels"): Flagged is reality's own verdict
    // and labels bad; Clean only says "parses", so it labels good ONLY as the fix-sibling of
    // a flagged block in its own section (violation first, fix after — the documented
    // convention). Every other clean unit trains nothing: it stays reference material, so
    // neutral tutorial prose can no longer flood the endorsement prototype and a lint doc's
    // clean-parsing "incorrect" example no longer teaches the opposite of its page.
    let flagged_sections: std::collections::HashSet<(&str, &str)> = probes
        .iter()
        .zip(&verdicts)
        .filter(|(_, v)| matches!(v, crate::lint_toolchain::Verdict::Flagged))
        .map(|((url, s, _, _), _)| (url.as_str(), s.as_str()))
        .collect();
    for ((url, s, prose, code), verdict) in probes.iter().zip(&verdicts) {
        match verdict {
            crate::lint_toolchain::Verdict::Flagged => {
                builder.accumulate(prose, true);
                // The reality-tested label itself (ledger #19): compile may keep this
                // example's literal tokens because the toolchain, not register drift,
                // called it a violation.
                flagged.insert(crate::lint_ai::token_seed(code));
            }
            crate::lint_toolchain::Verdict::Clean => {
                if flagged_sections.contains(&(url.as_str(), s.as_str())) {
                    // The fix-sibling convention: this clean block is the documented fix,
                    // so its prose is genuine endorsement register — prototype food.
                    builder.accumulate(prose, false);
                } else {
                    // Every other clean unit is EXPOSURE only (LINTER.md): reality the
                    // words stood next to, never an endorsement label.
                    builder.tally_exposure(prose);
                }
            }
            crate::lint_toolchain::Verdict::Unknown => {}
        }
    }
    let mut polarity = builder.build();
    // Knowledge transfer: prohibition prose reads the same in every language — but a
    // language's OWN grounding outranks it absolutely: transfer is only for a language that
    // CANNOT be grounded (no toolchain installed — or none exists) and whose reading
    // therefore produced no evidence at all. Vote counts never decide (measured: rust
    // reference fragments false-flagging under a bare-file check once out-voted MDN's own
    // healthy 8-flagged/112-clean grounding and every tutorial sentence minted as law).
    // There is no committed classifier: a fresh machine self-bootstraps from its
    // dictionary's meaning network plus its own grounded reading (or a registry module that
    // already did).
    if !polarity.has_evidence() {
        if let Some(t) = transferred_polarity() {
            polarity = t.as_ref().clone(); // cold crawl path — one clone, kept and retrained
        }
    }
    if polarity.is_ready() {
        save_global_polarity(&polarity);
    }

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
    // An UNREADY classifier is still kept once it has READ: its reader's frequencies feed
    // the dictionary-negation cold floor (LINTER.md, "The cold floor"), which is how an
    // ungroundable language on a fresh machine mints its first overt prohibitions.
    let keep = polarity.is_ready() || polarity.reader().total_read() > 0;
    Memory { bindings, reference, polarity: keep.then_some(polarity), pages_read, extensions, flagged }
}

/// Ensure a SITE source's page cache exists (crawling when allowed) and return the languages
/// its pages attribute to — the discovery step of "A site is a source" (LINTER.md). Replay-
/// only when the cache is current; offline+cold yields nothing, honestly.
#[cfg(feature = "crawl")]
pub(crate) fn ensure_site_cache(
    src: &DocsSource,
    max_pages: usize,
    extra_langs: &std::collections::HashSet<String>,
) -> Vec<String> {
    let mut langs: std::collections::BTreeSet<String> = Default::default();
    for page in crawled_source(src, max_pages, Some(crate::lint_train::train_version()), extra_langs) {
        if !page.lang.is_empty() {
            langs.insert(page.lang);
        }
    }
    langs.into_iter().collect()
}

/// The languages a site source's CACHED pages attribute to — the offline read
/// [`crate::lint_train::resolved_sources`] and language enumeration consult. Memoized per
/// (tool, file state): the caches are multi-MB and this is asked once per language per run.
pub(crate) fn cached_site_langs(tool: &str) -> std::collections::HashSet<String> {
    type Cache = std::sync::Mutex<std::collections::HashMap<String, (u128, std::collections::HashSet<String>)>>;
    static CACHE: std::sync::OnceLock<Cache> = std::sync::OnceLock::new();
    let path = crawl_cache_path(tool);
    let stat = std::fs::metadata(&path)
        .map(|m| {
            let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
            (t.map(|d| d.as_nanos()).unwrap_or(0)) ^ ((m.len() as u128) << 64)
        })
        .unwrap_or(0);
    let cache = CACHE.get_or_init(Default::default);
    if let Ok(m) = cache.lock() {
        if let Some((have, langs)) = m.get(tool) {
            if *have == stat {
                return langs.clone();
            }
        }
    }
    // MACHINE-persistent sidecar (`<tool>.langs.json`, keyed by the crawl file's state): the
    // answer needs the multi-megabyte crawl parsed, every run is a fresh process, and this is
    // asked on every lint — parse once per crawl, not once per run.
    let sidecar = path.with_extension("langs.json");
    let sidecar_hit = std::fs::read_to_string(&sidecar)
        .ok()
        .and_then(|raw| serde_json::from_str::<(String, std::collections::HashSet<String>)>(&raw).ok())
        .and_then(|(have, langs)| (have == format!("{stat:x}")).then_some(langs));
    let langs: std::collections::HashSet<String> = match sidecar_hit {
        Some(langs) => langs,
        None => {
            let langs: std::collections::HashSet<String> = std::fs::read_to_string(&path)
                .ok()
                .and_then(|raw| serde_json::from_str::<CrawledSource>(&raw).ok())
                .map(|c| c.pages.into_iter().map(|p| p.lang).filter(|l| !l.is_empty()).collect())
                .unwrap_or_default();
            if let Ok(json) = serde_json::to_string(&(format!("{stat:x}"), &langs)) {
                let _ = std::fs::write(&sidecar, json);
            }
            langs
        }
    };
    if let Ok(mut m) = cache.lock() {
        m.insert(tool.to_string(), (stat, langs.clone()));
    }
    langs
}

// ── Per-source crawl cache (one network read per machine per source) ──────────

/// One documentation source reduced to exactly what reading consumes — per page, its URL, its
/// readable prose, and the `(slug, governing prose, code example)` block units. Cached once per
/// machine keyed by the toolchain version whose docs these are, so a source shared by two
/// languages (TypeScript ⊇ JavaScript both read MDN) or re-read after a `TRAIN_VERSION` bump
/// costs the network exactly once.
#[cfg(feature = "crawl")]
#[derive(serde::Serialize, serde::Deserialize)]
struct CrawledSource {
    version: String,
    /// Unix seconds of the crawl — setup re-validates against the live pages when this is
    /// older than [`CRAWL_MAX_AGE`] (LINTER.md, "Setup guarantees the documentation is
    /// CURRENT"). Legacy caches default to 0 and re-crawl on the next setup.
    #[serde(default)]
    crawled_at: u64,
    pages: Vec<CrawledPage>,
}

/// How old a source's page cache may be before SETUP re-crawls it — a project is never set up
/// against documentation staler than this. Applies only when the network is allowed (train);
/// replay-only lint runs use whatever setup ensured.
#[cfg(feature = "crawl")]
const CRAWL_MAX_AGE: u64 = 24 * 60 * 60;

/// Now, in unix seconds.
#[cfg(feature = "crawl")]
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// One page of a [`CrawledSource`].
#[cfg(feature = "crawl")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct CrawledPage {
    url: String,
    prose: String,
    /// `(slug, governing prose, code, language hint)` — the hint is the block's OWN declared
    /// language (ledger #18; "" = undeclared ⇒ the page's language). The arity change from the
    /// hintless shape is deliberate cache poison: an old cache fails to parse and recrawls, so
    /// no hintless page can masquerade as attributed.
    units: Vec<(String, String, String, String)>,
    /// The LANGUAGE this page documents (LINTER.md, "A site is a source"): block-hint
    /// majority, else URL typography (path segment / host label through the one resolver),
    /// else "" — reading material for everyone, law for no one. Unhinted blocks inherit it.
    #[serde(default)]
    lang: String,
    /// The server's `Last-Modified` at fetch time — the per-page anchor the verification
    /// sweep revalidates with (`If-Modified-Since` → 304 proves the page current for free).
    #[serde(default)]
    modified: Option<u64>,
    /// Fingerprint of the extracted content — change detection for servers that send no
    /// `Last-Modified` (a refetched body that extracts identically is NOT a change).
    #[serde(default)]
    fp: u64,
}

/// One extracted page: prose + units + freshness anchors, shared by the crawl path and the
/// verification sweep so both always extract identically.
#[cfg(feature = "crawl")]
/// The language `url`'s page documents, resolved as LINTER.md's "A site is a source" orders:
/// the majority language among the page's own block hints, else the first URL path segment
/// that resolves to a known language (or names one in `extra_langs` — languages this run
/// already knows by name, e.g. the project's own), else a host label. "" when nothing
/// answers.
fn attribute_page(
    url: &str,
    units: &[(String, String, String, String)],
    extra_langs: &std::collections::HashSet<String>,
) -> String {
    let known = |token: &str| -> Option<String> {
        let t = token.to_lowercase();
        if extra_langs.contains(&t) {
            return Some(t);
        }
        crate::lint_train::hint_language(&t)
    };
    let mut votes: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    let mut hinted = 0u32;
    for (_, _, _, hint) in units {
        if hint.is_empty() {
            continue;
        }
        if let Some(l) = known(hint) {
            hinted += 1;
            *votes.entry(l).or_default() += 1;
        }
    }
    if let Some((lang, n)) = votes.iter().max_by_key(|(_, n)| **n) {
        if *n * 2 > hinted {
            return lang.clone();
        }
    }
    let path = url.split("://").nth(1).unwrap_or(url);
    let mut segs = path.split('/');
    let host = segs.next().unwrap_or("");
    for seg in segs {
        let seg = seg.split(['?', '#', '.']).next().unwrap_or("");
        if seg.chars().count() >= 2 {
            if let Some(l) = known(seg) {
                return l;
            }
        }
    }
    // Host labels split on '.' AND '-' — sites name their language both ways
    // (`docs.python.org`, `elm-lang.org`), and a hyphen is word typography, not meaning.
    for label in host.split(['.', '-']) {
        if label.chars().count() >= 2 {
            if let Some(l) = known(label) {
                return l;
            }
        }
    }
    String::new()
}

fn extract_crawled_page(
    seed_url: &str,
    url: &str,
    prose: String,
    html: &str,
    modified: Option<u64>,
    extra_langs: &std::collections::HashSet<String>,
) -> CrawledPage {
    let page_slug = rule_slug_under(seed_url, url);
    let blocks = pre_blocks(html);
    let units: Vec<(String, String, String, String)> = block_contexts(html, &blocks)
        .into_iter()
        .zip(blocks.iter())
        .map(|((prose, code), (open, _, _))| {
            let s = page_slug
                .clone()
                .or_else(|| anchor_slug_before(html, *open))
                .unwrap_or_else(|| slug(&prose));
            let hint = crate::doc_crawler::block_lang_hint(html, *open);
            (s, prose, code, hint)
        })
        .collect();
    // Blockless sections form NO units for now (LINTER.md, "A blockless section cannot
    // teach law yet"): lead units were built and measured — under the current classifier
    // they minted only error-register junk while the true prohibition they were built for
    // still failed its span. They re-land with the per-token side-count classifier.
    let lang = attribute_page(url, &units, extra_langs);
    let mut h = crate::lint_ai::token_seed(&prose) ^ crate::lint_ai::token_seed(&lang).rotate_left(4);
    for (a, b, c, d) in &units {
        h ^= crate::lint_ai::token_seed(a)
            ^ crate::lint_ai::token_seed(b).rotate_left(1)
            ^ crate::lint_ai::token_seed(c).rotate_left(2)
            ^ crate::lint_ai::token_seed(d).rotate_left(3);
    }
    CrawledPage { url: url.to_string(), prose, units, modified, fp: h, lang }
}

/// The pages of one documentation source: from the per-source cache
/// (`~/.cache/helpers/lint-index/crawls/<tool>.json`) when it covers `cache_version`, from the
/// network otherwise (writing the cache back). `cache_version: None` (discovery probes) always
/// reads the network and never writes. A process-wide per-tool lock makes parallel languages
/// sharing a source line up: the first crawls, the rest replay its pages from disk.
/// `HELPERS_LINT_REFRESH` bypasses and rewrites the cache.
#[cfg(feature = "crawl")]
fn crawled_source(
    src: &DocsSource,
    max_pages: usize,
    cache_version: Option<&str>,
    extra_langs: &std::collections::HashSet<String>,
) -> Vec<CrawledPage> {
    use crate::doc_crawler::{crawl, fetch};
    // OFFLINE means no network, never no learning: the disk cache below still replays. A
    // cold cache offline yields nothing — the caller reports the language as not-yet-learned
    // and the next online run fills it.
    let offline = !crate::lint_train::network_allowed();
    let read_from_network = || -> Vec<CrawledPage> {
        if offline {
            return Vec::new();
        }
        let mut pages = Vec::new();
        if src.crawl {
            for p in crawl(&[&src.url], max_pages, 0) {
                // Unit identity, best evidence first: a per-rule page's URL slug; else the
                // nearest preceding `id="…"` anchor (single-page rule docs put each rule in
                // its own anchored section — HTML typography); else the prose itself.
                pages.push(extract_crawled_page(&src.url, &p.url, p.prose, &p.html, p.modified, extra_langs));
            }
        } else if let Some((ct, body)) = fetch(&src.url) {
            for (prose, code, hint) in crate::doc_crawler::extract_hinted(&ct, &body) {
                let unit = (slug(&prose), prose.clone(), code, hint);
                let lang = attribute_page(&src.url, std::slice::from_ref(&unit), extra_langs);
                pages.push(CrawledPage { url: src.url.clone(), prose, units: vec![unit], modified: None, fp: 0, lang });
            }
        }
        pages
    };
    // Site caches (tool "site-…") are language-independent: keyed by TRAIN_VERSION, never a
    // toolchain version, so every language trained from the site replays ONE crawl.
    let cache_version = if src.tool.starts_with("site-") {
        Some(crate::lint_train::train_version())
    } else {
        cache_version
    };
    let Some(version) = cache_version else { return read_from_network() };
    // The units-format marker rides the cached version (LINTER.md, "A blockless section
    // still teaches law"): cached pages store FORMED units, not HTML, so a unit-former
    // change cannot re-form them offline — poisoning the version re-crawls instead of
    // silently keeping old units.
    let version = format!("{version}@{UNITS_FORMAT}");
    let lock = source_lock(&src.tool);
    let _guard = lock.lock().expect("source lock poisoned");
    let path = crawl_cache_path(&src.tool);
    let cached = if std::env::var_os("HELPERS_LINT_REFRESH").is_none() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<CrawledSource>(&raw).ok())
            .filter(|c| c.version == version && !c.pages.is_empty())
    } else {
        None
    };
    if let Some(cached) = cached {
        // Freshness is owned by the verification sweep ([`refresh_language_pages`], run at
        // setup before modules build) — a version-matched cache is what setup ensured.
        return cached.pages;
    }
    let pages = read_from_network();
    if !pages.is_empty() {
        write_crawl_cache(&path, &version, &pages);
    }
    pages
}

/// Units-format marker folded into every cached version — bump when the unit FORMER
/// changes shape or coverage (cached pages cannot re-form units without the HTML).
#[cfg(feature = "crawl")]
const UNITS_FORMAT: &str = "u4-intact-lead-heading-excluded";

/// Persist a source's crawled pages with the crawl timestamp (see [`CrawledSource`]).
#[cfg(feature = "crawl")]
fn write_crawl_cache(path: &std::path::Path, version: &str, pages: &[CrawledPage]) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let entry = CrawledSource {
        version: version.to_string(),
        crawled_at: unix_now(),
        pages: pages.to_vec(),
    };
    if let Ok(json) = serde_json::to_string(&entry) {
        let _ = std::fs::write(path, json);
    }
}

/// This source's crawl-cache file, its tool id sanitized to a safe file name.
#[cfg(feature = "crawl")]
fn crawl_cache_path(tool: &str) -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let safe: String = tool
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    std::path::Path::new(&home)
        .join(".cache/helpers/lint-index/crawls")
        .join(format!("{safe}.json"))
}

/// The process-wide lock for one source's crawl — parallel languages sharing a source serialize
/// here, so exactly one of them pays the network.
#[cfg(feature = "crawl")]
fn source_lock(tool: &str) -> std::sync::Arc<std::sync::Mutex<()>> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};
    static LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    let map = LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    map.lock().expect("lock map poisoned").entry(tool.to_string()).or_default().clone()
}

// ── Polarity transfer (grounded knowledge is shared across languages) ─────────

/// Where the best grounded classifier lives for cross-language transfer
/// (`<model cache>/polarity.global.bin`, an `HLM1` container). `pub(crate)` so the model cache stamp
/// ([`crate::lint_train`]) can fingerprint the classifier's file state — the classifier shapes
/// construct selection, so a better-read classifier must retrain compiled detectors.
pub(crate) fn global_polarity_path() -> std::path::PathBuf {
    crate::lint_train::model_dir_pub().join("polarity.global.bin")
}

/// Read a polarity classifier from an `HLM1` container at `path`, falling through to that
/// path's legacy `.json` twin (pre-container machines) — read-only: migration happens on the
/// next [`save_global_polarity`], which always writes the container.
fn load_polarity_store(path: &std::path::Path) -> Option<crate::lint_read::Polarity> {
    use crate::lint_codec::Bin as _;
    if let Some(p) = std::fs::read(path)
        .ok()
        .and_then(|b| crate::lint_codec::Dec::open(&b, crate::lint_codec::kind::POLARITY))
        .and_then(|(_, mut d)| crate::lint_read::Polarity::dec(&mut d))
    {
        return Some(p);
    }
    serde_json::from_str(&std::fs::read_to_string(path.with_extension("json")).ok()?).ok()
}

/// One-time migration of a legacy JSON polarity store into its container — the setup
/// sweep's hook ([`crate::lint_train::sweep_legacy_cache`]); the load path stays read-only.
pub(crate) fn migrate_global_polarity() -> bool {
    use crate::lint_codec::Bin as _;
    let path = global_polarity_path();
    let legacy = path.with_extension("json");
    if !legacy.exists() {
        return false;
    }
    if let Some(p) = load_polarity_store(&path) {
        let mut e = crate::lint_codec::Enc::new();
        p.enc(&mut e);
        if std::fs::write(&path, e.finish(crate::lint_codec::kind::POLARITY, &p.votes().to_string())).is_ok() {
            let _ = std::fs::remove_file(&legacy);
        }
    }
    !legacy.exists()
}

/// Keep `p` in the shared store when it is better grounded (more reality-tested votes) than what
/// is already there — the store only ever improves.
fn save_global_polarity(p: &crate::lint_read::Polarity) {
    let path = global_polarity_path();
    let existing_votes = load_polarity_store(&path).map_or(0, |e| e.votes());
    if p.votes() <= existing_votes {
        return;
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    use crate::lint_codec::Bin as _;
    let mut e = crate::lint_codec::Enc::new();
    p.enc(&mut e);
    let _ = std::fs::write(&path, e.finish(crate::lint_codec::kind::POLARITY, &p.votes().to_string()));
    let _ = std::fs::remove_file(path.with_extension("json"));
}

/// The classifier LOCAL documents (project rule files, root `lintPref`, the principles corpus)
/// are read through — the same transferred/global polarity an ungroundable language reads
/// through during a crawl. Local documents carry no toolchain grounding of their own, so they
/// borrow the best classifier available; `None` on a machine that has not yet grounded any
/// reading (and holds no registry module that did).
pub fn document_polarity() -> Option<std::sync::Arc<crate::lint_read::Polarity>> {
    transferred_polarity()
}

/// The classifier an ungroundable language reads through: the machine's shared store — the
/// self-bootstrapped product of the dictionary's meaning network plus grounded web reading
/// (LINTER.md, "Honest grounding labels"). There is NO committed classifier artifact: the
/// system starts from the dictionary and the docs, grounds its own labels against
/// toolchains, and shares the best-read classifier through this store and through
/// per-language registry modules.
fn transferred_polarity() -> Option<std::sync::Arc<crate::lint_read::Polarity>> {
    // Memoized per source-file state: the classifier is consulted once per LANGUAGE per run
    // (project rules, grounding, advice), and re-parsing the store each time dominated
    // warm-run latency. The fingerprint (mtime + size) keeps a long-lived MCP process
    // correct when a crawl updates the global store.
    type Cache =
        std::sync::Mutex<Option<(u128, Option<std::sync::Arc<crate::lint_read::Polarity>>)>>;
    static CACHE: Cache = std::sync::Mutex::new(None);
    let stat = |p: &Path| -> u128 {
        std::fs::metadata(p)
            .map(|m| {
                let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
                (t.map(|d| d.as_nanos()).unwrap_or(0)) ^ ((m.len() as u128) << 64)
            })
            .unwrap_or(0)
    };
    let fp = stat(&global_polarity_path())
        ^ stat(&global_polarity_path().with_extension("json")).rotate_left(2);
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((have, p)) = cache.as_ref() {
        if *have == fp {
            return p.clone();
        }
    }
    let best = load_polarity_store(&global_polarity_path())
        // Shared, not cloned: the classifier carries the reader's frequency arrays, and
        // cloning it per language per run serialized every training thread on this lock.
        .map(std::sync::Arc::new);
    *cache = Some((fp, best.clone()));
    best
}

/// QUERY rule candidates out of a read [`Memory`]: every binding whose WHOLE prose the learned
/// classifier calls a prohibition becomes a rule — id from the binding's slug, description the
/// docs' own prose, bad the bound code; the fix is the next binding on the SAME page whose prose
/// classifies as an endorsement (docs put "use instead" right after the anti-pattern; neutral
/// output blocks between them are skipped, but another page's code is never borrowed). One rule
/// per slug. Returns each rule with its source url for citation. Pure over the memory — offline,
/// deterministic, testable.
///
/// The mint gate is deliberately STRICTER than the compile entry gate (whole span vs some
/// sentence): a sentence-level mint was tried and measured — MDN reference/error sections whose
/// one "cannot be parsed"-register sentence classifies prohibition minted 14 junk rules (105
/// findings on one repo) while the eval warning still failed its span. The whole-span gate is
/// the dam; widening it to sentences is gated on the per-token side-count classifier (LINTER.md,
/// open problems).
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
            // The fix must come from the SAME section as the violation: on a single-page rule
            // document every rule shares the URL, and pairing across anchors once handed one
            // lint's "Use instead" block to the next lint as its good example.
            .take_while(|nb| nb.url == b.url && nb.slug == b.slug)
            .find(|nb| polarity.classify(&nb.prose) == Some(false))
            .map(|nb| nb.code.clone())
            // words → sentences → ORDER (LINTER.md): endorsement needs grounding only
            // reality can give, so an UNREADY classifier cannot find the fix by reading —
            // there, the section's IMMEDIATELY next non-violation block is the fix by the
            // document-order convention (violation first, fix after). A ready classifier
            // never guesses by position (neutral output dumps are skipped, never paired).
            .or_else(|| {
                if polarity.is_ready() {
                    return None;
                }
                memory.bindings[i + 1..]
                    .first()
                    // Same PAGE is the cold pairing scope (an anchorless page's slugs are
                    // prose-derived, never equal), and the reading is RELATIVE: the fix must
                    // read at most half as negated as the violation — a neighboring rule's
                    // own "Never …" block reads comparably negated and is excluded, while
                    // litotes vocabulary in a genuine fix ("the plain form") is not.
                    .filter(|nb| {
                        nb.url == b.url
                            && polarity.negation_bits(&nb.prose) * 2
                                <= polarity.negation_bits(&b.prose)
                    })
                    .map(|nb| nb.code.clone())
            })
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

/// Grounding probes per crawl — a runaway safety valve, not a working limit (LINTER.md: the
/// whole in-scope site is read). Probes run in parallel; 500 covers every real docs site's
/// example diversity while keeping check-mode spawns bounded.
#[cfg(feature = "crawl")]
const MAX_GROUND_CHECKS: usize = 500;

/// Pages of prose the reader learns from — a runaway safety valve sized far above any real
/// documentation site (reading is millions of tokens per second; every crawled page is read).
#[cfg(feature = "crawl")]
const MAX_READ_PAGES: usize = 50_000;

/// Stored associations per language — a runaway safety valve bounding the serialized memory
/// (~2-3 KB per binding), sized so every real docs site's examples all bind.
#[cfg(feature = "crawl")]
const MAX_BINDINGS: usize = 25_000;

/// The "what's normal" reference corpus — a runaway safety valve; the reference-fire and
/// quarantine gates only get MORE accurate as this grows.
#[cfg(feature = "crawl")]
const MAX_REFERENCE: usize = 10_000;

/// The `(governing prose, code)` pair for every block: the prose is what the page wrote BETWEEN
/// the previous code block and this one — sliced at tag boundaries by construction (`</pre>` to
/// `<pre`), so no window can ever open mid-tag and leak attribute junk into what the reader
/// learns. Only the closing words (up to [`GOVERNING_CTX`]) are kept: the sentence immediately
/// above a block is the one that governs it.
fn block_contexts(html: &str, blocks: &[(usize, usize, String)]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (i, (open, _, code)) in blocks.iter().enumerate() {
        let prev_end = if i == 0 { 0 } else { blocks[i - 1].1 };
        // A HEADING is a hard boundary (LINTER.md, "Read"): the prose that governs a block
        // never crosses into the previous section — without this cut a section's window
        // swallowed the neutral intro above it and the mixed span classified as nothing.
        // Slicing AT the '<' of the heading tag keeps the cut tag-safe by construction.
        let window = &html[prev_end..*open];
        let section_start = ["<h1", "<h2", "<h3", "<h4", "<h5", "<h6", "<H1", "<H2", "<H3"]
            .iter()
            .filter_map(|h| window.rfind(h))
            .max()
            .map(|pos| prev_end + pos)
            .unwrap_or(prev_end);
        // The heading is the BOUNDARY, not the prose: its own words belong to the slug, and
        // tag-stripping would weld them onto the section's first sentence
        // ("StatementsNever use…"), hiding the very negation the law states. The governing
        // prose is the section BODY — everything after the heading's closing tag.
        let body_start = html[section_start..*open]
            .find("</h")
            .and_then(|c| {
                html[section_start + c..*open].find('>').map(|g| section_start + c + g + 1)
            })
            .unwrap_or(section_start);
        let prose = crate::doc_crawler::strip_tags(&html[body_start..*open]);
        let tail_start = prose
            .char_indices()
            .rev()
            .scan(0usize, |seen, (idx, _)| {
                *seen += 1;
                Some((idx, *seen))
            })
            .find(|(_, seen)| *seen >= GOVERNING_CTX)
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        // Advance to a word boundary ONLY when the cap actually cut (tail_start > 0):
        // skipping unconditionally ate the FIRST WORD of every governing prose — for doc
        // prohibitions that word is "Never"/"Avoid" itself (the operative negation).
        let tail = if tail_start == 0 {
            prose.trim().to_string()
        } else {
            match prose[tail_start..].find(char::is_whitespace) {
                Some(ws) => prose[tail_start + ws..].trim().to_string(),
                None => prose[tail_start..].trim().to_string(),
            }
        };
        out.push((tail, code.clone()));
    }
    out
}

/// `<pre>…</pre>` blocks of an HTML fragment as `(byte_offset, code_text)`, tags stripped. Offsets
/// let the caller tell which block follows a "Use instead" marker.
fn pre_blocks(html: &str) -> Vec<(usize, usize, String)> {
    use crate::doc_crawler::strip_code;
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = html[from..].find("<pre") {
        let open = from + rel;
        let Some(gt) = html[open..].find('>') else { break };
        let body_start = open + gt + 1;
        let Some(crel) = html[body_start..].find("</pre>") else { break };
        let end = body_start + crel + 6;
        let code = strip_code(&html[body_start..body_start + crel]);
        if code.len() >= 3 {
            out.push((open, end, code));
        }
        from = end;
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

/// Sanitize a raw `id="…"` value into a rule-id slug — the one spelling every anchor-derived
/// id goes through.
fn sanitize_anchor(raw: &str) -> String {
    raw.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect()
}

/// The nearest `id="…"` anchor BEFORE byte `pos` — the section a code block belongs to on a
/// single-page rule document (clippy's lint list anchors every rule's `<article>` with the
/// rule's own name). Pure HTML typography: an attribute the page's author wrote, never a
/// vocabulary. Sanitized like every slug; `None` when no anchor precedes the block.
fn anchor_slug_before(html: &str, pos: usize) -> Option<String> {
    let head = html.get(..pos)?;
    let at = head.rfind("id=\"")?;
    let raw = head[at + 4..].split('"').next()?;
    let s = sanitize_anchor(raw);
    (s.len() >= 2).then_some(s)
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


// ── The added-sources store (`add_source` writes it, training reads it) ───────

/// Public path accessor for the added-sources store — [`crate::lint_train`] reads it to
/// enumerate every language a URL was handed over for (batch training).
pub fn learned_sources_path_pub() -> std::path::PathBuf {
    learned_sources_path()
}

fn learned_sources_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join(".cache/helpers/lint-index/sources.json")
}

/// The added documentation source for `lang`, when a URL was handed over (`add_source`).
/// Legacy `kind:"none"` entries — negative markers from the deleted web-discovery era — read
/// as "no source": the resolution for an unknown language is to ASK, never to search.
pub fn learned_source(lang: &str) -> Option<DocsSource> {
    let raw = std::fs::read_to_string(learned_sources_path()).ok()?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    for e in json["sources"].as_array()? {
        if e["language"].as_str() != Some(lang) {
            continue;
        }
        if e["kind"].as_str() == Some("none") {
            return None;
        }
        return Some(DocsSource {
            url: e["seed"].as_str().unwrap_or("").to_string(),
            crawl: true,
            tool: e["tool"].as_str().unwrap_or(lang).to_string(),
        });
    }
    None
}

/// Persist a handed-over source into the added-sources store (one entry per language; a new
/// URL replaces the old, and any legacy negative marker with it).
fn remember_source(lang: &str, src: &DocsSource) {
    let path = learned_sources_path();
    let mut json: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({ "version": 1, "sources": [] }));
    let entry = serde_json::json!({
        "tool": src.tool, "language": lang, "kind": "crawl", "seed": src.url
    });
    if let Some(arr) = json["sources"].as_array_mut() {
        arr.retain(|e| e["language"].as_str() != Some(lang));
        arr.push(entry);
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap_or_default());
}

/// VERIFY 100% of `lang`'s page inventory against the live sites and refresh what moved —
/// the setup-time guarantee that modules always train on the most recent documentation
/// (LINTER.md, "Save"). Every known page gets one conditional request (`If-Modified-Since` →
/// 304 proves it current; no header → the refetched body's extraction fingerprint decides);
/// changed pages re-extract and their links discover NEW pages (an unchanged page's links are
/// already in the inventory, so coverage stays complete without refetching bodies); 404/410
/// drops a page; a transport error keeps the cached page (an outage never erases knowledge).
/// Returns whether anything actually changed; the refreshed inventory is written back either
/// way (the sweep timestamp restarts the verification window).
#[cfg(feature = "crawl")]
pub fn refresh_language_pages(data_root: &Path, lang: &str, version: &str) -> bool {
    // The sweep validates the same composed version the crawl cache is written under.
    let version: &str = &format!("{version}@{UNITS_FORMAT}");
    let mut sources = crate::lint_train::registered_docs_sources(data_root, lang);
    if sources.is_empty() {
        sources.extend(learned_source(lang));
    }
    // Attribution candidates for re-extracted pages — the same name universe the original
    // crawl used.
    let mut extra_langs: std::collections::HashSet<String> =
        crate::lint_train::registered_languages(data_root).into_iter().collect();
    extra_langs.insert(lang.to_lowercase());
    let mut changed_any = false;
    for src in &sources {
        let lock = source_lock(&src.tool);
        let _guard = lock.lock().expect("source lock poisoned");
        let path = crawl_cache_path(&src.tool);
        let Some(cached) = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<CrawledSource>(&raw).ok())
            .filter(|c| c.version == version && !c.pages.is_empty())
        else {
            // Cold or version-mismatched: the module rebuild will crawl it whole.
            changed_any = true;
            continue;
        };
        if unix_now().saturating_sub(cached.crawled_at) < CRAWL_MAX_AGE {
            continue; // verified recently enough
        }
        let (pages, changed) = verify_inventory(src, cached.pages, &extra_langs);
        write_crawl_cache(&path, version, &pages);
        changed_any |= changed;
    }
    changed_any
}

#[cfg(not(feature = "crawl"))]
pub fn refresh_language_pages(_data_root: &Path, _lang: &str, _version: &str) -> bool {
    false
}

/// One source's verification sweep: conditional-revalidate every inventoried page in
/// parallel waves, re-extract what changed, expand to newly-linked in-scope pages, drop what
/// the site removed.
#[cfg(feature = "crawl")]
fn verify_inventory(
    src: &DocsSource,
    cached: Vec<CrawledPage>,
    extra_langs: &std::collections::HashSet<String>,
) -> (Vec<CrawledPage>, bool) {
    use crate::doc_crawler::{extract_anchors, extract_prose, fetch_conditional, in_scope, Revalidation};
    const WAVE: usize = 192;
    let known: std::collections::HashMap<String, CrawledPage> =
        cached.into_iter().map(|p| (p.url.clone(), p)).collect();
    let mut seen: std::collections::HashSet<String> = known.keys().cloned().collect();
    let mut worklist: Vec<String> = known.keys().cloned().collect();
    if seen.insert(src.url.clone()) {
        worklist.push(src.url.clone());
    }
    let mut out: Vec<CrawledPage> = Vec::new();
    let mut changed = false;
    while !worklist.is_empty() && seen.len() < 40_000 {
        let batch: Vec<String> = std::mem::take(&mut worklist);
        let mut next: Vec<String> = Vec::new();
        for wave in batch.chunks(WAVE) {
            let results: Vec<(String, Revalidation)> = std::thread::scope(|scope| {
                let handles: Vec<_> = wave
                    .iter()
                    .map(|url| {
                        let since = known.get(url).and_then(|p| p.modified);
                        scope.spawn(move || fetch_conditional(url, since))
                    })
                    .collect();
                wave.iter()
                    .cloned()
                    .zip(handles.into_iter().map(|h| {
                        h.join().unwrap_or(Revalidation::Unreachable)
                    }))
                    .collect()
            });
            for (url, outcome) in results {
                match outcome {
                    Revalidation::NotModified => {
                        if let Some(p) = known.get(&url) {
                            out.push(p.clone());
                        }
                    }
                    Revalidation::Gone => {
                        if known.contains_key(&url) {
                            changed = true;
                        }
                    }
                    Revalidation::Unreachable => {
                        if let Some(p) = known.get(&url) {
                            out.push(p.clone());
                        }
                    }
                    Revalidation::Changed(_, body, modified) => {
                        let fresh = extract_crawled_page(&src.url, &url, extract_prose(&body), &body, modified, extra_langs);
                        // A refetch that extracts identically is not a change (servers
                        // without Last-Modified answer 200 every time).
                        if known.get(&url).map(|p| p.fp) != Some(fresh.fp) {
                            changed = true;
                        }
                        for (link, _) in extract_anchors(&url, &body) {
                            if in_scope(&src.url, &link) && seen.insert(link.clone()) {
                                next.push(link);
                            }
                        }
                        out.push(fresh);
                    }
                }
            }
        }
        worklist = next;
    }
    (out, changed)
}

/// Register `url` as `lang`'s official documentation (LINTER.md, "Online to set up, offline
/// to run"). A data write into the discovery cache, overwriting any failed discovery's
/// negative marker, so every later train/lint resolves the language's docs to this URL.
/// Offline-safe; the caller invalidates the language's model stamp and TRAINING learns it.
pub fn add_docs_source(lang: &str, url: &str) -> DocsSource {
    let tool = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .and_then(|rest| rest.split('/').next())
        .unwrap_or(lang)
        .trim_start_matches("www.")
        .to_string();
    let src = DocsSource { url: url.to_string(), crawl: true, tool };
    remember_source(lang, &src);
    // The manifest is the user-facing record (LINTER.md, "The language manifest") — the
    // legacy store above stays only as migration input for older machines.
    crate::lint_train::manifest_set(lang, vec![url.to_string()]);
    src
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
        Memory { bindings, reference: Vec::new(), polarity: Some(polarity), pages_read: units.len(), extensions: Default::default(), flagged: Default::default() }
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
    #[cfg(feature = "crawl")]
    fn a_blockless_section_forms_no_unit_until_the_classifier_can_read_it() {
        // MDN's "Never use direct eval()!" shape: a heading anchor, prose and bullets, zero
        // <pre>. LEAD units for such sections were built and MEASURED (LINTER.md, "A
        // blockless section cannot teach law yet"): under the span classifier they minted
        // only error-register junk while the prohibition they were built for still failed
        // its span — so blockless sections deliberately form no unit until the per-token
        // side-count classifier lands. This pins the decision.
        let html = r#"
            <h2 id="using_flurb">Using flurb</h2>
            <p>The flurb function combines values.</p>
            <pre>flurb(a, b);</pre>
            <h2 id="never_use_zap">Never use zap!</h2>
            <p>Never use a global mutable variable here. zap() is dangerous and simply wrong.</p>
        "#;
        let page = extract_crawled_page(
            "https://d/docs",
            "https://d/docs/zap",
            crate::doc_crawler::extract_prose(html),
            html,
            None,
            &Default::default(),
        );
        assert!(
            page.units.iter().all(|(_, _, code, _)| code.len() >= 3),
            "every unit carries a code block: {:?}",
            page.units.iter().map(|(s, _, c, _)| (s.clone(), c.len())).collect::<Vec<_>>()
        );
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
    fn added_sources_round_trip_and_legacy_negative_markers_read_as_absent() {
        let dir = std::env::temp_dir().join(format!("lint-docs-test-{}", std::process::id()));
        let _env = crate::test_env_lock();
        std::env::set_var("HOME", &dir); // the added-sources store lives under $HOME
        add_docs_source("zig", "https://ziglang.org/documentation/");
        assert_eq!(learned_source("zig").unwrap().url, "https://ziglang.org/documentation/");
        // A legacy negative marker (from the deleted web-discovery era) means "no source" —
        // and a handed-over URL replaces it.
        let store = learned_sources_path();
        let mut json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&store).unwrap()).unwrap();
        json["sources"].as_array_mut().unwrap().push(serde_json::json!({
            "tool": "brainfuck", "language": "brainfuck", "kind": "none"
        }));
        std::fs::write(&store, json.to_string()).unwrap();
        assert!(learned_source("brainfuck").is_none(), "legacy negative marker reads as absent");
        add_docs_source("brainfuck", "https://example.org/bf-docs/");
        assert_eq!(learned_source("brainfuck").unwrap().url, "https://example.org/bf-docs/");
        assert!(learned_source("cobol").is_none(), "a language nobody added has no source");
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

}

#[cfg(test)]
mod transfer_probe {
    use super::*;

    /// SELF-BOOTSTRAP contract: a classifier built from NOTHING but honestly-labeled reading
    /// (no committed artifact, no curated seed) must render verdicts on plain normative prose
    /// it has never seen — that is the whole point of transfer to ungroundable languages. The
    /// live equivalent (the machine's own store after grounded web reading) is exercised by
    /// training; this is the hermetic floor.
    #[test]
    fn self_bootstrapped_classifier_reads_unseen_normative_prose() {
        let p = crate::lint_read::Polarity::from_labeled(&[
            ("this call is deprecated and fails with a syntax error", true),
            ("the following example throws and must not be used", true),
            ("wrong: this form is invalid and raises an exception", true),
            ("instead, prefer the supported form shown in the fix", false),
            ("the corrected example passes and is the recommended pattern", false),
        ]);
        assert_eq!(
            p.classify("Never use the goto statement anywhere; it is deprecated and will be removed."),
            Some(true),
            "unseen prohibition prose classifies bad"
        );
        assert_ne!(
            p.classify("The flowlang language reference."),
            Some(true),
            "a neutral title must not classify as a prohibition"
        );
    }

    /// DEV TOOL — regenerates `lint-index/extensions-bootstrap.json` from this machine's crawl
    /// page caches: for every registered source, the same reading (prose learned by a
    /// [`Reader`]) and the same typographic tally the live pipeline uses, head-filtered per
    /// language — learned data end to end, committed so cold machines are wired before their
    /// first training run (LINTER.md, "File types are learned by reading"). Run after reading
    /// or tally changes: `cargo test --release generate_extensions_bootstrap -- --ignored`
    #[test]
    #[ignore = "dev tool: regenerates the committed extensions bootstrap from cached crawls"]
    fn generate_extensions_bootstrap() {
        let data_root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
        let raw = crate::lint_train::lint_index_file(&data_root, "sources.json")
            .expect("sources.json exists");
        let json: serde_json::Value = serde_json::from_str(&raw).expect("sources.json parses");
        // One tally per LANGUAGE (a language may register several sources).
        let mut per_lang: std::collections::BTreeMap<String, std::collections::BTreeMap<String, u32>> =
            std::collections::BTreeMap::new();
        for src in json["sources"].as_array().into_iter().flatten() {
            let (Some(tool), Some(lang)) = (src["tool"].as_str(), src["language"].as_str())
            else {
                continue;
            };
            let Ok(raw) = std::fs::read_to_string(crawl_cache_path(tool)) else { continue };
            let Ok(cached) = serde_json::from_str::<CrawledSource>(&raw) else { continue };
            let tally = per_lang.entry(lang.to_string()).or_default();
            for page in &cached.pages {
                crate::lint_read::tally_dotted_tokens(&page.prose, tally);
                crate::lint_read::tally_name_aliases(lang, &page.prose, tally);
                for (_, _, code, _) in &page.units {
                    crate::lint_read::tally_dotted_tokens(code, tally);
                }
            }
        }
        assert!(!per_lang.is_empty(), "no crawl caches found — run `lint_config action=train` first");
        let claims: std::collections::BTreeMap<String, std::collections::BTreeMap<String, u32>> =
            per_lang;
        for (lang, exts) in &claims {
            let mut top: Vec<(&String, &u32)> = exts.iter().collect();
            top.sort_by(|a, b| b.1.cmp(a.1));
            let show: Vec<String> =
                top.iter().take(5).map(|(e, c)| format!(".{e}×{c}")).collect();
            println!("{lang}: {}", show.join(" "));
        }
        let out = data_root.join("lint-index/extensions-bootstrap.json");
        std::fs::write(&out, serde_json::to_string_pretty(&claims).expect("serializes"))
            .expect("bootstrap written");
        println!("wrote {}", out.display());
    }

}
