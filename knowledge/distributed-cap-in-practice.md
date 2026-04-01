# CAP Theorem in Practice — Consistency, Availability, and Partition Trade-offs

## Overview

The CAP theorem states that a distributed system can guarantee at most two of three properties: **Consistency** (all nodes see the same data), **Availability** (every request receives a response), and **Partition tolerance** (the system continues despite network partitions). In practice, partition tolerance is non-negotiable—network failures happen. The real trade-off is between consistency and availability *during* a partition. This requires understanding consistency models, response guarantees, and how real systems navigate these choices.

## Consistency Models: A Spectrum

**Consistency** isn't binary. Systems offer different guarantees:

### Linearizability (Strong Consistency)

Every read returns the most recent write; all operations appear ordered as if executed serially. High cost: requires quorum acknowledgment before returning writes, and reads may block waiting for consistency. Examples: Spanner, strict quorum-based systems.

- **Latency:** Write latency = min(leader latency, quorum confirmation latency). Read latency = quorum consensus latency.
- **Availability:** Decreases with partition size. A network partition splits the cluster; the minority partition becomes unavailable.
- **Use case:** Financial transactions, distributed locks, strong invariant enforcement.

### Sequential Consistency

Writes are ordered globally but reads can be stale. A client's writes are ordered; clients see a consistent order of all writes, but reads may lag. Cheaper than linearizability.

- Weaker guarantee than linearizability but avoids some quorum overheads.
- Used by Cassandra's consistency level QUORUM (with caveats about read-repair).

### Causal Consistency

Operations causally dependent are seen in order. Write W1 then W2; any process reading W2 also sees W1. Non-causal writes can be concurrent.

- Allows greater availability than linearizability.
- Requires tracking version vectors or similar; complex for clients.
- Examples: Dynamo, Cassandra lightweight-transactions, some replicated databases.

### Eventual Consistency

All writes eventually propagate; no ordering guarantee. Clients observe different states temporarily. Strongest availability, weakest consistency.

- Gossip replication used by Dynamo, Cassandra (default mode).
- Suitable for read-heavy workloads where conflicts are rare (user preferences, caches, counters).

## The PACELC Model

**Trade-off constraint:** CAP says during a Partition, choose A or C. But what about normal operation (no partition)?

Eric Brewer extended this with **PACELC**: 

- **P (Partition):** Choose A (availability) or C (consistency)
- **E (Else, normal operation):** Choose L (latency) or C (consistency)

This captures the latency-consistency trade-off when the network works fine. Systems optimizing for low latency accept stale reads. Systems optimizing for consistency accept latency from quorum coordination.

**Real systems by PACELC positioning:**

| System        | Partition Choice | Normal Operation | Notes                                                   |
|---------------|------------------|-----------------|---------------------------------------------------------|
| **Spanner**   | C                | C+L overhead    | External consistency via TrueTime; partition→unavailable |
| **DynamoDB**  | A                | L (low latency) | Single-region available; cross-region eventual          |
| **Cassandra** | A                | L               | Tunable consistency via quorum, but default is eventual |
| **etcd**      | C                | C+L overhead    | Raft consensus; quorum reads; partition→minority blocks |
| **PostgreSQL** (multi-master replication) | Custom | Conflict resolution | Depends on replication topology            |

## Tunable Consistency

Modern systems avoid binary C/A choices. They expose consistency dials:

### DynamoDB Strong Consistency

```
GetItem with ConsistentRead=true → quorum read (higher latency, always consistent)
GetItem with ConsistentRead=false → any replica read (lower latency, eventually consistent)
```

### Cassandra Consistency Levels

```
consistency_level = ONE     → fast, risky (may read deleted data, stale writes)
consistency_level = QUORUM  → threshold reads/writes (N/2+1 nodes)
consistency_level = ALL     → all replicas (fails if any replica down)
```

Default write/read = QUORUM QUORUM doesn't guarantee read-your-writes; requires tracking.

### Spanner Multi-Region

- Single region: strong consistency, ~10ms read latency
- Multi-region: eventual consistency via propagation, adjustable staleness bounds (e.g., "data is at most 15 seconds old")

