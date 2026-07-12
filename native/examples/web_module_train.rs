//! THROWAWAY MEASUREMENT HARNESS (untracked): run the REAL construct-module training workflow
//! (`lint_module`) over the shared web-stack crawls (MDN HTML/CSS/JS + ESLint + W3Schools) for ALL
//! THREE languages, and MEASURE it honestly — the funnel (pages → candidates → proven), per-language +
//! total training time, and the kitchen-sink acceptance per language (a bad file flags the prohibited
//! constructs, a clean file is zero). Nothing here or in `lint_module` names a language or a construct
//! in the READING; the bad/good acceptance snippets below are TEST FIXTURES for measurement only.
//! Run: `cargo run --release --features crawl --example web_module_train`
use helpers_native::lint_char;
use helpers_native::lint_codec::{self, Dec};
use helpers_native::lint_english;
use helpers_native::lint_module::{self, Outcome};

use helpers_native::lint_trace::{run_plan, Plan};
use helpers_native::lint_train;
use std::time::Instant;

fn decode_crawl(bytes: &[u8]) -> Option<Vec<(String, String)>> {
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

/// Approximate the crawl's own per-page language attribution by URL shape — ONLY so this throwaway
/// harness can partition the cached pages when no per-language read `Memory` is on this machine. The
/// live path uses the real binding attribution; this is a measurement stand-in, never shipped logic.
fn url_lang(url: &str) -> Option<&'static str> {
    let u = url.to_lowercase();
    if u.contains("eslint.org") {
        return Some("javascript");
    }
    if u.contains("/docs/web/javascript") {
        return Some("javascript");
    }
    if u.contains("/docs/web/css") {
        return Some("css");
    }
    if u.contains("/docs/web/html") || u.contains("w3schools") {
        return Some("html");
    }
    None
}

/// Pull every `<code>…</code>` interior from a page body — a crude harvest-corpus reconstruction for
/// TIMING ONLY (the shipped harvest reads `memory.reference`). Not covenant-clean; a measurement prop.
fn code_interiors(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    // ASCII-lowercased byte copy: same length as the UTF-8 bytes (ASCII-only fold), so byte offsets
    // found here index `body` on char boundaries wherever the markers are ASCII.
    let lb: Vec<u8> = body.bytes().map(|b| b.to_ascii_lowercase()).collect();
    let find = |from: usize, needle: &[u8]| -> Option<usize> {
        if from > lb.len() {
            return None;
        }
        lb[from..].windows(needle.len()).position(|w| w == needle).map(|p| from + p)
    };
    let mut i = 0;
    while let Some(open) = find(i, b"<code") {
        let Some(gt) = find(open, b">").map(|g| g + 1) else { break };
        let Some(close) = find(gt, b"</code>") else { break };
        if !body.is_char_boundary(gt) || !body.is_char_boundary(close) {
            i = open + 5;
            continue;
        }
        let raw = &body[gt..close];
        // strip any nested tags (Prism spans) — keep text
        let mut txt = String::new();
        let mut intag = false;
        for ch in raw.chars() {
            match ch {
                '<' => intag = true,
                '>' => intag = false,
                _ if !intag => txt.push(ch),
                _ => {}
            }
        }
        let txt = txt
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"");
        if txt.trim().len() >= 2 {
            out.push(txt.trim().to_string());
        }
        i = close + 7;
    }
    out
}

/// Reconstruct a harvest `Memory` for `lang` from the cached crawl pages when this machine holds no
/// read `Memory` (the owner's per-language catalogs are absent here). `bindings` stays EMPTY — with no
/// bindings `lint_module::lang_pages` keeps exactly the pages passed in, so the caller controls the
/// partition — and `reference` is the harvested code corpus. Measurement scaffolding only.
fn reconstruct_memory(lang: &str, pages: &[(String, String)]) -> helpers_native::lint_read::Memory {
    let mut mem = helpers_native::lint_read::Memory::default();
    let mut seen = std::collections::HashSet::new();
    for (url, body) in pages {
        if url_lang(url) != Some(lang) {
            continue;
        }
        for block in code_interiors(body) {
            if seen.insert(block.clone()) {
                mem.reference.push(block);
            }
        }
    }
    mem
}

fn all_pages() -> Vec<(String, String)> {
    let home = std::env::var("HOME").unwrap();
    let dir = format!("{home}/.cache/helpers/lint-index/crawls");
    let mut pages = Vec::new();
    let Ok(rd) = std::fs::read_dir(&dir) else { return pages };
    for e in rd.flatten() {
        let nm = e.file_name().to_string_lossy().to_string();
        if nm.ends_with(".bin")
            && (nm.starts_with("developer-mozilla-org") || nm.starts_with("eslint-org") || nm.starts_with("w3schools"))
        {
            if let Ok(b) = std::fs::read(e.path()) {
                if let Some(p) = decode_crawl(&b) {
                    pages.extend(p);
                }
            }
        }
    }
    pages
}

