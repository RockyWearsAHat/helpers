//! THROWAWAY PROBE (untracked): WHY does a page attest deprecated — which marker run or class-token
//! context matches? Prints every `deprecated`-bearing class attribute context and whether the learned
//! text-run markers hit. Run: crawl shard + URL fragment as args.
use helpers_native::lint_attest::Attestation;
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

fn main() {
    let mut args = std::env::args().skip(1);
    let shard = args.next().expect("shard");
    let frag = args.next().expect("url fragment");
    let home = std::env::var("HOME").unwrap();
    let all: Vec<(String, String)> = ["mdn-api", "mdn-js", "mdn-css", "mdn-html", "mdn-svg", "w3schools-js", "w3schools-css", "w3schools-html"]
        .iter()
        .filter_map(|s| std::fs::read(format!("{home}/.cache/helpers/lint-index/crawls/{s}.bin")).ok())
        .filter_map(|b| decode_crawl(&b))
        .flatten()
        .collect();
    let att = Attestation::discover(&all);
    println!("markers={} class_markers={:?}", att.markers().len(), att.class_markers());
    let bytes = std::fs::read(format!("{home}/.cache/helpers/lint-index/crawls/{shard}.bin")).unwrap();
    let pages = decode_crawl(&bytes).unwrap();
    let (url, body) = pages.iter().find(|(u, _)| u.contains(&frag)).expect("page");
    println!("page: {url}");
    println!("attests={} page_scope(markers-only)={}", att.attests(body), att.attests_page_scope(body));
    // Every class attribute containing a marker token, with 80 bytes of leading context.
    for cm in att.class_markers() {
        for (i, _) in body.match_indices(&format!("{cm}")) {
            let start = i.saturating_sub(120);
            let ctx: String = body[start..(i + cm.len() + 20).min(body.len())].chars().collect();
            if ctx.contains("class") {
                println!("  ctx: …{}…", ctx.replace('\n', "⏎"));
            }
        }
    }
}
