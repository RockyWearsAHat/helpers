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
        let Some(memory) = lint_train::cached_memory(lang) else {
            println!("== {lang}: no cached memory\n");
            continue;
        };
        let t = Instant::now();
        let outcomes = lint_module::graduate(lang, &pages, &memory, m, en);
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
