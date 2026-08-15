//! THROWAWAY MEASUREMENT HARNESS (untracked): PASS 30 — the SELF-REFEREE, measured before it ships.
//! For every revoked-role node of the persisted python web, judge every OTHER source's governing
//! sentence that names the construct (bounded, full token): claim = revocation-asserting /
//! revocation-denying / neutral, from the frozen negation classifier × the prohibits/removed anchors.
//! Prints the verdict distribution + samples, and the is_negated behavior on canonical shapes — the
//! truth table is chosen from THIS measurement, not assumed.
//! Run: `cargo run --release --features crawl --example referee_probe`
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

/// Bounded full-token mention: the construct appears delimited by non-identifier chars (dots kept).
fn mentions_full(sentence: &str, construct: &str) -> bool {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.';
    let mut from = 0usize;
    while let Some(rel) = sentence[from..].find(construct) {
        let s = from + rel;
        let e = s + construct.len();
        let before = sentence[..s].chars().next_back().map(|c| !is_ident(c)).unwrap_or(true);
        let after = sentence[e..].chars().next().map(|c| !is_ident(c)).unwrap_or(true);
        if before && after {
            return true;
        }
        from = e;
    }
    false
}

fn revokes(sentence_lower: &str, anchors: &[String]) -> bool {
    sentence_lower
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|w| anchors.iter().any(|a| a == w))
}

fn main() {
    let (Some(_br), Some(en)) = (helpers_native::lint_char::brain(), helpers_native::lint_english::brain())
    else {
        eprintln!("no frozen brains");
        return;
    };
    // The negation classifier's behavior on the canonical claim shapes — the truth table's basis.
    for s in [
        "the cgi module is deprecated",
        "cgi is not deprecated",
        "do not use the cgi module",
        "use urllib.parse instead of cgi",
        "deprecated since version 3.10: passing loop is deprecated",
        "this method returns the socket options",
    ] {
        println!("is_negated={:5}  {s}", helpers_native::lint_corroborate::is_negated(en, s));
    }

    let mut anchors = helpers_native::lint_attest::prohibition_class_tokens();
    anchors.extend(helpers_native::lint_attest::removal_class_tokens());
    println!("anchors: {anchors:?}");

    // The persisted python web's revoked-role nodes.
    let web = helpers_native::lint_web::load("python");
    let revoked: Vec<_> = web
        .iter()
        .filter(|n| n.attested_deprecated || !n.roles.is_empty())
        .collect();
    println!("python web: {} nodes, {} revoked-role", web.len(), revoked.len());

    // The corpus's governing-sentence pool, replicated through the real reader.
    let mut pages = load("python-library");
    pages.extend(load("python-docs"));
    let attest = helpers_native::lint_attest::Attestation::discover(&pages);
    let attested: std::collections::HashSet<String> =
        pages.iter().filter(|(_, b)| attest.attests(b)).map(|(u, _)| u.clone()).collect();
    let chrome = helpers_native::lint_graph::site_chrome(&pages);
    let m = helpers_native::lint_char::brain().unwrap().meanings();
    let bridge = helpers_native::lint_trace::Bridge::new(m, en);
    let construction = std::collections::HashMap::new();
    let mut pool: Vec<(String, String, bool)> = Vec::new(); // (url, sentence_lower, negated)
    for (url, body) in &pages {
        let stripped = chrome.strip(url, body);
        let p = helpers_native::lint_lang_layer::read_doc_page(url, &stripped, en, &bridge, &attested, &construction);
        for s in &p.governing {
            let neg = helpers_native::lint_corroborate::is_negated(en, s);
            pool.push((url.clone(), s.to_lowercase(), neg));
        }
    }
    println!("pool: {} governing sentences", pool.len());

    let (mut n_cohere, mut n_contra, mut n_neutral) = (0usize, 0usize, 0usize);
    let mut contra_samples = Vec::new();
    let mut cohere_samples = Vec::new();
    for node in &revoked {
        let c_lower = node.construct.to_lowercase();
        let own: std::collections::HashSet<&str> = node.sources.iter().map(String::as_str).collect();
        let mut cohere_urls: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for (url, s, neg) in &pool {
            if own.contains(url.as_str()) || !mentions_full(s, &c_lower) {
                continue;
            }
            let rev = revokes(s, &anchors);
            match (rev, *neg) {
                (true, false) => {
                    cohere_urls.insert(url);
                    if cohere_samples.len() < 8 {
                        cohere_samples.push(format!("{} <- {}", node.construct, &s[..s.len().min(110)]));
                    }
                }
                (true, true) => {
                    n_contra += 1;
                    if contra_samples.len() < 12 {
                        contra_samples.push(format!("{} <- {}", node.construct, &s[..s.len().min(140)]));
                    }
                }
                _ => n_neutral += 1,
            }
        }
        n_cohere += cohere_urls.len();
    }
    println!("\ncoherent (distinct other-source urls, summed over nodes): {n_cohere}");
    println!("contradiction-shaped (revokes && negated): {n_contra}");
    println!("neutral mentions: {n_neutral}");
    println!("\ncohere samples:");
    for s in &cohere_samples {
        println!("  {s}");
    }
    println!("contradiction samples:");
    for s in &contra_samples {
        println!("  {s}");
    }
}
