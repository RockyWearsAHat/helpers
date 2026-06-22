//! Scale test for the mixture-of-experts linter: experts = clippy categories, routing
//! and reasoning in pure signal space. Reports recall, attribution (right rule), and
//! held-out false flags, plus expert count and timing — to compare against the flat pool.
//!
//!   cargo run --release --example measure_moe [clean_cap] [heldout_loc] [rule_stride] [cap] [filter] [topk]

use std::fs;
use std::path::Path;
use std::time::Instant;

use helpers_native::lint_moe::{Example, Moe};

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
    let a: Vec<String> = std::env::args().collect();
    let clean_cap: usize = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(1500);
    let heldout_cap: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(1500);
    let rule_stride: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let cap: u32 = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(1400);
    let filter: u32 = a.get(5).and_then(|s| s.parse().ok()).unwrap_or(1000);
    let topk: usize = a.get(6).and_then(|s| s.parse().ok()).unwrap_or(2);

    let raw = fs::read_to_string("../lint-index/clippy.json").expect("clippy.json");
    let idx: serde_json::Value = serde_json::from_str(&raw).expect("parse");
    let mut examples: Vec<Example> = Vec::new();
    for r in idx["rules"].as_array().expect("rules") {
        let bad = r["exampleBad"].as_str().unwrap_or("");
        let good = r["exampleGood"].as_str().unwrap_or("");
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

    let mut clean = Vec::new();
    rust_sources(Path::new("src"), &mut clean);
    clean.sort();
    let split = clean.len() * 4 / 5;
    let (calib, held_out) = clean.split_at(split);
    let mut cw: Vec<&str> = calib.iter().map(String::as_str).collect();
    if cw.len() > clean_cap {
        let stride = (cw.len() / clean_cap).max(1);
        cw = cw.iter().copied().step_by(stride).collect();
    }

    let t0 = Instant::now();
    let moe = Moe::train(&examples, &cw, filter, cap, topk);
    let train_s = t0.elapsed().as_secs_f64();
    let (n_experts, n_sigs) = moe.stats();

    // recall + attribution on a strided sample of rules.
    let t1 = Instant::now();
    let mut any = 0;
    let mut right = 0;
    let mut tested = 0;
    for (ri, e) in examples.iter().enumerate().step_by(rule_stride) {
        tested += 1;
        let hits = moe.judge(&e.bad);
        if !hits.is_empty() {
            any += 1;
        }
        // attribution: did it flag THIS rule's name?
        if hits.iter().any(|&h| moe.rule_name(h) == e.rule) {
            right += 1;
        }
        let _ = ri;
    }
    let tested = tested.max(1);

    // held-out false flags.
    let mut loc = 0usize;
    let mut ff = 0usize;
    for s in held_out {
        if loc >= heldout_cap {
            break;
        }
        loc += s.lines().count();
        ff += moe.judge(s).len();
    }
    let infer_s = t1.elapsed().as_secs_f64();

    println!("experts: {n_experts}  signals: {n_sigs}  (cap={cap} filter={filter} topk={topk})");
    println!("train {train_s:.1}s  infer {infer_s:.1}s");
    println!(
        "recall (flags own bad):          {any}/{tested} ({:.0}%)",
        any as f64 / tested as f64 * 100.0
    );
    println!(
        "attribution (flags RIGHT rule):  {right}/{tested} ({:.0}%)",
        right as f64 / tested as f64 * 100.0
    );
    println!(
        "held-out clean: {loc} LOC, {ff} false flags = {:.2} per 100 lines",
        ff as f64 / loc.max(1) as f64 * 100.0
    );
}
