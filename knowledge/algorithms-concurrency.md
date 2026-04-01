# Concurrent Algorithms — Lock-Free Structures, Memory Reclamation, Work Stealing

Lock-free and wait-free algorithms enable safe concurrent access without explicit locks. Challenges: coordinating multiple threads, preventing data races, and managing safe deallocation of reclaimed objects.

## Foundations: Compare-and-Swap (CAS)

Most lock-free algorithms rely on **compare-and-swap (CAS)** atomic primitives. CAS atomically:
1. Loads current value from memory
2. Compares it to expected value
3. If match, stores new value (returns true); otherwise leaves unchanged (returns false)

**Semantics**: single atomic operation; no race window. Sequential consistency established via memory barriers.

Alternative: load-linked/store-conditional (LL/SC) on some architectures (ARM, PowerPC) — similar properties, slightly different semantics.

## Lock-Free Queue: Michael-Scott

Classical lock-free queue using two separate CAS operations (one on head pointer, one on tail pointer).

**Structure**: singly-linked list with dummy node; nodes contain value and next pointer. Head points to dummy; tail points to last real node (or dummy if empty).

**Enqueue**:
1. Allocate new node
2. CAS tail's next pointer to new node; retry if fails (another thread advanced tail)
3. Help advance tail pointer if needed

**Dequeue**:
1. Load head, check if empty
2. CAS head to next node; retry if fails
3. Delete old head, return its value

**Properties**: $O(1)$ average case per operation (one or few CAS attempts). Worst case $O(n)$ under extreme contention (all threads colliding). ABA problem present (requires version counters or garbage collection to fix).

**ABA Problem**: thread reads pointer $A$, another thread changes $A \to B \to A$, first thread CAS succeeds but data structure is inconsistent. Mitigated by adding version bits to pointers (double-wide CAS) or safe memory reclamation (see hazard pointers, epoch-based).

## Lock-Free Stack: Treiber

Simple lock-free stack: single top pointer. Insertion/popping use CAS on top.

**Push**:
1. Create new node, set its next to current top (load from memory)
2. CAS top to new node; retry if fails

**Pop**:
1. Load top pointer, check if empty
2. Load next pointer of top
3. CAS top to next; retry if fails
4. Delete old top, return its value

**Properties**: $O(1)$ expected per operation; ABA problem same as Michael-Scott. Much simpler than queue but suffers from higher contention under high concurrency (single top pointer is a bottleneck).

## Lock-Free Skip List

**Fraser's lock-free skip list**: skip list structure but all accesses protected by CAS and atomic marking.

Each node has a "marked" flag (deleted flag). Search, insert, delete use CAS to manage next pointers and apply node marks atomically. Enables efficient concurrent range queries (traversing level 0 yields sorted scan).

**Complexity**: expected $O(\log n)` per operation with high probability (same as sequential skip list).

**Advantage**: supports traversal (range scans) natively; better than lock-free trees for this reason.

**Disadvantage**: more complex than sequential skip list; marking/unmarking adds instruction overhead.

## Concurrent Hash Map

Scaling hash maps to multiple threads without a global lock requires strategies like **bucket-level locking** or **lock-free** implementations.

**Bucket-level locking**: partition table into segments; each segment has one lock. Inserts/deletes holding only segment lock reduces contention. Java `ConcurrentHashMap` historically used 16 segments; modern versions use finer-grained locking.

**Lock-free variant**: combine lock-free dynamic array with lock-free chains or trees within buckets. More complex; used in high-performance libraries.

## Epoch-Based Memory Reclamation (Retirement-Based Reclamation)

Solves the **memory reclamation problem**: when can we safely delete a node removed from the data structure?

**Approach**:
1. Logical deletion: mark node (e.g., set a deleted flag), remove from structure via CAS
2. Physical deletion deferred: retired/deleted nodes remain allocated until safe
3. Epoch tracking: global counter incremented periodically; threads enter/leave epochs

**Mechanism**:
- Threads announce their current epoch when accessing the data structure
- A writer retires nodes by moving them to a queue
- Reclamation thread waits until all threads have advanced past the epoch in which a node was retired
- Then physically delete the node

**Amortized cost**: $O(1)$ per operation; memory overhead for retired node queues.

**Requirement**: threads must gracefully leave critical sections (announce epoch transitions); not suitable for threads performing truly long operations.

## Hazard Pointers

Deferred object-reclamation scheme; threads announce which pointers they might dereference.

**Mechanism**:
1. Each thread maintains a small set of hazard pointers (typically 2-4)
2. Before dereferencing a pointer obtained from shared memory, thread announces it as a hazard pointer
3. Writer (thread performing deletion) collects all announced hazard pointers
4. Object is only physically freed if its address is not in any thread's hazard list

**Example**: before dereferencing a node from a lock-free queue, store its address in a hazard pointer slot. Deleting threads scan hazard pointers; if node is hazarded, defer deletion.

**Properties**: $O(k)$ memory overhead per thread ($k$ hazard pointers); $O(n)` time to scan all hazard pointers for reclamation decision. More responsive than epoch (no waiting for epoch transitions).

