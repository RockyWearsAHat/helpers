# Google Cloud Spanner — TrueTime, External Consistency & Distributed SQL

Cloud Spanner is a globally distributed relational database that maintains strong consistency at scale through **TrueTime**, a hybrid logical/physical clock bound to atomic clocks and GPS receivers. It challenges the traditional CAP theorem by making consistency and availability the priority, accepting geographic latency tradeoff.

## TrueTime Architecture

TrueTime is not a standard NTP clock. It is a **privileged system service** with explicit uncertainty bounds, used to order events globally.

### Atomic Clock Infrastructure

Spanner deployments include:
- **GPS receivers** at each data center (stratum-0 reference)
- **Atomic clocks** (cesium oscillators) synchronized to GPS, as backup if GPS fails
- **Multiple independent time references** across zones (redundancy)

Each machine queries TrueTime, which returns an interval **[earliest, latest]** rather than a point-in-time:

```
TrueTime.now() -> [10:00:00.000, 10:00:00.001]  
– Guaranteed: true time is within this 1ms window
– Uncertainty shrinks as clocks stabilize/sync
```

### Uncertainty Bounds

The uncertainty interval width varies:
- **Fresh sync:** ~1-2 ms (clocks very close)
- **Under load/network delay:** ~10+ ms (delays accumulate)
- **GPS/atomic clock failure:** Falls back to server's cesium, tolerance degrades

Spanner **waits out** uncertainty before committing writes: if a write finishes at `[10:00:00.000, 10:00:00.001]`, Spanner delays the commit timestamp to 10:00:00.001, ensuring no concurrent reader sees uncommitted state.

## External Consistency

Spanner guarantees **external consistency** (strong serializability): if transaction T1 ends before T2 begins in wall-clock time, T1's effects are visible to T2.

### How It Works

1. **Assign commit timestamps** using TrueTime, not Lamport clocks. Timestamps have real-world semantics.
2. **On write:** Spanner holds locks (X locks on rows) until the commit timestamp passes its current local time. This **wait** is the cost of external consistency.
3. **On read:** Reader uses snapshot isolation at a past timestamp, avoiding locks entirely.

Example:
```
Transaction T1 ends at wall time 10:00:00.000
  – Assigned commit timestamp: 10:00:00.001 (past TrueTime uncertainty)
  – Releases locks at 10:00:00.001
  
Transaction T2 begins at wall time 10:00:00.002
  – Can read snapshot at 10:00:00.001 or later
  – SEES T1's effects: external consistency guaranteed
```

**Tradeoff:** Writers wait (average 5-10 ms per commit for uncertainty). Readers are fast. Workloads are write-latency-sensitive.

## Schema Design: Interleaved Tables

Spanner optimizes for **hierarchical data** using **interleaving**, a co-location technique that reduces cross-shard joins.

### Parent-Child Interleaving

```sql
CREATE TABLE users (
  user_id INT64,
  name STRING,
  PRIMARY KEY(user_id)
);

CREATE TABLE accounts (
  user_id INT64,
  account_id INT64,
  balance FLOAT64,
  PRIMARY KEY(user_id, account_id),
  INTERLEAVE IN PARENT users ON DELETE CASCADE
);
```

**Storage layout:**
```
Shard 1:
  user:1000 [name]
    account:1000/001 [balance]
    account:1000/002 [balance]
  user:1001 [name]
    account:1001/001 [balance]
```

All accounts for a user are **co-located** in the same Shard. A query `SELECT * FROM accounts WHERE user_id = 1000` scans a single shard, no cross-shard fetch.

**Benefits:**
- JOIN on (user_id) becomes a single-shard scan; near-zero latency
- DELETE on parent cascades locally (one shard update)
- Avoids distributed transaction overhead

**Drawbacks:**
- Child table **inherits parent's sharding key** (loses independent cardinality control)
- Deep interleaving (3+ levels) reduces flexibility; restructuring is expensive

**When to interleave:**
- Strong 1:N relationships (users → accounts, tenants → resources)
- Frequent queries that join parent-child
- Cascading deletes are semantically required

**When NOT to interleave:**
- Many:many relationships (normalize into bridge table)
- Unbalanced hierarchies (one user with millions of accounts → skewed shards)
- Frequent child bulk operations independent of parent

## Data Access Patterns

### Stale Reads

Readers can explicitly request **stale snapshots** to avoid wait time and contention:

```java
Statement statement = Statement.newBuilder(
    "SELECT balance FROM accounts WHERE account_id = ?")
  .bind("account_id", 123)
  .build();

// Default: Strong read at current time, waits for commits
ResultSet rs1 = client.executeQuery(statement);

// Stale read: Reads snapshot 15 seconds old (no wait)
ResultSet rs2 = client.executeQuery(
  ReadOption.minReadTimestamp(Instant.now().minus(15, ChronoUnit.SECONDS)),
  statement
);
```

