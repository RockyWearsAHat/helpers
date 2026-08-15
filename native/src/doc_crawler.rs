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

/// Latched TRUE the first time a network request fails at the TRANSPORT level this run —
/// the wire is down, not "this page had nothing". Callers use [`network_down`] to keep
/// linting from caches, skip further network attempts, avoid caching negative discovery
/// answers, and report honestly (native/architecture.dx, "No connectivity flags").
pub static NET_DOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether any network request this run failed at the transport level (see [`NET_DOWN`]).
pub fn network_down() -> bool {
    NET_DOWN.load(std::sync::atomic::Ordering::Relaxed)
}

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
    /// The server's `Last-Modified` for this page (unix seconds), when sent — the per-page
    /// freshness anchor the verification sweep revalidates against.
    pub modified: Option<u64>,
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
    extract_hinted(content_type, body).into_iter().map(|(p, c, _)| (p, c)).collect()
}

/// [`extract`] carrying each block's own LANGUAGE HINT as a third field — what the page itself
/// declared this block to be (fence info string, `brush:`/`language-*` classes), "" when the
/// block declares nothing. Sites are polyglot by default (native/architecture.dx, ledger #18): the hint is
/// how a reader avoids binding an MDN JavaScript page's HTML example into javascript. The raw
/// declared token is returned verbatim (lowercased); resolving it to a KNOWN language is the
/// caller's judgment ([`crate::lint_train::hint_language`]) — extraction only reports what the
/// author wrote.
pub fn extract_hinted(content_type: &str, body: &str) -> Vec<(String, String, String)> {
    let ct = content_type.to_lowercase();
    if ct.contains("json") {
        extract_sections_json_hinted(body)
    } else if ct.contains("html") || ct.contains("xml") || body.contains("</") {
        extract_sections_html_hinted(body)
    } else {
        // Markdown / reStructuredText / plain text — fenced code blocks with their lead-in prose.
        extract_sections_text_hinted(body)
    }
}

/// Sections from a Markdown/plain-text body: each fenced ```code``` block paired with the prose
/// just before it and the fence's own info-string language label (the docs' declaration of what
/// the block is — kept as the hint, never as an assumption).
pub fn extract_sections_text_hinted(text: &str) -> Vec<(String, String, String)> {
    let parts: Vec<&str> = text.split("```").collect();
    let mut out = Vec::new();
    let mut i = 1;
    while i < parts.len() {
        let block = parts[i];
        // The fence info string (the first line after ```) is the block's language label; the
        // body below it is the code.
        let (info, code) = block.split_once('\n').unwrap_or(("", block));
        let hint: String = info
            .trim()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '#')
            .collect::<String>()
            .to_lowercase();
        let code = code.trim();
        let local: String = words_tail(parts[i - 1], 40);
        if code.len() >= 3 && local.len() >= 8 {
            out.push((local, code.to_string(), hint));
        }
        i += 2; // parts alternate prose / code / prose / code …
    }
    out
}

/// Back-compat pair view of [`extract_sections_text_hinted`].
pub fn extract_sections_text(text: &str) -> Vec<(String, String)> {
    extract_sections_text_hinted(text).into_iter().map(|(p, c, _)| (p, c)).collect()
}

/// The language a code block DECLARES for itself, or "" — pure HTML typography the page's own
/// generator wrote: `class="brush: js"` (MDN), `class="language-css"` / `lang-rust`
/// (Prism/highlight.js and most static generators) on the `<pre …>` open tag at `open`, or on a
/// `<code …>` tag immediately inside it (Node.js docs, Docusaurus). No vocabulary: the token is
/// whatever the author labeled, verbatim.
pub fn block_lang_hint(html: &str, open: usize) -> String {
    let Some(gt) = html[open..].find('>') else { return String::new() };
    let mut tag = html[open..open + gt].to_ascii_lowercase();
    let inner = html[open + gt + 1..].trim_start();
    if inner.starts_with("<code") {
        if let Some(cgt) = inner.find('>') {
            tag.push(' ');
            tag.push_str(&inner[..cgt].to_ascii_lowercase());
        }
    }
    lang_label_in_tag(&tag)
}

