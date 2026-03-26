# Large Language Model Architecture — Transformers, Tokenization & Scaling

## The Transformer Foundation

The transformer architecture, introduced in "Attention Is All You Need" (Vaswani et al., 2017), replaces recurrence with purely attention-based computation. Unlike RNNs that process sequences sequentially, transformers process all tokens in parallel, enabling massive parallelism during training.

The core insight is **self-attention**: each token learns to dynamically weight its relationships with all other tokens. For each position, the model computes:

- **Query (Q)**: What am I looking for?
- **Key (K)**: What information do I represent?
- **Value (V)**: What should I contribute?

The attention score for each pair is: `softmax(QK^T / √d_k) × V`. Multi-head attention repeats this process with different learned linear projections, allowing the model to attend to different semantic relationships simultaneously.

| Component | Role | Notes |
|-----------|------|-------|
| Attention heads (typically 8-96) | Parallel attention subspaces | More heads ≠ always better; diminishing returns |
| Feed-forward network (FFN) | Non-linear expansion | Usually 4× hidden dimension width |
| Layer normalization | Stable gradient flow | Applied before or after sublayers (pre-norm vs post-norm) |
| Residual connections | Enable deep stacking | Skip connections around each sublayer |

Decoder-only models (like GPT) use causal masking: tokens can only attend to earlier tokens, enforcing causality during autoregressive generation.

## Positional Encoding

Since attention has no inherent notion of sequence position, positional information must be injected. Early transformers used **sinusoidal positional encoding**: absolute position $p$ and dimension $d$ encode position as $\text{PE}(p, 2i) = \sin(p / 10000^{2i/d})$ and $\text{PE}(p, 2i+1) = \cos(p / 10000^{2i/d})$.

Modern alternatives:
- **RoPE** (Rotary Position Embeddings): Rotates query/key vectors in complex space; extrapolates better to longer sequences
- **ALiBi** (Attention with Linear Biases): Adds position-dependent bias directly to attention scores; simple, works surprisingly well
- **Absolute embeddings**: Learnable position vectors; limited to training seq length

Longer context windows require different positional schemes; extrapolation to unseen sequence lengths is an active research area.

## Tokenization Strategies

LLMs don't process raw text—they first split it into tokens and map each to a vocabulary index. Three dominant subword tokenization algorithms:

**Byte Pair Encoding (BPE)**
- Starts with character-level vocabulary
- Greedily merges the most frequent adjacent pairs
- Deterministic; reversible
- Used by GPT models

**WordPiece**
- Similar to BPE but merges based on likelihood gain (vocabulary coverage), not raw frequency
- Only saves final vocabulary, not merge rules
- Used by BERT

**SentencePiece**
- Treats entire raw text as input; no pre-tokenization assumption
- Language-agnostic (handles CJK better)
- Can output characters, subwords, or mixed
- Used by T5, LLaMA, Gemini

Trade-offs: BPE/WordPiece assume whitespace-delimited words (problems for languages like Chinese); SentencePiece is universal but slower. A 7B parameter model typically uses 30K-100K token vocabulary. Larger vocabularies reduce tokens-per-sequence but increase embedding memory.

## Scaling Laws and Compute-Optimal Training

**Chinchilla Scaling Law** (Hoffmann et al., 2022) fundamentally reshaped model training. Prior work (Kaplan et al., OpenAI) suggested compute scaling favored larger models over more data. Chinchilla empirically showed allocating compute equally between model size and training tokens yields optimal loss:

- For a fixed compute budget $C$, set model params $N ≈ C / 6D$ and training tokens $D ≈ 6C / N^2$
- Optimal ratio: **model size = training tokens** (e.g., 70B model trained on 70B tokens)

This contradicted prior large-model practice (training 280B-parameter models on 300B tokens). Modern frontier models (Gemini, Claude) roughly follow this principle, though practitioners debate the exact coefficients.

Scaling laws predict loss as a power law: $L(N, D) = E + \frac{A}{N^α} + \frac{B}{D^β}$ where small exponents allow smooth prediction. However, scaling laws don't capture:
- Phase transitions (sudden emergent abilities at certain scales)
- Quality of training data (more important than raw token count)
- Architectural innovations that change efficiency

## Key Caching and Inference Optimization

During autoregressive decoding, computing attention for token $t$ requires keys and values from all previous tokens. Naive recomputation is wasteful; instead, **KV cache** stores precomputed keys/values from earlier positions. For a 13B model, this reduces decoder inference by ~10x but introduces memory I/O bottlenecks—often the limiting factor for deployment at scale.

**Speculative decoding** reduces latency by sampling multiple candidate tokens in parallel, then verifying with the main model. Works well when a small draft model predicts the main model's distribution with high probability.

## Quantization: Trading Precision for Speed

Large models fit only on high-end GPUs. **Quantization** reduces parameter precision (FP32 → INT8 → INT4), cutting memory by 4-8x. Approaches:

- **GPTQ**: Post-training quantization to 4-bits per parameter; requires calibration on sample data; preserves accuracy surprisingly well
- **AWQ**: Activation-aware weight quantization; channels with high activation variance kept in higher precision
- **GGUF**: Portable format combining quantization with metadata; enables inference on consumer hardware

Trade-off exists between memory savings, compute speed, and accuracy loss. 4-bit GPTQ models typically retain 95%+ capability on downstream tasks.

## Mixture of Experts (MoE)

Sparse models (e.g., Switch Transformer, Mixtral) replace dense FFN layers with conditional routing: a **gating network** selects a subset of expert networks for each token. Only active experts compute, reducing FLOPs but potentially introducing load imbalance (some experts rarely selected) and training instability.

See also: **Extending context windows**, **Inference engines**, **Machine learning operations**