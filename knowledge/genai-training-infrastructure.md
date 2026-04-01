# LLM Training Infrastructure — Distributed Strategies & Optimization

## The Scale Challenge

Training a 70B parameter model on 2 trillion tokens requires ~800 petaflops of compute (170K H100-hours). No single GPU suffices; training is inherently distributed. Infrastructure must manage three constraints simultaneously: **model parallelism** (model too large for one GPU), **data parallelism** (training on massive datasets), and **synchronization overhead** (communication between devices dominates computation at scale).

## Distributed Training Strategies

### Data Parallelism (DDP)

Each GPU holds a full model copy. Mini-batches are distributed across GPUs; gradients are averaged (all-reduce) before weight updates.

```
GPU-0: batch[0:B/n]   --> compute gradients --> all-reduce --> weight update
GPU-1: batch[B/n:2B/n]--> compute gradients --> all-reduce --> weight update
```

**Strengths**: Simple; works with existing code; scales linearly to ~8 GPUs
**Limitations**: Each GPU needs enough memory for the full model + optimizer state + activations. For 70B params in FP16: ~140GB per GPU. Not sufficient for modern models.

### Tensor Parallelism

Split weight matrices across GPUs horizontally. A single forward pass requires GPU-to-GPU communication **per layer**.

```
W = [W0 | W1 | W2 | W3]  (split across 4 GPUs)
y = xW --> requires communication after each matmul
```

**Memory reduction**: Near-linear (8x GPUs ≈ 8x reduction per GPU)
**Communication cost**: ~20-40% latency overhead; synchronized communication bubbles must be hidden via pipeline.

**When worth it**: Model exceeds single GPU memory (~70B+ in FP16).

### Pipeline Parallelism

Split model by layers across GPUs. Layer 0-15 on GPU-0, layers 16-31 on GPU-1, etc.

```
GPU-0: layers 0-15  [flush] --> GPU-1: layers 16-31 [flush] --> ...
```

**Advantage**: Reduced per-GPU memory; hiding communication behind computation
**Challenge**: Pipeline bubbles with small batch size. If batch size < num_stages, some GPUs go idle waiting for prior GPU to finish.

**Optimizations**:
- **Interleaved pipeline (GPT-3 style)**: Each GPU processes multiple sub-batches; increases utilization
- **1F1B (1 Forward 1 Backward)**: Schedule forward and backward passes to keep pipeline full

### Fully Sharded Data Parallel (FSDP, PyTorch)

Combine data + tensor parallelism: shard model parameters, optimizer states, and gradients across GPU cluster.

```
Parameter sharding: split params across data-parallel group
Gradient accumulation: only compute gradients for local shard
All-reduce: gather gradients to compute full update
```

**Strengths**: Extremely flexible; works with complex models (any forward/backward code)
**Limitations**: Frequent all-reduce communication; lower arithmetic intensity than tensor parallelism alone

**Variants**:
- **FSDP zero-2**: Shard gradients and optimizer states; parameters replicated
- **FSDP zero-3**: Shard parameters, gradients, optimizer states (lowest memory per GPU; highest communication)

### DeepSpeed ZeRO

Microsoft's optimizer state sharding framework (similar goals to FSDP but earlier and more aggressive in optimization).

- **ZeRO-1**: Shard optimizer states (4x memory reduction over DDP)
- **ZeRO-2**: Shard gradients + optimizer states (8x reduction)
- **ZeRO-3**: Shard parameters + gradients + optimizer states (extreme sharding; requires all-gather on each forward pass)

**Integration**: Works with existing PyTorch code; often outperforms FSDP via careful memory management.

**Tradeoff**: ZeRO-3 achieves 8-16x memory reduction but at 30-50% communication overhead. Break-even at ~16 GPUs.

## Mixed Precision Training

Using lower-precision formats saves memory and computation with minimal accuracy loss. Gradients, activations, and optimizer states can use lower precision; weights typically stay in higher precision for stability.

### FP16 (Half-Precision Float)
1 sign + 5 exponent + 10 mantissa. Range: ±65K; smallest nonzero: ~6e-5.

**Advantage**: 2x memory reduction; native support on Volta+ GPUs
**Risk**: Gradient underflow; weight updates can vanish. Requires **loss scaling**: multiply loss by large constant (e.g., 2^16), scale down gradients after backward pass.

### BF16 (Brain Float)
1 sign + 8 exponent + 7 mantissa. Range: ±3.4e38; smallest nonzero: ~1e-38.

**Advantage**: Wider range than FP16; less prone to gradient underflow; no loss scaling needed
**Limitation**: Lower precision mantissa (7 vs 10 bits); slightly noisier gradients
**Adoption**: Preferred on newer hardware (A100, H100) where native support available; becoming industry standard.

### FP8 (Micro-Precision)
Emerging standard for both weights and activations. Reduces memory by 75% vs FP32.

