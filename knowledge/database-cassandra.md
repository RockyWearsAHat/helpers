# Apache Cassandra

## Data Model

Cassandra is a wide-column distributed database designed for high write throughput and linear horizontal scalability. Data is organized into keyspaces (analogous to databases), tables (column families), and rows.

### Wide-Column Concepts

| Concept       | Description                                                        |
| ------------- | ------------------------------------------------------------------ |
| Keyspace      | Top-level namespace, defines replication strategy                  |
| Table         | Collection of rows with a defined schema                           |
| Partition     | Unit of data distribution, all rows sharing the same partition key |
| Row           | Unique combination of partition key + clustering columns           |
| Column        | Name-value pair with a timestamp, stored per row                   |
| Static column | Shared across all rows in a partition (one value per partition)    |

### Partition Key and Clustering Key

```cql
CREATE TABLE orders (
    customer_id UUID,          -- partition key
    order_date TIMESTAMP,      -- clustering column
    order_id UUID,             -- clustering column
    total DECIMAL,
    items LIST<FROZEN<item_type>>,
    PRIMARY KEY ((customer_id), order_date, order_id)
) WITH CLUSTERING ORDER BY (order_date DESC, order_id ASC);
```

- **Partition key** `(customer_id)`: Determines which node stores the data. Hashed via Murmur3 to a token in the ring.
- **Clustering columns** `(order_date, order_id)`: Determine sort order within the partition. Stored as a sorted structure on disk.
- **Composite partition key**: `PRIMARY KEY ((region, customer_id), order_date)` — both columns hashed together.

### Query-First Data Modeling

Cassandra requires you to model tables around your queries, not around entities. Each query pattern typically gets its own table.

```
Query: "Get all orders for customer X in the last 30 days"
  → Table: orders_by_customer (partition: customer_id, clustering: order_date DESC)

Query: "Get all orders in region Y on date Z"
  → Table: orders_by_region_date (partition: (region, date), clustering: order_id)

Query: "Get order details by order_id"
  → Table: orders_by_id (partition: order_id)
```

Denormalization is the norm. The same data lives in multiple tables. Writes are cheap; reads from wrong partitions are expensive or impossible.

## CQL (Cassandra Query Language)

### DDL

```cql
CREATE KEYSPACE ecommerce
    WITH replication = {'class': 'NetworkTopologyStrategy', 'dc1': 3, 'dc2': 3};

CREATE TABLE ecommerce.users (
    user_id UUID PRIMARY KEY,
    email TEXT,
    name TEXT,
    created_at TIMESTAMP
);

ALTER TABLE ecommerce.users ADD phone TEXT;

CREATE INDEX ON ecommerce.users (email);  -- secondary index (use sparingly)

CREATE MATERIALIZED VIEW ecommerce.users_by_email AS
    SELECT * FROM ecommerce.users
    WHERE email IS NOT NULL AND user_id IS NOT NULL
    PRIMARY KEY (email, user_id);
```

### DML

```cql
-- Insert (also acts as upsert — last-write-wins)
INSERT INTO users (user_id, email, name, created_at)
VALUES (uuid(), 'alice@example.com', 'Alice', toTimestamp(now()))
USING TTL 86400;

-- Update (creates row if it doesn't exist)
UPDATE users SET name = 'Alice Smith' WHERE user_id = ?;

-- Delete
DELETE FROM users WHERE user_id = ?;

-- Batch (use only for atomicity within ONE partition)
BEGIN BATCH
    INSERT INTO orders_by_customer (...) VALUES (...);
    INSERT INTO orders_by_id (...) VALUES (...);
APPLY BATCH;

-- Range query on clustering column
SELECT * FROM orders
WHERE customer_id = ? AND order_date >= '2025-01-01'
ORDER BY order_date DESC
LIMIT 50;
```

### Collection Types

```cql
-- Sets, lists, maps
ALTER TABLE users ADD tags SET<TEXT>;
ALTER TABLE users ADD preferences MAP<TEXT, TEXT>;
ALTER TABLE users ADD login_history LIST<TIMESTAMP>;

UPDATE users SET tags = tags + {'premium'} WHERE user_id = ?;
UPDATE users SET preferences['theme'] = 'dark' WHERE user_id = ?;
```

## Consistency Levels

