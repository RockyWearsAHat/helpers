# CockroachDB — Distributed SQL, Raft Consensus & Geo-Partitioning

CockroachDB is a distributed, ACID-compliant SQL database built on Raft consensus for replication. It scales horizontally via range-based partitioning while maintaining strong consistency and serializability, positioning itself against Google Spanner and traditional SQL databases.

## Architecture: Ranges, Replicas & Raft

Data is divided into **ranges** (contiguous key spans, typically 64 MB). Each range is replicated across **3+ nodes** as a **Raft group**. One replica is the **Leaseholder** (elected for lease duration ~9s) and handles reads without consensus; writes must achieve Raft quorum (majority).

**Raft log-based replication:**
- Leader appends write to log, broadcasts to followers
- Followers apply when leader confirms majority acknowledgment
- On leader failure, new leader elected from followers with most recent log
- No external consensus service required (unlike Bigtable + ZooKeeper)

**Local read optimization:** Leaseholder serves read from local state without round-trip to followers, assuming no newer writes in-flight. Lease prevents split-brain: only one node holds lease per range at any time.

## SQL Layer: ACID Transactions & Serializable Isolation

CockroachDB executes SQL through a **distributed transaction protocol similar to Spanner but without atomic clocks**:

**Snapshot isolation (default):**
- Transactions read from consistent snapshot at transaction start time using logical clocks (hybrid-logical clocks, HLC). Concurrent writes don't interfere.
- Read-only transactions incur minimal overhead (no keys locked in Raft log).

**Serializable isolation (user-selectable):**
- Detects write-write conflicts; aborts a conflicting transaction. Client retries with exponential backoff.
- Implemented via **read/write intent tracking**: intent records mark keys as "locked" in consensus log.
- If transaction T1 overwrites a key read by concurrent transaction T2, T2 aborts on commit.

**Span-level locking** — Large transactions (many rows) may acquire intent spans to avoid O(n) lock overhead. Ranges within span treated as locked.

Transactions may span arbitrary ranges; CockroachDB selects commit-wait (2-phase commit) or optimistic concurrency depending on read/write pattern.

## Multi-Region: Locality-Aware Replication & Global Tables

**Zone configs** — Operators define replica placement policies per table/range:
```
CONFIGURE ZONE FOR TABLE orders:
  num_replicas = 5
  constraints = [...us-east:+, ...us-west:+, ...eu:+]
  lease_preference = LEASE_PREFERENCE (zone = 'us-east')
```

Ranges prefer leaseholder in low-latency region (e.g., us-east); read-only followers in remote regions reduce read latency elsewhere.

**Global tables** — Replicated to all regions, leaseholder placed in **home region** (usually closest to writes). Read/write from any region uses `ZONE_CONFIG` to redirect to home; stale reads (secondary replicas) available with explicit `AS OF SYSTEM TIME` clause.

**ZONE_CONFIG tradeoffs:**
- **5-way replication across regions:** higher write latency (quorum = 3), stronger against double-region failure
- **3-way (majority in home region):** lower latency, single-region outage acceptable, cross-region reads eventually consistent

## Query Planning & Execution

**Distributed execution plan:**
1. SQL parser + optimizer builds plan tree (filter, join, sort, aggregate)
2. Physical planner maps logical ops to range-aware execution (e.g., TableReader + Aggregator per range)
3. Each node executes its fragment; results streamed to coordinator
4. Coordinator merges results (e.g., sorts/groups if needed)

**Vectorized execution** — Row batches (1000+ rows) processed in tight loops for cache efficiency (see performance-cpu-caching.md). Reduces per-row function call overhead vs. iterator model.

**Automatic stats collection** — Background job samples tables, builds histogram of key distributions, feeds optimizer. Stale stats = poor plans; statistics hints available to override.

## Change Data Capture (CDC)

Streams table mutations (INSERT/UPDATE/DELETE) to Kafka, cloud storage, or CockroachDB itself:

```sql
CREATE CHANGEFEED FOR TABLE orders, customers
INTO 'kafka://broker:9092' WITH format = 'json', resolved = '1s';
```

**Guarantees:**
- At-least-once delivery per row change
- Ordered per row (no out-of-order deltas for single row)
- No ordering across rows (eventual consistency within seconds)
- Resolved timestamp: all changes before T are emitted (useful for consistent snapshots in downstream systems)

Used for maintaining materialized views, syncing to data lakes, or feeding event systems. **Staleness knob**: high frequency = frequent resolved checkpoints, low latency; sparse = efficient but stale.

## Comparison to Spanner & YugabyteDB

| Aspect | CockroachDB | Spanner | YugabyteDB |
|--------|-------------|---------|-----------|
| **Consensus** | Raft per range | Paxos (learner quorum) | Raft per tablet |
| **Timestamp** | HLC (no external clock) | TrueTime GPS+atomic clock | HLC |
| **Serializable writes** | Optimistic (abort on conflict) | Wait for write intent commit | Optimistic with compare-and-set |
| **Geo-read latency** | Write region only (fast) | Fast via TrueTime (any region) | Write region + read replicas |
| **SQL compliance** | PostgreSQL-compatible | Google SQL dialect | PostgreSQL-compatible |
| **Operational cost** | Self-managed or Cloud | Fully managed only | Self-managed or Cloud |
| **CDC** | Native Kafka/S3 output | BigQuery native | Debezium integration |

Spanner prioritizes consistent global reads (TrueTime); CockroachDB minimizes operation count (Raft-only, no external service). YugabyteDB similar to CockroachDB but different query optimizer heuristics and indexing choices.

## Operational Patterns

**Rolling upgrades** — Nodes drain leases before restart; new node version compatible with majority before restart completes. No coordinated stop-apply-restart dance.

**Backup/restore** — Full or incremental snapshots to cloud storage via `BACKUP` command. Point-in-time recovery supported (consistent with logical clock).

**Rebalancing** — Background rebalancer monitors replica counts and leaseholder distribution, moves ranges to underutilized nodes automatically (minutes to hours for large data moves).

## Trade-offs

**Strengths:**
- Distributed SQL without global synchronization (Raft only)
- ACID transactions span clusters
- Serializable isolation with automatic conflict detection
- CDC for operational analytics

**Weaknesses:**
- Write latency higher than single-region databases (Raft quorum required)
- Optimal for write-in-one-region scenario (read replicas elsewhere are stale without long waits)
- HLC less precise than TrueTime (stale reads older than wall-clock staleness config)
- Operator must tune zone configs for geo-locality (no "best-effort" default)

See also: database-distributed-sql.md, distributed-consensus.md, cloud-gcp-spanner.md, distributed-transactions.md