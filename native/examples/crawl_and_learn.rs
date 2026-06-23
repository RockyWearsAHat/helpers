//! Hand it the official docs homepage; it crawls the whole site (direct fetch, no browser) and
//! builds what it reads into the net. The autonomous version of "learn a language from its docs".
//!
//!   cargo run --release --features crawl --example crawl_and_learn -- <seed-url> [--max N] [--lang L]
//!   e.g. ... -- https://doc.rust-lang.org/book/ --max 150 --lang rust
//!
//! It graph-crawls in-domain from the seed, extracts each page's prose + code blocks, and trains
//! the concept lexicon (lint_concept) on the (prose, code) pairs — so the model becomes an expert
//! on whatever language's documentation it was pointed at, from the source the maintainers publish.
//! Nothing is hand-structured: the crawl is the training set.

use helpers_native::doc_crawler::{crawl, Page};
use helpers_native::lint_concept::Lexicon;

/// Best-effort language from the seed host, so code blocks parse in the right grammar. A `--lang`
/// flag overrides it.
fn lang_from_host(seed: &str) -> &'static str {
    let s = seed.to_lowercase();
    if s.contains("python") || s.contains("astral") {
        "python"
    } else if s.contains("golang") || s.contains("go.dev") {
        "go"
    } else if s.contains("eslint") || s.contains("javascript") || s.contains("mdn") || s.contains("typescript") {
        "javascript"
    } else {
        "rust"
    }
}

fn arg(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed = args.iter().skip(1).find(|a| a.starts_with("http")).cloned().unwrap_or_else(|| {
        eprintln!("usage: crawl_and_learn -- <seed-url> [--max N] [--lang L]");
        std::process::exit(2);
    });
    let max: usize = arg(&args, "--max").and_then(|s| s.parse().ok()).unwrap_or(150);
    let lang = arg(&args, "--lang").unwrap_or_else(|| lang_from_host(&seed).to_string());

    println!("Crawling {seed} (direct fetch, no browser; up to {max} pages, lang={lang})…\n");
    let pages: Vec<Page> = crawl(&[&seed], max, 150);
    let total_code: usize = pages.iter().map(|p| p.code.len()).sum();
    println!("\nCrawled {} pages, {total_code} code blocks. Building into the net…", pages.len());

    // Each (local prose, code) section is a training record — the snippet paired with the
    // explanation right above it, so co-occurrence stays tight instead of whole-page-blurred.
    let mut records: Vec<(String, String, String)> = Vec::new();
    for p in &pages {
        for (prose, code) in &p.sections {
            records.push((lang.clone(), prose.clone(), code.clone()));
        }
    }
    let refs: Vec<(&str, &str, &str)> = records.iter().map(|(l, d, c)| (l.as_str(), d.as_str(), c.as_str())).collect();
    let lex = Lexicon::learn(&refs);

    println!(
        "Learned {} concepts from {} (prose, code) records crawled off the official docs.\n",
        lex.concept_count(),
        refs.len()
    );
    println!("A few concepts and what the docs taught they look like in code:");
    for concept in ["iterator", "mutable", "borrow", "panic", "async", "trait", "error", "owned"] {
        let m = lex.meaning_of(concept, 6);
        if !m.is_empty() {
            let cs: Vec<String> = m.iter().map(|(k, n)| format!("{k}({n})")).collect();
            println!("  {concept:10} → {}", cs.join(", "));
        }
    }
    println!("\nThe input was a documentation URL; the net is what it found by crawling everything.");
}
