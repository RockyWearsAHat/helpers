//! `hv_batch` — batched Hamming-distance engine for 8192-bit hypervectors.
//!
//! The concept-confirmation gate ([`crate::lint_ai::ConceptModel::confirms`]) asks one
//! question per matched code construct: *is the fired rule's fingerprint the concept this
//! construct is closest to?* Answering it needs, **per query**, two numbers against the key
//! set (the rule fingerprints): the nearest key's distance, and the distance to the one
//! specified key (the fired rule). Whole-project lint runs pose this for ~10^3–10^4 queries
//! against ~10^3 keys — a dense `M × N` popcount grid.
//!
//! This module computes that grid and reduces it to the per-query answers the gate needs, in
//! **one GPU dispatch per query-chunk** (never one dispatch per query). The GPU does the
//! argmin/target reduction itself, so only `3 × M` u32 cross the bus, not the whole `M × N`
//! matrix. The CPU path is the always-available correctness reference; the GPU path is a
//! bit-identical accelerator that silently falls back to CPU on any device error.
//!
//! Backend choice is by workload size ([`GPU_PAIR_THRESHOLD`]): below it the dispatch/readback
//! overhead loses to the rayon CPU scan, above it the batched dispatch pulls ahead. Measured on
//! this Apple-Silicon machine (`examples/hv_bench.rs` / `hv_probe.rs`): break-even near ~3M
//! `M×N` pairs, rising to ~1.5× the CPU throughput by ~10^7 pairs — which is exactly where the
//! whole-project gate lives (10^4 queries × 10^3 keys). (Rayon popcount on Apple cores is very
//! strong and the key set fits cache, so the win is ~1.5×, not the order-of-magnitude a
//! single-threaded baseline would show.) Correctness is identical on both sides of the line, so
//! an imperfect threshold only ever costs a little speed, never accuracy.

use crate::lint_ai::Hv;

/// Per-query answer for the confirmation gate: the nearest key and the fired rule's distance.
///
/// The gate keeps a finding iff `fired_dist <= nearest_dist` (the fired rule is the — or a
/// tied — closest concept). `nearest_key` is the argmin key index; on an empty key set the
/// row is `{0, u32::MAX, u32::MAX}` and callers treat it as "no evidence" (abstain).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GateRow {
    /// Index into `keys` of the closest key (argmin). Zero when there are no keys.
    pub nearest_key: usize,
    /// Hamming distance to `keys[nearest_key]`. `u32::MAX` when there are no keys.
    pub nearest_dist: u32,
    /// Hamming distance to the fired rule's key (the `fired[qi]` index).
    pub fired_dist: u32,
}

/// Above this many `queries × keys` pairs the batched GPU dispatch pays for its
/// init/readback overhead; below it the rayon CPU scan is faster. Set to the measured
/// break-even point on Apple-Silicon Metal (`examples/hv_probe.rs`): GPU ≈ CPU at ~3M pairs and
/// wins above, loses below. Correctness is identical either side, so a slightly-off threshold
/// only ever costs a little speed.
pub const GPU_PAIR_THRESHOLD: usize = 3_000_000;

/// Per-query nearest key: `out[qi] = (argmin key index, Hamming distance)`.
///
/// Empty `keys` yields `(0, u32::MAX)` per query; empty `queries` yields an empty vector.
/// Ties resolve to the lowest key index. Backend chosen automatically; result is
/// backend-independent.
pub fn nearest(queries: &[Hv], keys: &[Hv]) -> Vec<(usize, u32)> {
    // fired index is irrelevant here; 0 is always in range for a non-empty key set.
    let fired = vec![0usize; queries.len()];
    gate(queries, keys, &fired)
        .into_iter()
        .map(|r| (r.nearest_key, r.nearest_dist))
        .collect()
}

/// Per-query distance to one specified key: `out[qi] = distance(queries[qi], keys[key_idx[qi]])`.
///
/// `key_idx` must have one entry per query, each a valid index into `keys`. Panics on a
/// length mismatch (a caller bug); an out-of-range index yields `u32::MAX` for that row.
pub fn distance_to(queries: &[Hv], keys: &[Hv], key_idx: &[usize]) -> Vec<u32> {
    assert_eq!(queries.len(), key_idx.len(), "one key index per query required");
    gate(queries, keys, key_idx).into_iter().map(|r| r.fired_dist).collect()
}

