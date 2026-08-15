//! THROWAWAY PROBE (untracked, PASS 34 design): for each partition-excluded attested element page,
//! print its own `<pre><code>` blocks and per block: contains-subject, parses_cleanly(html), fires.
//! Names which arm of `page_proves_in_lang` shed it. Read-only.
use helpers_native::lint_codec::{self, Dec};
use helpers_native::lint_lang_layer::read_doc_page;
use helpers_native::lint_trace::{parses_cleanly, run_plan, Bridge, Plan};

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
    let bytes = std::fs::read(format!("{home}/.cache/helpers/lint-index/crawls/mdn-html.bin")).unwrap();
    let pages = decode_crawl(&bytes).unwrap();
    let (br, en) = (helpers_native::lint_char::brain().unwrap(), helpers_native::lint_english::brain().unwrap());
    let bridge = Bridge::new(br.meanings(), en);
    let att = helpers_native::lint_attest::Attestation::discover(&pages);
    let attested: std::collections::HashSet<String> =
        pages.iter().filter(|(_, b)| att.attests(b)).map(|(u, _)| u.clone()).collect();
    for name in ["strike", "nobr", "noembed", "noframes", "rtc", "font", "xmp", "dir", "plaintext"] {
        let Some((url, body)) = pages.iter().find(|(u, _)| u.ends_with(&format!("/Elements/{name}"))) else {
            println!("{name}: not cached");
            continue;
        };
        let dp = read_doc_page(url, body, en, &bridge, &attested, &Default::default());
        let mentions = dp.governing.iter().filter(|s| s.contains(name)).count();
        println!(
            "{name}: governing={} mentioning-subject={} constructs={:?}",
            dp.governing.len(),
            mentions,
            dp.constructs
        );
        let own = dp.example_code;
        println!("== {name}: {} own block(s)", own.len());
        let plan = Plan::UsesConstruct { construct: name.to_string() };
        for (i, blk) in own.iter().enumerate() {
            let contains = blk.contains(name);
            let clean = parses_cleanly("html", blk);
            let fires = !run_plan(&plan, "html", blk).is_empty();
            let head: String = blk.chars().take(90).collect();
            println!("   [{i}] contains={contains} clean={clean} fires={fires}  ⟨{}⟩", head.replace('\n', "⏎"));
        }
    }
}
