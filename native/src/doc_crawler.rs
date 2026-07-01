//! `doc_crawler` — a direct-fetch graph crawler over official documentation. No browser.
//!
//! Seed it with a documentation homepage (or a few seeds) and it walks the site as a graph:
//! fetch a page over plain HTTP, pull its in-domain links, follow them breadth-first, and keep
//! going until it has seen the whole doc tree. From each page it extracts the prose and the code
//! blocks — the raw material the net trains on. The point is autonomy: handed only the official
//! docs the language's own creators publish, it finds *everything*, and becomes an expert on that
//! language from the source of truth.
//!
//! The HTML handling is deliberately dependency-light string scanning (links, `<pre>`/`<code>`
//! blocks, tag-stripped prose) — robust enough for documentation, and pure functions so they are
//! unit-tested offline. Only [`fetch`]/[`crawl`] touch the network, behind the `crawl` feature, so
//! the default binary stays browser-free and dependency-light.

/// One crawled page reduced to what training needs.
#[derive(Debug, Clone)]
pub struct Page {
    /// The page URL.
    pub url: String,
    /// Tag-stripped prose of the whole page.
    pub prose: String,
    /// Code blocks found on the page (`<pre>` / `<code>` contents).
    pub code: Vec<String>,
    /// `(local prose, code)` pairs — each snippet with the explanation right before it. This is
    /// the clean training material; `prose`/`code` are kept for inspection.
    pub sections: Vec<(String, String)>,
    /// The raw fetched body. Kept so a caller can run a structure-aware per-page extractor (e.g. a
    /// rule page's ordered `<pre>` blocks + incorrect/correct markers) instead of the lossy
    /// flattened sections. Held only for the lifetime of the returned crawl.
    pub html: String,
}

/// Decode the handful of HTML entities that actually appear in docs prose/code.
fn decode_entities(s: &str) -> String {
    let mut out = s
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
        .replace("&rsquo;", "'")
        .replace("&lsquo;", "'")
        .replace("&mdash;", "—");
    // Numeric decimal entities (&#NN;) — best effort for the common ASCII range.
    while let Some(i) = out.find("&#") {
        let rest = &out[i + 2..];
        if let Some(semi) = rest.find(';') {
            if let Ok(n) = rest[..semi].parse::<u32>() {
                if let Some(c) = char::from_u32(n) {
                    out.replace_range(i..i + 2 + semi + 1, &c.to_string());
                    continue;
                }
            }
        }
        break;
    }
    out
}

/// Remove HTML tags from a fragment, decode entities, collapse whitespace.
pub fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    decode_entities(&out).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Like [`strip_tags`], but for CODE: removes tags and decodes entities while PRESERVING line
/// structure. Prose collapses all whitespace (a paragraph is one logical line), but code is
/// newline-significant — collapsing a multi-line snippet onto one line makes it unparseable (a
/// `for`/`break`/`def` body vanishes), which silently destroys every multi-line example the model
/// learns from. Trailing spaces per line are trimmed and surrounding blank lines dropped.
pub fn strip_code(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    decode_entities(&out)
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_matches('\n')
        .to_string()
}

/// Extract the contents of every `<pre …>…</pre>` and `<code …>…</code>` block as code text.
pub fn extract_code_blocks(html: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    for (open, close) in [("<pre", "</pre>"), ("<code", "</code>")] {
        let mut rest = html;
        while let Some(start) = rest.find(open) {
            let after_open = &rest[start..];
            let Some(gt) = after_open.find('>') else { break };
            let body_start = start + gt + 1;
            let Some(end_rel) = rest[body_start..].find(close) else { break };
            let body = &rest[body_start..body_start + end_rel];
            let code = strip_code(body);
            if code.len() >= 3 {
                blocks.push(code);
            }
            rest = &rest[body_start + end_rel + close.len()..];
        }
    }
    blocks
}

