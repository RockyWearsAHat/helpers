//! THROWAWAY DIAGNOSTIC (untracked, COMPLETION PASS 15). Per-element funnel for the HTML module:
//! for a target element name, is its MDN page cached? attested deprecated? does it partition into
//! HTML (page_proves_in_lang, surfaced via `proposed`)? does it graduate (graduate Outcome)?
//! Run: cargo run --release --features crawl --example htmlfunnel
use helpers_native::lint_attest::Attestation;
use helpers_native::lint_char;
use helpers_native::lint_codec::{self, Dec};
use helpers_native::lint_english;
use helpers_native::lint_lang_layer::read_doc_page;
use helpers_native::lint_module::{self, Outcome};
use helpers_native::lint_trace::Bridge;
use helpers_native::lint_train;
use std::collections::HashSet;

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

fn all_pages() -> Vec<(String, String)> {
    let home = std::env::var("HOME").unwrap();
    let dir = format!("{home}/.cache/helpers/lint-index/crawls");
    let mut pages = Vec::new();
    let Ok(rd) = std::fs::read_dir(&dir) else { return pages };
    for e in rd.flatten() {
        let nm = e.file_name().to_string_lossy().to_string();
        if nm == "mdn-html.bin" || nm == "w3schools-html.bin" {
            if let Ok(b) = std::fs::read(e.path()) {
                if let Some(p) = decode_crawl(&b) {
                    pages.extend(p);
                }
            }
        }
    }
    pages
}

const TARGETS: &[&str] = &[
    // graduated today
    "center", "acronym", "big", "frame", "frameset", "marquee", "rb", "tt",
    // fall-out
    "font", "xmp",
    // the 17 missing
    "blink", "strike", "applet", "basefont", "dir", "noframes", "isindex", "keygen",
    "listing", "menuitem", "nobr", "noembed", "plaintext", "spacer", "rtc",
];

fn main() {
    let (Some(br), Some(en)) = (lint_char::brain(), lint_english::brain()) else {
        eprintln!("no frozen brains on disk");
        return;
    };
    let m = br.meanings();
    let bridge = Bridge::new(m, en);
    let pages = all_pages();
    println!("crawl pages: {}", pages.len());

    // Attestation + attested URL set (as graduate does).
    let attest = Attestation::discover(&pages);
    let attested: HashSet<String> =
        pages.iter().filter(|(_, b)| attest.attests(b)).map(|(u, _)| u.clone()).collect();
    println!("markers={} class_markers={:?} attested pages={}", attest.markers().len(), attest.class_markers(), attested.len());

    // Index element pages by lowercased last path segment under /Elements/.
    let mut by_elem: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    for (i, (u, _)) in pages.iter().enumerate() {
        let lu = u.to_lowercase();
        if let Some(pos) = lu.find("/elements/") {
            let seg = lu[pos + "/elements/".len()..].trim_end_matches('/');
            if !seg.contains('/') && !seg.is_empty() {
                by_elem.entry(seg.to_string()).or_default().push(i);
            }
        }
    }

    // Run full graduate once for the html Outcomes.
    let memory = lint_train::cached_memory("html").unwrap_or_else(|| {
        eprintln!("no cached html memory; using empty");
        helpers_native::lint_read::Memory::default()
    });
    let constructions = helpers_native::lint_construct::load("html");
    let (outcomes, _read, _corr): (Vec<Outcome>, _, _) =
        lint_module::graduate("html", &pages, &memory, m, en, &constructions);
    let by_construct: std::collections::HashMap<String, &Outcome> =
        outcomes.iter().map(|o| (o.candidate.construct.clone(), o)).collect();

    println!("\n{:<12} {:<7} {:<6} {:<7} {:<6} {:<6} {:<8} {:<10}", "elem", "cached", "attst", "prohib", "cons#", "part", "cand", "verdict/fire");
    for t in TARGETS {
        let idxs = by_elem.get(*t).cloned().unwrap_or_default();
        let cached = !idxs.is_empty();
        if !cached {
            println!("{:<12} {:<7} (no MDN /Elements/ page cached)", t, "NO");
            continue;
        }
        // Pick the reference page if present.
        let idx = *idxs.iter().find(|&&i| pages[i].0.to_lowercase().contains("/reference/")).unwrap_or(&idxs[0]);
        let (url, body) = &pages[idx];
        let att = attested.contains(url);
        let dp = read_doc_page(url, body, en, &bridge, &attested, &Default::default());
        let out = by_construct.get(*t);
        let cand = out.is_some();
        let verd = out.map(|o| format!("{:?}/{}", o.verdict, o.violating)).unwrap_or_else(|| "-".into());
        println!(
            "{:<12} {:<7} {:<6} {:<7} {:<6} {:<6} {:<8} {}",
            t, "yes", att, dp.prohibited, dp.constructs.len(), "?", cand, verd
        );
        if !cand {
            // Diagnose: constructs read, attested_deprecated
            println!("      url={}", url);
            println!("      attested_deprecated={} constructs={:?} incorrect={} correct={} example_code={}",
                dp.attested_deprecated, dp.constructs, dp.incorrect.len(), dp.correct.len(), dp.example_code.len());
        }
    }
}
