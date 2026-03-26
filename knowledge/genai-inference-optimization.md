# LLM Inference Optimization — Serving at Scale

## The Core Problem: Inference is Bottleneck Work

LLM inference is fundamentally different from training: models are compute-bound during prefill (initial prompt tokenization) but **memory-bound during decode** (where each token is generated one at a time, and the model must access the full KV cache for every new token). Most optimization techniques target this memory bottleneck, which accounts for ~75% of latency in typical deployments.

## Key-Value (KV) Cache Management

Every attention layer stores key and value vectors for recalled tokens. With a 70B parameter model, a single 2048-token sequence can consume 500MB+ of GPU memory just for KV cache.

### Standard Attention Memory Problem
In naive batching, each batch request gets its own KV cache. During generation of token N, the model reads KV cache for tokens 1..N-1, then appends token N's keys/values. Unused memory fragments accumulate when requests finish at different times, forcing **premature batch eviction** and GPU underutilization.

### PagedAttention Solution (vLLM)
PagedAttention treats KV cache as "pages" (e.g., 16 tokens per page), allowing **logical-to-physical remapping** similar to virtual memory. Benefits:
- **Memory pooling**: Finished requests free pages immediately; new requests claim freed pages without reallocation
- **Prefix caching**: Reuse KV cache blocks for identical prefixes (e.g., system prompt present in many requests)
- **KV cache multiplexing**: Run more requests per GPU by reusing pages across sequence prefixes

Performance gain: 2-4x throughput improvement by increasing batch utilization from ~40% to ~85%.

## Continuous Batching and Prefill/Decode Separation

### Problem with Static Batching
Traditional batching waits for all requests in a batch to finish before flushing the GPU. One slow request delays all others (head-of-line blocking).

### Continuous Batching (In-Flight Batching)
- **Decode-first scheduling**: Prioritize completed prefill requests ready for token generation
- **Chunked prefill**: Break long prompts into chunks, interleaving prefill and decode steps so GPU never goes idle
- Modern schedulers achieve **90%+ GPU utilization** compared to ~60% with static batching

Trade-off: Complexity in request tracking and scheduler design, but essential for multi-tenant serving.

## Speculative Decoding

Generate candidate tokens via a small "draft" model (1-3B parameters), then verify them with the full model in parallel. If verification fails, discard candidates and continue normally.

**Mechanism**:
1. Draft model generates K candidate tokens
2. Full model processes all K candidates + context in one forward pass
3. Full model's attention logits validate draft predictions
4. Accept all matching tokens at once; reject and resample on first mismatch

**Impact**: 1.5-2x latency reduction on tasks with predictable outputs (code generation, instruction following). Lower gain on creative tasks where model entropy is high.

## Quantization for Inference

Reducing precision trades accuracy for speed and memory. For inference, only weights and activations need quantization (no gradient computation).

### Post-Training Quantization (PTQ)
- **GPTQ**: Quantize weights to INT4 layer-by-layer, minimizing reconstruction error. Requires calibration dataset (~500 examples). ~4x compression, negligible accuracy loss
- **AWQ (Activation-Aware Quantization)**: Identify and preserve high-activation channels at higher precision. Better accuracy than GPTQ at same bitwidth
- **GGUF**: Quantization format designed for edge deployment; supports multiple precisions (Q4, Q5, Q6). Popular in llama.cpp ecosystem

### Dynamic Quantization
Quantize weights to INT8/INT4, keep activations in FP16. Simpler than PTQ but slightly higher overhead per token.

### Supported Hardware
- **NVIDIA GPUs**: Native INT8/FP8 with Tensor Cores (Ampere+); INT4 requires custom kernels (GPTQ, AWQ)
- **CPU-only**: GGUF format with optimized quantized matmul in llama.cpp
- **Edge TPUs, mobile**: Limited to fully quantized INT8

Trade-off: Accuracy floor. 4-bit quantization may drop performance 5-15% depending on task criticality.

## Flash Attention

Standard scaled dot-product attention has I/O complexity that dominates GPU memory bandwidth. Flash Attention restructures the computation:

- **Standard**: Compute QK^T (quadratic in sequence length), softmax, multiply by V
- **Flash Attention**: Tile the computation to keep intermediate results in fast SRAM, perform quantization-aware softmax in-register

**Impact**: 3-10x speedup on medium/long sequences (512+ tokens). Especially effective for long-context inference where cache misses are expensive.

Variants:
- **Flash Attention v2**: Further optimizations; now standard in most frameworks
- **Flash Decoding**: Specialized for decode-phase (one token at a time)

## Parallelism Strategies

### Tensor Parallelism for Inference
Split weight matrices across GPUs; each forward pass requires allreduce synchronization. Adds communication overhead (~10-20% latency overhead on 8x GPU cluster).

**When worth it**: Models that don't fit on a single GPU (135B+). For smaller models, sharding increases latency.

### Pipeline Parallelism
Split model layers across GPUs. GPU0 processes layer 0, GPU1 processes layer 1, etc. Requests flow through pipeline.

**Pros**: Reduces per-GPU memory requirement; hides communication behind computation
**Cons**: Low batch size → pipeline bubbles; high variance in latency per request

### Disaggregated Prefill/Decode
Separate GPU pools for prefill and decode phases. Prefill cluster uses aggressive batching; decode cluster optimizes for low latency. Intermediate results (KV cache) moved between clusters.

Trade-off: Adds network transfer cost but enables fine-grained hardware optimization per phase.

## Serving Frameworks

### vLLM
Production-grade serving system combining PagedAttention, continuous batching, chunked prefill, and speculative decoding.
- **Strengths**: Best throughput for batch inference; easy to integrate
- **Limitations**: Higher latency than TensorRT-LLM for latency-critical workloads

### Text Generation Inference (TGI, Hugging Face)
Rust-based framework optimized for latency + throughput balance. Feature-rich (LoRA, guided generation).

### TensorRT-LLM (NVIDIA)
Compiled inference engine for lowest latency. Requires explicit GPU code optimization per model. Best for production single-request inference at ultra-low latencies (<50ms).

### SGLang
Newer framework focusing on **structured generation** (constrained outputs, agentic workflows). Implements parallelism across tree-of-thought branches and ReAct loops.

## Practical Architecture Patterns

**Throughput optimization** (batch inference): vLLM on A100 cluster, aggressive continuous batching, quantization to INT4.

**Latency-critical** (<50ms p99 latency): TensorRT-LLM on H100, single-instance replica sharding, prefill/decode separation.

**Edge deployment**: GGUF + llama.cpp with CPU quantization, or TensorRT on Jetson.

**Long-context serving**: Flash Attention + PagedAttention + KV cache offload to NVMe for retrieval-augmented workloads.

## Trade-Offs and Limitations

- **Quantization accuracy floor**: INT4 can fail on reasoning tasks requiring high precision
- **Speculative decoding variance**: Effectiveness depends on draft model quality and task predictability
- **Parallelism communication overhead**: Tensor parallelism adds latency; worth it only for very large models
- **KV cache memory ceiling**: Even with paging, 2048+ token sequences remain challenging on modest hardware

## See Also

- **genai-training-infrastructure** — distributed training infrastructure that produces inference-optimized checkpoints
- **architecture-resilience** — designing redundant serving systems
- **cloud-aws-containers** — deployment patterns for inference servers