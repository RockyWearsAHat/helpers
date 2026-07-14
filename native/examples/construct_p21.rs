//! PASS 21 CONSTRUCTION PROOF — closing the two PASS-20 walls (measurement probe).
//!
//! PASS 20 (`examples/construct_prove`) measured two walls that blocked committing a construction MEANING
//! state:
//!   WALL A — the `site_chrome` strip the live prose reader REQUIRES eats the construction scaffold: a
//!            scaffold's invariant flanking run keys as chrome and is blanked, so after the strip every
//!            design-relevant register construction is gone (378 → 113, only junk endorsement proves).
//!   WALL B — page-role families (deprecation-attested pages vs Baseline pages) label the PAGE's subject
//!            status, not a SENTENCE-level register, so the 15 states that reach the ≥15 floor are
//!            API-description co-occurrence artifacts and the genuine register constructions are neutral.
//!
//! This probe builds the two PASS-20 re-land conditions and MEASURES them, machine-derived witnesses only:
//!
//! RUNG 1 (Wall A) — SLOT-AWARE SCAFFOLD/CHROME DISAMBIGUATION, co-resident with the chrome pass.
//!   The owner's signal: chrome's content is INVARIANT BY DEFINITION, so a run flanking a code slot whose
//!   filler VARIES page-to-page is definitionally a SCAFFOLD, not chrome. A nav menu that carries a code
//!   slot (MDN's sidebar list) is still chrome — but it recurs the WHOLE list on EVERY page, so it fires
//!   MANY times per page, while a real scaffold sentence fires ONCE per page. So the structural
//!   discriminator is MAX OCCURRENCES ON A SINGLE PAGE: a scaffold ≈ 1, a menu ≫ 1. Mining runs on the
//!   UNSTRIPPED bodies; a construction whose flanking runs the strip would eat is RESCUED iff it is a
//!   genuine scaffold (few occurrences per page). The live prose reader keeps stripping chrome fully; only
//!   the construction miner claims the scaffold runs back.
//!
//!   Also measured: the rustdoc STRAY-BADGE page-role contamination (PASS 20's 117 → 277). A page-level
//!   class-token attestation fires on ANY `class="stab deprecated"` badge — including a sidebar cross-ref to
//!   a DIFFERENT deprecated item — so a whole rust-std integer page attests as deprecated. The PASS-14
//!   item-level unit is the fix: a badge attests only the DOTTED ITEM it anchors, so a page's SUBJECT is
//!   proven-deprecated only when the page's own item carries the badge.
//!
//! RUNG 2 (Wall B) — INSTANCE-LEVEL WITNESSES, not page-role families. A construction's witnesses are its
//!   OWN INSTANCES cross-referenced against the machine's proven-deprecated SUBJECT set (each subject was
//!   independently proven by the attestation faculty — genuine independence, the ISM independence law). An
//!   instance whose slot filler NAMES a proven-deprecated subject is one witness for deprecation-direction;
//!   for a two-slot construction, slotA deprecated + slotB clean = a supersession/remedy witness. Count
//!   DISTINCT deprecated subjects (independence). ≥ REQUIRED_WITNESSES (15, frozen) with zero contradiction
//!   proves; below floor stays MINED-UNPROVEN (the PASS-12 discovery-starved wall — no floor relitigating).
//!
//! Run: `cargo run --release --features crawl --example construct_p21`
use helpers_native::lint_attest::{prohibition_class_tokens, Attestation};
use helpers_native::lint_codec::{self, Dec};
use helpers_native::lint_graph::site_chrome;
use helpers_native::lint_ism::REQUIRED_WITNESSES;
use std::collections::{HashMap, HashSet};

