//! Diagnostic: of the rules whose bad and good examples encode to the SAME structural features,
//! what information actually distinguishes them? Splits them into "my encoding is lossy"
//! (operators, modifiers, attributes the generic walk drops — recoverable in syntax) vs
//! "genuinely semantic" (types, dataflow, purity — needs comprehension). Grounds the question
//! of how much of the gap is a richer encoding vs real semantic learning.
//!
//!   cargo run --release --example diag_semantic

use std::collections::HashSet;
use std::fs;

use helpers_native::lint_ast::generic_features;

fn feats(code: &str) -> HashSet<String> {
    generic_features("rust", code).into_iter().map(|(f, _)| f).collect()
}

fn main() {
    let raw = fs::read_to_string("../lint-index/clippy.json").expect("clippy.json");
    let idx: serde_json::Value = serde_json::from_str(&raw).expect("parse");
    let rules = idx["rules"].as_array().unwrap();

    let mut same = 0;
    let mut shown = 0;
    for r in rules {
        let bad = r["exampleBad"].as_str().unwrap_or("");
        let good = r["exampleGood"].as_str().unwrap_or("");
        if bad.is_empty() || good.is_empty() {
            continue;
        }
        let fb = feats(bad);
        let fg = feats(good);
        // Structurally indistinguishable under the current generic encoding.
        if fb.difference(&fg).next().is_none() {
            same += 1;
            if shown < 22 {
                let id = r["id"].as_str().unwrap_or("");
                println!("── {id}");
                println!("   bad : {}", bad.replace('\n', " ⏎ "));
                println!("   good: {}", good.replace('\n', " ⏎ "));
                shown += 1;
            }
        }
    }
    println!("\n{same} rules have bad ⊆ good structurally (no distinguishing feature in the current encoding).");
}
