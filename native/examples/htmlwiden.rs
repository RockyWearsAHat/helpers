//! THROWAWAY PROBE (untracked). Measure CROSS-SOURCE widening of the HTML layer's subject-keyed
//! witness pool: MDN element pages + WHATWG text-level-semantics spec + W3Schools tag pages, all keyed
//! by the SAME structural page-of-origin subject. Questions: (1) does the leak stay 0 after widening
//! (foreign-subject witnesses still admitted 0)? (2) how many INDEPENDENT corroborations does each
//! construct reach cross-source, and which graduate at the frozen ≥15 bar?
use helpers_native::lint_char;
use helpers_native::lint_codec::{self, Dec};
use helpers_native::lint_english::{self};
use helpers_native::lint_html_layer::{
    discard_chrome, graduate_construct, page_witnesses, w3schools_witnesses, whatwg_witnesses,
    KeyedWitness,
};
use helpers_native::lint_ism::{classify, Candidate, Corroboration, Verdict};
use std::collections::BTreeMap;

struct Page {
    url: String,
    body: String,
}

fn decode_crawl(bytes: &[u8]) -> Option<Vec<Page>> {
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
        pages.push(Page { url, body });
    }
    Some(pages)
}

fn element_of(url: &str) -> Option<String> {
    let lower = url.to_lowercase();
    let idx = lower.find("/html/element/").or_else(|| lower.find("/html/reference/elements/"))?;
    let name = url[idx..].rsplit('/').find(|s| !s.is_empty())?;
    (!name.is_empty() && !name.contains('#')).then(|| name.to_lowercase())
}

fn main() {
    let (Some(br), Some(en)) = (lint_char::brain(), lint_english::brain()) else {
        eprintln!("no frozen brains on disk");
        return;
    };
    let m = br.meanings();
    let home = std::env::var("HOME").unwrap();
    let dir = format!("{home}/.cache/helpers/lint-index/crawls");

    let mut by_source: BTreeMap<&str, Vec<KeyedWitness>> = BTreeMap::new();
    // MDN — decode every developer-mozilla-org CRAWL container, key each page by its URL element.
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name.starts_with("developer-mozilla-org") && name.ends_with(".bin") {
            if let Some(pages) = decode_crawl(&std::fs::read(&path).unwrap()) {
                for pg in pages {
                    if let Some(el) = element_of(&pg.url) {
                        by_source.entry("mdn").or_default().extend(page_witnesses(&el, &pg.body));
                    }
                }
            }
        }
    }
    // WHATWG — one page, all text-level elements, keyed by section anchor.
    let whatwg = format!("{dir}/whatwg-text-level-semantics.html");
    if let Ok(html) = std::fs::read_to_string(&whatwg) {
        by_source.entry("whatwg").or_default().extend(whatwg_witnesses(&html));
    }
    // W3Schools — per-tag pages, keyed by tag name from the filename.
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if let Some(tag) = name.strip_prefix("w3schools-tag_").and_then(|s| s.strip_suffix(".html")) {
            let html = std::fs::read_to_string(&path).unwrap();
            by_source.entry("w3schools").or_default().extend(w3schools_witnesses(tag, &html));
        }
    }

    for (src, ws) in &by_source {
        eprintln!("source {src:<10}: {} governing witnesses", ws.len());
    }

    // Combined pool, cross-page/cross-source chrome discarded, grouped by subject.
    let mut pool: Vec<KeyedWitness> = by_source.values().flatten().cloned().collect();
    let before = pool.len();
    pool = discard_chrome(pool);
    eprintln!("\ncombined: {before} raw -> {} after cross-subject chrome discard", pool.len());
    let mut by_el: BTreeMap<String, Vec<KeyedWitness>> = BTreeMap::new();
    for w in pool {
        by_el.entry(w.subject.clone()).or_default().push(w);
    }

    // Definition truth = the FIRST governing sentence of a construct after chrome discard (the MDN
    // definition sentence — MDN sorts first in the pool, and its Baseline banner is gone as chrome).
    let definition =
        |el: &str| -> Option<String> { by_el.get(el).and_then(|w| w.first()).map(|w| w.sentence.clone()) };

    let pairs = [("strong", "b"), ("em", "b"), ("b", "strong"), ("mark", "strong"), ("i", "em"), ("small", "strong")];

    eprintln!("\n======== WIDENED WITNESS COUNTS (subject-keyed, cross-source) ========");
    for (c, _) in &pairs {
        let total = by_el.get(*c).map(|w| w.len()).unwrap_or(0);
        let per: Vec<String> = by_source
            .iter()
            .map(|(s, ws)| format!("{s} {}", ws.iter().filter(|w| w.subject == *c).count()))
            .collect();
        eprintln!("  <{c:<7}> total {total:<3} [{}]", per.join(", "));
    }

    eprintln!("\n======== A. GRADUATE PER CONSTRUCT (widened, frozen engine) ========");
    for (c, foil_c) in &pairs {
        let (Some(truth), Some(foil)) = (definition(c), definition(foil_c)) else {
            eprintln!("  <{c}>: missing definition — skip");
            continue;
        };
        let ws = by_el.get(*c).cloned().unwrap_or_default();
        let (verdict, offered) = graduate_construct(m, en, c, &truth, &foil, &ws);
        let cand = Candidate::new(&truth, &foil);
        let corr: Vec<&KeyedWitness> =
            ws.iter().filter(|w| classify(m, en, &cand, &w.sentence) == Corroboration::Corroborates).collect();
        let contra: Vec<&KeyedWitness> =
            ws.iter().filter(|w| classify(m, en, &cand, &w.sentence) == Corroboration::Contradicts).collect();
        let tag = if verdict == Verdict::Proven { "GRADUATES" } else { "unproven" };
        let self_d = helpers_native::lint_corroborate::consistency(m, en, &truth, &truth).map(|c| c.reference_distance);
        let foil_d = helpers_native::lint_corroborate::consistency(m, en, &truth, &foil).map(|c| c.reference_distance);
        eprintln!("\n  <{c}> (foil <{foil_c}>): {tag}  {verdict:?}");
        eprintln!("    truth: {truth}");
        eprintln!("    foil : {foil}");
        eprintln!("    self_dist={self_d:?}  foil_dist={foil_d:?}");
        eprintln!("    offered {offered}  raw-corroborations {}  raw-contradictions {}", corr.len(), contra.len());
        for w in contra.iter().take(4) {
            eprintln!("    CONTRA [{}] {}", w.subject, w.sentence);
        }
    }

    eprintln!("\n======== B. LEAK TEST (widened pool): <strong> truth vs FOREIGN subjects ========");
    let truth = definition("strong").unwrap();
    let foil = definition("b").unwrap_or_default();
    let cand = Candidate::new(&truth, &foil);
    let mut total_admitted = 0usize;
    for foreign in ["em", "b", "i", "mark", "small", "code", "cite", "span", "dfn"] {
        let Some(ws) = by_el.get(foreign) else { continue };
        let would = ws.iter().filter(|w| classify(m, en, &cand, &w.sentence) == Corroboration::Corroborates).count();
        let (_v, admitted) = graduate_construct(m, en, "strong", &truth, &foil, ws);
        total_admitted += admitted;
        eprintln!("  <{foreign:<6}> comparator-would={would:<3}  subject-gate-admits={admitted}");
    }
    eprintln!("\n  TOTAL foreign-subject witnesses admitted to <strong>: {total_admitted}  (must be 0)");
}
