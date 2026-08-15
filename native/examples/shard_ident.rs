//! PASS-33 incident probe: which manifest URL each hashed crawl shard belongs to.
//!
//! Prints `manifest_tool`-style ids (`host-<hash8>`) for the candidate URLs, so the hashed
//! `~/.cache/helpers/lint-index/crawls/*.bin` files can be matched back to their source URL
//! and renamed to their registry identity. Read-only; no network, no writes.

fn main() {
    for url in [
        "https://developer.mozilla.org/en-US/docs/Web/HTML/",
        "https://developer.mozilla.org/en-US/docs/Web/JavaScript/",
        "https://developer.mozilla.org/en-US/docs/Web/CSS/",
        "https://developer.mozilla.org/en-US/docs/Web/SVG/",
        "https://www.w3schools.com/html/",
        "https://www.w3schools.com/js/",
        "https://www.w3schools.com/css/",
        "https://docs.python.org/3/library/",
    ] {
        let host = url
            .strip_prefix("https://")
            .and_then(|rest| rest.split('/').next())
            .unwrap_or("docs")
            .trim_start_matches("www.");
        println!("{host}-{:08x}  {url}", helpers_native::lint_ai::token_seed(url) as u32);
    }
}
