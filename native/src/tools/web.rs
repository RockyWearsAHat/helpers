//! `search_web` + `scrape_webpage` — drive a real Chrome over CDP
//! (`headless_chrome`, synchronous, Node-free). Mirrors the previous JS behavior:
//!
//! * Normal Google searches run in **headless** Chrome, automated like a person.
//! * On a CAPTCHA the user is asked to solve it in a **visible** Chrome window.
//!   While unsolved, that window is kept open across calls so the user can finish
//!   at their own pace. Once solved, its results are returned and the window is
//!   closed gracefully (a CDP `Browser.close` on drop), which flushes the
//!   cleared-CAPTCHA cookie to a shared on-disk profile so subsequent headless
//!   searches are automated again.
//! * Any later CAPTCHA simply re-surfaces the visible window to the user.
//!
//! A shared persistent profile (`~/.cache/helpers/google-browser-profile`) carries
//! the `GOOGLE_ABUSE_EXEMPTION` cookie between the interactive solve and headless
//! runs, so no manual cookie transfer is needed. Chrome instances are launched one
//! at a time (the MCP server is single-threaded) to avoid profile-lock contention.

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

use headless_chrome::{Browser, LaunchOptions, Tab};
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
    /// Long-lived headless Chrome on the shared profile — plus one reusable tab —
    /// shared by every `search_web` / `scrape_webpage` call. Each call merely
    /// *navigates* this tab; we never open or close tabs per request. That matters
    /// because current Chrome drops the reply to both `Browser.close` (on `Browser`
    /// drop) and `Target.closeTarget` (on `Tab::close`), so either call blocks for
    /// the full idle timeout (~30s). Keeping one browser + one tab alive sidesteps
    /// both; the slow close is paid only when the cache is reset or the process exits.
    static HEADLESS: RefCell<Option<(Browser, Arc<Tab>)>> = const { RefCell::new(None) };

    /// A visible Chrome (and the tab on it) opened for a CAPTCHA the user hasn't
    /// finished solving. Kept open across calls so the user can solve at their own
    /// pace; the next search re-runs the query on that same solved tab. Thread-local
    /// because the MCP loop is single-threaded and `Browser` need not cross threads.
    static PENDING_INTERACTIVE: RefCell<Option<(Browser, Arc<Tab>)>> = const { RefCell::new(None) };
}

/// Borrow the cached headless tab, launching the browser and opening the tab on
/// first use. The returned `Arc<Tab>` shares the one long-lived tab; dropping the
/// clone is just a refcount decrement, never a (slow) tab close.
fn ensure_tab() -> Result<Arc<Tab>, String> {
    HEADLESS.with(|cell| {
        if cell.borrow().is_none() {
            let browser = launch_with_retry(true)?;
            let tab = browser
                .new_tab()
                .map_err(|e| format!("new tab failed: {e}"))?;
            *cell.borrow_mut() = Some((browser, tab));
        }
        Ok(cell.borrow().as_ref().expect("just populated").1.clone())
    })
}

/// Drop the cached browser/tab — because it died, or to free the shared profile's
/// lock so a visible CAPTCHA window can take it. The graceful close runs on a
/// detached thread so callers never block on Chrome's slow shutdown reply.
fn reset_headless() {
    if let Some((browser, _tab)) = HEADLESS.with(|c| c.borrow_mut().take()) {
        background_drop(browser);
    }
}

/// Drop a `Browser` off the calling thread. Current Chrome makes the graceful
/// `Browser.close` on drop block for ~the idle timeout; the Chrome process itself
/// exits (and frees the profile lock) quickly, so only this throwaway thread waits.
fn background_drop(browser: Browser) {
    let _ = std::thread::Builder::new()
        .name("helpers-chrome-close".into())
        .spawn(move || drop(browser));
}