/// The first language label inside an open tag's text: the token after `brush:` or after a
/// `language-`/`lang-` class prefix. Alphanumeric plus `+`/`#` so `c++`/`c#` survive.
pub(crate) fn lang_label_in_tag(tag: &str) -> String {
    let token_at = |rest: &str| -> String {
        rest.trim_start()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '#')
            .collect()
    };
    if let Some(i) = tag.find("brush:") {
        let tok = token_at(&tag[i + "brush:".len()..]);
        if !tok.is_empty() {
            return tok;
        }
    }
    for marker in ["language-", " lang-", "\"lang-"] {
        if let Some(i) = tag.find(marker) {
            let tok = token_at(&tag[i + marker.len()..]);
            if !tok.is_empty() {
                return tok;
            }
        }
    }
    String::new()
}

/// Sections from a JSON body: walk to every string leaf and run the text/HTML extractor on it, so
/// a rules file whose fields embed Markdown or HTML examples (e.g. clippy's `lints.json` `docs`)
/// yields its (prose, code) pairs — no knowledge of the schema's field names required.
pub fn extract_sections_json(body: &str) -> Vec<(String, String)> {
    extract_sections_json_hinted(body).into_iter().map(|(p, c, _)| (p, c)).collect()
}

/// [`extract_sections_json`] carrying each embedded block's own language hint.
pub fn extract_sections_json_hinted(body: &str) -> Vec<(String, String, String)> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let mut strings = Vec::new();
    collect_json_strings(&value, &mut strings);
    let mut out = Vec::new();
    for s in strings {
        if s.contains("```") {
            out.extend(extract_sections_text_hinted(&s));
        } else if s.contains("</") {
            out.extend(extract_sections_html_hinted(&s));
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
    extract_sections_html_hinted(html).into_iter().map(|(p, c, _)| (p, c)).collect()
}

/// [`extract_sections_html`] carrying each block's own language hint ([`block_lang_hint`]).
pub fn extract_sections_html_hinted(html: &str) -> Vec<(String, String, String)> {
    // The AI reads the page (native/architecture.dx, "Reading a page is UNDERSTANDING"): units are the char
    // brain's meaning judgment plus its learned structural roles over the raw body — no tag list
    // here. Without a trained brain nothing can read HTML, and nothing pretends to.
    let Some(brain) = crate::lint_char::brain() else {
        return Vec::new();
    };
    let body = drop_script_style(html);
    crate::lint_graph::read_page(&body, brain)
        .into_iter()
        .filter(|u| u.prose.len() >= 8)
        .map(|u| (u.prose, u.code, u.hint))
        .collect()
}

/// Strip `<script>`/`<style>` blocks from HTML.
pub(crate) fn drop_script_style(html: &str) -> String {
    let mut h = html.to_string();
    // Page chrome carries no documentation: scripts/styles are code for the BROWSER, and
    // nav/header/footer/aside are the site's furniture ("Skip to main content", theme pickers)
    // that would otherwise pollute every extracted description.
    for (open, close) in [
        ("<script", "</script>"),
        ("<style", "</style>"),
        ("<nav", "</nav>"),
        ("<header", "</header>"),
        ("<footer", "</footer>"),
        ("<aside", "</aside>"),
    ] {
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

/// True if `url` belongs to the same host as `seed` and sits inside its docs tree — the
/// "stay inside the official docs" rule that keeps the crawl on-topic and in-domain.
///
/// The tree is the seed path itself when its last segment is a directory-like name, and the
/// seed's parent directory when it names a file (`…/bash.html` scopes to its folder). The
/// match is boundary-safe: `/c` covers `/c` and `/c/…`, never `/cpp` — a prefix comparison
/// without the boundary once put cppreference's entire site in one language's scope.
pub fn in_scope(seed: &str, url: &str) -> bool {
    match (split_url(seed), split_url(url)) {
        (Some((_, sh, sp)), Some((_, uh, up))) => {
            if uh != sh {
                return false;
            }
            let sp = sp.trim_end_matches('/');
            let last = sp.rsplit_once('/').map(|(_, f)| f).unwrap_or(sp);
            let tree = if last.contains('.') {
                sp.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
            } else {
                sp
            };
            up.trim_end_matches('/') == tree || up.starts_with(&format!("{tree}/"))
        }
        _ => false,
    }
}

#[cfg(feature = "crawl")]
mod net {
    use super::*;
    use std::collections::HashSet;
    use std::time::Duration;

    /// Fetch a URL directly over HTTP (no browser). Returns `(content_type, body)` for any TEXTUAL
    /// response — HTML, JSON, Markdown, plain text — and `None` only for true binaries (images,
    /// fonts, archives) or network errors. We keep everything textual a docs site serves; what to
    /// do with it is decided later by content type, not discarded up front.
    pub fn fetch(url: &str) -> Option<(String, String)> {
        fetch_meta(url).map(|(ct, body, _)| (ct, body))
    }

    /// [`fetch`] plus the response's `Last-Modified` (unix seconds) — the per-page freshness
    /// anchor the verification sweep stores and revalidates against.
    /// Circuit-breaker state per origin, process-local: (has ever answered, transport failures).
    fn origin_state() -> &'static std::sync::Mutex<std::collections::HashMap<String, (bool, u32)>> {
        static STATE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, (bool, u32)>>> =
            std::sync::OnceLock::new();
        STATE.get_or_init(Default::default)
    }

    /// `scheme://host` of a URL — the breaker's unit (a dead HOST is dead for every path).
    fn origin_of(url: &str) -> String {
        let end = url.find("://").map(|i| i + 3).unwrap_or(0);
        let host_end = url[end..].find('/').map(|i| end + i).unwrap_or(url.len());
        url[..host_end].to_string()
    }

    /// True when this origin transport-failed without ever answering this process.
    fn origin_is_dead(url: &str) -> bool {
        origin_state()
            .lock()
            .map(|m| m.get(&origin_of(url)).is_some_and(|(ok, fails)| !ok && *fails >= 1))
            .unwrap_or(false)
    }

    /// Record that this origin answered (any HTTP status — the wire works).
    fn origin_answered(url: &str) {
        if let Ok(mut m) = origin_state().lock() {
            m.entry(origin_of(url)).or_insert((false, 0)).0 = true;
        }
    }

    /// Record a transport failure against this origin.
    fn origin_failed(url: &str) {
        if let Ok(mut m) = origin_state().lock() {
            m.entry(origin_of(url)).or_insert((false, 0)).1 += 1;
        }
    }

    /// Public reachability verdict for a documentation URL — the setup report's classifier
    /// (native/architecture.dx, "Online to set up"): `true` when the origin answers at all. Probes twice
    /// before saying no (retry — a one-off handshake hiccup is not a dead site); the per-run
    /// breaker remembers the verdict so nothing else pays for it again.
    pub fn origin_reachable(url: &str) -> bool {
        if origin_probe(url) {
            return true;
        }
        // Second opinion: clear only this origin's failure count so the retry actually
        // probes instead of reading the breaker back.
        if let Ok(mut m) = origin_state().lock() {
            m.remove(&origin_of(url));
        }
        origin_probe(url)
    }

    /// Whether `url`'s origin answers AT ALL, decided in seconds: a HEAD with a short
    /// deadline (any HTTP status counts — 405 to HEAD is an answer). Skipped when the
    /// breaker already knows the origin either way. Feeds the breaker so every later fetch
    /// of a dead origin is instant.
    fn origin_probe(url: &str) -> bool {
        {
            let state = origin_state().lock().ok();
            if let Some(m) = state.as_ref() {
                if let Some((ok, fails)) = m.get(&origin_of(url)) {
                    if *ok {
                        return true;
                    }
                    if *fails >= 1 {
                        return false;
                    }
                }
            }
        }
        static PROBE: std::sync::OnceLock<ureq::Agent> = std::sync::OnceLock::new();
        let agent = PROBE.get_or_init(|| {
            ureq::AgentBuilder::new()
                // ureq's overall `timeout` does NOT bound the CONNECT phase (its separate
                // default is 30s) — a host that accepts nothing held every probe for that
                // long. Both phases get the short deadline here.
                .timeout_connect(Duration::from_secs(3))
                .timeout(Duration::from_secs(3))
                .user_agent("helpers-doc-crawler/1.0 (+direct-fetch)")
                .build()
        });
        match agent.head(url).call() {
            Ok(_) | Err(ureq::Error::Status(..)) => {
                origin_answered(url);
                true
            }
            Err(ureq::Error::Transport(_)) => {
                super::NET_DOWN.store(true, std::sync::atomic::Ordering::Relaxed);
                origin_failed(url);
                false
            }
        }
    }

    pub fn fetch_meta(url: &str) -> Option<(String, String, Option<u64>)> {
        // Per-run circuit breaker: an origin that transport-failed before EVER answering is
        // dead for this process — stop paying its timeout on every subsequent page (measured:
        // two unresponsive sites held a 0.4s all-languages train at 60s of pure waiting). A
        // one-off timeout on an origin that HAS answered never trips it, and nothing is
        // cached across runs — the next setup run probes the site fresh (native/architecture.dx: a
        // network failure is reported plainly, never cached as a negative answer).
        if origin_is_dead(url) {
            return None;
        }
        // ONE pooled agent for the whole process: ureq keeps connections alive per host, so a
        // whole-site crawl pays TCP+TLS once per host per lane instead of once per page — a
        // fresh agent per request made the handshake the crawl's dominant wall time.
        static AGENT: std::sync::OnceLock<ureq::Agent> = std::sync::OnceLock::new();
        let agent = AGENT.get_or_init(|| {
            ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(5))
                .timeout(Duration::from_secs(10))
                .max_idle_connections_per_host(256)
                .user_agent("helpers-doc-crawler/1.0 (+direct-fetch)")
                .build()
        });
        let resp = match agent.get(url).call() {
            Ok(resp) => {
                origin_answered(url);
                resp
            }
            // The site ANSWERED (an HTTP status): the network is fine, this URL just has
            // nothing for us.
            Err(ureq::Error::Status(..)) => {
                origin_answered(url);
                return None;
            }
            // TRANSPORT failure: the network itself is unreachable. Latch it — the run keeps
            // linting from caches, callers stop caching negative answers, and the report asks
            // to reconnect instead of failing (native/architecture.dx, "No connectivity flags").
            Err(ureq::Error::Transport(_)) => {
                super::NET_DOWN.store(true, std::sync::atomic::Ordering::Relaxed);
                origin_failed(url);
                return None;
            }
        };
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
        let modified = resp.header("last-modified").and_then(parse_http_date);
        resp.into_string().ok().map(|body| (ct, body, modified))
    }

    /// Fetch a URL's raw BYTES — the registry module path (`HLM1` containers are binary; the
    /// textual [`fetch`] would mangle them). Same breaker, same pooled agent, same transport
    /// latching as [`fetch_meta`]; capped at `max` bytes so a mis-pointed URL cannot balloon
    /// memory (the signed index pins real module sizes far below any sane cap).
    pub fn fetch_bytes(url: &str, max: u64) -> Option<Vec<u8>> {
        if origin_is_dead(url) {
            return None;
        }
        static AGENT: std::sync::OnceLock<ureq::Agent> = std::sync::OnceLock::new();
        let agent = AGENT.get_or_init(|| {
            ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(5))
                .timeout(Duration::from_secs(30))
                .user_agent("helpers-doc-crawler/1.0 (+registry)")
                .build()
        });
        let resp = match agent.get(url).call() {
            Ok(resp) => {
                origin_answered(url);
                resp
            }
            Err(ureq::Error::Status(..)) => {
                origin_answered(url);
                return None;
            }
            Err(ureq::Error::Transport(_)) => {
                super::NET_DOWN.store(true, std::sync::atomic::Ordering::Relaxed);
                origin_failed(url);
                return None;
            }
        };
        let mut body = Vec::new();
        use std::io::Read as _;
        resp.into_reader().take(max).read_to_end(&mut body).ok()?;
        Some(body)
    }

    /// Conditional fetch: `If-Modified-Since` when `since` is known. `NotModified` proves the
    /// page current for free; `Gone` means the page left the site; `Changed` carries the fresh
    /// body. This is how 100% of an inventory is VERIFIED against the live site without
    /// refetching what did not move.
    pub enum Revalidation {
        NotModified,
        Gone,
        Changed(String, String, Option<u64>),
        Unreachable,
    }

    pub fn fetch_conditional(url: &str, since: Option<u64>) -> Revalidation {
        static AGENT: std::sync::OnceLock<ureq::Agent> = std::sync::OnceLock::new();
        let agent = AGENT.get_or_init(|| {
            ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(10))
                .max_idle_connections_per_host(256)
                .user_agent("helpers-doc-crawler/1.0 (+revalidate)")
                .build()
        });
        let mut req = agent.get(url);
        if let Some(since) = since {
            req = req.set("If-Modified-Since", &format_http_date(since));
        }
        match req.call() {
            Ok(resp) => {
                let modified = resp.header("last-modified").and_then(parse_http_date);
                match resp.into_string() {
                    Ok(body) => Revalidation::Changed(String::new(), body, modified),
                    Err(_) => Revalidation::Unreachable,
                }
            }
            Err(ureq::Error::Status(304, _)) => Revalidation::NotModified,
            Err(ureq::Error::Status(404 | 410, _)) => Revalidation::Gone,
            Err(ureq::Error::Status(..)) => Revalidation::Unreachable,
            Err(ureq::Error::Transport(_)) => {
                super::NET_DOWN.store(true, std::sync::atomic::Ordering::Relaxed);
                Revalidation::Unreachable
            }
        }
    }

    /// Unix seconds to RFC 1123 — the inverse of [`parse_http_date`], for `If-Modified-Since`.
    fn format_http_date(secs: u64) -> String {
        let days_total = (secs / 86400) as i64;
        let (h, m, sec) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
        // civil_from_days (Howard Hinnant), then day-of-week from the epoch (Thu = day 0).
        let z = days_total + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = if month <= 2 { y + 1 } else { y };
        let dow = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][(days_total.rem_euclid(7)) as usize];
        let mon = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
            [(month - 1) as usize];
        format!("{dow}, {d:02} {mon} {year} {h:02}:{m:02}:{sec:02} GMT")
    }

    /// RFC 1123 (`Tue, 03 Jun 2025 11:05:30 GMT`) to unix seconds. Minimal by design — the
    /// one format HTTP requires; anything else reads as `None` (assume changed).
    fn parse_http_date(s: &str) -> Option<u64> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        // ["Tue,", "03", "Jun", "2025", "11:05:30", "GMT"]
        if parts.len() < 6 {
            return None;
        }
        let day: i64 = parts[1].parse().ok()?;
        let month = 1 + ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
            .iter()
            .position(|m| *m == parts[2])? as i64;
        let year: i64 = parts[3].parse().ok()?;
        let mut hms = parts[4].split(':');
        let (h, m, sec): (i64, i64, i64) = (
            hms.next()?.parse().ok()?,
            hms.next()?.parse().ok()?,
            hms.next()?.parse().ok()?,
        );
        // Days since the unix epoch (Howard Hinnant's days_from_civil).
        let y = if month <= 2 { year - 1 } else { year };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let mp = (month + 9) % 12;
        let doy = (153 * mp + 2) / 5 + day - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let days = era * 146097 + doe - 719468;
        u64::try_from(days * 86400 + h * 3600 + m * 60 + sec).ok()
    }

    /// `false` for links that can never be documentation pages — binary/asset endpoints and raw
    /// metadata files. Filtering keeps the crawl budget on real pages (MDN hangs a
    /// `contributors.txt` off every article; fetching those burns the budget on zero sections).
    fn is_page_url(url: &str) -> bool {
        let path = url.split(['?', '#']).next().unwrap_or(url).to_lowercase();
        const SKIP: &[&str] = &[
            ".txt", ".pdf", ".zip", ".gz", ".tar", ".png", ".jpg", ".jpeg", ".gif", ".svg",
            ".ico", ".css", ".js", ".mjs", ".json", ".xml", ".rss", ".woff", ".woff2", ".ttf",
            ".mp4", ".webm", ".epub",
        ];
        !SKIP.iter().any(|e| path.ends_with(e))
    }

    /// Every in-scope page URL a site's sitemap enumerates: try `<origin>/sitemap.xml` for each
    /// seed origin (one nesting level of sitemap indexes), parse `<loc>` entries by scan — no XML
    /// dependency. Missing or malformed sitemaps yield nothing and cost one request.
    fn sitemap_urls(seeds: &[&str]) -> Vec<String> {
        let mut out = Vec::new();
        let mut origins: Vec<String> = Vec::new();
        for s in seeds {
            let origin = s.split('/').take(3).collect::<Vec<_>>().join("/");
            if origin.starts_with("http") && !origins.contains(&origin) {
                origins.push(origin);
            }
        }
        let locs = |xml: &str| -> Vec<String> {
            xml.split("<loc>")
                .skip(1)
                .filter_map(|part| part.split("</loc>").next())
                .map(|u| u.trim().to_string())
                .collect()
        };
        for origin in origins {
            let Some((_, body)) = fetch(&format!("{origin}/sitemap.xml")) else { continue };
            let top = locs(&body);
            // A sitemap INDEX lists further sitemaps; fetch those (bounded) for their pages.
            let (maps, pages): (Vec<_>, Vec<_>) = top.into_iter().partition(|u| u.contains("sitemap") && u.ends_with(".xml"));
            out.extend(pages);
            for m in maps.into_iter().take(8) {
                if let Some((_, body)) = fetch(&m) {
                    out.extend(locs(&body));
                }
            }
        }
        // Only pages inside a seed's scope belong to this crawl.
        out.retain(|u| seeds.iter().any(|s| in_scope(s, u)));
        out
    }

    /// LEVEL-PARALLEL site mapping — the fetch stage of the per-language training pipeline:
    /// the sitemap (when one exists) enumerates the site in one request, then the crawl
    /// balloons outward one LEVEL at a time — every link of the level fetched concurrently,
    /// pages marked visited — until the whole in-scope site is mapped or `max_pages` is hit.
    /// No pacing: the goal is to map a documentation site in seconds, once per version, and
    /// never again. `_delay_ms` is kept for call-site compatibility and ignored.
    pub fn crawl(seeds: &[&str], max_pages: usize, _delay_ms: u64) -> Vec<Page> {
        /// Concurrent connections per wave inside a level — bounded so a thousand-page level
        /// does not spawn a thousand sockets at once.
        const WAVE: usize = 192;
        // Pre-flight: probe each seed's origin with a SHORT deadline before mapping. A
        // stalling host (alive TCP, no bytes — bot mitigation, dying server) otherwise costs
        // a full fetch timeout per phase (measured: one such site held an all-languages
        // 0.4s setup at 30s); a healthy origin answers this in milliseconds. The probe's
        // verdict feeds the same per-run breaker `fetch_meta` consults — nothing is cached
        // across runs.
        let seeds: Vec<&str> = seeds.iter().copied().filter(|s| origin_probe(s)).collect();
        if seeds.is_empty() {
            return Vec::new();
        }
        let seeds = seeds.as_slice();
        let mut seen: HashSet<String> = seeds.iter().map(|s| s.to_string()).collect();
        let mut level: Vec<String> = seeds.iter().map(|s| s.to_string()).collect();
        for url in sitemap_urls(seeds) {
            if is_page_url(&url) && seen.insert(url.clone()) {
                level.push(url);
            }
        }
        let mut pages: Vec<Page> = Vec::new();
        while !level.is_empty() && pages.len() < max_pages {
            level.truncate(max_pages - pages.len());
            let mut fetched: Vec<Option<(String, String, Option<u64>)>> = Vec::with_capacity(level.len());
            for wave in level.chunks(WAVE) {
                let got: Vec<Option<(String, String, Option<u64>)>> = std::thread::scope(|scope| {
                    let handles: Vec<_> =
                        wave.iter().map(|url| scope.spawn(move || fetch_meta(url))).collect();
                    handles.into_iter().map(|h| h.join().unwrap_or(None)).collect()
                });
                fetched.extend(got);
            }
            let mut next: Vec<String> = Vec::new();
            for (url, got) in level.drain(..).zip(fetched) {
                let Some((ct, body, modified)) = got else { continue };
                let sections = extract(&ct, &body);
                for (link, _anchor) in extract_anchors(&url, &body) {
                    if seen.len() < max_pages * 8
                        && is_page_url(&link)
                        && seeds.iter().any(|s| in_scope(s, &link))
                        && seen.insert(link.clone())
                    {
                        next.push(link);
                    }
                }
                pages.push(Page {
                    url,
                    prose: extract_prose(&body),
                    code: extract_code_blocks(&body),
                    sections,
                    html: body,
                    modified,
                });
                if pages.len() >= max_pages {
                    break;
                }
            }
            eprintln!("level complete: {} pages mapped, {} links queued", pages.len(), next.len());
            level = next;
        }
        pages
    }
}

