# Distributed SQL — Architecture, Consensus, Transactions, and Clock Sync

Distributed SQL databases (CockroachDB, YugabyteDB, TiDB, Google Spanner) scale horizontally while maintaining SQL semantics and strong consistency. They blend relational database guarantees with distributed systems coordination, trading latency for durability and availability.

## Fundamental Architecture

### Data Partitioning and Replication

Data is partitioned across nodes by a shard key (often a prefix of the primary key). Each partition (range) exists on multiple replicas (typically 3-5 for availability and fault tolerance).

Example: Users table sharded by user_id ranges:
```
Partition [1-1000]:     Replicas: Node A (leader), Node B, Node C
Partition [1001-2000]:  Replicas: Node D (leader), Node E, Node F
Partition [2001-3000]:  Replicas: Node G (leader), Node H, Node I
```

Writes go to the leader; reads can go to any replica (potentially stale). Cross-partition writes require distribution and coordination.

### Raft Consensus Protocol

Most modern distributed SQL systems use **Raft** for replication coordination within a partition.

**Raft guarantees**: If a write is committed (replicated to a quorum, typically N/2 + 1 nodes), it survives failures as long as quorum remains available.

**Phases**:
1. Client sends write to leader
2. Leader appends entry to its WAL (write-ahead log), sends to followers
3. Followers append to their WALs, respond with ACK
4. Leader waits for quorum ACKs, then commits
5. Leader notifies followers that entry is committed

**Properties**: 
- **Strong consistency within a partition**: All replicas reach the same final state
- **Leader election**: If leader fails, remaining cluster elects a new leader (temporary unavailability)
- **Fault tolerance**: Survives failure of up to floor(N/2) nodes

## Distributed Transactions

### Multi-Partition Consistency

Cross-partition writes need distributed transaction protocols. Systems vary:

**Two-Phase Commit (2PC)**: Coordinator phase:
1. **Prepare phase**: Coordinator asks all partitions "can you commit this write?" Each partition locks and prepares
2. **Commit phase**: If all yes, coordinator sends commit to all partitions

Atomic: Either all partitions commit or all abort.

**Cost**: Multiple round-trips, high latency. Tail latency is a problem (slowest partition blocks all others).

**Percolator / BigTable transactional model** (used by Spanner, Tidb):
- Write to a primary partition (coordinator)
- Write to secondary partitions asynchronously, with timestamps
- If secondary writes fail, primary + timestamps allow secondary to commit or abort later

Reduces latency for cross-partition writes.

### Snapshot Isolation and MVCC

Most distributed SQL systems use **Multi-Version Concurrency Control (MVCC)**.

At transaction start, a read timestamp is assigned. The transaction sees all committed data as of that timestamp, ignoring concurrent writes:

```
Transaction A: BEGIN (timestamp 100)
  T=100: SELECT count(*) FROM users; // sees data committed at T < 100
  
Transaction B: (timestamp 101)
  T=101: INSERT INTO users VALUES (...); // committed at T=101
  
Transaction A (still at T=100):
  SELECT count(*) FROM users; // still sees T=100 snapshot
```

**Serializable isolation** (stronger) requires conflict detection; **snapshot isolation** is weaker but faster, avoiding phantom read anomalies in most practical cases.

Distributed SQL systems typically offer snapshot isolation by default, serializable on request (higher contention/latency).

## Time Synchronization and Distributed Clocks

### The Problem: Global Ordering

Distributed systems need to order events globally. Naively, using system clocks from different nodes leads to inconsistencies: Node A's clock might be ahead of Node B's, even if B's event logically happened after A's.

**Linearizability requirement**: External consistency — if client A observes write W1, then later sends message to client B, then client B reads, B must see W1.

### TrueTime (Google Spanner's Solution)

**TrueTime** is a hardware-backed clock synchronization API. Each server has GPS and atomic clocks; uncertainty is explicit.

```
TrueTime: [earliest, latest] time bounds (microsecond-level)
```

Spanner uses TrueTime to assign commit timestamps. A write commits at a timestamp T > its TrueTime.latest at commit time. By waiting for time T to arrive (TrueTime.now() > T), subsequent reads are guaranteed to see the write.

**Consequences**:
- Writes incur commit-wait latency (microseconds to milliseconds)
- Reads never need to wait
- Globally consistent without cross-node message rounds

**Trade-off**: Availability of TrueTime (requires special hardware; not available in most cloud regions).

### Hybrid Logical Clocks (HLC)

**HLC** combines physical clocks (system time) with logical clocks (counters) to achieve ordering without GPS.

```
HLC(A) = max(PT(A), PT(B) + 1, L(A) + 1)
    where PT = physical time, L = logical counter
```

