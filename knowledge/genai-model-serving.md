# Model Serving for LLMs — vLLM, TensorRT, and Production Inference

## The Inference Challenge

Training optimizes for accuracy; inference optimizes for **throughput** and **latency** under real-time constraints. LLM inference is memory-bound during prefill and compute-bound during decode, requiring different optimization strategies.

**Key constraint**: KV cache (key-value embeddings from attention layers) grows with sequence length and batch size. A 70B model with 80 layers, seq_len=4096, batch_size=128 needs ~576 GB GPU memory—far exceeding typical VRAM. Modern serving systems solve this via memory-efficient attention, continuous batching, and intelligent scheduling.

## Core Serving Systems

### vLLM (Berkeley, open-source)

**Key innovation**: **PagedAttention**—treats KV cache like virtual memory, enabling dynamic batching without fragmentation.

**How PagedAttention works**:
- Allocate KV cache in fixed-size "pages" (e.g., 16 tokens/page)
- Requests reference logical pages; physical pages allocated dynamically
- When prefill finishes, pages become available for other requests
- Drastically reduces fragmentation; enables 10-20x higher throughput

**Architecture**:
- **Parallel leader scheduler** (CPU): Groups requests into batches by token position
- **GPU kernels**: Custom CUDA for PagedAttention, KV cache management
- **Tokenizer offloading**: CPU handles tokenization while GPU computes

**Strengths**:
- Open-source; actively developed
- Highest throughput for dense models
- Supports LoRA adapters; multi-model serving
- Excellent for inference-only deployments

**Weaknesses**:
- Requires specific hardware tuning; CUDA knowledge
- Limited support for inference-time optimization (quantization integration is recent)
- Variable latency due to dynamic batching

**Use**: Dense model serving at scale (thousands of requests/sec).

### TensorRT-LLM (NVIDIA)

NVIDIA's proprietary optimization suite for inference on NVIDIA hardware.

**Stack**:
- C++ runtime for scheduling, memory management
- Custom CUDA kernels (fused ops, attention, quantization)
- Model compiler: PyTorch → TensorRT IR → optimized exe
- Multi-GPU orchestration

**Key optimizations**:
- **Kernel fusion**: Combine adjacent ops into single kernel (e.g., LayerNorm + Attention kernel)
- **Flash Attention**: Memory-efficient exact attention
- **Quantization-aware inference**: INT4, INT8 without accuracy loss
- **Tensor parallelism**: Split model across GPUs; synchronize at communication boundaries

**Strengths**:
- Best-in-class latency on NVIDIA hardware
- Mature, battle-tested (used by enterprise customers)
- Full integration with NVIDIA stack (Triton, Nemo)
- Superior quantization support

**Weaknesses**:
- Requires recompilation for new models; longer development cycle than vLLM
- NVIDIA hardware only; no CPU or AMD support
- Proprietary; closed-source optimization rules
- Requires CUDA expertise

**Use**: Mission-critical deployments; latency-sensitive applications.

### Ollama (open-source)

Simplified local LLM serving; focus on ease of use over performance.

**Design philosophy**: One command ("ollama run llama2") → running inference server.

**Features**:
- Model library with pre-built binaries
- Automatic quantization (Q4_0, Q4_1, Q5_K, Q6_K GGUF formats)
- CPU inference (Apple Metal, Linux, Windows WSL2)
- Simple REST API

**Strengths**:
- Trivial setup; no CUDA/system config
- Good for local development and demo
- Excellent model download/distribution
- CPU inference viable for small models

**Weaknesses**:
- Can't compete with vLLM/TensorRT on throughput
- Memory inefficient vs. production systems
- Quantization quality varies; Q4 models lose accuracy
- Single-GPU only (no tensor parallelism)

**Use**: Local development, fine-tuning evaluation, demos, edge deployment.

### Text Generation Inference (TGI, from Hugging Face)

Production inference server for open models; intermediate between Ollama's simplicity and vLLM's complexity.

