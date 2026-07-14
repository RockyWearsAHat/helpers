//! PASS 19 CONSTRUCTION-MINING FEASIBILITY MEASUREMENT (probe — the design's empirical premise).
//!
//! The owner's PASS 19 frame: a CONSTRUCTION is a sentence-level INVARIANT SCAFFOLD with VARIANT SLOTS —
//! exactly the pattern the page-level chrome detector (`lint_graph::site_chrome`) and the banner-level
//! attestation detector (`lint_attest`) already proved, GENERALISED from a whole-run atom to a
//! scaffold-with-slot atom. This harness asks the ONE empirical question the whole design rests on:
//!
//!   Do sentence scaffolds with a variant CODE slot actually FALL OUT of the real cached corpus by pure
//!   recurrence — "Use `{X}` instead of `{Y}`", "The `{X}` element is deprecated", "Never use `{X}`" —
//!   the way the 117-page deprecation banner and the Baseline endorsement banners fall out as invariant
//!   runs? If yes, constructions are minable STATES (no meaning geometry, sidestepping the passes-11/12/16
//!   flat-geometry wall the same way attestation did). If the recurrence is noise, the design has a wall.
//!
//! MECHANISM (structural only — NO templates, NO word lists in code):
//!   1. Read each page's prose with `<code>…</code>` interiors marked as SLOTS (the author's own code
//!      typography — the backtick construct, rendered), all other tags stripped to spaces.
//!   2. Split into sentences; a sentence with ≥1 slot is a candidate.
//!   3. SHAPE = the sentence with every slot filler replaced by `⟨⟩` (scaffold invariant, slot varies).
//!   4. Count each shape's DISTINCT-PAGE support and its DISTINCT fillers. A construction = a shape
//!      recurring on ≥ SUPPORT_FLOOR pages with ≥2 distinct fillers (a real variant slot, not a constant).
//!
//! Run: `cargo run --release --features crawl --example construct_mine`
use helpers_native::lint_codec::{self, Dec};
use std::collections::{HashMap, HashSet};

/// The same cross-page repetition-support floor `site_chrome`/`lint_attest` trust: a scaffold is a
/// construction only once at least this many INDEPENDENT pages testify it (never on one sighting).
const SUPPORT_FLOOR: usize = 8;

/// A scaffold below this many non-slot words is too thin to be a construction (a bare "the ⟨⟩ property").
const MIN_SCAFFOLD_WORDS: usize = 3;

/// Slot delimiters embedded in the flattened prose: `SLOT_OPEN filler SLOT_CLOSE` marks one code slot with
/// its captured filler inline, so sentence splitting and shape extraction keep the filler attached.
const SLOT_OPEN: char = '\u{1}';
const SLOT_CLOSE: char = '\u{2}';

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

/// Flatten one HTML body to prose where every `<code>…</code>` interior is an inline slot
/// `SLOT_OPEN filler SLOT_CLOSE` (whitespace collapsed inside the filler so it stays one token) and every
/// other tag becomes a space. Structural only — the author's own code typography marks the slot.
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

/// Split flattened prose into sentences on terminal punctuation followed by whitespace, plus newlines.
/// Slots ride inside sentences intact (a '.' INSIDE a slot region never splits).
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

/// The (shape, ordered fillers) of a sentence, or `None` when it carries no slot or is too thin. The SHAPE
/// replaces each slot region with `⟨⟩` and lowercases scaffold words; the fillers are captured in order.
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

    let mut pages_of: HashMap<String, HashSet<String>> = HashMap::new();
    let mut fillers_of: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut example_of: HashMap<String, String> = HashMap::new();
    let mut total_pages = 0usize;

    for name in corpora {
        let pages = load(name);
        if pages.is_empty() {
            continue;
        }
        println!("corpus {name}: {} pages", pages.len());
        total_pages += pages.len();
        for (url, body) in &pages {
            let text = flatten(body);
            let mut seen_on_page: HashSet<String> = HashSet::new();
            for sent in sentences(&text) {
                let Some((shape, fillers)) = shape_and_fillers(&sent) else { continue };
                pages_of.entry(shape.clone()).or_default().insert(url.clone());
                let fe = fillers_of.entry(shape.clone()).or_default();
                for f in fillers {
                    *fe.entry(f).or_default() += 1;
                }
                if seen_on_page.insert(shape.clone()) {
                    example_of.entry(shape.clone()).or_insert_with(|| render_example(&sent));
                }
            }
        }
    }
    println!("\ntotal pages read: {total_pages}");
    println!("distinct slot-bearing shapes seen: {}\n", pages_of.len());

    let mut constructions: Vec<(&String, usize, usize)> = pages_of
        .iter()
        .filter_map(|(shape, urls)| {
            let n_pages = urls.len();
            let n_fillers = fillers_of.get(shape).map(HashMap::len).unwrap_or(0);
            (n_pages >= SUPPORT_FLOOR && n_fillers >= 2).then_some((shape, n_pages, n_fillers))
        })
        .collect();
    constructions.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));

    println!(
        "###### MINED CONSTRUCTIONS — scaffold on ≥{SUPPORT_FLOOR} pages with ≥2 distinct slot fillers ######"
    );
    println!("  {} distinct constructions mined\n", constructions.len());
    let show = |shape: &String, n_pages: usize, n_fillers: usize| {
        let ex = example_of.get(shape).cloned().unwrap_or_default();
        let disp: String = shape.chars().take(74).collect();
        println!("  pages={n_pages:>4} fillers={n_fillers:>4}  {disp}");
        let exd: String = ex.chars().take(100).collect();
        println!("        e.g. {exd}");
    };
    for (shape, n_pages, n_fillers) in constructions.iter().take(PRINT_TOP) {
        show(shape, *n_pages, *n_fillers);
    }

    // REPORTING-ONLY grouping (like `super_probe`'s family list — DISPLAY, never mining logic). The mining
    // above used NO word list; here we merely BUCKET the already-mined scaffolds by a design cue so the
    // report shows the register families the classics need did fall out. The MEANING of each (prohibition /
    // supersession / description) is NOT decided here — that is the next rung's proof against the labeled
    // witness families + the frozen graduate/blind loop.
    println!("\n###### DESIGN-RELEVANT REGISTER FAMILIES (report grouping only) ######");
    let cues: &[(&str, &[&str])] = &[
        ("supersession / remedy", &["instead", "replace", "rather than", "in favor", "in favour"]),
        ("prohibition / caution", &["should not", "do not", "don't", "must not", "avoid", "never"]),
        ("deprecation / removal", &["deprecated", "obsolete", "no longer", "removed", "last version"]),
    ];
    for (label, words) in cues {
        let hits: Vec<_> = constructions
            .iter()
            .filter(|(s, _, _)| words.iter().any(|w| s.contains(w)))
            .collect();
        println!("\n  -- {label}: {} construction(s) --", hits.len());
        for (shape, n_pages, n_fillers) in hits.iter().take(12) {
            show(shape, *n_pages, *n_fillers);
        }
    }
}

/// How many mined constructions to print in the full ranked table (a reporting bound, not a mining one).
const PRINT_TOP: usize = 120;

/// Render a flattened sentence for display: slot regions become `⟨filler⟩`.
fn render_example(sentence: &str) -> String {
    sentence.replace(SLOT_OPEN, "⟨").replace(SLOT_CLOSE, "⟩")
}
