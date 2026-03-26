# Database Sharding — Strategies, Shard Key Selection, and Resharding

## Overview

**Sharding** (horizontal partitioning) splits a dataset across multiple independent database instances (shards), each holding a disjoint subset of the data. Each shard is a complete database—it has its own schema, indexes, and WAL. Unlike vertical partitioning (splitting columns), sharding distributes rows.

Sharding is a scaling pattern for when a single database instance cannot handle:
- Write throughput (single-instance bottleneck)
- Data volume (cannot fit in one machine's storage)
- Query latency (too many rows for index efficiency)

The cost is operational complexity: cross-shard queries become application-level joins, transactions may span shards (coordination overhead), and resharding is intricate.

---

## Sharding Strategies

### Hash Sharding

**Mechanism:** Apply a hash function to the shard key, modulo number of shards: `shard_id = hash(key) % N`.

**Characteristics:**
- **Data distribution:** Uniform across shards (if hash function is good). Avoids hotspots from natural key skew.
- **Resharding cost:** Very high. Adding shards requires rehashing and moving ~1/N of all records.
- **Read pattern:** Single lookup. Cache-friendly (predictable shard ID).
- **Examples:** Citus uses consistent hashing for this reason; early Vitess sharding.

**When used:**
- Simple, uniform workloads where shard count is stable
- Systems that can tolerate the cost of full resharding

**Downsides:**
- Resharding is a full re-partition of data—expensive during growth.
- No natural correlation between shard range and recency/geography.

### Range Sharding

**Mechanism:** Partition key space into contiguous ranges. Shard 1: `key < 1M`, Shard 2: `1M <= key < 2M`, etc.

**Characteristics:**
- **Data distribution:** Depends on key value distribution. Natural hotspots if ranges misalign with actual traffic.
- **Resharding cost:** Moderate. Split a range: move data >= boundary to new shard.
- **Scan pattern:** Efficient range queries across shard boundaries (know which shards to scan).
- **Examples:** Traditional approach in early NoSQL systems. Used in some time-series databases.

**When used:**
- Time-series or sequential key patterns
- Workloads with natural time-based partitioning (e.g., user signup date)
- Known, stable key distribution

**Downsides:**
- Manual tuning of ranges needed. Hotspots develop as ranges become skewed.
- Resharding requires careful boundary selection; mistakes cause uneven load.

### Directory Sharding

**Mechanism:** Maintain a lookup table: `(shard_key → shard_id)`. Query the directory to find the target shard.

**Characteristics:**
- **Data distribution:** Whatever you decide. Can be changed without moving data (remap the directory).
- **Resharding cost:** Low. Remap a subset of keys in the directory, then move those rows.
- **Coordination overhead:** Directory is a single point of contention. Must be highly available.
- **Examples:** Used by Vitess with range-based sharding; DynamoDB global tables.

**When used:**
- Workloads where resharding must be incremental and non-blocking
- Systems that can tolerate a directory lookup per query
- Scenarios with complex sharding rules (e.g., customer -> shard based on contract tier, not just ID)

**Downsides:**
- Extra lookup latency (directory query precedes data query).
- Directory must be replicated and kept in sync.
- If directory is incorrect, data is unreachable—operational risk.

---

## Shard Key Selection

The **shard key** is the column used to route records to shards. Once chosen, it's nearly immutable; changing it requires resharding.

### Criteria

**1. High cardinality:** Key must have many distinct values. Shard keys with low cardinality (e.g., `country_code` with 200 values) waste shards and create hotspots.

**2. Uniform distribution:** Value distribution should be roughly even across the key's range. Example: `user_id` (randomly assigned) → good. `user_signup_timestamp` (skewed toward recent users) → potential hotspot if not reshaped into ranges.

**3. Immutability:** Shard key must not change after insertion. If a user's region changes and region is the shard key, resharding is required.

**4. Query affinity:** Choose a key that matches most queries. If the majority of queries filter by `customer_id`, use it as shard key. Applications must route all queries by this key (if not shard-aware, queries become cross-shard scans).

**5. Avoid multi-key logic:** Single shard key is easiest. Composite keys (e.g., `(account_id, request_date)`) complicate routing and resharding logic.

### Anti-Patterns

- **Shard by surrogate ID only:** If `user_id` is the sole shard key but queries frequently filter by `tenant_id`, every query is cross-shard. Better: shard by `tenant_id` if multi-tenancy dominates.
- **Shard by timestamp:** Creating shards per day/hour leads to uneven load. Old shards go cold; new ones hotspot.
- **Shard by low-cardinality field:** E.g., `payment_status` (few states) → few shards, or all rows in one shard.

---

## Resharding Strategies

Adding shards or rebalancing existing ones requires moving data. Resharding is the hardest part of sharding maintenance.

### Virtual Shards (Consistent Hashing)

**Approach:** Hash each key to a virtual shard ID (e.g., hash(key) % 1000), then map virtual shards to physical shards. Adding a new physical shard remaps only a fraction of virtual shards.

**Mechanics:**
- Virtual shards: 1000 (fixed)
- Physical shards: 3 initially. Each owns ~333 virtual shards.
- Add physical shard 4 → rebalance ownership. Shard 4 takes 250 virtual shards from others. Only 25% of keys move (250 virtual / 1000 total).

**Benefits:**
- Incremental resharding. Each new shard absorbs a fraction of data.
- Reduces write amplification during growth.

**Drawbacks:**
- Still requires coordination to migrate data.
- Virtual-to-physical mapping must be maintained and versioned.
- Used in Vitess, Redis Cluster, Cassandra.

### Consistent Hashing

**Approach:** Place keys and shards on a hash ring. Hash(key) % ring_size determines position. Shard owns keys whose hash falls in its hash range.

**Benefit:** Adding a shard only remaps its immediate neighbors' keys. ~1/N of keys move.

**Characteristic:** Mitigates the "moving all keys" problem of naive modulo sharding, but still requires data migration infrastructure.

---

## Cross-Shard Queries

When a query must span multiple shards (e.g., "find all transactions > $1000" without filtering by shard key), the application becomes the coordinator:

1. **Fan-out:** Send query to all shards in parallel.
2. **Merge:** Aggregate results at application layer (union, sort, limit).
3. **Potential N+1 problem:** If the outer query results reference other shards, each result may trigger additional shard queries.

**Optimization:** 
- Minimize cross-shard scans. Co-locate related data (e.g., customer and orders in same shard).
- Use indexing per shard (e.g., local indexes on `amount` in each Transactions shard).
- Techniques like **bloom filters** can pre-filter shards before querying (sparse data).

---

## Distributed Transactions

Transactions spanning multiple shards require coordination:

1. **Two-phase commit (2PC):** Prepare phase checks all shards; commit/abort phase finalizes. Slow, blocks on slowest shard.
2. **Saga pattern:** Sequence of local transactions with compensating transactions on failure. Weaker guarantees; eventual consistency.
3. **Single-shard guarantee:** Design schema to keep related records in one shard. Most reliable.

Vitess recommends single-shard transactions where possible; cross-shard operations use sagas or eventual consistency.

---

## Practical Systems

### Vitess

- **Approach:** Directory-based sharding with range keys (range sharding by shard key range).
- **Resharding:** Online, non-blocking. Uses VReplication to stream changes while data moves.
- **Multi-tenancy:** Can shard by `keyspace` (logical database), then by key within keyspace.
- **Used by:** YouTube, DoorDash, HubSpot.

### Citus (PostgreSQL Extension)

- **Approach:** Hash-based distribution. Tables are "distributed" (sharded) or "reference" (replicated), running on worker nodes.
- **Cross-shard queries:** Handled transparently by Citus coordinator. Pushes down subqueries where possible.
- **Multi-tenant:** Built-in sharding by `tenant_id`.
- **Used by:** Figma, Grab, Azure Database for PostgreSQL (Hyperscale).

### ProxySQL

- **Approach:** Middleware layer that routes queries to MySQL shards based on rules.
- **Sharding logic:** Application-defined or SQL-based rules.
- **Not a full solution:** Requires application awareness of shard topology.

---

## Monitoring and Operational Concerns

1. **Shard imbalance:** Monitor data size, query load per shard. Uneven shards indicate poor key distribution or changed workload.
2. **Resharding cost:** Lock/downtime, data migration time, dual-write complexity during transition.
3. **Shard failure:** One shard outage affects that shard's data. Combine with replication (primary-replica per shard) for HA.
4. **Shard topology changes:** Additions, removals, or rebalancing require careful coordination. Most systems require a coordination service (etcd, ZooKeeper in Vitess).

---

## When NOT to Shard

- Small dataset (< 100GB). Single-instance replication (primary-replica) is simpler.
- Workload not actually bottlenecked on a single database. Often a query inefficiency or connection pooling issue, not storage/compute.
- High ratio of cross-shard queries. Sharding adds latency; if you can't isolate queries to single shards, the cost outweighs benefits.
- Team lacks expertise. Sharding is operationally complex; vertical scaling (bigger hardware) is often a better first choice.

See also: distributed-partitioning, database-patterns, distributed-replication.