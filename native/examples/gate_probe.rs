//! Probes the REAL trained model's pipeline for one file — raw fires, then the gate's
//! verdict and concept ranking per imprecise finding, so "where did the finding die?" (or
//! "why did this fire?") is answered against the exact artifacts a lint run would use.
//! Run: `cargo run --release --example gate_probe -- <root> <data_root> [file] [lang]`.

fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("root"));
    let data = std::path::PathBuf::from(std::env::args().nth(2).expect("data_root"));
    let file = std::env::args().nth(3).unwrap_or_else(|| "app.js".into());
    let lang_arg = std::env::args().nth(4).unwrap_or_else(|| "javascript".into());
    let src = std::fs::read_to_string(root.join(&file)).expect("fixture file");

    struct Src(String);
    impl helpers_native::lint_train::ProjectSource for Src {
        fn fingerprint(&self, _lang: &str) -> u64 {
            helpers_native::lint_ai::token_seed(&self.0)
        }
        fn sources(&self, _lang: &str) -> Vec<(String, String)> {
            vec![("app.js".to_string(), self.0.clone())]
        }
    }
    let (report, models) = helpers_native::lint_train::ensure_models(
        &[lang_arg.clone()],
        &data,
        &root,
        &Src(src.clone()),
    );
    println!("trained={:?} reused={:?} unenforced={:?}", report.trained, report.reused, report.unenforced);
    let model = models.get(&lang_arg).expect("model");
    println!("rules={} detector={:?}", model.rules.rule_count(), model.rules.detector_of("no_var_declaration"));
    let findings = model.rules.flag(&src);
    println!(
        "raw findings: {:?}",
        findings.iter().map(|f| (f.rule.clone(), f.line, f.precise)).collect::<Vec<_>>()
    );
    for f in &findings {
        if !f.precise {
            let line = src.lines().nth(f.line - 1).unwrap_or("");
            let tokens: Vec<&str> = line
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .filter(|t| t.len() >= 2)
                .collect();
            let kept = model.concept.confirms(&f.rule, &tokens);
            println!("gate({}, {:?}) -> keep={}", f.rule, tokens, kept);

            // Who won? Rebuild the query exactly as the gate does and rank all concepts.
            use helpers_native::lint_ai::{token_hv, token_seed, Bundler};
            let mut b = Bundler::new();
            for t in &tokens {
                let t = t.to_lowercase();
                if t.len() >= 2 && t.len() <= 64 {
                    b.add(&token_hv(&t));
                }
            }
            let q = b.finalize();
            let id_of: std::collections::HashMap<u64, &str> =
                model.rules.rule_ids().map(|id| (token_seed(id), id)).collect();
            let mut dists: Vec<(u32, &str)> = model
                .concept
                .rules
                .iter()
                .map(|r| (q.distance(&r.rule_hv), id_of.get(&r.id_hash).copied().unwrap_or("?")))
                .collect();
            dists.sort();
            println!("nearest 6: {:?}", &dists[..dists.len().min(6)]);
        }
    }
}
