//! THROWAWAY PROBE (untracked): COMPLETION PASS 17 rung 1 — TRAIN THE REGISTER.
//! Question: does the existing side-count `Polarity` (lint_read.rs), trained on the system's OWN
//! machine-derived labels — the 117 deprecation-attested MDN banner sentences (BAD register) vs the
//! ~1284 Baseline availability banners (GOOD/non-prohibition register), with the rest of the corpus as
//! neutral EXPOSURE — GENERALIZE past the banners to novel prose the passes-11/12/16 wall blocked:
//!   (b) prose-command register ("Never use direct eval()!", "…recommend avoiding var", "Use ===") -> BAD
//!   (c) abstention foils (docs-v83 junk governing prose, neutral reference, the endorsement banners) -> NOT bad
//!   (d) pass-14 note-scope sentences ("except…", "if loop is not specified") -> report
//! NO word lists: the vocabulary weights are LEARNED from the page families; the labels are the machine's
//! own attestation/availability page-role facts. Run: `cargo run --release --features crawl --example register_train`
use helpers_native::lint_codec::{self, Dec};
use helpers_native::lint_read::Polarity;
use std::collections::{HashMap, HashSet};

// ── corpus load (same MDN crawl bins metajoin reads) ────────────────────────────
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
// the 117-page ground truth (retired hand marker) — the negative register family.
fn hand_attested(url: &str, body: &str) -> bool {
    !is_rule_page(url) && (body.contains("notecard deprecated") || body.contains("no longer recommended"))
}
fn norm_run(text: &str) -> Option<String> {
    let norm: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if norm.len() < 6 || norm.split(' ').count() < 2 { return None; }
    Some(norm)
}
// invariant text runs = whitespace-collapsed text between tags (a rendered banner is such a run).
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
fn slug_of_url(url: &str) -> Option<String> {
    let u = url.split('#').next().unwrap_or(url);
    let i = u.find("/docs/")?;
    Some(u[i + 6..].trim_end_matches('/').to_lowercase())
}

