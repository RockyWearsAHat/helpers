# `memory` — Unbounded Speaking & Memory Architecture

A layer built *around* a fixed-capability language model so that neither conversation
length, output length, nor accumulated knowledge ever forces it to forget, truncate, or
degrade. It is not a model — it is the architecture that lets a model behave as though its
ability to **remember** and **speak** is without practical limit, at roughly constant cost
per step.

The one load-bearing invariant is **the live model input is always bounded**. Everything
else grows outward from it. The unboundedness lives in an external store; the controller is
a small finite-state machine that holds no growing hidden state.

## Components

| Component | File | Responsibility |
|-----------|------|----------------|
| controller / orchestrator | `controller.rs` | One action per cycle (`answer/retrieve/compact/write_memory/continue_output/clarify/refuse`); `MemorySystem` wires it all together. |
| working set | `working_set.rs` | Bounded live context; enforces the token budget before every model call; evicts oldest spans for compaction. |
| store | `store.rs` | Immutable raw spans + versioned, never-overwritten memory items. |
| embedding | `embed.rs` | **Training-free** hypervector fingerprints (hashing, not learned weights) + deterministic fact extraction. |
| gpu kernel | `gpu.rs` | Popcount Hamming similarity, backend chosen automatically (rayon-CPU / wgpu-GPU), bit-identical either way. |
| concept index | `concept_index.rs` | Files memory under concepts so retrieval is scoped, not a bulk dump. |
| retriever | `retriever.rs` | Capped, ranked recall fusing semantic similarity, keyword overlap, entity match, recency, importance; provenance on every hit. |
| compactor | `compactor.rs` | Compresses evicted spans into concept-linked summaries with pointers back to raw. |
| recall gate | `recall_gate.rs` | Deterministically verifies every concrete fact survived a compaction. |
| output streamer | `output_streamer.rs` | Arbitrarily long output in planned segments with bounded per-segment input. |
| audit log | `audit.rs` | Append-only, plain-language record of every decision and state change, with provenance. |
| model seam | `model.rs` | The single replaceable `LanguageModel` trait; `MockModel` is the deterministic test/demo stand-in. |

## Run

```bash
cd native

# End-to-end demo: prints, per cycle, the model-facing input size, retrieved items with
# provenance, long-output segment numbers, and the plain-language audit log.
cargo run --example memory_demo

# Unit tests (per-component) + the eight architectural invariants (end-to-end).
cargo test --lib memory::
cargo test --test memory_invariants

# Optional GPU similarity backend (off by default; CPU path is identical and usually
# faster on Apple-Silicon unified memory, so the kernel auto-selects).
cargo build --lib --features gpu
```

## The two unbounded axes

* **Knowledge & memory** — history beyond the live window is compacted into concept-linked,
  versioned summaries that point back to **immutable** raw spans, and pulled back only when
  relevant, ranked, and capped. No relevant fact is lost: if a summary is insufficient, the
  untouched raw span is rehydrated.
* **Speaking & expression** — long output is produced in planned segments; the model sees a
  bounded running summary + the section directive + style constraints, never its entire
  prior output, so a very long answer stays on-thread at flat per-segment cost.

## Why "without any extra training"

Retrieval similarity comes from **hyperdimensional computing**, not learned embeddings:
every token hashes to a fixed random 8192-bit vector, and a span's fingerprint is the
majority-vote bundle of its tokens. Shared vocabulary ⇒ small Hamming distance. No corpus,
no gradient, no model weights — just hashing and popcount, which is exactly why the
similarity search maps onto the GPU/CPU popcount kernel in `gpu.rs`.

## Calibration (what "unbounded" does *not* mean)

Finite hardware means the target is asymptotic — unbounded for any realistic session at
flat per-step cost, not literal infinity. Compaction is lossy on the margin and retrieval
has false positives; the design drives confusion *down* and makes it *auditable* rather than
claiming it is gone. Raw expressive range and language coverage come from the underlying
model; this layer's job is to never subtract from them.
