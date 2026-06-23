//! Show the model learning SEMANTIC concepts from the documentation alone.
//!
//!   cargo run --release --example learn_concepts
//!
//! It reads every clippy rule's description (English) paired with its bad example (code), and
//! learns a co-occurrence lexicon between concepts and constructs. Then it prints, for a handful
//! of type/ownership/dataflow concepts, what the model learned they LOOK LIKE in code — and, for
//! a few functions, what concepts the model thinks they INVOLVE. No type engine: the semantics
//! are grown from the docs, the seed the user pointed at.

use std::fs;

use helpers_native::lint_concept::Lexicon;

fn main() {
    let raw = fs::read_to_string("../lint-index/clippy.json").expect("clippy.json");
    let idx: serde_json::Value = serde_json::from_str(&raw).expect("parse");
    let rules: Vec<(String, String)> = idx["rules"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| {
            let desc = r["description"].as_str().unwrap_or("");
            let bad = r["exampleBad"].as_str().unwrap_or("");
            (!desc.is_empty() && !bad.is_empty()).then(|| (desc.to_string(), bad.to_string()))
        })
        .collect();
    let refs: Vec<(&str, &str, &str)> = rules.iter().map(|(d, b)| ("rust", d.as_str(), b.as_str())).collect();

    let lex = Lexicon::learn(&refs);
    println!("Read {} documented rules → learned {} concepts.\n", refs.len(), lex.concept_count());

    println!("What the model learned each concept LOOKS LIKE in code (from the docs):");
    for concept in ["clone", "iterator", "mutable", "borrow", "unsafe", "panic", "allocation", "const", "format", "transmute"] {
        let m = lex.meaning_of(concept, 6);
        if m.is_empty() {
            continue;
        }
        let constructs: Vec<String> = m.iter().map(|(k, n)| format!("{k}({n})")).collect();
        println!("  {concept:12} → {}", constructs.join(", "));
    }

    println!("\nWhat concepts the model thinks a function INVOLVES (its learned semantic profile):");
    let samples = [
        ("a clone-heavy fn", "fn dup(items: &[String]) -> Vec<String> { items.iter().map(|s| s.clone()).collect() }"),
        ("an unsafe transmute", "fn cast(x: u32) -> i32 { unsafe { std::mem::transmute(x) } }"),
        ("a panicky parse", "fn n(s: &str) -> i64 { s.parse().unwrap() }"),
    ];
    for (label, code) in samples {
        let cs = lex.concepts_of("rust", code, 6);
        let profile: Vec<String> = cs.iter().map(|(c, s)| format!("{c}({s:.2})")).collect();
        println!("  {label:20} → {}", profile.join(", "));
    }

    println!("\nThe concepts (Copy/mutable/borrow/unsafe/…) and what they mean in code were never");
    println!("hand-coded — they were read from the rule descriptions. This is the seed a dataflow-");
    println!("aware layer grows from: the model already knows which constructs a concept implies.");
}
