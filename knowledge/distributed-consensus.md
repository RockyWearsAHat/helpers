# Distributed Consensus — Protocols, Impossibility, and Trade-offs

## Overview

Consensus is the problem of getting multiple machines to agree on a single value despite failures. This is the core challenge underlying leader election, state machine replication, and most distributed fault tolerance. Understanding consensus requires knowledge of fault models (crash vs. Byzantine), synchrony assumptions (synchronous vs. asynchronous), and the fundamental impossibility results that constrain what is achievable.

## The FLP Impossibility Result

Fischer, Lynch, and Paterson proved in 1985 that **no deterministic consensus protocol can be guaranteed to terminate in a fully asynchronous system with even one crash failure**. Asynchronous means there are no bounds on message delays or relative process speeds. The result is profoundly depressing because every real network appears asynchronous from a protocol's perspective (you can't distinguish a delayed message from a crashed node).

**The implications:** Real systems escape FLP by adding partial synchrony (eventual bounds on message delays) or randomization (probabilistic protocols). No totally synchronous system exists; systems are partially synchronous—long periods of asynchrony punctuated by brief synchrony windows.

## Paxos Family

Paxos is Leslie Lamport's consensus protocol for crash faults in synchronous/partially synchronous systems. It's notoriously difficult to understand; Lamport's original 1998 paper wrapped the algorithm in historical fiction to obscure its brilliance.

### Basic Paxos (Single Decree)
Paxos elects a proposer, who proposes a value through three roles: **Proposers** (clients), **Acceptors** (storage replicas), and **Learners** (clients receiving results).

**Two rounds:**
1. **Prepare phase:** Proposer sends prepare(n) to acceptors with a proposal number n. Acceptors respond if n is higher than any n they've seen, promising not to accept lower-numbered proposals.
2. **Accept phase:** If a majority replies, proposer sends accept(n, value). Acceptors accept if n ≥ the promised number.

**Safety:** A value is chosen if accepted by a majority. Because majorities overlap, a proposer cannot choose a value different from one already accepted by any majority.

**Liveness:** Paxos can livelock if two proposers keep incrementing proposal numbers (dueling proposers). This is addressed by randomized backoff or external leader election.

### Multi-Paxos (Log Replication)
Most systems use Multi-Paxos: elect a single leader (proposer), which efficiently replicates log entries. The leader proposes entries sequentially without the prepare phase for each entry (optimization). If the leader crashes, failover triggers a new prepare phase to ensure consistency.

**Why Multi-Paxos is practical:** Google Chubby, Apache ZooKeeper, and consensus in many databases use variants. The algorithm is mathematically sound but implementation is complex—subtle bugs in handling leader changes, out-of-order delivery, and failure scenarios are common.

## Raft — Consensus Engineered for Understandability

Raft (2014) reformulates Multi-Paxos emphasizing clarity over novelty. Raft decomposes consensus into three subproblems:

### Leader Election
Time is divided into **terms**. Each term begins with an election. Nodes start as followers; if a follower doesn't hear from the leader within an election timeout, it becomes a candidate and requests votes. Candidates increment the term and request votes from all nodes. A node votes for the first candidate in each term (stores voted-for-term to enforce this). A candidate wins if it gets a majority of votes.

**Safety:** Leaders in a term are unique (a node can vote only once per term). All nodes use randomized timeouts to vary election start times, avoiding perpetual ties.

### Log Replication
The leader appends entries to its log and sends heartbeats (AppendEntries RPCs) to followers. Followers replicate entries and acknowledge. The leader advances the commit index once a majority has replicated an entry. Followers apply committed entries to the state machine.

**The critical invariant:** The leader's log is always a superset of followers' logs in terms of entries and log structure. This is enforced by the `prevLogIndex` and `prevLogTerm` fields in AppendEntries—followers reject entries if the previous entry at the given index doesn't match.

### Safety Guarantees
**Election restriction:** A candidate's log must be at least as up-to-date as any voter's log (compared by last log term, then last log index). This ensures a newly elected leader's log already contains all committed entries.

**Commit safety:** A leader commits entries from previous terms only after committing one from its own term. This prevents the leader from rolling back committed entries if it crashes and a new leader is elected.

### Raft vs Paxos
| Aspect | Raft | Paxos |
|--------|------|-------|
| **Clarity** | Single leader per term, explicit phases | Multiple proposers, implicit roles |
| **Performance** | Steady-state: leader broadcasts entries | Steady-state: same, via Multi-Paxos optimization |
| **Liveness** | Randomized timeouts prevent dueling | Randomized backoff or external resolution |
| **Formalism** | Designed for teachability; proved correct | Mathematically precise; harder to verify in practice |

## Byzantine Fault Tolerance (BFT)

Byzantine faults allow nodes to behave arbitrarily: crash, corrupt data, send conflicting messages. Tolerating Byzantine faults requires more votes than crash faults.

