//! Train per-language MoE linters from the scraped docs and save them. One-time
//! (checksum-gated) job: the `lint` tool then LOADS these instead of retraining.
//!
//!   cargo run --release --example train_lint            # all languages in lint-index/
//!   cargo run --release --example train_lint rust       # just one

use std::fs;
use std::path::Path;

use helpers_native::lint_moe::{Example, Moe};

const FILTER: u32 = 1200;
const CAP: u32 = 1400;
const TOPK: usize = 9; // consult all experts → ~94% attribution; inference still ~2s

fn rust_sources(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            if let Ok(t) = fs::read_to_string(&p) {
                out.push(t);
            }
        }
    }
}

fn main() {
    let only = std::env::args().nth(1);
    let index_dir = Path::new("../lint-index");
    let entries = fs::read_dir(index_dir).expect("read lint-index/");

    // Repo Rust as the clean reference for the rust model (sharper distinctiveness).
    let mut rust_clean = Vec::new();
    rust_sources(Path::new("src"), &mut rust_clean);

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        if path.file_name().is_some_and(|n| n == "sources.json") {
            continue;
        }
        let raw = match fs::read_to_string(&path) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let idx: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let lang = idx["language"].as_str().unwrap_or("").to_string();
        if lang.is_empty() {
            continue;
        }
        if let Some(want) = &only {
            if &lang != want {
                continue;
            }
        }

        let mut examples = Vec::new();
        for r in idx["rules"].as_array().into_iter().flatten() {
            let bad = r["exampleBad"].as_str().unwrap_or("");
            let good = r["exampleGood"].as_str().unwrap_or("");
            // Only a bad example is required — the good one just adds to the clean
            // reference. Requiring both would drop ~40% of rules (e.g. bool_comparison).
            if bad.is_empty() {
                continue;
            }
            examples.push(Example {
                rule: r["id"].as_str().unwrap_or("").to_string(),
                slice: r["category"].as_str().unwrap_or("other").to_string(),
                bad: bad.to_string(),
                good: good.to_string(),
            });
        }
        if examples.is_empty() {
            println!("{lang}: no bad→good pairs, skipped");
            continue;
        }

        let clean: Vec<&str> = if lang == "rust" {
            rust_clean.iter().map(String::as_str).collect()
        } else {
            Vec::new() // good examples serve as the distinctiveness reference
        };
        let moe = Moe::train(&examples, &clean, FILTER, CAP, TOPK);
        let (experts, sigs) = moe.stats();
        let out = Moe::model_path(&lang);
        match moe.save(&out) {
            Ok(()) => println!(
                "{lang}: {} rules, {experts} experts, {sigs} signals -> {}",
                examples.len(),
                out.display()
            ),
            Err(e) => println!("{lang}: save failed: {e}"),
        }
    }
}
