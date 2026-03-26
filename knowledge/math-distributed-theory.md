# Distributed Systems Theory — Impossibilities, Consistency, and Convergence

## Overview: The Limits of Coordination

Distributed systems operate across multiple independent nodes connected by unreliable communication. Theory in this domain establishes fundamental limits: what cannot be achieved no matter the algorithm or implementation. These impossibility results shape practical system design.

The landscape:
- **Impossibilities:** FLP, CAP, and relatives bound what's achievable.
- **Consistency models:** Formalizations of what "correct" means in the presence of concurrency.
- **Convergent systems:** CRDTs and eventual consistency offer weaker guarantees with practical benefits.

## The FLP Impossibility Theorem

Fischer, Lynch, and Paterson proved in 1985 that **no deterministic consensus algorithm can guarantee progress despite asynchrony and a single crash failure**.

### The Setup

- **Asynchronous system:** No bounds on message delays or relative node speeds. You cannot distinguish a crashed node from one that is slow.
- **Crash faults:** Nodes can crash and never recover, but don't send corrupted messages.
- **Consensus problem:** Nodes start with input values, must decide on a single value, obeying:
  - **Validity:** The decided value was proposed by some node.
  - **Agreement:** All non-faulty nodes decide the same value.
  - **Termination:** All non-faulty nodes eventually decide.

### The Impossibility

FLP proves that no deterministic protocol satisfies all three properties in an asynchronous system with even one crash failure. In particular, **termination cannot be guaranteed**; the protocol can enter a state where no progress is possible.

### Proof Sketch

The proof uses an adversarial argument: an adversary controls message delivery (delaying but never dropping). It shows that from any consensus protocol, an adversary can construct an "indecisive" state where no execution leads to agreement without risking a crash. This contradicts the existence of a terminating consensus algorithm.

### Implications

1. **Real systems are partially synchronous:** Practical systems assume that (eventually) message delays become bounded—long periods of asynchrony are punctuated by synchrony windows. This breaks the FLP barrier.
2. **Randomization works:** Probabilistic algorithms can achieve consensus asynchronously with crash faults. The adversary cannot be fully arbitrary if the algorithm uses randomness (coin flips), though it can be computationally bounded.
3. **Byzantine faults are worse:** If nodes can fail arbitrarily (send corrupted messages), consensus under asynchrony and Byzantine faults is impossible even with randomization in certain configurations.

## The CAP Theorem

Eric Brewer's CAP theorem (Brewer, 2000; Gilbert-Lynch proof, 2002) states that a distributed system cannot simultaneously guarantee all three of:

- **Consistency (C):** All nodes see the same data at the same time.
- **Availability (A):** Every request receives a response (non-error).
- **Partition tolerance (P):** System continues operating despite network partitions (broken links).

### Formal Statement

In a distributed system, if a network partition occurs, the system must choose between:

1. **CP:** Stop accepting writes to nodes in minority partitions, maintaining consistency.
2. **AP:** Accept writes in all partitions, sacrificing consistency. Partitions will eventually reconcile, but temporarily diverge.

### What CAP Does NOT Claim

CAP does **not** say "pick two" in steady state—most modern systems are AP or CP depending on failure scenarios. systems that are CA (no partition handling) are non-distributed (single coherent machine).

The theorem applies specifically to **partition scenarios** (rare but catastrophic). Between partitions, systems can and do maintain both consistency and availability.

### Common Misconceptions

- **CAP is about latency:** No. A system can be CA with high latency; CAP is about fault tolerance under partition.
- **CAP limits throughput:** No. CAP is a logical concern, not a performance ceiling.
- **Eventual consistency violates CAP:** No. Eventual consistency is an AP system; it sacrifices immediate consistency.

### Practical Relevance

Brewer later formalized the trade-off: under partition, systems slide on a **consistency-availability spectrum**—not an all-or-nothing choice. A CP system can achieve high availability if partitions are rare; an AP system can achieve high consistency if partitions heal quickly.

## The CALM Theorem

Consistency as Logical Monotonicity (CALM) theorem (Hellerstein et al., 2010) provides a formal connection between monotonic logic and consistency in distributed systems.

### Monotonicity

A program is monotonic if it produces output only by deduction from input: adding new input can only increase output, never retract previous conclusions.

```
Monotonic: SELECT * FROM users WHERE age > 18;
           (adding users never retracts previous results)

Non-monotonic: SELECT * FROM users WHERE age > 18 AND approved = TRUE;
               (if approval status flips, results change non-monotonically)
```

### The Insight

CALM proves that **a distributed program is eventually consistent without coordination if and only if it is monotonic**.

- **Intuition:** Monotonic programs tolerate out-of-order delivery and partial information. Non-monotonic programs require seeing all input before deciding, necessitating coordination (consensus, locks).
- **Practical consequence:** Identifying monotonic subcomputations enables splitting them from coordinated ones, reducing synchronization overhead.

