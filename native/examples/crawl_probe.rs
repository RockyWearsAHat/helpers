//! THROWAWAY PROBE (untracked): what a crawl shard actually holds — page count, URL section
//! histogram, and presence of the named deprecated-element reference pages the coverage rung
//! needs. Read-only. Run: `cargo run --release --example crawl_probe -- <shard.bin> [needle…]`
use helpers_native::lint_codec::{self, Dec};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: crawl_probe <shard.bin> [needle…]");
    let needles: Vec<String> = args.collect();
    let bytes = std::fs::read(&path).expect("shard readable");
    let (stamp, mut d) = Dec::open(&bytes, lint_codec::kind::CRAWL).expect("CRAWL container");
    let _version = d.str().expect("version");
    let _crawled_at = d.fixed_u64().expect("crawled_at");
    let n = d.u().expect("page count") as usize;
    let mut sections: std::collections::BTreeMap<String, usize> = Default::default();
    let mut urls = Vec::with_capacity(n);
    for i in 0..n {
        let url = d.str().unwrap_or_else(|| panic!("url of page {i}"));
        let _body = d.str().unwrap_or_else(|| panic!("body of page {i}"));
        let _has_modified = d.boolean().expect("modified flag");
        let _modified = d.fixed_u64().expect("modified");
        let _fp = d.fixed_u64().expect("fp");
        let sect = url.split("/docs/").nth(1).map(|r| {
            r.split('/').take(3).collect::<Vec<_>>().join("/")
        });
        *sections.entry(sect.unwrap_or_else(|| "(other)".into())).or_default() += 1;
        urls.push(url);
    }
    println!("stamp={stamp} pages={n}");
    for (s, c) in &sections {
        println!("  {c:>5}  {s}");
    }
    for needle in &needles {
        let hits: Vec<&String> = urls.iter().filter(|u| u.contains(needle.as_str())).collect();
        println!("needle {needle}: {} hit(s)", hits.len());
        for h in hits.iter().take(3) {
            println!("    {h}");
        }
    }
}
