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
fn strip_tags(html: &str) -> String {
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
            let code = strip_tags(body);
            if code.len() >= 3 {
                blocks.push(code);
            }
            rest = &rest[body_start + end_rel + close.len()..];
        }
    }
    blocks
}

/// Pair each code block with the prose immediately before it — its local explanation — instead
/// of the whole page. Documentation puts the lesson for a snippet right above the snippet; whole-
/// page pairing instead lets one ubiquitous construct (a doctest `assert_eq!`) co-occur with every
/// concept on the page and blur the signal. The local window keeps each (prose, code) record tight.
pub fn extract_sections(html: &str) -> Vec<(String, String)> {
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
            let code = strip_tags(&h[body_start..body_start + end_rel]);
            // Local context: the ~1500 chars of markup before this block, tag-stripped, last words.
            let ctx_start = start.saturating_sub(1500);
            let prose = strip_tags(&h[ctx_start..start]);
            let local: String = prose.split_whitespace().rev().take(40).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join(" ");
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

/// File extensions that are page assets, not documentation — never crawled.
fn is_asset(path: &str) -> bool {
    let p = path.split('?').next().unwrap_or(path);
    [
        ".css", ".js", ".mjs", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico", ".woff",
        ".woff2", ".ttf", ".eot", ".map", ".xml", ".pdf", ".zip", ".wasm", ".webp",
    ]
    .iter()
    .any(|e| p.ends_with(e))
}

/// Resolve `href` against `base` into an absolute, path-normalized URL (drops fragments and page
/// assets). Handles absolute, protocol-relative, root-relative, and relative links.
pub fn resolve(base: &str, href: &str) -> Option<String> {
    let href = href.split('#').next().unwrap_or(href).trim();
    if href.is_empty() || href.starts_with("mailto:") || href.starts_with("javascript:") {
        return None;
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
    if is_asset(&rpath) {
        return None;
    }
    Some(format!("{rscheme}://{rhost}{}", normalize_path(&rpath)))
}

/// Extract and resolve every `href` link on the page.
pub fn extract_links(base: &str, html: &str) -> Vec<String> {
    let mut out = Vec::new();
    for attr in ["href=\"", "href='"] {
        let quote = attr.chars().last().unwrap();
        let mut rest = html;
        while let Some(i) = rest.find(attr) {
            let after = &rest[i + attr.len()..];
            let Some(end) = after.find(quote) else { break };
            if let Some(u) = resolve(base, &after[..end]) {
                out.push(u);
            }
            rest = &after[end + 1..];
        }
    }
    out
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
    use std::collections::{HashSet, VecDeque};
    use std::time::Duration;

    /// Fetch a URL directly over HTTP (no browser). `None` on any network/decode error.
    pub fn fetch(url: &str) -> Option<String> {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(15))
            .user_agent("helpers-doc-crawler/1.0 (+direct-fetch)")
            .build();
        match agent.get(url).call() {
            Ok(resp) => {
                // Documentation pages only — not CSS/JSON/plain-text assets that slip through.
                let ct = resp.content_type().to_string();
                if !(ct.contains("html") || ct.is_empty()) {
                    return None;
                }
                resp.into_string().ok()
            }
            Err(_) => None,
        }
    }

    /// Crawl the documentation graph breadth-first from `seeds`, staying in scope of each seed,
    /// up to `max_pages`. Returns every page's extracted prose + code. Polite fixed delay.
    pub fn crawl(seeds: &[&str], max_pages: usize, delay_ms: u64) -> Vec<Page> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();
        for s in seeds {
            queue.push_back((*s).to_string());
            seen.insert((*s).to_string());
        }
        let mut pages = Vec::new();
        while let Some(url) = queue.pop_front() {
            if pages.len() >= max_pages {
                break;
            }
            let Some(html) = fetch(&url) else { continue };
            for link in extract_links(&url, &html) {
                if seen.len() < max_pages * 8
                    && !seen.contains(&link)
                    && seeds.iter().any(|s| in_scope(s, &link))
                {
                    seen.insert(link.clone());
                    queue.push_back(link);
                }
            }
            pages.push(Page {
                url: url.clone(),
                prose: extract_prose(&html),
                code: extract_code_blocks(&html),
                sections: extract_sections(&html),
            });
            if delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(delay_ms));
            }
            eprintln!("crawled {} ({} pages, {} queued)", url, pages.len(), queue.len());
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
    fn extracts_links_from_html() {
        let html = r#"<a href="a.html">A</a> <a href='/b.html'>B</a> <a href="mailto:x@y.z">M</a>"#;
        let links = extract_links("https://d.example/docs/index.html", html);
        assert!(links.iter().any(|l| l.ends_with("/docs/a.html")));
        assert!(links.iter().any(|l| l.ends_with("/b.html")));
        assert!(!links.iter().any(|l| l.contains("mailto")), "mailto dropped");
    }
}
