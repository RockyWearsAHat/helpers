# Distributed Data Partitioning — Sharding, Consistent Hashing, and Rebalancing

## Overview

Partitioning (sharding) splits data across multiple machines to scale storage, compute, and throughput. Partitioning introduces challenges: uneven load distribution (hotspots), complexity in querying across partitions, and rebalancing when nodes scale up or down. The choice of partitioning scheme fundamentally shapes query patterns, replication strategy, and operational burden.

## Partitioning Schemes

### Range Partitioning

Partition boundaries are defined by key ranges. For example, usernames A-M on partition 1, N-Z on partition 2.

**Advantages:**
- Range scans are efficient (all data for a key range is on one partition).
- Easy to understand and reason about.
- No hash computation needed.

**Disadvantages:**
- Uneven distribution if keys are skewed (e.g., many users named "Smith").
- Creates hotspots when popular key ranges receive disproportionate load.
- Requires manual rebalancing as data grows; boundaries must shift.

### Hash Partitioning

Apply a hash function to the partition key; the hash output determines which partition owns the record. For example, `hash(user_id) % num_partitions`.

**Advantages:**
- Even distribution under random key distributions.
- Automatic mapping from key to partition (no boundary management).

**Disadvantages:**
- Adding or removing partitions requires rehashing nearly all data (every key potentially maps to a new partition).
- Range queries are inefficient (data for consecutive keys may be scattered across partitions).
- Hotspots persist if the hash input itself is skewed (e.g., hash function spreads evenly, but one user_id receives 1000x more traffic than others).

### Consistent Hashing

Instead of `hash(key) % num_partitions`, which breaks on rebalancing, consistent hashing maps keys and partitions to a ring. Adding or removing a partition only rehashes keys in a limited range.

**Mechanism:**
1. Hash both keys and partition identifiers (node IDs) to positions on a ring (e.g., 0 to $2^{32} - 1$).
2. A key belongs to the first partition encountered when moving clockwise from the key's hash.
3. Removing a partition X causes keys previously owned by X to move to the next partition clockwise.
4. Adding a partition Y causes keys between the previous partition and Y to move to Y.

**Virtual Nodes:**
In basic consistent hashing, uneven partition capacities cause skewed ownership. If node A is twice as powerful as node B, give A two (virtual) positions on the ring. When A is removed, its keys distribute back to two or more other nodes proportionally. Virtual nodes improve distribution and faster rebalancing.

**Jump Hash (Google):**
An optimization that avoids the ring lookup entirely. Given a key, deterministic arithmetic produces the partition number directly, reducing memory overhead and improving rebalancing speed.

**Advantages:**
- Adding or removing nodes requires rehashing only a fraction of keys (those in the affected range).
- Scales well for large clusters.

**Disadvantages:**
- Still requires data movement (the problem is reduced, not eliminated).
- Cache misses spike during rebalancing.
- More complex to implement than simple modulo hashing.

## Secondary Indexes

Queries on non-partition keys (e.g., partition by user_id, query by email) require secondary indexing. Two strategies:

### Local Secondary Indexes (Partition-Local)

Each partition maintains indexes on non-partition columns within its own data.

**Advantage:** Index updates are fast (local to the partition).

**Disadvantage:** Queries on secondary keys require scattering to all partitions and gathering results (scatter-gather). If a query hits 1000 partitions, the system sends 1000 requests.

### Global Secondary Indexes (Term-Partitioned)

A global index is itself partitioned by the indexed term. For example, partition emails by the first letter: emails starting with A are on partition 1, B-D on partition 2, etc.

**Advantage:** Queries on secondary keys go directly to the relevant partition (no scatter-gather).

**Disadvantage:** Index writes are distributed across partitions and require coordination. If many user records are updated, many index partitions must update asynchronously, risking consistency anomalies.

## Partition Rebalancing

As data grows, hardware capacity changes, or query load shifts, existing partitions must move to new machines.

### Strategies

**Fixed Number of Partitions:**
Create more partitions than machines at the start (e.g., 1000 partitions across 10 machines, 100 per machine). When adding a machine, move ~100 partitions to it. When removing, redistribute. Rebalancing is bounded and predictable.

**Dynamic Partitioning:**
Partitions are split when they exceed a threshold size. New partitions are created and moved to underutilized nodes. Simple for operators but harder to predict performance (unpredictable splits).

**Directory-Based (Routing Table):**
A central directory maps ranges or hash buckets to machines. Rebalancing updates the directory, not the data. Queries consult the directory. Simple but introduces a coordinator dependency (must be replicated for fault tolerance).

### Rebalancing Challenges

1. **Throughput:** Moving large partitions consumes network and disk I/O, degrading performance for other users.
2. **Consistency:** Reads and writes during rebalancing must be handled correctly. Common approach: reads/writes go to the old partition until rebalancing completes, then switch to the new location.
3. **Coordination:** If rebalancing is not centralized, nodes must agree on boundaries and movement order (requires consensus).

## Hotspots

A hotspot occurs when one partition receives disproportionate load (queries, writes, or data size).

**Causes:**
- Skewed data distribution (one user account has 1 million followers).
- Skewed query patterns (one user's feed is viewed 10x more than others).
- Popular events (millions of users react to a single post).

**Mitigation:**
1. **Replication:** Replicate hot partitions to multiple machines; use sharding on the replicas to spread reads.
2. **Caching:** Cache hot items in front of the partition (removes load from the partition).
3. **Logical sharding:** Split the hot item across multiple keys by adding a random suffix (user_123_0, user_123_1, ...), then aggregate results on read.
4. **Application-level guidance:** Some systems (e.g., Cassandra) ask the application to declare hot items and auto-shed reads to secondary replicas.

## Cross-Partition Queries

Queries spanning multiple partitions incur latency and complexity.

**Scatter-Gather:** Send the query to all partitions, collect results, and merge. Slow because the result is as slow as the slowest partition (tail latency amplification).

**Shard Affinity:** Denormalize data or co-locate related data on the same partition so that queries can run locally. Trade-off: increased storage and update complexity.

**Two-Phase Reads:** Metadata queries (e.g., "find all partitions") followed by parallel data queries. Used in systems like HBase and Cassandra.

## Partition-Aware Routing

Clients maintain a routing table mapping partition keys to partition locations. On write/read, the client hashes the key, looks up the partition, and sends the request directly. Reduces hop count and latency compared to random routing.

**Coordination:** When partitions rebalance, the routing table must update. Updates are typically broadcast by a metadata service (e.g., ZooKeeper, etcd). Clients cache stale routing tables and retry if a request misses, triggering a refresh.

## See Also

- [distributed-replication.md](distributed-replication.md) — how replicas handle writes across partitions
- [distributed-consensus.md](distributed-consensus.md) — consensus protocols for rebalancing coordination
- [system-design-distributed.md](system-design-distributed.md) — holistic system design tradeoffs