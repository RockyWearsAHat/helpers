//! THROWAWAY PROBE (untracked): SENTENCE-ASSERTED ELEMENT witnesses — for each dead element, does
//! any W3/MDN page's RAW text write `&lt;name&gt;` within a window that carries a revocation anchor
//! (deprecated/removed) and no negation? Prints per element: witnesses found (url + window head).
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
    let home = std::env::var("HOME").unwrap();
    let mut pages: Vec<(String, String)> = Vec::new();
    for s in ["mdn-web", "mdn-html", "mdn-js", "mdn-css", "mdn-api", "w3schools-site", "w3schools-html", "w3schools-tags", "w3schools-js", "w3schools-css"] {
        if let Some(mut v) = std::fs::read(format!("{home}/.cache/helpers/lint-index/crawls/{s}.bin")).ok().and_then(|b| decode_crawl(&b)) {
            pages.append(&mut v);
        }
    }
    println!("pool: {}", pages.len());
    for name in ["isindex", "blink", "applet", "bgsound", "spacer", "keygen", "basefont", "listing", "menuitem", "plaintext", "center", "font"] {
        let needle = format!("&lt;{name}&gt;");
        let mut hits = 0;
        for (url, body) in &pages {
            let mut from = 0;
            while let Some(rel) = body[from..].find(&needle) {
                let i = from + rel;
                from = i + needle.len();
                let s = i.saturating_sub(220);
                let e = (i + 220).min(body.len());
                let w = body[s..e].to_lowercase();
                let anchored = w.contains("deprecated") || w.contains("removed");
                let negated = w.contains(" not deprecated") || w.contains(" not removed");
                if anchored && !negated {
                    hits += 1;
                    if hits <= 2 {
                        let head: String = body[s..e].chars().filter(|c| *c != '\n').take(150).collect();
                        println!("  {name} ⟵ {url}\n      ⟨…{head}…⟩");
                    }
                    break;
                }
            }
            if hits >= 3 { break; }
        }
        println!("{name}: witnesses {hits}");
    }
}