/// Launch Chrome, retrying briefly on failure. After a sibling Chrome on the shared
/// profile is dropped, its `SingletonLock` lingers until the process exits; a few
/// short retries ride that out so the next launch succeeds instead of erroring.
fn launch_with_retry(headless: bool) -> Result<Browser, String> {
    const ATTEMPTS: usize = 8;
    let mut last = String::new();
    for attempt in 0..ATTEMPTS {
        match launch(headless) {
            Ok(browser) => return Ok(browser),
            Err(e) => {
                last = e;
                if attempt + 1 < ATTEMPTS {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        }
    }
    Err(last)
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
    /// Direct image URL for image-search hits; `None` for ordinary web results.
    image_url: Option<String>,
}

/// Outcome of collecting one result set (web or images): either parsed results,
/// a genuine "no results" page, or a still-unsolved CAPTCHA the user must clear.
enum Collected {
    Results(Vec<SearchResult>),
    NoResults,
    CaptchaPending,
}

// ── schemas ────────────────────────────────────────────────────────────────

/// MCP schema for `search_web` (mirrors the prior JS tool's contract).
pub fn schema_search() -> Value {
    json!({
        "name": "search_web",
        "description": "Search the web via Google in a real (automated) Chrome. Returns up to max_results deduplicated results (default 20, max 100). Use search_type to choose web pages, images, or both. If Google shows a CAPTCHA, a visible Chrome opens for you to solve once; the window closes itself as soon as the check passes and subsequent searches are automated using the cleared session. Set auto_scrape to inline full page content for the top N results.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query." },
                "search_type": { "type": "string", "enum": ["web", "images", "both"], "description": "What to search for: web pages, images, or both (default). Image results include the direct image URL and its source page." },
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

/// Remove a `SingletonLock` orphaned by a **dead** Chrome on the shared profile.
///
/// Chrome writes `SingletonLock` as a symlink named `<host>-<pid>`. If a prior
/// helpers process (or its Chrome) was killed, that symlink lingers; the next
/// launch then attaches to a phantom "primary" and its DevTools target never
/// initializes — surfacing as `new tab failed: The event waited for never came`.
/// We delete the singleton files **only when the recorded PID is provably gone**
/// (`kill -0` fails), so a genuinely live Chrome is never disturbed.
fn clear_stale_singleton_lock(profile: &std::path::Path) {
    let lock = profile.join("SingletonLock");
    let Ok(target) = std::fs::read_link(&lock) else { return };
    let pid = target
        .to_string_lossy()
        .rsplit('-')
        .next()
        .and_then(|p| p.parse::<i32>().ok());
    let alive = match pid {
        Some(pid) => std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false),
        None => false, // unparseable target -> treat as stale
    };
    if !alive {
        for name in ["SingletonLock", "SingletonSocket", "SingletonCookie"] {
            let _ = std::fs::remove_file(profile.join(name));
        }
    }
}

/// Launch Chrome (headless or visible) against the shared profile. Returns a
/// clear, actionable error when Chrome can't be found or started so the agent
/// can fix it (install Chrome / set HELPERS_CHROME_EXECUTABLE).
fn launch(headless: bool) -> Result<Browser, String> {
    let profile = profile_dir();
    clear_stale_singleton_lock(&profile);
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
    Browser::new(opts).map_err(|e| match &resolved {
        None => format!(
            "Google Chrome is required for web search/scrape but none was found. \
             Install Google Chrome (https://www.google.com/chrome) or set the \
             HELPERS_CHROME_EXECUTABLE environment variable to a Chrome/Chromium \
             binary, then retry. (underlying error: {e})"
        ),
        Some(path) => format!(
            "Found Chrome at {} but could not launch it: {e}. If you are on a \
             headless/CI machine the visible-browser CAPTCHA step needs a display.",
            path.display()
        ),
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
    results.push({ url: url, title: title, snippet: snippet.slice(0, 400), image_url: null });
  }
  var captchaText = /detected unusual traffic|about this page|before you continue|verify you are human|not a robot|press and hold|enable javascript|unusual traffic from your computer/i.test(challengeText);
  var noResultsText = /did not match any documents|no results found for|try different keywords|try using more general keywords|check your spelling/i.test(challengeText);
  var noResults = googleOwned && results.length === 0 && noResultsText;
  var challenge = location.href.indexOf("/sorry/") !== -1 || interruptionUi || captchaText || (googleOwned && results.length === 0 && !noResults);
  return JSON.stringify({ challenge: challenge, noResults: noResults, results: results });
})()
"#;

// Image-search extractor for Google's current Images vertical (`udm=2`). Results
// are `<img>` thumbnails served from `encrypted-tbn0.gstatic.com` carrying the
// description in their `alt` (empty-alt thumbnails are UI chrome / related-search
// chips, so we skip them). The gstatic thumbnail is itself a real, loadable image
// URL; we also walk up the DOM for a best-effort non-Google source page. Unlike
// the web extractor, an *empty* image grid is NOT treated as a CAPTCHA (Google no
// longer renders parseable anchors here), only explicit challenge signals are.
// Returns a JSON STRING.
const EXTRACT_IMAGES_JS: &str = r#"
(function () {
  var bodyText = document.body ? (document.body.innerText || "") : "";
  var pageTitle = document.title || "";
  var interruptionUi = !!document.querySelector(
    'form[action*="sorry"], iframe[src*="recaptcha"], #captcha, input[name="captcha"], textarea#g-recaptcha-response, div.g-recaptcha, form#captcha-form'
  );
  var challengeText = (pageTitle + "\n" + bodyText).replace(/\s+/g, " ").slice(0, 4000);
  var results = [];
  var seen = {};
  var imgs = document.querySelectorAll("img");
  for (var i = 0; i < imgs.length; i++) {
    var im = imgs[i];
    var src = im.src || "";
    if (!/^https?:\/\//.test(src)) continue;
    if (!/(encrypted-tbn|gstatic\.com\/images|googleusercontent)/.test(src)) continue;
    var alt = (im.getAttribute("alt") || "").replace(/\s+/g, " ").trim();
    if (alt.length < 3) continue;            // skip logos / related-search chips
    if (seen[src]) continue;
    seen[src] = 1;
    // Best-effort source page: nearest non-Google external link up the tree.
    var page = "", node = im;
    for (var d = 0; d < 8 && node; d++) {
      node = node.parentElement;
      if (!node) break;
      var a = node.querySelector('a[href^="http"], a[href^="/url?"]');
      if (a) {
        var h = a.getAttribute("href") || "";
        if (h.indexOf("/url?") === 0) {
          try { h = decodeURIComponent((h.split("q=")[1] || "").split("&")[0]); } catch (e) {}
        }
        if (/^https?:\/\//.test(h) && !/(^|\.)google\.com/.test(h)) { page = h; break; }
      }
    }
    results.push({ url: page || src, title: alt, snippet: "", image_url: src });
  }
  var captchaText = /detected unusual traffic|about this page|before you continue|verify you are human|not a robot|press and hold|enable javascript|unusual traffic from your computer/i.test(challengeText);
  var noResultsText = /did not match any documents|no results found for|try different keywords|try using more general keywords|check your spelling/i.test(challengeText);
  var noResults = results.length === 0 && noResultsText;
  var challenge = location.href.indexOf("/sorry/") !== -1 || interruptionUi || captchaText;
  return JSON.stringify({ challenge: challenge, noResults: noResults, results: results });
})()
"#;

/// Navigate `tab` to `url`, apply the navigation profile, and extract the outcome.
/// `images` selects the image-search extractor over the web-results one.
fn fetch_and_extract(tab: &Tab, url: &str, images: bool) -> Result<Outcome, String> {
    let _ = tab.set_user_agent(USER_AGENT, Some("en-US,en;q=0.9"), Some("macOS"));
    tab.navigate_to(url).map_err(|e| format!("navigate failed: {e}"))?;
    let _ = tab.wait_until_navigated();
    extract(tab, images)
}

/// Run the appropriate extractor script in a tab and parse its JSON result.
fn extract(tab: &headless_chrome::Tab, images: bool) -> Result<Outcome, String> {
    let script = if images { EXTRACT_IMAGES_JS } else { EXTRACT_JS };
    let ro = tab
        .evaluate(script, false)
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
                        image_url: r
                            .get("image_url")
                            .and_then(Value::as_str)
                            .filter(|s| !s.is_empty())
                            .map(str::to_string),
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

/// Build a Google search URL from the tool arguments. `images` switches to Google
/// Images (`tbm=isch`); all other filters apply to both modes.
fn build_query_url(args: &Value, images: bool) -> Option<String> {
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
    if images {
        // `udm=2` is Google's current Images vertical (the old `tbm=isch` now just
        // redirects here). Setting it directly avoids the redirect round-trip.
        url.push_str("&udm=2");
    }
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

/// The message returned when the user still needs to solve the CAPTCHA.
const CAPTCHA_MSG: &str =
    "Google showed a CAPTCHA. A Chrome window has been opened — please solve the \
     \"I'm not a robot\" check there, then run your search again. The window closes \
     itself as soon as the check passes, and that verified session is reused so \
     further searches run automatically.";

/// `search_web` handler. Honors `search_type` (`web`, `images`, or `both`) and
/// returns one text block per requested mode.
pub fn run_search(args: &Value) -> ToolResult {
    let max_results = args
        .get("max_results")
        .and_then(Value::as_f64)
        .map(|n| (n.round() as usize).clamp(1, 100))
        .unwrap_or(2 * RESULTS_PER_PAGE);
    let (want_web, want_images) = match args
        .get("search_type")
        .and_then(Value::as_str)
        .unwrap_or("both")
        .to_ascii_lowercase()
        .as_str()
    {
        "image" | "images" => (false, true),
        "web" | "pages" => (true, false),
        _ => (true, true),
    };

    let mut blocks: Vec<Content> = Vec::new();
    for (images, label) in [(false, "Web results"), (true, "Image results")] {
        if (images && !want_images) || (!images && !want_web) {
            continue;
        }
        let url = build_query_url(args, images)
            .ok_or_else(|| "search_web requires a non-empty query.".to_string())?;
        match collect(&url, images)? {
            // A still-unsolved CAPTCHA blocks every mode — surface it once and stop.
            Collected::CaptchaPending => return Ok(vec![text(CAPTCHA_MSG.to_string())]),
            Collected::NoResults => {
                blocks.push(text(format!("{label}: Google reported no matching results.")))
            }
            Collected::Results(results) => {
                blocks.push(format_results(&results, max_results, label))
            }
        }
    }
    Ok(blocks)
}

/// Collect one result set for `url`: try headless first, then fall back to a
/// visible Chrome the user can solve a CAPTCHA in.
fn collect(url: &str, images: bool) -> Result<Collected, String> {
    // 1) Headless attempt against the long-lived cached browser.
    let outcome = fetch_with_cached(url, images)?;
    if !outcome.challenge && !outcome.results.is_empty() {
        return Ok(Collected::Results(outcome.results));
    }
    if outcome.no_results || !outcome.challenge {
        // Either an explicit "no results" page, or a genuinely empty result set
        // that is not a CAPTCHA — report empty rather than opening a visible window.
        return Ok(Collected::NoResults);
    }
    // 2) CAPTCHA path. The visible window needs the shared profile, so drop the
    //    cached headless first to free its lock, then drive the solve.
    reset_headless();
    resolve_via_visible_chrome(url, images)
}

/// Fetch + extract on the cached headless tab, relaunching once if the cached
/// browser/tab has died (a navigation that can't even start), then retrying.
fn fetch_with_cached(url: &str, images: bool) -> Result<Outcome, String> {
    let tab = ensure_tab()?;
    match fetch_and_extract(&tab, url, images) {
        Err(e) if e.starts_with("navigate failed") => {
            reset_headless();
            let tab = ensure_tab()?;
            fetch_and_extract(&tab, url, images)
        }
        other => other,
    }
}

/// Handle a CAPTCHA by driving a visible Chrome the user can solve, reusing a
/// window kept open from a previous call when present. On a successful solve the
/// window is dropped, which triggers a graceful CDP `Browser.close`: the tab is
/// closed and the cleared-CAPTCHA cookie is flushed to the shared profile.
fn resolve_via_visible_chrome(url: &str, images: bool) -> Result<Collected, String> {
    // Reuse the window (and its tab) left open by a prior call — the user may have
    // solved the CAPTCHA since. Re-running the query on that same tab now passes.
    let reused = PENDING_INTERACTIVE.with(|cell| cell.borrow_mut().take());
    if let Some((browser, tab)) = reused {
        if let Ok(out) = fetch_and_extract(&tab, url, images) {
            if !out.challenge && !out.results.is_empty() {
                // Solved — close the window off-thread (Chrome flushes the cleared
                // cookie to the profile as it exits), then return.
                background_drop(browser);
                return Ok(Collected::Results(out.results));
            }
        }
        // Still challenged — keep it open for the user and re-prompt.
        PENDING_INTERACTIVE.with(|c| *c.borrow_mut() = Some((browser, tab)));
        return Ok(Collected::CaptchaPending);
    }

    // Open a fresh visible Chrome on the query and poll briefly for a solve.
    let browser = launch_with_retry(false)?;
    let tab = browser.new_tab().map_err(|e| format!("new tab failed: {e}"))?;
    let _ = tab.set_user_agent(USER_AGENT, Some("en-US,en;q=0.9"), Some("macOS"));
    tab.navigate_to(url).map_err(|e| format!("navigate failed: {e}"))?;
    let _ = tab.wait_until_navigated();
    let _ = tab.bring_to_front();
    foreground_chrome();

    for _ in 0..CAPTCHA_POLL_ATTEMPTS {
        if let Ok(out) = extract(&tab, images) {
            if !out.challenge && !out.results.is_empty() {
                // Solved within the poll window — close the window off-thread so it
                // doesn't linger; Chrome flushes the cleared session as it exits.
                background_drop(browser);
                return Ok(Collected::Results(out.results));
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(CAPTCHA_POLL_DELAY_MS));
    }

    // Not solved within the window — keep Chrome (and its tab) open for the user.
    PENDING_INTERACTIVE.with(|c| *c.borrow_mut() = Some((browser, tab)));
    Ok(Collected::CaptchaPending)
}

/// Format one result set as a single text block: a `label:` header, then deduped
/// numbered entries capped at `max_results`. Image hits add an `Image:` line.
fn format_results(results: &[SearchResult], max_results: usize, label: &str) -> Content {
    let mut seen = std::collections::HashSet::new();
    let mut lines = vec![format!("{label}:"), String::new()];
    let mut rank = 0usize;
    for r in results {
        if rank >= max_results || !seen.insert(r.url.clone()) {
            continue;
        }
        rank += 1;
        lines.push(format!("{rank}. {}", r.title));
        lines.push(format!("   URL: {}", r.url));
        if let Some(img) = &r.image_url {
            lines.push(format!("   Image: {img}"));
        }
        if !r.snippet.is_empty() {
            lines.push(format!("   {}", r.snippet));
        }
    }
    if rank == 0 {
        return text(format!("{label}: no results returned."));
    }
    text(lines.join("\n"))
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
    let mut blocks = Vec::new();
    for url in &urls {
        match scrape_with_cached(url) {
            Ok((title, body)) => {
                blocks.push(text(format!("Title: {title}\nURL: {url}\n\n{body}")));
            }
            Err(e) => blocks.push(text(format!("URL: {url}\nScrape error: {e}"))),
        }
    }
    Ok(blocks)
}

/// Scrape on the cached headless tab, relaunching once if it has died.
fn scrape_with_cached(url: &str) -> Result<(String, String), String> {
    let tab = ensure_tab()?;
    match scrape_one(&tab, url) {
        Err(e) if e.starts_with("navigate failed") => {
            reset_headless();
            let tab = ensure_tab()?;
            scrape_one(&tab, url)
        }
        other => other,
    }
}

/// Render one page on the reusable tab and return (title, readable text).
fn scrape_one(tab: &Tab, url: &str) -> Result<(String, String), String> {
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
