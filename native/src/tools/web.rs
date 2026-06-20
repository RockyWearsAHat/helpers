//! `search_web` + `scrape_webpage` — drive a real Chrome over CDP
//! (`headless_chrome`, synchronous, Node-free). Mirrors the previous JS behavior:
//!
//! * Normal Google searches run in **headless** Chrome, automated like a person.
//! * On a CAPTCHA the user is asked to solve it in a **visible** Chrome window;
//!   that interactive Chrome stays open (cached for the process) and, once solved,
//!   its results are used and the cleared-CAPTCHA cookie persists in a shared
//!   on-disk profile so subsequent headless searches are automated again.
//! * Any later CAPTCHA simply re-surfaces the visible window to the user.
//!
//! A shared persistent profile (`~/.cache/helpers/google-browser-profile`) carries
//! the `GOOGLE_ABUSE_EXEMPTION` cookie between the interactive solve and headless
//! runs, so no manual cookie transfer is needed. Chrome instances are launched one
//! at a time (the MCP server is single-threaded) to avoid profile-lock contention.

use std::cell::RefCell;
use std::path::PathBuf;

use headless_chrome::{Browser, LaunchOptions};
use serde_json::{json, Value};

use crate::git::home;
use crate::proto::{text, Content, ToolResult};

const USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36";
const RESULTS_PER_PAGE: usize = 10;
// How long a single search_web call waits for the user to solve a CAPTCHA in the
// visible browser before returning "solve it, then retry" (the window stays open).
const CAPTCHA_POLL_ATTEMPTS: usize = 20;
const CAPTCHA_POLL_DELAY_MS: u64 = 2000;

thread_local! {
    /// A visible Chrome opened for a CAPTCHA the user hasn't finished solving.
    /// Kept open across calls so the user can solve at their own pace; the next
    /// search reuses it. Thread-local because the MCP loop is single-threaded and
    /// `Browser` need not cross threads.
    static PENDING_INTERACTIVE: RefCell<Option<Browser>> = const { RefCell::new(None) };
}

/// One page's outcome: parsed results, plus whether Google showed a CAPTCHA or a
/// genuine "no results" page.
struct Outcome {
    challenge: bool,
    no_results: bool,
    results: Vec<SearchResult>,
}

struct SearchResult {
    url: String,
    title: String,
    snippet: String,
}

// ── schemas ────────────────────────────────────────────────────────────────

/// MCP schema for `search_web` (mirrors the prior JS tool's contract).
pub fn schema_search() -> Value {
    json!({
        "name": "search_web",
        "description": "Search the web via Google in a real (automated) Chrome. Returns up to max_results deduplicated results (default 20, max 100). If Google shows a CAPTCHA, a visible Chrome opens for you to solve once; subsequent searches are automated using the cleared session. Set auto_scrape to inline full page content for the top N results.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query." },
                "site_filter": { "type": "string", "description": "Restrict to a site, e.g. example.com." },
                "exact_terms": { "type": "string", "description": "Terms that must appear in results." },
                "exclude_terms": { "type": "string", "description": "Terms to exclude from results." },
                "file_type": { "type": "string", "description": "Optional file type filter, for example pdf." },
                "language": { "type": "string", "description": "Optional language code, for example en. Defaults to en." },
                "time_range": { "type": "string", "description": "Optional time range filter: day, week, month, or year." },
                "max_results": { "type": "number", "description": "Max results (1-100, default 20)." },
                "auto_scrape": { "type": "number", "description": "Inline full page content for the top N results (0-10, default 0)." }
            },
            "required": ["query"]
        }
    })
}

/// MCP schema for `scrape_webpage`.
pub fn schema_scrape() -> Value {
    json!({
        "name": "scrape_webpage",
        "description": "Fetch and return the readable text of one or more web pages using a real headless Chrome (renders JS).",
        "inputSchema": {
            "type": "object",
            "properties": {
                "urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Absolute URLs to fetch."
                }
            },
            "required": ["urls"]
        }
    })
}

// ── chrome / profile helpers ─────────────────────────────────────────────────

/// Locate a Chrome/Chromium binary: `$HELPERS_CHROME_EXECUTABLE`, then common
/// install paths, else `None` (let headless_chrome auto-detect/fetch).
fn chrome_executable() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("HELPERS_CHROME_EXECUTABLE") {
        if !p.is_empty() && std::path::Path::new(&p).exists() {
            return Some(PathBuf::from(p));
        }
    }
    let candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
        "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
    ];
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
}

