//! Second modality: read the rule's English DESCRIPTION, not just its two code samples.
//!
//!   cargo run --release --example measure_modalities
//!
//! ~144 rules can't be grounded structurally — their bad and good examples parse to the same
//! tree (the distinction is a name, a type, a semantic property). But the description names the
//! construct: "Checks for usage of `transmute`", "the `as` operator", "`#[test]` in doctests".
//! So we mine each description's backtick code-spans for identifiers, keep the ones DISTINCTIVE
//! across rules (rare in other descriptions), and treat them as features the violating code must
//! contain — exactly like learning a word from its dictionary entry. Combined with the
//! structural signature, this is the entropy the syntax alone didn't carry.

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

/// Structural features of code (the labels + edges).
fn feats(code: &str) -> HashSet<String> {
    generic_features("rust", code).into_iter().map(|(f, _)| f).collect()
}

/// The "values" a piece of code exhibits: the head/leaf identity of each node (the part after
/// the last `:` of a non-edge feature). This is what a description token like `transmute` is
/// matched against.
fn values(code: &str) -> HashSet<String> {
    generic_features("rust", code)
        .into_iter()
        .filter_map(|(f, _)| (!f.contains('>')).then(|| f.rsplit(':').next().unwrap_or(&f).to_string()))
        .collect()
}

/// Identifiers inside backtick spans of a description: `transmute`, `Vec`, `as`, `#[test]`→test.
fn desc_tokens(desc: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut rest = desc;
    while let Some(i) = rest.find('`') {
        let after = &rest[i + 1..];
        let Some(j) = after.find('`') else { break };
        for tok in after[..j].split(|c: char| !c.is_alphanumeric() && c != '_') {
            if tok.len() >= 2 && tok.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_') {
                out.insert(tok.to_string());
            }
        }
        rest = &after[j + 1..];
    }
    out
}

struct Rule {
    id: String,
    bad: String,
    good: String,
    desc: HashSet<String>,
}

fn main() {
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
                desc: desc_tokens(r["description"].as_str().unwrap_or("")),
            })
        })
        .collect();

    // Study the language for structural rarity (as before).
    let mut files = Vec::new();
    rust_files(Path::new(".."), &mut files);
    files.retain(|p| !p.to_string_lossy().contains("measure_"));
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
    // Distinctiveness of a description token = how few rules mention it (rare across rules).
    let mut tok_df: HashMap<String, usize> = HashMap::new();
    for r in &rules {
        for t in &r.desc {
            *tok_df.entry(t.clone()).or_default() += 1;
        }
    }
    let held_loc: usize = held_out.iter().map(|s| s.lines().count()).sum();
    println!("studied {} files ({} held-out, {} LOC); {} rules\n", sources.len(), held_out.len(), held_loc, rules.len());

    // Build signatures two ways and compare: structure-only vs structure + description.
    let max_df = 1usize; // structural rarity
    let desc_max = 6usize; // a description token in <= this many rules is distinctive

    let struct_sig = |r: &Rule| -> HashSet<String> {
        let good = if r.good.is_empty() { HashSet::new() } else { feats(&r.good) };
        feats(&r.bad)
            .into_iter()
            .filter(|f| !good.contains(f) && df.get(f).copied().unwrap_or(0) <= max_df)
            .collect()
    };
    let desc_sig = |r: &Rule| -> HashSet<String> {
        r.desc
            .iter()
            .filter(|t| tok_df.get(*t).copied().unwrap_or(0) <= desc_max)
            .cloned()
            .collect()
    };

    for use_desc in [false, true] {
        // (id, structural features required, description values required)
        let sigs: Vec<(String, HashSet<String>, HashSet<String>)> = rules
            .iter()
            .filter_map(|r| {
                let s = struct_sig(r);
                let d = if use_desc { desc_sig(r) } else { HashSet::new() };
                // Grounded when structure alone is strong (>=2) OR a distinctive description
                // token backs it (so the non-syntactic rules become reachable).
                (s.len() >= 2 || (!d.is_empty() && !s.is_empty()) || (use_desc && s.is_empty() && d.len() >= 2))
                    .then(|| (r.id.clone(), s, d))
            })
            .collect();

        let fires = |fs: &HashSet<String>, vs: &HashSet<String>| -> Vec<&str> {
            sigs.iter()
                .filter(|(_, s, d)| s.iter().all(|f| fs.contains(f)) && d.iter().all(|t| vs.contains(t)))
                .map(|(id, _, _)| id.as_str())
                .collect()
        };

        let (mut answered, mut correct) = (0usize, 0usize);
        for r in &rules {
            let hits = fires(&feats(&r.bad), &values(&r.bad));
            if !hits.is_empty() {
                answered += 1;
                if hits.contains(&r.id.as_str()) {
                    correct += 1;
                }
            }
        }
        let mut ff = 0usize;
        for src in held_out {
            ff += fires(&feats(src), &values(src)).len();
        }
        let acc = if answered == 0 { 100.0 } else { correct as f64 / answered as f64 * 100.0 };
        println!(
            "{:18} grounded {:3} | answered {answered} | accuracy {correct}/{answered} ({acc:.1}%) | held-out {ff} flags ({:.2}/100 LOC)",
            if use_desc { "structure+desc" } else { "structure-only" },
            sigs.len(),
            ff as f64 / held_loc.max(1) as f64 * 100.0,
        );
    }
    println!("\nDescription tokens are matched against code the same way a word is learned from its");
    println!("entry: the rule names the construct, the construct must be present. Held-out 'flags' for");
    println!("description rules include REAL usages (e.g. unwrap/as/transmute) — those are true hits.");
}