**Key features**:
- Continuous batching with Transformers optimization
- Speculative decoding (see below)
- Watermarking (embeddings + optional output watermarking)
- ORCA-style batching (pad sequences to same length, process together)
- RAI mitigation (can block certain generations)

**Supported backends**: GPU (NVIDIA), CPU, can fallback.

**Strengths**:
- Easy deployment (Docker container)
- Good middle ground: easier than vLLM, more performant than Ollama
- Speculative decoding out-of-the-box
- Safety features (watermarking, blocked tokens)

**Weaknesses**:
- Less throughput than vLLM under heavy load
- Watermarking adds latency
- Fewer advanced options than TensorRT

**Use**: Managed hosting scenarios, fine-tuned models, safety-critical deployments.

## Model Formats

LLMs are stored in various formats, each with trade-offs:

### Full Precision (FP32, FP16)

**Size**: 13B params ≈ 26 GB (FP16), 52 GB (FP32)

**Pros**: Maximum accuracy; no quantization artifacts.

**Cons**: Huge memory footprint; slow inference; expensive storage; impractical for anything past 13B on consumer hardware.

**When used**: Academic evaluation; research; where accuracy is paramount and compute unlimited.

### GGUF (Georgio Gasparin Unified Format)

Quantized weights in a single file; designed for CPU inference and easy distribution.

**Format**:
- 32-bit tensor metadata
- Quantized weights (Q4_0, Q4_1, Q5_K, Q6_K, etc.)
- Token vocab
- Produced via ggml quantization tools

**Quantization levels**:
- Q4_0, Q4_1: 4 bits/weight; 70-86% size reduction; noticeable quality loss
- Q5_K, Q6_K: 5-6 bits; 50-60% reduction; minimal quality loss
- IQ3_M, IQ4_XS: 3-4 bits; experimental; extreme compression

**Pros**: Portable (any system with llama.cpp); efficient CPU inference; easy to distribute.

**Cons**: Slower GPU inference than native formats; quantization quality varies; harder to merge/continue training.

**Use**: Local/edge deployment, models for distribution (e.g., Ollama models), CPU servers.

### GPTQ (Generative Pretrained Transformer Quantization)

Post-training quantization method; weights quantized to 2-8 bits using calibration data.

**Process**:
1. Collect calibration data (real prompts)
2. Compute Hessian (sensitivity of each weight)
3. Quantize insensitive weights more aggressively
4. Produce GPTQ quantized weights (`.safetensors` or `.bin`)

**Advantages**:
- 3 bits/weight possible with minimal accuracy loss
- Can run 70B models on 24GB GPU (INT3)
- Better compression than uniform quantization

**Disadvantages**:
- Requires calibration data and compute to quantize
- Slower than GGUF due to overhead
- Limited ecosystem support (gptq-for-llama, VLLM, ExLLaMA, etc.)

**Use**: High compression needed (3-4 bits); running large models on consumer GPUs.

### AWQ (Activation-Aware Quantization Weights)

Similar to GPTQ; focuses on weights whose inputs have high activation magnitudes (Chebyshev centering).

**Improvements over GPTQ**:
- Faster quantization process
- Slightly better accuracy at same bit-width
- Active development

**Use**: Alternative to GPTQ; gradually replacing it.

## Advanced Optimization Techniques

### KV Cache Management

The KV cache from all attention layers grows:
- For a 70B model: 70 layers × 2 (K, V) = 140 tensors
- Each tensor: batch_size × num_heads × seq_len × head_dim
- Typical: 128 batch × 80 heads × 4096 seq × 128 head_dim × 2 bytes = 536 GB at seq_len=4096

**Optimizations**:
- **Pruning**: Remove unimportant heads (head importance scoring)
- **Quantization**: Store KV in INT8/INT4 instead of FP16 (4-8x compression)
- **Distillation**: Smaller KV caches from distilled models
- **Sparse attention**: Skip non-critical attention head computations

