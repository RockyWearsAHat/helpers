//! Run the production lint tool (built-in lexer or learned-tokenizer models, whichever is
//! on disk) against a real project and print its report. A harness for validating the
//! linter on actual codebases, not just the documentation corpus.
//!
//!   cargo run --release --example lint_project -- <path> [max]

use serde_json::json;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: lint_project <path> [max] [modules,csv]");
    let max: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(200);
    // Optional comma-separated `modules` filter (e.g. `cs` to run the deterministic floor + CS
    // norms without the official-rule self-setup crawl). Absent ⇒ the tool runs everything.
    let mut req = json!({ "root": path, "max": max });
    if let Some(modules) = args.next() {
        let list: Vec<&str> = modules.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
        req["modules"] = json!(list);
    }
    let res = helpers_native::tools::lint::run(&req);
    match res {
        Ok(content) => {
            for c in content {
                println!("{}", c.text);
            }
        }
        Err(e) => eprintln!("lint error: {e}"),
    }
}
