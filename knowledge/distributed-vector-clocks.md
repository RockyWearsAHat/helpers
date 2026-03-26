# Vector Clocks and Causality — Detecting Concurrency, Conflict Resolution, and Causal Delivery

## Overview

Vector clocks track the causal relationships between events in a distributed system. They enable detection of whether two events are ordered (one caused the other) or concurrent (neither caused the other). This distinguishes **causal** updates from **conflicting** updates, enabling automatic conflict resolution in eventual consistency systems. Trade-offs span message overhead, memory per node, and support for complex causality patterns.

## The Problem: Causality Without Physical Time

In a distributed system, two events can be causally related (A → B) without any physical clock showing their relationship. Process A sends a message to process B; B's action depends on A's input. If A's clock and B's clock are poorly synchronized, we can't determine order by wall-clock timestamps alone.

**Lamport clocks** (see `distributed-clocks-ordering.md`) provide a partial order: if A's timestamp < B's timestamp, then A happened before B (causally). But they don't distinguish causality from concurrency. If A's timestamp = B's timestamp, we don't know if A and B are concurrent or if the clock increments were insufficient.

**Vector clocks** solve this: they explicitly track causality between every pair of processes.

## Vector Clocks

### Mechanism

Each process maintains a **vector** of integers, one per process. For a system of n processes:

```
Process 0: [0, 0, 0]
Process 1: [0, 0, 0]
Process 2: [0, 0, 0]
```

**On a local event or send:**
- Increment the process's own vector entry.

**On receive of a message tagged with sender's vector:**
- For each position, set local[i] = max(local[i], received[i]).
- Then increment local's own entry.

**Example trace:**

```
P0 sends a message with VC = [1, 0, 0] to P1
P1 receives, takes max([0, 0, 0], [1, 0, 0]) = [1, 0, 0], then increments own: [1, 1, 0]
P1 sends a message with VC = [1, 1, 0] to P2
P2 receives, takes max([0, 0, 0], [1, 1, 0]) = [1, 1, 0], then increments own: [1, 1, 1]
```

### Comparison

**Happens-before relation (→):** Event A at process i with VC_A happens before event B at process j with VC_B if and only if VC_A[i] < VC_B[i] AND for all k, VC_A[k] ≤ VC_B[k].

**Example:**
- [1, 0, 0] → [2, 0, 0]: First event at P0 happened before second event at P0.
- [1, 0, 0] → [1, 1, 0]: P0's event at VC=1 caused P1's event at VC=[1,1,0] (P0 communicated to P1).
- [1, 0, 0] and [0, 1, 0]: Concurrent (neither → relation exists).

### Limitations

- **O(n) metadata per event:** Each timestamp carries n integers. For 10K processes, each event metadata is large.
- **Memory overhead:** Process must retain vector entries for all n processes even if most are stale.
- **Scalability cliff:** Beyond ~1K processes, memory and network overhead become prohibitive.

## Advanced Clock Variants

### Interval Tree Clocks (ITC)

**Problem:** Vector clocks require O(n) space. Interval Tree Clocks optimize by using a hierarchical tree of intervals instead of a flat vector.

**Mechanism:**
- Processes are arranged in a binary tree. Each process owns an interval of the clock space (e.g., process 0 owns [0, 0.5), process 1 owns [0.5, 1)).
- When a process forks (e.g., a thread spawns), it splits its interval with the child.
- The clock is represented as a set of intervals, not a vector.

**Properties:**
- **Space efficiency:** O(log n) expected size (intervals, not full vector).
- **Dynamic processes:** Supports process creation/destruction without pre-allocating n slots.
- **Interval merging:** Unused intervals can be reclaimed.

**Limitations:**
- Interval splitting and merging add complexity.
- Comparison and merging operations are more involved than vector comparison.

**Use case:** Systems with dynamic process counts (Erlang VM, cloud native; see Freyr project).

### Dotted Version Vectors (DVV)

**Problem:** Vector clocks assume every event creates a new causal dependency. But in practice, multiple concurrent processes often operate on disjoint data without causal dependency. Version vectors bloat.

**Dotted version vectors** separate **current state** from **causal ancestry**.

**Mechanism:**
```
{ node_id -> counter, clock }

E.g., for a key-value pair:
  v_v_1 = { server_A -> 2, server_B -> 1 }    // v_v_1[server_A] > all concurrent versions
  v_v_2 = { server_A -> 1, server_B -> 2 }    // v_v_2[server_B] > all concurrent versions
```

