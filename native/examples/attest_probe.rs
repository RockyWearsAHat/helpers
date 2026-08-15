//! THROWAWAY PROBE (untracked): de-risk the LEARNED page-role attestation faculty.
//! Enumerate invariant recurring text runs on the crawled MDN corpus (the SiteChrome atom), and for
//! each measure: support (distinct pages), distinct-subject count, overlap with the hand-attested 117,
//! and a covenant-clean prohibition-meaning score against the DISCOVERED negator cluster. Reports the
//! separation (or lack of it) between the deprecation-notecard run and chrome.
//! Run: `cargo run --release --features crawl --example attest_probe`
use helpers_native::lint_char;
use helpers_native::lint_codec::{self, Dec};
use helpers_native::lint_english;
use std::collections::{HashMap, HashSet};

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
        if let Some(p) = decode_bin(&b) { return p; }
    }
    if let Ok(b) = std::fs::read(format!("{dir}/{name}.json")) {
        if let Some(p) = decode_json(&b) { return p; }
    }
    Vec::new()
}
fn is_rule_page(url: &str) -> bool { url.to_lowercase().contains("/rules/") }
fn hand_attested(url: &str, body: &str) -> bool {
    !is_rule_page(url) && (body.contains("notecard deprecated") || body.contains("no longer recommended"))
}
// Whitespace-collapse; a run of >=2 words and >=6 chars is a comparison atom (SiteChrome::run_key rule).
fn norm_run(text: &str) -> Option<String> {
    let norm: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if norm.len() < 6 || norm.split(' ').count() < 2 { return None; }
    Some(norm)
}
fn runs_of(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let b = body.as_bytes();
    let (mut in_tag, mut start) = (false, 0usize);
    for i in 0..b.len() {
        match b[i] {
            b'<' => { if !in_tag { if let Some(r) = norm_run(&body[start..i]) { out.push(r); } } in_tag = true; }
            b'>' => { in_tag = false; start = i + 1; }
            _ => {}
        }
    }
    if let Some(r) = norm_run(&body[start..]) { out.push(r); }
    out
}
// page subject = last non-empty url path segment (varies per construct).
fn subject(url: &str) -> String {
    url.split('#').next().unwrap_or(url).trim_end_matches('/').rsplit('/').next().unwrap_or("").to_string()
}