/// Extract `(local prose, code)` sections from a fetched body of ANY textual type — not just
/// HTML. Documentation knowledge lives in JSON (machine-readable rule data), Markdown, and plain
/// text too, so the right extractor is chosen by the server's content type. Binary types never
/// reach here (rejected at fetch). This is what lets the crawler pull *everything* a site serves.
pub fn extract(content_type: &str, body: &str) -> Vec<(String, String)> {
    let ct = content_type.to_lowercase();
    if ct.contains("json") {
        extract_sections_json(body)
    } else if ct.contains("html") || ct.contains("xml") || body.contains("</") {
        extract_sections_html(body)
    } else {
        // Markdown / reStructuredText / plain text — fenced code blocks with their lead-in prose.
        extract_sections_text(body)
    }
}

/// Sections from a Markdown/plain-text body: each fenced ```code``` block paired with the prose
/// just before it. No language assumptions — the optional info string after the fence is dropped.
pub fn extract_sections_text(text: &str) -> Vec<(String, String)> {
    let parts: Vec<&str> = text.split("```").collect();
    let mut out = Vec::new();
    let mut i = 1;
    while i < parts.len() {
        let block = parts[i];
        // Drop the fence info string (the first line after ```), keep the code body.
        let code = block.split_once('\n').map(|(_, c)| c).unwrap_or(block).trim();
        let local: String = words_tail(parts[i - 1], 40);
        if code.len() >= 3 && local.len() >= 8 {
            out.push((local, code.to_string()));
        }
        i += 2; // parts alternate prose / code / prose / code …
    }
    out
}

/// Sections from a JSON body: walk to every string leaf and run the text/HTML extractor on it, so
/// a rules file whose fields embed Markdown or HTML examples (e.g. clippy's `lints.json` `docs`)
/// yields its (prose, code) pairs — no knowledge of the schema's field names required.
pub fn extract_sections_json(body: &str) -> Vec<(String, String)> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let mut strings = Vec::new();
    collect_json_strings(&value, &mut strings);
    let mut out = Vec::new();
    for s in strings {
        if s.contains("```") {
            out.extend(extract_sections_text(&s));
        } else if s.contains("</") {
            out.extend(extract_sections_html(&s));
        }
    }
    out
}

/// Recursively gather every string leaf in a JSON value.
fn collect_json_strings(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(a) => a.iter().for_each(|x| collect_json_strings(x, out)),
        serde_json::Value::Object(o) => o.values().for_each(|x| collect_json_strings(x, out)),
        _ => {}
    }
}

