//! THROWAWAY PROBE (untracked): COMPLETION PASS 17 rung 2 — MODULE ISOLATION ("like C: don't pay
//! for what we don't use"). Audits the live lint load path: for a single-language project, which
//! per-language artifacts load (bytes), the wall time, and — the isolation claim — that REMOVING one
//! language's module changes NOTHING for another. Run against a throwaway HELPERS_LINT_MODELS copy so
//! the real model cache is never mutated.
//! Run: `cargo run --release --features crawl --example module_isolation`
use serde_json::json;
use std::path::{Path, PathBuf};

fn dir_bytes(dir: &Path, prefix: &str) -> (u64, usize) {
    let mut bytes = 0u64;
    let mut n = 0usize;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with(prefix) {
                if let Ok(m) = e.metadata() { bytes += m.len(); n += 1; }
            }
        }
    }
    (bytes, n)
}

fn lint_json(root: &Path) -> String {
    match helpers_native::tools::lint::run(&json!({ "root": root.display().to_string() })) {
        Ok(contents) => contents.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>().join("\n"),
        Err(e) => format!("ERR: {e}"),
    }
}

fn write(p: &Path, body: &str) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, body).unwrap(); }

fn main() {
    let tmp = std::env::temp_dir().join(format!("modiso-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);

    // Copy the real model cache into a throwaway dir we can freely delete language files from.
    let real_models = {
        let home = std::env::var("HOME").unwrap();
        PathBuf::from(home).join(".cache/helpers/lint-models")
    };
    let models = tmp.join("models");
    std::fs::create_dir_all(&models).unwrap();
    let mut copied = 0u64;
    if let Ok(rd) = std::fs::read_dir(&real_models) {
        for e in rd.flatten() {
            if e.path().is_file() {
                let _ = std::fs::copy(e.path(), models.join(e.file_name()));
                copied += 1;
            }
        }
    }
    std::env::set_var("HELPERS_LINT_MODELS", &models);
    println!("copied {copied} model files into throwaway {}", models.display());

    // ── the per-language artifact inventory ──
    println!("\n== per-language artifact bytes on disk (model cache) ==");
    println!("{:>12} {:>10} {:>7}", "language", "bytes", "files");
    let langs = ["javascript", "css", "html", "python", "rust", "typescript", "go", "bash"];
    for l in langs {
        let (b, n) = dir_bytes(&models, &format!("{l}."));
        if n > 0 { println!("{l:>12} {b:>10} {n:>7}", ); }
    }
    let (total, tn) = dir_bytes(&models, "");
    println!("{:>12} {total:>10} {tn:>7}  (WHOLE model dir — every language + globals)", "ALL");
    for g in ["char.global.bin", "english.global.bin", "polarity.global.bin", "extensions.bin"] {
        let sz = std::fs::metadata(models.join(g)).map(|m| m.len()).unwrap_or(0);
        println!("   global {g:24} {sz:>10}", );
    }

    // ── single-language project: pure JS ──
    let js = tmp.join("js_proj");
    write(&js.join("a.js"), "var x = 1;\nif (x == 1) { document.write('hi'); }\neval('2+2');\n");
    write(&js.join("b.js"), "let y = 2;\nconst z = y === 2;\n");
    // Warm any process-global memoization first (brain, extensions) so the timed run measures steady state.
    let _ = lint_json(&js);
    let t = std::time::Instant::now();
    let out_js = lint_json(&js);
    let dt_js = t.elapsed();
    println!("\n== single-language (pure JS) lint ==");
    println!("wall (warm) = {:.1}ms", dt_js.as_secs_f64() * 1e3);
    println!("--- findings ---\n{}", first_lines(&out_js, 12));

    // ── ISOLATION: remove CSS (+ html/python/rust) module files, re-lint JS — must be BYTE-IDENTICAL ──
    for victim in ["css", "html", "python", "rust", "typescript", "go", "bash", "c", "java", "ruby"] {
        if let Ok(rd) = std::fs::read_dir(&models) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with(&format!("{victim}.")) { let _ = std::fs::remove_file(e.path()); }
            }
        }
    }
    let (after, an) = dir_bytes(&models, "");
    println!("\n== after removing every NON-JS language module ({} files, {} bytes remain) ==", an, after);
    let out_js2 = lint_json(&js);
    println!("JS findings identical after stripping all other languages: {}", out_js == out_js2);
    if out_js != out_js2 {
        println!("  !! DIFF !!\n--- before ---\n{}\n--- after ---\n{}", first_lines(&out_js, 12), first_lines(&out_js2, 12));
    }

    // ── and the removed language is genuinely gone: a CSS project now finds no css module ──
    let css = tmp.join("css_proj");
    write(&css.join("a.css"), "a { color: red; float: left; }\n");
    let out_css = lint_json(&css);
    println!("\nCSS project after css module removed (should report css not set up / zero css rules):");
    println!("{}", first_lines(&out_css, 8));

    let _ = std::fs::remove_dir_all(&tmp);
}

fn first_lines(s: &str, n: usize) -> String {
    s.lines().take(n).collect::<Vec<_>>().join("\n")
}