#[cfg(feature = "crawl")]
pub use net::{crawl, fetch, fetch_bytes, fetch_conditional, origin_reachable, Revalidation};

/// Crawler disabled at compile time: reachability cannot be judged, so never claim a site dead.
#[cfg(not(feature = "crawl"))]
pub fn origin_reachable(_url: &str) -> bool {
    true
}

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
        // In scope: same host, inside the seed's docs tree. Out: other host, above the path,
        // or a sibling that merely shares the seed as a string prefix (`/c` vs `/cpp`).
        assert!(in_scope("https://doc.rust-lang.org/book/", "https://doc.rust-lang.org/book/ch02.html"));
        assert!(!in_scope("https://doc.rust-lang.org/book/", "https://crates.io/x"));
        assert!(in_scope("https://en.cppreference.com/c", "https://en.cppreference.com/c/language"));
        assert!(in_scope("https://en.cppreference.com/c", "https://en.cppreference.com/c"));
        assert!(!in_scope("https://en.cppreference.com/c", "https://en.cppreference.com/cpp/language"));
        assert!(in_scope("https://gnu.org/bash/manual/bash.html", "https://gnu.org/bash/manual/x.html"));
        assert!(!in_scope("https://doc.rust-lang.org/book/", "https://doc.rust-lang.org/std/index.html"));
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
        // A `<pre>` preceded by prose containing a multi-byte char positioned so the governing
        // look-back window starts inside that char — the real ruff-docs crash. Read through a
        // fixture char brain (hermetic — no machine brain) whose only role is `pre` a code
        // carrier; the reader must not panic and must extract the code.
        let mut brain = crate::lint_char::CharReader::new();
        brain.set_structure(crate::lint_char::StructureRoles::from_learned(
            vec![(crate::lint_ai::token_seed("pre"), 1)],
            0,
        ));
        let prose = format!("{}🛠 fast linter", "x".repeat(1490));
        let body = format!("<p>{prose}</p><pre>code here</pre>");
        let units = crate::lint_graph::read_page(&body, &brain);
        assert!(units.iter().any(|u| u.code.contains("code here")), "code extracted: {units:?}");
    }

    #[test]
    fn extracts_links_from_html() {
        let html = r#"<a href="a.html">A</a> <a href='/b.html'>B</a> <a href="mailto:x@y.z">M</a>"#;
        let links = extract_links("https://d.example/docs/index.html", html);
        assert!(links.iter().any(|l| l.ends_with("/docs/a.html")));
        assert!(links.iter().any(|l| l.ends_with("/b.html")));
        assert!(!links.iter().any(|l| l.contains("mailto")), "mailto dropped");
    }

    /// Ledger #18, as a table: every markup style real generators use declares its block's
    /// language, and extraction reports exactly that token — or "" when nothing is declared.
    #[test]
    fn block_language_hints_are_read_from_every_real_markup_style() {
        let cases: &[(&str, &str)] = &[
            (r#"<pre class="brush: html">&lt;div&gt;&lt;/div&gt;</pre>"#, "html"),      // MDN legacy
            (r#"<pre class="brush: js example-bad">var x;</pre>"#, "js"),               // MDN + marker
            (r#"<pre class="language-css">a { color: red }</pre>"#, "css"),             // Prism
            (r#"<pre><code class="language-js">require("x")</code></pre>"#, "js"),      // Node docs
            (r#"<pre class="lang-rust">let x = 1;</pre>"#, "rust"),                     // highlight.js
            (r#"<pre>plain block, no declaration</pre>"#, ""),                          // undeclared
        ];
        for (html, want) in cases {
            let open = html.find("<pre").expect("fixture has a pre");
            assert_eq!(
                block_lang_hint(html, open),
                *want,
                "hint of {html:?}"
            );
        }
        // Fenced text: the info string is the declaration.
        let md = "Some governing prose sits here.\n```css\na { color: red }\n```\n";
        let secs = extract_sections_text_hinted(md);
        assert_eq!(secs.len(), 1);
        assert_eq!(secs[0].2, "css", "fence info string is the hint");
    }
}
