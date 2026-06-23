//! Final check on the unified signature detector ([`helpers_native::lint_sig`]): structure +
//! descriptions, per-rule, learned from the docs. Reports how many rules it grounds (and via
//! which modality), accuracy on the documented examples, and the false-flag rate on held-out
//! code it never studied.
//!
//!   cargo run --release --example measure_sig

use std::fs;
use std::path::{Path, PathBuf};

use helpers_native::lint_sig::{Rule, SigModel};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(n, "target" | ".git" | "node_modules" | ".helpers") {
                continue;
            }
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn main() {
    let raw = fs::read_to_string("../lint-index/clippy.json").expect("clippy.json");
    let idx: serde_json::Value = serde_json::from_str(&raw).expect("parse");
    let rules: Vec<Rule> = idx["rules"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| {
            let bad = r["exampleBad"].as_str().unwrap_or("");
            (!bad.is_empty()).then(|| Rule {
                id: r["id"].as_str().unwrap_or("").to_string(),
                bad: bad.to_string(),
                good: r["exampleGood"].as_str().unwrap_or("").to_string(),
                description: r["description"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect();

    let mut files = Vec::new();
    rust_files(Path::new(".."), &mut files);
    files.retain(|p| !p.to_string_lossy().contains("measure_"));
    files.sort();
    let sources: Vec<String> = files.iter().filter_map(|p| fs::read_to_string(p).ok()).collect();
    let split = sources.len() * 4 / 5;
    let (calib, held_out) = sources.split_at(split);
    let calib_refs: Vec<&str> = calib.iter().map(String::as_str).collect();
    let held_loc: usize = held_out.iter().map(|s| s.lines().count()).sum();

    let m = SigModel::train("rust", &rules, &calib_refs);
    let (by_struct, by_desc) = m.grounding();

    let (mut answered, mut correct) = (0usize, 0usize);
    for r in &rules {
        let hits = m.judge(&r.bad);
        if !hits.is_empty() {
            answered += 1;
            if hits.contains(&r.id) {
                correct += 1;
            }
        }
    }
    let mut ff = 0usize;
    for src in held_out {
        ff += m.judge(src).len();
    }

    println!("studied {} files ({} held-out, {} LOC); {} documented rules\n", sources.len(), held_out.len(), held_loc, rules.len());
    println!("grounded {} rules  ({by_struct} via structure, {by_desc} via description)", m.rule_count());
    println!("  = {:.0}% of documented rules now have a usable, learned signature", m.rule_count() as f64 / rules.len() as f64 * 100.0);
    println!("answered {answered}/{}  accuracy-when-answered {correct}/{answered} ({:.1}%)", rules.len(), correct as f64 / answered.max(1) as f64 * 100.0);
    println!("held-out flags: {ff} over {held_loc} LOC = {:.2}/100", ff as f64 / held_loc.max(1) as f64 * 100.0);
}
