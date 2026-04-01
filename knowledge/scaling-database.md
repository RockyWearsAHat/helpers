# Database Scaling — Vertical vs Horizontal, Read Paths, Write Scaling & Operational Considerations

## Overview

Database scaling addresses two fundamental limits: **throughput** (write QPS, read QPS) and **storage** (data volume). Scaling strategies differ by constraint and workload pattern. A single database server can only grow so far; eventually, application demand exceeds what vertical scaling (bigger hardware) can economically achieve, forcing horizontal strategies.

The choice between vertical and horizontal scaling, and how to combine them, shapes application architecture for years. Reversing the decision later is expensive.

## Vertical Scaling: Single Instance Growth

**Vertical scaling** adds CPU, memory, and faster storage to a single database server.

**Characteristics:**
- **Throughput ceiling:** Eventually hits hardware limits—even the largest single server has finite CPU cores and network bandwidth. A single machine with 64 cores maxes out around 100k-200k QPS for typical transactional workloads.
- **Operations simplicity:** Single database, unified schema, no resharding, no distributed transactions.
- **Cost curve:** Initially economical (double the cores costs roughly double). Becomes less economical beyond commodity hardware (specialized servers with 256+ cores are expensive per core).
- **Latency:** Lowest. Local storage access, no network hops between data partitions.

**Typical vertical scaling sequence:**
1. SSD upgrade (latency improvement)
2. More RAM (better buffer pool hit ratio)
3. More CPU cores (for parallelism)
4. High-speed network (10Gbps → 25Gbps)

**When vertical scaling stops:** Around 100k-200k QPS for write-heavy workloads, or when a single machine's cost exceeds business tolerance. Different for read-heavy workloads (replicas can scale reads independently).

## Horizontal Scaling for Read Throughput: Replication

**Read replicas** duplicate the primary database, allowing read queries to distribute across replicas while writes still go to the primary.

### Primary-Replica Architecture

1. **Writes** → Primary database (single bottleneck for write throughput)
2. **Reads** → Primary or any replica (unlimited scaling by adding replicas)
3. **Replication lag** → Replicas apply changes asynchronously, creating a window where data diverges

**Replication lag consequence:** Replicas serve stale data. A user writes to primary, then reads from a replica that hasn't yet applied the write, seeing old data. Applications must tolerate eventual consistency for this to work.

### Consistency Levels

