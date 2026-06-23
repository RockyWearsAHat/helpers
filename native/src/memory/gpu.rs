//! `memory::gpu` — the similarity kernel that makes retrieval fast **without any training**.
//!
//! Retrieval's hot path is one popcount-heavy operation: the Hamming distance from a query
//! fingerprint to every stored item's fingerprint (8192-bit vectors). It is float-free,
//! embarrassingly parallel, and — crucially — needs no learned weights, because the
//! fingerprints come from hashing, not training. That is exactly a GPU's job.
//!
//! What's different from the older `lint_gpu` kernel (which collapsed each query to a single
//! *min* distance, for training): retrieval needs the **whole distance row** so the caller
//! can fuse other signals and pick top-k. So this kernel keeps one thread per *key* and
//! writes back every distance — full parallelism across the key set, no serial inner loop
//! over keys, and a result the retriever can re-rank.
//!
//! Honest hardware note: on Apple-Silicon unified memory the rayon CPU path
//! ([`cpu_query_distances`]) usually matches or beats the GPU for the item counts a single
//! session holds, because dispatch/readback overhead dominates a few-thousand-vector scan.
//! So [`query_distances`] is the public entry point and it *chooses*: CPU below a size
//! threshold, GPU above it, with bit-identical results either way. "Fast" here means "always
//! takes the faster path," not "always the GPU."

use crate::lint_ai::Hv;

/// Above this many keys, the GPU dispatch is worth its overhead; below it, rayon-CPU wins.
/// Tuned conservatively — correctness is identical on both sides of the line, so an
/// imperfect threshold only ever costs a little speed, never accuracy.
const GPU_WORTH_IT: usize = 50_000;

/// Hamming distance from `query` to every key, choosing the faster backend automatically.
/// Result `out[i]` is the distance to `keys[i]`, identical regardless of backend.
pub fn query_distances(query: &Hv, keys: &[Hv]) -> Vec<u32> {
    if keys.is_empty() {
        return Vec::new();
    }
    #[cfg(feature = "gpu")]
    {
        if keys.len() >= GPU_WORTH_IT {
            if let Some(d) = gpu_query_distances(query, keys) {
                return d;
            }
        }
    }
    let _ = GPU_WORTH_IT; // referenced even when the gpu feature is off
    cpu_query_distances(query, keys)
}

/// The portable CPU path: a parallel popcount scan. Always available, always correct, and
/// the default on hardware where the GPU dispatch does not pay for itself.
pub fn cpu_query_distances(query: &Hv, keys: &[Hv]) -> Vec<u32> {
    use rayon::prelude::*;
    keys.par_iter().map(|k| query.distance(k)).collect()
}

/// u32 lanes per hypervector (8192 bits ÷ 32). WGSL has no 64-bit popcount, so vectors are
/// packed as 32-bit lanes the shader counts directly.
#[cfg(feature = "gpu")]
const WORDS_U32: usize = crate::lint_ai::DIM / 32;

/// One query against all keys on the GPU: one thread per key computes `popcount(q XOR key)`
/// and writes the distance. Returns `None` if no usable GPU is present so the caller falls
/// back to the CPU path. Bit-identical to [`cpu_query_distances`].
#[cfg(feature = "gpu")]
pub fn gpu_query_distances(query: &Hv, keys: &[Hv]) -> Option<Vec<u32>> {
    pollster::block_on(run(query, keys))
}

#[cfg(feature = "gpu")]
const SHADER: &str = r#"
@group(0) @binding(0) var<storage, read>       query: array<vec4<u32>>;
@group(0) @binding(1) var<storage, read>       keys:  array<vec4<u32>>;
@group(0) @binding(2) var<storage, read_write> out:   array<u32>;
@group(0) @binding(3) var<uniform>             params: vec2<u32>; // (n_keys, vec4_per_vec)

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let w  = params.y;
    let ki = gid.x;
    if (ki >= params.x) { return; }
    let kbase = ki * w;
    var d: u32 = 0u;
    for (var i: u32 = 0u; i < w; i = i + 1u) {
        let c = countOneBits(query[i] ^ keys[kbase + i]);
        d = d + c.x + c.y + c.z + c.w;
    }
    out[ki] = d;
}
"#;

#[cfg(feature = "gpu")]
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

#[cfg(feature = "gpu")]
async fn run(query: &Hv, keys: &[Hv]) -> Option<Vec<u32>> {
    use wgpu::util::DeviceExt;

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
                label: Some("memory-retrieval"),
                required_features: wgpu::Features::empty(),
                required_limits: adapter.limits(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )
        .await
        .ok()?;

    let q = pack(std::slice::from_ref(query));
    let k = pack(keys);
    let params = [keys.len() as u32, (WORDS_U32 / 4) as u32];
    let out_bytes = (keys.len() * std::mem::size_of::<u32>()) as u64;

    let qbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("query"),
        contents: bytemuck::cast_slice(&q),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let kbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("keys"),
        contents: bytemuck::cast_slice(&k),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let pbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("params"),
        contents: bytemuck::cast_slice(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let obuf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("out"),
        size: out_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging"),
        size: out_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("retrieval-hamming"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("retrieval-hamming"),
        layout: None,
        module: &module,
        entry_point: "main",
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: qbuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: kbuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: obuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: pbuf.as_entire_binding() },
        ],
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        let groups = (keys.len() as u32).div_ceil(64);
        pass.dispatch_workgroups(groups, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&obuf, 0, &staging, 0, out_bytes);
    queue.submit(Some(encoder.finish()));

    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().ok()?.ok()?;
    let data = slice.get_mapped_range();
    let result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging.unmap();
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint_ai::bind;

    #[test]
    fn cpu_distances_match_direct() {
        let q = bind(&["a", "==", "true"]);
        let keys = [bind(&["a", "==", "true"]), bind(&["x", "+", "y"])];
        let d = cpu_query_distances(&q, &keys);
        assert_eq!(d[0], 0, "identical vector is distance 0");
        assert_eq!(d, keys.iter().map(|k| q.distance(k)).collect::<Vec<_>>());
    }
}
