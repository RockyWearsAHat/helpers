# Garbage Collection — Allocation, Reachability, Algorithms & Trade-offs

## Overview

Garbage collection (GC) automates memory reclamation by freeing objects the program no longer references. Two categories dominate: **tracing GC** (mark objects reachable from roots, sweep unmarked memory) and **reference counting** (decrement counts on each assignment, free when count reaches zero). Tracing GC is more common; reference counting trades simplicity for overhead and poor cycle handling—though Swift ARC combines reference counting with cycle detection, and Python uses a hybrid.

The core challenge: balancing throughput (total time in GC), latency (pause time per GC), heap size, and complexity. No algorithm dominates all dimensions.

## Tracing Garbage Collection: Core Algorithms

### Mark-Sweep
The foundational algorithm. Two phases: **mark** traverses the heap from roots (stack, globals, registers) using DFS/BFS, marking reachable objects. **Sweep** scans the heap linearly, freeing unmarked objects. Issues: fragmentation (free blocks scattered, inhibiting cache locality, defeating coalescing), and pause time scales with heap size. Suitable for small heaps or throughput-oriented workloads tolerating occasional stop-the-world pauses.

### Mark-Compact
Improves on mark-sweep by relocating live objects to a contiguous region after marking, eliminating fragmentation. Two strategies: **Lisp-style pointers** (update all references during compaction—expensive but simple), and **break-forwarding pointers** (leave a forwarding address at the old location during compaction, then fix references two-pass style). Compaction cost is proportional to live object count, not heap size, making it efficient for high-survival workloads. Used when predictable allocation patterns matter.

### Copying (Semi-Space)
Divides the heap into two spaces. Garbage collect by copying live objects from the current space to the free space, then swap roles. All objects are relocated; pointers adjusted implicitly via a forwarding table. Cheap allocation (bump pointer in the target space). Doubles memory overhead—problematic where heap is constrained. Excellent for young-object collection in generational systems. Basis for many modern collectors.

### Tri-Color Marking
An incremental scheme: objects are marked **white** (unvisited), **gray** (visited but children not processed), or **black** (visited and all children processed). Process grays until none remain, then unmarked whites are garbage. Extends marking to run interleaved with the mutator (application code) without stop-the-world. Critical invariant: no black object may reference a white object. Write barriers enforce this when the mutator modifies references. Basis for concurrent collectors.

## Generational Hypothesis & Generational GC

Empirical observation: most objects die young. Generational collectors partition the heap into generations (typically young and old). Collect young frequently, old rarely. Young-generation collection via copying; old-generation via mark-sweep or mark-compact. Inter-generational references (old→young) tracked via **write barriers**—code inserted at pointer assignments to record cross-generational edges. Dramatically reduces pause time and improves cache locality by focusing effort on short-lived objects.

Trade-off: write barriers add overhead to every pointer write, and they're complex to integrate with JIT compilation (must survive recompilation).

## Concurrent GC: Low-Pause Collectors

### CMS (Concurrent Mark-Sweep)
Marks concurrently with the mutator, then sweeps (requires a stop-the-world pause to finalize marking—mutator changes references while marking runs). Reduces pause time compared to stop-the-world mark-sweep, but fragmentation remains, sweep itself is not concurrent (requires heap scanning), and write barrier overhead is substantial. Widely used in older JVMs; largely superseded by G1.

### G1 (Garbage-First)
Divides heap into ~2000 regions. Collects regions with the highest garbage density first, balancing pause-time targets (soft real-time: "collect within N ms"). Incremental concurrent marking with a write barrier, young-generation collection via copy+mark, old-generation collection region-by-region. Pause time is largely independent of heap size (depends on region throughput). Default GC in Java 9+. Complexity is high (region-tracking bookkeeping, card marking for inter-region references, evacuation failures), but predictability appeals to production systems.

### ZGC (Z Garbage Collector)
Ultra-low pause times (sub-10ms even with terabytes of heap). Concurrent marking, concurrent compaction, and concurrent reference updating via **colored pointers** (bits encoding object state in the pointer itself). Relocates objects concurrently; the load barrier intercepts heap accesses and fixes stale pointers on-the-fly. Requires 64-bit architecture (colored pointers). High throughput cost vs. G1, but pause-time jitter eliminated. Suited for latency-sensitive systems.

