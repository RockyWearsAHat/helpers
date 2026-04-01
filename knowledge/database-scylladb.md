# ScyllaDB — C++ Cassandra Compatible, Shard-Per-Core & Seastar

ScyllaDB is a C++ rewrite of Apache Cassandra focused on extreme throughput and low-latency access patterns. It maintains Cassandra wire protocol compatibility (CQL) and deployment semantics while reimplements the entire stack for single-server performance 10-100x higher than Cassandra, and maintains cluster-wide guarantees.

## The Problem ScyllaDB Solves

Cassandra in Java faces inherent limitations:

1. **Garbage collection pauses** — 200-500ms STW GC pauses kill latency SLOs
2. **Memory overhead** — JVM footprint + object headers + GC bookkeeping
3. **Inefficient thread scheduling** — Context switches, OS thread per connection

ScyllaDB rewrites in C++ (no GC), focusing on **shard-per-core** architecture (embarrassingly parallel within single server) + **userspace I/O scheduling**.

## Shard-Per-Core Architecture

Cassandra uses a thread pool (e.g., 4 threads) shared by all incoming requests; requests compete for pool resources, introducing context switching and lock contention.

ScyllaDB instead **pins one shard (logical partition of data) per CPU core**. Each core:
- Owns exclusive data partitions
- Runs its own event loop (no lock contention)
- Handles I/O without OS context switching

Incoming request routing:
```
Request arrives → Hash to shard S → Execute on core C (bound to shard S)
                                     (no locks, no thread pool contention)
```

Result: **Lock-free** within-core execution. Multi-core throughput scales linearly (8 cores ≈ 8x throughput).

### Shared State Challenges

Cluster-wide operations (gossip, repair, rebalancing) still require coordination. ScyllaDB uses:
- **Actor model** — Tasks scheduled across cores via message passing (similar to Akka)
- **Cross-core communication** — Ring buffers (lock-free queues) between shards
- **Quorum operations** — Replicas on different cores coordinate via these channels

Contrast: Cassandra's Java synchronization (locks, wait-notify) becomes ScyllaDB's message dispatch.

## Seastar Framework

ScyllaDB is built on **Seastar**, a C++ async I/O framework for network services. Seastar provides:

1. **Async I/O without callbacks hell** — `co_await` (C++20 coroutines) for readable async code
2. **Userspace scheduler** — Bypasses OS scheduler for deterministic latency
3. **Lock-free design** — Futures and promises coordinate across coroutines without mutex

Example: A Cassandra read in Seastar (pseudocode):
```cpp
future<result> read(key) {
    return fetch_from_disk(key)
        .then([](buffer buf) {
            return check_bloom_filter(buf);
        })
        .then([](bloom_result b) {
            return deserialize(b);
        });
}
```

Executes without blocking threads; Seastar schedules continuations onto available CPU.

## Lightweight Transactions (LWT)

Cassandra supports lightweight transactions (compare-and-set, CAS) for ACID-like semantics on single keys. ScyllaDB adds:

1. **Faster Paxos** — Reduced latency via optimized message passing
2. **Per-partition isolation** — LWT on partition P doesn't block partition Q
3. **Userspace coordination** — No OS syscalls for consensus

```cql
UPDATE users SET status = 'active' 
WHERE user_id = 123 
IF status = 'inactive'
```

Still linearizable on single key; doesn't change semantics, but latency improves.

## Workload Prioritization & QoS

ScyllaDB allows tagging requests with priority levels, enabling **Quality of Service (QoS)**:

```
High priority: User-facing reads
Medium priority: Batch analytics
Low priority: Background repair
```

Userspace scheduler deprioritizes low-priority work under load, protecting SLOs for critical queries.

Cassandra doesn't expose prioritization; all requests compete equally.

## Disk I/O & Cache Hierarchy

### Seastar I/O

Seastar integrates native Linux I/O (AIO, io_uring) without blocking the event loop:

```
Read request → Add to queue → Continue other work → Completion interrupt → Resume
```

No thread pool waiting on I/O; requests and responses flow through non-blocking channels.

### Memory Management

No garbage collection = predictable latency. Memory allocation strategies:

- **Buffer pools** — Pre-allocated buffers reused (avoid malloc/free overhead)
- **Arena allocators** — Allocate regions, reclaim entire region
- **Off-heap structures** — Metadata separate from column data (reduces cache misses)

## Cluster Operations: Drop-In Cassandra Replacement

### Wire Protocol Compatibility

ScyllaDB implements CQL binary protocol and shares Cassandra's gossip scheme. Existing tools (cqlsh, drivers, monitoring) work with minimal changes.

```bash
cqlsh scylla-node:9042  # Same as Cassandra
```

### Deployment Differences

**Cassandra config:**
```yaml
cluster_name: my_cluster
initial_token: 0
```

**ScyllaDB config:** (simpler)
```yaml
cluster_name: my_cluster
# Token assignment automatic based on core count
```

ScyllaDB auto-configures shard count and token distribution based on hardware. Easier to deploy than Java Cassandra.

### No Multi-Tenancy within ScyllaDB

Cassandra allows multiple keyspaces with different replication policies sharing a cluster. ScyllaDB assumes:
- Single keyspace replication strategy per cluster
- Separate cluster per tenancy tier (if multi-tenant)

Simpler model reduces bugs and coordination overhead.

## Comparison to Cassandra

| Aspect               | Cassandra        | ScyllaDB          |
|----------------------|------------------|-------------------|
| **Language**         | Java             | C++               |
| **GC latency**       | 100-500ms pauses | None (no GC)      |
| **Single-node throughput** | ~50k ops/sec | 500k-1M ops/sec   |
| **Latency (p99)**    | 10-50ms          | 0.5-2ms           |
| **Memory footprint** | 8-16GB base      | 1-2GB base        |
| **Deploy complexity** | Moderate         | Simple            |
| **Ecosystem maturity** | 15 years        | Maturing (good)   |

### When to Choose Cassandra

- Team expertise in Java ops
- Need multi-tenancy within cluster
- Wide ecosystem plugins (backup, monitoring)
- Small per-node throughput OK

### When to Choose ScyllaDB

- Latency-sensitive (p99 < 5ms required)
- High throughput on single node preferred over cluster scaling
- GC pauses unacceptable
- Simpler ops preferred

## Limitations

1. **Single-cluster per keyspace** — No multi-keyspace replication in one cluster (vs Cassandra's flexibility)
2. **Less mature ecosystem** — Backup solutions, custom extensions fewer than Cassandra
3. **No multi-tenancy** — Designed for single application per cluster
4. **Limited observability** — Logging/tracing not as rich as Cassandra community tools
5. **Operator skills gap** — Cassandra ops knowledge transfers, but tuning differs

## Use Cases

**Analytics workload:** Cassandra cluster at write limit; switch to ScyllaDB, get 10x throughput on same hardware.

**Real-time serving:** Application requires < 5ms p99 get/set; Cassandra GC pauses violate SLO; ScyllaDB meets it.

**Streaming ingestion:** Time-series metrics (Prometheus-like); Cassandra struggles with write concurrency; ScyllaDB absorbs millions of writes/sec on single node.

**Data mastery service:** Authoritative reference data with low-latency lookups; ScyllaDB's latency predictability ideal.

## See Also

- [Apache Cassandra](database-cassandra.md) — Original wide-column design
- [Distributed Replication](distributed-replication.md) — Quorum consistency model
- [Distributed Consensus](distributed-consensus.md) — Paxos for LWT