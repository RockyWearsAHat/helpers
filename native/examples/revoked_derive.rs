//! THROWAWAY PROBE (untracked): PASS 26 RUNG A — DERIVE the revoked status family STRUCTURALLY, killing
//! the `deprecation-status.json` seed (`prohibits:["deprecated"]`). The frontmatter `status:` enum families
//! (deprecated / experimental / non-standard) are DISCOVERED by shape already; the seed only supplies WHICH
//! family means REVOKED. Derive that from the families' own structural behavior across the corpus — no
//! English polarity, no word list in the SHIPPED faculty (this probe uses light proxies ONLY to MEASURE
//! whether an axis separates; a separating axis is re-expressed structurally when wired).
//!
//! Three axes, measured per family with the experimental family as the mandatory FOIL (not-yet ≠ no-longer):
//!   1. USAGE DEATH   — fraction of a family's member constructs that appear in the corpus's own current
//!                      example code (other pages' `<pre>` blocks). Dead ≈ absent; Baseline ≈ ubiquitous.
//!   2. SUCCESSION    — fraction of member pages whose prose designates a replacement pointing AWAY.
//!   3. VERSIONED     — fraction of member pages carrying a past-tense version-bound END structure.
//!      CESSATION
//! Run: `cargo run --release --features crawl --example revoked_derive`
use helpers_native::lint_codec::{self, Dec};
use std::collections::{HashMap, HashSet};

// ── corpus load (same codec as metajoin) ────────────────────────────────────────────────────────────
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
fn load(name: &str) -> Vec<(String, String)> {
    let home = std::env::var("HOME").unwrap();
    let dir = format!("{home}/.cache/helpers/lint-index/crawls");
    std::fs::read(format!("{dir}/{name}.bin")).ok().and_then(|b| decode_bin(&b)).unwrap_or_default()
}

fn slug_of_url(url: &str) -> Option<String> {
    let u = url.split('#').next().unwrap_or(url);
    let i = u.find("/docs/")?;
    Some(u[i + 6..].trim_end_matches('/').to_lowercase())
}
fn subject(url: &str) -> String {
    url.split('#').next().unwrap_or(url).trim_end_matches('/').rsplit('/').next().unwrap_or("").to_lowercase()
}

// ── markdown frontmatter: (slug, status-enum-values, page-type) per *.md ──
fn md_frontmatter(text: &str) -> Option<(String, Vec<String>, String)> {
    let t = text.strip_prefix("---")?;
    let end = t.find("\n---")?;
    let fm = &t[..end];
    let mut slug = None;
    let mut page_type = String::new();
    let mut status: Vec<String> = Vec::new();
    let mut cur_key: Option<String> = None;
    for line in fm.lines() {
        if let Some(rest) = line.strip_prefix("  - ") {
            if cur_key.as_deref() == Some("status") {
                status.push(rest.trim().trim_matches('"').to_lowercase());
            }
            continue;
        }
        if line.starts_with(' ') { continue; }
        if let Some(colon) = line.find(':') {
            let key = line[..colon].trim().to_lowercase();
            let val = line[colon + 1..].trim().trim_matches('"');
            if key == "slug" { slug = Some(val.to_lowercase()); }
            if key == "page-type" { page_type = val.to_lowercase(); }
            cur_key = if val.is_empty() { Some(key) } else { None };
        } else {
            cur_key = None;
        }
    }
    Some((slug?, status, page_type))
}
struct Meta { status: Vec<String>, page_type: String }
fn md_corpus() -> HashMap<String, Meta> {
    let home = std::env::var("HOME").unwrap();
    let root = format!("{home}/.cache/helpers/mdn-content/files");
    let mut out = HashMap::new();
    let mut stack = vec![std::path::PathBuf::from(&root)];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() { stack.push(p); continue; }
            if p.extension().and_then(|x| x.to_str()) != Some("md") { continue; }
            if let Ok(txt) = std::fs::read_to_string(&p) {
                if let Some((slug, status, pt)) = md_frontmatter(&txt) {
                    out.insert(slug, Meta { status, page_type: pt });
                }
            }
        }
    }
    out
}