**Asynchronous replication:** Primary commits immediately without waiting for replicas. Replicas apply changes at their own pace (bounded by network and I/O).
- **RPO (Recovery Point Objective):** Can lose unreplicated writes if primary fails.
- **Latency:** Minimal for writes (primary doesn't wait).
- **Replica lag:** Can grow unbounded under load.

**Synchronous replication:** Primary stalls writes until replicas acknowledge receipt. Zero data loss but terrible write latency and availability risk (one slow replica blocks all writes).

**Quorum (semi-synchronous):** Primary waits for majority of replicas to acknowledge, then proceeds. Balances durability and latency.

**Read consistency options:**
- **Read from primary always:** Consistent but concentrates read load on primary.
- **Read from replica if acceptable staleness:** Distributes load but risks reading old data.
- **Session-read-consistency:** Read own writes (session-level guarantee) by routing own writes to primary, other reads to replicas.

## Horizontal Scaling for Write Throughput: Sharding

Replication doesn't increase write capacity—the primary is still the bottleneck. **Sharding** partitions data across independent database instances so writes distribute across shards.

### Sharding Strategies

See also: **database-sharding.md** for detailed shard key selection and resharding patterns.

**Hash sharding:** `shard_id = hash(key) % N`. Uniform distribution but resharding is expensive (affects all data).

**Range sharding:** Partition key space into ranges (e.g., `user_id < 1M`, `1M <= user_id < 2M`). Efficient range scans but naturally develops hotspots if ranges misalign with actual traffic distribution.

**Directory sharding:** Lookup table maps `(key → shard_id)`. Resharding only requires remapping entries; no data movement until directories are remapped. Single point of contention if not replicated.

### Cross-Shard Consequences

- **Transactions:** Cannot ACID-guarantee transactions across shards. Requires either single-shard transactions or distributed consensus (two-phase commit, expensive).
- **Joins:** Queries joining tables on different shards become application-level (fetch from shard 1, then fetch from shard 2).
- **Aggregations:** SUM, COUNT, GROUP BY across shards requires scatter-gather (query each shard, aggregate results).
- **Rebalancing:** Growing from 10 to 20 shards requires moving half the data. Downtime required unless very careful orchestration.

## Connection Pooling

Databases have finite connection slots (typically hundreds to low thousands). Each application server maintains persistent connections to avoid TCP handshake overhead. **Connection pooling** multiplexes application connections onto a smaller pool of database connections.

**Architecture:** Application servers → pgBouncer/ProxySQL → Database

**Benefits:**
- Reduces connection count to database (N app servers × 10 connections/server → 1 connection pool × 50 connections)
- Connection reuse amortizes authentication and SSL handshake cost
- Can enable fail-fast detection (healthchecks, connection draining)

**Configuration:**
- **Pool size:** Typically `2-4 × database_server_cpu_cores`. Too many idle connections waste memory; too few causes queuing.
- **Statement pooling:** Can even pool prepared statements (parse/plan once, reuse with different parameters).
- **Timeout:** Idle connection timeout prevents leaks.

## Caching Layer

Even with replicas and pooling, database load can be prohibitive. **Application-level caching** (Redis, Memcached) serves hot data from memory, eliminating database queries.

**Characteristics:**
- **Latency:** Sub-millisecond for hits vs 5-50ms for database queries.
- **Durability:** Not persistent; data loss on cache restart doesn't break application (warm up from database).
- **Staleness:** Application controls TTL; stale reads are acceptable for most use cases (user profile, product listing).

**Strategies:**
- **Cache-aside:** Application checks cache first, falls back to database. Requires explicit cache population.
- **Read-through:** Cache layer fetches from database on miss transparently.
- **Write-through/Write-behind:** Application updates cache immediately (write-through) or cache asynchronously queues writes (write-behind, riskier).

**Cache invalidation challenges:**
- **TTL-based:** Simple but risks stale data beyond TTL window.
- **Event-based:** Application explicitly invalidates cache on writes (complex if many code paths can write).
- **Database triggers:** Trigger cache invalidation on database changes (couples database to cache layer).

See also: **performance-caching-strategies.md**.

## Write Scaling Strategies

### Multi-Master Replication

Multiple databases accept writes, each replica streams changes to others. Distribution of write load but adds complexity: **write conflicts** occur when the same record is modified on different masters.

**Conflict resolution** is application-dependent: retry, merge, vector clocks, or last-write-wins. Most multi-master systems are application-specific (CRDTs, specialized databases like Cassandra) rather than SQL.

### Write Batching and Async Processing

Reduce write urgency by batching: Instead of `client → database`, use `client → queue → async worker → database`. Decouples client latency from database commit latency.

**Tradeoff:** Higher latency to "completion" but better throughput.

## Database Proxies and Middleware

**Database proxies** (PGCAT, ProxySQL, Vitess) sit between application and database tier, enabling:
- **Routing:** Direct queries to appropriate shard or replica
- **Load balancing:** Distribute connections across database instances
- **Connection pooling:** Mentioned above
- **Caching:** Query result caching at proxy layer
- **Circuit breaking:** Fail-fast on degraded databases

Vitess specifically automates sharding, resharding, and failover for MySQL.

## Partition Tolerance and Rebalancing

Scaling is not one-time; systems grow. **Rebalancing** (moving data from one shard to another) is operationally complex:
- **Downtime risk:** If not done carefully, brief outages occur.
- **Data consistency:** Must avoid duplicate writes during transition.
- **Predictability:** Some sharding strategies (directory-based) rebalance easier than others (hash-based).

Plan for rebalancing from the start; it's not a one-time event but an ongoing operational necessity.

## Choosing Your Path

**Read-heavy (OLAP, analytics):** Replicate liberally; caching layer reduces database load. Sharding unnecessary.

**Write-heavy (transactional, real-time):** Sharding is required. Keep shard count stable if possible; resharding is expensive.

**Mixed:** Replicas for reads, sharding for writes, caching for hot data, async queues for non-urgent writes.

**Single-tenant SaaS:** Sharding by tenant; each growing tenant gets promoted to dedicated shard. Simplifies updates and billing.

See also: **database-replication-patterns.md**, **database-sharding.md**, **database-indexing-strategies.md**, **performance-database.md**.