### Shenandoah
Similar low-pause design to ZGC (sub-10ms), but uses load barriers instead of colored pointers, allowing 32-bit support. Concurrent marking and concurrent evacuation. Compacts live objects and forwards references using an invalidation map. Less mature than G1 or ZGC at scale, but competitive for specific workloads. Different trade-off: compatible with more architectures, but load barrier cost may exceed colored-pointer cost on some CPUs.

## Reference Counting & Hybrid Approaches

### Pure Reference Counting
Each object tracks how many references point to it. Decrement on pointer reassignment; free when count reaches zero. Immediate reclamation (no GC pauses). Issues: cycle collection (circular references are never freed—must detect and break cycles separately), overhead on every pointer mutation, atomic decrements in multithreaded code, and performance degradation as counts overflow or require atomic operations.

Used in C++ smart pointers (`std::shared_ptr`), and historically in Python (until 2024; Python 3.13+ adds a cyclic GC pass).

### Swift ARC (Automatic Reference Counting)
Reference counting with compile-time optimization. Swift eliminates redundant retain/release pairs and hoists operations outside loops where possible. Swift also uses a deferred-reference-counting pass to batch decrements. Cycles are rare in Swift code (strong cycles require explicit `weak` annotations), reducing cycle-collection cost. Close to zero GC pause time. Trade-off: throughput overhead vs. tracing GC, and runtime cost of atomic operations in multithreaded contexts.

### Python: Reference Counting + Generational Tracer
Python uses reference counting as primary reclamation. Cycles detected via a separate generational mark-sweep pass on suspected cycles (objects modified while their refcount > 0). Rare in-use, but required for code with circular references. This hybrid avoids pause time for acyclic code while addressing cycles.

## Weak & Soft References; Finalization

### Weak References
References that don't prevent garbage collection. Used to avoid cycles (e.g., parent→child pointers are strong; child→parent are weak). When the object is collected, weak references return null. Manual null-check required; used in caches (cache → object is weak; object keeps itself alive if referenced elsewhere).

### Soft References
Like weak references, but the GC reclaims soft references only under memory pressure. Useful for caches that should survive unless heap is tight. Semantic varies across VMs; Java defines soft references as collected before an OutOfMemoryError is thrown.

### Phantom References
Used for cleanup callbacks when an object is unreachable. The object is already gone; phantom references are used to queue cleanup actions. More predictable than finalizers (see below).

### Finalization
Objects can define a finalizer—code run before the object is reclaimed. Pitfall: finalizers run in a finalizer thread; if slow, they back up reclamation, causing pause-time spikes and memory bloat. Also non-deterministic timing. Modern practice: avoid finalizers; use try-with-resources or weak/phantom references instead.

## GC Tuning Trade-offs

### Throughput vs. Latency
Mark-sweep maximizes throughput (minimal overhead per allocation); concurrent collectors minimize pause time but incur per-access barriers and coordination overhead. Real-time systems (aircraft, finance) pay throughput tax for latency guarantees. Throughput-oriented systems (batch processing) pay pause time.

### Heap Overhead
Copying collectors (copying, G1, ZGC) require spare heap capacity (copying needs free space; colored pointers in ZGC reserve encoding space). Mark-sweep in-place has no overhead.

### Allocator Complexity
Bump-pointer allocation (copying, semi-space) is O(1) but requires a dedicated free space. Free-list allocation (mark-sweep) handles fragmentation but has variable-cost searches.

### Latency Predictability
Generational GC with write barriers is predictable: young collections are fast and frequent, old collections rare. Fully concurrent collectors (ZGC, Shenandoah) flatten latency over time but add barrier overhead.

## Real-World Examples

- **Java**: Serial (mark-sweep), Parallel (parallel mark-sweep), G1 (incremental regional compaction), ZGC/Shenandoah (concurrent region-based)
- **Python**: Refcounting + mark-sweep generational backup
- **Go**: Concurrent mark-sweep with write barriers; designed for short cycles (~10ms)
- **Rust**: No GC (ownership-based memory management); allows zero-cost abstractions
- **Ruby**: Mark-sweep with write barriers; generational since 2.1

## Key References

- *The Garbage Collection Handbook* (Jones, Hosking, Moss) — comprehensive technical treatment
- *Crafting Interpreters* (Nystrom) — accessible explanation of mark-sweep and generational GC
- OpenJDK GC documentation for tuning parameters and choice matrices