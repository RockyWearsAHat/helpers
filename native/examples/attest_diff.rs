//! THROWAWAY PROBE (untracked): why does `Elements/marquee` mint a web node while `Elements/font`
//! does not, both in the fresh mdn-html crawl? Runs the REAL attestation faculty over the html
//! corpus and reports, per named element page: present-in-crawl, marker attestation, class
//! attestation. Read-only. Run: `cargo run --release --example attest_diff`
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
    let home = std::env::var("HOME").unwrap();
    let mut pages = Vec::new();
    for shard in ["mdn-html", "w3schools-html"] {
        let p = format!("{home}/.cache/helpers/lint-index/crawls/{shard}.bin");
        if let Some(mut v) = std::fs::read(&p).ok().and_then(|b| decode_crawl(&b)) {
            println!("shard {shard}: {} pages", v.len());
            pages.append(&mut v);
        }
    }
    let att = Attestation::discover(&pages);
    println!("markers: {}", att.markers().len());
    for m in att.markers() {
        println!("  ⟨{}⟩", &m[..m.len().min(100)]);
    }
    println!("class_markers: {:?}", att.class_markers());
    for name in ["font", "strike", "marquee", "center", "big", "acronym", "dir", "param", "frame"] {
        let hit = pages.iter().find(|(u, _)| u.ends_with(&format!("/Elements/{name}")));
        match hit {
            None => println!("{name:>10}: NOT IN CRAWL"),
            Some((u, b)) => println!("{name:>10}: in-crawl attests={} ({u})", att.attests(b)),
        }
    }
}
