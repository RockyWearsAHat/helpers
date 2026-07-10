//! `lint_socrawl` — a native, cached fetcher for EXPLANATORY PROGRAMMING PROSE, built into this
//! program (no browser, no external helper). It pulls real Stack Overflow discussion about error
//! handling — the English text where words like "swallow", "catch", "ignore", "suppress",
//! "exception", "error" and "result" are genuinely used and explained — so the character
//! substrate can learn a word's MEANING from usage, not only from its one dictionary definition
//! (LINTER.md, "Meaning is learned from usage, not only definition").
//!
//! It is deliberately small and polite: a handful of Stack Overflow tag LISTING pages are read for
//! their question links, a bounded set of those question pages is fetched in modest parallel waves,
//! and every page is cached RAW on disk with its `Last-Modified` anchor. A refresh REVALIDATES the
//! cache conditionally (`If-Modified-Since` → 304 costs nothing) and only re-downloads what changed
//! — the "download what moved, nothing else" the owner asked for. The HTTP itself reuses the
//! crawler's pooled, keep-alive [`crate::doc_crawler`] agent, so a whole refresh pays TCP+TLS once
//! per host. Network only (`crawl` feature); offline it reads whatever the cache holds.

/// The Stack Overflow tags whose listings are read for their question links. These are the tags
/// DENSE in real error-handling English — the vocabulary a learned "swallow" sense must co-occur
/// with — plus enough breadth that the corpus generalises. Listing pages (not `/search`, which is
/// bot-gated) are read only for the `/questions/<id>/…` links they carry. Deep pagination over each
/// tag (see [`listing_seeds`]) is how the crawl reaches toward "all of Stack Overflow" a batch at a
/// time; the persistent cache means each run resumes where the last left off.
#[cfg(feature = "crawl")]
const TAGS: &[&str] = &[
    // The error-handling core — where "swallow"/"catch"/"ignore"/"suppress"/"exception" actually
    // co-occur, the signal a learned "swallow" sense needs.
    "exception-handling", "try-catch", "error-handling", "exception", "throw", "catch-block",
    "try-catch-finally", "custom-exceptions", "stack-trace", "nullpointerexception",
    "error-logging", "logging", "raise", "rethrow", "finally",
    // BREADTH across many unrelated topics — this is what lets inverse document frequency separate
    // function words (in ~all topics) from topic words (in a few). Without diversity a single-topic
    // corpus makes even "exception" look as generic as "the". Popular, prose-rich general tags:
    "python", "javascript", "java", "c++", "c#", "sql", "html", "css", "arrays", "string",
    "algorithm", "database", "performance", "security", "multithreading", "regex", "json",
    "git", "linux", "memory-management", "pointers", "recursion", "sorting", "data-structures",
    "networking", "concurrency", "oop", "functional-programming", "unit-testing", "debugging",
];

/// Listing-page depth per tag (pages of `?tab=Votes&page=N`). Deep enough that one refresh
/// discovers thousands of question links; the [`MAX_PAGES`] cap and the time budget bound how many
/// are actually fetched, and the cache accumulates the rest on later runs.
#[cfg(feature = "crawl")]
const LISTING_DEPTH: usize = 12;

/// Every listing URL the crawl seeds this run, in PAGE-MAJOR (interleaved) order: page 1 of every
/// tag, then page 2 of every tag, and so on. Interleaving is what keeps the corpus DIVERSE under a
/// page cap — discovery samples across all topics before the cap is hit, instead of draining the
/// first tag to depth and never reaching the rest. Sorted by Votes so the prose-richest questions
/// surface first.
#[cfg(feature = "crawl")]
fn listing_seeds() -> Vec<String> {
    let mut seeds = Vec::with_capacity(TAGS.len() * LISTING_DEPTH);
    for page in 1..=LISTING_DEPTH {
        for tag in TAGS {
            seeds.push(format!("https://stackoverflow.com/questions/tagged/{tag}?tab=Votes&page={page}"));
        }
    }
    seeds
}