/// The largest byte index `<= i` that lies on a UTF-8 char boundary of `s` (a stable stand-in for
/// the unstable `str::floor_char_boundary`). Slicing at a raw byte offset computed by arithmetic can
/// land inside a multi-byte char and panic; flooring it first keeps the slice safe.
fn floor_char_boundary(s: &str, i: usize) -> usize {
    let mut i = i.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// The last `n` whitespace-separated words of `s`, in order — the local lead-in prose.
fn words_tail(s: &str, n: usize) -> String {
    let w: Vec<&str> = s.split_whitespace().collect();
    w[w.len().saturating_sub(n)..].join(" ")
}

/// Pair each code block with the prose immediately before it — its local explanation — instead
/// of the whole page. Documentation puts the lesson for a snippet right above the snippet; whole-
/// page pairing instead lets one ubiquitous construct (a doctest `assert_eq!`) co-occur with every
/// concept on the page and blur the signal. The local window keeps each (prose, code) record tight.
pub fn extract_sections_html(html: &str) -> Vec<(String, String)> {
    let h = drop_script_style(html);
    let mut out = Vec::new();
    for (open, close) in [("<pre", "</pre>"), ("<code", "</code>")] {
        let mut search_from = 0usize;
        while let Some(rel) = h[search_from..].find(open) {
            let start = search_from + rel;
            let after_open = &h[start..];
            let Some(gt) = after_open.find('>') else { break };
            let body_start = start + gt + 1;
            let Some(end_rel) = h[body_start..].find(close) else { break };
            let code = strip_code(&h[body_start..body_start + end_rel]);
            // Local context: the ~1500 bytes of markup before this block, tag-stripped, last words.
            // Floor the window start to a char boundary — `start - 1500` can land inside a multi-byte
            // char (docs prose has emoji/punctuation), which would panic the slice.
            let ctx_start = floor_char_boundary(&h, start.saturating_sub(1500));
            let local = words_tail(&strip_tags(&h[ctx_start..start]), 40);
            if code.len() >= 3 && local.len() >= 8 {
                out.push((local, code));
            }
            search_from = body_start + end_rel + close.len();
        }
    }
    out
}

/// Strip `<script>`/`<style>` blocks from HTML.
fn drop_script_style(html: &str) -> String {
    let mut h = html.to_string();
    for (open, close) in [("<script", "</script>"), ("<style", "</style>")] {
        while let Some(s) = h.find(open) {
            if let Some(e) = h[s..].find(close) {
                h.replace_range(s..s + e + close.len(), " ");
            } else {
                break;
            }
        }
    }
    h
}

/// Tag-stripped prose of a whole page (after dropping script/style).
pub fn extract_prose(html: &str) -> String {
    strip_tags(&drop_script_style(html))
}

/// The (scheme, host, path) of a URL — minimal, enough to resolve doc links and stay in-domain.
fn split_url(url: &str) -> Option<(String, String, String)> {
    let (scheme, rest) = url.split_once("://")?;
    let (host, path) = match rest.find('/') {
        Some(i) => (rest[..i].to_string(), rest[i..].to_string()),
        None => (rest.to_string(), "/".to_string()),
    };
    Some((scheme.to_string(), host, path))
}

/// Collapse `.`/`..` segments in a URL path so scope checks and dedup see canonical paths
/// (otherwise `/std/vec/../../static.files/x.css` lexically "starts with" `/std/vec`).
fn normalize_path(path: &str) -> String {
    let mut stack: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                stack.pop();
            }
            s => stack.push(s),
        }
    }
    format!("/{}", stack.join("/"))
}

/// Resolve `href` against `base` into an absolute, path-normalized URL (drops fragments). Handles
/// absolute, protocol-relative, root-relative, and relative links. It does NOT guess which links
/// are "assets" by extension — what is and isn't a document is decided generally, by the content
/// type the server returns at fetch time, so the crawler works for any site in any language
/// without a hardcoded file-type list.
pub fn resolve(base: &str, href: &str) -> Option<String> {
    let href = href.split('#').next().unwrap_or(href).trim();
    if href.is_empty() {
        return None;
    }
    // Exclude only non-HTTP URI schemes (mailto:, javascript:, tel:, data:) — they can't be
    // fetched. This is a scheme check, not a content/extension guess: a `:` appearing before the
    // first `/` marks a scheme.
    if let Some(colon) = href.find(':') {
        let first_slash = href.find('/').unwrap_or(usize::MAX);
        if colon < first_slash && !matches!(&href[..colon], "http" | "https") {
            return None;
        }
    }
    let (scheme, host, path) = split_url(base)?;
    let raw = if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if let Some(rest) = href.strip_prefix("//") {
        format!("{scheme}://{rest}")
    } else if let Some(rest) = href.strip_prefix('/') {
        format!("{scheme}://{host}/{rest}")
    } else {
        let dir = path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        format!("{scheme}://{host}{dir}/{href}")
    };
    let (rscheme, rhost, rpath) = split_url(&raw)?;
    Some(format!("{rscheme}://{rhost}{}", normalize_path(&rpath)))
}

