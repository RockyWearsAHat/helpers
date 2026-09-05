//! THROWAWAY MEASUREMENT HARNESS (untracked) — Item 3b step 2, the REFEREE GRADING.
//!
//! Across the whole cached web-stack corpus, measure page-by-page AGREEMENT between the LEARNED
//! reader (`lint_graph::read_page` — the char-substrate unit former that learns element registers
//! by exposure) and the HAND anatomy (`lint_lang_layer::read_doc_page` + `governing_sentences`,
//! the URL/notecard/anchor structural paths). Two axes, per the mandate:
//!   (a) governing-prose extraction — do the learned reader's prose units carry the hand path's
//!       governing sentences, and how much CODE welds into the learned prose (the named defect)?
//!   (b) construct-page subjects + page roles — does the learned reader offer ANY equivalent to the
//!       hand path's prohibition/deprecation attestation + subject? (It does not yet; measured.)
//! Reported per SOURCE (MDN reference, MDN API, W3Schools tutorial, ESLint).
//! Run: `cargo run --release --features crawl --example reader_grade`
use helpers_native::lint_char;
use helpers_native::lint_codec::{self, Dec};
use helpers_native::lint_english;
use helpers_native::lint_graph::{read_page, site_chrome};
use helpers_native::lint_lang_layer::read_doc_page;
use helpers_native::lint_trace::Bridge;

fn decode_crawl(bytes: &[u8]) -> Option<Vec<(String, String)>> {
    let (_stamp, mut d) = Dec::open(bytes, lint_codec::kind::CRAWL)?;
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

fn all_pages() -> Vec<(String, String)> {
    let home = std::env::var("HOME").unwrap();
    let dir = format!("{home}/.cache/helpers/lint-index/crawls");
    let mut pages = Vec::new();
    let Ok(rd) = std::fs::read_dir(&dir) else { return pages };
    for e in rd.flatten() {
        let nm = e.file_name().to_string_lossy().to_string();
        if nm.ends_with(".bin")
            && (nm.starts_with("developer-mozilla-org")
                || nm.starts_with("eslint-org")
                || nm.starts_with("w3schools"))
        {
            if let Ok(b) = std::fs::read(e.path()) {
                if let Some(p) = decode_crawl(&b) {
                    pages.extend(p);
                }
            }
        }
    }
    pages
}

/// Replica of `doc_crawler::drop_script_style` (pub(crate)) — drop script/style + SEMANTIC chrome
/// element blocks by name, exactly what the live reader caller strips before `read_page`.
fn drop_chrome(html: &str) -> String {
    let mut h = html.to_string();
    for (open, close) in [
        ("<script", "</script>"),
        ("<style", "</style>"),
        ("<nav", "</nav>"),
        ("<header", "</header>"),
        ("<footer", "</footer>"),
        ("<aside", "</aside>"),
    ] {
        while let Some(s) = h.find(open) {
            if let Some(e) = h[s..].find(close) {
                h.replace_range(s..s + e + close.len(), " ");
            } else {
                break;
            }
        }
    }
    h
}

/// The reporting SOURCE bucket of a page — the owner's requested per-source breakdown.
fn source(url: &str) -> &'static str {
    let u = url.to_lowercase();
    if u.contains("w3schools") {
        "W3Schools"
    } else if u.contains("eslint.org") {
        "ESLint"
    } else if u.contains("developer.mozilla") && u.contains("/docs/web/api/") {
        "MDN API"
    } else if u.contains("developer.mozilla") && u.contains("/reference/") {
        "MDN reference"
    } else if u.contains("developer.mozilla") {
        "MDN other"
    } else {
        "misc"
    }
}

/// Content tokens (lowercased alnum runs) of a prose blob — the comparison atoms.
fn tokens(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2)
        .map(|w| w.to_lowercase())
        .collect()
}

/// Whether a whitespace token is CODE-shaped: it carries a non-alphanumeric symbol that is not a
/// sentence terminal — the welding proxy (a highlighter's `=`, `{`, `();`, `x.y`).
fn codey_token(t: &str) -> bool {
    t.chars().any(|c| !c.is_alphanumeric() && !matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | '\'' | '"' | '-'))
}

/// Fraction of a prose blob's whitespace tokens that are code-shaped — the welding ratio.
fn welding_ratio(prose: &str) -> f64 {
    let toks: Vec<&str> = prose.split_whitespace().collect();
    if toks.is_empty() {
        return 0.0;
    }
    toks.iter().filter(|t| codey_token(t)).count() as f64 / toks.len() as f64
}