// ── code-block token index (axis 1) ──
// Extract identifier tokens from every `<pre>` block; a token is [a-z0-9_-]+ (keeps CSS prop / element
// names intact). token -> set of source page urls carrying it in code.
fn pre_blocks(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let low = body;
    let mut i = 0usize;
    while let Some(rel) = low[i..].find("<pre") {
        let start = i + rel;
        let Some(gt) = low[start..].find('>') else { break };
        let content_start = start + gt + 1;
        let Some(end_rel) = low[content_start..].find("</pre>") else { break };
        out.push(low[content_start..content_start + end_rel].to_string());
        i = content_start + end_rel + 6;
    }
    out
}
fn strip_tags(frag: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in frag.chars() {
        match c { '<' => in_tag = true, '>' => in_tag = false, _ if !in_tag => out.push(c), _ => {} }
    }
    out.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&").replace("&quot;", "\"")
}
fn code_tokens(body: &str) -> HashSet<String> {
    let mut toks = HashSet::new();
    for pre in pre_blocks(body) {
        let text = strip_tags(&pre).to_lowercase();
        let mut cur = String::new();
        for c in text.chars() {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                cur.push(c);
            } else {
                if cur.len() >= 3 { toks.insert(std::mem::take(&mut cur)); } else { cur.clear(); }
            }
        }
        if cur.len() >= 3 { toks.insert(cur); }
    }
    toks
}

// prose flatten (axes 2/3 proxies)
fn prose(body: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in body.chars() {
        match c { '<' => in_tag = true, '>' => in_tag = false, _ if !in_tag => out.push(c), _ => out.push(' ') }
    }
    out.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&").to_lowercase()
}