/// Extract `(url, anchor_text)` for every link — the anchor text is the human label for where a
/// link goes, the strongest pre-fetch hint of whether it leads to real documentation.
pub fn extract_anchors(base: &str, html: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find("<a") {
        let tag_and_after = &rest[i..];
        let Some(gt) = tag_and_after.find('>') else { break };
        let tag = &tag_and_after[..gt];
        let after = &tag_and_after[gt + 1..];
        let anchor = match after.find("</a>") {
            Some(e) => strip_tags(&after[..e]),
            None => String::new(),
        };
        if let Some(href) = attr_value(tag, "href") {
            if let Some(u) = resolve(base, &href) {
                out.push((u, anchor));
            }
        }
        rest = after;
    }
    out
}

/// Read an attribute's value out of a tag's text (`href="…"` / `href='…'`).
fn attr_value(tag: &str, name: &str) -> Option<String> {
    for q in ['"', '\''] {
        let needle = format!("{name}={q}");
        if let Some(i) = tag.find(&needle) {
            let after = &tag[i + needle.len()..];
            if let Some(end) = after.find(q) {
                return Some(after[..end].to_string());
            }
        }
    }
    None
}

/// Extract and resolve every link URL on the page (anchor text discarded).
pub fn extract_links(base: &str, html: &str) -> Vec<String> {
    extract_anchors(base, html).into_iter().map(|(u, _)| u).collect()
}

/// A learned crawl frontier: it builds a fingerprint of what links to GOOD documentation pages
/// look like (their URL + anchor words), then scores unvisited links by similarity to it. The
/// crawler visits the highest-scoring links first and the model evolves as it reads — so the walk
/// is guided by what it has learned leads to content, not a blind queue. Cold (nothing learned
/// yet) every link scores 0 and order falls back to discovery order.
#[derive(Default)]
pub struct Frontier {
    prototype: crate::lint_ai::Bundler,
    learned: usize,
}

/// Fingerprint a link's words (its URL path tokens + anchor text) into a hypervector.
fn link_fingerprint(url: &str, anchor: &str) -> crate::lint_ai::Hv {
    let path = split_url(url).map(|(_, _, p)| p).unwrap_or_default();
    let mut b = crate::lint_ai::Bundler::new();
    for tok in path.split(|c: char| !c.is_alphanumeric()).chain(anchor.split_whitespace()) {
        let t = tok.to_lowercase();
        if t.len() >= 2 {
            b.add(&crate::lint_ai::token_hv(&t));
        }
    }
    if b.is_empty() {
        crate::lint_ai::Hv::zero()
    } else {
        b.finalize()
    }
}

impl Frontier {
    /// A fresh frontier with nothing learned.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the link that led to a page and whether that page was valuable (yielded content).
    /// Valuable links teach the prototype what to chase next.
    pub fn observe(&mut self, url: &str, anchor: &str, valuable: bool) {
        if valuable {
            self.prototype.add(&link_fingerprint(url, anchor));
            self.learned += 1;
        }
    }

    /// Predicted value of an unvisited link in `[0,1000]`: similarity of its words to the learned
    /// "leads to good docs" prototype. `0` until something has been learned.
    pub fn score(&self, url: &str, anchor: &str) -> i64 {
        if self.learned == 0 {
            return 0;
        }
        let proto = self.prototype.finalize();
        let d = link_fingerprint(url, anchor).distance(&proto) as f64;
        ((1.0 - d / crate::lint_ai::DIM as f64) * 1000.0) as i64
    }
}

/// True if `url` belongs to the same host as `seed` and sits under its directory prefix — the
/// "stay inside the official docs" rule that keeps the crawl on-topic and in-domain.
pub fn in_scope(seed: &str, url: &str) -> bool {
    match (split_url(seed), split_url(url)) {
        (Some((_, sh, sp)), Some((_, uh, up))) => {
            let prefix = sp.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
            uh == sh && up.starts_with(prefix)
        }
        _ => false,
    }
}

#[cfg(feature = "crawl")]
mod net {
    use super::*;
    use std::collections::{BinaryHeap, HashSet};
    use std::time::Duration;