#[derive(Default)]
struct Agg {
    pages: usize,
    prohib_pages: usize,
    sent_recall_sum: f64,
    sent_recall_n: usize,
    learn_weld_sum: f64,
    hand_weld_sum: f64,
    weld_n: usize,
    allpage_weld_sum: f64,
    learned_has_units: usize,
}

fn main() {
    let (Some(br), Some(en)) = (lint_char::brain(), lint_english::brain()) else {
        eprintln!("no frozen brains on disk");
        return;
    };
    let bridge = Bridge::new(br.meanings(), en);
    let pages = all_pages();
    eprintln!("corpus pages: {}  (structure roles learned: {})", pages.len(), !br.structure().is_empty());

    // LANDED cross-page-invariance chrome filter — the same `lint_graph::SiteChrome` the live module
    // workflow and brain curriculum now use. The learned reader is fed the CHROME-STRIPPED body; the
    // hand path stays on the original body as the ground-truth governing-sentence reference, so
    // sent-recall measures whether the strip removed any GOVERNING prose (it should not) while
    // learn-weld / allpg-weld measure the W3S collapse.
    let chrome = site_chrome(&pages);
    eprintln!("site chrome runs discovered: {}", chrome.len());

    use std::collections::BTreeMap;
    let mut by_src: BTreeMap<&'static str, Agg> = BTreeMap::new();
    let mut welding_example: Option<(String, String)> = None;

    for (url, body) in &pages {
        let src = source(url);
        let a = by_src.entry(src).or_default();
        a.pages += 1;

        // Match the LIVE caller (`doc_crawler::extract_sections_html_hinted`): semantic chrome
        // (`<nav>/<header>/<footer>/<aside>`, script/style) is dropped before the reader sees it,
        // AND the landed cross-page-invariance chrome (div-based menus W3S wraps non-semantically).
        let dropped = drop_chrome(&chrome.strip(url, body));
        let learned = read_page(&dropped, br);
        if !learned.is_empty() {
            a.learned_has_units += 1;
        }
        let learned_prose: String = learned.iter().map(|u| u.prose.as_str()).collect::<Vec<_>>().join(" \u{2022} ");

        // Welding on EVERY page (not just prohibition pages), so W3S tutorial pages — which the hand
        // path skips entirely — are measured. This is where the named `==`/`var` defect lives.
        a.allpage_weld_sum += welding_ratio(&learned_prose);
        if welding_example.is_none() && src == "W3Schools" && welding_ratio(&learned_prose) > 0.18 && !learned_prose.is_empty() {
            let snippet: String = learned_prose.chars().take(260).collect();
            welding_example = Some((url.clone(), snippet));
        }

        let attested: std::collections::HashSet<String> = std::collections::HashSet::new();
        let construction_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        let hand = read_doc_page(url, body, en, &bridge, &attested, &construction_map);
        if hand.prohibited {
            a.prohib_pages += 1;
            let hand_gov = hand.governing.join(" \u{2022} ");
            // Welding: compare code-token density of the two prose extractions on the same page.
            a.learn_weld_sum += welding_ratio(&learned_prose);
            a.hand_weld_sum += welding_ratio(&hand_gov);
            a.weld_n += 1;

            // Sentence recall: fraction of hand governing sentences whose content tokens are >=70%
            // covered by the learned reader's prose token set.
            let learned_set: std::collections::HashSet<String> = tokens(&learned_prose).into_iter().collect();
            let mut covered = 0usize;
            let mut total = 0usize;
            for s in &hand.governing {
                let st = tokens(s);
                if st.len() < 3 {
                    continue;
                }
                total += 1;
                let hit = st.iter().filter(|t| learned_set.contains(*t)).count();
                if hit * 10 >= st.len() * 7 {
                    covered += 1;
                }
            }
            if total > 0 {
                a.sent_recall_sum += covered as f64 / total as f64;
                a.sent_recall_n += 1;
            }

            // Capture one vivid welding example from a W3Schools page for the report.
            if welding_example.is_none() && src == "W3Schools" && welding_ratio(&learned_prose) > 0.15 {
                let snippet: String = learned_prose.chars().take(220).collect();
                welding_example = Some((url.clone(), snippet));
            }
        }
    }

    println!("\n== 3b REFEREE GRADING — learned reader vs hand anatomy, per source ==\n");
    println!(
        "{:<15} {:>7} {:>8} {:>10} {:>12} {:>11} {:>10} {:>11}",
        "source", "pages", "prohib", "learn≠∅", "sent-recall", "learn-weld", "hand-weld", "allpg-weld"
    );
    for (src, a) in &by_src {
        let recall = if a.sent_recall_n > 0 { a.sent_recall_sum / a.sent_recall_n as f64 } else { 0.0 };
        let lw = if a.weld_n > 0 { a.learn_weld_sum / a.weld_n as f64 } else { 0.0 };
        let hw = if a.weld_n > 0 { a.hand_weld_sum / a.weld_n as f64 } else { 0.0 };
        let apw = a.allpage_weld_sum / a.pages.max(1) as f64;
        println!(
            "{:<15} {:>7} {:>8} {:>9.0}% {:>11.1}% {:>10.1}% {:>9.1}% {:>10.1}%",
            src,
            a.pages,
            a.prohib_pages,
            100.0 * a.learned_has_units as f64 / a.pages.max(1) as f64,
            100.0 * recall,
            100.0 * lw,
            100.0 * hw,
            100.0 * apw,
        );
    }
    println!("\nsent-recall = mean fraction of the hand path's governing sentences whose content is");
    println!("  recovered in the learned reader's prose units (>=70% token coverage).");
    println!("learn-weld / hand-weld = mean fraction of code-shaped tokens in each path's prose");
    println!("  (higher = more code welded into prose; the named W3S defect).\n");

    if let Some((u, s)) = welding_example {
        println!("WELDING EXAMPLE (learned reader prose, W3Schools):\n  {u}\n  \"{s}...\"");
    }

    // ── PROTOTYPE (measurement only): would cross-page-INVARIANCE chrome discovery fix the weld? ──
    // For each source, collect every tag-separated text run and count on how many DISTINCT pages the
    // identical run appears. A run recurring across many of a site's pages is invariant boilerplate
    // (menus, breadcrumb, footer) — the north-star's chrome definition, learned from data, no element
    // names. Report the fraction of a source's text MASS (weighted by run length) that is invariant
    // (>=8 distinct pages). High on W3Schools, low on real content = the invariance filter is the fix.
    use std::collections::HashMap;
    println!("\n== INVARIANCE PROTOTYPE — text mass recurring across a site's pages (would-be chrome) ==\n");
    for want in ["MDN reference", "MDN API", "W3Schools"] {
        let mut run_pages: HashMap<String, std::collections::HashSet<usize>> = HashMap::new();
        let mut page_id = 0usize;
        for (url, body) in &pages {
            if source(url) != want {
                continue;
            }
            for run in text_runs(&drop_chrome(body)) {
                run_pages.entry(run).or_default().insert(page_id);
            }
            page_id += 1;
        }
        // Weight by OCCURRENCES (length × pages-seen-on): the fraction of text a reader actually
        // ENCOUNTERS across the site that is boilerplate — a menu run recurs on every page.
        let (mut total_mass, mut invariant_mass) = (0usize, 0usize);
        for (run, ps) in &run_pages {
            let mass = run.len() * ps.len();
            total_mass += mass;
            if ps.len() >= 8 {
                invariant_mass += mass;
            }
        }
        let pct = if total_mass > 0 { 100.0 * invariant_mass as f64 / total_mass as f64 } else { 0.0 };
        println!("  {want:<14} {page_id:>4} pages   invariant text mass = {pct:>5.1}%  (unique runs {})", run_pages.len());
    }
    println!("\n  (invariant = identical text run on >=8 distinct pages of the SAME site — chrome by the");
    println!("   north-star definition. A high % is removable boilerplate the learned reader currently welds.)");
}

/// Every tag-separated, whitespace-normalized text run of a body with >=2 words — the atoms a
/// cross-page-invariance chrome detector would key on. Pure typography (split on `<…>`), no names.
fn text_runs(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_tag = false;
    let mut cur = String::new();
    for c in body.chars() {
        match c {
            '<' => {
                in_tag = true;
                let t: String = cur.split_whitespace().collect::<Vec<_>>().join(" ");
                if t.split(' ').count() >= 2 && t.len() >= 6 {
                    out.push(t);
                }
                cur.clear();
            }
            '>' => in_tag = false,
            _ if !in_tag => cur.push(c),
            _ => {}
        }
    }
    out
}
