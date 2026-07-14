//! PASS 22 PRODUCTION VALIDATION — the proven construction mechanism, now via the LIBRARY code path
//! (`helpers_native::lint_construct::mine_and_prove`), reproduces the PASS-21 measurement over the real
//! corpus. Two runs: (1) python-library ALONE (exactly what the train path mines for python), (2) the
//! full deduped cross-corpus set (the construct_p21 basis) — to confirm the tight removal faculty admits
//! ONLY the python-removal construction and nothing else co-proves.
//!
//! Run: `cargo run --release --features crawl --example construct_prod`
use helpers_native::lint_attest::{attests_module_removal, prohibition_class_tokens, Attestation};
use helpers_native::lint_construct::{mine_and_prove, ConstructionKind};
use helpers_native::lint_codec::{self, Dec};
use std::collections::HashSet;

fn decode_bin(bytes: &[u8]) -> Option<Vec<(String, String)>> {
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
fn decode_json(bytes: &[u8]) -> Option<Vec<(String, String)>> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let arr = v.get("pages").or(Some(&v))?.as_array()?;
    let mut out = Vec::new();
    for p in arr {
        let url = p.get("url")?.as_str()?.to_string();
        let body = p.get("body").or_else(|| p.get("html"))?.as_str()?.to_string();
        out.push((url, body));
    }
    Some(out)
}
fn load(name: &str) -> Vec<(String, String)> {
    let home = std::env::var("HOME").unwrap();
    let dir = format!("{home}/.cache/helpers/lint-index/crawls");
    if let Ok(b) = std::fs::read(format!("{dir}/{name}.bin")) {
        if let Some(p) = decode_bin(&b) {
            return p;
        }
    }
    if let Ok(b) = std::fs::read(format!("{dir}/{name}.json")) {
        if let Some(p) = decode_json(&b) {
            return p;
        }
    }
    Vec::new()
}

fn report(label: &str, pages: &[(String, String)]) {
    println!("\n══ {label}: {} pages ══", pages.len());
    let proven = mine_and_prove(pages);
    println!("  PROVEN construction states: {}", proven.len());
    for c in &proven {
        let kind = match c.kind {
            ConstructionKind::Removal => "REMOVAL",
            ConstructionKind::Prohibition => "PROHIBITION",
            ConstructionKind::Supersession => "SUPERSESSION",
        };
        println!("  [{kind}] witnesses={} :: {}", c.witnesses, c.shape);
        println!("        e.g. {}", c.example.chars().take(100).collect::<String>());
    }
    if proven.is_empty() {
        println!("  (none proven — held mined-unproven)");
    }
}

/// The RUNG-3 DELTA: of the whole-module removal pages (the removal-construction subjects), how many are
/// ALREADY attested by the existing faculty (so their subject already graduates today) vs GENUINELY NEW
/// (covered ONLY by the removal construction). Answers "new rules vs already-covered" honestly.
fn removal_delta(pages: &[(String, String)]) {
    let attest = Attestation::discover(pages);
    let _tokens = prohibition_class_tokens();
    let mut removal_pages = 0usize;
    let mut already = 0usize;
    let mut new = 0usize;
    let mut new_subjects: Vec<String> = Vec::new();
    for (url, body) in pages {
        if !attests_module_removal(body) {
            continue;
        }
        removal_pages += 1;
        // Does the page ALREADY attest via the existing page-role faculty (text-run OR class-badge route)?
        if attest.attests(body) {
            already += 1;
        } else {
            new += 1;
            let subj = url.rsplit('/').next().unwrap_or("").trim_end_matches(".html").to_string();
            new_subjects.push(subj);
        }
    }
    println!("\n── RUNG-3 DELTA (removal construction vs existing attestation) ──");
    println!("  whole-module removal pages:            {removal_pages}");
    println!("  already covered by existing attests(): {already}");
    println!("  GENUINELY NEW (removal-only):          {new}");
    new_subjects.sort();
    if !new_subjects.is_empty() {
        println!("  new subjects: {}", new_subjects.join(", "));
    }
}

fn main() {
    // (1) python-library alone — the per-language train-path mining.
    let py = load("python-library");
    report("python-library ALONE (train-path mining)", &py);
    removal_delta(&py);

    // (2) the full deduped cross-corpus set — the construct_p21 basis.
    let corpora = [
        "mdn-js",
        "mdn-css",
        "developer-mozilla-org-572fe52b",
        "developer-mozilla-org-4cbd1761",
        "developer-mozilla-org-611ab56b",
        "w3schools-js",
        "w3schools-css",
        "w3schools-com-3ab5de42",
        "rust-std",
        "python-library",
    ];
    let mut pages: Vec<(String, String)> = Vec::new();
    let mut seen = HashSet::new();
    for name in corpora {
        for (u, b) in load(name) {
            if seen.insert(u.clone()) {
                pages.push((u, b));
            }
        }
    }
    report("full deduped cross-corpus (construct_p21 basis)", &pages);
}