| Level          | Description                                                        | When to Use                          |
| -------------- | ------------------------------------------------------------------ | ------------------------------------ |
| `ANY`          | Write succeeds if any node (including hinted handoff) acknowledges | Fire-and-forget logging              |
| `ONE`          | One replica acknowledges                                           | Low-latency reads where stale is OK  |
| `QUORUM`       | Majority of replicas (`RF/2 + 1`)                                  | Default for strong consistency       |
| `LOCAL_QUORUM` | Quorum within the local datacenter                                 | Multi-DC with local consistency      |
| `EACH_QUORUM`  | Quorum in each datacenter (writes only)                            | Cross-DC strong consistency          |
| `ALL`          | All replicas must acknowledge                                      | Rarely used; one node down = failure |

**Strong consistency formula**: `R + W > RF` where R = read CL, W = write CL, RF = replication factor. `QUORUM` reads + `QUORUM` writes with RF=3 gives strong consistency (2 + 2 > 3).

## Replication

### Strategies

```cql
-- SimpleStrategy: for single-datacenter (dev/test only)
{'class': 'SimpleStrategy', 'replication_factor': 3}

-- NetworkTopologyStrategy: for production, multi-DC
{'class': 'NetworkTopologyStrategy', 'us-east': 3, 'eu-west': 3}
```

### Replication Mechanics

- Each partition has a **primary replica** (determined by token range) and `RF-1` additional replicas on subsequent nodes in the ring.
- **Rack-aware placement**: Cassandra distributes replicas across racks to survive rack failures.
- **Hinted handoff**: If a replica is down during a write, the coordinator stores a hint and replays it when the node recovers (default: hints kept for 3 hours).
- **Read repair**: On read, if replicas disagree, the most recent value (by timestamp) is propagated to stale replicas.
- **Anti-entropy repair** (`nodetool repair`): Full Merkle-tree comparison between replicas. Run regularly (at least every `gc_grace_seconds`).

## Gossip Protocol

Nodes communicate cluster state via gossip — a peer-to-peer protocol where each node periodically exchanges state with 1-3 random nodes.

- **Heartbeat**: Generation number (node restart counter) + version number (incremented each update)
- **State propagation**: Node status (NORMAL, LEAVING, JOINING, MOVING), load, schema version, tokens
- **Failure detection**: Phi-accrual failure detector — assigns a suspicion level rather than binary alive/dead
- **Seed nodes**: Bootstrap entry points for gossip. Not special once the cluster is running. Use 2-3 seeds per DC.

## Compaction Strategies

SSTables are immutable on-disk files. Compaction merges them to reclaim space and remove obsolete data.

| Strategy                      | Best For               | Behavior                                                                                                         |
| ----------------------------- | ---------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `SizeTieredCompaction` (STCS) | Write-heavy workloads  | Merges similarly-sized SSTables. Good write amplification, poor space amplification.                             |
| `LeveledCompaction` (LCS)     | Read-heavy workloads   | Organizes SSTables into levels (L0, L1, ...). Guarantees 90% of reads hit 1 SSTable. Higher write amplification. |
| `TimeWindowCompaction` (TWCS) | Time-series data       | Groups SSTables by time window. Old windows are never compacted again. Ideal with TTL.                           |
| `UnifiedCompaction` (UCS)     | Cassandra 5.0+ default | Adaptive strategy that adjusts between tiered and leveled behavior based on workload.                            |

```cql
ALTER TABLE events WITH compaction = {
    'class': 'TimeWindowCompactionStrategy',
    'compaction_window_unit': 'DAYS',
    'compaction_window_size': 1
};
```

## Tombstones and TTL

### Tombstones

Deletes in Cassandra don't remove data immediately — they write a **tombstone** marker. Data is actually removed during compaction after `gc_grace_seconds` (default: 10 days).

**Tombstone problems**:

- Too many tombstones slow reads (Cassandra must scan past them)
- Default warning at 1,000 tombstones per read, failure at 100,000
- Common cause: deleting many rows in a partition, or wide rows with TTL'd columns

**Mitigation**:

- Use TTL instead of explicit DELETEs where possible (cleaner compaction)
- Use TWCS for time-series to compact entire time windows away
- Run `nodetool repair` before `gc_grace_seconds` expires to prevent zombie data
- Monitor with `nodetool tablestats` — check `Average tombstones per slice`

### TTL (Time-To-Live)

