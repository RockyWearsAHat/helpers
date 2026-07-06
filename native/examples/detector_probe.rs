//! Prints the compiled detectors of suspect doc rules — what does the engine actually
//! watch for? Run: `cargo run --release --example detector_probe -- <root> <data> <lang> <id>...`

fn main() {
    let mut args = std::env::args().skip(1);
    let root = std::path::PathBuf::from(args.next().expect("root"));
    let data = std::path::PathBuf::from(args.next().expect("data"));
    let lang = args.next().expect("lang");
    let ids: Vec<String> = args.collect();
    let (_, models) = helpers_native::lint_train::ensure_models(
        &[lang.clone()],
        &data,
        &root,
        &helpers_native::lint_train::NoProject,
    );
    let model = models.get(&lang).expect("model");
    if ids.is_empty() {
        // No ids ⇒ the full inventory: every compiled rule and what it watches.
        let mut all: Vec<&str> = model.rules.rule_ids().collect();
        all.sort();
        for id in all {
            println!("{id}: {:?}", model.rules.detector_of(id));
        }
        return;
    }
    for id in ids {
        println!(
            "{id}: detector={:?} degenerate={}",
            model.rules.detector_of(&id),
            model.rules.degenerate_detector(&id)
        );
    }
}