    /// Fetch a URL directly over HTTP (no browser). Returns `(content_type, body)` for any TEXTUAL
    /// response — HTML, JSON, Markdown, plain text — and `None` only for true binaries (images,
    /// fonts, archives) or network errors. We keep everything textual a docs site serves; what to
    /// do with it is decided later by content type, not discarded up front.
    pub fn fetch(url: &str) -> Option<(String, String)> {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(15))
            .user_agent("helpers-doc-crawler/1.0 (+direct-fetch)")
            .build();
        let resp = agent.get(url).call().ok()?;
        let ct = resp.content_type().to_string();
        let binary = ct.starts_with("image/")
            || ct.starts_with("font/")
            || ct.starts_with("audio/")
            || ct.starts_with("video/")
            || ct.contains("octet-stream")
            || ct.contains("zip")
            || ct.contains("pdf")
            || ct.contains("wasm");
        if binary {
            return None;
        }
        resp.into_string().ok().map(|body| (ct, body))
    }

    /// Pages with at least this many (prose, code) sections taught the frontier that the link
    /// which led to them is worth chasing — they are documentation, not navigation/landing pages.
    const VALUABLE_SECTIONS: usize = 2;

    /// Crawl the documentation graph from `seeds`, staying in scope, up to `max_pages` — BEST-FIRST,
    /// not blindly. A learned [`Frontier`] scores each unvisited link by how much its URL/anchor
    /// words resemble the links that have led to real content so far, and the crawler always visits
    /// the highest-scoring link next. So it spends its budget on the documentation and evolves
    /// toward the meaty pages as it reads. Returns each page's extracted (prose, code) sections from
    /// whatever textual type it is.
    pub fn crawl(seeds: &[&str], max_pages: usize, delay_ms: u64) -> Vec<Page> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut frontier = Frontier::new();
        // Max-heap of (score, seq, url, anchor); higher score is visited first.
        let mut heap: BinaryHeap<(i64, u64, String, String)> = BinaryHeap::new();
        let mut seq: u64 = 0;
        for s in seeds {
            heap.push((i64::MAX, seq, (*s).to_string(), String::new()));
            seen.insert((*s).to_string());
            seq += 1;
        }
        let mut pages = Vec::new();
        while let Some((_, _, url, anchor)) = heap.pop() {
            if pages.len() >= max_pages {
                break;
            }
            let Some((ct, body)) = fetch(&url) else { continue };
            let sections = extract(&ct, &body);
            // Teach the frontier: did the link that led here pay off in content?
            frontier.observe(&url, &anchor, sections.len() >= VALUABLE_SECTIONS);
            // Score and enqueue new in-scope links by predicted value.
            for (link, atext) in extract_anchors(&url, &body) {
                if seen.len() < max_pages * 8
                    && !seen.contains(&link)
                    && seeds.iter().any(|s| in_scope(s, &link))
                {
                    seen.insert(link.clone());
                    let score = frontier.score(&link, &atext);
                    heap.push((score, seq, link, atext));
                    seq += 1;
                }
            }
            if delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(delay_ms));
            }
            eprintln!("crawled {} ({} sections; {} pages, {} queued)", url, sections.len(), pages.len() + 1, heap.len());
            pages.push(Page { url, prose: extract_prose(&body), code: extract_code_blocks(&body), sections, html: body });
        }
        pages
    }
}