/// The gate's one-shot query: per query, the nearest key (index + distance) **and** the
/// distance to its fired key, in a single traversal of the grid.
///
/// `fired` holds one key index per query (the rule that fired on that construct). Length must
/// equal `queries`. This is the call the batched `confirms()` should make: build the query and
/// key Hv arrays plus the fired-index vector once, invoke this, then keep row `qi` iff
/// `row.fired_dist <= row.nearest_dist`.
pub fn gate(queries: &[Hv], keys: &[Hv], fired: &[usize]) -> Vec<GateRow> {
    assert_eq!(queries.len(), fired.len(), "one fired key index per query required");
    if queries.is_empty() {
        return Vec::new();
    }
    if keys.is_empty() {
        return vec![GateRow { nearest_key: 0, nearest_dist: u32::MAX, fired_dist: u32::MAX }; queries.len()];
    }
    #[cfg(feature = "gpu")]
    {
        if queries.len().saturating_mul(keys.len()) >= GPU_PAIR_THRESHOLD {
            if let Some(rows) = gpu::gate(queries, keys, fired) {
                return rows;
            }
        }
    }
    cpu_gate(queries, keys, fired)
}

/// The portable, always-correct reference: parallel over queries, one tight u64 XOR+popcount
/// scan over keys per query, tracking argmin and the fired-key distance in the same pass.
pub fn cpu_gate(queries: &[Hv], keys: &[Hv], fired: &[usize]) -> Vec<GateRow> {
    use rayon::prelude::*;
    queries
        .par_iter()
        .zip(fired.par_iter())
        .map(|(q, &f)| {
            let mut best_key = 0usize;
            let mut best_dist = u32::MAX;
            for (ki, k) in keys.iter().enumerate() {
                let d = q.distance(k);
                if d < best_dist {
                    best_dist = d;
                    best_key = ki;
                }
            }
            let fired_dist = keys.get(f).map_or(u32::MAX, |k| q.distance(k));
            GateRow { nearest_key: best_key, nearest_dist: best_dist, fired_dist }
        })
        .collect()
}

#[cfg(feature = "gpu")]
mod gpu {
    //! wgpu compute path, two passes in one submit:
    //!
    //! 1. **distance matrix** — one thread per `(query, key)` pair fills `dist[qi*n_k+ki]`.
    //!    This is where the parallelism lives: a whole-project batch is millions of pairs, so
    //!    millions of threads keep the GPU saturated (a one-thread-per-query reduction, by
    //!    contrast, launches only thousands of threads and loses to the rayon CPU scan).
    //! 2. **reduce** — one thread per query scans its finished row for the argmin key and reads
    //!    the fired key's distance. Cheap comparisons only; the expensive popcount already ran
    //!    fully parallel in pass 1.
    //!
    //! The matrix stays in GPU memory between passes (never read back — only the `3 × M` result
    //! crosses the bus). Device/queue/pipelines build once (OnceLock); each call only allocates
    //! buffers and dispatches. Query-chunked so no single submit can hog the GPU past the macOS
    //! compositor watchdog window.

    use super::{GateRow, Hv};
    use std::sync::OnceLock;
    use wgpu::util::DeviceExt;

    /// u32 lanes per hypervector (8192 bits ÷ 32); WGSL has no 64-bit popcount, so vectors are
    /// packed as 32-bit lanes and read four-at-a-time as `vec4<u32>`.
    const WORDS_U32: usize = crate::lint_ai::DIM / 32;
    /// `vec4<u32>` units per hypervector — the distance shader's inner-loop bound.
    const VEC4_PER_VEC: usize = WORDS_U32 / 4;

    /// Max `queries × keys` pairs per submit. Keeps each submit's matrix pass well under the
    /// ~100ms the macOS WindowServer GPU watchdog tolerates, so a large batch never risks a
    /// compositor kill — it is split across several small submits instead. Also bounds the
    /// intermediate matrix buffer (`pairs × 4` bytes) far under any device buffer limit.
    const PAIRS_PER_DISPATCH: usize = 8_000_000;

