//! THROWAWAY PROBE (untracked): PASS 27 rung 3 — regression fixtures for the GRADED tier through the REAL
//! compiled firing surface ([`RuleSet::build`] + `flag`, the live path's engine). Per language: a CLEAN
//! MODERN file must stay ZERO, a LANDMINE file (deprecated names only in strings/comments) must stay ZERO,
//! and a KITCHEN-SINK file gains the graded LOW findings. Prints findings verbatim.
//! Run: `cargo run --release --example graded_fixtures -- <fixtures-dir>`
use helpers_native::lint_match::{Grounding, RuleSet};
use helpers_native::lint_module::graduated_rules;
use helpers_native::lint_read::Memory;

fn main() {
    let dir = std::env::args().nth(1).expect("usage: graded_fixtures <fixtures-dir>");
    let fixtures: &[(&str, &str)] = &[
        ("javascript", "js"),
        ("python", "py"),
    ];
    for (lang, ext) in fixtures {
        let module = graduated_rules(lang, &Memory::default());
        let tuples: Vec<(String, String, String, String, String, String, Option<String>)> = module
            .rules
            .iter()
            .chain(module.graded.iter())
            .map(|(r, url)| {
                (r.id.clone(), r.severity.clone(), r.bad.clone(), r.good.clone(), r.description.clone(), url.clone(), r.construct.clone())
            })
            .collect();
        let rs = RuleSet::build(lang, &tuples, &Grounding::default());
        println!("======== {lang}: proven={} graded={} compiled={} ========", module.rules.len(), module.graded.len(), tuples.len());
        for name in ["clean_modern", "landmine", "kitchen_sink"] {
            let path = format!("{dir}/{name}.{ext}");
            let Ok(code) = std::fs::read_to_string(&path) else {
                println!("  {name}.{ext}: MISSING");
                continue;
            };
            let findings = rs.flag(&code);
            println!("  -- {name}.{ext}: {} findings --", findings.len());
            for f in &findings {
                println!("     line {:>2} [{:6}] {}", f.line, f.severity, f.rule);
            }
        }
        println!();
    }
}
