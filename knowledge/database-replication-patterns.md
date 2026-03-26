# Database Replication Patterns — Primary-Replica, Multi-Primary, Synchronization, and Failover

## Overview

**Replication** copies data from a source database (primary/leader) to one or more destination databases (replicas/followers). Replication serves multiple purposes:
- **Availability:** Replicas provide failover if the primary fails.
- **Read scalability:** Distribute read queries to replicas, freeing the primary for writes.
- **Disaster recovery:** Geographically distant replicas enable recovery from region-wide failures.
- **Reporting/analytics:** Offload expensive reads to dedicated replicas.

The tradeoff is **consistency vs. latency**. Replication is inherently asynchronous—data reaches replicas after the primary commits, creating a **replication lag window** where data on the primary and replicas diverges.

---

## Primary-Replica (Leader-Follower) Replication

**Architecture:** One primary accepts all writes; replicas consume a replication log and apply changes in order.

### Write Path

1. Client commits transaction on primary.
2. Primary writes to its local storage.
3. Primary streams changes (e.g., binary log, WAL) to replicas.
4. Replicas parse and apply changes asynchronously.

### Replication Mechanisms

**Statement-based replication:** Primary sends SQL statements. Replicas execute them.
- **Downside:** Non-deterministic statements (e.g., `NOW()`, `RAND()`, `AUTO_INCREMENT()`) may produce different results on replicas.
- **Example:** Early MySQL.

**Row-based (physical) replication:** Primary sends row changes (before/after images).
- **Advantage:** Deterministic. Exact same state replicated.
- **Downside:** Larger log volume if many rows change.
- **Example:** PostgreSQL WAL streaming, MySQL binary log with `ROW` format.

**Hybrid (mixed):** Use row-based for problematic statements, statement-based otherwise.

### Replica Consistency Levels

#### Asynchronous Replication

Primary acknowledges commit **before** waiting for replicas to apply changes.

**Characteristics:**
- **RPO (Recovery Point Objective):** Can lose transactions not yet replicated if primary fails.
- **Latency:** Minimal. Primary doesn't wait.
- **Replica lag:** Unbounded. May grow under load.

**When used:** Non-critical data, high-throughput workloads, geo-distributed setups where sync replication would be too slow.

#### Synchronous Replication

Primary stalls the committing client until **all** replicas acknowledge receipt of the change.

**Characteristics:**
- **RPO:** Zero. All acknowledged data exists on replicas.
- **Latency:** High. Primary waits for slowest replica. Single slow replica stalls the system.
- **Availability:** If any replica is unreachable, writes block. Requires careful quorum design.

**Risk:** Availability regression. Used rarely; requires dedicated fast replicas.

#### Semi-Synchronous (Quorum) Replication

Primary waits for **one or more** (typically majority) replicas to acknowledge, then commits locally and returns to client.

**Characteristics:**
- **RPO:** Weak. At least one replica has the data (may not be the one that survives).
- **Latency:** Moderate. Faster than full synchronous, slower than async.
- **Availability:** Single replica failure doesn't stall writes (if quorum > 1).

**Example:** PostgreSQL `synchronous_commit = on` (waits for replica flush), MySQL `rpl_semi_sync_master_enabled`.

---

## Multi-Primary (Multi-Master) Replication

**Architecture:** Multiple primaries accept writes independently. Each primary replicates to all others.

### Peer-to-Peer Topology

All nodes are equal. Any node accepts writes and propagates changes to peers.

**Challenge:** **Conflict resolution.** Two primaries can write to the same row concurrently:
- Primary A: `UPDATE users SET status = 'active' WHERE id = 1` (timestamp t1)
- Primary B: `UPDATE users SET status = 'inactive' WHERE id = 1` (timestamp t1 + 1ms)

Which state "wins"? Common policies:
- **Last-write-wins (LWW):** Use timestamp. Later write overwrites earlier. Simple but can lose intentional updates (causality-violating).
- **Application-specific:** Custom merge logic (e.g., field-level OR, field-specific rules).
- **Conflict detection + abort:** Flag conflicts for manual resolution.

**Durability tradeoff:** If conflicts must be resolved manually, RPO is complex (partial data applied, other data pending).

### Cascading/Star Topology

One "hub" primary replicates to multiple replicas. Each replica can replicate to its own secondaries.

**Benefit:** Reduces load on the hub. Replicas are read-only or semi-primary.

---

## GTID (Global Transaction IDs)

**Purpose:** Identify transactions across a replication cluster uniquely, enabling safe failover and resuming replication from arbitrary points.

**Format (MySQL/PostgreSQL):** `server_uuid:transaction_number`

### Use Cases

**Failover:** When primary fails, promote a replica. Use GTID to ensure replicas resume from where they left off (not by binlog filename/offset, which is fragile).

**Replication chain changes:** Move a replica from one primary to another. GTID ensures no gaps or duplicates.

**Cross-cluster replication:** Cluster A's primary→replicas; Cluster A's replica replicates to Cluster B's primary (cascade).

### Configuration

**MySQL:**
```sql
SET GLOBAL gtid_mode = ON;
SET GLOBAL enforce_gtid_consistency = ON;
```