### Speculative Decoding (Assisted Generation)

Use a fast, smaller model to generate candidate tokens; larger model verifies and accepts/rejects.

**Process**:
1. Small model generates k candidate tokens (fast, low-quality)
2. Large model scores all k tokens in parallel (batch K)
3. Accept prefix where predictions match; revert if diverge
4. Continue from agreement point

**Throughput gain**: 2-3x theoretically; practically 1.5-2.2x (due to mismatch overhead).

**Trade-off**: Adds latency if candidates mostly rejected.

**Implementation**: Hugging Face (transformers.generation), vLLM, TGI, llama.cpp.

### Tensor Parallelism and Pipeline Parallelism

**Tensor Parallelism**: Split model layers across GPUs horizontally.
- Each GPU computes subset of attention heads or FFN neurons
- All-reduce synchronization after each layer
- Communication overhead scales with model size

**Pipeline Parallelism**: Split layers across GPUs sequentially.
- GPU 1 computes layers 1-10, GPU 2 does 11-20, etc.
- Requires bubble scheduling (GPUs idle between stages)
- Lower communication overhead; more underutilization

**Typically**: Tensor parallelism for dense multi-GPU systems; pipeline for distributed data centers.

### Flash Attention and Memory-Efficient Attention

**Problem**: Standard attention computes O(seq_len²) attention matrix; requires seq_len² memory even temporarily.

**Flash Attention v1/v2 (Dao et al.)**:
- Tiles attention computation; fuses into memory-efficient kernel  
- Reduces memory from O(seq_len²) to O(seq_len)
- Near-exact numerically; 2-3x wall-clock speedup

**Alternatives**: Sparse attention (local windows), low-rank approximations (Linformer), linear attention.

**Status**: Built into modern frameworks; vLLM, TensorRT, transformers all use it.

## Latency vs. Throughput Trade-Offs

### Prefill Phase (Compute-Bound)

Processing initial sequence is compute-heavy. Optimization: batch multiple requests' prefills together.

**Latency**: Few ms for small contexts.
**Throughput**: Can achieve high throughput (many requests in parallel).

### Decode Phase (Memory-Bound)

Generating one token at a time is memory-bound (need to fetch entire model weights for each new token). Latency dominates.

**Latency**: 50-500 ms per token depending on model size and hardware.
**Throughput**: Limited by latency; best case ~20 tokens/sec on A100 for 70B model.

**Optimization**: Batching decode requests (PagedAttention, continuous batching) to hide latency.

### Time-to-First-Token vs. Throughput

- **TTFT (time-to-first-token)**: How long until first token appears. Critical for interactive applications.
- **Throughput**: Tokens/second during streaming. Critical for batch workloads.

Batching improves throughput but increases TTFT (requests queue). Trade-off: SLA selection.

**Example SLA**: "Serve 50 requests/second with <100ms TTFT" → might need pre-batch of 5-10 requests.

## Production Checklist

- [ ] Model format chosen (GGUF for local, safetensors for GPU)
- [ ] Hardware specified (GPU vram, CPU cores, network bandwidth)
- [ ] Serving system selected (Ollama/local, vLLM/high throughput, TensorRT/latency, TGI/managed)
- [ ] Quantization strategy decided (none, GPTQ, AWQ, INT8)
- [ ] Monitoring: TTFT, throughput, error rates, GPU/memory utilization
- [ ] Batching policy: Static batch size, continuous batching, adaptive
- [ ] Fallback/circuit breaker: If primary server fails, route to alternative
- [ ] Cost model: Tokens/request × request rate = monthly cost; optimize jointly

## See Also

- [LLM Inference Optimization — Serving at Scale](genai-inference-optimization.md)
- [LLM Cost Optimization — Token Economics, Caching, and Strategic Selection](genai-lm-cost-optimization.md)
- [Machine Learning Operations — Model Lifecycle, Serving & Production Concerns](ml-operations.md)