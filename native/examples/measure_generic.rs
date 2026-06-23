//! Does the "read the docs, learn the language" idea actually work — with NO hand-written
//! rules? The linter parses every rule's bad/good examples into a fully generic AST encoding
//! (node label + parent edge, zero special cases), studies a corpus of real code to learn how
//! RARE each structure is, and keeps as a rule's signature only the rare structures unique to
//! its bad example. A target is flagged when it exhibits that whole signature.
//!
//!   cargo run --release --example measure_generic
//!
//! The bet (the dictionary analogy): common grammar (`struct`, `block`, `field`) is frequent in
//! the corpus → weight ~0; the distinctive structure of a violation (`Vec<Box>` nesting) is rare
//! → it's the signature. If that's right, siblings separate and clean code is never flagged,
//! without a single per-pattern rule in the code.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use helpers_native::lint_ast::generic_features;

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

struct Rule {
    id: String,
    bad: String,
    good: String,
}

fn feats(code: &str) -> HashSet<String> {
    generic_features("rust", code).into_iter().map(|(f, _)| f).collect()
}

fn main() {
    // Documentation: clippy rules with examples.
    let raw = fs::read_to_string("../lint-index/clippy.json").expect("clippy.json");
    let idx: serde_json::Value = serde_json::from_str(&raw).expect("parse");
    let rules: Vec<Rule> = idx["rules"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| {
            let bad = r["exampleBad"].as_str().unwrap_or("");
            if bad.is_empty() {
                return None;
            }
            Some(Rule {
                id: r["id"].as_str().unwrap_or("").to_string(),
                bad: bad.to_string(),
                good: r["exampleGood"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect();

    // Study the language: document frequency of each generic structural feature over a corpus.
    let mut files = Vec::new();
    rust_files(Path::new(".."), &mut files);
    files.retain(|p| !p.to_string_lossy().contains("measure_generic"));
    files.sort();
    let sources: Vec<String> = files.iter().filter_map(|p| fs::read_to_string(p).ok()).collect();
    let split = sources.len() * 4 / 5;
    let (calib, held_out) = sources.split_at(split);

    let mut df: HashMap<String, usize> = HashMap::new();
    for src in calib {
        for f in feats(src) {
            *df.entry(f).or_default() += 1;
        }
    }
    let held_loc: usize = held_out.iter().map(|s| s.lines().count()).sum();
    println!(
        "studied {} files ({} corpus / {} held-out, {} LOC); {} documented rules\n",
        sources.len(), calib.len(), held_out.len(), held_loc, rules.len()
    );

    let siblings = [
        ("single_match", "single_match_else"),
        ("vec_box", "box_collection"),
        ("useless_transmute", "wrong_transmute"),
        ("iter_filter_is_some", "option_filter_map"),
    ];

    // A rule's signature = its rare (df <= max_df) structural features not present in its fix.
    // `min_sig` requires more than one coincidental rare structure to fire, which is what drives
    // held-out false flags to zero without losing the distinctive multi-feature violations.
    for (max_df, min_sig) in [(0usize, 1usize), (1, 1), (1, 2), (2, 2), (2, 3)] {
        let sigs: Vec<(String, HashSet<String>)> = rules
            .iter()
            .filter_map(|r| {
                let good = if r.good.is_empty() { HashSet::new() } else { feats(&r.good) };
                let sig: HashSet<String> = feats(&r.bad)
                    .into_iter()
                    .filter(|f| !good.contains(f) && df.get(f).copied().unwrap_or(0) <= max_df)
                    .collect();
                (sig.len() >= min_sig).then(|| (r.id.clone(), sig))
            })
            .collect();

        let fires = |code_feats: &HashSet<String>| -> Vec<&str> {
            sigs.iter()
                .filter(|(_, s)| s.iter().all(|f| code_feats.contains(f)))
                .map(|(id, _)| id.as_str())
                .collect()
        };

        // Coverage + accuracy on the documented bad examples.
        let (mut answered, mut correct) = (0usize, 0usize);
        for r in &rules {
            let hits = fires(&feats(&r.bad));
            if !hits.is_empty() {
                answered += 1;
                if hits.contains(&r.id.as_str()) {
                    correct += 1;
                }
            }
        }
        // Held-out false flags: clean code it never studied.
        let mut ff = 0usize;
        for src in held_out {
            ff += fires(&feats(src)).len();
        }
        let acc = if answered == 0 { 100.0 } else { correct as f64 / answered as f64 * 100.0 };
        println!(
            "df<={max_df} sig>={min_sig}: grounded {} | answered {answered} | accuracy {correct}/{answered} ({acc:.1}%) | held-out {ff} FF ({:.2}/100 LOC)",
            sigs.len(),
            ff as f64 / held_loc.max(1) as f64 * 100.0,
        );

        // Sibling separation: judge each sibling's bad example.
        let by_id: HashMap<&str, &Rule> = rules.iter().map(|r| (r.id.as_str(), r)).collect();
        let mut line = String::new();
        for (a, b) in siblings {
            for id in [a, b] {
                if let Some(r) = by_id.get(id) {
                    let hits = fires(&feats(&r.bad));
                    let v = if hits.is_empty() { "abstain" } else if hits == [id] { "✓only" } else if hits.contains(&id) { "✓+others" } else { "✗wrong" };
                    line.push_str(&format!(" {id}={v}"));
                }
            }
        }
        println!("   siblings:{line}\n");
    }
    println!("No per-rule or per-node-kind code: signatures are rare generic structures, learned from the corpus.");
}
