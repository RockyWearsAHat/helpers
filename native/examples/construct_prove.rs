//! PASS 20 CONSTRUCTION-MEANING PROOF (probe — the meaning rung's empirical de-risk).
//!
//! PASS 19 rung 1 (`examples/construct_mine`) proved the ATOM: 378 sentence-CONSTRUCTIONS (invariant
//! scaffold + `⟨⟩` slots) fall out of 7848 real cached pages by pure recurrence. Its HONEST STOP named the
//! next rung: a construction's MEANING is NOT yet proven — rung 1 shows the STATES exist and REPEAT; it does
//! not decide which are prohibition vs endorsement vs description. This harness answers that, through the
//! frozen machinery, witnesses machine-derived only:
//!
//!   For each mined construction, its INSTANCE PAGES are labeled by the machine's own page-role families —
//!   the 117 deprecation-attested pages (`lint_attest::Attestation`, PROHIBITION witnesses) and the Baseline
//!   availability banners (ENDORSEMENT witnesses, discovered structurally exactly as `register_train` does).
//!   A construction inherits register R iff ≥ `REQUIRED_WITNESSES` (15 — the frozen `lint_ism` count) of its
//!   instances are INDEPENDENT (distinct slot subject) witnesses of R AND ZERO independent instances witness
//!   the OPPOSITE register (a contradiction is fatal — the same law `lint_ism::graduate` enforces). This is
//!   the ISM ≥15-independent-non-contradicting law folded STRUCTURALLY: the labeled families ARE the
//!   witnesses, the count + contradiction law is the frozen one. A construction that cannot reach the bar or
//!   that a contradiction blocks stays MINED-UNPROVEN (held, never committed — the ISM commit discipline).
//!
//! TWO FIXES over the rung-1 probe (its own noted remainder): (1) mining runs AFTER the `site_chrome` strip
//! the live pipeline applies, so the nav-chrome degenerate "constructions" (`css guides modules …`) are
//! gone. (2) The meaning is decided by machine witnesses, never by the scaffold's English words — the cue
//! grouping stays REPORTING-ONLY, as in rung 1.
//!
//! Run: `cargo run --release --features crawl --example construct_prove`
use helpers_native::lint_attest::Attestation;
use helpers_native::lint_codec::{self, Dec};
use helpers_native::lint_graph::site_chrome;
use helpers_native::lint_ism::REQUIRED_WITNESSES;
use std::collections::{HashMap, HashSet};

/// The cross-page repetition-support floor `site_chrome`/`lint_attest`/`construct_mine` all trust: a
/// scaffold is a construction only once at least this many INDEPENDENT pages testify it.
const SUPPORT_FLOOR: usize = 8;
/// A scaffold below this many non-slot words is too thin to be a construction.
const MIN_SCAFFOLD_WORDS: usize = 3;
/// Baseline availability banners are the high-support, deprecation-family-ABSENT, prose invariant runs; a
/// run must recur on at least this many pages to be a trusted endorsement banner (the `register_train`
/// floor — above ordinary section chrome).
const ENDORSE_RUN_FLOOR: usize = 400;

const SLOT_OPEN: char = '\u{1}';
const SLOT_CLOSE: char = '\u{2}';

// ── corpus loading (identical to construct_mine) ──────────────────────────────────────────────────
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