    /// Pass 1: one thread per `(query, key)` pair → `dist[qi*n_k+ki] = popcount(q XOR key)`.
    ///
    /// A 2D dispatch dodges the 65535-per-dimension group limit; the linear thread id is
    /// `gid.y * x_stride + gid.x`, with `x_stride` (= `gx * 64`) passed in `params.w`. This flat
    /// per-pair form is deliberately un-tiled: the key set (~1.5 MB) already fits the GPU's L2, so
    /// staging rows into threadgroup memory only adds barrier overhead here without cutting real
    /// traffic — measured slower than this straight-line kernel on Apple-Silicon.
    const DIST_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read>       queries: array<vec4<u32>>;
@group(0) @binding(1) var<storage, read>       keys:    array<vec4<u32>>;
@group(0) @binding(2) var<storage, read_write> dist:    array<u32>;
@group(0) @binding(3) var<uniform>             params:  vec4<u32>; // (n_q, n_k, w, x_stride)

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n_q      = params.x;
    let n_k      = params.y;
    let w        = params.z;
    let x_stride = params.w;
    let thread   = gid.y * x_stride + gid.x;
    let qi = thread / n_k;
    let ki = thread % n_k;
    if (qi >= n_q) { return; }
    let qbase = qi * w;
    let kbase = ki * w;
    var d: u32 = 0u;
    for (var i: u32 = 0u; i < w; i = i + 1u) {
        let c = countOneBits(queries[qbase + i] ^ keys[kbase + i]);
        d = d + c.x + c.y + c.z + c.w;
    }
    dist[qi * n_k + ki] = d;
}
"#;

    /// Pass 2: one thread per query scans its `dist` row for the argmin key and reads the fired
    /// key's distance, writing three u32 (`nearest_key`, `nearest_dist`, `fired_dist`) per query.
    /// Ties keep the lowest key index (strict `<`), matching the CPU reference.
    const REDUCE_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read>       dist:  array<u32>;
