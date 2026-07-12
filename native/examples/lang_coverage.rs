//! THROWAWAY MEASUREMENT HARNESS (untracked): scan EVERY cached language crawl on this machine for the
//! reader's recognized structural page-kind markers, and report the honest coverage map — per language:
//! total pages, reference-URL pages (`/reference/`), rule-URL pages (`/rules/`), and MDN-style
//! deprecation-notecard pages (the three signals `lint_lang_layer::read_doc_page` keys candidates on).
//! A language whose docs expose NONE of these proposes ZERO under the current reader — that zero is the
//! measurement, recorded honestly. Nothing here names a language in the READING; the registry supplies
//! tool->language purely as DATA so each crawl file is attributed to its language.
//!
//! Run: `cargo run --release --features crawl --example lang_coverage`
use helpers_native::lint_codec::{self, Dec};
use std::collections::BTreeMap;

/// Decode a `.bin` CRAWL container into `(url, body)` pages — the exact wire shape `write_crawl_cache`
/// persists (version, crawled_at, n, then per page: url, body, bool, fp, modified).
fn decode_bin(bytes: &[u8]) -> Option<Vec<(String, String)>> {
    let (_stamp, mut d) = Dec::open(bytes, lint_codec::kind::CRAWL)?;
    let _v = d.str()?;
    let _c = d.fixed_u64()?;
    let n = d.u()? as usize;
    let mut pages = Vec::with_capacity(n.min(65_536));
    for _ in 0..n {
        let url = d.str()?;
        let body = d.str()?;
        let _ = d.boolean()?;
        let _ = d.fixed_u64()?;
        let _ = d.fixed_u64()?;
        pages.push((url, body));
    }
    Some(pages)
}

/// Decode a legacy `.json` crawl into `(url, body)` — these hold `prose`/`units`, NOT raw HTML `body`,
/// so `body` comes back EMPTY (notecard scan cannot apply); the URL is still present for path-marker
/// scanning. Reported distinctly so a language whose only cache is legacy is not mistaken for one whose
/// live-readable `.bin` exposes no markers.
fn decode_json(bytes: &[u8]) -> Option<(Vec<(String, String)>, bool)> {
    let j: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let arr = j.get("pages")?.as_array()?;
    let mut pages = Vec::new();
    for p in arr {
        let url = p.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string();
        let body = p.get("body").and_then(|b| b.as_str()).unwrap_or("").to_string();
        pages.push((url, body));
    }
    Some((pages, true))
}

/// The three markers `lint_lang_layer` keys on — replicated here (the module's are private one-liners);
/// this is measurement only, no judgement.
fn is_reference(url: &str) -> bool {
    url.to_lowercase().contains("/reference/")
}
fn is_rule(url: &str) -> bool {
    url.to_lowercase().contains("/rules/")
}
fn has_notecard(body: &str) -> bool {
    body.contains("notecard deprecated") || body.contains("no longer recommended")
}

#[derive(Default)]
struct Cov {
    files: Vec<String>,
    pages: usize,
    reference: usize,
    rule: usize,
    notecard: usize,
    legacy_only: bool, // every crawl file for this language was body-less legacy json
    has_bin: bool,
}

fn main() {
    let home = std::env::var("HOME").unwrap();
    let dir = format!("{home}/.cache/helpers/lint-index/crawls");
    let sources_path = ["lint-index/sources.json", "../lint-index/sources.json"]
        .into_iter()
        .find(|p| std::path::Path::new(p).exists())
        .expect("sources.json in repo root or parent");
    let reg: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(sources_path).unwrap()).unwrap();
    // tool -> language (DATA from the registry; no language named in code).
    let mut tool_lang: BTreeMap<String, String> = BTreeMap::new();
    for s in reg["sources"].as_array().unwrap() {
        let (Some(t), Some(l)) = (s["tool"].as_str(), s["language"].as_str()) else { continue };
        tool_lang.insert(t.to_string(), l.to_string());
    }

    let mut cov: BTreeMap<String, Cov> = BTreeMap::new();
    for (tool, lang) in &tool_lang {
        let bin = format!("{dir}/{tool}.bin");
        let json = format!("{dir}/{tool}.json");
        let (pages, is_legacy, fname) = if let Ok(b) = std::fs::read(&bin) {
            match decode_bin(&b) {
                Some(p) => (p, false, format!("{tool}.bin")),
                None => continue,
            }
        } else if let Ok(b) = std::fs::read(&json) {
            match decode_json(&b) {
                Some((p, _)) => (p, true, format!("{tool}.json")),
                None => continue,
            }
        } else {
            continue; // MISSING crawl (e.g. w3schools-css) — not cached on this machine
        };
        let c = cov.entry(lang.clone()).or_default();
        c.files.push(fname);
        c.pages += pages.len();
        if !is_legacy {
            c.has_bin = true;
        }
        for (url, body) in &pages {
            if is_reference(url) {
                c.reference += 1;
            }
            if is_rule(url) {
                c.rule += 1;
            }
            if !is_legacy && is_reference(url) && has_notecard(body) {
                c.notecard += 1;
            }
        }
    }
    for c in cov.values_mut() {
        c.legacy_only = !c.has_bin;
    }

    // ── Report: the coverage map, sorted so the languages that EXPOSE markers surface first ──
    println!("LANG COVERAGE MAP — cached crawls scanned for the reader's page-kind markers\n");
    println!("{:14} {:>7} {:>6} {:>5} {:>8}  {:<7} files", "lang", "pages", "ref", "rule", "notecard", "cache");
    let mut rows: Vec<(&String, &Cov)> = cov.iter().collect();
    rows.sort_by_key(|(_, c)| std::cmp::Reverse(c.reference + c.rule + c.notecard));
    let (mut exposing, mut zero) = (0usize, 0usize);
    for (lang, c) in &rows {
        let signal = c.reference + c.rule + c.notecard;
        if signal > 0 {
            exposing += 1;
        } else {
            zero += 1;
        }
        let cache = if c.legacy_only { "legacy" } else { "bin" };
        println!(
            "{:14} {:>7} {:>6} {:>5} {:>8}  {:<7} {}",
            lang, c.pages, c.reference, c.rule, c.notecard, cache,
            c.files.join(",")
        );
    }
    println!(
        "\n{} languages with cached crawls; {} EXPOSE a recognized marker, {} propose ZERO under the current reader.",
        rows.len(),
        exposing,
        zero
    );
    println!("(legacy = body-less .json cache: URL markers scan, notecards cannot — the live path reads .bin only.)");
}