#[cfg(feature = "crawl")]
pub use net::{crawl, fetch};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_code_and_prose() {
        let html = r#"<html><body><h1>Rule</h1><p>Avoid &amp; prefer this.</p>
            <pre><code>let x = y.unwrap();</code></pre>
            <script>var a=1;</script></body></html>"#;
        let code = extract_code_blocks(html);
        assert!(code.iter().any(|c| c.contains("unwrap")), "code block extracted: {code:?}");
        let prose = extract_prose(html);
        assert!(prose.contains("Avoid & prefer this"), "prose decoded: {prose}");
        assert!(!prose.contains("var a=1"), "script content dropped");
    }

    #[test]
    fn resolves_and_scopes_links() {
        let base = "https://doc.rust-lang.org/book/ch01.html";
        assert_eq!(resolve(base, "ch02.html").unwrap(), "https://doc.rust-lang.org/book/ch02.html");
        assert_eq!(resolve(base, "/std/index.html").unwrap(), "https://doc.rust-lang.org/std/index.html");
        assert_eq!(resolve(base, "https://other.com/x").unwrap(), "https://other.com/x");
        // In scope: same host, under the seed's directory. Out: other host or above the path.
        assert!(in_scope("https://doc.rust-lang.org/book/", "https://doc.rust-lang.org/book/ch02.html"));
        assert!(!in_scope("https://doc.rust-lang.org/book/", "https://crates.io/x"));
    }

    #[test]
    fn frontier_learns_which_links_lead_to_content() {
        let mut f = Frontier::new();
        assert_eq!(f.score("https://d/api/Vec.html", "Vec struct"), 0, "cold frontier scores 0");
        // Learn that /api/<Type> links with type-ish anchors led to good pages.
        f.observe("https://d/api/HashMap.html", "HashMap struct", true);
        f.observe("https://d/api/String.html", "String struct", true);
        let relevant = f.score("https://d/api/BTreeMap.html", "BTreeMap struct");
        let irrelevant = f.score("https://d/about/license.html", "license and legal");
        assert!(relevant > irrelevant, "learned frontier ranks api/type links above unrelated ones ({relevant} vs {irrelevant})");
    }

    #[test]
    fn extracts_from_markdown_and_json_not_just_html() {
        // Markdown fenced block + its lead-in prose.
        let md = "Prefer iterators here.\n```rust\nv.iter().map(f).collect()\n```\n";
        let secs = extract("text/markdown", md);
        assert!(secs.iter().any(|(p, c)| p.contains("iterators") && c.contains("iter")), "markdown section: {secs:?}");
        // JSON whose field embeds a markdown example (the lints.json shape) — schema-free.
        let json = r#"{"id":"x","docs":"Avoid this.\n```rust\nfoo.unwrap()\n```"}"#;
        let secs = extract("application/json", json);
        assert!(secs.iter().any(|(_, c)| c.contains("unwrap")), "json-embedded code extracted: {secs:?}");
    }

    #[test]
    fn context_window_across_multibyte_char_does_not_panic() {
        // A `<pre>` preceded by prose containing a multi-byte char positioned so the 1500-byte
        // look-back window starts inside that char — the real ruff-docs crash. Must not panic.
        let prose = format!("{}🛠 fast linter", "x".repeat(1490));
        let html = format!("<p>{prose}</p><pre>code here</pre>");
        let secs = extract_sections_html(&html);
        assert!(secs.iter().any(|(_, c)| c.contains("code here")), "code extracted: {secs:?}");
    }

    #[test]
    fn floor_char_boundary_never_splits_a_char() {
        let s = "ab🛠cd"; // '🛠' is 4 bytes at offsets 2..6
        assert_eq!(floor_char_boundary(s, 3), 2, "floors into the emoji back to its start");
        assert_eq!(floor_char_boundary(s, 2), 2, "already a boundary stays put");
        assert_eq!(floor_char_boundary(s, 100), s.len(), "clamped to len");
    }

    #[test]
    fn extracts_links_from_html() {
        let html = r#"<a href="a.html">A</a> <a href='/b.html'>B</a> <a href="mailto:x@y.z">M</a>"#;
        let links = extract_links("https://d.example/docs/index.html", html);
        assert!(links.iter().any(|l| l.ends_with("/docs/a.html")));
        assert!(links.iter().any(|l| l.ends_with("/b.html")));
        assert!(!links.iter().any(|l| l.contains("mailto")), "mailto dropped");
    }
}