**Use cases:**
- Analytics queries tolerating staleness (15+ seconds)
- Read replicas in non-primary regions (1+ second lag acceptable)
- High-volume read workloads where commit latency is the bottleneck

**Consistency:** Stale reads are **consistent** but not fresh.

### Change Streams

Spanner can publish **change data capture (CDC)** to Pub/Sub when rows mutate:

```sql
CREATE CHANGE STREAM user_changes FOR users;
```

Events published to Pub/Sub:
```json
{
  "type": "UPDATE",
  "commit_timestamp": "2024-03-25T12:34:56.789Z",
  "record": {"user_id": 1000, "name": "Alice"}
}
```

**Applications:**
- Sync to external storage (BigQuery, ElasticSearch)
- Real-time analytics (materialized views in BigQuery)
- Event sourcing applications

**Ordering:** Events are ordered by commit timestamp, providing causal consistency.

## Multi-Region Deployment

Spanner supports **multi-region configurations** where replicas span geographic zones:

### Architecture

```
Config: multi-region-us (3+ regions)
  – Replica 1: us-central1 (strong leader)
  – Replica 2: us-east1 (read-only; async secondary)
  – Replica 3: us-west1 (read-only; async secondary)
```

**Write path:** All writes route to the leader (us-central1). Commits replicate asynchronously to secondaries (1-5 second lag).

**Read path:**
- **Strong reads:** Query leader, get latest data, high latency
- **Stale reads:** Query nearest replica, low latency, acceptable lag

**Failover:** If leader fails, quorum election promotes an In-Sync secondary to leader (seconds not hours).

### Latency Characteristics

| Operation           | Intra-region | Multi-region (us, 2000 km+) |
|---------------------|-------------|------------------------------|
| Strong write        | 10 ms       | 80-200 ms (round-trip)       |
| Strong read         | 5 ms        | 80-200 ms (round-trip)       |
| Stale read (1s)     | 5 ms        | 5 ms (local replica)         |

**Design pattern:** Writes to leader, stale reads from local replicas → minimize cross-region latency.

## Spanner vs Alternatives

### Spanner vs CockroachDB

| Aspect                 | Spanner                    | CockroachDB               |
|------------------------|----------------------------|---------------------------|
| Consistency            | External (TrueTime)        | Causal+ (Hybrid Logical Clock) |
| Deployment            | GCP managed                | Self-hosted, multi-cloud   |
| Clock dependency      | GPS + atomic clocks        | Server clocks (tolerate skew) |
| Write latency         | 5-200 ms (TrueTime wait)   | 1-5 ms (no clock wait)    |
| Multi-region          | native, low latency        | possible, higher latency   |
| Scaling               | Horizontal (sharded)       | Horizontal (sharded)      |
| License               | Proprietary                | Open source (BSL; MIT)    |

**Choose Spanner if:**
- Requiring strongest consistency guarantees (financial transactions, compliance-critical)
- Multi-region reads tolerate 5-50 ms latency
- GCP commitment is acceptable

**Choose CockroachDB if:**
- Self-hosting required or multi-cloud mandatory
- Write latency must be minimized
- Cost control critical (self-hosted, no compute opex)

### Spanner vs YugabyteDB

| Aspect                 | Spanner                    | YugabyteDB               |
|------------------------|----------------------------|---------------------------|
| Consistency            | External strict            | Serializability (Raft-based) |
| Architecture           | Spline (proprietary)       | Raft + doc store (open)   |
| Multi-region latency   | 5-50 ms optimized          | 50-200 ms typical         |
| Pricing                | GCP proprietary            | Self-hosted or managed    |
| Write performance      | Limited by commit latency  | Higher TPS for writes     |

**Choose YugabyteDB if:**
- Open-source architecture required
- Geographic redundancy needed without GPS dependency
- Lower write latency acceptable in exchange for less strict consistency

## Partitioning and Sharding

Spanner **automatically shards** based on primary key:

```sql
CREATE TABLE orders (
  customer_id INT64,
  order_id INT64,
  amount FLOAT64,
  PRIMARY KEY(customer_id, order_id)
);
```

Spanner splits by customer_id ranges:
```
Shard 1: customer_id [1, 100K)
Shard 2: customer_id [100K, 200K)
...
```

**Range-based sharding benefits:**
- Locality: sequential IDs co-located
- Skew avoidance: UUIDs or high-variance keys balanced

**Anti-patterns:**
- **Monotonic IDs with single large customer:** Skews one shard (hotspot). Mitigation: Reverse sharding (upper bits vary).
- **Composite key without shard diversity:** PARTITION BY the high-cardinality field first.

## See Also

- `database-distributed-sql` — CockroachDB, YugabyteDB, distributed consensus
- `distributed-transactions` — ACID across shards
- `distributed-consensus` — Raft membership changes
- `cloud-gcp-data` — Spanner in context of GCP data services