//! Measure the `lint_ai` model on real data, honestly.
//!
//! Trains on the official clippy bad→good pairs, calibrates the per-rule radii on 80%
//! of this repo's own Rust (known-good), then reports:
//!   * how many rules survived training as separable,
//!   * self-recall: fraction of rules that flag their own bad example,
//!   * FALSE FLAGS on the held-out 20% of clean repo code (per 100 lines) — the number
//!     that has to approach zero. The held-out split is the point: the radius was never
//!     calibrated on it, so any flag there is a real generalization false positive.
//!
//!   cargo run --example measure_ai [window] [margin]

use std::fs;
use std::path::Path;

use helpers_native::lint_ai::Model;

/// Recursively collect `(path, text)` for every `.rs` file under `dir`.
fn rust_sources(dir: &Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            if let Ok(text) = fs::read_to_string(&path) {
                out.push((path.display().to_string(), text));
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let window: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);
    let cap: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2048);
    let show = args.get(3).map(|s| s == "show").unwrap_or(false);

    // 1. Official clippy rules → (id, bad, good) pairs.
    let raw = fs::read_to_string("../lint-index/clippy.json").expect("read clippy.json");
    let idx: serde_json::Value = serde_json::from_str(&raw).expect("parse clippy.json");
    let mut pairs: Vec<(String, String, String)> = Vec::new();
    for r in idx["rules"].as_array().expect("rules array") {
        let bad = r["exampleBad"].as_str().unwrap_or("");
        let good = r["exampleGood"].as_str().unwrap_or("");
        if !bad.is_empty() && !good.is_empty() {
            pairs.push((
                r["id"].as_str().unwrap_or("").to_string(),
                bad.to_string(),
                good.to_string(),
            ));
        }
    }

    // 2. This repo's Rust as known-good code; 80/20 split for calibrate vs held-out.
    let mut clean = Vec::new();
    rust_sources(Path::new("src"), &mut clean);
    clean.sort();
    let split = clean.len() * 4 / 5;
    let (calib, held_out) = clean.split_at(split);
    let calib_refs: Vec<&str> = calib.iter().map(|(_, t)| t.as_str()).collect();

    // 3. Train.
    let model = Model::train(window, cap, &pairs, &calib_refs);

    // 4a. Self-recall: does each rule flag its own documented bad example?
    let recalled = pairs
        .iter()
        .filter(|(id, bad, _)| model.judge(bad).iter().any(|f| &f.rule_id == id))
        .count();

    // 4b. Flags on held-out clean code — print each so they can be judged, not assumed.
    let held_loc: usize = held_out.iter().map(|(_, t)| t.lines().count()).sum();
    let mut total = 0usize;
    for (path, text) in held_out {
        let lines: Vec<&str> = text.lines().collect();
        for f in model.judge(text) {
            total += 1;
            if show {
                let src = lines.get(f.line.saturating_sub(1)).unwrap_or(&"").trim();
                println!("  {}:{}  [{}]  {}", path, f.line, f.rule_id, src);
            }
        }
    }
    let per_100 = total as f64 / held_loc.max(1) as f64 * 100.0;

    println!("window={window} cap={cap}");
    println!("clippy bad→good pairs: {}", pairs.len());
    println!("rules trained (separable): {}", model.rule_count());
    println!(
        "self-recall: {recalled}/{} ({:.0}%)",
        pairs.len(),
        recalled as f64 / pairs.len().max(1) as f64 * 100.0
    );
    println!("held-out clean: {held_loc} LOC, {total} flags = {per_100:.2} per 100 lines");
}
