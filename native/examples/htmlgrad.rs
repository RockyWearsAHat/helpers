//! THROWAWAY PROBE (untracked). Measure the HTML layer's STRUCTURAL construct-keying
//! (`lint_html_layer`) against real MDN docs: does keying each witness by its page-of-origin subject
//! (a) kill the cross-construct leak the flat pipeline suffered, and (b) let real per-construct truths
//! graduate through the frozen comparator + engine where the docs supply enough independent witnesses?
//!
//! Pipeline: decode the cached MDN crawl (HLM1 CRAWL container) → per element page, read its GOVERNING
//! witnesses via `lint_html_layer::page_witnesses` (furniture sections + code + sibling-tag sentences
//! excluded) keyed with the element as structural subject → truth = the construct's own definition
//! sentence, foil = a sibling construct's definition → `graduate_construct` gates by subject key and
//! runs the frozen `lint_ism::graduate`. Then the LEAK test: try to corroborate `<strong>`'s truth
//! with `<em>`/`<b>`-subject witnesses and confirm the key gate admits zero.
use helpers_native::lint_char;
use helpers_native::lint_codec::{self, Dec};
use helpers_native::lint_corroborate::consistency;
use helpers_native::lint_english::{self, English};
use helpers_native::lint_html_layer::{discard_chrome, graduate_construct, page_witnesses, KeyedWitness};
use helpers_native::lint_ism::{classify, Candidate, Corroboration};
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

    // Read every element page into subject-keyed governing witnesses.
    let home = std::env::var("HOME").unwrap();
    let dir = format!("{home}/.cache/helpers/lint-index/crawls");
    let mut pool: Vec<KeyedWitness> = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if !name.starts_with("developer-mozilla-org") || !name.ends_with(".bin") {
            continue;
        }
        let Some(pages) = decode_crawl(&std::fs::read(&path).unwrap()) else { continue };
        for pg in pages {
            let Some(el) = element_of(&pg.url) else { continue };
            pool.extend(page_witnesses(&el, &pg.body));
        }
    }
    let before = pool.len();
    let pool = discard_chrome(pool);
    eprintln!("governing witnesses: {before} raw -> {} after cross-page chrome discard", pool.len());
    let mut witnesses_by_element: BTreeMap<String, Vec<KeyedWitness>> = BTreeMap::new();
    for w in pool {
        witnesses_by_element.entry(w.subject.clone()).or_default().push(w);
    }
    eprintln!("element pages with governing witnesses: {}", witnesses_by_element.len());

    // Doc-grounded truth = a construct's own definition sentence (first governing witness of its page).
    let definition = |el: &str| -> Option<String> {
        witnesses_by_element.get(el).and_then(|w| w.first()).map(|w| w.sentence.clone())
    };

    // (construct, sibling foil construct)
    let pairs = [("strong", "b"), ("em", "b"), ("b", "strong"), ("mark", "strong"), ("i", "em")];

    eprintln!("\n======== GOVERNING WITNESS COUNTS (subject-keyed) ========");
    for (c, _) in &pairs {
        let n = witnesses_by_element.get(*c).map(|w| w.len()).unwrap_or(0);
        eprintln!("  <{c:<7}> governing witnesses: {n}");
    }

    eprintln!("\n======== A. GRADUATE PER CONSTRUCT (subject-keyed, frozen engine) ========");
    for (c, foil_c) in &pairs {
        let (Some(truth), Some(foil)) = (definition(c), definition(foil_c)) else {
            eprintln!("  <{c}>: missing definition (self or foil) — skip");
            continue;
        };
        let ws = witnesses_by_element.get(*c).cloned().unwrap_or_default();
        let (verdict, offered) = graduate_construct(m, en, c, &truth, &foil, &ws);
        // Count raw corroborations among the subject-keyed stream for context.
        let cand = Candidate::new(&truth, &foil);
        let raw: Vec<&String> = ws
            .iter()
            .filter(|w| classify(m, en, &cand, &w.sentence) == Corroboration::Corroborates)
            .map(|w| &w.sentence)
            .collect();
        eprintln!("\n  <{c}> (foil <{foil_c}>): {verdict:?}");
        eprintln!("    truth: {truth}");
        eprintln!("    foil : {foil}");
        eprintln!("    subject-keyed witnesses offered: {offered}   raw corroborations: {}", raw.len());
        for w in raw.iter().take(8) {
            let d = consistency(m, en, &truth, w).map(|c| c.reference_distance).unwrap_or(f64::NAN);
            eprintln!("      [{d:.2}] {w}");
        }
    }

    eprintln!("\n======== B. LEAK TEST: <strong> truth vs FOREIGN-subject witnesses ========");
    let Some(truth) = definition("strong") else {
        eprintln!("  no <strong> definition");
        return;
    };
    let foil = definition("b").unwrap_or_default();
    // What the OLD flat pipeline would count: run the comparator directly on foreign-page sentences.
    let cand = Candidate::new(&truth, &foil);
    for foreign in ["em", "b", "i", "mark", "small", "code", "cite", "span"] {
        let Some(ws) = witnesses_by_element.get(foreign) else { continue };
        let would_leak = ws
            .iter()
            .filter(|w| classify(m, en, &cand, &w.sentence) == Corroboration::Corroborates)
            .count();
        // What the GATE actually admits: graduate_construct with subject="strong" over foreign witnesses.
        let (_v, admitted) = graduate_construct(m, en, "strong", &truth, &foil, ws);
        eprintln!(
            "  <{foreign:<6}> sentences: comparator-would-corroborate={would_leak:<3}  subject-gate-admits={admitted}"
        );
    }
    eprintln!("\n(admitted must be 0 for every foreign construct — the leak is gone at the subject-key gate.)");
}
