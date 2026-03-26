# Consistent Hashing — Ring Topology, Virtual Nodes, and Partitioning Algorithms

## Overview

Consistent hashing maps keys and partitions to a circular namespace such that adding or removing partitions rehashes only a fraction of keys (roughly 1/n keys rehash when adding 1 partition to an n-partition system). This enables low-disruption scaling in distributed systems. Trade-offs span key distribution uniformity, rebalancing complexity, and compatibility with applications that depend on consistent key-to-partition mapping (CDNs, caches, load balancing).

## Basic Consistent Hashing

### The Ring

1. Hash keys and partition IDs to positions on a circle (e.g., 0 to $2^{32} - 1$, with wraparound).
2. A key belongs to the **first partition** encountered when moving clockwise from the key's hash position.
3. Removing a partition N causes keys that belonged to N to move to the next partition clockwise.
4. Adding a partition M between two existing partitions causes keys in the arc between the predecessor and M to move to M.

**Example:** Positions: key_hash("alice") = 20, partition A at 30, partition B at 70, partition C at 10.
- alice moves to A (next clockwise from 20).
- If A is removed, alice moves to B (next clockwise from 20 after A's removal).

### Message Complexity

**Standard modulo hashing:** $k \mapsto bucket_{k \bmod n}$. Adding a partition changes $n$, and roughly $(n-1)/n$ of all keys rehash.

**Consistent hashing:** Adding a partition rehashes only keys in a specific arc, roughly $1/n$ of all keys. In a 100-partition system, adding one partition rehashes ~1% of keys, not 99%.

## Virtual Nodes

### The Problem: Uneven Ownership

Basic consistent hashing with one position per partition causes skewed ownership if partitions have different sizes or computational capacity.

**Example:** Partition A (16-core machine) gets one position on the ring. Partition B (4-core machine) also gets one position. Both own equal key ranges, but A handles 4x the load. Keys don't redistribute if A fails; B alone must absorb A's load, causing cascade failure.

### Solution: Multiple Positions Per Partition

Assign each partition **v virtual nodes** (positions on the ring). Partition A gets positions at 10, 35, 67, 91, 120, etc.; partition B gets positions at 15, 42, 73, etc.

**Properties:**
- If A is 4x more powerful, give it 4x virtual node count.
- When A fails, its 20 virtual positions distribute to 15-20 other partitions, load is evenly absorbed.
- More granular rebalancing; adding a new partition steals small chunks from many existing partitions instead of one large chunk.

**Trade-off:** Larger virtual node counts reduce variance but increase metadata tracking and ring lookup overhead.

## Hashing Variants

### Jump Hash (Google)

Standard consistent hashing requires storing the ring (all partition positions) in memory and performing binary search on lookup. **Jump hash** avoids the ring entirely using arithmetic.

**Mechanism** (pseudocode):
```
key = hash(name)
partition = 0
for j in 1..num_partitions:
    key *= 2654435761  # magic constant
    if key >> 31 < j:  # compare MSB
        partition = j
return partition
```

Despite the loop, jump hash is **O(1) on average** and O(log n) in worst case, with much lower variance than ring-based lookup.

**Properties:**
- No ring structure to maintain; pure computation.
- Deterministic; same key always maps to same partition.
- **Minimal metadata:** Just the partition count, no per-partition state.
- **When partition is removed:** All keys rehash (because the loop bound changes). Not true consistent hashing in the purest sense, but comparable rebalance overhead to ring hashing in practice.

**Use case:** Google's load balancers use jump hash because metadata overhead and lookup speed matter more than zero rebalance on single partition removal.

### Rendezvous Hashing (HRW: Highest Random Weight)

Each partition is scored via hash(key, partition_id). The key maps to the partition with the highest score.

**Mechanism:**
```
scores = {}
for partition in all_partitions:
    scores[partition] = hash(key, partition_id)
return partition with max score
```

**Properties:**
- **Minimal movement:** When a partition is added, only keys that score higher for the new partition than their current partition move. In expectation, ~1/n of keys rehash.
- **No ring structure:** Fully symmetric; all partitions are equal.
- **Lookup cost:** O(n) (hash every partition, find max). Acceptable if partitions are ~10-100; breaks at scale.
- **Supports weighted partitions naturally:** Increase the partition's weight by including it multiple times in the hash input or using a modified scoring function.

**Use case:** Small systems where lookup speed < consistency. Used in some P2P systems and content distribution (lower scale).

### Maglev Hashing (Google Networking)

Maglev is a network load balancer algorithm (not directly a hash function) that uses a **lookup table** to map packets to backend servers. Keys (packet 5-tuple, e.g., source IP + port + destination IP + port) are hashed to a table position, and the table entry points to a backend.

**Mechanism:**
1. For each backend B, assign it a unique offset on the hash table (computed via auxiliary hash function).
2. Initialize the table as empty.
3. Iterate round-robin through backends; each backend places itself in its next available position (skipping occupied slots).
4. When a packet arrives, hash it to a position in the table and send to the backend at that position.

**Properties:**
- **Fast lookup:** O(1) fixed-position lookup.
- **Good load balance:** All backends fill roughly equal table slots.
- **Stateful connection preservation:** If a backend is removed and the table is recomputed, some connections shift to new backends, but many remain on their current backend (if the table structure allows).
- **Bounded rehashing:** With careful table management, adding a backend rehashes only that backend's connection quorum.

**Use case:** Google Front End (GFE) load balancing, high-throughput packet forwarding.

## Distribution Analysis

### Ring (Consistent Hashing with Virtual Nodes)

**Load distribution:** Expected variance in keys per partition is $O(1)$ with $v = \Theta(\log n)$ virtual nodes per partition.

**Rebalance on failure:** 1/n of remaining keys rehash per removed partition. With m failures, m/n of all keys rehash.

### Jump Hash

**Load distribution:** Depends on partition count and hash quality. With k partitions, expected variance is $O(\sqrt{k})$ (slightly skewed but acceptable).

**Rebalance on failure:** All keys potentially rehash because the partition count changes (but many land in the same partition by luck).

### Rendezvous (HRW)

**Load distribution:** Near-perfect uniformity. Expected variance is $O(1/\sqrt{n})$.

**Rebalance on failure:** Roughly 1/n of keys rehash per added partition.

| Algorithm     | Lookup | Rebalance | Scale     | Metadata Overhead |
|---------------|--------|-----------|-----------|-------------------|
| Ring + VNs    | O(log n) | O(n) minimal | 10K+    | O(vn)             |
| Jump Hash     | O(1) avg | O(n) | 10K+    | O(1)              |
| Rendezvous    | O(n)  | O(n) minimal | <100   | O(1)              |
| Maglev        | O(1)  | O(n) | 1K-10K  | O(n) table        |

## Applications

### Distributed Caches (Memcached, Redis)

Clients hash keys to caches using consistent hashing. When a cache joins, only its fraction of keys migrate. Failure doesn't cascade: remaining caches each absorb a small fraction of the failed cache's keys.

**Practice:** Clients typically use ring-based consistent hashing with virtual nodes. On cache removal, clients recompute the ring and rehash affected keys.

### CDNs

Content delivery networks map URLs to edge servers using consistent hashing variants. A user requesting a file is routed to the edge server responsible for that URL's hash. Adding a new edge server rehashes a small subset of URLs to that server.

### DynamoDB Partitioning

DynamoDB uses a variant of consistent hashing (tokens) to assign key ranges to partitions. Each key's hash falls in a token range; that range owns the key. Adding a partition (token range) causes rehashing of keys that land in the new token range—not 1/n but often smaller due to deliberate token placement.

### Load Balancing

Load balancers use consistent hashing to assign incoming connections to backend servers. Maglev (Google) uses a direct-table approach. Nginx uses ring-based consistent hashing (with virtual nodes) for upstream selection.

## Anti-Patterns and Failure Modes

### Undersizing Virtual Nodes

With too few virtual nodes (e.g., v=2), load imbalance can approach O(sqrt(n)), causing hotspots. Recommendation: v = 100-200 for systems with ~10K partitions.

### Assuming Zero Rehashing

Jump hash doesn't reduce the number of keys that must be rehashed; it only avoids the ring lookup. The application still must handle rebalancing (e.g., cache refilling for new keys after a server is added).

### Hashing Without Randomization

Some naive hashing puts all partitions at predicted positions (e.g., hash(partition_id) = partition_id). An adversarial key distribution can cause all keys to hash to one partition. Use cryptographic hashing (SHA-1, SHA-256) or randomized hashing (SipHash with seeds).

### Mixing Hash Functions

If some clients use SHA-1 and others use MD5, keys map to different partitions. Consistency breaks. Standardize on one hash function (typically SHA-256 or xxHash for performance).

## See Also

- `distributed-partitioning.md` — Broader partitioning schemes (range, hash, geo), rebalancing strategies
- `infrastructure-load-balancing.md` — Load balancer use of consistent hashing
- `algorithms-hash-tables.md` — Hash function design, collision resolution