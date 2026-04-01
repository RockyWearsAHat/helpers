# Systems: Memory Allocators — Strategies, Trade-offs, and Implementations

Dynamic memory allocation is central to systems programming. Allocators must balance speed, fragmentation, concurrency, and memory overhead. Different allocators make different trade-offs; choosing well depends on workload characteristics.

## The Allocation Problem

**Fragmentation** is the core challenge. When allocations and deallocations are interleaved, memory becomes fragmented into small free blocks that cannot satisfy large requests, even if total free memory is large. Two orthogonal approaches mitigate fragmentation:

1. **Coalescing** — merging adjacent free blocks to create larger blocks. This requires metadata tracking (usually a boundary tag in the header/footer of each allocation), adding space overhead. Coalescing is deferred (lazy) in some allocators to save time.
2. **Pooling** — partitioning memory into pools of fixed sizes, reducing fragmentation by ensuring allocations from the same size class use the same pool.

**Threading** introduces contention: if all threads must lock a shared heap, allocation becomes a bottleneck. Thread-local caches and per-thread arenas reduce contention by allowing each thread to allocate from its own private pool, falling back to a shared pool only when local memory runs dry.

## Classical Approaches: dlmalloc and ptmalloc

Doug Lea's **dlmalloc** (starting 1987) pioneered the boundary-tag allocator. Memory is divided into chunks: allocated chunks have an 8- or 16-byte header storing size and flags; free chunks store pointers to other free chunks, reducing minimum free size. Free chunks are grouped into "bins" by size:

- **Small bins** (< 256 bytes): use a simple power-of-two best-fit search.
- **Tree bins** (≥ 256 bytes, modern versions): use a binary trie for faster lookup.
- **Large bins** (> 128 KB threshold): allocated via `mmap`, which reserves entire pages and avoids fragmentation of smaller allocations.

**ptmalloc** (Wolfram Gloger, later adopted by glibc) extended dlmalloc with per-thread arenas to reduce lock contention. Each thread gets its own arena with its own set of bins; threads can migrate between arenas under load. ptmalloc remains the standard allocator in glibc.

## Arena and Per-CPU Strategies: jemalloc

**jemalloc** (Jason Evans, adopted by FreeBSD and NetBSD) replaces per-thread arenas with per-CPU arenas for even finer granularity. The key innovation is **scalable allocation**: with N CPUs, jemalloc maintains N independent arenas (plus a shared arena for global operations). Each thread allocates from its CPU's arena with minimal synchronization. Experiments show near-linear scalability—throughput increases proportionally with thread count—whereas older allocators showed inverse scaling (throughput drops as threads increase).

jemalloc further introduces a **tier of caches**:

1. **Thread-local cache** (fast, no lock): thread allocates from its own small pool of pre-allocated bumps.
2. **CPU arena** (minimal lock): thread allocates from its CPU's larger pool.
3. **Global shared arena** (lock-contended): last resort for large allocations or cache evictions.

This hierarchy reduces latency for common allocations.

## Run Size Classes and Size Segregation

Allocators subdivide the size space into "size classes" (e.g., 8, 16, 24, 32, 40, …, 4096 bytes). Allocation requests are rounded up to the nearest size class. Segregating sizes reduces fragmentation because allocations of similar sizes keep each other company; external fragmentation within a size class is bounded.

The trade-off: **internal fragmentation** (wasted space when a 25-byte request is rounded to 32) increases, but external fragmentation decreases. The net effect is usually positive.

## mmap vs. sbrk

The operating system provides two mechanisms to grow the heap:

- **`sbrk`** grows a linear heap, returning contiguous memory. It is fast but inflexible; the heap cannot shrink unless the last allocation is freed.
- **`mmap`** allocates arbitrary pages anywhere in the address space. It is flexible (pages can be unmapped independently) but slower (system calls are expensive) and triggers page table overhead.

Allocators hybrid this: small allocations from a heap managed via `sbrk`, large allocations directly from `mmap`. The threshold is tunable (glibc's MALLOC_MMAP_THRESHOLD_, typically 128 KB).

## Specialized Allocators

**tcmalloc** (Google) uses thread-local size-class caches, similar to jemalloc but with different synchronization. Available for non-Google code via Abseil.

**mimalloc** (Microsoft Research) emphasizes compactness and speed, using a page-based allocator (allocations from pages dedicated to a size class). It is designed for modern many-core systems and avoids some of jemalloc's complexity.

**Hoard** (Emery Berger) uses per-processor heaps and superblocks (64 KB chunks), balancing fragmentation and concurrency. It predates jemalloc but is less commonly deployed.

**OpenBSD malloc** uses `mmap` exclusively, allocating even small objects via system calls. Every free unmaps the memory, immediately catching use-after-free bugs via segfault. Not suitable for performance-critical code but excellent for security-sensitive systems.

## Slab and Bump Allocation

**Slab allocation** (origin: kernel memory allocation) preallocates large blocks ("slabs") of a fixed size and subdivides each into fixed-size objects. All objects in a slab are the same size, eliminating fragmentation within the slab. New objects come from the slab's free list; deallocation returns to the list. A slab with no free objects is retired. This is effective for homogeneous allocations.

**Bump allocation** is the simplest: maintain a pointer into a pre-allocated block; each allocation increments the pointer. Deallocation is a no-op. Bump is only viable when all allocations live for roughly the same duration (e.g., within a single request in a web server). Many JIT compilers and allocators for short-lived objects use bump allocation.

## Fragmentation and Compaction

External fragmentation—when free memory is scattered across many small blocks—cannot be eliminated without compaction (moving objects to pack memory). Most language runtimes (Java, Go, Python with certain GC modes) use garbage collection and compaction; C allocators do not move objects (pointers would break) and instead rely on size segregation and pooling.

Some C allocators (mimalloc, jemalloc) employ **decommit** strategies: if a page remains mostly free, the allocator returns it to the OS via `madvise`, freeing virtual address space and physical memory.

## Overriding malloc

Applications can override `malloc` by setting `LD_PRELOAD` to a custom allocator library path (e.g., `LD_PRELOAD=/usr/lib/libjemalloc.so.2`), forcing the dynamic linker to use the custom allocator. This is useful for performance tuning without recompilation. Kernel code uses different mechanisms (kernel memory pools like `kmalloc` on Linux) and cannot use userspace allocators.

## Trade-offs Summary

| Allocator | Speed | Fragmentation | Concurrency | Memory overhead |
|-----------|-------|---------------|-------------|-----------------|
| dlmalloc | Moderate | High (simple bins) | Poor (global lock) | Low |
| ptmalloc | Moderate | Moderate | Good (per-thread) | Moderate |
| jemalloc | High | Low (size classes, per-CPU) | Excellent | Moderate |
| tcmalloc | High | Low | Excellent | Moderate |
| mimalloc | High | Very low | Excellent | Low |
| OpenBSD | Slow | Excellent | Good (mmap/munmap) | High |

The "best" allocator depends on the workload. Multithreaded servers benefit from jemalloc or tcmalloc. Security-critical systems may prefer OpenBSD's paranoid approach. Embedded systems might use a custom bump allocator. Most modern systems use jemalloc or a variant thereof.

See also: virtual memory, concurrency patterns, performance profiling, sanitizers.