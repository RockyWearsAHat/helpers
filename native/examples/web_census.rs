//! THROWAWAY PROBE (untracked): the knowledge census — per language web: node count, attested,
//! roles, proven, graded, and the crawl-pages→web-nodes funnel ratio. Read-only.
use helpers_native::lint_web;

fn main() {
    for lang in lint_web::languages_with_web() {
        let web = lint_web::load(&lang);
        let attested = web.iter().filter(|n| n.attested_deprecated).count();
        let proven = web.iter().filter(|n| n.proven).count();
        let ruled = web.iter().filter(|n| n.rule.is_some()).count();
        let graded = web.iter().filter(|n| n.graded.is_some()).count();
        let revoked = web
            .iter()
            .filter(|n| n.roles.iter().any(|r| r == "deprecated" || r == "removal" || r == "removed"))
            .count();
        let succession = web.iter().filter(|n| n.superseded_by.is_some()).count();
        println!(
            "{lang:>12}: nodes={} attested={attested} revoked-role={revoked} proven={proven} rule={ruled} graded={graded} superseded={succession}",
            web.len()
        );
        for n in web.iter().filter(|n| n.superseded_by.is_some()).take(6) {
            let s = n.superseded_by.as_ref().unwrap();
            println!("      {} → {}  ⟨{}⟩", n.construct, s.successor, s.sentence.chars().take(90).collect::<String>());
        }
    }
}
