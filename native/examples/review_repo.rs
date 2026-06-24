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
use std::path::{Path, PathBuf};

use helpers_native::linter::{review_repository, Knowledge, LintModule, ModuleRegistry, Reasoner};

fn ext_lang(p: &Path) -> Option<&'static str> {
    match p.extension().and_then(|e| e.to_str())? {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "js" | "mjs" | "cjs" => Some("javascript"),
        "ts" => Some("typescript"),
        "go" => Some("go"),
        _ => None,
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(n, "target" | ".git" | "node_modules" | ".helpers" | "dist" | "build") {
                continue;
            }
            walk(&p, out);
        } else if ext_lang(&p).is_some() {
            out.push(p);
        }
    }
}

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

    // 1) Learn the principles from the document, and read idiomatic reference so grounding is
    //    tested (precise or abstaining) rather than guessed.
    let cs_doc = fs::read_to_string("../corpus/cs-principles.md").expect("read corpus/cs-principles.md");
    let mut reasoner = Reasoner::from_cs_principles("rust", &cs_doc);
    if let Ok(k) = Knowledge::from_corpus(&PathBuf::from("../corpus/lint-corpus.jsonl")) {
        let store: Vec<String> = k.rules.iter().filter(|r| r.language == "rust" && !r.good.is_empty()).map(|r| r.good.clone()).collect();
        let refs: Vec<&str> = store.iter().map(|s| s.as_str()).collect();
        reasoner.study_reference(&refs);
    }

    // 2) Calibrate behavioral norms to THIS repository, so single-responsibility / complexity are
    //    judged against how this project actually writes code.
    let mut files = Vec::new();
    walk(&root, &mut files);
    let sources: Vec<(String, String)> = files
        .iter()
        .filter_map(|f| Some((ext_lang(f)?.to_string(), fs::read_to_string(f).ok()?)))
        .collect();
    let src_refs: Vec<(&str, &str)> = sources.iter().map(|(l, c)| (l.as_str(), c.as_str())).collect();
    reasoner.calibrate(&src_refs);

    // 3) Pull the language modules lazily as the repo needs them.
    let mut registry = ModuleRegistry::open("../lint-modules");
    ensure_rust_module(&mut registry);

    // 4) Read the whole repository and talk back in English.
    let report = review_repository(&root, &reasoner, &mut registry);
    println!("{}", report.to_english());
}