/// The cross-page repetition-support floor `site_chrome`/`lint_attest`/`construct_mine` all trust.
const SUPPORT_FLOOR: usize = 8;
/// A scaffold below this many non-slot words is too thin to be a construction.
const MIN_SCAFFOLD_WORDS: usize = 3;
/// A construction is a genuine SCAFFOLD (not chrome/boilerplate) iff its primary slot VARIES across pages:
/// no single filler covers more than this fraction of the construction's pages. Chrome and rustdoc
/// trait-method boilerplate carry the SAME code on every page (one filler covers ~all pages, mode → 1.0); a
/// real scaffold's code is page-specific (mode low). The owner's signal — "chrome's content is invariant, a
/// slot-varying run is definitionally not chrome" — made structural. Measured cut; the probe prints the
/// mode-fraction per construction so the separation is visible.
const SCAFFOLD_MODE_FRAC: f64 = 0.5;
/// Baseline availability banners recur on at least this many pages (the `register_train` endorsement floor).
const ENDORSE_RUN_FLOOR: usize = 400;

const SLOT_OPEN: char = '\u{1}';
const SLOT_CLOSE: char = '\u{2}';

// ── corpus loading (identical to construct_prove) ──────────────────────────────────────────────────
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

// ── flatten / sentence / shape (identical to construct_prove) ──────────────────────────────────────
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

// ── ITEM-LEVEL deprecation attestation (the PASS-14 unit, replicated here for the measurement) ──────
// A deprecation badge attests only the DOTTED ITEM it anchors, never the whole page. This kills the
// stray-sidebar-badge page contamination (PASS 20's 117 → 277). Replicated from `lint_lang_layer`'s
// `attested_item_shapes` so the library stays untouched during this measurement rung.
const MARKER_ANCHOR_WINDOW: usize = 20_000;
fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=");
    let i = tag.find(&key)?;
    let rest = &tag[i + key.len()..];
    let q = rest.chars().next()?;
    if q != '"' && q != '\'' {
        return None;
    }
    let rest = &rest[1..];
    let end = rest.find(q)?;
    Some(rest[..end].to_string())
}
fn dotted_item(v: &str) -> bool {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    v.len() >= 3
        && v.contains('.')
        && !v.starts_with('.')
        && !v.ends_with('.')
        && !v.contains("..")
        && v.chars().all(|c| is_ident(c) || c == '.')
        && v.rsplit('.').next().map(|last| last.len() >= 2).unwrap_or(false)
}
/// The dotted item ids a body marks deprecated, each attributed by the marker element's typography
/// (self-anchored / container-opening / trailing-badge), exactly as the PASS-14 item unit does.
fn attested_items(body: &str, tokens: &[String]) -> Vec<String> {
    if tokens.is_empty() {
        return Vec::new();
    }
    let mut ids: Vec<(usize, String)> = Vec::new();
    let mut markers: Vec<(usize, Option<String>)> = Vec::new();
    let mut text_positions: Vec<usize> = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let Some(rel) = body[i..].find('>') else { break };
            let tag = &body[i..i + rel + 1];
            let own_id = attr(tag, "id");
            if let Some(id) = &own_id {
                ids.push((i, id.clone()));
            }
            if let Some(class) = attr(tag, "class") {
                if class.split_whitespace().any(|t| tokens.iter().any(|m| m.eq_ignore_ascii_case(t))) {
                    markers.push((i, own_id));
                }
            }
            i += rel + 1;
        } else {
            if !(bytes[i] as char).is_whitespace() {
                text_positions.push(i);
            }
            i += 1;
        }
    }
    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (mpos, own_id) in &markers {
        let mpos = *mpos;
        let next = ids.iter().find(|(apos, _)| *apos > mpos);
        let prose_intervenes = |apos: usize| {
            let k = text_positions.partition_point(|&t| t <= mpos);
            text_positions.get(k).is_some_and(|&t| t < apos)
        };
        let attributed = match (own_id, next) {
            (Some(id), _) => Some(id),
            (None, Some((apos, id))) if apos - mpos <= MARKER_ANCHOR_WINDOW && !prose_intervenes(*apos) => {
                Some(id)
            }
            _ => ids
                .iter()
                .rev()
                .find(|(apos, _)| *apos < mpos && mpos - apos <= MARKER_ANCHOR_WINDOW)
                .map(|(_, id)| id),
        };
        if let Some(id) = attributed.filter(|id| dotted_item(id)) {
            if seen.insert(id.clone()) {
                out.push(id.clone());
            }
        }
    }
    out
}

