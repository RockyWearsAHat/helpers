//! Verify whether the model's "false flags" are actually false: judge every repo Rust
//! file with the trained model and print each flag as `path:line rule`, so it can be
//! cross-referenced against real clippy output. A flag on a line clippy also flags is a
//! REAL finding, not a false positive.
//!
//!   cargo run --release --example verify_fp > /tmp/moe_flags.txt

use std::fs;
use std::path::{Path, PathBuf};

use helpers_native::lint_moe::Moe;

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn main() {
    let moe = match Moe::load(&Moe::model_path("rust")) {
        Some(m) => m,
        None => {
            eprintln!("no rust model — run: cargo run --release --example train_lint rust");
            return;
        }
    };
    let mut files = Vec::new();
    rust_files(Path::new("src"), &mut files);
    let mut total = 0;
    for path in files {
        let Ok(code) = fs::read_to_string(&path) else { continue };
        for (line, rule) in moe.judge_located(&code) {
            total += 1;
            println!("{}:{} {}", path.display(), line, moe.rule_name(rule));
        }
    }
    eprintln!("{total} flags");
}
