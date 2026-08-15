//! THROWAWAY (untracked): crawl-coverage probe — are the known-missing MDN deprecation pages even in
//! the crawl corpus, and are they attested? Names the funnel stage that sheds each one.
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
    std::fs::read(format!("{home}/.cache/helpers/lint-index/crawls/{name}.bin"))
        .ok()
        .and_then(|b| decode_crawl(&b))
        .unwrap_or_default()
}

fn main() {
    let mut pages = Vec::new();
    for n in ["mdn-css", "mdn-js", "mdn-html", "developer-mozilla-org-4cbd1761",
              "developer-mozilla-org-572fe52b", "developer-mozilla-org-611ab56b",
              "developer-mozilla-org-5d48a8d0"] {
        let p = load(n);
        println!("shard {n}: {} pages", p.len());
        pages.extend(p);
    }
    let mut seen = std::collections::HashSet::new();
    pages.retain(|(u, _)| seen.insert(u.clone()));
    println!("total unique: {}", pages.len());
    let att = helpers_native::lint_attest::Attestation::discover(&pages);
    let attested: Vec<&(String, String)> = pages.iter().filter(|(_, b)| att.attests(b)).collect();
    println!("attested pages: {}", attested.len());
    // The known-missing subjects, by URL suffix.
    let want = ["Element/font", "Element/strike", "Element/applet", "Element/blink", "Element/keygen",
                "Element/dir", "Element/plaintext", "Element/xmp", "Element/menuitem", "Element/spacer",
                "Element/nobr", "Element/noembed", "Element/param", "Element/marquee",
                "lastParen", "n_dollar", "toGMTString", "ime-mode", "azimuth", "attachEvent",
                "Document/all", "Element/bgsound"];
    for w in want {
        let hit = pages.iter().find(|(u, _)| u.ends_with(w) || u.contains(&format!("{w}$")) || u.contains(w));
        match hit {
            Some((u, b)) => println!("IN-CRAWL {} attested={}", u, att.attests(b)),
            None => println!("MISSING-FROM-CRAWL {w}"),
        }
    }
}
