# Consistent Hashing — Hash Rings, Virtual Nodes, and Distributed Algorithms

## Overview

**Consistent hashing** solves a core distributed systems problem: How do you partition a large dataset across multiple machines such that adding or removing a machine requires minimal data movement?

Traditional modulo hashing ($\text{partition} = \text{hash}(k) \bmod N$) fails catastrophically: adding one machine causes nearly every key to remap, requiring full dataset redistribution.

Consistent hashing ensures that only a fraction of keys (roughly $1/N$) need to move when machines are added or removed, drastically reducing rebalancing cost and network overhead.

**Found in:** DynamoDB, Cassandra, Redis Cluster, load balancers, CDN caching hierarchies, peer-to-peer networks.

---

## Basic Hash Ring

### Construction

1. **Hash Range:** Map both keys and machines to a circular hash space (typically $[0, 2^{160})$ or $[0, 2^{128})$).

2. **Machine Placement:** Each machine gets a position $p_i = \text{hash}(\text{machine\_id})$ on the ring.

3. **Key Assignment:** Key $k$ is assigned to the **next machine clockwise** (smallest $p_i \geq \text{hash}(k)$).

**Illustration:** Consider machines A, B, C at positions 10, 50, 80 on a ring of size [0, 100). Key hashing to 65 is assigned to C (the next position ≥ 65).

### Adding/Removing Machines

- **Removing a machine:** All keys from that machine move to the next machine clockwise. Keys on other machines are unaffected.
- **Adding a machine:** The new machine "claims" keys from the previous machine clockwise. All other machines are unaffected.

**Key insight:** Only $1/N$ of keys nominally move in either case (where there are $N$ machines).

---

## Load Imbalance & Virtual Nodes

### The Problem

If machines are placed uniformly at random on the ring, some receive far more keys than others. A machine placed between two far-apart neighbors inherits both the "clockwise" segment; a machine between close neighbors gets less.

**Variance:** With $N$ real machines and uniform random placement, load variance is $O(1)$, meaning imbalance can be significant.

### Solution: Virtual Nodes

Each **real machine** is represented by many (typically 100-500) **virtual nodes** on the ring. Each virtual node is a separate hash position.

**Example:** Machine A gets virtual positions `hash("A:0")`, `hash("A:1")`, ..., `hash("A:499")`.

**Effect:** 
- Keys are distributed across 500 × N total positions instead of N positions.
- Load distribution becomes much more uniform (variance drops to $O(1/V)$ where $V$ is virtual nodes per machine).
- Removing a machine spreads its keys to all remaining machines proportionally.

**Trade-off:** More virtual nodes = more uniform load, but higher metadata overhead and slower lookup (more positions to track).

### Load Distribution Quality

With $V$ virtual nodes per machine and $N$ machines:
- Standard deviation in load per machine ≈ $\frac{\sigma}{\sqrt{VN}}$ where $\sigma$ is the per-key variance.
- Doubling virtual nodes halves load imbalance (roughly).

**Practical guidance:** 100-256 virtual nodes per machine gives acceptable balance for most systems.

---

## Hash Ring Implementation

**Data structure:**
```
sorted list: [(position, machine_id, is_virtual), ...]
```

**Lookup:**
```
function find_machine(key):
    hash_value = hash(key)
    for each position in sorted list (circularly):
        if position >= hash_value:
            return machine_id at this position
    return machine_id at first position (wraparound)
```

**Complexity:** $O(\log N)$ per lookup with binary search; $O(N)$ if a simple sequential scan is used. At scale, a segment tree or other indexed structure may be preferred.

**Rebalancing after topology change:**
```
old_ring = current hash ring
new_ring = recalculate with added/removed machine
affected_machines = machines whose key range changed
for each affected machine:
    compute new vs old key ownership
    transfer keys accordingly
```

---

## Modern Variants

### Jump Consistent Hash

**Problem:** Consistent hashing with virtual nodes requires storing metadata for each virtual node. At scale (thousands of machines, hundreds of virtual nodes each), memory overhead is significant.

**Solution (Google, 2014):** A deterministic formula that maps keys directly to machines without requiring an explicit ring.

**Algorithm (pseudocode):**
```
function jump_consistent_hash(key, num_machines):
    b = 0
    j = 0
    while j < num_machines:
        b = j
        key = (key * 2862933555777941757) % 2^64
        j = j + 1 + (key >> 33)
    return b
```

**Properties:**
- No ring data structure needed; computation is stateless.
- Maps keys uniformly across machines.
- Only $O(1)$ memory per machine.
- Adding a machine $k$ causes only keys mapping to $k$ to move.

**Trade-off:** Keys are not assigned to the "next" machine on a ring; the assignment is based on a different (but deterministic) criterion. Rebalancing still requires recomputing which machine each key belongs to, but the cost of adding/removing machines is $O(\log N)$ per key rescan, not $O(N/M)$.

### Rendezvous Hashing (HRW)

**Problem:** Both consistent hashing and jump hashing assign keys to machines in a way that depends on the full set of machines.

**Alternative criterion:** For each key, compute a hash with every machine and assign to the machine with the **highest hash value**.

**Algorithm:**
```
function rendezvous_hash(key, machines):
    max_score = -∞
    chosen_machine = null
    for each machine in machines:
        score = hash(key, machine)
        if score > max_score:
            max_score = score
            chosen_machine = machine
    return chosen_machine
```

**Properties:**
- No ring structure or virtual nodes needed.
- When a machine is removed, only keys where that machine was the top candidate move; uniform distribution ensures only ~1/N keys are affected.
- The assignment is truly stateless and symmetric (any node can compute it).

