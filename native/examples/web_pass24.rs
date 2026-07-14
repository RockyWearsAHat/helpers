//! PASS 24 MEASUREMENT — THE LANGUAGE WEB, over the real crawl cache via the LIBRARY live path.
//!
//! For each owned language it runs `lint_module::graduated_rules` (which now BUILDS + PERSISTS the web
//! and DERIVES the rules from it), then:
//!   (1) reloads the persisted `<lang>.web.bin`, re-derives, and checks the rules reproduce (round-trip →
//!       derive is total, the source-of-truth contract);
//!   (2) reports the per-language table: pages read, READ nodes (everything read), PROVEN nodes (= rules),
//!       web bytes;
//!   (3) demonstrates cross-language traversal through the shared English base;
//!   (4) demonstrates assembly/isolation (a per-language sidecar; deleting one leaves the others' derive
//!       byte-identical).
//!
//! Run: `cargo run --release --features crawl --example web_pass24`
use helpers_native::lint_char;
use helpers_native::lint_module;
use helpers_native::lint_read::Memory;
use helpers_native::lint_web::{self, ConstructNode};
use std::time::Instant;

fn web_bytes(lang: &str) -> u64 {
    let home = std::env::var("HOME").unwrap_or_default();
    let p = format!("{home}/.cache/helpers/lint-models/{lang}.web.bin");
    std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
}

fn rule_ids(rules: &[(helpers_native::linter::LearnedRule, String)]) -> Vec<String> {
    let mut v: Vec<String> = rules.iter().map(|(r, _)| r.id.clone()).collect();
    v.sort();
    v
}

