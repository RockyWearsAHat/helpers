//! THROWAWAY PROBE (untracked): de-risk COMPLETION PASS 13 — attestation via AUTHOR METADATA TYPOGRAPHY.
//! Question: does the markdown frontmatter `status:` enum (a data-keyed author marker), JOINED to the
//! crawled HTML pages by slug, uniquely + purely LABEL the structural 117-cluster the SiteChrome atom
//! already discovers? If the deprecated-metadata family points at exactly the notecard run and near-zero
//! elsewhere, the metadata LABEL crosses the wall passes 11/12 hit (no English polarity needed).
//! Run: `cargo run --release --features crawl --example metajoin`
use helpers_native::lint_codec::{self, Dec};
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
// slug = url path after "/docs/", case-folded, trimmed of trailing slash + fragment. Join key both sides.
fn slug_of_url(url: &str) -> Option<String> {
    let u = url.split('#').next().unwrap_or(url);
    let i = u.find("/docs/")?;
    let s = &u[i + 6..];
    Some(s.trim_end_matches('/').to_lowercase())
}
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
fn subject(url: &str) -> String {
    url.split('#').next().unwrap_or(url).trim_end_matches('/').rsplit('/').next().unwrap_or("").to_string()
}

// ── markdown frontmatter: (slug, status-enum-values) per *.md, discovered by shape ──
// Parse the leading `---`…`---` block; find a key whose value is a YAML block-sequence (lines `  - v`).
// That is the enum shape; `status` is discovered as such (small closed value set), never hardcoded here.
fn md_frontmatter(text: &str) -> Option<(String, HashMap<String, Vec<String>>)> {
    let t = text.strip_prefix("---")?;
    let end = t.find("\n---")?;
    let fm = &t[..end];
    let mut slug = None;
    let mut enums: HashMap<String, Vec<String>> = HashMap::new();
    let mut cur_key: Option<String> = None;
    for line in fm.lines() {
        if let Some(rest) = line.strip_prefix("  - ") {
            // continuation of a block-sequence under cur_key
            if let Some(k) = &cur_key {
                enums.entry(k.clone()).or_default().push(rest.trim().trim_matches('"').to_lowercase());
            }
            continue;
        }
        if line.starts_with(' ') { continue; }
        // top-level `key: value` or `key:`
        if let Some(colon) = line.find(':') {
            let key = line[..colon].trim().to_lowercase();
            let val = line[colon + 1..].trim().trim_matches('"');
            if key == "slug" { slug = Some(val.to_lowercase()); }
            cur_key = if val.is_empty() { Some(key) } else { None };
        } else {
            cur_key = None;
        }
    }
    Some((slug?, enums))
}
fn md_corpus() -> Vec<(String, HashMap<String, Vec<String>>)> {
    let home = std::env::var("HOME").unwrap();
    let root = format!("{home}/.cache/helpers/mdn-content/files");
    let mut out = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(&root)];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() { stack.push(p); continue; }
            if p.extension().and_then(|x| x.to_str()) != Some("md") { continue; }
            if let Ok(txt) = std::fs::read_to_string(&p) {
                if let Some(fm) = md_frontmatter(&txt) { out.push(fm); }
            }
        }
    }
    out
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
    let n_pages = pages.len();

    // url -> slug; slug -> url (for join). hand-117 set (url).
    let mut slug_url: HashMap<String, String> = HashMap::new();
    let mut url_slug: HashMap<String, String> = HashMap::new();
    for (u, _) in &pages {
        if let Some(s) = slug_of_url(u) { slug_url.entry(s.clone()).or_insert_with(|| u.clone()); url_slug.insert(u.clone(), s); }
    }
    let hand: HashSet<String> = pages.iter().filter(|(u, b)| hand_attested(u, b)).map(|(u, _)| u.clone()).collect();
    let hand_slugs: HashSet<String> = hand.iter().filter_map(|u| url_slug.get(u).cloned()).collect();
    println!("crawl {n_pages} pages, {} distinct slugs; hand-attested {} pages / {} slugs",
             slug_url.len(), hand.len(), hand_slugs.len());

    // ── markdown status enum discovery ──
    let md = md_corpus();
    // discover enum keys: block-sequence keys recurring >= floor with a small closed value set.
    let floor = 20usize;
    let mut key_vals: HashMap<String, HashMap<String, usize>> = HashMap::new();
    for (_slug, enums) in &md {
        for (k, vs) in enums {
            for v in vs { *key_vals.entry(k.clone()).or_default().entry(v.clone()).or_default() += 1; }
        }
    }
    println!("\n== discovered frontmatter enum keys (block-seq, support>={floor}, closed value set) ==");
    let mut enum_keys: Vec<&String> = Vec::new();
    for (k, vals) in &key_vals {
        let total: usize = vals.values().sum();
        // closed value set: few distinct values, each recurring (median support high). Heuristic: <=6 values
        // each with >=5 occurrences dominate. Filter noise (browser-compat one-offs) by per-value floor.
        let strong: Vec<(&String, &usize)> = vals.iter().filter(|(_, c)| **c >= 5).collect();
        if total >= floor && strong.len() >= 2 && strong.len() <= 6 {
            enum_keys.push(k);
            let mut sv: Vec<_> = strong.clone();
            sv.sort_by(|a, b| b.1.cmp(a.1));
            let show: Vec<String> = sv.iter().map(|(v, c)| format!("{v}={c}")).collect();
            println!("  {k:16} values[{}]: {}", strong.len(), show.join(" "));
        }
    }

    // md slug -> status values (for the discovered `status` key IF present as enum)
    let status_key = "status".to_string();
    let has_status = enum_keys.iter().any(|k| **k == status_key);
    println!("\nstatus discovered as enum key: {has_status}");
    let mut md_status: HashMap<String, Vec<String>> = HashMap::new();
    for (slug, enums) in &md {
        if let Some(vs) = enums.get(&status_key) { md_status.insert(slug.clone(), vs.clone()); }
    }
    // per status-value family: slug set
    let mut fam: HashMap<String, HashSet<String>> = HashMap::new();
    for (slug, vs) in &md_status { for v in vs { fam.entry(v.clone()).or_default().insert(slug.clone()); } }
    println!("md status families (by slug): {}",
             fam.iter().map(|(v, s)| format!("{v}={}", s.len())).collect::<Vec<_>>().join(" "));

    // ── JOIN by slug: each status family vs (a) crawl presence, (b) hand-117, (c) notecard structural run ──
    // First recover the structural notecard run (support ~117) exactly as PASS 11.
    let mut sup: HashMap<String, HashSet<String>> = HashMap::new();
    let mut subj: HashMap<String, HashSet<String>> = HashMap::new();
    for (url, body) in &pages {
        let mut on: HashSet<String> = HashSet::new();
        for r in runs_of(body) { on.insert(r); }
        for r in on { sup.entry(r.clone()).or_default().insert(url.clone()); subj.entry(r).or_default().insert(subject(url)); }
    }
    // the notecard run = the invariant run whose page-set best matches hand-117 (structural, PASS 11 F1=1.0)
    let mut best = (0.0f64, String::new(), HashSet::new());
    for (run, pgs) in &sup {
        if pgs.len() < 8 { continue; }
        let ov = pgs.iter().filter(|u| hand.contains(*u)).count();
        if ov == 0 { continue; }
        let prec = ov as f64 / pgs.len() as f64; let rec = ov as f64 / hand.len() as f64;
        let f1 = if prec + rec > 0.0 { 2.0 * prec * rec / (prec + rec) } else { 0.0 };
        if f1 > best.0 { best = (f1, run.clone(), pgs.clone()); }
    }
    let notecard_pages = best.2;
    let notecard_slugs: HashSet<String> = notecard_pages.iter().filter_map(|u| url_slug.get(u).cloned()).collect();
    println!("\nstructural notecard run: support={} F1-vs-hand={:.3} run={:?}",
             notecard_pages.len(), best.0, best.1.chars().take(50).collect::<String>());

    println!("\n== JOIN: each md status family vs crawl / hand-117 / structural notecard ==");
    println!("{:>14} {:>6} {:>10} {:>10} {:>12}", "family", "slugs", "in-crawl", "in-hand117", "in-notecard");
    for (v, slugs) in &fam {
        let in_crawl = slugs.iter().filter(|s| slug_url.contains_key(*s)).count();
        let in_hand = slugs.iter().filter(|s| hand_slugs.contains(*s)).count();
        let in_note = slugs.iter().filter(|s| notecard_slugs.contains(*s)).count();
        println!("{v:>14} {:>6} {:>10} {:>10} {:>12}", slugs.len(), in_crawl, in_hand, in_note);
    }

    // The decisive purity check: of the deprecated-family slugs present in crawl, how many land in the
    // notecard cluster (should be ~all) vs how many experimental/non-standard slugs do (should be ~0).
    let dep = fam.get("deprecated").cloned().unwrap_or_default();
    let dep_in_crawl: Vec<&String> = dep.iter().filter(|s| slug_url.contains_key(*s)).collect();
    let dep_in_note = dep_in_crawl.iter().filter(|s| notecard_slugs.contains(**s)).count();
    println!("\ndeprecated family: {} slugs, {} in crawl, {} of those in notecard cluster (label purity {:.1}%)",
             dep.len(), dep_in_crawl.len(), dep_in_note,
             if dep_in_crawl.is_empty() { 0.0 } else { 100.0 * dep_in_note as f64 / dep_in_crawl.len() as f64 });
    // reverse: notecard cluster slugs — how many have md counterpart, and of those how many are deprecated-family?
    let note_with_md = notecard_slugs.iter().filter(|s| md_status.contains_key(*s) || md.iter().any(|(ms,_)| ms==*s)).count();
    let note_md_dep = notecard_slugs.iter().filter(|s| dep.contains(*s)).count();
    println!("notecard cluster: {} slugs, {} have any md file, {} are md-deprecated (join coverage {:.1}% of 117)",
             notecard_slugs.len(), note_with_md, note_md_dep,
             100.0 * note_md_dep as f64 / notecard_slugs.len().max(1) as f64);

    // ── THE FULL DISCOVERY ALGORITHM (what the library will do): markers = runs present on >= T of the
    // prohibition family's crawled pages AND ABSENT from every non-prohibition family page. Measure the
    // resulting page-level attester (contains-any-marker) vs hand-117: precision, recall, and whether it
    // GENERALIZES past the 84 md-covered pages to the full 117. ──
    let dep_urls: HashSet<String> = dep.iter().filter_map(|s| slug_url.get(s).cloned()).collect();
    let other_urls: HashSet<String> = fam.iter().filter(|(v, _)| *v != "deprecated")
        .flat_map(|(_, s)| s.iter()).filter(|s| !dep.contains(*s))
        .filter_map(|s| slug_url.get(s).cloned()).collect();
    // run -> (count on dep pages, count on other-family pages)
    let mut score: HashMap<&String, (usize, usize)> = HashMap::new();
    for (run, pgs) in &sup {
        let on_dep = pgs.iter().filter(|u| dep_urls.contains(*u)).count();
        let on_other = pgs.iter().filter(|u| other_urls.contains(*u)).count();
        if on_dep > 0 { score.insert(run, (on_dep, on_other)); }
    }
    let t = (dep_urls.len() as f64 * 0.5).ceil() as usize; // present on >= half the family
    // A marker must ALSO be family-DOMINANT: most pages carrying it are the family (or its generalization),
    // not scattered site-wide. This drops generic runs ("compatibility table") that merely happened to miss
    // the small other-family slug sets. on_dep / total_support >= 0.5.
    // A banner is a rendered SENTENCE (the author's macro expands to prose), so require a sentence-scale
    // run (>= 6 words) — a typography property, not a word list. Drops 2-word fragments.
    let markers: Vec<String> = score.iter()
        .filter(|(run, (d, o))| *d >= t && *o == 0
                && run.split(' ').count() >= 6
                && (*d as f64 / sup.get(**run).map(|p| p.len()).unwrap_or(1) as f64) >= 0.5)
        .map(|(r, _)| (*r).clone()).collect();
    for (run, (d, o)) in score.iter().filter(|(_, (d, o))| *d >= t && *o == 0) {
        println!("   cand d={d} o={o} total={} words={}  {:?}", sup.get(*run).map(|p| p.len()).unwrap_or(0),
                 run.split(' ').count(), run.chars().take(50).collect::<String>());
    }
    println!("\n== discriminative markers (>= {t}/{} family, 0 other-family) ==", dep_urls.len());
    let marker_set: HashSet<String> = markers.iter().cloned().collect();
    let attests = |body: &str| -> bool {
        // normalize the body the same way discovery did, then substring-test each marker.
        let norm_runs: HashSet<String> = runs_of(body).into_iter().collect();
        marker_set.iter().any(|m| norm_runs.contains(m))
    };
    let mut tp = 0; let mut fp = 0; let mut fn_ = 0;
    for (u, b) in &pages {
        let a = attests(b);
        let h = hand.contains(u);
        match (a, h) { (true, true) => tp += 1, (true, false) => fp += 1, (false, true) => fn_ += 1, _ => {} }
    }
    let prec = tp as f64 / (tp + fp).max(1) as f64;
    let rec = tp as f64 / (tp + fn_).max(1) as f64;
    println!("markers={} | page attester vs hand-117: TP={tp} FP={fp} FN={fn_}  P={:.3} R={:.3}",
             markers.len(), prec, rec);
    for m in markers.iter().take(6) { println!("   marker: {:?}", m.chars().take(70).collect::<String>()); }
}
