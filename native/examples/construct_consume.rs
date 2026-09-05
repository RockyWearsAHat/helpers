//! THROWAWAY MEASUREMENT HARNESS (untracked): COMPLETION PASS 23 rung 1 — the CONSUMER funnel. Runs the
//! REAL `lint_module::graduate` over python-library WITH the proven construction states fed in, and reports
//! the rung-1 funnel: the removal-only subjects (a whole-module removal construction, but no inline
//! deprecation class the existing faculty already catches) → PROPOSED as candidates → PROVEN into rules,
//! with a per-miss reason for every removal-only subject that does NOT graduate.
//!
//! Run: `cargo run --release --features crawl --example construct_consume`
use helpers_native::lint_attest::{attests_module_removal, Attestation};
use helpers_native::lint_char;
use helpers_native::lint_codec::{self, Dec};
use helpers_native::lint_construct::{attested_subjects, mine_and_prove};
use helpers_native::lint_english;
use helpers_native::lint_module;
use std::collections::{HashMap, HashSet};

fn decode_crawl(bytes: &[u8]) -> Option<Vec<(String, String)>> {
    let (_s, mut d) = Dec::open(bytes, lint_codec::kind::CRAWL)?;
    let _v = d.str()?;
    let _c = d.fixed_u64()?;
    let n = d.u()? as usize;
    let mut pages = Vec::with_capacity(n.min(65_536));
    for _ in 0..n {
        let url = d.str()?;
        let body = d.str()?;
        let _ = d.boolean()?;
        let _ = d.fixed_u64()?;
        let _ = d.fixed_u64()?;
        pages.push((url, body));
    }
    Some(pages)
}
fn load(name: &str) -> Vec<(String, String)> {
    let home = std::env::var("HOME").unwrap();
    let dir = format!("{home}/.cache/helpers/lint-index/crawls");
    std::fs::read(format!("{dir}/{name}.bin")).ok().and_then(|b| decode_crawl(&b)).unwrap_or_default()
}
fn code_interiors(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lb: Vec<u8> = body.bytes().map(|b| b.to_ascii_lowercase()).collect();
    let find = |from: usize, needle: &[u8]| -> Option<usize> {
        if from > lb.len() { return None; }
        lb[from..].windows(needle.len()).position(|w| w == needle).map(|p| from + p)
    };
    let mut i = 0;
    while let Some(open) = find(i, b"<code") {
        let Some(gt) = find(open, b">").map(|g| g + 1) else { break };
        let Some(close) = find(gt, b"</code>") else { break };
        if !body.is_char_boundary(gt) || !body.is_char_boundary(close) { i = open + 5; continue; }
        let raw = &body[gt..close];
        let mut txt = String::new();
        let mut intag = false;
        for ch in raw.chars() {
            match ch { '<' => intag = true, '>' => intag = false, _ if !intag => txt.push(ch), _ => {} }
        }
        let txt = txt.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&").replace("&quot;", "\"");
        if txt.trim().len() >= 2 { out.push(txt.trim().to_string()); }
        i = close + 7;
    }
    out
}
fn reconstruct_memory(pages: &[(String, String)]) -> helpers_native::lint_read::Memory {
    let mut mem = helpers_native::lint_read::Memory::default();
    let mut seen = HashSet::new();
    for (_u, body) in pages {
        for block in code_interiors(body) {
            if seen.insert(block.clone()) { mem.reference.push(block); }
        }
    }
    mem
}

