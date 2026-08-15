//! THROWAWAY PROBE (untracked): PASS 27 — measure the GRADED tier funnel per language by running the REAL
//! graduation pass (`lint_module::graduated_rules`) over this machine's crawl cache. Prints the proven set
//! (unchanged), the shipped graded LOW forms, and each form's kind (receiver-generic `.member` = usage-dead,
//! or dotted-literal = member alive). Pairs with `graded_probe` (candidates + qualified-safe endpoints).
//! Run: `cargo run --release --example graded_measure`
use helpers_native::lint_module::graduated_rules;
use helpers_native::lint_read::Memory;

fn main() {
    for lang in ["css", "html", "javascript", "python", "rust", "typescript"] {
        let m = Memory::default();
        let module = graduated_rules(lang, &m);
        let (mut recv, mut dotted) = (0usize, 0usize);
        for (r, _) in &module.graded {
            match r.construct.as_deref() {
                Some(c) if c.starts_with('.') => recv += 1,
                _ => dotted += 1,
            }
        }
        println!(
            "== {lang}: proven={} | GRADED shipped={} ({recv} receiver-generic usage-dead, {dotted} dotted-literal) ==",
            module.rules.len(),
            module.graded.len()
        );
        let mut forms: Vec<_> = module.graded.iter().collect();
        forms.sort_by(|a, b| a.0.id.cmp(&b.0.id));
        for (r, url) in forms {
            println!("    {} fires `{}` [{}]", r.id, r.construct.as_deref().unwrap_or("?"), r.severity);
            println!("        {}  ⟨{url}⟩", r.description);
        }
    }
}
