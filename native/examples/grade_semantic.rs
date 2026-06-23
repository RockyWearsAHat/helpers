//! Grade a codebase on CS principles by COMPREHENSION, in any language.
//!
//!   cargo run --release --example grade_semantic [root]
//!
//! It reads every source file (Rust/Python/JS/TS/Go) it can parse, learns the project's normal
//! distribution of behavior — how many things a function usually does, how complex it usually is
//! — then flags the functions that violate single-responsibility / complexity / error-handling
//! relative to that learned norm. No per-language rules, no fixed thresholds: it judges the code
//! by what the code does, which is the part a prebuilt syntactic linter can't reach.

use std::fs;
use std::path::{Path, PathBuf};

use helpers_native::lint_semantic::{functions, FnMetrics, Norms, Principle};

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

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "..".to_string()));
    let mut files = Vec::new();
    source_files(&root, &mut files);
    files.sort();

    // Read everything, keep per-file (lang, code) and per-function metrics with provenance.
    let mut sources: Vec<(String, String, PathBuf)> = Vec::new();
    for f in &files {
        if let (Some(lang), Ok(code)) = (ext_lang(f), fs::read_to_string(f)) {
            sources.push((lang.to_string(), code, f.clone()));
        }
    }
    let learn_refs: Vec<(&str, &str)> = sources.iter().map(|(l, c, _)| (l.as_str(), c.as_str())).collect();
    let norms = Norms::learn(&learn_refs);

    let mut by_lang: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let mut flagged: Vec<(FnMetrics, Vec<Principle>, PathBuf, &str)> = Vec::new();
    let mut total_fns = 0usize;
    for (lang, code, path) in &sources {
        *by_lang.entry(lang.as_str()).or_default() += 1;
        for m in functions(lang, code) {
            total_fns += 1;
            let v = norms.judge(&m);
            if !v.is_empty() {
                flagged.push((m, v, path.clone(), lang.as_str()));
            }
        }
    }

    println!("Graded {} files across {:?}", sources.len(), by_lang.keys().collect::<Vec<_>>());
    println!(
        "Learned norms from {} functions: single-responsibility > {} concerns, complexity > {}\n",
        norms.sampled, norms.responsibility_p90, norms.complexity_p90
    );

    // Worst offenders first, by responsibility then complexity.
    flagged.sort_by(|a, b| {
        (b.0.responsibility(), b.0.complexity()).cmp(&(a.0.responsibility(), a.0.complexity()))
    });
    println!("{} of {} functions violate a principle. Worst offenders:\n", flagged.len(), total_fns);
    for (m, v, path, _lang) in flagged.iter().take(15) {
        let rel = path.strip_prefix(&root).unwrap_or(path).display();
        let tags: Vec<&str> = v
            .iter()
            .map(|p| match p {
                Principle::SingleResponsibility => "single-responsibility",
                Principle::Complexity => "complexity",
                Principle::ErrorHandling => "error-handling",
                Principle::NamingMismatch => "naming-vs-behavior",
            })
            .collect();
        println!(
            "  {rel}:{}  fn {}  [{}]",
            m.line, m.name, tags.join(", ")
        );
        println!(
            "      does {} distinct things, {} branches, {} loops, depth {}, {} forced results",
            m.distinct_calls, m.branches, m.loops, m.depth, m.forced_results
        );
    }
    // The comprehension highlight: names that don't match behavior, judged against the
    // convention the project itself taught the model.
    let naming: Vec<_> = flagged.iter().filter(|(_, v, _, _)| v.contains(&Principle::NamingMismatch)).collect();
    println!("\n{} name-vs-behavior mismatches (the learned-convention check):", naming.len());
    for (m, _, path, _) in naming.iter().take(8) {
        let rel = path.strip_prefix(&root).unwrap_or(path).display();
        let why = if m.produces_value { "reads as a query but mutates" } else { "name implies a value it never returns" };
        println!("  {rel}:{}  fn {}  — {why}", m.line, m.name);
    }

    println!("\nLanguage-agnostic, learned from the project itself — judging what the code DOES.");
}
