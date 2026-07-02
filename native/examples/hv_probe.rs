//! Crossover sweep: warmed CPU vs auto(GPU) gate wall time across workload sizes, to pick
//! `hv_batch::GPU_PAIR_THRESHOLD` by measurement. Run with `--features gpu`.
use helpers_native::hv_batch::{self, cpu_gate};
use helpers_native::lint_ai::Hv;
use std::time::Instant;

fn sample(n: usize, s: u64) -> Vec<Hv> {
    (0..n).map(|i| Hv::random(i as u64 * 0x9E3779B97F4A7C15 ^ s)).collect()
}

fn bench(n_q: usize, n_k: usize) {
    let q = sample(n_q, 1);
    let k = sample(n_k, 2);
    let f: Vec<usize> = (0..n_q).map(|i| (i * 7) % n_k).collect();
    // Warm both paths (device init, rayon pool, pipeline build).
    let _ = cpu_gate(&q, &k, &f);
    let _ = hv_batch::gate(&q, &k, &f);
    let mut cms = f64::MAX;
    let mut gms = f64::MAX;
    for _ in 0..3 {
        let t = Instant::now();
        let c = cpu_gate(&q, &k, &f);
        cms = cms.min(t.elapsed().as_secs_f64() * 1e3);
        let t = Instant::now();
        let g = hv_batch::gate(&q, &k, &f);
        gms = gms.min(t.elapsed().as_secs_f64() * 1e3);
        assert_eq!(c, g);
    }
    println!(
        "{n_q:6} x {n_k:5} = {:>10} pairs | cpu {cms:7.2}ms  gpu {gms:7.2}ms  speedup {:.2}x",
        n_q * n_k,
        cms / gms
    );
}

fn main() {
    for &(q, k) in &[
        (500usize, 1000usize),
        (2000, 1500),
        (4000, 1500),
        (8000, 1500),
        (10000, 2000),
        (20000, 2000),
    ] {
        bench(q, k);
    }
}