### Example: Distributed Aggregation

```
Monotonic: Counting unique visitors (add-only set)
           Each node independently adds visitors to its set.
           Global count = union of all sets.
           Non-monotonic: Detecting when count exceeds threshold
           (requires coordination: stopping new additions once threshold met)
```

## CRDTs: Conflict-Free Replicated Data Types

Replicated data without coordination diverges under concurrent updates. CRDTs (Shapiro et al., 2011) are data structures designed so that **any two replicas converge to the same state** without requiring consensus or a central authority.

### Formal Model

A CRDT is an abstract data type with two interpretations:

1. **State-based CRDT:** Full state is replicated; updates produce new states that are merged via a join operation. Merge must be:
   - Commutative: a ⊔ b = b ⊔ a
   - Associative: (a ⊔ b) ⊔ c = a ⊔ (b ⊔ c)
   - Idempotent: a ⊔ a = a (duplicate updates are harmless)

2. **Operation-based CRDT:** Operations are replicated; any permutation of delivery produces the same final state. Operations must be **commutative**.

### Lattice Algebra

CRDTs map onto lattices: partially ordered sets where any two elements have a least upper bound (join, ⊔).

```
Example: Grow-only set (G-Set)
  State: Set S
  Merge: a ⊔ b = a ∪ b (union)
  Lattice: ⊆ (subset) ordering
           Any two sets converge to their union.

Example: Last-Write-Wins Register (LWR)
  State: (value v, timestamp t)
  Merge: (v1,t1) ⊔ (v2,t2) = (v1,t1) if t1 > t2, else (v2,t2)
  Lattice: Timestamps form a total order; merge picks the later one.
  Pitfall: Relies on clocks being adequately synchronized and monotonic.
```

### Simple CRDTs

- **G-Set (Grow-only Set):** Only additions, union merge. Monotonic.
- **OR-Set (Observed-Remove Set):** Tag each element with unique IDs. Add/remove by including/excluding tagged variants. Handles concurrent add/remove.
- **Counter:** Replicate per-node counter. Global value = sum. Any replica can increment its own counter; merge is addition.
- **LWR Register:** Last write wins via timestamp.

### The Trade-off

CRDTs require **no coordination** — each replica applies operations locally. Merge is deterministic. But CRDTs sacrifice **immediate consistency**:
- LWR register can lose writes (if clocks diverge, one write overwrites another, called a "lost update").
- G-Sets cannot remove elements efficiently.
- OR-Sets require metadata per element, overhead grows with tombstones.

Practical systems (Amazon DynamoDB, Riak, Cassandra, Apple CloudKit) use CRDTs for high-availability scenarios where partitions are anticipated.

## Session Guarantees and Causal Consistency

Distributed systems offer many consistency levels. Session guarantees (Sessionc consistency, Bagwell and Carter, 1994) are consistency promises held **per client session**.

### Four Session Guarantees

1. **Read Your Writes:** A client's write is visible to the client's subsequent reads.
2. **Monotonic Reads:** Successive reads return >= previous values (no time travel backwards).
3. **Writes Follow Reads:** If a client reads value v and later writes, the write is causally after the read.
4. **Monotonic Writes:** A client's writes are applied at all replicas in the same order.

### Causal Consistency

A stronger guarantee: **all clients see causally related events in the same order**. If event A causally precedes event B (someone read A's result then wrote B), all clients see them in that order.

```
Client 1: writes x = 1
Client 2: reads x (gets 1), writes y = 2
Client 3: should see {x=1, y=2} or {x=1, y unset}, never {x unset, y=2}
```

Implementing causal consistency requires tracking causal dependencies (version vectors, Lamport timestamps). Stronger than session guarantees but weaker than strong consistency.

### Eventual Consistency vs. Session Consistency

- **Eventual consistency:** No guarantee on how long divergence persists.
- **Session consistency:** Holds within client sessions, but other clients may see different states.
- **Causal consistency:** All clients see causally ordered events, stronger guarantee.

## Impossibility Landscape

| Scenario                              | Result                                                |
| ------------------------------------- | ----------------------------------------------------- |
| Asynchronous + crash fault            | FLP: no deterministic consensus                       |
| Asynchronous + Byzantine fault        | No consensus if ≥1/3 nodes faulty                     |
| Partition (any synchrony)             | CAP: choose consistency or availability               |
| Immediate consistency + no synch      | Impossible                                            |
| Monotonic program + asynchronous      | CALM: eventual consistency without coordination       |
| Non-monotonic + partition             | Requires consensus or serialization                   |

## Cross-References

See also: distributed-consensus.md, system-design-distributed.md, distributed-replication.md, formal-verification.md.