fn main() {
    let eng = lint_english::brain().expect("english brain");
    let br = lint_char::brain();

    let mut pages = load("mdn-css");
    pages.extend(load("mdn-js"));
    pages.extend(load("mdn-html"));
    for n in ["developer-mozilla-org-4cbd1761", "developer-mozilla-org-572fe52b",
              "developer-mozilla-org-611ab56b", "developer-mozilla-org-5d48a8d0"] {
        pages.extend(load(n));
    }
    let mut seen = HashSet::new();
    pages.retain(|(u, _)| seen.insert(u.clone()));
    let n_pages = pages.len();
    let hand: HashSet<String> =
        pages.iter().filter(|(u, b)| hand_attested(u, b)).map(|(u, _)| u.clone()).collect();
    println!("corpus {n_pages} MDN pages, hand-attested 117 baseline = {}", hand.len());

    // Per run: distinct pages carrying it, distinct subjects, how many are hand-attested.
    let mut sup: HashMap<String, HashSet<String>> = HashMap::new();     // run -> page urls
    let mut subj: HashMap<String, HashSet<String>> = HashMap::new();    // run -> subjects
    for (url, body) in &pages {
        let mut on_page: HashSet<String> = HashSet::new();
        for r in runs_of(body) { on_page.insert(r); }
        for r in on_page {
            sup.entry(r.clone()).or_default().insert(url.clone());
            subj.entry(r).or_default().insert(subject(url));
        }
    }
    // Discovered-negator prohibition score for a run: max is_negation over its words (frozen judge).
    let neg_score = |run: &str| -> bool {
        run.split_whitespace().any(|w| {
            let t = w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
            !t.is_empty() && eng.is_negation(helpers_native::lint_ai::token_seed(&t))
        })
    };
    // char-brain closest distance of any run word to any discovered baseline prohibition concept.
    let anchors = ["obsolete", "forbidden", "avoid", "removed", "unsafe"];
    let sem_score = |run: &str| -> u32 {
        let Some(br) = br else { return 9999 };
        run.split_whitespace().filter_map(|w| {
            let t = w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
            if t.is_empty() { return None; }
            Some(anchors.iter().map(|a| br.related(&t, a)).min().unwrap_or(9999))
        }).min().unwrap_or(9999)
    };

    // Candidate attestation runs: invariant (support>=8), PARTIAL (not near-universal), subjects vary.
    let mut cands: Vec<(&String, usize, usize, usize)> = Vec::new(); // run, support, distinct-subj, hand-overlap
    for (run, pgs) in &sup {
        let s = pgs.len();
        if s < 8 { continue; }
        let ds = subj.get(run).map(|x| x.len()).unwrap_or(0);
        let overlap = pgs.iter().filter(|u| hand.contains(*u)).count();
        cands.push((run, s, ds, overlap));
    }
    // Show the runs whose pages MOST overlap the hand set (the notecard prose should be here).
    cands.sort_by(|a, b| b.3.cmp(&a.3).then(b.1.cmp(&a.1)));
    println!("\n== top-40 invariant runs by hand-117 overlap ==");
    println!("{:>5} {:>5} {:>5} {:>4} {:>6}  run", "sup", "dsubj", "olap", "neg", "sem");
    for (run, s, ds, ov) in cands.iter().take(40) {
        let short: String = run.chars().take(70).collect();
        println!("{s:>5} {ds:>5} {ov:>5} {:>4} {:>6}  {short}", neg_score(run), sem_score(run));
    }

    // If we take "invariant run whose pages ALL fall inside a small hand-like subset" — measure the
    // single best run's precision/recall as a page-level attester.
    println!("\n== best single-run attester (max F1 vs hand-117) ==");
    let mut best = (0.0f64, String::new(), 0usize, 0usize);
    for (run, s, _ds, ov) in &cands {
        if *ov == 0 { continue; }
        let prec = *ov as f64 / *s as f64;
        let rec = *ov as f64 / hand.len() as f64;
        let f1 = if prec + rec > 0.0 { 2.0 * prec * rec / (prec + rec) } else { 0.0 };
        if f1 > best.0 { best = (f1, (*run).clone(), *ov, *s); }
    }
    println!("  best F1={:.3} overlap={}/{} support={}  run={:?}", best.0, best.2, hand.len(), best.3, best.1);

    // ── definition-level negator signal (any DEFINITION word is a negation-operator) ──
    println!("\n== definition carries a negation-operator (public is_negation over def seeds) ==");
    let def_neg = |w: &str| -> (bool, usize) {
        match eng.definition_of(helpers_native::lint_ai::token_seed(w)) {
            Some(def) => (def.iter().any(|s| eng.is_negation(*s)), def.len()),
            None => (false, 0),
        }
    };
    for w in ["deprecated", "deprecate", "obsolete", "avoid", "cease", "discourage", "discouraged",
              "forbidden", "removed", "recommended", "aware", "guide", "search", "menu", "note"] {
        let (dn, dl) = def_neg(w);
        println!("  {w:12} def_carries_negator={:5}  def_len={dl}", dn);
    }

    // ── partial-invariant landscape: runs with support in [8, 900), by distinct-subject ratio ──
    println!("\n== partial-invariant runs (8<=sup<900), sorted by support desc, top 45 ==");
    println!("{:>5} {:>5} {:>5} {:>4} {:>6}  run", "sup", "dsubj", "olap", "neg", "sem");
    let mut partial: Vec<_> = cands.iter().filter(|(_, s, _, _)| *s < 900).collect();
    partial.sort_by(|a, b| b.1.cmp(&a.1));
    for (run, s, ds, ov) in partial.iter().take(45) {
        let short: String = run.chars().take(64).collect();
        println!("{s:>5} {ds:>5} {ov:>5} {:>4} {:>6}  {short}", neg_score(run), sem_score(run));
    }
}