## Read-Copy-Update (RCU)

Synchronization mechanism for read-heavy workloads. Readers traverse data structure without locks; writers may copy and update.

**Principle**: **readers are never blocked**. Writers:
1. Publish new version of data structure (or modified substructure)
2. Wait for all readers (using quiescent state tracking or grace period)
3. Deallocate old version

**Grace period**: all CPUs must pass through a quiescent state (no reader active). Tracked via context switches, atomic counters, or IPIs (inter-processor interrupts). Complex in kernel context.

**Scope**: primarily for OS kernels (Linux RCU in scheduler, networking stack) due to kernel's control over context switching. User-space libraries rarely use it (complex to detect quiescent states).

**Trade-off**: readers extremely fast (no synchronization), writers slow (copy + grace period wait). Suits read-biased workloads (<<1% writers).

## Work-Stealing Deques

Data structure for parallel task scheduling. Each worker thread has a private deque. Work stealing enables dynamic load balancing.

**Operations**:
- **Owner only**: pushes and pops from the tail (fast, single-threaded optimization)
- **Other threads**: steal from the head (requires synchronization, typically lock-free)

**Lock-free head CAS**: stealing thread atomically exchanges head to next element. Allows owner and stealers to work concurrently without locks.

**Benefit**: scalable work distribution; heavily used in thread pools (Java ForkJoinPool, C++17 parallel algorithms).

**Complexity**: push/pop $O(1)$ amortized (owner's tail operations); steal $O(1)$ amortized (head CAS).

## Software Transactional Memory (STM)

Speculative execution model: transactions optimistically read/write, then commit atomically. If conflict detected, abort and retry.

**Execution**:
1. Read phase: transaction loads values into thread-local read-set
2. Write phase: transaction buffers writes (doesn't mutate shared memory)
3. Commit: CAS attempts to commit all writes atomically; if any readable location changed, abort and retry

**Properties**: provides isolation without explicit locks. Complex to implement; CPU support limited. High abort rate under contention degrades performance.

**Practical use**: marginal; few production systems rely on STM despite theoretical appeal. Language support experimental (Haskell transactional memory, Clojure refs). Simpler to reason about programs using explicit locks or lock-free algorithms.

## Amortized & Worst-Case Analysis

Lock-free algorithms often have **expected $O(1)`** amortized complexity but **unbounded worst-case** (under extreme contention, CAS attempts may fail many times). 

**High-probability bounds**: with $\log n$ attempts, CAS-based operations succeed with high probability. This suffices for practical concurrency; true worst-case rare in practice.

**Comparison to locks**: mutexes have $O(1)` deterministic operations (assuming no starvation), but introduce blocking and priority inversion risks. Lock-free trades bounded worst-case for better latency tail guarantees.

## Correctness: Linearizability

Gold standard for concurrent data structure correctness. A concurrent execution is **linearizable** if there exists a sequential ordering of operations respecting real-time constraints and each operation takes effect atomically at some point between its call and return.

Ensures: concurrent programs behave as if all operations occurred sequentially (despite actual parallelism). Easier to reason about than weaker guarantees (sequential consistency, relaxed atomicity).

## Tradeoffs

| Technique | Latency | Throughput | Memory | Complexity | Ready for production |
|-----------|---------|-----------|--------|-----------|----------------------|
| Lock | high (blocking) | low-medium | low | low | yes |
| Lock-free (CAS) | low (non-blocking) | medium-high | medium | high | yes (with care) |
| Epoch-based | medium | high | medium | high | yes (for read-heavy) |
| Hazard pointers | medium-high | high | high | high | yes (complex) |
| RCU | very low (readers) | low-medium | high | very high | kernel only |
| STM | variable (abort retry) | low (contention) | high | very high | experimental |
| Work-stealing | low | high | low | medium | yes (task parallelism) |

## See Also

algorithms-randomized (randomized load balancing), systems-reasoning (concurrency models), architecture-event-driven (async patterns)