/// A construction's per-family independent-witness tallies (kept for the reporting comparison to PASS 20).
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
        for (u, b) in got {
            if seen.insert(u.clone()) {
                pages.push((u, b));
            }
        }
    }
    println!("total distinct pages: {}", pages.len());

    // ── machine-derived labeled families ────────────────────────────────────────────────────────────
    let attest = Attestation::discover(&pages);
    let tokens = prohibition_class_tokens();
    // PAGE-LEVEL prohibition (PASS 20's contaminated family — kept for comparison).
    let prohibition_page: HashSet<String> =
        pages.iter().filter(|(_, b)| attest.attests(b)).map(|(u, _)| u.clone()).collect();
    // ITEM-LEVEL deprecated SUBJECTS — the badge-contamination fix. Two clean sources:
    //   (i)  MDN frontmatter/text-run attestation whose marker is NOT the class route: the page IS the
    //        deprecated feature, subject = URL last segment. (attest.markers() route, whole-page-true.)
    //   (ii) rendered class badges attributed to their OWN dotted item — the item member name is the
    //        subject, never the page. A stray sidebar badge anchors a DIFFERENT item (or none), so the
    //        page's own subject is not contaminated.
    // We separate the two attestation routes to measure the contamination cleanup precisely.
    let text_markers = attest.markers().to_vec();
    let attests_by_text = |body: &str| -> bool {
        if text_markers.is_empty() {
            return false;
        }
        let runs: HashSet<String> = runs_of(body).into_iter().collect();
        text_markers.iter().any(|m| runs.contains(m))
    };
    // Subject set — the machine's proven-deprecated subjects, keyed clean (item-level).
    let mut dep_subjects: HashSet<String> = HashSet::new();
    let mut item_attested_pages = 0usize;
    let mut text_attested_pages = 0usize;
    for (url, body) in &pages {
        if attests_by_text(body) {
            text_attested_pages += 1;
            let subj = url_subject(url);
            if !subj.is_empty() {
                dep_subjects.insert(subj);
            }
        }
        let items = attested_items(body, &tokens);
        if !items.is_empty() {
            item_attested_pages += 1;
        }
        for id in items {
            // The QUALIFIED subject keys only: the full dotted id and the `owner.member` tail pair. The BARE
            // last component (`open`, `is`, `name`) is deliberately NOT added — a bare common member name
            // collides with unrelated fillers and manufactures false deprecation witnesses (the item unit's
            // own "receiver-generic form must be corpus-unambiguous" covenant). A slot filler matches only
            // when it carries the receiver, so an ordinary `s.open()` never witnesses a deprecated `x.open`.
            let low = id.to_lowercase();
            dep_subjects.insert(low.clone());
            let parts: Vec<&str> = low.split('.').collect();
            if parts.len() >= 2 {
                dep_subjects.insert(parts[parts.len() - 2..].join("."));
            }
        }
    }
    // The PASS-20 contaminated subject basis, for the before/after comparison.
    let contaminated_subjects: HashSet<String> =
        prohibition_page.iter().map(|u| url_subject(u)).filter(|s| !s.is_empty()).collect();

    println!("\n── BADGE-CONTAMINATION FIX (Wall A part 2) ──");
    println!("  PASS-20 page-level prohibition family:      {} pages", prohibition_page.len());
    println!("    → text-run (MDN, whole-page-true):        {text_attested_pages} pages");
    println!("    → class-badge pages (any badge, contaminated): {}", prohibition_page.len() - text_attested_pages);
    println!("    → item-level badge pages (own dotted item):    {item_attested_pages} pages");
    println!("  proven-deprecated SUBJECTS:");
    println!("    PASS-20 contaminated (page URL segment):  {}", contaminated_subjects.len());
    println!("    PASS-21 item-level clean:                 {}", dep_subjects.len());

    let endorsement = endorsement_urls(&pages, &prohibition_page);

    // ── chrome, discovered once ─────────────────────────────────────────────────────────────────────
    let chrome = site_chrome(&pages);
    let stripped: Vec<(String, String)> =
        pages.iter().map(|(u, b)| (u.clone(), chrome.strip(u, b))).collect();

    // ── RUNG 1: mine UNSTRIPPED with per-page occupancy, and mine STRIPPED, to find eaten scaffolds ──
    struct Shape {
        pages: HashSet<String>,
        fillers: HashSet<String>,
        slot_fillers: Vec<HashSet<String>>,
        // primary-slot filler → the set of DISTINCT pages carrying that filler. The invariance test: if one
        // filler covers nearly every page, the slot is INVARIANT (chrome/boilerplate); if fillers spread
        // one-per-page, the slot VARIES (a real scaffold) — the owner's "chrome's content is invariant".
        filler_pages: HashMap<String, HashSet<String>>,
        witnesses: Witnesses,
        example: String,
        // per-page slotA subject (for the two-slot instance witness) + slotB fillers of THAT page
        instances: Vec<(String, Vec<String>)>,
    }
    let mine = |bodies: &[(String, String)]| -> HashMap<String, Shape> {
        let mut m: HashMap<String, Shape> = HashMap::new();
        for (url, body) in bodies {
            let text = flatten(body);
            for sent in sentences(&text) {
                let Some((shape, fillers)) = shape_and_fillers(&sent) else { continue };
                let e = m.entry(shape.clone()).or_insert_with(|| Shape {
                    pages: HashSet::new(),
                    fillers: HashSet::new(),
                    slot_fillers: Vec::new(),
                    filler_pages: HashMap::new(),
                    witnesses: Witnesses::default(),
                    example: render_example(&sent),
                    instances: Vec::new(),
                });
                e.pages.insert(url.clone());
                e.filler_pages.entry(fillers[0].to_lowercase()).or_default().insert(url.clone());
                for (i, f) in fillers.iter().enumerate() {
                    e.fillers.insert(f.clone());
                    if e.slot_fillers.len() <= i {
                        e.slot_fillers.push(HashSet::new());
                    }
                    e.slot_fillers[i].insert(f.clone());
                }
                let subj = fillers[0].to_lowercase();
                if prohibition_page.contains(url) {
                    e.witnesses.prohibition.insert(subj.clone());
                } else if endorsement.contains(url) {
                    e.witnesses.endorsement.insert(subj.clone());
                } else {
                    e.witnesses.neutral.insert(subj.clone());
                }
                e.instances.push((fillers[0].clone(), fillers.clone()));
            }
        }
        m
    };
    let unstripped = mine(&pages);
    let stripped_m = mine(&stripped);

    let is_construction = |s: &Shape| s.pages.len() >= SUPPORT_FLOOR && s.fillers.len() >= 2;
    // The SLOT-INVARIANCE test (the owner's signal made structural). A construction's primary slot VARIES
    // across pages iff no single filler covers more than `SCAFFOLD_MODE_FRAC` of its pages: chrome and
    // rustdoc trait-boilerplate carry the SAME code on every page (mode → 1.0), a real scaffold's code is
    // page-specific (mode low). This is exactly "chrome's content is invariant; a slot-varying run is not
    // chrome" — computed with zero meaning geometry.
    let mode_frac = |s: &Shape| -> f64 {
        let pages = s.pages.len().max(1) as f64;
        let mode = s.filler_pages.values().map(HashSet::len).max().unwrap_or(0) as f64;
        mode / pages
    };
    let is_scaffold = |s: &Shape| is_construction(s) && mode_frac(s) <= SCAFFOLD_MODE_FRAC;

    let u_set: HashSet<&String> =
        unstripped.iter().filter(|(_, s)| is_construction(s)).map(|(k, _)| k).collect();
    let s_set: HashSet<&String> =
        stripped_m.iter().filter(|(_, s)| is_construction(s)).map(|(k, _)| k).collect();

    // The co-resident design: mine on the UNSTRIPPED bodies (scaffolds intact) and keep every construction
    // whose slot VARIES (a genuine scaffold). The strip comparison below is REPORTING only — how many the
    // naive PASS-20 strip would have eaten, and how many of those were real scaffolds it wrongly lost.
    let mut final_set: Vec<&String> =
        u_set.iter().filter(|k| is_scaffold(&unstripped[**k])).copied().collect();
    final_set.sort_by_key(|k| std::cmp::Reverse(unstripped[*k].pages.len()));

    // Eaten by chrome = in the unstripped construction set, gone from the stripped set.
    let eaten: Vec<&String> = u_set.iter().filter(|k| !s_set.contains(**k)).copied().collect();
    // Of the eaten, the genuine scaffolds RESCUED by the slot-variance test; the rest were invariant menus.
    let rescued: Vec<&String> =
        eaten.iter().filter(|k| is_scaffold(&unstripped[**k])).copied().collect();
    let nav_junk: usize = eaten.len() - rescued.len();

    println!("\n── RUNG 1: SLOT-AWARE SCAFFOLD/CHROME DISAMBIGUATION (Wall A) ──");
    println!("  (scaffold = construction whose primary slot varies: mode page-fraction ≤ {SCAFFOLD_MODE_FRAC})");
    println!("  constructions mined UNSTRIPPED:            {}", u_set.len());
    println!("  constructions surviving the naive STRIP:   {}", s_set.len());
    println!("  eaten by the naive strip:                  {}", eaten.len());
    println!("    → rescued as VARYING-slot scaffolds:     {}", rescued.len());
    println!("    → left stripped as INVARIANT-slot menus:  {nav_junk}");
    println!("  FINAL co-resident scaffold set (all varying-slot constructions): {}", final_set.len());

    let show = |k: &String| {
        let s = &unstripped[k];
        let disp: String = k.chars().take(60).collect();
        println!(
            "    pages={:>4} fillers={:>4} modeFrac={:>4.2}  {disp}",
            s.pages.len(),
            s.fillers.len(),
            mode_frac(s)
        );
        let ex: String = s.example.chars().take(90).collect();
        println!("          e.g. {ex}");
    };
    // Show the rescued scaffolds (the register constructions PASS 20 lost).
    println!("\n  RESCUED SCAFFOLDS (register constructions recovered from the strip) ");
    let mut resc_sorted = rescued.clone();
    resc_sorted.sort_by_key(|k| std::cmp::Reverse(unstripped[*k].pages.len()));
    for k in resc_sorted.iter().take(40) {
        show(k);
    }
    // Show what stayed stripped as invariant menus/boilerplate (the discriminator's true negatives).
    println!("\n  LEFT STRIPPED as INVARIANT-slot menus/boilerplate (mode → high) ");
    let mut junk_sorted: Vec<&String> =
        eaten.iter().filter(|k| !is_scaffold(&unstripped[**k])).copied().collect();
    junk_sorted.sort_by_key(|k| std::cmp::Reverse(unstripped[*k].pages.len()));
    for k in junk_sorted.iter().take(12) {
        show(k);
    }

    // ── RUNG 2: INSTANCE-LEVEL WITNESSES (Wall B) ───────────────────────────────────────────────────
    // For every construction in the FINAL co-resident set, cross-reference its OWN instances against the
    // machine's proven-deprecated SUBJECT set (item-level, cleaned). A single-slot construction whose slot
    // filler names a deprecated subject witnesses deprecation-direction; a two-slot construction with slotA
    // deprecated + slotB clean witnesses supersession/remedy. Independence = DISTINCT subject.
    // EXACT membership only — the normalized filler, or its qualified `owner.member` form, must EQUAL a
    // proven-deprecated subject. A lossy last-component tail fallback (`Object.is` → `is`) collides with
    // common method names and MANUFACTURES witnesses (measured: it falsely "proved" `polyfill of ⟨⟩ in ⟨⟩`),
    // so it is rejected — an instance witnesses deprecation only when it names the deprecated subject itself.
    let is_dep = |f: &str| -> bool {
        let low = f.to_lowercase();
        let bare: String = low
            .trim_end_matches("()")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
            .collect();
        if bare.len() < 2 {
            return false;
        }
        // exact bare name, or the `owner.member` tail pair (the qualified form the item unit proposes).
        if dep_subjects.contains(&bare) {
            return true;
        }
        let parts: Vec<&str> = bare.split('.').collect();
        parts.len() >= 2 && dep_subjects.contains(&parts[parts.len() - 2..].join("."))
    };

    println!("\n── RUNG 2: INSTANCE-LEVEL WITNESSES vs the proven-deprecated subject set ──");
    println!("  (proven-deprecated subjects, item-level: {})", dep_subjects.len());

    // Single-slot deprecation-direction witnesses.
    #[derive(Clone)]
    struct Verdict {
        shape: String,
        pages: usize,
        two_slot: bool,
        dep_a: usize,        // distinct slotA subjects that are proven-deprecated (independent witnesses)
        clean_b: usize,      // distinct slotB fillers that are NOT deprecated (remedy witnesses)
        dep_b: usize,        // distinct slotB deprecated (contradiction to supersession direction)
        example: String,
    }
    let mut verdicts: Vec<Verdict> = Vec::new();
    for k in &final_set {
        let s = &unstripped[*k];
        let two_slot = s.slot_fillers.len() >= 2;
        // slotA deprecated subjects (distinct) — the independent deprecation-direction witnesses.
        let dep_a = s.slot_fillers.first().map(|set| set.iter().filter(|f| is_dep(f)).count()).unwrap_or(0);
        let (clean_b, dep_b) = if two_slot {
            let b = &s.slot_fillers[1];
            (b.iter().filter(|f| !is_dep(f)).count(), b.iter().filter(|f| is_dep(f)).count())
        } else {
            (0, 0)
        };
        verdicts.push(Verdict {
            shape: (*k).clone(),
            pages: s.pages.len(),
            two_slot,
            dep_a,
            clean_b,
            dep_b,
            example: s.example.clone(),
        });
    }
    // The supersession/deprecation candidates: those with any deprecated slotA subject.
    let mut supers: Vec<&Verdict> = verdicts.iter().filter(|v| v.dep_a > 0).collect();
    supers.sort_by_key(|v| std::cmp::Reverse(v.dep_a));
    println!("\n  DEPRECATION-DIRECTION CONSTRUCTIONS (slotA fillers naming proven-deprecated subjects):");
    println!("  {:>6} {:>6} {:>6} {:>6}  {:<7}  scaffold", "pages", "depA", "cleanB", "depB", "2slot");
    let mut proven_any = false;
    for v in supers.iter().take(30) {
        let proven = v.dep_a >= REQUIRED_WITNESSES && (!v.two_slot || v.dep_b == 0);
        if proven {
            proven_any = true;
        }
        let mark = if proven { " ✔PROVEN" } else { "" };
        let disp: String = v.shape.chars().take(52).collect();
        println!(
            "  {:>6} {:>6} {:>6} {:>6}  {:<7}  {disp}{mark}",
            v.pages,
            v.dep_a,
            v.clean_b,
            v.dep_b,
            v.two_slot
        );
        let ex: String = v.example.chars().take(90).collect();
        println!("          e.g. {ex}");
    }
    if !proven_any {
        println!("\n  NO construction reaches ≥{REQUIRED_WITNESSES} independent deprecated-subject witnesses.");
        println!("  → the register stays MINED-UNPROVEN (the PASS-12 discovery-starved wall; no floor relitigating).");
    }

    // ── NAMED REGISTER CONSTRUCTIONS — the exact scaffolds PASS 20 lost, with recovery + witness count ──
    // Each is located by a distinctive substring of its scaffold (reporting only — the mining/witness logic
    // above used no word list). Reports: is it in the FINAL co-resident scaffold set (recovered from the
    // strip)? its pages/modeFrac, and its instance-level deprecated-subject witness count.
    println!("\n── NAMED REGISTER CONSTRUCTIONS (the PASS-20-lost scaffolds) — recovery + witnesses ──");
    let named: &[(&str, &[&str])] = &[
        ("supersession: replace X with Y", &["replace", "with"]),
        ("supersession: use the X css property instead", &["css property instead"]),
        ("supersession: instead of using X (document.write)", &["instead of using", "dom"]),
        ("supersession: consider, instead, using X", &["consider", "instead", "using"]),
        ("prohibition: you should not compare X", &["should not compare"]),
        ("deprecation: last version of python that provided X", &["last version", "provided"]),
    ];
    let final_set_hs: HashSet<&String> = final_set.iter().copied().collect();
    for (label, needles) in named {
        let hit = unstripped.iter().find(|(k, s)| {
            is_construction(s) && needles.iter().all(|n| k.contains(n))
        });
        match hit {
            None => println!("  [{label}] NOT MINED"),
            Some((k, s)) => {
                let recovered = final_set_hs.contains(k);
                let dep_a = s.slot_fillers.first().map(|set| set.iter().filter(|f| is_dep(f)).count()).unwrap_or(0);
                let clean_b = if s.slot_fillers.len() >= 2 {
                    s.slot_fillers[1].iter().filter(|f| !is_dep(f)).count()
                } else {
                    0
                };
                println!(
                    "  [{label}]\n    pages={} fillers={} modeFrac={:.2} recovered={} | witnesses: depA={} cleanB={} {}",
                    s.pages.len(),
                    s.fillers.len(),
                    mode_frac(s),
                    recovered,
                    dep_a,
                    clean_b,
                    if dep_a >= REQUIRED_WITNESSES { "≥15 PROVEN" } else { "<15 MINED-UNPROVEN" }
                );
                println!("    e.g. {}", s.example.chars().take(96).collect::<String>());
            }
        }
    }

    // ── RESIDUAL (ii) EXPERIMENT — admit WHOLE-PAGE removal subjects, measure what newly proves ──────
    // The python removed-module pages carry `class="deprecated-removed"` — a COMPOUND author token the
    // faculty misses (it knows only the exact `deprecated`). Hyphen-splitting the compound recovers the
    // `deprecated` token; the marker sits at the TOP of the module page (the page's OWN whole-page subject,
    // no dotted sub-item), so admitting the page URL-subject is the item unit's whole-page case, not a stray
    // sidebar badge. This is a MEASUREMENT: does the python-removal construction cross ≥15, and — the
    // Wall-B-in-disguise test — do OTHER sentences on those same pages FALSELY co-prove?
    let prohibit_tok: HashSet<String> = tokens.iter().cloned().collect();
    let mut removal_pages: HashSet<String> = HashSet::new();
    for (url, body) in &pages {
        // a whole-page marker: a class token (split on whitespace AND hyphen) equal to a prohibition token,
        // on a page that has NO dotted-item badge (so the marker is the page's own module-level status).
        if !attested_items(body, &tokens).is_empty() {
            continue;
        }
        let lb = body.as_bytes();
        let needle = b"class=";
        let mut i = 0usize;
        let mut hit = false;
        while i + needle.len() < lb.len() {
            if &lb[i..i + needle.len()] == needle {
                let q = lb[i + needle.len()];
                if q == b'"' || q == b'\'' {
                    let start = i + needle.len() + 1;
                    if let Some(rel) = lb[start..].iter().position(|&b| b == q) {
                        let val = &body[start..start + rel].to_lowercase();
                        // TIGHT: the module-REMOVAL compound only — a class token carrying BOTH the
                        // prohibition token AND `removed` (`deprecated-removed`). Removal is the strongest
                        // prohibition status; the compound is the whole-module marker, distinct from an
                        // inline `versionmodified deprecated` per-method note. This is the precise typography
                        // the faculty would learn, not a loose `deprecated` substring.
                        // LOOSE (env `LOOSE_REMOVAL`): any class token containing the prohibition token —
                        // MEASURED contamination (2812 pages, junk co-provers). TIGHT (default): the
                        // module-removal compound only, carrying BOTH the prohibition token AND `removed`.
                        let toks: HashSet<&str> = val.split([' ', '-', '\t']).collect();
                        let loose = std::env::var("LOOSE_REMOVAL").is_ok();
                        let matched = if loose {
                            toks.iter().any(|t| prohibit_tok.contains(*t))
                        } else {
                            toks.contains("removed") && toks.iter().any(|t| prohibit_tok.contains(*t))
                        };
                        if matched {
                            hit = true;
                            break;
                        }
                        i = start + rel + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }
        if hit {
            removal_pages.insert(url.clone());
        }
    }
    let mut dep_plus = dep_subjects.clone();
    for u in &removal_pages {
        // the module subject is the page basename WITHOUT its file extension (`aifc.html` → `aifc`) — the
        // form the construction's slot filler carries.
        let mut s = url_subject(u);
        for ext in [".html", ".htm", ".php"] {
            if let Some(base) = s.strip_suffix(ext) {
                s = base.to_string();
            }
        }
        if !s.is_empty() {
            dep_plus.insert(s);
        }
    }
    let is_dep_plus = |f: &str| -> bool {
        let low = f.to_lowercase();
        let bare: String = low
            .trim_end_matches("()")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
            .collect();
        if bare.len() < 2 {
            return false;
        }
        if dep_plus.contains(&bare) {
            return true;
        }
        let parts: Vec<&str> = bare.split('.').collect();
        parts.len() >= 2 && dep_plus.contains(&parts[parts.len() - 2..].join("."))
    };
    println!("\n── RESIDUAL (ii) EXPERIMENT: whole-page removal subjects admitted ──");
    println!("  whole-page removal pages (dotless module-level badge): {}", removal_pages.len());
    println!("  proven-deprecated subjects with removal admitted:      {}", dep_plus.len());
    let mut newly: Vec<(&String, usize, usize)> = Vec::new();
    for k in &final_set {
        let s = &unstripped[*k];
        let dep_a = s.slot_fillers.first().map(|set| set.iter().filter(|f| is_dep_plus(f)).count()).unwrap_or(0);
        let dep_b = if s.slot_fillers.len() >= 2 {
            s.slot_fillers[1].iter().filter(|f| is_dep_plus(f)).count()
        } else {
            0
        };
        if dep_a >= REQUIRED_WITNESSES {
            newly.push((k, dep_a, dep_b));
        }
    }
    newly.sort_by_key(|(_, a, _)| std::cmp::Reverse(*a));
    println!("  constructions reaching ≥{REQUIRED_WITNESSES} deprecated-slotA witnesses (0 contradiction = clean):");
    for (k, a, b) in &newly {
        let clean = if *b == 0 { "✔ 0-contradiction" } else { "✗ CONTRADICTED (slotB also deprecated)" };
        let disp: String = k.chars().take(56).collect();
        println!("    depA={a:>3} depB={b:>3}  [{clean}]  {disp}");
        println!("          e.g. {}", unstripped[*k].example.chars().take(84).collect::<String>());
    }
    if newly.is_empty() {
        println!("    (none — even with removal subjects admitted, no construction crosses the floor)");
    }

    // The named document.write classic, explicitly located and reported.
    println!("\n  DOCUMENT.WRITE classic (its own governing construction), honest witness count:");
    for v in verdicts.iter().filter(|v| v.shape.contains("instead of using") && v.shape.contains("dom")) {
        println!(
            "    slotA deprecated-subject witnesses = {}/{} (need {REQUIRED_WITNESSES}); slotB clean {} dep {}",
            v.dep_a,
            unstripped[&v.shape].slot_fillers.first().map(HashSet::len).unwrap_or(0),
            v.clean_b,
            v.dep_b
        );
        println!("    e.g. {}", v.example.chars().take(110).collect::<String>());
    }
}

/// A page's subject key: the last URL path segment (case-folded, fragment/trailing-slash trimmed) — the
/// same key `lint_attest::subject_of_url` uses.
fn url_subject(url: &str) -> String {
    url.split('#')
        .next()
        .unwrap_or(url)
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

fn render_example(sentence: &str) -> String {
    sentence.replace(SLOT_OPEN, "⟨").replace(SLOT_CLOSE, "⟩")
}
