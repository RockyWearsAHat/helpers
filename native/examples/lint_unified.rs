//! The unified linter, end to end: ONE reasoning model + plug-and-play, self-packed modules.
//!
//!   cargo run --release --example lint_unified
//!
//! What this demonstrates, in order:
//!   1. The reasoner LEARNS the CS2420/CS3500 principles from a plain text document
//!      (`corpus/cs-principles.md`) — no hardcoded rules. Teaching it the off-by-one pattern
//!      there closes a blind spot the old pipeline had to abstain on.
//!   2. Modules are SELF-PACKED and stored: a Rust module is trained once from the crawled docs
//!      corpus and published to the `lint-modules/` store (the local stand-in for GitHub).
//!   3. The registry pulls modules LAZILY — only the languages the project uses are loaded, so a
//!      Python module sitting in the store costs nothing here.
//!   4. The reasoner composes floor + taught principles + behavioral norms + the loaded module
//!      into one verdict, and we score it against a known answer key (incl. 0 false positives).

use std::fs;
use std::path::{Path, PathBuf};

use helpers_native::linter::{Knowledge, LintModule, ModuleRegistry, Reasoner};

/// Map a file extension to a language tag (for calibrating norms on the project's own code).
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

fn source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(n, "target" | ".git" | "node_modules" | ".helpers" | "dist" | "build") {
                continue;
            }
            source_files(&p, out);
        } else if ext_lang(&p).is_some() {
            out.push(p);
        }
    }
}

/// Ensure a Rust module exists in the store, training+publishing it once if absent. This is the
/// "self-pack a module and store it for reuse" step — afterwards it is just pulled, never retrained.
fn ensure_rust_module(reg: &mut ModuleRegistry) {
    // Self-heal: present only if the artifact actually loads (a manifest row whose blob is gone —
    // e.g. a fresh clone where the store is gitignored — repacks instead of failing).
    if reg.load("rust-clippy").is_some() {
        return;
    }
    let corpus = PathBuf::from("../corpus/lint-corpus.jsonl");
    let Ok(knowledge) = Knowledge::from_corpus(&corpus) else {
        eprintln!("(no corpus to pack a module from — skipping module publish)");
        return;
    };
    let module = LintModule::pack("rust-clippy", "1.95.0", "corpus/lint-corpus.jsonl", "rust", &knowledge);
    println!("Packed module 'rust-clippy' ({} rules grounded) and published to the store.", module.rule_count());
    if let Err(e) = reg.publish(&module) {
        eprintln!("(could not publish module: {e})");
    }
}

