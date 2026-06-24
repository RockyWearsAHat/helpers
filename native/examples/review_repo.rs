//! Feed it a repository; it reads the whole folder and talks back in English.
//!
//!   cargo run --release --example review_repo [path]   # defaults to the parent repo
//!
//! This is the linter as the project intends it: an AI that learned its rules from documents
//! (the CS2420/CS3500 principles in `corpus/cs-principles.md`) and from per-language modules, then
//! reads an entire repository — every file, what language each is, the shape of the project — and
//! reports, in plain English, whether the code follows those principles, what to fix, and what it
//! could not analyze. It needs no language toolchain installed: parsing is the tree-sitter grammars
//! compiled into the binary, so it "just works" offline for any covered language.

use std::fs;
use std::path::PathBuf;

use helpers_native::linter::{review_repository, Knowledge, LintModule, ModuleRegistry, Reasoner};

/// Train+publish the Rust module once if the store doesn't have it (self-heals a gitignored store).
fn ensure_rust_module(reg: &mut ModuleRegistry) {
    if reg.load("rust-clippy").is_some() {
        return;
    }
    if let Ok(k) = Knowledge::from_corpus(&PathBuf::from("../corpus/lint-corpus.jsonl")) {
        let m = LintModule::pack("rust-clippy", "1.95.0", "corpus/lint-corpus.jsonl", "rust", &k);
        let _ = reg.publish(&m);
    }
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "..".to_string()));

    // 1) Learn the principles from the document. (review_repository then calibrates the reasoner to
    //    the repo itself — studies its idiomatic code and learns its behavioral norms — before
    //    judging, so a correct `0..len` in this project is recognized as normal.)
    let cs_doc = fs::read_to_string("../corpus/cs-principles.md").expect("read corpus/cs-principles.md");
    let mut reasoner = Reasoner::from_cs_principles("rust", &cs_doc);

    // 2) Pull the language modules lazily as the repo needs them.
    let mut registry = ModuleRegistry::open("../lint-modules");
    ensure_rust_module(&mut registry);

    // 3) Read the whole repository, calibrate to it, and talk back in English.
    let report = review_repository(&root, &mut reasoner, &mut registry);
    println!("{}", report.to_english());
}
