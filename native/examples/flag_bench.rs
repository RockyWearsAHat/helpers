//! Isolates the single-file cost the edit path pays: parse+flag of one real file through
//! the real machine model, then the pieces. Run: cargo run --release --example flag_bench

fn main() {
    let root = std::path::PathBuf::from("/Users/alexwaldmann/bin");
    let src = std::fs::read_to_string(root.join("pages/assets/neural.js")).expect("file");
    let (_, models) = helpers_native::lint_train::ensure_models(
        &["javascript".to_string()],
        &root,
        &root,
        &helpers_native::lint_train::NoProject,
    );
    let model = models.get("javascript").expect("model");
    // Warm one flag (lazy grammar init), then measure.
    let _ = model.rules.flag(&src);
    for round in 0..3 {
        let t = std::time::Instant::now();
        let findings = model.rules.flag(&src);
        let d_flag = t.elapsed();
        let t = std::time::Instant::now();
        let items: Vec<(&str, Vec<&str>)> = findings
            .iter()
            .map(|f| (f.rule.as_str(), vec!["var", "x"]))
            .collect();
        let _ = model.concept.confirms_batch(&items);
        let d_gate = t.elapsed();
        println!(
            "round {round}: flag {:.0}µs ({} findings, {} bytes), gate batch {:.0}µs",
            d_flag.as_micros(),
            findings.len(),
            src.len(),
            d_gate.as_micros()
        );
    }
}