## Partition Resolution Strategies

When a partition heals, replicas must converge:

1. **Last-Write-Wins (LWW):** Keep the most-recent write by wall-clock (Cassandra default). Simple but loses concurrent writes without conflict detection.
2. **Multi-value:** Store all conflicting writes; application resolves (CRDTs, merge procedures). Used by Dynamo, Riak.
3. **Quorum consensus:** Restart from the majority partition's state. Minority partition's writes lost. Used by Raft systems (etcd, Consul).
4. **Version vectors:** Track causality; allow safe merging of concurrent writes if there's a happens-before relationship.

## System Positioning Examples

**Google Spanner:**
- Strong consistency across regions via synchronized clocks (TrueTime + GPS/atomic clocks)
- Partition → minority unavailable (C over A)
- Trades partition resilience for consistency guarantee

**Amazon DynamoDB:**
- Eventual consistency by default (A over C during partition)
- Strong consistency option per request (tunable)
- Single-region available; cross-region eventual via propagation
- Multi-master writes (eventual), not quorum

**Apache Cassandra:**
- Availability + low latency (A + L in PACELC)
- Tunable consistency via quorum depth (1 = eventual, N = strong)
- Gossip replication; partitions heal via anti-entropy
- Suitable for write-heavy, read-forgiving workloads

**etcd (Raft-based):**
- Consistency over availability (C over A during partition)
- Strict leader election; minority partition stops
- Used for coordination, configuration, distributed locks
- Predictable strong consistency

## Implications for Application Design

1. **Understand your partition window:** How often do partitions occur in your deployment? Cloud regions, multi-AZ? Rare = pay consistency cost. Frequent = demand availability.

2. **Conflict detection:** If you choose A, conflicts are inevitable. Plan for them: version vectors, CRDTs, application-level resolution, or accept data loss (LWW).

3. **Read-your-writes:** Not automatic in distributed systems. Implement via:
   - Routing reads to the primary replica
   - Causal consistency tokens (client tracks version metadata)
   - Monotonic reads (client pins to a single node)

4. **Test partition behavior:** Single-replica failure != network partition. Partition behavior (quorum splits, majority-minority, cross-AZ isolation) differs fundamentally.

## Common Anti-Patterns

**"Just use strong consistency everywhere":** This ignores latency and availability costs. Global quorum coordination ≈ 100ms+ latency on geographically distributed systems. If your SLA is <10ms p50, not feasible.

**"Ignore CAP entirely":** Networks fail. Plan for it. Test it explicitly (chaos engineering, failure injection).

**"Tunable consistency without understanding it":** Cassandra's `QUORUM` is not linearizable. DynamoDB's strong read costs 2x throughput. Know what you're buying.

## Further Tensions: Consistency in the Wild

### Read-After-Write Inconsistency

Application writes to leader; reads from replica before replication completes. Classic problem in multi-region systems:

```
User writes profile (region A, leader)
Replication lag: 50ms
User reads profile (region B, replica)
→ Sees old profile
```

Workaround: Sticky reads (pin user to region A) or read-your-writes tokens.

### Cascading Failures Under Load

During network partition, clients may hammer minority partition (which can't respond). Circuit breakers at application level prevent cascade. If minority sees "quorum unreachable," stop processing instead of buffering writes that will fail.

### Clock-Based Consistency

Google Spanner addresses CAP with **TrueTime**: hybrid physical + logical clocks synchronized via GPS + atomic clocks. Guarantees external consistency (linearizable across regions). But requires special hardware; not available outside Google Cloud.

Alternative: Bounded staleness (accept "old data" within known bounds), Microsoft's Cosmos DB approach.

## See Also

- **distributed-replication.md** — Replication architectures (single-leader, multi-leader, leaderless)
- **distributed-consensus.md** — Paxos, Raft, and consensus protocols
- **distributed-clocks-ordering.md** — Clock synchronization and temporal ordering
- **distributed-data-consistency.md** — Fine-grained consistency models (causal, session, bounded staleness)