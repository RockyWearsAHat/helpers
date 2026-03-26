# ClickHouse — Column-Oriented OLAP, MergeTree & Distributed Analytics

ClickHouse is a columnar database engine optimized for online analytical processing (OLAP), processing billions of rows per second through vectorized execution and aggressive compression. Built by Yandex, it powers real-time analytics dashboards, time-series data, and fact tables in data warehouses.

## Column-Oriented Storage: Compression & Access Patterns

Traditional row-oriented databases (e.g., MySQL, PostgreSQL) store records sequentially disk → memory: [id, name, email, age, ...]. Analytical queries often access one or two columns over millions of rows, loading much unused data.

ClickHouse stores each column separately on disk and in memory:
```
Column: id      [1, 2, 3, ..., 1000000]
Column: name    ["Alice", "Bob", "Charlie", ...]
Column: age     [28, 35, 42, ...]
```

**Benefits:**
- **Compression** — Column values homogeneous (all INT, all STRING), enabling algorithms tailored to type. Repeated values compress via dictionary encoding; numeric ranges via delta encoding. 10-100x compression ratios common.
- **Vectorization** — Single CPU instruction processes 1000 values (SIMD). Column selection skips unneeded columns (no decompression).
- **Cache efficiency** — Loading single column into L3 cache yields 1000+ relevant values per cache line.

**Cost:** INSERT/UPDATE slow (must write to all columns; no direct row update). ClickHouse designed for immutable inserts + rare updates (bulk REPLACE, not row-level mutations).

## MergeTree Engine Family

ClickHouse's default table type provides LSM-tree semantics optimized for time-series data:

**MergeTree basics:**
- Data arrives in memory buffer, flushed to disk as immutable part when buffer full
- Background **merge** thread combines small parts into larger parts (reduces query overhead of reading many small files)
- Reads may span multiple parts (merged or unmerged)

```sql
CREATE TABLE events (
  timestamp DateTime,
  user_id UInt64,
  event_type String,
  payload String
)
ENGINE = MergeTree()
ORDER BY (user_id, timestamp)
PARTITION BY toDate(timestamp);
```

**ORDER BY** — Defines sort key; data sorted on disk for range scan efficiency. Range queries on prefix of ORDER BY use index skipping (partition pruning).

**PARTITION BY** — Splits data by key (usually date). Enables efficient deletion (DROP old partitions) and parallelization (queries scan partitions independently).

**Variants:**

- **ReplacingMergeTree** — Retains only latest version per primary key; deletes deduplicated rows during merge
- **SummingMergeTree** — Sums numeric columns during merge (pre-aggregation for fact tables)
- **AggregatingMergeTree** — Applies aggregate functions during merge (e.g., max, min, count)
- **VersionedCollapsingMergeTree** — Handles versioned facts (insert truth + retraction pair with version field)

## Vectorized Execution

Queries execute on **column blocks** (typically 65,536 rows) rather than individual rows:

1. **Filter evaluation** — Apply WHERE predicates to block in tight loop; mark matching rows
2. **Aggregation** — Accumulate aggregate state per group key in hash table or pre-allocated vector
3. **Sorting** — Radix sort block in-place if small, or streaming merge-sort if results large
4. **Output** — Format (JSON, CSV) or stream to network

SIMD (Single Instruction Multiple Data) instructions apply single operation to 256-bit/512-bit registers (8-16 values simultaneously). Dramatically reduces per-row function call overhead; tight loops maximize branch prediction.

## Distributed Queries

ClickHouse separates **query processing** (coordinator) from **data storage** (shards):

```sql
CREATE TABLE events_distributed AS events
ENGINE = Distributed(cluster_name, database, events, rand());
```

Query to Distributed table:
1. Coordinator parses query
2. Transforms to local query on each shard (with modified WHERE to filter shard's data range)
3. Sends to all shards in parallel
4. Collects results, applies final aggregation (if applicable)
5. Returns merged result set

**Sharding key** — rand() (random) balances load; other keys (user_id, timestamp) co-locate related data. Query optimization depends on whether query filters shard key.

Replication (via ClickHouse Keeper, formerly ZooKeeper):
- Replicas maintain identical replicated tables
- INSERT succeeds when majority replicas acknowledge
- Query routes to one replica (quorum read configurable)

## Materialized Views

Pre-computed aggregations updated incrementally:

```sql
CREATE MATERIALIZED VIEW events_hourly_mv
ENGINE = SummingMergeTree()
ORDER BY (hour, event_type)
AS SELECT
  toStartOfHour(timestamp) as hour,
  event_type,
  count() as cnt,
  sum(payload_size) as total_size
FROM events
GROUP BY hour, event_type;
```

New INSERT into `events` triggers automatic INSERT into `events_hourly_mv` (same data flow). Queries that benefit from hourly granularity read pre-computed mv; detailed queries hit original table.

## TTL (Time-To-Live) & Data Lifecycle

Remove old data automatically:

```sql
ALTER TABLE events
MODIFY TTL timestamp + INTERVAL 90 DAY DELETE;
```

Background job deletes parts where all rows exceed TTL. Can also move old data to slower storage (S3, HDFS) via MOVE TO VOLUME clause.

## Comparison to Druid & Pinot

| Aspect | ClickHouse | Druid | Pinot |
|--------|-----------|-------|-------|
| **Primary use** | Time-series, analytics | Real-time metrics, dashboards | Real-time operational metrics |
| **Storage model** | Columnar, LSM-tree | Columnar, segment-based | Columnar, segment-based |
| **Write latency** | Batch/streaming (seconds-minutes) | Milliseconds (real-time ingestion) | Milliseconds (real-time sink) |
| **Query latency** | Fast (sub-second) | Fast (milliseconds) | Fast (milliseconds) |
| **Replication** | ClickHouse Keeper + replicas | Druid Deep Storage + nodes | Kafka replica sets + replicas |
| **SQL support** | Full SQL-92 + extensions | DrQL (simplified SQL) | PinotSQL (SQL variant) |
| **Scaling** | Horizontal via sharding | Horizontal (data nodes) | Horizontal (segment replicas) |
| **Consistency** | Eventual (replicas) | Eventual | Eventual |
| **Operator overhead** | Moderate (Keeper) | High (ZooKeeper, Deep Storage) | High (Kafka, segment servers) |

ClickHouse optimizes **throughput + flexibility** (full SQL, batch/streaming mix); Druid/Pinot prioritize **sub-second write ingestion + state management complexity**.

## Operational Patterns & Trade-offs

**Strengths:**
- Queries on 1B+ rows in sub-second (vectorization + compression)
- Full parameterized SQL (joins, subqueries, CTEs)
- Materialized views for pre-aggregation
- ACID table-level transactions
- Low storage footprint (10-100x compression)

**Weaknesses:**
- Not suitable for OLTP (INSERT slow, no row updates)
- UPDATE/DELETE expensive (requires rewriting parts)
- "Approximate query processing" only if explicitly opt-in (no probabilistic query answering)
- Distributed transactions unavailable (single-INSERT ACID only per table)
- Steep learning curve for MergeTree engine tuning (partition size, merge strategy)

**When to use:** Immutable logs, fact tables (append-only), metrics aggregation, time-series, dashboard queries against frozen snapshots.

**When not to:** Frequent row mutations, transactional consistency across rows, sub-millisecond latency required.

See also: data-warehousing.md, data-engineering-formats.md, performance-database.md, data-replication-patterns.md