/// Per-language acceptance fixtures: (bad file, good file, the prohibited constructs to look for).
fn fixtures(lang: &str) -> (&'static str, &'static str, Vec<&'static str>) {
    match lang {
        "javascript" => (
            "var x = 1;\nif (x == '1') { eval('danger'); }\nwith (Math) { var y = cos(0); }\ndocument.write('<b>'+x);\nfor (var i=0;i<3;i++){}\n",
            "const x = 1;\nif (x === 1) { JSON.parse('{}'); }\nlet y = Math.cos(0);\nel.append(node);\nfor (let i=0;i<3;i++){}\n",
            vec!["var", "==", "eval", "with", "document.write"],
        ),
        "html" => (
            "<center><font size=3>hi</font></center>\n<marquee>scroll</marquee>\n<frameset><frame></frameset>\n<tt>code</tt>\n",
            "<main><p>hi</p><section><h1>title</h1></section></main>\n",
            vec!["center", "font", "marquee", "frameset", "tt"],
        ),
        "css" => (
            "a { box-orient: horizontal; page-break-after: always; text-decoration-skip: ink; }\n",
            "a { display: flex; break-after: page; }\n",
            vec!["box-orient", "page-break-after", "text-decoration-skip"],
        ),
        _ => ("", "", vec![]),
    }
}

fn main() {
    let (Some(br), Some(en)) = (lint_char::brain(), lint_english::brain()) else {
        eprintln!("no frozen brains on disk");
        return;
    };
    let m = br.meanings();
    let pages = all_pages();
    println!("shared web-stack crawl pages: {}\n", pages.len());

    let mut total = std::time::Duration::ZERO;
    for lang in ["javascript", "css", "html"] {
        // Prefer the real per-language read Memory; fall back to a harvest reconstruction from the
        // cached crawls (measurement only) when this machine holds no catalog.
        let (memory, lang_pages): (helpers_native::lint_read::Memory, Vec<(String, String)>) =
            match lint_train::cached_memory(lang) {
                Some(mem) => (mem, pages.clone()),
                None => {
                    let mem = reconstruct_memory(lang, &pages);
                    // WHOLE-SITE corpus (owner directive 2026-07-12): every page is proposed to every
                    // language and the GRAMMAR-verification partition inside `graduate` decides — a
                    // `/Web/API/Document/write` page (URL-attributed to nothing) joins the JS partition
                    // because its subject fires under the JS grammar. The old `url_lang` subset dropped
                    // exactly those cross-section pages, under-measuring the live `site_corpus` path.
                    println!(
                        "== {lang}: reconstructed harvest memory ({} ref blocks, {} whole-site pages)",
                        mem.reference.len(),
                        pages.len()
                    );
                    (mem, pages.clone())
                }
            };
        let t = Instant::now();
        let outcomes = lint_module::graduate(lang, &lang_pages, &memory, m, en);
        let elapsed = t.elapsed();
        total += elapsed;
        let proven: Vec<&Outcome> = outcomes.iter().filter(|o| o.rule.is_some()).collect();
        println!(
            "======== {lang}: {} candidates, {} PROVEN, {:.2}s (memory {} bindings) ========",
            outcomes.len(),
            proven.len(),
            elapsed.as_secs_f64(),
            memory.bindings.len()
        );
        for o in &proven {
            if let Some((r, url)) = &o.rule {
                println!("  PROVEN {:20} fire={:4} id={:24} src={}", o.candidate.construct, o.violating, r.id, url);
            }
        }
        // Any non-proven candidate that still fired a lot (a near-miss) — visibility.
        let mut near: Vec<&Outcome> = outcomes.iter().filter(|o| o.rule.is_none() && o.violating >= 5).collect();
        near.sort_by_key(|o| std::cmp::Reverse(o.violating));
        for o in near.iter().take(12) {
            println!("  unproven {:18} fire={:4} {:?}", o.candidate.construct, o.violating, o.verdict);
        }

        // ── Acceptance: bad file flags the prohibited constructs, good file is clean ──
        let (bad, good, wanted) = fixtures(lang);
        println!("  -- acceptance ({lang}) --");
        let mut bad_flags = 0;
        for o in &proven {
            let plan = Plan::UsesConstruct { construct: o.candidate.construct.clone() };
            let b = run_plan(&plan, lang, bad);
            if !b.is_empty() {
                bad_flags += 1;
                println!("     BAD flag {:16} lines {:?}", o.candidate.construct, b);
            }
        }
        let good_flags: Vec<String> = proven
            .iter()
            .filter(|o| !run_plan(&Plan::UsesConstruct { construct: o.candidate.construct.clone() }, lang, good).is_empty())
            .map(|o| o.candidate.construct.clone())
            .collect();
        println!("     bad file: {bad_flags} proven rules flag it; good file wrongly flagged by {:?}", good_flags);
        for w in &wanted {
            let hit = proven.iter().any(|o| &o.candidate.construct == w);
            println!("     want {w:16} -> {}", if hit { "PROVEN" } else { "missing" });
        }
        println!();
    }
    println!("TOTAL training time (all three languages): {:.2}s", total.as_secs_f64());
}
