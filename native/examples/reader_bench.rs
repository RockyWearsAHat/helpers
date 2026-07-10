//! `reader_bench` — measures the 1-bit predictive-coding core in isolation, so "training is
//! slow" claims can be split into what the AI costs (this) and what the scaffolding costs
//! (network crawls, toolchain process spawns, pattern compilation).
//!
//! Run: `cargo run --release --example reader_bench`

use std::time::Instant;

use helpers_native::lint_read::{PolarityBuilder, Reader};

fn main() {
    // A realistic reading corpus: the shipped teaching prose (`extraDocs/*.md` — the same
    // material the linter itself reads), split to sentences and repeated to a ~5 MB body.
    // (The former `lint-corpus.jsonl` rule catalog is deleted; enforcement grows from reading.)
    let docs_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../extraDocs");
    let mut prose = String::new();
    for entry in std::fs::read_dir(&docs_dir).expect("corpus present").flatten() {
        if entry.path().extension().is_some_and(|e| e == "md") {
            prose.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
            prose.push('\n');
        }
    }
    assert!(!prose.is_empty(), "corpus present");
    let sentences: Vec<String> =
        helpers_native::lint_read::sentences(&prose).iter().map(|s| s.to_string()).collect();
    let mut body = String::new();
    while body.len() < 5_000_000 {
        for s in &sentences {
            body.push_str(s);
            body.push(' ');
        }
    }
    let tokens = helpers_native::lint_read::tokens(&body).len();

    // READ (learn): the predictive-coding write path over the whole body.
    let mut reader = Reader::new();
    let t = Instant::now();
    reader.learn_span(&body);
    let read_s = t.elapsed().as_secs_f64();
    println!(
        "read  {:>9} tokens ({:.1} MB) in {:.2}s  → {:>9.0} tokens/s",
        tokens,
        body.len() as f64 / 1e6,
        read_s,
        tokens as f64 / read_s
    );

    // TRAIN (polarity): accumulate every sentence into the prototypes.
    let t = Instant::now();
    let mut b = PolarityBuilder::new(reader);
    for (i, s) in sentences.iter().enumerate() {
        b.accumulate(s, i % 2 == 0);
    }
    let polarity = b.build();
    let train_s = t.elapsed().as_secs_f64();
    println!(
        "train {:>9} labeled sentences in {:.2}s  → {:>9.0} sentences/s",
        sentences.len(),
        train_s,
        sentences.len() as f64 / train_s
    );

    // CLASSIFY (inference): verdicts per second.
    let t = Instant::now();
    let mut verdicts = 0usize;
    for s in sentences.iter().cycle().take(20_000) {
        if polarity.classify(s).is_some() {
            verdicts += 1;
        }
    }
    let cls_s = t.elapsed().as_secs_f64();
    println!(
        "class {:>9} sentences in {:.2}s  → {:>9.0} sentences/s ({} rendered a verdict)",
        20_000,
        cls_s,
        20_000f64 / cls_s,
        verdicts
    );
}
