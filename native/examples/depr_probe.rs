//! THROWAWAY PROBE (untracked): for a few tools, find pages whose body mentions deprecation and print
//! the structural markup around the FIRST hit — so we can judge whether a cheap, per-SOURCE structural
//! marker (a class/notecard shape) exists to key on, or whether the docs signal deprecation only in
//! narrative prose (which the covenant forbids keying on with a word list).
//! Run: `cargo run --release --features crawl --example depr_probe`
use helpers_native::lint_codec::{self, Dec};

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

fn main() {
    let home = std::env::var("HOME").unwrap();
    let dir = format!("{home}/.cache/helpers/lint-index/crawls");
    for tool in ["rust-reference", "python-docs", "mdn-svg", "crystal-reference", "cue-docs"] {
        let path = format!("{dir}/{tool}.bin");
        let Ok(b) = std::fs::read(&path) else {
            println!("== {tool}: no .bin\n");
            continue;
        };
        let Some(pages) = decode_bin(&b) else {
            println!("== {tool}: decode fail\n");
            continue;
        };
        // Count pages whose body mentions any deprecation word; how many carry the MDN notecard class.
        let mut depr = 0usize;
        let mut notecard = 0usize;
        let mut sample: Option<(String, String)> = None;
        for (url, body) in &pages {
            let lb = body.to_lowercase();
            let hit = lb.contains("deprecat") || lb.contains("no longer recommended") || lb.contains("obsolete");
            if hit {
                depr += 1;
            }
            if lb.contains("notecard deprecated") || lb.contains("no longer recommended") {
                notecard += 1;
            }
            if hit && sample.is_none() {
                // window around the first deprecation word
                if let Some(i) = lb.find("deprecat").or_else(|| lb.find("obsolete")) {
                    let s = i.saturating_sub(220);
                    let e = (i + 160).min(body.len());
                    // snap to char boundaries
                    let s = (s..=i).find(|&k| body.is_char_boundary(k)).unwrap_or(i);
                    let e = (i..=e).rev().find(|&k| body.is_char_boundary(k)).unwrap_or(i);
                    sample = Some((url.clone(), body[s..e].replace('\n', " ")));
                }
            }
        }
        println!("== {tool}: {} pages, {depr} mention deprecation/obsolete, {notecard} carry MDN notecard", pages.len());
        if let Some((u, snip)) = sample {
            println!("   first hit: {u}");
            println!("   markup: …{snip}…\n");
        } else {
            println!("   (no deprecation mention found)\n");
        }
    }
}