fn main() {
    // Read the project's own sources first — they serve double duty: the rarity corpus that keeps
    // principle grounding precise, and the distribution the behavioral norms calibrate to.
    let mut files = Vec::new();
    source_files(Path::new(".."), &mut files);
    let sources: Vec<(String, String)> = files
        .iter()
        .filter_map(|f| Some((ext_lang(f)?.to_string(), fs::read_to_string(f).ok()?)))
        .collect();
    let src_refs: Vec<(&str, &str)> = sources.iter().map(|(l, c)| (l.as_str(), c.as_str())).collect();

    // ---- 1) The reasoner reads the CS principles from the document and SELF-VALIDATES ----
    let cs_doc = fs::read_to_string("../corpus/cs-principles.md").expect("read corpus/cs-principles.md");
    let mut reasoner = Reasoner::from_cs_principles("rust", &cs_doc);

    // It reads known-idiomatic reference code — the "good" forms the docs themselves publish (from
    // the crawled lint corpus) — and tests every candidate rule against all of it, keeping only
    // what genuinely separates a violation from normal idiomatic code, abstaining otherwise. This
    // is the "read it, try it, test it, thousands of times" step. The reference is trusted doc
    // code, NOT the project under review (using the project would let its own bugs hide a rule).
    let corpus_knowledge = Knowledge::from_corpus(&PathBuf::from("../corpus/lint-corpus.jsonl")).ok();
    let ref_store: Vec<String> = corpus_knowledge
        .as_ref()
        .map(|k| k.rules.iter().filter(|r| r.language == "rust" && !r.good.is_empty()).map(|r| r.good.clone()).collect())
        .unwrap_or_default();
    let rust_ref: Vec<&str> = ref_store.iter().map(|s| s.as_str()).collect();
    reasoner.study_reference(&rust_ref);
    let (grounded, total, failures) = reasoner.self_test();
    println!(
        "Reasoner read {total} principle(s) from corpus/cs-principles.md, tested them {} times against\n  {} idiomatic reference snippets, and grounded {grounded} (abstained on {}); self-test failures: {failures}.",
        reasoner.fit_tests(),
        rust_ref.len(),
        total - grounded
    );

    // Calibrate the behavioral norms on the project's own code (tailors the bar to the project).
    reasoner.calibrate(&src_refs);
    println!("Calibrated behavioral norms on {} project source files.\n", sources.len());

    // ---- 2) + 3) The registry: self-packed modules, pulled lazily ----
    let mut reg = ModuleRegistry::open("../lint-modules");
    ensure_rust_module(&mut reg);
    println!("Store advertises: {:?}", reg.available().iter().map(|e| &e.id).collect::<Vec<_>>());

    let project_langs = vec!["rust".to_string()];
    let needed = reg.select(&project_langs);
    println!("Project is {project_langs:?} ⇒ modules needed: {needed:?} (python module, if any, stays unloaded).");
    let mut loaded: Vec<LintModule> = Vec::new();
    for id in &needed {
        if let Some(m) = reg.load(id) {
            loaded.push(m.clone());
        }
    }
    let module_refs: Vec<&LintModule> = loaded.iter().collect();

    // ---- 4) Review a planted file with a known answer key ----
    let code = r#"// 1
fn parse_port(s: &str) -> u16 {         // 2
    s.parse().unwrap()                  // 3: unwrap_used (floor) + error-handling (norm)
}

fn flag_on(enabled: bool) -> bool {     // 6
    if enabled == true {                // 7: bool_comparison (floor)
        panic!("nope");                 // 8: panic (floor)
    }
    return enabled;                     // 10: needless_return (floor)
}

fn sum(xs: &[i32]) -> i32 {             // 13
    let mut total = 0;
    for i in 0..=xs.len() {             // 15: OFF-BY-ONE -> taught CS principle
        total += xs[i];
    }
    total
}

fn describe(items: &[i32]) -> &str {    // 21
    if items.len() == 0 { "empty" } else { "full" } // 22: idiomatic emptiness -> CS principle
}

fn god(xs: &[i32]) -> i32 {             // 25: complexity (norm)
    let mut t = 0;
    for x in xs {
        if *x > 0 {
            if *x > 10 { t += a1(*x); } else { t += a2(*x); }
        } else if *x < 0 {
            while t > 0 { t = b1(t); if t == 5 { break; } }
        } else {
            t += c1(*x); t += c2(*x); t += c3(*x);
        }
    }
    t
}

// Clean code — MUST NOT be flagged:
fn add(a: i32, b: i32) -> i32 { a + b } // 40
fn dice() -> u32 { let mut n = 0; for r in 1..=6 { n += r; } n } // 41: LEGIT inclusive range (FP probe)
"#;

    let findings = reasoner.review("rust", code, &module_refs);

    println!("\n=== COMPOSED VERDICT (reasoner + {} module) ===", needed.len());
    for f in &findings {
        let msg = if f.message.is_empty() { String::new() } else { format!("  — {}", f.message) };
        println!("  L{:<3} [{:<11}] {:<22} ({}){}", f.line, f.source, f.rule_id, f.severity, msg);
    }

    // ---- Score against the answer key ----
    struct Expect {
        line: usize,
        rule: &'static str,
        what: &'static str,
    }
    let key = [
        Expect { line: 3, rule: "unwrap_used", what: "unwrap (floor)" },
        Expect { line: 7, rule: "bool_comparison", what: "== true (floor)" },
        Expect { line: 8, rule: "panic", what: "panic! (floor)" },
        Expect { line: 10, rule: "needless_return", what: "needless return (floor)" },
        Expect { line: 15, rule: "off_by_one_indexing", what: "off-by-one (TAUGHT principle)" },
        Expect { line: 22, rule: "idiomatic_emptiness_check", what: "len()==0 (TAUGHT principle)" },
        Expect { line: 25, rule: "complexity", what: "god fn complexity (norm)" },
    ];
    println!("\n=== SCORECARD vs answer key ===");
    let (mut caught, mut missed) = (0, 0);
    for e in &key {
        let hit = findings.iter().any(|f| f.line == e.line && f.rule_id == e.rule);
        if hit {
            caught += 1;
        } else {
            missed += 1;
        }
        println!("  L{:<3} {:<34} {}", e.line, e.what, if hit { "CAUGHT" } else { "MISSED" });
    }
    // Honest FP probe: L40 is plain clean code; L41 is a LEGITIMATE inclusive range (1..=6) that
    // the off-by-one pattern could over-flag, since the taught contrast only distinguishes `..=`.
    let fp: Vec<&_> = findings.iter().filter(|f| f.line == 40 || f.line == 41).collect();
    println!("\n{caught}/{} caught, {missed} missed.  Flags on clean code (L40 plain, L41 legit `1..=6`): {} {}", key.len(), fp.len(), if fp.is_empty() { "✓ none" } else { "← see below" });
    for f in &fp {
        println!("  L{} {} [{}] — unexpected flag on clean code", f.line, f.rule_id, f.source);
    }

    // ---- 5) Incremental, non-lossy update: a new rule starts working immediately ----
    let before = reasoner.principle_count();
    reasoner.learn(
        r#"
# Double negation [medium]
Don't write a redundant boolean — a double negation says nothing the plain condition doesn't.
```rust:bad
fn ready(c: bool) -> bool { if !!c { true } else { false } }
```
```rust:good
fn ready(c: bool) -> bool { c }
```
"#,
    );
    reasoner.study_reference(&rust_ref); // re-test the new rule against the same idiomatic reference
    let probe = "fn ok(flag: bool) -> bool { if !!flag { true } else { false } }";
    let new_hit = reasoner.review("rust", probe, &[]).iter().any(|f| f.rule_id == "double_negation");
    let old_still = reasoner
        .review("rust", "fn s(xs: &[i32]) -> i32 { let mut t = 0; for i in 0..=xs.len() { t += xs[i]; } t }", &[])
        .iter()
        .any(|f| f.rule_id == "off_by_one_indexing");
    println!("\n=== INCREMENTAL UPDATE (no retrain) ===");
    println!("  Learned a new principle at runtime: {before} → {} grounded.", reasoner.principle_count());
    println!("  New rule fires immediately: {new_hit}.  Previously-learned off-by-one still fires: {old_still} (non-lossy).");

    println!("\nMinimal input — a documented bad/good pair — read and tested thousands of times against");
    println!("idiomatic reference, grounded precisely or abstained. Add a rule to the doc, it works at once.");
}