**Variants**:
- **E4M3** (4 exponent, 3 mantissa): Used for weights and gradients; higher range
- **E5M2** (5 exponent, 2 mantissa): Used for activations and KV cache; lower precision but wider range

**Challenge**: Training stability; requires careful scaling and outlier treatment. Not yet mainstream for full model training, but used for quantization-aware training (QAT).

## Gradient Checkpointing

Strategy to reduce peak memory consumption by trading compute for memory. Instead of storing activations for all layers during forward pass, checkpoint strategy computes them again during backward pass.

```
Forward: compute layer 1, discard activation
        compute layer 2, store activation
        compute layer 3, discard activation
Backward: recompute layer 1, compute gradients
         use stored layer 2
         recompute layer 3, compute gradients
```

**Impact**: Reduces activation memory by ~50%; increases compute by ~20-30% (recomputation during backward).

**When worth it**: Model fits uneasily on GPU (90%+ memory utilization). Not worth it if GPU has spare capacity.

## Training Data Pipeline and Storage

### Data Loading Architecture
Modern training requires **streaming data** (cannot fit entire dataset on disk attached to cluster). Common patterns:

- **Cloud object store (S3, GCS, etc.)**: Data lives in cloud storage; training machines download on-demand
- **Local NVMe raids**: High-throughput tier; ~5 GB/s read for modern NVMe arrays
- **Network NFS**: Shared storage; simplest but slowest (~100-500 MB/s)

**Throughput requirement**: 70B model on H100 requires ~30 GB/s of data throughput to keep GPU fed. Requires optimized I/O (parallel readers, prefetching).

### Data Formatting
- **Consolidated format (JSONL, Parquet)**: Single large files; enables efficient I/O and resumable checkpointing
- **Preprocessing**: Tokenization offline (not during training); store as memmap arrays for O(1) access
- **Interleaving**: Multiple datasets mixed during training; requires careful weight scheduling to prevent forgetting

### Deduplication and Quality Filtering
Pre-training datasets contain near-duplicate examples. Deduplication via:
- **Exact dedup**: Hash-based removal (simple, effective for exact duplicates)
- **Approximate dedup**: MinHash, Bloom filters (catch near-duplicates; better but slower)
- **Language model scoring**: Score examples; discard low-quality (below quantile)

Estimated impact: 10-20% final performance boost by removing duplicates and low-quality.

## Training Cluster Architecture

### Typical Setup
- **Compute**: 256-1024 GPUs (A100/H100) arranged in 8-way tensor parallel groups (32 groups, each group = 1 node with 8 GPUs)
- **Interconnect**: NVLink (600 GB/s GPU-GPU) within node; RDMA (200 GB/s) between nodes
- **Storage**: 100+ TB NVMe for checkpoint staging; petabyte-scale cloud storage for data
- **Monitoring**: Continuous instrumentation: GPU utilization, MFU (Model FLOPs Utilization), I/O bandwidth, gradient scaling metrics

### Training Efficiency Metrics
- **Model FLOPs Utilization (MFU)**: Fraction of GPU peak theoretical compute achieved. Target: 40-60% for multi-GPU training, 50-70% single-GPU where overhead is lower
- **Allocation efficiency**: Fraction of GPU memory actively used. Target: 85-95% for optimal throughput
- **Gradient accumulation steps**: Increase effective batch size without increasing GPU memory; tradeoff: more iterations, noisier gradient estimates

## Cost Estimation and Optimization

For 70B model on 2T tokens with H100 ($1.98/hour):

```
Compute hours = (70B * 2T * 6) / (1.98e17 FLOPS * 3600)
              ≈ 310K GPU-hours
Cost = 310K * $1.98 ≈ $614K (per full training run)
```

Constant factors:
- 6x flops per token per parameter (2x forward, 2x backward, 2x optimizer)
- H100 peak: ~1.98e17 FLOPS (FP8 mode; lower for FP32)

**Cost reduction strategies**:
- **Mixed precision (FP8)**: 4x speedup; reduces cost proportionally
- **Gradient accumulation**: Amortizes communication overhead; improves MFU
- **Efficient data loading**: Avoid I/O bottlenecks on critical path
- **Flash Attention**: 2-3x faster attention computation
- **Sparsity and pruning**: Speculative method; gains unclear at scale

## Checkpointing and Resumption

**Frequency**: Checkpoint every 1-2 hours. Each checkpoint ≈ 3x model size (weights + optimizer states + RNG state). Example: 70B model ≈ 400-600 GB checkpoint.

**Resumption**: Load weights, optimizer state, RNG seed; resume from exact loss step. Critical for long training runs (enable recovery from preemption).

**Asynchronous checkpointing**: Background checkpoint-writing doesn't block training; requires careful RNG handling to avoid divergence.

## See Also

- **genai-inference-optimization** — deployment strategies for trained models
- **system-design-distributed** — distributed systems fundamentals
- **cloud-aws-compute** — cloud infrastructure for ML workloads