fn main() {
    let (Some(br), Some(en)) = (lint_char::brain(), lint_english::brain()) else {
        eprintln!("no frozen brains on disk"); return;
    };
    let m = br.meanings();
    let pages = load("python-library");
    if pages.is_empty() { eprintln!("no python-library crawl"); return; }
    let mem = reconstruct_memory(&pages);

    let constructions = mine_and_prove(&pages);
    println!("PROVEN constructions: {}", constructions.len());
    for c in &constructions {
        println!("  witnesses={} :: {}", c.witnesses, c.shape);
    }

    // The removal-only set (removal construction page, NOT already attested) and their subjects.
    let attest = Attestation::discover(&pages);
    let mut removal_only: Vec<(String, Vec<String>)> = Vec::new();
    for (url, body) in &pages {
        if attests_module_removal(body) && !attest.attests(body) {
            let subs = attested_subjects(&constructions, url, body);
            if !subs.is_empty() {
                removal_only.push((url.clone(), subs));
            }
        }
    }
    let removal_subjects: HashSet<String> =
        removal_only.iter().flat_map(|(_, s)| s.iter().cloned()).collect();
    println!("\nremoval-only pages that BIND a proven construction: {}", removal_only.len());
    let mut names: Vec<String> = removal_subjects.iter().cloned().collect();
    names.sort();
    println!("removal-only subjects ({}): {}", names.len(), names.join(", "));

    // Run the REAL graduate WITH the constructions fed in (the shipped path).
    let attested_urls: HashSet<String> =
        pages.iter().filter(|(_, b)| attest.attests(b)).map(|(u, _)| u.clone()).collect();
    let (outcomes, _, _, _, _) = lint_module::graduate("python", pages.clone(), &mem, m, en, &constructions, &attested_urls);
    let by_construct: HashMap<&str, &lint_module::Outcome> =
        outcomes.iter().map(|o| (o.candidate.construct.as_str(), o)).collect();

    let proven_subjects: HashSet<String> = outcomes
        .iter()
        .filter(|o| o.rule.is_some())
        .map(|o| o.candidate.construct.clone())
        .collect();

    // The funnel over the removal-only subjects.
    let proposed: Vec<&String> = names.iter().filter(|n| by_construct.contains_key(n.as_str())).collect();
    let graduated: Vec<&String> = names.iter().filter(|n| proven_subjects.contains(*n)).collect();
    println!("\n── RUNG-1 FUNNEL (removal-only subjects) ──");
    println!("  candidates (bound):  {}", names.len());
    println!("  proposed:            {}", proposed.len());
    println!("  PROVEN → new rules:  {}", graduated.len());
    println!("\n  per-subject:");
    for n in &names {
        let g = proven_subjects.contains(n);
        let reason = match by_construct.get(n.as_str()) {
            Some(o) if g => format!("PROVEN (viol={}, clean={})", o.violating, o.clean),
            Some(o) => format!(
                "miss: not emitted (viol={}, clean={}, stated={}, attested_dep={}, verdict={:?})",
                o.violating, o.clean, o.candidate.stated, o.candidate.attested_deprecated, o.verdict
            ),
            None => "miss: not proposed (no candidate reached propose)".to_string(),
        };
        println!("    {n:16} {reason}");
    }

    // Whole-run summary: total python rules and how many are the new removal-only subjects.
    let total_rules = outcomes.iter().filter(|o| o.rule.is_some()).count();
    println!("\n  TOTAL python rules this run: {total_rules}  (removal-only new: {})", graduated.len());
    let mut all_proven: Vec<String> = proven_subjects.iter().cloned().collect();
    all_proven.sort();
    println!("  all proven subjects: {}", all_proven.join(", "));

    // REGRESSION: graduate with EMPTY constructions (the pre-consumer path) must produce the SAME rules
    // for every NON-removal-only subject — the consumer is purely additive, never perturbs the frozen set.
    let (base, _, _, _, _) = lint_module::graduate("python", pages.clone(), &mem, m, en, &[], &attested_urls);
    let base_rules: HashMap<String, (String, String, String)> = base
        .iter()
        .filter_map(|o| o.rule.as_ref().map(|(r, _)| (r.id.clone(), (r.bad.clone(), r.good.clone(), r.description.clone()))))
        .collect();
    let with_rules: HashMap<String, (String, String, String)> = outcomes
        .iter()
        .filter_map(|o| o.rule.as_ref().map(|(r, _)| (r.id.clone(), (r.bad.clone(), r.good.clone(), r.description.clone()))))
        .collect();
    let base_ids: HashSet<&String> = base_rules.keys().collect();
    let mut perturbed = 0usize;
    for (id, v) in &base_rules {
        match with_rules.get(id) {
            Some(w) if w == v => {}
            Some(_) => { perturbed += 1; println!("  PERTURBED (content changed): {id}"); }
            None => { perturbed += 1; println!("  LOST (was proven, now gone): {id}"); }
        }
    }
    let added: Vec<&String> = with_rules.keys().filter(|k| !base_ids.contains(*k)).collect();
    println!("\n── REGRESSION vs empty-constructions baseline ──");
    println!("  baseline rules: {}   with-consumer rules: {}", base_rules.len(), with_rules.len());
    println!("  perturbed/lost existing rules: {perturbed}   (MUST be 0)");
    println!("  net-new rule ids: {}", added.len());
}
