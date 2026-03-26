# Memory Hierarchy

The memory hierarchy is a foundational concept in computer architecture, organizing storage from fastest/smallest to slowest/largest to optimize access patterns for typical workloads.

## Hierarchy Levels (Speed/Access Time Ordering)

**Registers** — the fastest tier, located on the CPU die itself. Latency measured in picoseconds. Capacity measured in kilobytes (x86-64 has ~16-32 registers × 64 bits).

**L1 Cache** — per-core cache, typically 32-64 KB. Latency ~4 cycle. Split into instruction (I-cache) and data (D-cache) in most architectures. Highest bandwidth to core.

**L2 Cache** — per-core cache, typically 256 KB - 1 MB. Latency ~12 cycles. Still private to each core; feeds L1.

**L3 Cache** — shared among cores on a socket, typically 8-20 MB. Latency ~40-75 cycles. Acts as a unified level before main memory.

**Main Memory (DRAM)** — system RAM, typically 8-256 GB. Latency ~100-300 cycle (50-150 ns). Volatile; loses contents when powered off. Requires periodic refresh due to capacitive charge leakage.

**Storage** — SSD/HDD, typically terabytes. Latency seconds (HDD) to milliseconds (SSD). Non-volatile.

## Optimizations

**Cache Line** — smallest unit of data moved between hierarchy levels. Standard is 64 bytes on modern x86/ARM. Spatial locality principles motivate this size: programs tend to access nearby memory addresses, so fetching adjacent data is efficient.

**Inclusivity** — an important distinction with three forms:
- **Inclusive**: L1 ⊆ L2 ⊆ L3; contents of smaller cache are guaranteed present in larger ones (Intel typical)
- **Exclusive**: L1 ⊄ L2; smaller cache and larger cache do not duplicate data (AMD Opteron, some PowerPC)
- **NINE** (Non-Inclusive Non-Exclusive): Neither guarantee holds; optimal use of capacity but complex coherence logic

**Associativity** — describes mapping flexibility from main memory into cache. Fully associative allows any block to go anywhere; n-way associative restricts to n sets of locations. Direct-mapped (1-way) is fastest but prone to severe conflicts. Most L1 caches are 8-way or 4-way associative to balance conflict misses and lookup latency.

**Write Policies** — govern data propagation:
- **Write-back**: CPU writes to L1; dirty line only written to lower level when evicted. Reduces bus traffic; adds complexity (must track dirty bits).
- **Write-through**: CPU writes propagate immediately to lower levels. Higher bus traffic but simpler coherence model.

## Specialized Components

**Translation Lookaside Buffer (TLB)** — a cache of virtual-to-physical address translations. Multiple levels typically present (L1 TLB ~per-core, L2 TLB shared). Reduces translation latency from walking multi-level page tables (which requires 4-5 main memory accesses on x86).

**Page Tables** — hierarchical data structures (typically 4-level on x86-64: PML4 → PDPT → PD → PT) mapping virtual addresses to physical. Walking them incurs multiple memory accesses, motivating TLB caching. Common page size 4 KB; large pages (2 MB, 1 GB) reduce TLB pressure.

**NUMA (Non-Uniform Memory Architecture)** — multiple memory controllers across sockets, introducing distance-dependent latency. Local access ~100 cycles; remote access ~300+ cycles. Modern OS schedulers attempt to colocate threads and memory.

## Performance Considerations

**Memory Bandwidth** — the rate at which data can be transferred. Modern DDR4 ~60 GB/s; DDR5/HBM higher. Peak CPU throughput can exceed DRAM bandwidth, causing stalls. Cache hierarchy mitigates this for locality-friendly access patterns.

**Cache Coherence** — in multicore systems, writes in one core's cache must be visible to others. MESI protocol (Modified/Exclusive/Shared/Invalid) and its variants (MOESI adds Owned state) ensure consistency at cost of coherence traffic. Some designs employ directory-based coherence rather than snooping to reduce bus traffic.

**Miss Penalties** — the latency stall incurred when data is not found at a given level. L1 miss to L2: ~12 cycles lost. L3 miss to DRAM: ~100+ cycles lost. Prefetching (hardware or software) can hide some of this latency.

## Design Trade-offs

Larger caches reduce miss rate but increase latency of hits and area/power. Smaller caches are faster and cheaper but incur more misses. Inclusive caches simplify coherence but waste capacity; exclusive caches maximize capacity but complicate coherence logic. The hierarchy reflects this fundamental tension between speed and capacity.

See also: cache replacement policies, memory consistency models, performance profiling.