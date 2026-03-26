# GPU Computing — CUDA, Parallelism Model, Memory, and Tensor Cores

## Overview

GPUs are specialized processors optimized for massive data parallelism, not low-latency sequential execution. A modern GPU contains thousands of small cores that execute the same instruction on different data simultaneously. Understanding the GPU execution model, memory hierarchy, and architectural features is crucial for high-performance computing, machine learning, and graphics workloads.

## GPU vs CPU: Fundamental Difference

**CPUs**: Few (~8-64 cores), complex per-core, sophisticated caching and branch prediction. Optimized for latency.

**GPUs**: Thousands (2,000-10,000) of simple cores, minimal branch prediction, simple caches, high memory bandwidth. Optimized for throughput.

A GPU trades per-core complexity for massive parallelism. While a CPU stalls on a cache miss, a GPU context-switches to another warp and overlaps latency.

## CUDA Programming Model

NVIDIA's **CUDA** (Compute Unified Device Architecture) is the de facto standard for GPU computing. It abstracts GPU execution as:

### Grid → Block → Thread Hierarchy

```
Grid: Collection of blocks executing the same kernel
  ├─ Block 0: Collection of threads executing together
  │   ├─ Thread 0
  │   ├─ Thread 1
  │   └─ ...
  ├─ Block 1
  └─ ...
```

**Grid**: Logical layout of thread blocks. Size defined by programmer (e.g., `(32, 32)` = 1,024 blocks).

**Block**: A work group of threads that execute on the same streaming multiprocessor (SM). Max **1,024 threads per block** (typical). Threads in a block can synchronize and share fast memory.

**Thread**: Individual parallel execution unit. Each thread executes the same kernel code but with different inputs (`blockIdx`, `threadIdx`).

### Kernel Execution Example

```c
__global__ void add_kernel(float *A, float *B, float *C, int N) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < N) C[i] = A[i] + B[i];
}

// Launch: 256 threads per block, 1000+ blocks for large N
add_kernel<<<(N+255)/256, 256>>>(A, B, C, N);
```

Thread 0 of block 0 computes element 0, thread 0 of block 1 computes element 256, etc. Threads in different blocks **cannot synchronize**; blocks execute in any order.

## Warp Execution

Hardware doesn't manage individual threads; it manages **warps**—groups of 32 consecutive threads.

### Warp Scheduling

All 32 threads in a warp:
- Execute the **same instruction** on different data (**SIMT**: Single Instruction Multiple Threads)
- Read/write different registers and local memory
- Can make independent decisions (if/else), but diverged threads serialize

### Branch Divergence

```cuda
if (threadIdx.x < 16) {
    // Execute this when warp threads 0-15 branch here
    result = fastPath();
} else {
    // Execute this when warp threads 16-31 branch here
    result = slowPath();
}
```

**Branch divergence penalty**: Both paths execute serially; threads in the opposite path are masked (disabled). On a 32-thread warp, full divergence halves throughput.

**Best practice**: Keep branches inside a warp **coherent**. If all threads in a warp need the same branch, divergence is free.

## Memory Hierarchy on GPUs

GPU memory is a pyramid of bandwidth vs latency.

### Registers

**Per-thread**, **64 bytes - 256 bytes** typical. **Ultra-fast** (~2-3 cycle latency), but limited. Spill to local memory on excess.

### Shared Memory

**Per-block**, **96 KB - 160 KB** typical (Ampere). **~30 cycle latency** (much faster than global). Accessed by all threads in the block.

**Shared memory is fast but small**. Common pattern: load global data into shared memory, then access it quickly.

```cuda
__shared__ float tile[256];
tile[threadIdx.x] = input[global_index];  // Global memory load (slow)
__syncthreads();                           // Synchronize block
// Process tile[...] with low latency
```

### Global Memory

**GPU VRAM**, **4 GB - 80 GB** typical. **~200-400 cycle latency**. Accessed by all threads. Uncached or weakly cached depending on generation.

### Local Memory

Spilled registers and large per-thread arrays. Stored in global memory but addressed per-thread. Same **~200-400 cycle latency** as global.

## Memory Coalescing

Global memory is accessed via **cache lines** (32 bytes per transaction on older GPUs, 128 bytes on newer). Multiple threads' accesses to nearby addresses coalesce into fewer transactions.

### Coalesced Access Pattern

```cuda
// Good: Thread i accesses element i (consecutive addresses)
int idx = blockIdx.x * blockDim.x + threadIdx.x;
float x = global_array[idx];  // Threads 0-31 access consecutive floats

// Bad: Thread i accesses element i*32 (stride-32 access)
float x = global_array[idx * 32];  // Only 1 of 32 threads per warp hits each transaction
```

