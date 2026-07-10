//! Wall-clock bench for the batched Hamming gate: CPU reference vs the auto-selected backend
//! (GPU when built `--features gpu` and the batch clears [`hv_batch::GPU_PAIR_THRESHOLD`]).
//!
//! Run:
//!   cargo run --release --example hv_bench                 # CPU only
//!   cargo run --release --example hv_bench --features gpu  # CPU vs GPU
//!
//! Prints both wall times and the speedup for the canonical 2000 × 1500 workload.

use helpers_native::hv_batch::{self, cpu_gate};
use helpers_native::lint_ai::Hv;
use std::time::Instant;

fn sample(n: usize, salt: u64) -> Vec<Hv> {
    (0..n).map(|i| Hv::random(i as u64 * 0x9E3779B97F4A7C15 ^ salt)).collect()
}

fn main() {
    let n_q = 2000usize;
    let n_k = 1500usize;
    let queries = sample(n_q, 1);
    let keys = sample(n_k, 2);
    let fired: Vec<usize> = (0..n_q).map(|i| (i * 7) % n_k).collect();

    println!("workload: {n_q} queries × {n_k} keys = {} pairs", n_q * n_k);
    println!("GPU_PAIR_THRESHOLD = {}", hv_batch::GPU_PAIR_THRESHOLD);

    // Warm both paths before timing: a full-size gate() so the GPU device/pipelines are built
    // (a tiny warmup would stay under the threshold, run on CPU, and leave device init to leak
    // into the timed call), and a rayon call so the pool is spun up.
    let _ = cpu_gate(&queries, &keys, &fired);
    let _ = hv_batch::gate(&queries, &keys, &fired);

    let t0 = Instant::now();
    let cpu = cpu_gate(&queries, &keys, &fired);
    let cpu_ms = t0.elapsed().as_secs_f64() * 1e3;

    let t1 = Instant::now();
    let auto = hv_batch::gate(&queries, &keys, &fired);
    let auto_ms = t1.elapsed().as_secs_f64() * 1e3;

    assert_eq!(cpu, auto, "auto backend must match the CPU reference bit-for-bit");

    let backend = if cfg!(feature = "gpu") { "gpu(auto)" } else { "cpu(auto)" };
    println!("cpu_gate     : {cpu_ms:8.2} ms");
    println!("{backend:<13}: {auto_ms:8.2} ms");
    if auto_ms > 0.0 {
        println!("speedup      : {:.2}×", cpu_ms / auto_ms);
    }
}