**Drawback:** $O(N)$ hash computations per key lookup (vs. $O(\log N)$ in consistent hashing with binary search).

**Used in:** Distributed cache systems, consistent load balancing where every node must independently compute the same assignment.

### Maglev Hashing

**Problem:** Consistent hashing with rebalancing can cause "thundering herd" — many machines simultaneously accessing a newly promoted machine during rebalancing, causing cascading failures.

**Solution (Google, 2016):** A deterministic rebalancing algorithm that spreads the affected keys gradually.

**Key idea:** 
1. Each machine has a **preference list** (permutation of other machines).
2. Keys are assigned to the machine highest in its preference list that is currently alive.
3. When a machine dies, keys reassign uniformly to survivors based on preference lists.

**Effect:** 
- Load redistribution is smooth (small subset of keys reassign at a time).
- Highly available; no single node becomes a bottleneck during rebalancing.

**Used in:** Google's load balancers, systems requiring controlled rebalancing during failures.

---

## Comparison of Approaches

| Approach | Lookup Complexity | Add/Remove Complexity | Metadata | Rebalancing Cost | Use Case |
|----------|-----------|----------|---------|-----------|----------|
| Ring + Virtual Nodes | $O(\log N)$ | $O(1)$ to add, $O(N/M)$ rescan | $O(VN)$ | Medium | Cassandra, standard distributed systems |
| Jump Consistent Hash | $O(1)$ | $O(1)$ to add, $O(N/M)$ recompute | $O(N)$ | Low | Google Bigtable, systems with frequent machine additions |
| Rendezvous (HRW) | $O(N)$ | $O(1)$ to add, $O(N)$ recompute | $O(N)$ | Medium | Memcached, peer-to-peer systems |
| Maglev | $O(1)$ | $O(1)$ to add, $O(\text{pref list})$ | $O(N^2)$ | Low (spread over time) | Load balancers, critical path infrastructure |

---

## Bounded Loads

**Challenge:** Even with virtual nodes, some machines may still become overloaded if data distribution is skewed or machines have heterogeneous capacity.

**Approach — Power of Two Choices:**
1. Hash the key to two random machines.
2. Assign to the machine with lower current load.

**Effect:** Load balances to within a factor of $\ln \ln N$ of optimal (theoretical lower bound).

**Drawback:** Requires tracking current load on each machine; adds state and requires load information distribution.

**Variant — Weighted Consistent Hashing:**
Adjust the number of virtual nodes per machine proportionally to its capacity. A machine with 2× capacity gets 2× virtual nodes.

---

## Real-World Implementations

### DynamoDB Partitioning

- Uses consistent hashing with virtual nodes.
- Each machine owns a "token range"; keys are assigned to the machine owning their hash value's token range.
- When machines are added, token ranges are rebalanced; keys are moved automatically.
- Virtual nodes ensure uniform distribution; 256 tokens per machine by default.

### Cassandra Ring Topology

- Each node owns a contiguous token range on a ring.
- Consistent hashing with virtual tokens (typically 256 per node).
- Gossip protocol propagates topology changes; nodes coordinate rebalancing.
- Streaming mechanism transfers affected key ranges between nodes.

### Redis Cluster

- 16,384 "hash slots" (similar to virtual nodes) partitioned across machines.
- **Manual:** Operator explicitly assigns slots to machines.
- **Semi-automatic:** Cluster can rebalance slots if nodes are added/removed.

### CDN Caching

- Edge caches use consistent hashing to direct content requests to the "nearest" cache.
- When a cache fails, its content is rehashed to adjacent caches; prevents single-cache-down from cascading to origin.

---

## Design Trade-offs

### Consistency vs. Simplicity

- **Strict consistency:** Ring-based consistent hashing provides deterministic assignment; easy to reason about.
- **Computational elegance:** Jump/Rendezvous hashing are simpler; no explicit ring structure, but harder to reason about rebalancing semantics.

### Rebalancing Strategy

- **Eager rebalancing:** Move all affected keys immediately upon topology change. Fast convergence but high transient load.
- **Lazy rebalancing:** Gradually move keys; spread load over time. Smoother but requires tracking "ownership" across topology generations.

### State vs. Statelessness

- **Stateful (ring + virtual nodes):** High metadata overhead but enables local decisions; each node knows the full topology and can compute routing independently.
- **Stateless (jump hash):** Minimal memory but requires full topology knowledge to recompute on changes.

---

## Failure Modes & Mitigations

**Problem:** Machine fails; keys are suddenly rehashed to a different machine.
- Mitigation: Replicate data; failed machine is only a loss if replicas are also unavailable.

**Problem:** Slow rebalancing overloads the new owner.
- Mitigation: Throttle rebalancing; use weighted consistent hashing; adopt Maglev-like preference lists.

**Problem:** Clock skew or timing issues cause inconsistent ring state across nodes.
- Mitigation: Gossip protocol (Cassandra) or anti-entropy (DynamoDB) ensures convergence to a stable ring view.

---

## When NOT to Use Consistent Hashing

- **Small scale (< 10 machines):** Simple modulo hashing suffices; rebalancing cost is acceptable.
- **Already using database with built-in sharding:** Don't reinvent; use the database's native partitioning.
- **Strongly consistent reads required:** Multi-machine consistency (consensus) is orthogonal to partitioning; add Raft/Paxos if needed.

---

## See Also

- distributed-partitioning.md (sharding strategies)
- database-cassandra.md (real-world implementation)
- database-dynamodb.md (another real-world implementation)
- infrastructure-load-balancing.md (load balancer consistent hashing use)
- algorithms-hash-tables.md (hash function fundamentals)