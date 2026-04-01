# CPU Architecture — Pipeline Stages, Execution Models, Cache, and Speculation

## Overview

Modern CPUs are not simple devices that execute one instruction at a time. They are highly parallel, speculative machines that attempt to predict the future, execute instructions out of order, and operate on multiple pieces of data simultaneously. Understanding the design decisions behind contemporary processor architectures—from pipeline stages to cache hierarchies—is essential for systems programming, performance optimization, and security hardening.

## The Pipeline: Instruction Execution Stages

Early processors executed one instruction completely before fetching the next. Modern processors overlap instruction execution using a **pipeline**—a sequence of stages where different instructions execute in parallel at different stages.

### Classic 5-Stage Pipeline

```
Stage 1: Fetch       (IF) — Read instruction from memory
Stage 2: Decode      (ID) — Parse instruction, read registers
Stage 3: Execute     (EX) — Perform arithmetic/logic operations
Stage 4: Memory      (MEM) — Read/write data from/to memory
Stage 5: Writeback   (WB) — Store result in register
```

In an ideal 5-stage pipeline with no conflicts, you complete one instruction per cycle after the initial 5-cycle startup. On a 3 GHz processor, that's 3 billion instructions per second—but only if execution flows perfectly.

### Pipeline Hazards and Stalls

**Data hazards**: An instruction needs a result that hasn't been written yet. Stall until the result is available.

**Control hazards**: A branch instruction doesn't know which instruction to fetch next until it executes. The pipeline fills with wrong instructions, wasting cycles.

**Structural hazards**: Two instructions compete for the same hardware resource (e.g., both need the ALU).

Modern processors add **forwarding** (pass results directly between stages) to reduce data hazards, but stalls still occur on cache misses and dependent branches.

## Superscalar Execution

A superscalar processor executes multiple instructions per cycle by replicating hardware: multiple ALUs, multiple load/store units, multiple fetch paths.

### Multiple Issue Width

- **2-way**: 2 instructions per cycle (older systems)
- **4-way**: 4 instructions per cycle (common in mid-2000s)
- **6-8-way**: Modern desktop/server CPUs
- **Wide dispatch** reduces instruction latency in favorable code (loops with independent operations)

The catch: dependencies limit how many instructions can actually issue. If every instruction depends on the previous one, even an 8-way superscalar executes one per cycle.

## Out-of-Order Execution

**In-order processors** must stall if an instruction has dependencies. **Out-of-order (OoO) processors** reorder instruction execution to hide latencies.

### Reorder Buffer (ROB)

Instructions are fetched and decoded in program order, but executed as soon as dependencies are satisfied. A reorder buffer tracks execution and ensures results are written back in program order—maintaining correctness.

The ROB typically holds 40-512 instructions. Filling the ROB (without retiring) stalls the front-end. Mispredicted branches flush entries, wasting work.

## Branch Prediction

A branch instruction has two possible next addresses: taken or not taken. The CPU must know which to fetch. Guessing wrong means flushing the entire pipeline.

### Branch Predictor Types

**Static prediction**: Assume backward branches are taken, forward branches not taken. Simple, 60-70% accurate.

**Dynamic prediction**: Learn from history. A **branch history table** tracks recent outcomes.

**Pattern prediction**: Some branches have patterns (loop counters, zigzag conditionals). Predictors use multiple bits of history for 92-95% accuracy.

**Neural predictors** (modern): Machine learning models that consider instruction sequence history.

### Branch Misprediction Cost

On a 10-stage pipeline with misprediction, the CPU flushes ~10 cycles of speculative work. On a 3 GHz processor, that's 3-4 nanoseconds of wasted execution. Modern CPUs take 15-20 cycle penalties on deep pipelines, equivalent to hundreds of instructions lost.

## Speculative Execution

The CPU predicts branch outcomes and executes down both paths (or the predicted path) before the branch resolves. If wrong, squash the results.

### Spectre and Meltdown: Exploitation of Speculation

**Meltdown** (2017): Out-of-order execution can read privileged kernel memory before permission checks catch it. The attacker uses timing side-channels (cache behavior) to extract the data before the CPU exception clears the speculative results.

**Spectre** (2017): A branch predictor can be trained to predict wrong (e.g., always predict a bounds check as true). An attacker tricks the CPU into speculatively executing code out of bounds, accessing secrets, and encoding them into the cache.

**Defense mechanisms**:
- **KPTI** (Kernel Page Table Isolation): Separate kernel/user page tables to prevent Meltdown reads
- **Retpoline**: Replace indirect branches with return-stack-buffer-based jumps to prevent Spectre training
- **Fence instructions** (lfence, mfence): Serialize execution to prevent OoO speculation
- **MDS mitigations** (Microarchitectural Data Sampling): Prevent leaks through intermediate buffers
- **Cost**: 1-10% performance overhead; varies by workload

## Cache Hierarchy

Main memory (DRAM) is slow (~100-200 cycles latency). Caches are small, fast buffers close to the CPU.

### L1 Cache