**Coalesced**: 32 threads achieve 1 global memory transaction → ~128 bytes per warp.

**Strided**: 32 threads require 32 transactions → 4 bytes per thread, 7.7x slower.

Compiler optimizations and careful memory layout (interleaving data) are essential.

## GPU Occupancy

**Occupancy** = (active warps per SM) / (max warps per SM).

Modern Ampere GPUs: Up to **48 warps per SM** (1,536 threads). Achieving 100% occupancy requires 48 active warps.

**Occupancy limits**:
- Registers per thread: Kernel using 80+ registers → fewer threads fit → low occupancy
- Shared memory: Kernel using 80 KB → only 2 blocks fit per SM → low occupancy
- Thread blocks: If you launch only 1 block, 1 SM is active; 100+ idle

**High occupancy ≠ high performance**. A memory-bound kernel with 50% occupancy and better cache locality may outperform 100% occupancy with poor access patterns.

## Tensor Cores

**Tensor Cores** (introduced in Volta, 2017) accelerate matrix operations, particularly for AI/ML.

### Operation

```
Tensor Core operation on Ampere: D = A @ B + C
Input:  A (16x16 or 8x16 FP32), B (16x16 or 8x16 FP32)
Output: D (16x16 or 8x16 FP32)
Cost:   1 cycle (vs ~256 cycles for software)
```

Tensor Cores are specialized floating-point units optimized for low-precision arithmetic: TF32 (32-bit tensor float), bfloat16, FP16, INT8, INT4.

### Performance Multiplier

A single Ampere A100 GPU:
- 432 Tensor Cores per SM × 108 SMs = ~46,000 Tensor Cores
- Peak FP32 performance: ~40 TFLOPS (tera floating-point operations per second)
- Peak TF32 performance (Tensor Cores): ~312 TFLOPS (7.8× speedup)

**Trade-off**: Reduced precision (TF32 → FP16) speeds up matrix multiplication but may reduce model accuracy. Mixed-precision training uses FP16 for compute, FP32 for gradient accumulation.

## GPU Scheduling and Asynchronous Operations

NVIDIA GPUs support **asynchronous data copies**, decoupling host (CPU) and device (GPU) execution.

```cuda
cudaMemcpyAsync(device_data, host_data, size, cudaMemcpyHostToDevice, stream);
kernel<<<blocks, threads>>>(device_data);  // Can execute while copy proceeds
cudaMemcpyAsync(host_result, device_result, size, cudaMemcpyDeviceToHost, stream);
```

**Streams**: Independent command queues. Multiple kernels can execute on the same GPU if they don't depend on each other and occupancy allows.

**Limitation**: Most consumer GPUs have limited bandwidth to PCIe (16 GB/s on Gen4) and to GPU memory (900+ GB/s on A100), so CPU-GPU transfers are often the bottleneck.

## GPU Memory Management

NVIDIA GPUs manage their own memory independently from CPU virtual memory. Unified memory (CUDA 6.0+) provides a single address space abstracting CPU/GPU copies, but automatic transfers have overhead.

### Pinned (Pageable) Memory

**Pinned memory** is locked in CPU RAM and can be DMA'd to GPU quickly. **Unpinned memory** must be copied to a temporary pinned buffer, adding latency.

**Cost**: Pinned memory is a scarce resource (kernel sets a limit). Excessive pinning reduces CPU cache performance.

## Compute Shaders and Vulkan/OpenGL Compute

Beyond CUDA, GPUs support general-purpose compute via graphics APIs.

### Vulkan Compute

```glsl
#version 450
layout(local_size_x = 256) in;
layout(std430, binding = 0) buffer Data { float[] data; };

void main() {
    uint idx = gl_GlobalInvocationID.x;
    data[idx] *= 2.0;
}
```

Compute shaders are **mandatory in Vulkan** (hardware requirement), making them portable across providers (NVIDIA, AMD, Intel). Smaller overhead than CUDA for APIs but less direct memory control.

### OpenGL Compute

Similar model but older. Less optimization opportunity, higher overhead.

## GPU Scheduling Decisions

GPUs schedule warps to hide latency. On an L3 cache miss, the SM context-switches to another warp instead of stalling.

**Consequence**: Sequential algorithms that rely on low latency perform poorly on GPU; embarrassingly parallel algorithms excel.

## See Also

- [SIMD and Vector Instructions](hardware-cpu-architecture.md) — related data parallelism on CPUs
- [System Design: Distributed Computing](system-design-distributed.md) — scaling GPU workloads
- [Machine Learning Training Basics](paradigm-machine-learning.md) — GPUs for AI