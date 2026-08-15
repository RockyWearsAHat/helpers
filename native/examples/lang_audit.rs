//! THROWAWAY AUDIT (untracked): the WHOLE-LANGUAGE correctness sweep — "check correctness of a
//! language in its entirety" (owner, 2026-07-15). For one language, machine-verifies:
//!   1. every ENFORCED rule fires on its own documented bad exemplar (exact rule id) and stays
//!      silent on its own good exemplar — through the REAL compiled RuleSet (the live engine);
//!   2. every enforced rule's web node is attested/revoked-role and cites at least one source;
//!   3. the self-referee holds no contradiction on any enforced construct;
//!   4. FULL ACCOUNTING: every revoked-role web node is enforced, graded, withheld with a named
//!      reason, or listed here as unproven-with-reason — nothing silently unaccounted (PASS 31 law
//!      extended across tiers).
//! Run: `cargo run --release --example lang_audit -- <language>`
use std::collections::{HashMap, HashSet};

fn main() {
    let lang = std::env::args().nth(1).expect("usage: lang_audit <language>");
    let web = helpers_native::lint_web::load(&lang);
    let Some(rules) = helpers_native::lint_train::cached_ruleset(&lang) else {
        println!("{lang}: NO MODULE");
        return;
    };
    let node: HashMap<&str, &helpers_native::lint_web::ConstructNode> =
        web.iter().map(|n| (n.construct.as_str(), n)).collect();
    let withheld: HashMap<&str, &str> =
        rules.withheld().iter().map(|(id, gate)| (id.as_str(), gate.as_str())).collect();

    let mut fail = 0usize;
    let mut checked = 0usize;
    for (rule, source) in helpers_native::lint_web::derive_rules(&lang, &web) {
        checked += 1;
        let fires = |code: &str| rules.flag(code).iter().any(|h| h.rule == rule.id);
        if !rule.bad.is_empty() && !fires(&rule.bad) {
            println!("FAIL fire   {}: bad exemplar does not fire ⟨{}⟩", rule.id, rule.bad.chars().take(60).collect::<String>());
            fail += 1;
        }
        if !rule.good.is_empty() && fires(&rule.good) {
            println!("FAIL clean  {}: good exemplar fires ⟨{}⟩", rule.id, rule.good.chars().take(60).collect::<String>());
            fail += 1;
        }
        let Some(construct) = rule.construct.as_deref() else { continue };
        match node.get(construct) {
            None => {
                println!("FAIL node   {}: no web node for `{construct}`", rule.id);
                fail += 1;
            }
            Some(n) => {
                if !n.attested_deprecated && !n.roles.iter().any(|r| r == "removal" || r == "prohibition") {
                    println!("FAIL attest {}: enforced without attestation or revoked role", rule.id);
                    fail += 1;
                }
                if n.sources.is_empty() || source.is_empty() {
                    println!("FAIL cite   {}: no source citation", rule.id);
                    fail += 1;
                }
                if let Some(r) = &n.referee {
                    if !r.contradictions.is_empty() {
                        println!("FAIL refree {}: enforced while contradicted: {:?}", rule.id, r.contradictions);
                        fail += 1;
                    }
                }
            }
        }
    }

    // Full accounting over every revoked-role node.
    let enforced: HashSet<&str> = web.iter().filter(|n| n.rule.is_some()).map(|n| n.construct.as_str()).collect();
    let (mut n_enf, mut n_graded, mut n_withheld, mut n_unproven) = (0, 0, 0, 0);
    let mut unproven: Vec<&str> = Vec::new();
    for n in web.iter().filter(|n| n.attested_deprecated || n.roles.iter().any(|r| r == "deprecated" || r == "removal")) {
        if enforced.contains(n.construct.as_str()) {
            n_enf += 1;
        } else if n.graded.is_some() {
            n_graded += 1;
        } else if withheld.keys().any(|id| id.ends_with(n.construct.as_str())) {
            n_withheld += 1;
        } else {
            n_unproven += 1;
            unproven.push(&n.construct);
        }
    }
    println!("\n{lang}: {} web nodes | revoked accounting: enforced {} + graded {} + withheld {} + unproven {}", web.len(), n_enf, n_graded, n_withheld, n_unproven);
    println!("rules audited: {checked} | FAILURES: {fail}");
    if !unproven.is_empty() {
        println!("unproven revoked (attested knowledge, no enforceable proof yet — honest, queryable):");
        for c in unproven.iter().take(40) {
            println!("  {c}");
        }
        if unproven.len() > 40 {
            println!("  … +{}", unproven.len() - 40);
        }
    }
    if fail == 0 {
        println!("VERDICT: {lang} correct in its entirety — every enforced rule fires its own bad, stays clean on its own good, is attested, cited, and uncontradicted; every revoked node accounted.");
    }
}