**Per-core**, **32-64 KB** per core. Split into instruction (L1-I) and data (L1-D) caches. **~4 cycle latency**. Write-through or write-back to L2.

### L2 Cache

**Per-core**, **256 KB - 1 MB** per core. **~12 cycle latency**. Unified (instructions + data). Feeds L1 and backs main memory.

### L3 Cache (Last-Level Cache)

**Shared** among cores, **2-32 MB**. **~40 cycle latency**. Slower than L2, but much larger, reducing misses for large data structures.

### Cache Line

The minimum unit of transfer: **64 bytes** (common). A single byte load brings an entire 64-byte line into L1. Nearby data benefits; distant data wastes capacity.

### Cache Miss Hierarchy

```
L1 hit:    ~4 cycles
L2 hit:    ~12 cycles
L3 hit:    ~40 cycles
RAM hit:   ~100-200 cycles
```

An L3 miss on a 3 GHz processor stalls ~300 CPU cycles—enough time to execute hundreds of independent instructions.

## Cache Coherence: MESI Protocol

Multi-core processors must maintain a consistent view of memory. Each cache line has a state:

**M (Modified)**: Written by this core, not in other caches (exclusive owner).

**E (Exclusive)**: Read by this core, not in other caches.

**S (Shared)**: In multiple caches, all read-only.

**I (Invalid)**: Not in this cache.

### State Transitions

- Core A loads data → **E** (exclusive)
- Core B loads same data → Both transition to **S** (shared)
- Core A writes → Invalidates Core B's line, transitions to **M** (modified)
- Core B accesses invalidated line → Cache miss, refills from Core A's L2 or memory

**Cost**: Cache-to-cache transfers on a bus or ring are faster than main memory but slower than L3 hits. Write conflicts cause ping-pong coherence traffic, killing performance.

## NUMA: Non-Uniform Memory Access

On multi-socket systems (2+ CPUs), memory access times depend on which CPU owns the memory.

**Local access**: ~50-100 cycles to memory attached to your socket.

**Remote access**: ~200-300 cycles to memory attached to another socket.

**Penalty**: 2-3x slower. On a 3 GHz processor, that's a real difference.

### NUMA Challenges

- **Thread migration**: If a thread moves to another socket and accesses its old memory, remote stalls occur.
- **False sharing**: Two cores on different sockets cache-line ping-pong due to shared data (or adjacent data in the same line).
- **Socket imbalance**: Tasks clustered on one socket while the other runs cold wastes capacity.

**Mitigation**: `libnuma` and kernel scheduling (numactl, cpuset) pin threads to sockets and memory-bind data. Essential for latency-sensitive and throughput-critical workloads (databases, high-frequency trading, analytics).

## SIMD: Single Instruction Multiple Data

SIMD instructions process multiple data elements with one opcode: load 4 floats, add them to 4 other floats in parallel, store 4 results.

### x86/x64 Vector Extensions

**SSE (Streaming SIMD Extensions)**: 128-bit registers, 4×float32 or 2×float64 per instruction. Introduced 2000.

**AVX (Advanced Vector Extensions)**: 256-bit registers, 8×float32 or 4×float64. Introduced 2011. Includes FMA (fused multiply-add).

**AVX-512**: 512-bit registers, 16×float32 or 8×float64. Introduced 2016 (Intel Skylake-X); heterogeneous—not all cores support it.

### ARM NEON

128-bit registers (16 bytes). 8×int16 or 4×float32. Standard in ARMv8.

### Speedup

A 4-way SIMD loop processes 4× data per iteration. **Not 4× total speedup** (branch overhead, setup costs, alignment requirements matter), but often 2-4× in practice on tight loops.

**Limitations**:
- Requires data parallelism (independent iterations, no loop-carried dependencies)
- Alignment: data must be aligned to vector width
- Complexity: compiler auto-vectorization is imperfect; manual intrinsics or assembly required for full performance

## Instruction Set Trends

**x86-64**: CISC (complex instruction set), 1000+ instructions. High code density, but expensive to decode.

**ARM (ARMv8)**: RISC (reduced instruction set), ~100 common instructions. Simpler decoding, but larger code.

**RISC-V**: Emerging open standard. Minimal base ISA (39 instructions); extensible.

Modern CPUs translate complex instructions into micro-operations (µops) internally for execution, blurring the CISC/RISC distinction.

## Performance and Implications

**Instruction-level parallelism (ILP)** is the currency of modern CPU design. With 4-way superscalar OoO execution and deep pipelines, you need independent instructions to keep the machine fed.

Code with tight dependencies (sequential loops, pointer chasing) underutilizes the pipeline.

SIMD and branch prediction matter for real workloads—but so do cache misses. A cache miss is worth 200-300 wasted cycles of execution sophistication.

## See Also

- [Virtual Memory](os-memory-virtual.md) — page tables, TLB, demand paging
- [Concurrency Patterns](concurrency-patterns.md) — false sharing, memory ordering
- [Performance Profiling](systems-reasoning.md) — measuring what actually happens