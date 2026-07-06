//! Classifies given prose through the machine's best classifier (the same one bind-time
//! lead gating and law reading use) — answers "did this sentence read as a prohibition?".
//! Run: `cargo run --release --example polarity_probe -- <data_root> "<prose>" ...`

fn main() {
    let mut args = std::env::args().skip(1);
    let data = std::path::PathBuf::from(args.next().expect("data_root"));
    let polarity = helpers_native::lint_docs::document_polarity(&data).expect("classifier");
    for prose in args {
        println!("{:?} <- {:?}", polarity.classify(&prose), &prose[..prose.len().min(90)]);
    }
}