/// The shared, persistent browser profile dir (carries the cleared-CAPTCHA cookie
/// across runs). Created if missing.
fn profile_dir() -> PathBuf {
    let dir = home().join(".cache").join("helpers").join("google-browser-profile");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Launch Chrome (headless or visible) against the shared profile. Returns a
/// clear, actionable error when Chrome can't be found or started so the agent
/// can fix it (install Chrome / set HELPERS_CHROME_EXECUTABLE).
fn launch(headless: bool) -> Result<Browser, String> {
    let profile = profile_dir();
    let resolved = chrome_executable();
    let mut builder = LaunchOptions::default_builder();
    builder
        .headless(headless)
        .sandbox(false)
        .user_data_dir(Some(profile))
        .window_size(Some((1440, 900)));
    if let Some(path) = resolved.clone() {
        builder.path(Some(path));
    }
    let opts = builder
        .build()
        .map_err(|e| format!("invalid Chrome launch options: {e}"))?;
    Browser::new(opts).map_err(|e| {
        if resolved.is_none() {
            format!(
                "Google Chrome is required for web search/scrape but none was found. \
                 Install Google Chrome (https://www.google.com/chrome) or set the \
                 HELPERS_CHROME_EXECUTABLE environment variable to a Chrome/Chromium \
                 binary, then retry. (underlying error: {e})"
            )
        } else {
            format!(
                "Found Chrome at {} but could not launch it: {e}. If you are on a \
                 headless/CI machine the visible-browser CAPTCHA step needs a display.",
                resolved.as_ref().unwrap().display()
            )
        }
    })
}

// ── result extraction (runs in the page; returns a JSON string) ──────────────

// Mirrors the JS extractor: walk <a><h3> result blocks, and flag a CAPTCHA via
// /sorry/ URL, reCAPTCHA UI, known challenge text, or a Google page with zero
// parseable results. Returns a JSON STRING (evaluate returns primitives by value).
const EXTRACT_JS: &str = r#"
(function () {
  var bodyText = document.body ? (document.body.innerText || "") : "";
  var pageTitle = document.title || "";
  var googleOwned = /(^|\.)google\./i.test(location.hostname || "");
  var interruptionUi = !!document.querySelector(
    'form[action*="sorry"], iframe[src*="recaptcha"], #captcha, input[name="captcha"], textarea#g-recaptcha-response, div.g-recaptcha, form#captcha-form'
  );
  var challengeText = (pageTitle + "\n" + bodyText).replace(/\s+/g, " ").slice(0, 4000);
  var results = [];
  var seen = {};
  var anchors = document.querySelectorAll("a");
  for (var i = 0; i < anchors.length; i++) {
    var a = anchors[i];
    var h = a.querySelector("h3");
    if (!h) continue;
    var href = a.getAttribute("href") || a.href || "";
    if (!href || seen[href]) continue;
    seen[href] = 1;
    var title = (h.textContent || "").replace(/\s+/g, " ").trim();
    if (!title) continue;
    var url = href;
    if (url.indexOf("/url?q=") === 0 || url.indexOf("/url?") === 0) {
      try { url = decodeURIComponent((url.split("q=")[1] || "").split("&")[0]); } catch (e) {}
    }
    if (!/^https?:\/\//.test(url)) continue;
    if (/(^|\.)google\.com/.test(url)) continue;
    var c = a.closest("div.g") || a.closest("div.MjjYud") || a.closest("div[data-ved]") || a.closest("div");
    // Prefer Google's dedicated snippet element; fall back to the container text
    // with the title/URL chrome stripped so snippets aren't full of breadcrumbs.
    var snippet = "";
    if (c) {
      var sn = c.querySelector(".VwiC3b, .lEBKkf, .s3v9rd, span.aCOpRe, div[data-sncf], .IsZvec");
      if (sn) {
        snippet = (sn.innerText || "").replace(/\s+/g, " ").trim();
      } else {
        var ct = (c.innerText || "").replace(/\s+/g, " ").trim();
        var ti = title.replace(/\s+/g, " ").trim();
        var idx = ct.indexOf(ti);
        if (idx !== -1) ct = ct.slice(idx + ti.length);
        snippet = ct.replace(/^[\s›·|\-—]+/, "").replace(/\bRead more\b/gi, "").trim();
      }
    }
    results.push({ url: url, title: title, snippet: snippet.slice(0, 400) });
  }
  var captchaText = /detected unusual traffic|about this page|before you continue|verify you are human|not a robot|press and hold|enable javascript|unusual traffic from your computer/i.test(challengeText);
  var noResultsText = /did not match any documents|no results found for|try different keywords|try using more general keywords|check your spelling/i.test(challengeText);
  var noResults = googleOwned && results.length === 0 && noResultsText;
  var challenge = location.href.indexOf("/sorry/") !== -1 || interruptionUi || captchaText || (googleOwned && results.length === 0 && !noResults);
  return JSON.stringify({ challenge: challenge, noResults: noResults, results: results });
})()
"#;

/// Navigate `tab` to `url`, apply the navigation profile, and extract the outcome.
fn fetch_and_extract(browser: &Browser, url: &str) -> Result<Outcome, String> {
    let tab = browser.new_tab().map_err(|e| format!("new tab failed: {e}"))?;
    let _ = tab.set_user_agent(USER_AGENT, Some("en-US,en;q=0.9"), Some("macOS"));
    tab.navigate_to(url).map_err(|e| format!("navigate failed: {e}"))?;
    let _ = tab.wait_until_navigated();
    extract(&tab)
}

/// Run the extractor script in a tab and parse its JSON result.
fn extract(tab: &headless_chrome::Tab) -> Result<Outcome, String> {
    let ro = tab
        .evaluate(EXTRACT_JS, false)
        .map_err(|e| format!("extract failed: {e}"))?;
    let raw = match ro.value {
        Some(Value::String(s)) => s,
        _ => return Err("extractor returned no value".into()),
    };
    let v: Value = serde_json::from_str(&raw).map_err(|e| format!("bad extractor JSON: {e}"))?;
    let results = v
        .get("results")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|r| {
                    Some(SearchResult {
                        url: r.get("url")?.as_str()?.to_string(),
                        title: r.get("title")?.as_str().unwrap_or("").to_string(),
                        snippet: r.get("snippet").and_then(Value::as_str).unwrap_or("").to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Outcome {
        challenge: v.get("challenge").and_then(Value::as_bool).unwrap_or(false),
        no_results: v.get("noResults").and_then(Value::as_bool).unwrap_or(false),
        results,
    })
}

/// Bring a visible Chrome window to the foreground on macOS so the user sees the
/// CAPTCHA. Best-effort and silent on other platforms.
fn foreground_chrome() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("osascript")
            .args(["-e", "tell application \"Google Chrome\" to activate"])
            .output();
    }
}

// ── search ───────────────────────────────────────────────────────────────────

/// Build a Google search URL from the tool arguments.
fn build_query_url(args: &Value) -> Option<String> {
    let query = args.get("query")?.as_str()?.trim();
    if query.is_empty() {
        return None;
    }
    let mut terms = vec![query.to_string()];
    if let Some(s) = args.get("site_filter").and_then(Value::as_str) {
        terms.push(format!("site:{}", s.trim()));
    }
    if let Some(s) = args.get("exact_terms").and_then(Value::as_str) {
        terms.push(format!("\"{}\"", s.trim()));
    }
    if let Some(s) = args.get("exclude_terms").and_then(Value::as_str) {
        for t in s.split_whitespace() {
            terms.push(format!("-{t}"));
        }
    }
    if let Some(s) = args.get("file_type").and_then(Value::as_str) {
        terms.push(format!("filetype:{}", s.trim()));
    }
    let lang = args.get("language").and_then(Value::as_str).unwrap_or("en");
    let mut url = format!(
        "https://www.google.com/search?q={}&hl={}&filter=0",
        percent_encode(&terms.join(" ")),
        percent_encode(lang),
    );
    if let Some(tr) = args.get("time_range").and_then(Value::as_str) {
        let tbs = match tr {
            "day" => "qdr:d",
            "week" => "qdr:w",
            "month" => "qdr:m",
            "year" => "qdr:y",
            _ => "",
        };
        if !tbs.is_empty() {
            url.push_str(&format!("&tbs={tbs}"));
        }
    }
    Some(url)
}

/// Minimal percent-encoding for query strings (RFC 3986 unreserved kept as-is).
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `search_web` handler.
pub fn run_search(args: &Value) -> ToolResult {
    let url = build_query_url(args)
        .ok_or_else(|| "search_web requires a non-empty query.".to_string())?;
    let max_results = args
        .get("max_results")
        .and_then(Value::as_f64)
        .map(|n| (n.round() as usize).clamp(1, 100))
        .unwrap_or(2 * RESULTS_PER_PAGE);

    // 1) Headless attempt.
    let headless = launch(true)?;
    let outcome = fetch_and_extract(&headless, &url)?;
    if !outcome.challenge && !outcome.results.is_empty() {
        return Ok(format_results(&outcome.results, max_results));
    }
    if outcome.no_results {
        return Ok(vec![text(format!(
            "No results: Google reported no matching results for the query."
        ))]);
    }
    // Release the headless profile lock before opening a visible Chrome.
    drop(headless);

    // 2) CAPTCHA path — reuse a pending interactive window if the user solved it,
    //    else open one and either harvest the solved results or ask them to solve.
    resolve_via_visible_chrome(&url, max_results)
}

/// Handle a CAPTCHA by driving a visible Chrome the user can solve, reusing a
/// window kept open from a previous call when present.
fn resolve_via_visible_chrome(url: &str, max_results: usize) -> ToolResult {
    // Reuse a window left open by a prior call (user may have solved it since).
    let reused = PENDING_INTERACTIVE.with(|cell| cell.borrow_mut().take());
    if let Some(browser) = reused {
        if let Ok(out) = fetch_and_extract(&browser, url) {
            if !out.challenge && !out.results.is_empty() {
                // Solved — keep the verified window open for future searches.
                PENDING_INTERACTIVE.with(|c| *c.borrow_mut() = Some(browser));
                return Ok(format_results(&out.results, max_results));
            }
        }
        // Still challenged — keep it open and re-prompt below (reuse this browser).
        PENDING_INTERACTIVE.with(|c| *c.borrow_mut() = Some(browser));
        return resurface(url);
    }

    // Open a fresh visible Chrome on the query and poll briefly for a solve.
    let browser = launch(false)?;
    let tab = browser.new_tab().map_err(|e| format!("new tab failed: {e}"))?;
    let _ = tab.set_user_agent(USER_AGENT, Some("en-US,en;q=0.9"), Some("macOS"));
    tab.navigate_to(url).map_err(|e| format!("navigate failed: {e}"))?;
    let _ = tab.wait_until_navigated();
    let _ = tab.bring_to_front();
    foreground_chrome();

    for _ in 0..CAPTCHA_POLL_ATTEMPTS {
        if let Ok(out) = extract(&tab) {
            if !out.challenge && !out.results.is_empty() {
                PENDING_INTERACTIVE.with(|c| *c.borrow_mut() = Some(browser));
                return Ok(format_results(&out.results, max_results));
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(CAPTCHA_POLL_DELAY_MS));
    }

    // Not solved within the window — keep Chrome open for the user and ask them.
    PENDING_INTERACTIVE.with(|c| *c.borrow_mut() = Some(browser));
    resurface(url)
}

/// The message returned when the user still needs to solve the CAPTCHA.
fn resurface(_url: &str) -> ToolResult {
    Ok(vec![text(
        "Google showed a CAPTCHA. A Chrome window has been opened — please solve the \
         \"I'm not a robot\" check there, then run your search again. Once solved, that \
         verified session is reused so further searches run automatically."
            .to_string(),
    )])
}

/// Format results as MCP text content, deduped and capped at `max_results`.
fn format_results(results: &[SearchResult], max_results: usize) -> Vec<Content> {
    let mut seen = std::collections::HashSet::new();
    let mut lines = vec!["Results:".to_string(), String::new()];
    let mut rank = 0usize;
    for r in results {
        if rank >= max_results || !seen.insert(r.url.clone()) {
            continue;
        }
        rank += 1;
        lines.push(format!("{rank}. {}", r.title));
        lines.push(format!("   URL: {}", r.url));
        if !r.snippet.is_empty() {
            lines.push(format!("   {}", r.snippet));
        }
    }
    if rank == 0 {
        return vec![text("No results returned.".to_string())];
    }
    vec![text(lines.join("\n"))]
}

// ── scrape ───────────────────────────────────────────────────────────────────

/// `scrape_webpage` handler: render each URL in headless Chrome and return its
/// readable text.
pub fn run_scrape(args: &Value) -> ToolResult {
    let urls: Vec<String> = args
        .get("urls")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|u| u.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if urls.is_empty() {
        return Err("scrape_webpage requires at least one URL.".into());
    }
    let browser = launch(true)?;
    let mut blocks = Vec::new();
    for url in &urls {
        match scrape_one(&browser, url) {
            Ok((title, body)) => {
                blocks.push(text(format!("Title: {title}\nURL: {url}\n\n{body}")));
            }
            Err(e) => blocks.push(text(format!("URL: {url}\nScrape error: {e}"))),
        }
    }
    Ok(blocks)
}

/// Render one page and return (title, readable text).
fn scrape_one(browser: &Browser, url: &str) -> Result<(String, String), String> {
    let tab = browser.new_tab().map_err(|e| format!("new tab failed: {e}"))?;
    let _ = tab.set_user_agent(USER_AGENT, Some("en-US,en;q=0.9"), None);
    tab.navigate_to(url).map_err(|e| format!("navigate failed: {e}"))?;
    let _ = tab.wait_until_navigated();
    let title = tab.get_title().unwrap_or_default();
    let ro = tab
        .evaluate(
            "JSON.stringify(document.body ? (document.body.innerText || '') : '')",
            false,
        )
        .map_err(|e| format!("extract failed: {e}"))?;
    let body = match ro.value {
        Some(Value::String(s)) => serde_json::from_str::<String>(&s).unwrap_or(s),
        _ => String::new(),
    };
    Ok((title, body))
}