// ── flatten / sentence / shape (identical to construct_mine) ──────────────────────────────────────
fn flatten(body: &str) -> String {
    let b = body.as_bytes();
    let mut out = String::with_capacity(body.len() / 2);
    let mut i = 0usize;
    while i < b.len() {
        if b[i] == b'<' {
            let close = body[i..].find('>').map(|k| i + k).unwrap_or(b.len().saturating_sub(1));
            let tag = &body[i + 1..close.min(body.len())];
            let name: String =
                tag.chars().take_while(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
            if name == "code" {
                let after = (close + 1).min(body.len());
                if let Some(rel) = body[after..].find("</code>") {
                    let filler: String =
                        strip_tags(&body[after..after + rel]).split_whitespace().collect::<Vec<_>>().join(" ");
                    if !filler.is_empty() && filler.len() <= 60 {
                        out.push(SLOT_OPEN);
                        out.push_str(&filler);
                        out.push(SLOT_CLOSE);
                    } else {
                        out.push(' ');
                    }
                    i = after + rel + "</code>".len();
                    continue;
                }
            }
            out.push(' ');
            i = close + 1;
            continue;
        }
        let ch = body[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    decode_entities(&out)
}
fn strip_tags(frag: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in frag.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    decode_entities(&out)
}
fn decode_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}
fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_slot = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            SLOT_OPEN => in_slot = true,
            SLOT_CLOSE => in_slot = false,
            _ => {}
        }
        cur.push(c);
        let boundary = !in_slot
            && matches!(c, '.' | '!' | '?' | '\n')
            && chars.peek().is_none_or(|n| n.is_whitespace());
        if boundary {
            push_sentence(&mut out, &cur);
            cur.clear();
        }
    }
    push_sentence(&mut out, &cur);
    out
}
fn push_sentence(out: &mut Vec<String>, cur: &str) {
    let s: String = cur.split_whitespace().collect::<Vec<_>>().join(" ");
    if !s.is_empty() {
        out.push(s);
    }
}
fn shape_and_fillers(sentence: &str) -> Option<(String, Vec<String>)> {
    if !sentence.contains(SLOT_OPEN) {
        return None;
    }
    let mut shape = String::new();
    let mut fillers = Vec::new();
    let mut words = 0usize;
    let mut chars = sentence.chars().peekable();
    let mut word = String::new();
    let flush = |word: &mut String, shape: &mut String, words: &mut usize| {
        if !word.is_empty() {
            shape.push_str(&word.to_lowercase());
            shape.push(' ');
            *words += 1;
            word.clear();
        }
    };
    while let Some(c) = chars.next() {
        match c {
            SLOT_OPEN => {
                flush(&mut word, &mut shape, &mut words);
                let mut filler = String::new();
                for c2 in chars.by_ref() {
                    if c2 == SLOT_CLOSE {
                        break;
                    }
                    filler.push(c2);
                }
                shape.push_str("⟨⟩ ");
                fillers.push(filler);
            }
            c if c.is_whitespace() => flush(&mut word, &mut shape, &mut words),
            c => word.push(c),
        }
    }
    flush(&mut word, &mut shape, &mut words);
    (words >= MIN_SCAFFOLD_WORDS && !fillers.is_empty()).then(|| (shape.trim().to_string(), fillers))
}

// ── the endorsement (Baseline availability) family, discovered structurally (register_train's method) ─
fn norm_run(text: &str) -> Option<String> {
    let norm: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if norm.len() < 6 || norm.split(' ').count() < 2 {
        return None;
    }
    Some(norm)
}
fn runs_of(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let b = body.as_bytes();
    let (mut in_tag, mut start) = (false, 0usize);
    for i in 0..b.len() {
        match b[i] {
            b'<' => {
                if !in_tag {
                    if let Some(r) = norm_run(&body[start..i]) {
                        out.push(r);
                    }
                }
                in_tag = true;
            }
            b'>' => {
                in_tag = false;
                start = i + 1;
            }
            _ => {}
        }
    }
    if let Some(r) = norm_run(&body[start..]) {
        out.push(r);
    }
    out
}
/// The endorsement family: the URLs carrying a Baseline availability banner — a sentence-scale (≥6 word)
/// PROSE invariant run of high support (≥ [`ENDORSE_RUN_FLOOR`] pages) that is ABSENT from every
/// deprecation-attested page. Exactly `register_train`'s GOOD-register discovery, no banner text named.
fn endorsement_urls(pages: &[(String, String)], prohibition: &HashSet<String>) -> HashSet<String> {
    let mut sup: HashMap<String, HashSet<String>> = HashMap::new();
    for (url, body) in pages {
        for r in runs_of(body).into_iter().collect::<HashSet<_>>() {
            sup.entry(r).or_default().insert(url.clone());
        }
    }
    let is_prose = |r: &str| {
        let n = r.chars().count().max(1);
        let alpha = r.chars().filter(|c| c.is_alphabetic() || c.is_whitespace()).count();
        alpha as f64 / n as f64 >= 0.90
    };
    let mut endorse: HashSet<String> = HashSet::new();
    for (run, pgs) in &sup {
        if run.split(' ').count() >= 6
            && is_prose(run)
            && pgs.len() >= ENDORSE_RUN_FLOOR
            && pgs.iter().all(|u| !prohibition.contains(u))
        {
            endorse.extend(pgs.iter().cloned());
        }
    }
    endorse
}

/// A construction's per-family independent-witness tallies. Independence = DISTINCT slot subject (the
/// primary filler, lowercased): the same slot value across many pages is ONE witness, exactly as
/// `lint_ism` collapses a repeated phrasing to one independent witness.
#[derive(Default)]
struct Witnesses {
    prohibition: HashSet<String>,
    endorsement: HashSet<String>,
    neutral: HashSet<String>,
}