If A sends message to B, B's HLC becomes at least A's HLC. Provides causality without hardware.

**CockroachDB and YugabyteDB use HLC** because they don't assume dedicated time-sync infrastructure.

**Trade-off**: Slightly weaker guarantees than TrueTime (clock skew can still cause issues), but works on commodity hardware.

## Geo-Partitioning and Read Latency

### Regional Data Placement

Distribute partitions across geographic regions:
- US region: Partitions 1-100
- EU region: Partitions 101-200
- Asia region: Partitions 201-300

Writes to local region are fast (local quorum). Cross-region writes incur network latency (50-100ms+).

### Read Consistency Options

1. **Strong consistency (read-your-write)**: Read from primary replica. Latency = round-trip to primary region.
2. **Eventual consistency**: Read from local replica. Fast, but may see stale data.
3. **Bounded staleness**: Read from replicas as of T - 30s. Allows local reads while guaranteeing bounded staleness.

Most systems default to strong consistency; weak consistency is opt-in for specific queries.

## System Architectures

### CockroachDB

- **Design**: Raft-based, HLC clocks, regional awareness
- **K/V layer**: RocksDB per node; Raft replication across zones
- **SQL layer**: Distributed query planner, handles cross-node joins
- **Geo-partitioning**: Explicit zone configuration; reads can be pinned to regions
- **Trade-off**: Write latency for cross-geo transactions (50ms+); design to avoid them

### YugabyteDB

- **Design**: Inspired by Spanner; Raft replication with DocDB (similar to RocksDB) per node
- **Strengths**: Strong compatibility with PostgreSQL, built-in geo-distributed reads
- **Weak consistency reads**: Opt-in per query; enables local reads on replicas
- **Scaling**: Horizontal sharding + replication; tablet-based architecture (similar to BigTable)

### TiDB

- **Design**: Top layer (SQL) separated from storage (TiKV); Raft consensus in TiKV
- **TiFlash**: Column-oriented replica for OLAP workloads; TiDB routes queries intelligently
- **HTAP**: Hybrid transactional/analytical; same data, different storage engines
- **Scaling**: Horizontal; storage and compute scale independently

### Google Spanner

- **Design**: Managed service; TrueTime API, Paxos for replication (not Raft)
- **External consistency**: Guaranteed by TrueTime + commit-wait
- **Global schema**: Multi-region replication of schemas; single logical database
- **Cost**: Pricing based on node-months + replication count; geo-replication increases cost

## SQL Compatibility and Limitations

All four systems aim for PostgreSQL/MySQL compatibility but have gaps:

- **Constraints**: Foreign keys often require single-partition, no cross-partition triggers
- **Window functions**: Supported, but distributed execution adds complexity
- **Subqueries**: Allowed, but can cause full-table scans if not carefully written
- **Transactions**: Cross-partition transactions supported but higher latency; design to single partition when possible

Query planning requires distributed cost statistics; outdated stats cause slow queries.

## Performance Trade-Offs

### Write Amplification

Each write to a partition:
1. Write to leader's log (disk I/O)
2. Replicate to followers (network I/O × 2+)
3. Leader commit log (disk I/O)

Distributed SQL is 2-3x slower than single-node for skewed workloads where all writes go to one partition.

### Hot Spot Problems

If a shard key is poorly chosen (e.g., shard by region, but one region has 90% of traffic), that partition's replicas become bottlenecked. Rebalancing is slow; hot data needs application-level sharding (write to multiple partitions) or schema redesign.

### Tail Latency

Quorum-based replication means write latency = max(replica latencies). One slow node stalls all writes. Replication across multiple cloud regions exacerbates (tail latencies spike on bad network days).

**Mitigations**: Read-your-write from followers (sacrifices immediate consistency), asynchronous replication (sacrifices safety).

## When to Use Distributed SQL

**Good fit**:
- Global applications needing regional data sovereignty
- ACID transactions across multiple entities (e.g., debit account + credit account atomically)
- Workloads with diverse schemas (SQL's flexibility beats fixed proto schemas)

**Poor fit**:
- Extreme write throughput (sharding hotspots are hard to avoid)
- Existing single-node database that doesn't need distribution yet (operational burden outweighs benefit)
- Sparse, append-only workloads (use time-series or log databases)

**Avoid if**: Write latency SLAs are sub-10ms globally; distributed SQL can't achieve that across regions.

## One More Thing: The Spanner Paper

Read the original 2012 Spanner paper for context. Most distributed SQL advancements (since 2019) have been iterations on Spanner's design: better latency (eliminating PaxosUI round-trips), 99th percentile improvements, and multi-cloud. No fundamental breakthrough has replaced Spanner's model.