**PostgreSQL:** Built into WAL streaming; logical replication uses LSN (log sequence number) instead.

---

## Replication Lag Monitoring

**Replication lag:** Time between primary commit and replica applying the change. Measured as clock time or transaction count.

### Monitoring Methods

1. **Query-based:** `SHOW SLAVE STATUS\G` (MySQL) → `Seconds_Behind_Master`. Compares replica's apply timestamp to primary's commit time.
2. **Heartbeat tables:** Primary continuously updates a marker table; replica's replica updates its own marker table. Application compares timestamps.
3. **GTID lag:** Track LSN/GTID applied on replica vs. primary. More reliable than clock-based methods.

### Implications

- **< 1 second:** Acceptable for most applications. Reads from replica are nearly consistent.
- **> 10 seconds:** Significant lag. Application must decide: tolerate stale data, or route reads to primary.
- **Unbounded lag:** Indicates replica bottleneck (slow apply, network issues, or primary producer very fast). Investigate.

### Causes

- **Network latency:** Slow replication channel.
- **Replica hardware:** Slower disk/CPU than primary.
- **Large batch writes:** Primary writes fast; replica apply lags.
- **Complex queries on replica:** Long-running read queries block replication apply (single-threaded in some systems).

---

## Failover

### Automatic Failover

Cluster manager (Patroni, etcd, Kubernetes) detects primary failure and promotes a replica.

**Detection methods:**
- Heartbeat timeout (primary missed N health checks).
- Quorum verification (split-brain prevention).

**Promotion steps:**
1. Promote replica to primary (GTID ensures it's most up-to-date).
2. Update DNS/service discovery to point to new primary.
3. Demoted primary (if recoverable) becomes replica.

**RTO (Recovery Time Objective):** Seconds to minutes (depends on detection + promotion speed).

**RPO:** Depends on replication mode (async: potential data loss; semi-sync: loss up to committed but not on failed primary).

### Manual Failover

Operator manually specifies which replica becomes primary.

**Pros:** Safer. Operator can verify replica is up-to-date before promotion.
**Cons:** Slower. Manual intervention on-call.

---

## Read Replicas

**Purpose:** Dedicated replicas for read-only queries, separating OLTP (primary) from OLAP (replica analysis).

### Deployment

**Local replicas:** Same datacenter as primary. Reduce read latency on primary; enable failover.

**Distant replicas:** Different region or cloud. Slower reads (network latency), but enable disaster recovery and reduce egress costs for distant clients.

### Application Routing

- **Read-after-write consistency:** Client writes to primary, then reads from primary (ensure own write is visible). Or: use replica but check GTID >= client's last write.
- **Eventual consistency:** Accept that reads may see stale data (replication lag).

---

## Cascading Replication

**Topology:** Primary → Replica1 → Replica2 → Replica3...

**Benefits:**
- Reduces load on primary. Replica1 handles replication to downstream replicas.
- Enables multi-tier fallback.

**Drawbacks:**
- **Lag amplification:** Lag increases at each level (Replica1 applies after primary; Replica2 applies after Replica1).
- **Failure isolation:** If Replica1 fails, entire downstream tree loses replication.

**Monitoring:** Watch cascade of applied GTIDs to detect stalling at any level.

---

## Consistency Models in Replication

### Eventual Consistency

Replicas eventually match the primary, but reads may return stale data temporarily.

**Used when:** Acceptable staleness is known (e.g., "no older than 30 seconds").

### Read-Your-Own-Writes (RYOW)

A client's reads reflect its own writes, even if reading from a replica.

**Implementation:**
- Track the GTID/LSN of the client's last write.
- Before reading, check replica's applied GTID >= client's write GTID. If not, wait or route to primary.

### Causal Consistency

If transaction A happened-before transaction B, all observers see this order.

**Hard to implement in replicated systems** without strong consistency (sync replication). Most systems accept eventual consistency as an edge case.

---

## Operational Patterns

### Promoting a Replica to Primary

1. Stop writes to current primary (or verify it's unreachable).
2. Identify most up-to-date replica using GTID.
3. Promote via `PROMOTE STANDBY TO PRIMARY` (PostgreSQL) or similar.
4. Update application config/DNS to point to new primary.
5. Rebuild demoted primary as replica attaching to new primary.

### Adding a New Replica

1. Take backup of primary (or existing replica).
2. Restore backup on new replica.
3. Enable replication from the primary, starting from backup GTID/LSN.
4. Verify replica catches up and applies cleanly.

### Handling Replication Lag Spikes

- **Check replica hardware:** CPU, disk I/O saturated?
- **Check network:** Replication channel congestion?
- **Large write burst on primary:** May cause temporary lag (normal).
- **Long-running read query blocking apply:** Kill the query or use replication worker threads (parallel apply).

---

## When NOT to Replicate

- **Very high consistency requirements:** Use synchronous replication (slower) or strong consistency mechanisms (consensus, Paxos).
- **Bandwidth-constrained networks:** Replication stream can overwhelm WAN links. Filter/compress changes.
- **High-frequency schema changes:** Replicas may stumble on DDL, requiring manual intervention.

See also: distributed-replication, cloud-disaster-recovery, database-patterns.