### Quorum Mathematics
- **Crash faults:** Tolerating f faults requires 2f + 1 nodes (majority). A quorum is any f + 1 nodes.
- **Byzantine faults:** Tolerating f Byzantine nodes requires 3f + 1 nodes. A quorum is 2f + 1 nodes. The math: an attacker controls f nodes, so we need to isolate one of the remaining 2f + 1 honest nodes.

### Practical Byzantine Fault Tolerance (PBFT)
Castro and Liskov's PBFT (1999) uses a primary (leader) and a multi-round voting protocol.

**Three phases:**
1. **Pre-prepare:** Primary assigns a sequence number to a client request.
2. **Prepare:** Replicas exchange the request and sequence number.
3. **Commit:** Replicas exchange commit messages.

A replica executes the request after 2f + 1 replicas commit the same request at the same sequence number. If the primary is faulty, a view change (leader election) occurs after a timeout or when f + 1 nodes suspect the primary.

**Limitations:** PBFT has O(n²) message complexity and requires synchronous assumptions for liveness (timeouts). It's used in permissioned blockchains (Hyperledger Fabric) but not at scale without optimization (BFT-SMaRt, Tendermint).

## Viewstamped Replication

Oki and Liskov's Viewstamped Replication (1988) preceded Paxos and influenced its design. Like Raft, it uses a primary; unlike Raft, it uses two-phase commits.

**Views** organize time; each view has a primary. The protocol ensures that if a view change occurs, the new primary's log is up-to-date with all committed entries. Viewstamped Replication guarantees safety and liveness in partially synchronous systems.

It's less well-known than Raft but theoretically sound; TigerBeetle uses it for financial ledger replication.

## CAP Theorem and Consistency Models

Brewer's **CAP theorem** (2000) states a distributed system cannot simultaneously guarantee all three of:

- **Consistency (C):** Every read sees the most recent write (linearizability).
- **Availability (A):** Every request receives a response (no timeouts or errors).
- **Partition tolerance (P):** System continues despite network splits.

**Critical clarification:** P is unavoidable in any distributed system over an unreliable network. The real choice is consistency or availability during a partition.

- **CP systems** (ZooKeeper, etcd, HBase): Reject writes during partitions to maintain consistency.
- **AP systems** (Cassandra, DynamoDB): Serve stale data to remain available during partitions, reconciling later via eventual consistency.

**PACELC extension (Abadi, 2010):** Even with no partition (E = else), latency (L) and consistency (C) conflict. DynamoDB is PA/EL (partitions → available, else → low latency). Spanner is PC/EC (partitions → consistent, else → consistent, accepting latency).

## Linearizability vs Sequential Consistency

Both are strong consistency models but differ in time semantics.

- **Linearizability:** Total order of operations respects real-time order. If operation A finishes before operation B starts (in wall-clock time), A appears before B in the order. Expensive: requires coordination at every operation.

- **Sequential consistency:** Total order of operations respects the order each process issued them, but not real-time order. Two operations from different processes can be reordered if neither observes the other's result. Cheaper: weaker constraint allows more parallelism.

**Example:** An account with balance 100. Thread 1 writes 150, thread 2 reads, then thread 3 writes 200.

- Linearizable: Thread 2 reads 150 (it was ordered after thread 1's write).
- Sequentially consistent: Thread 2 could read 100 or 150 (order between threads 1 and 2 is not enforced by real-time).

Linearizability implies sequential consistency (stronger guarantee). Most practical systems aim for weaker models—pure linearizability is rarely used.

## Consensus in Modern Systems

- **Databases:** TiDB, CockroachDB, YugabyteDB use Raft for replica coordination and distributed strong consistency.
- **Distributed locks:** Consul, etcd use Raft for leader election and stored state.
- **Blockchains (permissioned):** Hyperledger Fabric uses PBFT; many others use Raft variants for sidechain consensus.
- **Message brokers:** Kafka uses a custom leader-based replication (not true consensus, but similar architecture).

## Trade-offs and Limitations

- **Consensus is expensive:** Every write incurs coordination overhead. Raft requires at least one round-trip to a majority before acknowledging a write.
- **Pauses during leader changes:** During failover, the system is unavailable until a new leader is elected. Raft's timeouts control this but add latency variability.
- **Scale limits:** Consensus overhead grows as you add more replicas. Systems often use hierarchical or sharded consensus (Raft per shard, then leader election for the cluster).
- **Byzantine protocols are even more expensive:** PBFT requires multiple rounds; practical deployments limit it to small groups.

## See Also

- [distributed-clocks-ordering](distributed-clocks-ordering.md) — How protocols establish order without centralized time
- [distributed-replication](distributed-replication.md) — Replication strategies that depend on consensus
- [distributed-transactions](distributed-transactions.md) — Coordinate transactions across consensus participants