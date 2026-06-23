//! Honest held-out comparison of the linter's *learning*: does reading the good code and
//! abstaining-unless-distinctive (precision mode) actually make it know good from bad when
//! sent out to code it never saw?
//!
//!   cargo run --release --example measure_precise
//!
//! It reads a broad corpus of real Rust (the whole repo), holds out 20% it never trains on,
//! trains two ways — with the recall fallback (default) and without it (precise) — and reports
//! recall, attribution, and the false-flag rate on the HELD-OUT files. The held-out number is
//! the one that matters: it is the model working on code it did not learn from.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use helpers_native::lint_moe::{Example, Moe};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(name, "target" | ".git" | "node_modules" | ".helpers") {
                continue;
            }
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn measure(label: &str, moe: &Moe, examples: &[Example], held_out: &[String]) {
    // Self-recall + attribution on the documented bad examples.
    let (mut any, mut right) = (0usize, 0usize);
    for e in examples {
        let hits = moe.judge(&e.bad);
        if !hits.is_empty() {
            any += 1;
        }
        if hits.iter().any(|&h| moe.rule_name(h) == e.rule) {
            right += 1;
        }
    }
    // False flags on held-out clean code the model never trained on.
    let (mut loc, mut ff) = (0usize, 0usize);
    for s in held_out {
        loc += s.lines().count();
        ff += moe.judge(s).len();
    }
    let n = examples.len().max(1);
    println!(
        "{label:8}  recall {any:>3}/{n} ({:>3.0}%)  attribution {right:>3}/{n} ({:>3.0}%)  held-out {ff} false flags / {loc} LOC = {:.2}/100",
        any as f64 / n as f64 * 100.0,
        right as f64 / n as f64 * 100.0,
        ff as f64 / loc.max(1) as f64 * 100.0
    );
}

fn main() {
    // Real Rust documentation: clippy rules with bad (and where present, good) examples.
    let raw = fs::read_to_string("../lint-index/clippy.json").expect("clippy.json");
    let idx: serde_json::Value = serde_json::from_str(&raw).expect("parse");
    let mut examples = Vec::new();
    for r in idx["rules"].as_array().expect("rules") {
        let bad = r["exampleBad"].as_str().unwrap_or("");
        if bad.is_empty() {
            continue;
        }
        examples.push(Example {
            rule: r["id"].as_str().unwrap_or("").into(),
            slice: r["category"].as_str().unwrap_or("other").into(),
            bad: bad.into(),
            good: r["exampleGood"].as_str().unwrap_or("").into(),
        });
    }

    // Broad clean corpus: ALL Rust in the repo. Read widely, then hold out 20% it never sees.
    let mut files = Vec::new();
    rust_files(Path::new(".."), &mut files);
    files.retain(|p| !p.to_string_lossy().contains("examples/measure_precise"));
    files.sort();
    let mut sources: Vec<String> = files.iter().filter_map(|p| fs::read_to_string(p).ok()).collect();
    sources.sort();
    let split = sources.len() * 4 / 5;
    let (calib, held_out) = sources.split_at(split);
    let calib_refs: Vec<&str> = calib.iter().map(String::as_str).collect();
    let held_loc: usize = held_out.iter().map(|s| s.lines().count()).sum();
    println!(
        "read {} Rust files ({} train, {} held-out = {} LOC); {} documented rules\n",
        sources.len(), calib.len(), held_out.len(), held_loc, examples.len()
    );

    let held = held_out.to_vec();
    let t = Instant::now();
    let default = Moe::train(&examples, &calib_refs, 1000, 1400, 2);
    measure("default", &default, &examples, &held);
    println!("--- precise mode: abstain unless distinctive; sweep the distinctiveness bar ---");
    for filter in [1000u32, 1500, 2000, 2500, 3000] {
        let precise = Moe::train_precise(&examples, &calib_refs, filter, 1400, 2);
        measure(&format!("f={filter}"), &precise, &examples, &held);
    }
    println!("\n(trained in {:.0}s) Held-out false flags = the model on code it never read.", t.elapsed().as_secs_f64());
    println!("Higher distinctiveness bar = it only fires when it KNOWS, abstaining otherwise.");
}