// Ordered `<code>` filler tokens per sentence of a body (word-free succession substrate). A filler is
// normalized to its dominant identifier token (`[a-z0-9_-]+`, longest). Sentences split on . ! ? outside code.
fn code_sentences(body: &str) -> Vec<Vec<String>> {
    // build flattened stream: text chars + slot markers carrying normalized filler
    let mut units: Vec<(bool, String)> = Vec::new(); // (is_code, token-or-text)
    let mut i = 0usize;
    while let Some(rel) = body[i..].find('<') {
        let start = i + rel;
        units.push((false, body[i..start].to_string()));
        let Some(gt) = body[start..].find('>') else { break };
        let tag = &body[start + 1..start + gt];
        let name: String = tag.chars().take_while(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
        if name == "code" {
            let after = start + gt + 1;
            if let Some(er) = body[after..].find("</code>") {
                let inner = strip_tags(&body[after..after + er]).to_lowercase();
                // dominant identifier token
                let tok = inner.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'))
                    .filter(|s| s.len() >= 2).max_by_key(|s| s.len()).unwrap_or("").to_string();
                units.push((true, tok));
                i = after + er + 7;
                continue;
            }
        }
        i = start + gt + 1;
    }
    let mut out = Vec::new();
    let mut cur: Vec<String> = Vec::new();
    for (is_code, s) in units {
        if is_code {
            if !s.is_empty() { cur.push(s); }
        } else {
            for c in s.chars() {
                if matches!(c, '.' | '!' | '?' | '\n') {
                    if !cur.is_empty() { out.push(std::mem::take(&mut cur)); }
                }
            }
        }
    }
    if !cur.is_empty() { out.push(cur); }
    out
}
// word-free directional succession: a sentence whose FIRST code names `own` and a LATER code names a
// DISTINCT construct that is NOT in the family (a replacement pointing away).
fn code_remedy_sentence(body: &str, own: &str, self_set: &HashSet<String>) -> bool {
    let name_of = |t: &str| t.rsplit('.').next().unwrap_or(t).to_string();
    for sent in code_sentences(body) {
        if sent.len() < 2 { continue; }
        let first = name_of(&sent[0]);
        if first != own { continue; }
        for later in &sent[1..] {
            let nm = name_of(later);
            if nm != own && nm.len() >= 2 && !self_set.contains(&nm) { return true; }
        }
    }
    false
}

// Mine sentence SHAPES (invariant words + ⟨⟩ slots) carrying >=2 code slots. Returns (shape, fillers).
fn two_slot_shapes(body: &str) -> Vec<(String, Vec<String>)> {
    // flatten to text with \u{1}filler\u{2} slots
    let mut flat = String::new();
    let mut i = 0usize;
    while let Some(rel) = body[i..].find('<') {
        let start = i + rel;
        flat.push_str(&strip_tags(&body[i..start]));
        let Some(gt) = body[start..].find('>') else { break };
        let tag = &body[start + 1..start + gt];
        let name: String = tag.chars().take_while(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
        if name == "code" {
            let after = start + gt + 1;
            if let Some(er) = body[after..].find("</code>") {
                let inner = strip_tags(&body[after..after + er]);
                let f: String = inner.split_whitespace().collect::<Vec<_>>().join(" ");
                if !f.is_empty() && f.len() <= 40 { flat.push('\u{1}'); flat.push_str(&f); flat.push('\u{2}'); }
                else { flat.push(' '); }
                i = after + er + 7; continue;
            }
        }
        flat.push(' ');
        i = start + gt + 1;
    }
    flat.push_str(&strip_tags(&body[i.min(body.len())..]));
    // split into sentences (not inside slot)
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_slot = false;
    let mut chars = flat.chars().peekable();
    while let Some(c) = chars.next() {
        match c { '\u{1}' => in_slot = true, '\u{2}' => in_slot = false, _ => {} }
        cur.push(c);
        if !in_slot && matches!(c, '.' | '!' | '?' | '\n') {
            emit_shape(&cur, &mut out); cur.clear();
        }
    }
    emit_shape(&cur, &mut out);
    out
}
fn emit_shape(sent: &str, out: &mut Vec<(String, Vec<String>)>) {
    let mut shape = String::new();
    let mut fillers = Vec::new();
    let mut word = String::new();
    let mut words = 0usize;
    let mut chars = sent.chars().peekable();
    let flush = |w: &mut String, sh: &mut String, n: &mut usize| {
        if !w.is_empty() { sh.push_str(&w.to_lowercase()); sh.push(' '); *n += 1; w.clear(); }
    };
    while let Some(c) = chars.next() {
        match c {
            '\u{1}' => {
                flush(&mut word, &mut shape, &mut words);
                let mut f = String::new();
                for c2 in chars.by_ref() { if c2 == '\u{2}' { break; } f.push(c2); }
                shape.push_str("⟨⟩ ");
                fillers.push(f.split_whitespace().next().unwrap_or("").to_lowercase());
            }
            c if c.is_whitespace() => flush(&mut word, &mut shape, &mut words),
            c => word.push(c),
        }
    }
    flush(&mut word, &mut shape, &mut words);
    if fillers.len() >= 2 && words >= 3 { out.push((shape.trim().to_string(), fillers)); }
}

fn main() {
    let mut pages = Vec::new();
    for n in ["mdn-css", "mdn-js", "mdn-html", "mdn-svg",
              "developer-mozilla-org-4cbd1761", "developer-mozilla-org-572fe52b",
              "developer-mozilla-org-611ab56b", "developer-mozilla-org-5d48a8d0"] {
        pages.extend(load(n));
    }
    let mut seen = HashSet::new();
    pages.retain(|(u, _)| seen.insert(u.clone()));
    println!("corpus: {} distinct MDN pages", pages.len());

    let md = md_corpus();
    println!("md frontmatter: {} slugs", md.len());

    // slug -> crawled (url, body)
    let mut slug_page: HashMap<String, (String, String)> = HashMap::new();
    for (u, b) in &pages {
        if let Some(s) = slug_of_url(u) { slug_page.entry(s).or_insert_with(|| (u.clone(), b.clone())); }
    }

    // ── family membership (frontmatter status value -> member slugs) ──
    let families = ["deprecated", "non-standard", "experimental"];
    let mut fam: HashMap<&str, Vec<String>> = HashMap::new();
    for (slug, m) in &md {
        for v in &m.status {
            for f in families { if v == f { fam.entry(f).or_default().push(slug.clone()); } }
        }
    }
    // Baseline control: CSS/HTML/JS reference pages (page-type property/element/method) with NO status enum.
    let mut baseline: Vec<String> = Vec::new();
    for (slug, m) in &md {
        let is_ref = m.page_type.ends_with("property") || m.page_type.ends_with("element")
            || m.page_type.ends_with("method") || m.page_type.ends_with("type");
        if is_ref && m.status.is_empty() { baseline.push(slug.clone()); }
    }
    fam.insert("BASELINE(current-ref)", baseline);

    // ── AXIS 1 index: token -> set of pages carrying it in a <pre> block ──
    println!("indexing code tokens over {} pages…", pages.len());
    let mut tok_pages: HashMap<String, HashSet<String>> = HashMap::new();
    for (u, b) in &pages {
        for t in code_tokens(b) { tok_pages.entry(t).or_default().insert(u.clone()); }
    }
    println!("code-token index: {} distinct tokens\n", tok_pages.len());

    // construct name of a slug = last path segment (the property/element/api name)
    let construct = |slug: &str| slug.rsplit('/').next().unwrap_or(slug).to_lowercase();

    // succession proxy: a DIRECTIONAL replacement phrase (points away to a successor). Sentence-local,
    // measured near a <code> slot. Drops the generic "use the" (fires on ordinary reference prose).
    let succession_words = ["replaced by", "superseded", "replacement for", "has been replaced",
                            "successor", "instead of using", "use it instead", "instead."];
    // versioned-cessation proxy: past-tense version-bound END structure
    let cessation_words = ["last version", "was removed", "removed in", "no longer supported",
                           "no longer available", "has been removed"];

    // Member-construct sets per family (for the word-free structural succession test: slotB points OUTSIDE
    // the family). These are the frontmatter-shape-derived families — no word list.
    let mut fam_constructs: HashMap<&str, HashSet<String>> = HashMap::new();
    for (k, slugs) in &fam {
        fam_constructs.insert(*k, slugs.iter().map(|s| construct(s)).collect());
    }

    println!("{:>26} {:>7} {:>8}  {:>18} {:>12} {:>12} {:>14}",
             "family", "members", "joined", "usage-DEATH(absent)", "succession", "cessation", "struct-succ");
    let order = ["deprecated", "non-standard", "experimental", "BASELINE(current-ref)"];
    for key in order {
        let Some(slugs) = fam.get(key) else { continue };
        let mut members: Vec<String> = slugs.iter().map(|s| construct(s)).collect();
        members.sort(); members.dedup();
        let n = members.len();

        // axis 1: for each member construct, present in OTHER pages' code?
        let mut absent = 0usize;
        let mut present_pagecounts: Vec<usize> = Vec::new();
        for (slug, c) in slugs.iter().map(|s| (s, construct(s))) {
            let own = slug_page.get(slug).map(|(u, _)| u.clone());
            let pgs = tok_pages.get(&c).cloned().unwrap_or_default();
            let others = pgs.iter().filter(|u| Some(*u) != own.as_ref()).count();
            if others == 0 { absent += 1; }
            present_pagecounts.push(others);
        }
        let death_frac = absent as f64 / n.max(1) as f64;
        present_pagecounts.sort();
        let median = present_pagecounts.get(present_pagecounts.len() / 2).copied().unwrap_or(0);

        // axes 2/3: only over JOINED member pages (need the body)
        let joined: Vec<(String, String)> =
            slugs.iter().filter_map(|s| slug_page.get(s).cloned()).collect();
        let nj = joined.len().max(1);
        let mut succ = 0usize;
        let mut cess = 0usize;
        let mut struct_succ = 0usize; // word-free: own construct in a <code>, then a DIFFERENT construct
                                      // in a later <code> within the same sentence, pointing OUTSIDE family
        let self_set = fam_constructs.get(key).cloned().unwrap_or_default();
        for (u, b) in &joined {
            let p = prose(b);
            let has_code = b.contains("<code");
            if has_code && succession_words.iter().any(|w| p.contains(w)) { succ += 1; }
            if cessation_words.iter().any(|w| p.contains(w)) { cess += 1; }
            // structural: this page's construct + a later distinct code construct outside the family
            let own = construct(&slug_of_url(u).unwrap_or_default());
            if code_remedy_sentence(b, &own, &self_set) { struct_succ += 1; }
        }
        println!("{:>26} {:>7} {:>8}  {:>10}({:.0}% med{:>2}) {:>10}({:.0}%) {:>6}({:.0}%) {:>7}({:.0}%)",
                 key, n, joined.len(),
                 absent, 100.0 * death_frac, median,
                 succ, 100.0 * succ as f64 / nj as f64,
                 cess, 100.0 * cess as f64 / nj as f64,
                 struct_succ, 100.0 * struct_succ as f64 / nj as f64);
    }

    // ── MINED recurrent scaffold succession (PASS-21, connective MINED not coded) ──
    println!("\n== MINED recurrent 2-slot scaffolds per family (>=8 pages, varying slot) ==");
    for key in ["deprecated", "non-standard", "experimental", "BASELINE(current-ref)"] {
        let Some(slugs) = fam.get(key) else { continue };
        let joined: Vec<(String, String)> =
            slugs.iter().filter_map(|s| slug_page.get(s).cloned()).collect();
        let mut shapes: HashMap<String, (HashSet<String>, HashMap<String, HashSet<String>>)> = HashMap::new();
        for (u, b) in &joined {
            for (shape, fillers) in two_slot_shapes(b) {
                let e = shapes.entry(shape).or_default();
                e.0.insert(u.clone());
                e.1.entry(fillers[0].clone()).or_default().insert(u.clone());
            }
        }
        let mut members_in: HashSet<String> = HashSet::new();
        let mut top: Vec<(String, usize)> = Vec::new();
        for (shape, (pgs, f0)) in &shapes {
            let mode = f0.values().map(|s| s.len()).max().unwrap_or(0) as f64 / pgs.len().max(1) as f64;
            if pgs.len() >= 8 && mode <= 0.5 {
                for u in pgs { members_in.insert(u.clone()); }
                top.push((shape.clone(), pgs.len()));
            }
        }
        top.sort_by(|a, b| b.1.cmp(&a.1));
        println!("  {:>26}: {} member-pages in a recurrent 2-slot scaffold ({:.0}% of {})",
                 key, members_in.len(), 100.0 * members_in.len() as f64 / joined.len().max(1) as f64, joined.len());
        for (s, c) in top.iter().take(3) {
            println!("        [{c}] {}", s.chars().take(72).collect::<String>());
        }
    }

    // ── decisive per-member listing for the deprecated vs experimental foil (spot check) ──
    println!("\n== deprecated members ABSENT from all other code (usage-dead) — sample ==");
    if let Some(slugs) = fam.get("deprecated") {
        let mut dead: Vec<String> = Vec::new();
        for slug in slugs {
            let c = construct(slug);
            let own = slug_page.get(slug).map(|(u, _)| u.clone());
            let others = tok_pages.get(&c).map(|p| p.iter().filter(|u| Some(*u) != own.as_ref()).count()).unwrap_or(0);
            if others == 0 { dead.push(c); }
        }
        dead.sort();
        println!("  {}", dead.iter().take(30).cloned().collect::<Vec<_>>().join(", "));
    }
    println!("\n== experimental members PRESENT in other code (alive) — sample ==");
    if let Some(slugs) = fam.get("experimental") {
        let mut alive: Vec<(String, usize)> = Vec::new();
        for slug in slugs {
            let c = construct(slug);
            let own = slug_page.get(slug).map(|(u, _)| u.clone());
            let others = tok_pages.get(&c).map(|p| p.iter().filter(|u| Some(*u) != own.as_ref()).count()).unwrap_or(0);
            if others > 0 { alive.push((c, others)); }
        }
        alive.sort_by(|a, b| b.1.cmp(&a.1));
        println!("  {}", alive.iter().take(20).map(|(c, n)| format!("{c}({n})")).collect::<Vec<_>>().join(", "));
    }
}
