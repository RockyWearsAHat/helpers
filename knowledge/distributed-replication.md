# Distributed Replication — Architectures, Consistency, and Conflict Resolution

## Overview

Replication copies data across multiple machines for fault tolerance and read throughput. Replication introduces latency and consistency challenges: how quickly do replicas converge? Can a user read their own writes? What happens when two nodes write different values? The model (single-leader, multi-leader, leaderless) fundamentally shapes the answers.

## Single-Leader Replication

One leader accepts all writes; followers replicate asynchronously.

### Setup
1. Client writes to leader.
2. Leader writes to its disk, then sends write to followers.
3. Leader acknowledges the write to client (synchronous) or waits for followers (semi-synchronous).
4. Followers apply writes to their logs asynchronously.

### Consistency Properties
- **Reads from leader:** Always see latest writes (monotonic, consistent, strong).
- **Reads from followers:** See only replicated writes (lag behind the leader).
- **Replication lag:** The delay between a write on the leader and its durability on followers. Measured in seconds in poorly provisioned systems, milliseconds in well-tuned systems.

### Durability Trade-offs
| Mode | Latency | Durability | Use |
|------|---------|-----------|-----|
| Asynchronous | Low | Low (leader crash loses writes) | Analytics, non-critical |
| Semi-synchronous | Medium | Medium (at least one follower has it) | Recommended balance |
| Synchronous | High | High (all replicas acknowledge) | Financial systems |

### Failover
If the leader crashes:
1. Detect the crash (timeout-based, heartbeat-based).
2. Elect a new leader from followers (via consensus or configuration).
3. Redirect writes to the new leader.
4. Risk: If the old leader accepted a write but didn't replicate it, the new leader won't have it (durability loss).

### Limitations
- Writes create a bottleneck on the leader.
- Reads scale (add followers for read replicas), but writes do not.
- Single point of failure if the leader is the only write target.

## Multi-Leader Replication

Multiple nodes accept writes and replicate to each other and to followers.

### Setup
Each leader replicates to its followers and to other leaders. Writes from different leaders can race; conflict resolution must be deterministic.

### Conflict Resolution Strategies

**Last-Write-Wins (LWW):** Each write is tagged with a timestamp or version. In a conflict, the write with the higher timestamp wins; lower timestamps are discarded.

- Advantages: Simple, deterministic, no coordination.
- Disadvantages: Data loss (discarded writes are silently dropped). Relies on clock synchronization; clock skew can make older writes win. Often causes user confusion.

**Operational Transforms (OT):** Applied in collaborative editing (Google Docs). Given two concurrent edits, transform one edit's effect relative to the other to preserve user intent.

- Example: User A deletes at position 3, User B inserts at position 5. After transformation, B's insert happens at position 4 (adjusted for A's deletion).
- Advantages: No data loss; intended semantics preserved.
- Disadvantages: Complex to implement; correctness proofs are subtle; requires a centralized server to merge conflicting edits (defeats decentralization).