fn main() {
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
        let got = load(name);
        if got.is_empty() {
            continue;
        }
        println!("corpus {name}: {} pages", got.len());
        for (u, b) in got {
            if seen.insert(u.clone()) {
                pages.push((u, b));
            }
        }
    }
    println!("total distinct pages: {}", pages.len());

    // FIX (1): strip site chrome exactly as `graduated_rules` does, BEFORE mining. The `NO_CHROME` env
    // toggle isolates the chrome-strip effect (measurement: does the strip eat scaffold text?).
    let strip_chrome = std::env::var("NO_CHROME").is_err();
    let chrome = site_chrome(&pages);
    println!("site chrome: {} runs discovered across hosts (strip={strip_chrome})", chrome.len());
    let stripped: Vec<(String, String)> = if strip_chrome {
        pages.iter().map(|(u, b)| (u.clone(), chrome.strip(u, b))).collect()
    } else {
        pages.clone()
    };

    // The machine-derived labeled families (witnesses), discovered on the ORIGINAL bodies (the
    // deprecation banner is a chrome-surviving marker; attestation reads the body's own class/run).
    let attest = Attestation::discover(&pages);
    let prohibition: HashSet<String> =
        pages.iter().filter(|(_, b)| attest.attests(b)).map(|(u, _)| u.clone()).collect();
    let endorsement = endorsement_urls(&pages, &prohibition);
    // The attested-SUBJECT set: the last path segment of every deprecation-attested page URL, lowercased
    // (the same subject key `lint_attest` uses). A slot filler naming one of these is naming a
    // machine-proven-deprecated construct — the behavioral cross-reference for two-slot supersession.
    let attested_subjects: HashSet<String> = prohibition
        .iter()
        .map(|u| u.split('#').next().unwrap_or(u).trim_end_matches('/').rsplit('/').next().unwrap_or("").to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    let is_dep_subject = |f: &str| {
        let low = f.to_lowercase();
        let bare: String = low.trim_end_matches("()").chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.').collect();
        let tail = bare.rsplit(['.', ':']).next().unwrap_or(&bare);
        !tail.is_empty() && (attested_subjects.contains(&bare) || attested_subjects.contains(tail))
    };
    println!(
        "\nlabeled families: prohibition (deprecation-attested) = {} pages, endorsement (Baseline) = {} pages",
        prohibition.len(),
        endorsement.len()
    );

    // Mine constructions on the STRIPPED bodies, and per construction accumulate per-family independent
    // witnesses keyed by the primary slot filler.
    let mut pages_of: HashMap<String, HashSet<String>> = HashMap::new();
    let mut fillers_of: HashMap<String, HashSet<String>> = HashMap::new();
    let mut slot_fillers_of: HashMap<String, Vec<HashSet<String>>> = HashMap::new();
    let mut witness_of: HashMap<String, Witnesses> = HashMap::new();
    let mut example_of: HashMap<String, String> = HashMap::new();
    for (url, body) in &stripped {
        let text = flatten(body);
        let in_proh = prohibition.contains(url);
        let in_endo = endorsement.contains(url);
        for sent in sentences(&text) {
            let Some((shape, fillers)) = shape_and_fillers(&sent) else { continue };
            pages_of.entry(shape.clone()).or_default().insert(url.clone());
            let subject = fillers[0].to_lowercase();
            let fe = fillers_of.entry(shape.clone()).or_default();
            for f in &fillers {
                fe.insert(f.clone());
            }
            let sf = slot_fillers_of.entry(shape.clone()).or_default();
            for (i, f) in fillers.iter().enumerate() {
                if sf.len() <= i {
                    sf.push(HashSet::new());
                }
                sf[i].insert(f.clone());
            }
            let w = witness_of.entry(shape.clone()).or_default();
            if in_proh {
                w.prohibition.insert(subject);
            } else if in_endo {
                w.endorsement.insert(subject);
            } else {
                w.neutral.insert(subject);
            }
            example_of.entry(shape.clone()).or_insert_with(|| render_example(&sent));
        }
    }

    // A construction: the recurrence floor, identical to construct_mine / the chrome atom.
    let constructions: Vec<&String> = pages_of
        .iter()
        .filter(|(shape, urls)| {
            urls.len() >= SUPPORT_FLOOR
                && fillers_of.get(*shape).map(HashSet::len).unwrap_or(0) >= 2
        })
        .map(|(shape, _)| shape)
        .collect();
    println!(
        "\nconstructions mined AFTER chrome strip: {} (rung-1 pre-strip probe reported 378 incl. chrome artifacts)\n",
        constructions.len()
    );

    // THE MEANING PROOF: register inheritance under the frozen ≥REQUIRED_WITNESSES-independent,
    // zero-contradiction law. proven(R) iff |independent R witnesses| >= 15 AND |opposite register| == 0.
    let mut proven: Vec<(&String, &'static str, usize, usize, usize)> = Vec::new();
    for shape in &constructions {
        let w = &witness_of[*shape];
        let (p, e, n) = (w.prohibition.len(), w.endorsement.len(), w.neutral.len());
        let verdict = if p >= REQUIRED_WITNESSES && e == 0 {
            Some("PROHIBITION")
        } else if e >= REQUIRED_WITNESSES && p == 0 {
            Some("ENDORSEMENT")
        } else {
            None
        };
        if let Some(reg) = verdict {
            proven.push((shape, reg, p, e, n));
        }
    }
    proven.sort_by(|a, b| b.2.max(b.3).cmp(&a.2.max(a.3)));

    println!("###### PROVEN CONSTRUCTION STATES — register inherited, ≥{REQUIRED_WITNESSES} independent, 0 contradiction ######");
    println!("  {} proven of {} mined\n", proven.len(), constructions.len());
    println!("  {:>5} {:>5} {:>5}  {:<12}  scaffold", "proh", "endo", "neut", "register");
    for (shape, reg, p, e, n) in &proven {
        let disp: String = shape.chars().take(66).collect();
        println!("  {p:>5} {e:>5} {n:>5}  {reg:<12}  {disp}");
        let exd: String = example_of.get(*shape).cloned().unwrap_or_default().chars().take(96).collect();
        println!("        e.g. {exd}");
    }

    // DESIGN-RELEVANT REGISTER FAMILIES — the classics' own governing constructions, with their raw
    // family breakdown so the meaning-proof outcome (proven / why-not) is visible per construction. This
    // grouping is REPORTING-ONLY (cue words, never mining/proof logic — the verdict above used families).
    println!("\n###### DESIGN-RELEVANT CONSTRUCTIONS — family breakdown + verdict (reporting cues) ######");
    let cues: &[(&str, &[&str])] = &[
        ("supersession / remedy", &["instead", "replace", "rather than", "in favor", "in favour"]),
        ("prohibition / caution", &["should not", "do not", "don't", "must not", "avoid", "never"]),
        ("deprecation / removal", &["deprecated", "obsolete", "no longer", "removed", "last version"]),
    ];
    for (label, words) in cues {
        let mut hits: Vec<&&String> = constructions
            .iter()
            .filter(|s| words.iter().any(|w| s.contains(w)))
            .collect();
        hits.sort_by_key(|s| std::cmp::Reverse(pages_of[**s].len()));
        println!("\n  -- {label}: {} construction(s) --", hits.len());
        for shape in hits.iter().take(12) {
            let w = &witness_of[**shape];
            let (p, e, n) = (w.prohibition.len(), w.endorsement.len(), w.neutral.len());
            let verdict = if p >= REQUIRED_WITNESSES && e == 0 {
                "PROVEN prohibition"
            } else if e >= REQUIRED_WITNESSES && p == 0 {
                "PROVEN endorsement"
            } else if p >= REQUIRED_WITNESSES && e > 0 {
                "blocked: contradicted by endorsement"
            } else {
                "unproven: too few independent"
            };
            let disp: String = shape.chars().take(60).collect();
            println!(
                "    pages={:>3} proh={p:>3} endo={e:>3} neut={n:>3}  [{verdict}]  {disp}",
                pages_of[**shape].len()
            );
        }
    }

    // BEHAVIORAL CROSS-REFERENCE (the task's rung-1 two-slot proof): for each supersession/remedy
    // construction, how many of each slot's distinct fillers NAME a machine-proven-deprecated subject
    // (member of `attested_subjects`)? A clean supersession direction would show slot-A fillers landing
    // on deprecated subjects and slot-B (remedy) fillers NOT — proven with zero meaning geometry.
    println!("\n###### TWO-SLOT SUPERSESSION — slot fillers vs the proven-deprecated SUBJECT set ######");
    println!("  (attested deprecation subjects in corpus: {})", attested_subjects.len());
    let super_cues = ["instead", "replace", "rather than", "in favor", "in favour"];
    let mut supers: Vec<&&String> = constructions
        .iter()
        .filter(|s| super_cues.iter().any(|w| s.contains(w)))
        .filter(|s| slot_fillers_of.get(**s).map(|v| v.len() >= 2).unwrap_or(false))
        .collect();
    supers.sort_by_key(|s| std::cmp::Reverse(pages_of[**s].len()));
    for shape in supers.iter().take(14) {
        let slots = &slot_fillers_of[**shape];
        let dep_a = slots[0].iter().filter(|f| is_dep_subject(f)).count();
        let dep_b = slots[1].iter().filter(|f| is_dep_subject(f)).count();
        let disp: String = shape.chars().take(56).collect();
        println!(
            "    slotA {}/{} dep   slotB {}/{} dep   {disp}",
            dep_a,
            slots[0].len(),
            dep_b,
            slots[1].len()
        );
    }
}

fn render_example(sentence: &str) -> String {
    sentence.replace(SLOT_OPEN, "⟨").replace(SLOT_CLOSE, "⟩")
}