/// Upper bound on cached question pages — a MEMORY/DISK safety valve, not a working limit (the
/// owner's directive is "no cap, everything findable"). Set very high so a run pulls everything the
/// frontier yields within its time budget; the persistent cache accumulates across runs toward the
/// whole findable question set. Overridable with `HELPERS_SO_MAX_PAGES`.
#[cfg(feature = "crawl")]
fn max_pages() -> usize {
    std::env::var("HELPERS_SO_MAX_PAGES").ok().and_then(|v| v.parse().ok()).unwrap_or(200_000)
}

/// How many times a throttled (HTTP 429 / transient) fetch is retried with growing backoff before
/// the page is left for the next run. Stack Overflow rate-limits sustained crawling; retrying is
/// what turns a 5%-yield burst into a near-complete polite pull.
#[cfg(feature = "crawl")]
const FETCH_RETRIES: usize = 4;

/// Wall-clock budget for one refresh's question-page fetching (seconds) — politeness over
/// completeness: when it elapses the run stops and saves what it has, and the next run resumes from
/// the cache. Overridable with `HELPERS_SO_BUDGET_SECS`.
#[cfg(feature = "crawl")]
fn budget_secs() -> u64 {
    std::env::var("HELPERS_SO_BUDGET_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(240)
}

/// Concurrent fetches per wave — deliberately small. Stack Overflow throttles aggressive bursts
/// (429s), which both loses pages AND is impolite; a small wave with a short pause between waves
/// (see [`WAVE_PAUSE_MS`]) keeps the success rate high and the site unhammered — the owner's
/// "don't hammer, but as fast as effective" bar.
#[cfg(feature = "crawl")]
const WAVE: usize = 8;

/// Milliseconds to pause between fetch waves — the pacing that keeps Stack Overflow from throttling.
#[cfg(feature = "crawl")]
const WAVE_PAUSE_MS: u64 = 250;

/// One cached explanation page: its URL, the body EXACTLY as served, the server's `Last-Modified`
/// (the conditional-revalidation anchor), and a prose fingerprint (change detection for servers
/// that send no `Last-Modified`). The same raw-cache discipline the doc crawler keeps.
#[derive(Clone)]
pub struct ExplPage {
    /// The question-page URL.
    pub url: String,
    /// The page body exactly as the server sent it.
    pub body: String,
    /// The server's `Last-Modified` at fetch time (unix seconds), when it sent one.
    pub modified: Option<u64>,
    /// Fingerprint of the extracted prose — a refetch whose reading is identical is not a change.
    pub fp: u64,
}

/// The on-disk explanation corpus: when it was last refreshed, and the cached pages.
#[derive(Clone, Default)]
pub struct Explanations {
    /// Unix seconds of the last refresh — the freshness anchor `train` revalidates against.
    pub fetched_at: u64,
    /// The cached question pages.
    pub pages: Vec<ExplPage>,
}

impl crate::lint_codec::Bin for Explanations {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.fixed_u64(self.fetched_at);
        e.u(self.pages.len() as u64);
        for p in &self.pages {
            e.str(&p.url);
            e.str(&p.body);
            e.boolean(p.modified.is_some());
            e.fixed_u64(p.modified.unwrap_or(0));
            e.fixed_u64(p.fp);
        }
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<Explanations> {
        let fetched_at = d.fixed_u64()?;
        let n = d.u()? as usize;
        let mut pages = Vec::with_capacity(n.min(4096));
        for _ in 0..n {
            let url = d.str()?;
            let body = d.str()?;
            let has_modified = d.boolean()?;
            let modified = d.fixed_u64()?;
            let fp = d.fixed_u64()?;
            pages.push(ExplPage { url, body, modified: has_modified.then_some(modified), fp });
        }
        Some(Explanations { fetched_at, pages })
    }
}

/// Where the explanation cache lives, beside the models.
fn cache_path() -> std::path::PathBuf {
    crate::lint_train::model_dir_pub().join("explanations.stackoverflow.bin")
}

/// Now, in unix seconds.
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Load the cached corpus, or `None` when it has not been fetched yet or is unreadable.
pub fn load() -> Option<Explanations> {
    use crate::lint_codec::{Bin, Dec};
    let bytes = std::fs::read(cache_path()).ok()?;
    let (_, mut d) = Dec::open(&bytes, crate::lint_codec::kind::CRAWL)?;
    Explanations::dec(&mut d)
}

/// Persist the corpus (`HLM1`, DATA-stream deflate — HTML compresses several ×).
fn save(corpus: &Explanations) {
    use crate::lint_codec::{Bin, Enc};
    if let Some(parent) = cache_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut e = Enc::new();
    corpus.enc(&mut e);
    let bytes = e.finish(crate::lint_codec::kind::CRAWL, &corpus.fetched_at.to_string());
    let _ = std::fs::write(cache_path(), bytes);
}

/// A Stack Overflow QUESTION-page URL is `/questions/<digits>/<slug>` — never `/questions/tagged/…`
/// (a listing) or `/questions/ask`. This is what turns a listing page's links into the pages that
/// actually carry discussion prose. Fragments and query strings are already dropped by
/// [`crate::doc_crawler::resolve`]; here we only classify the path shape.
fn is_question_url(url: &str) -> bool {
    let (host, path) = match url.split_once("://").and_then(|(_, r)| r.split_once('/')) {
        Some((h, p)) => (h, p),
        None => return false,
    };
    // Stay on Stack Overflow — a listing page links out to many hosts; we learn only from SO's own
    // discussion pages.
    if host != "stackoverflow.com" {
        return false;
    }
    let mut segs = path.split(['/', '?', '#']).filter(|s| !s.is_empty());
    if segs.next() != Some("questions") {
        return false;
    }
    matches!(segs.next(), Some(id) if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
}

/// The prose fingerprint of a page body — the reading-identity used for change detection.
#[cfg(feature = "crawl")]
fn prose_fp(body: &str) -> u64 {
    crate::lint_ai::token_seed(&crate::doc_crawler::extract_prose(body))
}

/// ENSURE the explanation corpus is present and (when `refresh` and online) current, returning it.
/// Offline or cache-fresh, this is a pure disk read; a stale-or-missing cache with the network up
/// triggers a bounded, polite refresh. The returned corpus is what the meaning substrate reads.
#[cfg(feature = "crawl")]
pub fn ensure(refresh: bool) -> Explanations {
    let cached = load();
    // Non-refresh (a normal brain build): use whatever the cache holds; only crawl when there is
    // no cache at all (first ever build). Refresh (an explicit pull): always re-crawl — revalidate
    // the held pages conditionally and discover new ones — regardless of cache age.
    if !refresh {
        if let Some(c) = cached {
            return c;
        }
    }
    let refreshed = crawl(cached);
    save(&refreshed);
    refreshed
}

/// Spawn a plain-GET fetch (body only) for each URL in `wave` on `scope` — the listing-page
/// concurrency helper. Listings need no `Last-Modified` anchor (they are read for links, never
/// cached), so this returns just the body.
#[cfg(feature = "crawl")]
fn scope_fetch<'s, 'e>(
    scope: &'s std::thread::Scope<'s, 'e>,
    wave: &'e [String],
) -> Vec<std::thread::ScopedJoinHandle<'s, Option<String>>> {
    wave.iter().map(|url| scope.spawn(move || fetch_body_with_retry(url))).collect()
}

/// Fetch a listing page's body, RETRYING a throttled miss with growing backoff — the discovery
/// analogue of [`fetch_with_retry`]. Stack Overflow throttles listings too under sustained crawling;
/// without this a rate-limited run discovers nothing.
#[cfg(feature = "crawl")]
fn fetch_body_with_retry(url: &str) -> Option<String> {
    for attempt in 0..=FETCH_RETRIES {
        if let Some((_, body)) = crate::doc_crawler::fetch(url) {
            return Some(body);
        }
        if attempt < FETCH_RETRIES {
            std::thread::sleep(std::time::Duration::from_millis(400 * (attempt as u64 + 1)));
        }
    }
    None
}

/// Fetch one question page, RETRYING a throttled/transient miss with growing backoff. `Changed`
/// (HTTP 200) yields the body; a `Gone` page returns `None` immediately (no point retrying a page
/// that left the site); anything else (429/transport — surfaced as `Unreachable`) is retried up to
/// [`FETCH_RETRIES`] times with an increasing sleep, which is what lifts the yield from a throttled
/// burst to a near-complete polite pull.
#[cfg(feature = "crawl")]
fn fetch_with_retry(url: &str) -> Option<(String, Option<u64>)> {
    use crate::doc_crawler::Revalidation;
    for attempt in 0..=FETCH_RETRIES {
        match crate::doc_crawler::fetch_conditional(url, None) {
            Revalidation::Changed(_, body, modified) => return Some((body, modified)),
            Revalidation::Gone => return None,
            // Throttled or a transient miss — back off and try again.
            Revalidation::NotModified | Revalidation::Unreachable => {
                if attempt < FETCH_RETRIES {
                    std::thread::sleep(std::time::Duration::from_millis(400 * (attempt as u64 + 1)));
                }
            }
        }
    }
    None
}

/// Fetch a wave of URLs concurrently, returning each as `Some((body, modified))` or `None`.
/// Stops launching new waves once `deadline` passes — the politeness budget — so a large frontier
/// degrades to "as much as the budget allowed", the rest left for the next run's cache resume.
#[cfg(feature = "crawl")]
fn fetch_wave(urls: &[String], deadline: std::time::Instant) -> Vec<Option<(String, Option<u64>)>> {
    let mut out = Vec::with_capacity(urls.len());
    for (w, wave) in urls.chunks(WAVE).enumerate() {
        if std::time::Instant::now() >= deadline {
            break;
        }
        if w > 0 {
            std::thread::sleep(std::time::Duration::from_millis(WAVE_PAUSE_MS));
        }
        let got: Vec<Option<(String, Option<u64>)>> = std::thread::scope(|scope| {
            let handles: Vec<_> =
                wave.iter().map(|url| scope.spawn(move || fetch_with_retry(url))).collect();
            handles.into_iter().map(|h| h.join().unwrap_or(None)).collect()
        });
        out.extend(got);
    }
    out
}

/// Refresh the corpus: REVALIDATE every cached page conditionally (keep 304s for free, drop pages
/// that left the site, replace changed bodies), then discover and fetch NEW question pages from the
/// listing seeds up to [`MAX_PAGES`]. Only what moved is downloaded. Transport failure mid-refresh
/// leaves the existing cache intact (the caller still saves what we have).
#[cfg(feature = "crawl")]
fn crawl(cached: Option<Explanations>) -> Explanations {
    use crate::doc_crawler::Revalidation;
    let mut pages: Vec<ExplPage> = Vec::new();
    let mut have: std::collections::HashSet<String> = std::collections::HashSet::new();

    // 1. Revalidate what we already hold — conditional GET, so unchanged pages cost one 304.
    for p in cached.into_iter().flat_map(|c| c.pages) {
        match crate::doc_crawler::fetch_conditional(&p.url, p.modified) {
            Revalidation::NotModified => {
                have.insert(p.url.clone());
                pages.push(p);
            }
            Revalidation::Changed(_, body, modified) => {
                let fp = prose_fp(&body);
                have.insert(p.url.clone());
                pages.push(ExplPage { url: p.url, body, modified, fp });
            }
            // Gone → drop it. Unreachable → keep the cached copy (network hiccup, not a change).
            Revalidation::Gone => {}
            Revalidation::Unreachable => {
                have.insert(p.url.clone());
                pages.push(p);
            }
        }
    }

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(budget_secs());
    let cap = max_pages();

    // 2. Discover new question URLs from the paginated listing seeds (read only for their links).
    // Listings are fetched concurrently too — they are the bulk of the requests at depth.
    let seeds = listing_seeds();
    let mut discovered: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = have.clone();
    for (w, wave) in seeds.chunks(WAVE).enumerate() {
        if std::time::Instant::now() >= deadline || pages.len() + discovered.len() >= cap {
            break;
        }
        if w > 0 {
            std::thread::sleep(std::time::Duration::from_millis(WAVE_PAUSE_MS));
        }
        let bodies: Vec<Option<String>> = std::thread::scope(|scope| {
            let handles: Vec<_> = scope_fetch(scope, wave);
            handles.into_iter().map(|h| h.join().unwrap_or(None)).collect()
        });
        for (seed, body) in wave.iter().zip(bodies) {
            let Some(body) = body else { continue };
            let mut on_page = 0usize;
            for (link, _anchor) in crate::doc_crawler::extract_anchors(seed, &body) {
                if is_question_url(&link) {
                    on_page += 1;
                    if seen.insert(link.clone()) {
                        discovered.push(link);
                    }
                }
            }
            if std::env::var_os("HELPERS_SO_TRACE").is_some() {
                eprintln!("listing {seed} -> {on_page} q-links, {} total discovered", discovered.len());
            }
        }
    }
    discovered.truncate(cap.saturating_sub(pages.len()));

    // 3. Fetch the new question pages in polite waves until the budget elapses.
    let discovered_n = discovered.len();
    let bodies = fetch_wave(&discovered, deadline);
    let mut ok = 0usize;
    for (url, got) in discovered.into_iter().zip(bodies) {
        if let Some((body, modified)) = got {
            let fp = prose_fp(&body);
            pages.push(ExplPage { url, body, modified, fp });
            ok += 1;
        }
    }
    if std::env::var_os("HELPERS_SO_TRACE").is_some() {
        eprintln!("crawl: {discovered_n} new discovered, {ok} fetched ok, {} pages total", pages.len());
    }

    Explanations { fetched_at: unix_now(), pages }
}

/// Read every cached page's prose into `net` as CO-OCCURRENCE — the substrate learns each word's
/// companions across real explanations. Prose is split into sentence windows (the natural
/// co-occurrence unit) and each window's content words are observed together. Returns
/// `(pages, sentences)` read, for the setup report. The caller [`MeaningNetwork::seal`]s afterward.
pub fn learn_into(corpus: &Explanations, net: &mut crate::lint_char::MeaningNetwork) -> (usize, usize) {
    let mut sentences = 0usize;
    for page in &corpus.pages {
        let prose = crate::doc_crawler::extract_prose(&page.body);
        sentences += net.observe_prose(&prose);
    }
    (corpus.pages.len(), sentences)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_question_urls() {
        assert!(is_question_url("https://stackoverflow.com/questions/921217/how-to-swallow"));
        assert!(is_question_url("https://stackoverflow.com/questions/921217"));
        // Listings, the ask page, and other hosts are not question pages.
        assert!(!is_question_url("https://stackoverflow.com/questions/tagged/try-catch"));
        assert!(!is_question_url("https://stackoverflow.com/questions/ask"));
        assert!(!is_question_url("https://example.com/questions/1/x"));
    }

    #[test]
    fn cache_round_trips() {
        use crate::lint_codec::{Bin, Dec, Enc};
        let corpus = Explanations {
            fetched_at: 42,
            pages: vec![ExplPage {
                url: "https://stackoverflow.com/questions/1/x".into(),
                body: "<p>Never swallow an exception; log or rethrow it.</p>".into(),
                modified: Some(1000),
                fp: 7,
            }],
        };
        let mut e = Enc::new();
        corpus.enc(&mut e);
        let bytes = e.finish(crate::lint_codec::kind::CRAWL, "t");
        let (_, mut d) = Dec::open(&bytes, crate::lint_codec::kind::CRAWL).expect("opens");
        let back = Explanations::dec(&mut d).expect("decodes");
        assert_eq!(back.fetched_at, 42);
        assert_eq!(back.pages.len(), 1);
        assert_eq!(back.pages[0].modified, Some(1000));
    }

    #[test]
    fn learns_cooccurrence_from_prose() {
        let corpus = Explanations {
            fetched_at: 0,
            pages: vec![ExplPage {
                url: "u".into(),
                // Real-shaped explanatory prose: swallow co-occurs with ignore/exception/error.
                body: "<p>Do not swallow the exception. Swallowing an error hides the real \
                       failure; catch it, then log or rethrow the exception instead of ignoring \
                       the error.</p>"
                    .into(),
                modified: None,
                fp: 0,
            }],
        };
        let mut net = crate::lint_char::MeaningNetwork::new();
        let (pages, sentences) = learn_into(&corpus, &mut net);
        net.seal();
        assert_eq!(pages, 1);
        assert!(sentences >= 1, "at least one sentence window observed");
        let usage = net.usage_words("swallow").expect("swallow gained a learned sense");
        let companions: Vec<&str> = usage.iter().map(|(w, _)| w.as_str()).collect();
        assert!(
            companions.iter().any(|w| *w == "exception" || *w == "error" || *w == "ignoring"),
            "swallow co-occurs with error-handling vocabulary: {companions:?}"
        );
    }
}