**Conflict-Free Replicated Data Types (CRDTs):** Data structures designed to commute (order of operations doesn't matter) or converge automatically.

- **Last-Write-Wins Register:** A CRDT value tagged with (timestamp, node_id). Any merge chooses the (timestamp, node_id) tuple that is greatest lexicographically. Deterministic, no coordinator needed.
- **Grow-only Set or Counter:** Values only grow; merging two sets is their union. Merging two counters sums them. No conflicts because operations commute.
- **Sequence CRDTs (e.g., RGA, YATA):** Each edit receives a unique (timestamp, node_id) pair. Conflicts automatically resolve by ordering globally unique IDs.
- Advantages: Decentralized, provably convergent, mathematically elegant.
- Disadvantages: Counterintuitive semantics (deletes can re-appear if not handled carefully). Overhead per element (carries tombstones, metadata).

**Custom Logic:** Application-specific conflict resolution; e.g., "newer order overrides older order." Requires explicit code per data type.

### Replication Topology
- **All-to-all:** Every leader replicates to every other leader. Message overhead: O(n²).
- **Star:** One leader replicates to others; those replicate back. Single chain of command.
- **Linear chain:** A → B → C. Latency scales linearly; chain breaks if any link fails.

### Challenges
- Detecting conflicts across async replication is hard. You don't know if a conflict happened until you see both writes.
- Causality visibility: User A writes X, then writes Y (depending on X). If X and Y reach different leaders out of order, the leader receiving Y first breaks causality.
- Write amplification: A write to any leader replicates to all leaders, then to all followers.

## Leaderless Replication

No single leader. All replicas accept writes directly from clients.

### Quorum Writes and Reads
For N replicas, define:
- **W = write quorum:** Minimum replicas that must acknowledge a write for it to be considered durable.
- **R = read quorum:** Minimum replicas that must respond to a read for it to be considered safe.

**Quorum condition for read-after-write:** If R + W > N, then any read quorum overlaps with any write quorum; the read is guaranteed to see the latest write.

- Example: N=3, W=2, R=2. Then R + W = 4 > 3. A write is durable when 2 of 3 replicas have it. A read queries 2 of 3; at least one has the latest write.
- Trade-off: W to 3 (sync on all replicas) has high latency; W=1 (ack immediately) has low durability.

### Repair Mechanisms
Replicas diverge over time (different subsets of replicas receive different writes). Two repair strategies:

**Read-repair:** On read, if replicas have different values, return the latest and update lagging replicas.
- Fast but repairs only data that is read (inactive data remains stale).

**Anti-entropy (gossip):** Periodically, replicas exchange their states and sync divergent values.
- Slow but repairs all data. Used in Cassandra, DynamoDB, etc.

### Sloppy Quorum
In partitions, strict quorums become unavailable (fewer than W nodes are reachable). Sloppy quorum: accept writes from a temporary node outside the replica set (a "hinted handoff"). When the partition heals, the temporary node syncs the data back to the permanent replicas.

- Enables high availability during partitions.
- Reduces durability: data written to a temporary node might be lost if that node crashes before syncing back.

### Consistency Guarantees
Leaderless replication with R + W > N provides **strong read-after-write consistency** within a client. However, other clients might read stale data until anti-entropy or read-repair catches up. Eventual consistency is the baseline.

## Chain Replication

Replicas form a linear chain: head → middle → ... → tail. Writes are acknowledged at the tail; reads are served by the tail.

### Protocol
1. Write sent to head; propagates linearly to tail.
2. Tail acknowledges write to client only after all nodes have applied it.
3. Reads are served by the tail, always seeing the latest committed writes.

### Advantages
- **Strong consistency:** Reads from the tail always reflect all committed writes.
- **Efficient:** Single chain is simpler than consensus or all-to-all replication.
- **High throughput:** No coordinator; writes pipeline through the chain (if the chain is stable).

### Disadvantages
- Failover is complex: if a middle node fails, the chain breaks; recovery requires detecting the failure and reforming the chain.
- Write latency is proportional to chain length.
- Single point of failure: head (can't accept writes) or tail (can't serve reads).

### Use
Chain replication is used in Azure Storage, Facebook's TAO cache, and some academic systems due to its simplicity and strong consistency.

## Reading Consistency Guarantees

Beyond "strong" vs. "eventual," distributed systems offer fine-grained consistency models for reads:

**Read-Your-Writes:** After a client writes a value, its subsequent reads see that write (even if served by different replicas).

- Implemented by sending the write timestamp with the read, and routing reads to replicas that have that timestamp.
- Example: Write X=1 at time 100. Read X from any replica; reads are routed to replicas with time ≥ 100.

**Monotonic Reads:** A client's reads never go backward. If a read at time 200 returns X=1, a later read cannot return X=0 (an older value).

- Implemented by tracking the highest version seen by a client and routing subsequent reads to replicas with at least that version.

**Consistent Prefix Reads:** If a sequence of writes happens in a certain order, any reader sees them in that order (or not at all).

- Example: A writes X=1, then A writes X=2. Any reader sees X=1 before X=2, never X=2 then X=1.
- Prevented by: Storing related data on different replicas. Writes from process A go to replica 1, reads by process B go to replica 2. B might see write 2 from replica 1 (via gossip) before seeing write 1 from replica 2 (not yet gossipped).

These guarantees are weaker than strong consistency but stronger than eventual, offering a middle ground for many applications.

## Replication Lag

**Replication lag** is the time between a write on the leader and its durability on a follower. Lag can cause:

- **Reading stale data:** A user reads from a follower before the latest write has replicated.
- **Violation of monotonic reads:** A user sees a newer value, then queries another follower and sees an older value (replication lag caused the followers to be out of order).
- **Temporary data loss:** In async replication, if a leader crashes before followers replicate a write, that write is lost.

**Mitigation:**
- Keep lag small by overprovisioning replication bandwidth and tuning replica heartbeats.
- Use sticky routing: route reads from a user to the same replica, ensuring monotonic reads from that user's perspective.
- Use causal consistency: track version vectors and route reads to replicas with sufficient versions.

## Anti-Entropy and Gossip Protocols

Gossip protocols ensure eventual consistency by having replicas exchange state periodically.

**Round-based gossip:** Each round, every node picks a random peer and exchanges state.

- After O(log N) rounds, state propagates to all nodes (exponential spread with high probability).
- Each node does O(1) work per round; total work is O(N log N).

**Conflict resolution:** On merge, apply the conflict resolution rule (e.g., LWW or CRDT merge semantics).

**Trade-offs:**
- Eventually consistent (slow spread in large systems).
- Decentralized (no coordinator).
- Robust to network partitions (nodes continue gossiping); data propagates once partitions heal.
- Network cost: If gossip includes all data, overhead is high. Many systems use Merkle tree syncs: exchange hashes of data ranges and sync only mismatches.

## Practical Replication Choices

| Model | Latency | Durability | Consistency | Conflict Handling | Scale |
|-------|---------|-----------|---|---|---|
| Single-leader | Low (leader) | Medium-high | Strong | N/A | Reads scale |
| Multi-leader | Medium | Medium | Eventual + custom | LWW, OT, CRDT | Write scale |
| Leaderless | Medium-high | High (if W+R>N) | Eventual | LWW, read-repair | Scale all |
| Chain | Medium | High | Strong | N/A | Limited |

## See Also

- [distributed-consensus](distributed-consensus.md) — Replication systems depend on consensus for failover
- [distributed-transactions](distributed-transactions.md) — Coordinating writes across replicas
- [distributed-clocks-ordering](distributed-clocks-ordering.md) — Ordering updates across replicas using logical clocks