When merging concurrent writes, the merge operation:
1. Compares dots; if one dominates, pick it (no conflict).
2. If neither dominates, flag a conflict (application or CRDTs resolve).

**Advantages:**
- Smaller representation: Only nodes that have written are included.
- Removes "stale" clock entries automatically.

**Use case:** Eventual consistency databases (Riak, DynamoDB, Dynamo-inspired systems).

### Bloom Clocks

**Problem:** Vector clocks in wide-area systems can have entries for thousands of datacenters, most of which are 0 (no events). Bloom clocks use **Bloom filters** to compress the clock.

**Mechanism:**
- Instead of storing an explicit vector [1, 0, 0, 2, 0, ..., 3, 0, 0], hash each (process_id, counter) pair into a Bloom filter.
- Two clocks are causally related if one clock's Bloom filter is a subset of the other (rare in Bloom filters; requires false-positive analysis).

**Properties:**
- **Compressed representation:** Fixed size (e.g., 256 bits) regardless of process count.
- **False positives:** May incorrectly report causal relationship where none exists (conservative, fails safe).
- **Trade-off:** Size vs. false-positive rate.

**Use case:** Wide-area systems with hundreds of datacenters (not widely deployed; academic interest).

## Causality in Eventual Consistency

### Conflict-Free Replicated Data Types (CRDTs)

CRDTs use version vectors to ensure concurrent updates merge deterministically **without** application logic.

**Example: Last-Write-Wins (LWW) Register**
```
{ value = "alice", version = [2, 1, 0] }
{ value = "bob", version = [2, 0, 1] }
```

When merging, the version [2, 1, 0] doesn't dominate [2, 0, 1] (P1 > P2, but P2 > P1 at position 2). The system must apply a tiebreaker (e.g., process ID); if process 1 > process 2, pick "alice". Deterministic, automated, no human conflict resolution.

### Causal Delivery

A message delivery is **causally ordered** if whenever A → B (A caused B), A is delivered before B.

**Implementation using vector clocks:**
1. Each process maintains its recv_vec = [0, 0, ..., 0] (the minimum VC it has delivered so far).
2. When receiving a message with VC, buffer it if not VC[sender] = recv_vec[sender] + 1 (must receive from sender in order) AND all other entries VC[i] ≤ recv_vec[i] (all causal dependencies delivered).
3. Deliver messages in order once conditions are met.

**Result:** The delivery system ensures causality without application involvement. If process A sends two messages M1, M2 in sequence to process B, B will receive M1 before M2 even if they arrive out of order or via different paths.

**Cost:** Potential buffering and out-of-order delivery if causal dependencies are not yet received.

## Comparison of Clock Types

| Clock Type       | Space | Scalability | Conflict Detection | Dynamic Processes |
|------------------|-------|-------------|--------------------|-------------------|
| Lamport          | O(1)  | Excellent   | No (partial)       | N/A               |
| Vector           | O(n)  | ~1K procs   | Yes (precise)      | Yes               |
| ITC              | O(log n) | 10K+ procs | Yes (precise)      | Yes               |
| DVV              | O(k)* | Excellent   | Yes (imprecise)    | Yes               |
| Bloom            | O(1)  | Excellent   | Yes (imprecise)    | Yes               |

*k = number of processes that have written (typically << n)

## Practical Considerations

### When NOT to Use Vector Clocks

- **Total order required:** Vector clocks provide partial order. If you need total order, use Lamport clocks + tie-breaker or consensus.
- **Read-heavy systems:** Every read that needs ordering adds clock overhead. If causality mattersonly for writes, track writes separately.
- **High-frequency events:** O(n) metadata per event is expensive at microsecond timescales.

### Hybrid Approaches

Many systems blend techniques:
- **etcd, ZooKeeper:** Use consensus (Raft, Zab) for total order; no need for vector clocks.
- **Cassandra:** Uses vector clocks but limits them to a configurable number of datacenters (to avoid unbounded growth).
- **Dynamo, Riak:** Use dotted version vectors (compact, sufficient for eventual consistency).

### Garbage Collection

Vector clocks grow unbounded if not pruned. Practical systems maintain a "compacted" clock by dropping entries that all processes have seen (e.g., if all processes have received a message from client C with clock timestamp 100, drop the entry).

## See Also

- `distributed-clocks-ordering.md` — Lamport clocks, NTP, physical time
- `distributed-partitioning.md` — Version vectors in multi-master replication
- `architecture-cqrs.md` — Event sourcing and causal ordering
- `database-distributed-sql.md` — Snapshot isolation using version vectors