fn main() {
    let mut pages = load("mdn-css");
    pages.extend(load("mdn-js"));
    pages.extend(load("mdn-html"));
    for n in ["developer-mozilla-org-4cbd1761", "developer-mozilla-org-572fe52b",
              "developer-mozilla-org-611ab56b", "developer-mozilla-org-5d48a8d0"] {
        pages.extend(load(n));
    }
    let mut seen = HashSet::new();
    pages.retain(|(u, _)| seen.insert(u.clone()));
    println!("corpus: {} pages", pages.len());

    // ── run support across the corpus ──
    let mut sup: HashMap<String, HashSet<String>> = HashMap::new();
    for (url, body) in &pages {
        for r in runs_of(body).into_iter().collect::<HashSet<_>>() {
            sup.entry(r).or_default().insert(url.clone());
        }
    }
    let hand: HashSet<String> =
        pages.iter().filter(|(u, b)| hand_attested(u, b)).map(|(u, _)| u.clone()).collect();
    println!("negative-register family (deprecation-attested): {} pages", hand.len());

    // ── BAD runs: sentence-scale (>=6w) invariant runs family-dominant on the 117, absent off it ──
    let mut bad_runs: Vec<String> = Vec::new();
    for (run, pgs) in &sup {
        if run.split(' ').count() < 6 { continue; }
        let on_fam = pgs.iter().filter(|u| hand.contains(*u)).count();
        let off_fam = pgs.len() - on_fam;
        if on_fam >= hand.len() / 2 && off_fam == 0 { bad_runs.push(run.clone()); }
    }
    bad_runs.sort_by_key(|r| std::cmp::Reverse(sup[r].len()));
    println!("\n== BAD register runs (deprecation banners, family-dominant, off-family=0) ==");
    for r in &bad_runs {
        println!("  sup={:>4}  {:?}", sup[r].len(), r.chars().take(74).collect::<String>());
    }

    // ── GOOD runs: the Baseline AVAILABILITY banners. Discovered structurally: sentence-scale
    //    (>=6w) invariant runs of high support that are ABSENT from the negative family (a banner
    //    the author renders across many subjects, never on a deprecated page). Section chrome
    //    ("Formal syntax") is <6 words and drops out; the two Baseline banners dominate by support. ──
    // PROSE (not code): a rendered banner is words — most characters are letters or spaces. The
    // localStorage widget snippet (support 2247) is code (brackets, quotes, `=`) and drops out.
    let is_prose = |r: &str| {
        let n = r.chars().count().max(1);
        let alpha = r.chars().filter(|c| c.is_alphabetic() || c.is_whitespace()).count();
        alpha as f64 / n as f64 >= 0.90
    };
    let mut good_cand: Vec<(String, usize)> = sup.iter()
        .filter(|(run, pgs)| run.split(' ').count() >= 6
            && is_prose(run)
            && pgs.iter().all(|u| !hand.contains(u)))
        .map(|(run, pgs)| (run.clone(), pgs.len()))
        .collect();
    good_cand.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    println!("\n== top sentence-scale PROSE non-family invariant runs (Baseline banners emerge by support) ==");
    for (run, n) in good_cand.iter().take(12) {
        println!("  sup={:>4}  {:?}", n, run.chars().take(74).collect::<String>());
    }
    // The GOOD/non-prohibition training set = the dominant availability banners (support clears a
    // natural gap above ordinary chrome). Take the prose runs above a support floor — this captures
    // BOTH Baseline banners including "…is not Baseline because it does not work…" (the descriptive
    // negation the good side must own so "not"/"work" stay neutral, not prohibition).
    let good_floor = 400usize;
    let good_runs: Vec<String> =
        good_cand.iter().filter(|(_, n)| *n >= good_floor).map(|(r, _)| r.clone()).collect();
    println!("GOOD register runs (taken, support>={good_floor}): {}", good_runs.len());

    // ── build the training set: per PAGE, its banner run text carries the page-role label; every
    //    OTHER sentence-scale run on the page is neutral EXPOSURE (the side-count denominator). ──
    // Held-out split by URL hash (80/20). Banners are invariant so held-out banner accuracy mostly
    // confirms training took; the FOILS are the real generalization test.
    let bad_set: HashSet<&String> = bad_runs.iter().collect();
    let good_set: HashSet<&String> = good_runs.iter().collect();
    let holdout = |u: &str| helpers_native::lint_ai::token_seed(u) % 5 == 0; // ~20%
    // The LABEL is the banner FACT (one prohibition sentence / one availability sentence), so each
    // DISTINCT run is one training example — class-balanced at the run level, independent of how many
    // pages render it (807 Baseline pages vs 117 deprecation pages would otherwise poison shared
    // nouns like "feature" toward the larger family). EXPOSURE stays per-page (real word frequency,
    // the neutral denominator). Held-out banners come from the ~20% held-out pages.
    let mut train: Vec<(String, bool)> = Vec::new();
    let mut exposure: Vec<String> = Vec::new();
    let mut test: Vec<(String, bool)> = Vec::new();
    let mut labeled = HashSet::new();
    for r in &bad_runs { if labeled.insert(r.clone()) { train.push((r.clone(), true)); } }
    for r in &good_runs { if labeled.insert(r.clone()) { train.push((r.clone(), false)); } }
    for (url, body) in &pages {
        let held = holdout(url);
        for r in runs_of(body).into_iter().collect::<HashSet<_>>() {
            if bad_set.contains(&r) { if held { test.push((r.clone(), true)); } }
            else if good_set.contains(&r) { if held { test.push((r.clone(), false)); } }
            else { exposure.push(r); } // every non-label run is neutral denominator
        }
    }
    println!("\ntrain: {} bad + {} good labels, {} exposure runs; held-out: {} banner sentences",
        train.iter().filter(|(_, b)| *b).count(),
        train.iter().filter(|(_, b)| !*b).count(),
        exposure.len(), test.len());

    // Build via the PolarityBuilder path (accumulate labels + tally_exposure), exactly as the
    // grounded crawl builds it — from_labeled has no exposure hook, so use the builder directly.
    let mut reader = helpers_native::lint_read::Reader::new();
    for (p, _) in &train { reader.learn_span(p); }
    for p in &exposure { reader.learn_span(p); }
    let mut b = helpers_native::lint_read::PolarityBuilder::new(reader);
    for (p, is_bad) in &train { b.accumulate(p, *is_bad); }
    for p in &exposure { b.tally_exposure(p); }
    let pol: Polarity = b.build();
    println!("classifier: {} votes, ready={}", pol.votes(), pol.is_ready());

    // ── (a) held-out banner accuracy ──
    let mut hit = 0; let mut tot = 0; let mut abst = 0;
    let mut seen_test = HashSet::new();
    for (p, want_bad) in &test {
        if !seen_test.insert(p.clone()) { continue; } // dedup identical banner text
        tot += 1;
        match pol.classify(p) {
            Some(v) if v == *want_bad => hit += 1,
            None => abst += 1,
            _ => {}
        }
    }
    println!("\n(a) HELD-OUT banners: {}/{} correct ({} abstain) — banners invariant, confirms training",
        hit, tot, abst);

    // per-token learned lean sanity: the register words
    println!("\n    learned token leans (Some(true)=bad, Some(false)=good, None=abstain):");
    for w in ["deprecated", "recommended", "avoid", "obsolete", "cease", "established",
              "widely", "available", "works", "not", "the", "feature", "use", "eval", "var"] {
        let (f, g, e) = pol.tally_of(w);
        println!("      {w:14} lean={:?}  tally(bad={f},good={g},exp={e})", pol.token_lean(w));
    }

    // ── foil legs ── report BOTH the full classify (tallies then prototype fallback) and the
    // tally-ONLY read (classify_tallied: decisive learned register words only, abstain otherwise).
    let show = |label: &str, prose: &str| {
        println!("      {label:9} {:?} -> full={:?} tally-only={:?}",
            prose.chars().take(50).collect::<String>(), pol.classify(prose), pol.classify_tallied(prose));
    };
    println!("\n(b) PROSE-COMMAND register (want BAD / Some(true)):");
    for s in ["Never use direct eval()!",
              "Modern JavaScript standards recommend avoiding var entirely",
              "Use === Comparison",
              "The == operator is not recommended; use === instead",
              "This method is deprecated and should be avoided",
              "It is no longer recommended to use this feature"] {
        show("cmd", s);
    }

    println!("\n(c) ABSTENTION FOILS (want NOT bad — Some(false) or None):");
    // the endorsement banners themselves
    for r in &good_runs { show("baseline", r); }
    // junk-page + neutral reference governing prose pulled from the corpus by slug
    let junk_slugs = ["web/javascript/reference/global_objects/function",
                      "web/javascript/reference/operators/this",
                      "web/javascript/reference/global_objects/boolean",
                      "web/javascript/reference/global_objects/string",
                      "web/css/direction", "web/css/clear"];
    let by_slug: HashMap<String, &String> = pages.iter()
        .filter_map(|(u, b)| slug_of_url(u).map(|s| (s, b))).collect();
    for slug in junk_slugs {
        if let Some(body) = by_slug.get(slug) {
            // the page's OWN description = the first sentence-scale run that is page-specific
            // (low corpus support — banners/chrome are high-support and skipped).
            if let Some(sent) = first_prose_sentence(body, &sup) {
                show("neutral", &sent);
            }
        }
    }

    println!("\n(d) NOTE-SCOPE sentences (report only):");
    for s in ["except when the loop variable is used after the loop",
              "if loop is not specified, the default applies",
              "This does not work on WebAssembly targets"] {
        show("note", s);
    }

    // ── FLOOD MEASUREMENT: how many NON-attested reference pages would the register OPEN as
    //    prohibition-bearing (the read_doc_page gate), and do the classics (var/eval) surface? A
    //    reference page's LEAD governing prose ~= its first page-specific sentence-scale prose runs.
    //    This is the wiring-risk number: opening thousands perturbs the order-sensitive self-test. ──
    let subj = |u: &str| u.split('#').next().unwrap_or(u).trim_end_matches('/').rsplit('/').next().unwrap_or("").to_lowercase();
    let lead_runs = |body: &str| -> Vec<String> {
        runs_of(body).into_iter()
            .filter(|r| r.split(' ').count() >= 8 && is_prose(r)
                && sup.get(r).map(|s| s.len()).unwrap_or(1) < 5) // page-specific, not a banner
            .take(4).collect()
    };
    // A TRAINED-bad word = decisive learned bad lean from the LABELS alone (f >= 8 bits, f >= 2*(g+e))
    // — excludes the frozen is_negation short-circuit ("not" is trained-neutral here). This isolates
    // the register's OWN signal from the passes-12 "not"-floods-everything effect.
    let trained_bad = |w: &str| { let (f, g, e) = pol.tally_of(w); f >= 8 && f >= (g + e).saturating_mul(2) };
    let run_has_trained_bad = |r: &str| r.split(|c: char| !c.is_alphanumeric()).any(|w| !w.is_empty() && trained_bad(&w.to_lowercase()));
    let (mut open_tally, mut open_full, mut open_trained) = (0usize, 0usize, 0usize);
    let mut trained_subjects: Vec<String> = Vec::new();
    let mut tally_subjects: Vec<String> = Vec::new();
    let classics = ["var", "eval", "with", "equality", "strict_equality", "document.write", "escape"];
    let mut classic_hits: Vec<(String, Option<bool>, Option<bool>)> = Vec::new();
    for (url, body) in &pages {
        if hand.contains(url) || is_rule_page(url) { continue; } // attested/rule already open
        let leads = lead_runs(body);
        let fires_tally = leads.iter().any(|r| pol.classify_tallied(r) == Some(true));
        let fires_full = leads.iter().any(|r| pol.classify(r) == Some(true));
        // trained-bad scan over the WHOLE page's page-specific prose (not just the lead) — the
        // genuine "this page's own prose recommends-against / avoids / deprecates a construct" set.
        let all_prose: Vec<String> = runs_of(body).into_iter()
            .filter(|r| is_prose(r) && sup.get(r).map(|s| s.len()).unwrap_or(1) < 20).collect();
        let fires_trained = all_prose.iter().any(|r| run_has_trained_bad(r));
        if fires_tally { open_tally += 1; if tally_subjects.len() < 40 { tally_subjects.push(subj(url)); } }
        if fires_full { open_full += 1; }
        if fires_trained { open_trained += 1; if trained_subjects.len() < 40 { trained_subjects.push(subj(url)); } }
        let s = subj(url);
        if classics.contains(&s.as_str()) {
            let ft = leads.iter().filter_map(|r| pol.classify_tallied(r)).find(|v| *v);
            let ff = leads.iter().filter_map(|r| pol.classify(r)).find(|v| *v);
            classic_hits.push((s, ft, ff));
        }
    }
    let total_ref = pages.iter().filter(|(u, _)| !hand.contains(u) && !is_rule_page(u)).count();
    println!("\n== FLOOD: non-attested reference pages the register would OPEN ==");
    println!("  total non-attested pages: {total_ref}");
    println!("  opened by TALLY-ONLY register: {open_tally}  ({:.1}%)", 100.0 * open_tally as f64 / total_ref as f64);
    println!("  opened by FULL register:       {open_full}  ({:.1}%)", 100.0 * open_full as f64 / total_ref as f64);
    println!("  opened by TRAINED-BAD-word (no frozen 'not'): {open_trained}  ({:.1}%)", 100.0 * open_trained as f64 / total_ref as f64);
    println!("  sample TRAINED-opened subjects: {:?}", &trained_subjects[..trained_subjects.len().min(30)]);
    println!("  sample tally-opened subjects: {:?}", &tally_subjects[..tally_subjects.len().min(30)]);
    println!("\n== CLASSICS on their own reference pages (subject -> tally-open, full-open) ==");
    for (s, ft, ff) in &classic_hits {
        println!("  {s:18} tally={ft:?} full={ff:?}");
    }
}

// the page's OWN first descriptive sentence: the first sentence-scale run that is PAGE-SPECIFIC
// (corpus support < 5 — banners/section chrome recur site-wide and are skipped), >= 12 words, a
// period, no code braces.
fn first_prose_sentence(body: &str, sup: &HashMap<String, HashSet<String>>) -> Option<String> {
    for r in runs_of(body) {
        let words = r.split(' ').count();
        let page_specific = sup.get(&r).map(|s| s.len()).unwrap_or(1) < 5;
        if page_specific && words >= 12 && r.contains('.') && !r.contains('{') {
            return Some(r.split('.').next().unwrap_or(&r).to_string());
        }
    }
    None
}