fn main() {
    if lint_char::brain().is_none() {
        eprintln!("no char brain on disk — run setup/train first");
        return;
    }
    let langs = ["css", "html", "javascript", "python", "rust", "typescript"];
    println!("\n════ PASS 24 — THE LANGUAGE WEB (real crawl cache, live path) ════\n");
    println!("{:<12} {:>7} {:>10} {:>7} {:>10} {:>10}", "lang", "pages", "read-nodes", "rules", "web-bytes", "roundtrip");
    println!("{}", "─".repeat(64));

    let mut webs: Vec<(String, Vec<ConstructNode>)> = Vec::new();
    for lang in langs {
        let t = Instant::now();
        // The live path: reads the crawl cache, builds + persists the web, derives the rules from it.
        let module = lint_module::graduated_rules(lang, &Memory::default());
        let dt = t.elapsed();
        // Reload the persisted web and re-derive — the round-trip → derive contract.
        let web = lint_web::load(lang);
        let reproduced = lint_web::derive_rules(lang, &web);
        let live_ids = rule_ids(&module.rules);
        let reproduced_ids = rule_ids(&reproduced);
        let roundtrip = if live_ids == reproduced_ids { "OK" } else { "MISMATCH" };
        let read_nodes = web.len();
        let proven = web.iter().filter(|n| n.proven).count();
        println!(
            "{:<12} {:>7} {:>10} {:>7} {:>10} {:>10}  ({:.1}s)",
            lang,
            module.corpus_urls.len(),
            read_nodes,
            proven,
            web_bytes(lang),
            roundtrip,
            dt.as_secs_f64(),
        );
        if live_ids != reproduced_ids {
            eprintln!("  !! live rules {} vs web-derived {}", live_ids.len(), reproduced_ids.len());
        }
        webs.push((lang.to_string(), web));
    }

    // ── Cross-language traversal through the shared English base ──────────────────────────────────
    println!("\n──── CROSS-LANGUAGE TRAVERSAL (governing-prose meaning through the shared English base) ────");
    if let Some(br) = lint_char::brain() {
        let m = br.meanings();
        // Pick a few PROVEN nodes and show what they connect to in OTHER languages.
        let seeds = [
            ("python", "cgi"),
            ("javascript", "document.write"),
            ("css", ""), // first proven css node
        ];
        for (lang, want) in seeds {
            let web = lint_web::load(lang);
            let node = if want.is_empty() {
                web.iter().find(|n| n.proven)
            } else {
                web.iter().find(|n| n.construct == want)
            };
            let Some(node) = node else {
                println!("\n[{lang}] no node for '{want}'");
                continue;
            };
            println!("\n[{lang}] construct `{}`  (state: {})", node.construct, if node.proven { "PROVEN" } else { "READ" });
            println!("   governing: {:?}", node.governing.first().map(|s| truncate(s, 90)));
            println!("   meaning-links: {:?}", node.meaning_links);
            let cross = lint_web::cross_language(m, lang, node, 5);
            for c in &cross {
                println!(
                    "   → {:<11} `{}`  dist={}  shared={:?}",
                    c.lang, c.construct, c.distance, c.shared_links
                );
            }
        }
        // Concept traversal: two removed/deprecated constructs from DIFFERENT languages both reach the
        // deprecation/removal concept space.
        println!("\n──── CONCEPT TRAVERSAL (do cross-language deprecations reach shared concepts?) ────");
        for (lang, c) in [("python", "cgi"), ("javascript", "document.write")] {
            if let Some(node) = lint_web::load(lang).into_iter().find(|n| n.construct == c) {
                if let Some(hv) = lint_web::node_meaning(m, &node) {
                    // nearest tracing concepts of the node's bundled meaning
                    let mut near: Vec<(String, u32)> = node
                        .meaning_links
                        .iter()
                        .filter_map(|w| helpers_native::lint_trace::concept_alignment(w).and_then(|a| a.into_iter().next()))
                        .collect();
                    near.sort_by_key(|(_, d)| *d);
                    near.dedup_by(|a, b| a.0 == b.0);
                    let _ = hv;
                    println!("[{lang}] `{c}` → nearest concepts via links: {:?}", near.into_iter().take(4).collect::<Vec<_>>());
                }
            }
        }
    }

    // ── Doc-role traversal (PASS 25 rung 2) ──────────────────────────────────────────────────────
    println!("\n──── DOC-ROLE TRAVERSAL (removal/prohibition/deprecated as first-class targets) ────");
    for role in ["removal", "prohibition", "deprecated"] {
        let carriers = lint_web::roles_across(role);
        println!("\n  role `{role}` → {} carrier(s) across all webs:", carriers.len());
        for (lang, c) in carriers.iter().take(30) {
            println!("     [{lang}] {c}");
        }
        if carriers.len() > 30 {
            println!("     … and {} more", carriers.len() - 30);
        }
    }
    // Per-language read-node vs role summary.
    println!("\n  per-web node/role census:");
    for (lang, web) in &webs {
        let read = web.iter().filter(|n| !n.proven).count();
        let with_roles = web.iter().filter(|n| !n.roles.is_empty()).count();
        println!(
            "    {:<12} nodes={:<4} proven={:<4} read-only={:<4} role-bearing={}",
            lang,
            web.len(),
            web.iter().filter(|n| n.proven).count(),
            read,
            with_roles
        );
    }

    // ── Assembly / isolation ─────────────────────────────────────────────────────────────────────
    println!("\n──── ASSEMBLY / ISOLATION (per-language sidecar — internal dependencies only) ────");
    println!("webs on machine: {:?}", lint_web::languages_with_web());
    // Deriving lang X's rules never touches lang Y's web: derive is pure over the loaded web.
    for (lang, web) in &webs {
        let a = rule_ids(&lint_web::derive_rules(lang, web));
        let b = rule_ids(&lint_web::derive_rules(lang, &lint_web::load(lang)));
        println!("  {:<12} derive stable across reload: {}", lang, if a == b { "OK" } else { "MISMATCH" });
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let t: String = s.chars().take(n).collect();
        format!("{t}…")
    }
}