```cql
-- Per-insert TTL
INSERT INTO sessions (id, data) VALUES (?, ?) USING TTL 3600;

-- Per-update TTL
UPDATE sessions USING TTL 3600 SET data = ? WHERE id = ?;

-- Default TTL on table
ALTER TABLE sessions WITH default_time_to_live = 3600;

-- Check remaining TTL
SELECT TTL(data) FROM sessions WHERE id = ?;
```

## Lightweight Transactions (LWT)

Paxos-based compare-and-set for linearizable consistency. Expensive (4 round trips vs 1 for normal writes).

```cql
-- Insert if not exists
INSERT INTO users (user_id, email, name)
VALUES (uuid(), 'alice@example.com', 'Alice')
IF NOT EXISTS;

-- Conditional update
UPDATE users SET email = 'newemail@example.com'
WHERE user_id = ?
IF email = 'oldemail@example.com';

-- Returns [applied] = true/false
```

**LWT caveats**:

- 4x latency of normal operations
- Mixing LWT and non-LWT writes on the same partition can cause inconsistencies
- All LWT operations on a partition are serialized through a single Paxos leader
- Use `SERIAL` or `LOCAL_SERIAL` consistency for LWT reads

## Nodetool Reference

| Command                                | Purpose                                                                 |
| -------------------------------------- | ----------------------------------------------------------------------- |
| `nodetool status`                      | Cluster overview: nodes, state, load, tokens, ownership                 |
| `nodetool info`                        | Local node details: uptime, load, heap usage                            |
| `nodetool ring`                        | Token ring layout                                                       |
| `nodetool repair`                      | Anti-entropy repair (run regularly)                                     |
| `nodetool cleanup`                     | Remove data that no longer belongs to this node (after topology change) |
| `nodetool compact`                     | Force compaction                                                        |
| `nodetool flush`                       | Flush memtables to SSTables                                             |
| `nodetool tablestats <keyspace.table>` | Table-level stats: reads, writes, SSTable count, tombstones             |
| `nodetool tpstats`                     | Thread pool stats (detect bottlenecks)                                  |
| `nodetool describecluster`             | Cluster name, snitch, partitioner, schema versions                      |
| `nodetool decommission`                | Gracefully remove node from cluster                                     |
| `nodetool drain`                       | Flush + stop accepting writes (pre-shutdown)                            |
| `nodetool snapshot`                    | Create backup snapshot                                                  |
| `nodetool proxyhistograms`             | Coordinator-level latency histograms                                    |

## Anti-Patterns

| Anti-Pattern                                  | Problem                                          | Fix                                                     |
| --------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------- |
| Large partitions (>100MB)                     | Slow reads, compaction pressure, memory pressure | Break partition with additional clustering or bucketing |
| Secondary indexes on high-cardinality columns | Full cluster scan                                | Create a dedicated lookup table                         |
| Using `ALLOW FILTERING`                       | Scans all partitions                             | Redesign table for the query                            |
| Unbounded `IN` clauses                        | Creates N sub-queries                            | Limit IN list or restructure                            |
| `SELECT *` without partition key              | Full table scan                                  | Always filter by partition key                          |
| Batch spanning partitions                     | Coordinator overload                             | Use async writes per partition                          |
| Frequent schema changes                       | Schema disagreement across cluster               | Plan schema upfront, use `ALTER` sparingly              |
| Not running repair                            | Stale data, zombie resurrections                 | Schedule repair within `gc_grace_seconds`               |

## Production Tuning

```yaml
# cassandra.yaml key settings
num_tokens: 16 # vnodes per node (256 legacy, 16 for new)
concurrent_reads: 32 # match disk I/O capability
concurrent_writes: 32 # match CPU cores
memtable_heap_space: 2048 # MB for memtables
commitlog_sync: periodic # or "batch" for durability
commitlog_sync_period: 10000 # ms (for periodic)
phi_convict_threshold: 8 # failure detector sensitivity (increase in cloud)
gc_grace_seconds: 864000 # 10 days default
read_request_timeout: 5000 # ms
write_request_timeout: 2000 # ms
```

**JVM tuning** (jvm.options):

- Heap: 8GB max (more causes GC pauses). Use G1GC for heaps > 6GB, CMS for smaller.
- Off-heap: Memtable offheap, bloom filters, compression metadata live off-heap
- Cassandra 4.0+: ZGC or Shenandoah supported for lower-latency GC

**Monitoring essentials**: Read/write latency p99, SSTable count per table, pending compactions, dropped mutations, GC pause duration, heap usage.
