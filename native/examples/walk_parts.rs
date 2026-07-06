//! Splits the fused walk's cost: raw batched enumeration (no ignore rules), the full walk,
//! and the witness fold. Run: `cargo run --release --example walk_parts -- <root>`.

use std::path::{Path, PathBuf};
use std::time::Instant;

fn raw(dir: &Path, out: &mut usize) {
    let entries = helpers_native::index::walk::scan_dir(dir);
    let mut subdirs: Vec<PathBuf> = Vec::new();
    for e in entries {
        if helpers_native::index::walk::SKIP_DIRS.contains(&e.name.as_str()) {
            continue;
        }
        if e.is_dir {
            subdirs.push(dir.join(&e.name));
        } else if e.is_file {
            *out += 1;
        }
    }
    for d in subdirs {
        raw(&d, out);
    }
}

fn raw_par(dir: &Path) -> usize {
    let entries = helpers_native::index::walk::scan_dir(dir);
    let mut subdirs: Vec<PathBuf> = Vec::new();
    let mut n = 0usize;
    for e in entries {
        if helpers_native::index::walk::SKIP_DIRS.contains(&e.name.as_str()) {
            continue;
        }
        if e.is_dir {
            subdirs.push(dir.join(&e.name));
        } else if e.is_file {
            n += 1;
        }
    }
    use rayon::prelude::*;
    n + subdirs.par_iter().map(|d| raw_par(d)).sum::<usize>()
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    for round in 0..4 {
        let t = Instant::now();
        let mut n = 0usize;
        raw(&root, &mut n);
        let d_serial = t.elapsed();

        let t = Instant::now();
        let np = raw_par(&root);
        let d_par = t.elapsed();

        let t = Instant::now();
        let files = helpers_native::index::walk::walk_repo(&root);
        let d_full = t.elapsed();

        let t = Instant::now();
        let w = helpers_native::lint_replay::walk_witness(&files);
        let d_fold = t.elapsed();

        println!(
            "round {round}: raw-serial {:.2}ms ({n}), raw-par {:.2}ms ({np}), full walk {:.2}ms ({}), fold {:.3}ms (newest {})",
            d_serial.as_secs_f64() * 1e3,
            d_par.as_secs_f64() * 1e3,
            d_full.as_secs_f64() * 1e3,
            files.len(),
            d_fold.as_secs_f64() * 1e3,
            w.newest,
        );
    }
}
