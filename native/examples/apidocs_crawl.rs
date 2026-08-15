//! THROWAWAY CORPUS-ACQUISITION HARNESS (untracked): COMPLETION PASS 14 rung 1. Politely, breadth-first,
//! budget-capped crawl the two API-doc corpora that CARRY the rendered deprecation markers — the Python
//! stdlib library reference (`docs.python.org/3/library/`, the `<div class="deprecated">` carrier) and the
//! Rust std API docs (`doc.rust-lang.org/std/`, the `class="stab deprecated"` badge carrier) — and write
//! each as an HLM1 CRAWL container beside the other crawls, RAW HTML preserved (the prior python-library
//! cache stored prose only, so the class markers were lost). Reports pages fetched + marker-bearing pages.
//! Run: `cargo run --release --features crawl --example apidocs_crawl`
use helpers_native::doc_crawler;
use helpers_native::lint_codec::{self, Enc};
use helpers_native::lint_train;

/// One source to acquire: its cache tool-id, the seed URL, and the page budget.
struct Job {
    tool: &'static str,
    seed: &'static str,
    max_pages: usize,
}

/// The two class-token markers we expect the rendered variant to discover (recon-verified live). Used
/// ONLY for the acquisition report here — the library discovers them from data, never from this list.
fn marker_hits(body: &str) -> bool {
    let lb = body.to_ascii_lowercase();
    lb.contains("class=\"deprecated\"")
        || lb.contains("versionmodified deprecated")
        || lb.contains("stab deprecated")
}

fn crawl_path(tool: &str) -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home)
        .join(".cache/helpers/lint-index/crawls")
        .join(format!("{tool}.bin"))
}

fn write_crawl(tool: &str, version: &str, pages: &[doc_crawler::Page]) {
    let mut e = Enc::new();
    e.str(version);
    e.fixed_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    );
    e.u(pages.len() as u64);
    for p in pages {
        e.str(&p.url);
        e.str(&p.html);
        e.boolean(p.modified.is_some());
        e.fixed_u64(p.modified.unwrap_or(0));
        e.fixed_u64(0);
    }
    let bytes = e.finish(lint_codec::kind::CRAWL, version);
    let path = crawl_path(tool);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, &bytes).expect("write crawl cache");
    // Retire the prose-era JSON twin so exactly one copy lives on disk.
    let _ = std::fs::remove_file(path.with_extension("json"));
    eprintln!("wrote {} ({} bytes)", path.display(), bytes.len());
}

fn main() {
    let version = lint_train::train_version();
    let jobs = [
        Job { tool: "python-library", seed: "https://docs.python.org/3/library/", max_pages: 600 },
        Job { tool: "rust-std", seed: "https://doc.rust-lang.org/std/", max_pages: 1400 },
    ];
    println!("== COMPLETION PASS 14 rung 1 — API-doc corpus acquisition (version {version}) ==\n");
    for job in jobs {
        let t = std::time::Instant::now();
        let pages = doc_crawler::crawl(&[job.seed], job.max_pages, 0);
        let secs = t.elapsed().as_secs_f64();
        if pages.is_empty() {
            println!("{:16} seed={} FETCHED 0 pages (offline/blocked?)", job.tool, job.seed);
            continue;
        }
        let marker_pages = pages.iter().filter(|p| marker_hits(&p.html)).count();
        let bytes: usize = pages.iter().map(|p| p.html.len()).sum();
        write_crawl(job.tool, version, &pages);
        println!(
            "{:16} seed={}\n    {} pages / {:.1} MiB raw HTML / {:.1}s / {} marker-bearing pages\n",
            job.tool,
            job.seed,
            pages.len(),
            bytes as f64 / (1024.0 * 1024.0),
            secs,
            marker_pages,
        );
    }
}
