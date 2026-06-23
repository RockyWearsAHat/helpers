//! The whole loop in one command: crawled documentation → knowledge built into the net → lint.
//!
//!   cargo run --release --example learn_from_docs [corpus.jsonl]
//!   cargo run --release --example learn_from_docs --seed doc.rust-lang.org   # live crawl
//!
//! The input is a CRAWL, not a hand-structured file. `scripts/crawl-docs.mjs` walks an entire
//! documentation site in-domain and emits one JSONL record per rule-candidate
//! ({language, description, bad, good, …}); this reads that crawl and builds it straight into
//! the net — a cross-language concept lexicon (lint_concept) plus per-language structural
//! signatures (lint_sig). Nothing here is clippy-specific: whatever the crawler read is what the
//! model learns. We default to the committed `corpus/lint-corpus.jsonl` (2221 rules already
//! crawled from rust/js/go/markdown docs) so it runs offline; `--seed <host>` crawls live first.
//!
//! What we are NO LONGER responsible for: structuring the knowledge by hand. The crawl produces
//! the records; this turns them into the trained net.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use helpers_native::lint_concept::Lexicon;
use helpers_native::lint_sig::{Rule as SigRule, SigModel};

/// One crawled rule-candidate, exactly as `crawl-docs.mjs` / the corpus emits it.
struct Record {
    language: String,
    id: String,
    description: String,
    bad: String,
    good: String,
}

/// Minimal JSONL parse via serde_json (the corpus is well-formed JSON per line).
fn read_corpus(path: &str) -> Vec<Record> {
    let text = fs::read_to_string(path).unwrap_or_else(|_| panic!("cannot read crawl corpus {path}"));
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .map(|v| Record {
            language: v["language"].as_str().unwrap_or("").to_string(),
            id: v["rule"].as_str().unwrap_or("").to_string(),
            description: v["description"].as_str().unwrap_or("").to_string(),
            bad: v["bad"].as_str().unwrap_or("").to_string(),
            good: v["good"].as_str().unwrap_or("").to_string(),
        })
        .collect()
}

/// Crawl a live docs host into a corpus jsonl using the existing crawler, returning its path.
fn crawl(seed: &str) -> String {
    println!("Crawling the whole site under {seed} … (scripts/crawl-docs.mjs)");
    let status = Command::new("node")
        .args(["scripts/crawl-docs.mjs", "--seed", seed])
        .current_dir("..")
        .status()
        .expect("failed to run node scripts/crawl-docs.mjs");
    assert!(status.success(), "crawl failed");
    format!("../crawl-index/{seed}.corpus.jsonl")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let corpus_path = if args.get(1).map(|s| s == "--seed").unwrap_or(false) {
        crawl(args.get(2).expect("--seed needs a hostname"))
    } else {
        args.get(1).cloned().unwrap_or_else(|| "../corpus/lint-corpus.jsonl".to_string())
    };

    let records = read_corpus(&corpus_path);
    let mut by_lang: BTreeMap<String, usize> = BTreeMap::new();
    for r in &records {
        *by_lang.entry(r.language.clone()).or_default() += 1;
    }
    println!(
        "Read {} crawled rule-candidates from {} ({:?}).\n",
        records.len(),
        PathBuf::from(&corpus_path).file_name().and_then(|s| s.to_str()).unwrap_or(&corpus_path),
        by_lang
    );

    // 1) Build the cross-language CONCEPT lexicon straight from the crawl (description + code).
    let lex_input: Vec<(&str, &str, &str)> = records
        .iter()
        .filter(|r| !r.description.is_empty() && !r.bad.is_empty())
        .map(|r| (r.language.as_str(), r.description.as_str(), r.bad.as_str()))
        .collect();
    let lex = Lexicon::learn(&lex_input);

    // 2) Build per-language structural SIGNATURES from the crawl (id + bad + good + description).
    let langs: Vec<String> = by_lang.keys().cloned().collect();
    let mut sig_counts: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new(); // lang -> (rules, struct, desc)
    for lang in &langs {
        let rules: Vec<SigRule> = records
            .iter()
            .filter(|r| &r.language == lang && !r.bad.is_empty())
            .map(|r| SigRule {
                id: r.id.clone(),
                bad: r.bad.clone(),
                good: r.good.clone(),
                description: r.description.clone(),
            })
            .collect();
        if rules.is_empty() {
            continue;
        }
        // The crawl's own examples are the corpus the signatures calibrate against.
        let corpus: Vec<&str> = records.iter().filter(|r| &r.language == lang).map(|r| r.bad.as_str()).collect();
        let model = SigModel::train(lang, &rules, &corpus);
        let (s, d) = model.grounding();
        sig_counts.insert(lang.clone(), (model.rule_count(), s, d));
    }

    println!("Built into the net — straight from the crawl, no hand-structuring:");
    println!("  concept lexicon: {} concepts across {:?}", lex.concept_count(), langs);
    for (lang, (rules, s, d)) in &sig_counts {
        println!("  {lang:10} signatures: {rules} rules grounded ({s} structural, {d} from descriptions)");
    }

    // 3) It can now lint/review — show learned semantic comprehension on unseen code per language.
    println!("\nThe net now comprehends code it never saw (concepts learned from the crawled docs):");
    let probes = [
        ("rust", "fn n(s: &str) -> i64 { s.parse().unwrap() }"),
        ("javascript", "function load(u){ return fetch(u).then(r=>r.json()); }"),
        ("rust", "fn dup(v: &[String]) -> Vec<String> { v.iter().map(|s| s.clone()).collect() }"),
    ];
    for (lang, code) in probes {
        let profile: Vec<String> = lex.concepts_of(lang, code, 5).into_iter().map(|(c, s)| format!("{c}({s:.2})")).collect();
        println!("  [{lang}] {} → {}", code.chars().take(46).collect::<String>(), profile.join(", "));
    }

    println!("\nInput was a crawl of documentation URLs; output is a trained, multi-language net.");
    println!("Point it at a new site with `--seed <host>` and the knowledge rebuilds itself.");
}
