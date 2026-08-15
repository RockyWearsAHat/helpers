//! THROWAWAY MEASUREMENT HARNESS (untracked): PASS 28 collateral check. Runs the REAL
//! `read_doc_page` on tkinter.html / datamodel.html from the on-disk crawl and prints what the reader
//! yields (marked vs counter-attested groups, constructs head) — to find where the true-deprecation
//! siblings (Variable.trace_variable, codeobject.co_lnotab) leave the pipeline.
//! Run: `cargo run --release --features crawl --example notescope_page_probe`
use helpers_native::lint_codec::{self, Dec};

fn decode_crawl(bytes: &[u8]) -> Option<Vec<(String, String)>> {
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
    std::fs::read(format!("{dir}/{name}.bin")).ok().and_then(|b| decode_crawl(&b)).unwrap_or_default()
}

fn main() {
    let (Some(br), Some(en)) = (helpers_native::lint_char::brain(), helpers_native::lint_english::brain())
    else {
        eprintln!("no frozen brains on disk");
        return;
    };
    let bridge = helpers_native::lint_trace::Bridge::new(br.meanings(), en);
    let mut pages = load("python-library");
    pages.extend(load("python-docs"));
    for (url, body) in &pages {
        let page_name = url.rsplit('/').next().unwrap_or(url);
        if page_name != "tkinter.html" && page_name != "datamodel.html" {
            continue;
        }
        let attested: std::collections::HashSet<String> = std::iter::once(url.clone()).collect();
        let construction = std::collections::HashMap::new();
        let p = helpers_native::lint_lang_layer::read_doc_page(url, body, en, &bridge, &attested, &construction);
        println!("== {page_name} ({url})");
        println!("   prohibited={} attested={} marked={} counter={} constructs={} examples={}",
            p.prohibited, p.attested_deprecated, p.marked_deprecated.len(), p.counter_attested.len(),
            p.constructs.len(), p.example_code.len());
        for g in &p.counter_attested {
            println!("   counter: {}", g[0]);
        }
        let want = ["trace_variable", "co_lnotab", "attributes", "__loader__"];
        for g in &p.marked_deprecated {
            if want.iter().any(|w| g[0].contains(w)) {
                println!("   marked:  {}", g[0]);
            }
        }
        for c in p.constructs.iter().filter(|c| want.iter().any(|w| c.contains(w))) {
            println!("   construct: {c}");
        }
        // The partition witness (page_proves_in_lang's core): does ANY construct fire on the page's own
        // example corpus with a clean python parse of the first containing block?
        use helpers_native::lint_trace::{parses_cleanly, run_plan, Plan};
        let cin = |blk: &str, c: &str| blk.contains(c);
        let witness = p.constructs.iter().find(|c| {
            p.example_code.iter().find(|blk| cin(blk, c)).is_some_and(|blk| {
                parses_cleanly("python", blk)
                    && !run_plan(&Plan::UsesConstruct { construct: (*c).clone() }, "python", blk)
                        .is_empty()
            })
        });
        println!("   partition witness: {witness:?}");
    }
}
