//! throwaway: why does lint_web::load("python") return 0?
fn main() {
    let home = std::env::var("HOME").unwrap();
    let p = format!("{home}/.cache/helpers/lint-models/python.web.bin");
    let bytes = std::fs::read(&p).unwrap();
    println!("file {} bytes", bytes.len());
    match helpers_native::lint_codec::Dec::open(&bytes, helpers_native::lint_codec::kind::WEB) {
        Some((stamp, _d)) => println!("stamp=[{stamp}] current=[{}]", helpers_native::lint_train::train_version()),
        None => println!("Dec::open FAILED (container kind mismatch?)"),
    }
}