@group(0) @binding(1) var<storage, read_write> out:   array<u32>; // 3 per query
@group(0) @binding(2) var<uniform>             params: vec4<u32>;  // (n_q, n_k, _, _)
@group(0) @binding(3) var<storage, read>       fired: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n_q = params.x;
    let n_k = params.y;
    let qi  = gid.x;
    if (qi >= n_q) { return; }
    let base = qi * n_k;
    let tgt  = fired[qi];
    var best_key: u32 = 0u;
    var best: u32 = 0xffffffffu;
    for (var ki: u32 = 0u; ki < n_k; ki = ki + 1u) {
        let d = dist[base + ki];
        if (d < best) { best = d; best_key = ki; }
    }
    var tgt_d: u32 = 0xffffffffu;
    if (tgt < n_k) { tgt_d = dist[base + tgt]; }
    out[qi * 3u + 0u] = best_key;
    out[qi * 3u + 1u] = best;
    out[qi * 3u + 2u] = tgt_d;
}
"#;

    /// Initialized-once GPU context: the device, its queue, and both compiled pipelines.
    struct Ctx {
        device: wgpu::Device,
        queue: wgpu::Queue,
        dist_pipeline: wgpu::ComputePipeline,
        reduce_pipeline: wgpu::ComputePipeline,
    }

    /// Build (or reuse) the GPU context. `None` when no usable adapter/device exists, which
    /// makes every caller fall back to the CPU reference. Never panics on missing hardware.
    fn ctx() -> Option<&'static Ctx> {
        static CTX: OnceLock<Option<Ctx>> = OnceLock::new();
        CTX.get_or_init(init).as_ref()
    }

    fn pipeline(device: &wgpu::Device, label: &str, src: &str) -> wgpu::ComputePipeline {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(src.into()),
        });
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(label),
            layout: None,
            module: &module,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    }

    fn init() -> Option<Ctx> {
        pollster::block_on(async {
            let instance = wgpu::Instance::default();
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await?;
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("hv-batch-gate"),
                        required_features: wgpu::Features::empty(),
                        required_limits: adapter.limits(),
                        memory_hints: wgpu::MemoryHints::Performance,
                    },
                    None,
                )
                .await
                .ok()?;
            let dist_pipeline = pipeline(&device, "hv-batch-dist", DIST_SHADER);
            let reduce_pipeline = pipeline(&device, "hv-batch-reduce", REDUCE_SHADER);
            Some(Ctx { device, queue, dist_pipeline, reduce_pipeline })
        })
    }

    /// Pack hypervectors into contiguous little-endian u32 lanes for the shader's `vec4<u32>`
    /// view — two lanes per source u64 word, low half first.
    fn pack(vecs: &[Hv]) -> Vec<u32> {
        let mut out = Vec::with_capacity(vecs.len() * WORDS_U32);
        for v in vecs {
            for &word in v.as_words() {
                out.push(word as u32);
                out.push((word >> 32) as u32);
            }
        }
        out
    }

    /// GPU implementation of [`super::gate`]. Returns `None` on any device error so the caller
    /// falls back to the CPU reference. Query-chunked to bound each submit's runtime.
    pub fn gate(queries: &[Hv], keys: &[Hv], fired: &[usize]) -> Option<Vec<GateRow>> {
        let ctx = ctx()?;
        let n_k = keys.len();
        let w = VEC4_PER_VEC as u32;
        // Keys are shared across every chunk — pack and upload once.
        let key_data = pack(keys);
        let kbuf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("keys"),
            contents: bytemuck::cast_slice(&key_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Chunk queries so no single submit exceeds the pair budget (watchdog headroom).
        let chunk_q = (PAIRS_PER_DISPATCH / n_k.max(1)).clamp(1, queries.len().max(1));
        let mut out = Vec::with_capacity(queries.len());

        for (chunk, fchunk) in queries.chunks(chunk_q).zip(fired.chunks(chunk_q)) {
            let n_q = chunk.len();
            let q_data = pack(chunk);
            let fired_u32: Vec<u32> = fchunk.iter().map(|&i| i as u32).collect();
            let pairs = (n_q * n_k) as u32;

            // 2D dispatch geometry for the flat matrix pass (stay under 65535 groups per dim).
            let total_groups = pairs.div_ceil(64);
            let gx = total_groups.min(65535);
            let gy = total_groups.div_ceil(gx.max(1));
            let x_stride = gx * 64;
            let dist_params = [n_q as u32, n_k as u32, w, x_stride];
            let reduce_params = [n_q as u32, n_k as u32, 0u32, 0u32];

            let matrix_bytes = (pairs as u64) * std::mem::size_of::<u32>() as u64;
            let out_bytes = (n_q * 3 * std::mem::size_of::<u32>()) as u64;

            let qbuf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("queries"),
                contents: bytemuck::cast_slice(&q_data),
                usage: wgpu::BufferUsages::STORAGE,
            });
            let fbuf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("fired"),
                contents: bytemuck::cast_slice(&fired_u32),
                usage: wgpu::BufferUsages::STORAGE,
            });
            let dpbuf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("dist-params"),
                contents: bytemuck::cast_slice(&dist_params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let rpbuf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("reduce-params"),
                contents: bytemuck::cast_slice(&reduce_params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let matrix = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("dist-matrix"),
                size: matrix_bytes,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            let obuf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("out"),
                size: out_bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let staging = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("staging"),
                size: out_bytes,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let dist_bg = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &ctx.dist_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: qbuf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: kbuf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: matrix.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: dpbuf.as_entire_binding() },
                ],
            });
            let reduce_bg = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &ctx.reduce_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: matrix.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: obuf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: rpbuf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: fbuf.as_entire_binding() },
                ],
            });

            let mut encoder = ctx
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("dist"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&ctx.dist_pipeline);
                pass.set_bind_group(0, &dist_bg, &[]);
                pass.dispatch_workgroups(gx, gy, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("reduce"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&ctx.reduce_pipeline);
                pass.set_bind_group(0, &reduce_bg, &[]);
                pass.dispatch_workgroups((n_q as u32).div_ceil(64), 1, 1);
            }
            encoder.copy_buffer_to_buffer(&obuf, 0, &staging, 0, out_bytes);
            ctx.queue.submit(Some(encoder.finish()));

            let slice = staging.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
            ctx.device.poll(wgpu::Maintain::Wait);
            rx.recv().ok()?.ok()?;
            let data = slice.get_mapped_range();
            let raw: &[u32] = bytemuck::cast_slice(&data);
            for row in raw.chunks_exact(3) {
                out.push(GateRow {
                    nearest_key: row[0] as usize,
                    nearest_dist: row[1],
                    fired_dist: row[2],
                });
            }
            drop(data);
            staging.unmap();
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint_ai::Hv;

    /// Deterministic pseudo-random Hv set for tests, seeded so runs reproduce.
    fn sample(n: usize, salt: u64) -> Vec<Hv> {
        (0..n).map(|i| Hv::random((i as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ salt)).collect()
    }

    fn brute_gate(queries: &[Hv], keys: &[Hv], fired: &[usize]) -> Vec<GateRow> {
        queries
            .iter()
            .zip(fired)
            .map(|(q, &f)| {
                let mut bk = 0usize;
                let mut bd = u32::MAX;
                for (ki, k) in keys.iter().enumerate() {
                    let d = q.distance(k);
                    if d < bd {
                        bd = d;
                        bk = ki;
                    }
                }
                GateRow {
                    nearest_key: bk,
                    nearest_dist: bd,
                    fired_dist: keys.get(f).map_or(u32::MAX, |k| q.distance(k)),
                }
            })
            .collect()
    }

    #[test]
    fn cpu_matches_brute_force() {
        let q = sample(37, 1);
        let k = sample(23, 2);
        let fired: Vec<usize> = (0..q.len()).map(|i| i % k.len()).collect();
        assert_eq!(cpu_gate(&q, &k, &fired), brute_gate(&q, &k, &fired));
    }

    #[test]
    fn empty_queries_empty_result() {
        assert!(nearest(&[], &sample(5, 3)).is_empty());
        assert!(distance_to(&[], &sample(5, 3), &[]).is_empty());
    }

    #[test]
    fn empty_keys_abstains() {
        let q = sample(4, 4);
        let rows = gate(&q, &[], &[0, 0, 0, 0]);
        assert!(rows.iter().all(|r| r.nearest_dist == u32::MAX && r.fired_dist == u32::MAX));
    }

    #[test]
    fn single_key_is_always_nearest() {
        let q = sample(10, 5);
        let k = sample(1, 6);
        let n = nearest(&q, &k);
        assert!(n.iter().all(|&(idx, _)| idx == 0));
    }

    #[test]
    fn all_identical_keys_pick_lowest_index() {
        let base = Hv::random(99);
        let keys = vec![base; 6];
        let q = sample(8, 7);
        for (idx, _) in nearest(&q, &keys) {
            assert_eq!(idx, 0, "ties must resolve to the lowest key index");
        }
    }

    #[test]
    fn identical_query_and_key_is_zero_distance() {
        let k = sample(5, 8);
        let q = vec![k[3]];
        let d = distance_to(&q, &k, &[3]);
        assert_eq!(d[0], 0);
        assert_eq!(nearest(&q, &k)[0], (3, 0));
    }

    #[test]
    fn gate_decision_matches_reference() {
        let q = sample(50, 10);
        let k = sample(40, 11);
        let fired: Vec<usize> = (0..q.len()).map(|i| (i * 7) % k.len()).collect();
        let rows = gate(&q, &k, &fired);
        let brute = brute_gate(&q, &k, &fired);
        assert_eq!(rows, brute);
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn gpu_equals_cpu_when_adapter_present() {
        // Big enough to cross the GPU threshold; skip gracefully when no adapter.
        let q = sample(600, 20);
        let k = sample(400, 21);
        let fired: Vec<usize> = (0..q.len()).map(|i| (i * 13) % k.len()).collect();
        match super::gpu::gate(&q, &k, &fired) {
            None => eprintln!("no GPU adapter — skipping gpu_equals_cpu"),
            Some(gpu_rows) => assert_eq!(gpu_rows, cpu_gate(&q, &k, &fired)),
        }
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn gpu_chunking_spans_multiple_dispatches() {
        // 6000 × 1500 = 9M pairs > PAIRS_PER_DISPATCH (8M), so the query-chunk loop runs twice;
        // verify the stitched-together result still matches the reference exactly.
        let q = sample(6000, 30);
        let k = sample(1500, 31);
        let fired: Vec<usize> = (0..q.len()).map(|i| (i * 3) % k.len()).collect();
        if let Some(rows) = super::gpu::gate(&q, &k, &fired) {
            assert_eq!(rows, cpu_gate(&q, &k, &fired));
        }
    }
}
