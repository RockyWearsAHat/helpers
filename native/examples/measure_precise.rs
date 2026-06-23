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
    // Accuracy = of the rules it CHOSE to answer on, how many got the right rule. This is the
    // "100% through abstaining" number: it should be 100% even while coverage (recall) is low.
    let accuracy = if any == 0 { 100.0 } else { right as f64 / any as f64 * 100.0 };
    println!(
        "{label:9}  answered {any:>3}/{n} ({:>3.0}%)  accuracy-when-answered {right}/{any} ({:>5.1}%)  held-out {ff} false flags / {loc} LOC = {:.2}/100",
        any as f64 / n as f64 * 100.0,
        accuracy,
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
    println!("--- flat lexer-token representation (what the model used to reason over) ---");
    let lexer = Moe::train_precise(&examples, &calib_refs, 2000, 1400, 2);
    measure("lexer", &lexer, &examples, &held);
    println!("--- STRUCTURAL (AST) representation, sweeping the distinctiveness bar ---");
    let siblings = [
        ("single_match", "single_match_else"),
        ("vec_box", "box_collection"),
        ("useless_transmute", "wrong_transmute"),
    ];
    let by_id = |id: &str| examples.iter().find(|e| e.rule == id);
    for filter in [1000u32, 1400, 1800, 2200] {
        let ast = Moe::train_ast(&examples, &calib_refs, filter, 1400, 2, "rust");
        measure(&format!("ast f={filter}"), &ast, &examples, &held);
        let mut sib = String::new();
        for (a, b) in siblings {
            for id in [a, b] {
                if let Some(e) = by_id(id) {
                    let hits: Vec<&str> = ast.judge(&e.bad).iter().map(|&h| ast.rule_name(h)).collect();
                    let v = if hits.is_empty() { "abstain" } else if hits.contains(&id) { "✓" } else { "✗WRONG" };
                    sib.push_str(&format!(" {id}={v}"));
                }
            }
        }
        println!("        siblings:{sib}");
    }

    // Recursive MoE: split each slice into sub-experts of <= cap signals. If routing
    // interference was capping recall, finer experts reach signals a single overloaded expert
    // drowned — more rules answered at the same precision. cap=0 is the flat baseline.
    println!("\n--- recursive MoE (structural, f=1400, topk=4): does the net hold more? ---");
    for cap in [0usize, 12, 6, 3] {
        let m = Moe::train_recursive(&examples, &calib_refs, 1400, 1400, 4, "rust", cap);
        let (experts, _sigs) = m.stats();
        let label = if cap == 0 { "flat".to_string() } else { format!("cap={cap}") };
        print!("{label:8} experts={experts:<4} ");
        measure("", &m, &examples, &held);
    }
    println!("\n(trained in {:.0}s)", t.elapsed().as_secs_f64());
}
