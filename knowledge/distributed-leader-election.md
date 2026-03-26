# Distributed Leader Election — Algorithms, Split-Brain Prevention, and Fencing

## Overview

Leader election solves the problem of designating a single process to coordinate in a distributed system. Without coordination, redundancy is useless; with multiple coordinators, conflicts arise (split-brain). Leader election algorithms ensure at most one leader exists, survive failures, and rebalance when the leader dies. Trade-offs span message complexity, election speed, and correctness guarantees.

## Classical Election Algorithms

### Bully Algorithm

**Mechanism:** When a process detects the current leader has failed (or on startup), it sends an election message to all higher-numbered processes. If any respond "still alive," the initiator abandons. If none respond, the initiator becomes leader and announces itself to all lower-numbered processes.

**Properties:**
- Assumes fixed, known process IDs; higher ID always "outranks" lower.
- Simple to implement but O(n²) messages in worst case (all processes elect simultaneously).
- **Convergence:** New leader emerges quickly once any process initiates.
- **Not network-partition tolerant:** Works within a single partition but requires connectivity.

**Failure modes:**
- If multiple processes have stable checksums or identical IDs, ambiguity arises (fixed by unique IDs).
- Rapid re-elections if the elected leader fails immediately after becoming leader (mitigated by a "grace period").

### Ring Algorithm

**Mechanism:** Processes are arranged in a logical ring. When election starts, a process sends a message containing its own ID around the ring. Each process that receives the message appends its ID if it has a higher ID than the initiator, or passes the message unchanged if lower. When the message returns to the initiator, the highest ID encountered becomes leader.

**Properties:**
- O(n) messages per election (assuming one initiator).
- More balanced behavior than Bully; every process gets equal weight in the outcome.
- **Message ordering:** Relies on reliable unidirectional message delivery around the ring.
- Single initiator is assumed; if multiple initiate, multiple messages circulate (messier but still correct).

**Comparison to Bully:**
- Ring: processes "vote" indirectly; highest ID wins regardless of who initiated.
- Bully: processes "vote" directly; process closest to the highest ID wins via hierarchy.

## Consensus-Based Election (Raft, Paxos)

Classical algorithms assume "best" leader based on ID or hierarchy. Modern systems use **consensus** to ensure a leader is acceptable to a quorum, not just by rank.

### Raft Leader Election

**Mechanism:**
1. Processes (peers) start in **follower** state with a random election timeout (150ms-300ms).
2. If a follower sees no RPCs from the leader for election_timeout milliseconds, it increments its **term** counter and becomes a **candidate**.
3. The candidate votes for itself, sends **RequestVote** RPCs to all other peers, and waits for responses.
4. If the candidate receives votes from a majority (including itself), it becomes **leader** for that term.
5. The leader periodically sends **AppendEntries** RPCs (heartbeats) to all followers. Followers reset their election timeout on receipt.
6. If a candidate receives an RPC from a higher-term leader, it steps down and becomes a follower.

**Properties:**
- **Term-based ordering:** Each term has at most one leader (a term is only advanced by leadership claim or higher-term discovery).
- **Quorum protection:** A leader can only be elected if it has votes from a majority. A majority always overlaps with any other majority (pigeon-hole principle), preventing split-brain.
- **Figure of merit:** Leader persists across multiple terms if it remains responsive; no artificial promotion to higher IDs.
- **Stable leaders:** Raft minimizes re-elections by using long heartbeat intervals and randomized timeouts. Election disturbances from one failure are localized.

**Why Raft works:**
- Term separation ensures a client can detect stale leaders (seen in previous term).
- Majority quorum is mandatory; no split-brain.
- RPCs include the sender's term; higher term always wins (leaders defer to newer terms).

### Paxos Leader Election (via Prepare Phase)

Paxos doesn't explicitly elect a leader; instead, a **ballot number** (like term in Raft) acts as a weak leader indicator. The proposer with the highest ballot that achieves a quorum of acceptors gains the right to propose values in the **accept phase**.

**Properties:**
- More abstract; "leadership" is implicit in ballot ownership.
- Ballots can be abandoned mid-protocol; no process has exclusive authority.
- Acceptable for asynchronous systems but more complex to implement correctly than Raft.

## Operational Patterns: Fencing, Leases, and Split-Brain Prevention

### The Problem: Split-Brain

Imagine a primary database with a standby. They're replicating data. The network partitions. The standby doesn't see the primary, elects itself as the new primary, and starts accepting writes. Minutes later, the network heals. Now two "primaries" exist, and writes have diverged.

**Solutions:**

### Fencing Tokens

The primary acquires a **token** (a monotonically increasing number) from a fencing service (etcd, ZooKeeper) when it becomes leader. Before writing, the primary includes the token. The storage layer verifies the token is the highest seen so far; if not, it rejects the write.

**Mechanism:**
1. Primary A holds token 1, starts writing.
2. Network partitions; primary A is isolated.
3. Primary B (standby) obtains token 2 from the fencing service, becomes leader.
4. Primary A tries to write with token 1; storage rejects (token < 2).
5. Primary B writes with token 2; storage accepts.

**Guarantees:**
- Even if stale primary attempts writes, they're rejected.
- False positives impossible; once token N is issued, no token < N will ever be admitted again.

**Limitations:**
- Fencing service must be highly available and correct.
- Storage backend must track token and enforce ordering (not true for all systems; e.g., dumb S3 buckets don't know about tokens).
- API changes required; clients must pass tokens with requests.

### Leader Leases

A leader acquires a **lease** (valid for a time window, e.g., 10 seconds) from a quorum of nodes. It renews the lease before expiry. The lease prevents another process from becoming leader while the current leader's lease is valid.

**Mechanism:**
1. Primary A acquires a lease valid for 10 seconds.
2. Network partitions; Primary A can't renew the lease (can't reach quorum).
3. After 10 seconds, the lease expires.
4. Primary B (standby) acquires a new lease now available.
5. Two leaders never coexist.

**Guarantees:**
- Fewer false positives than heartbeat timeouts alone (lease is a formal contract, not a heuristic).
- No external fencing service required; the quorum is the enforcement mechanism.

**Limitations:**
- Requires synchronized clocks (or loose bounds; Google Spanner uses TrueTime).
- Clock skew on the primary can cause early expiry and unwanted leadership transfer.
- Lease duration is a trade-off: short (fast failover) vs. long (stable leadership).

### Zookeeper and etcd Leadership

**ZooKeeper:**
- Clients create an **ephemeral node** `/app/leader` to claim leadership.
- A process acquiring the node becomes leader.
- If the process dies (connection closes), the node is automatically deleted.
- Other processes watch the node; when it's deleted, a new election begins.
- Zookeeper's internal quorum ensures only one leader is elected (via Zab protocol, similar to Raft).

**etcd:**
- Leaderless key-value store using Raft consensus.
- When a leader dies, a new leader is elected by the Raft consensus algorithm.
- Clients can read from any replica (stale data possible) or route writes to the leader for strong consistency.
- "Elections" are implicit in Raft term changes; the cluster automatically elects a new leader.

## Comparison and Trade-Offs

| Algorithm        | Message Complexity | Partition Tolerance | Requires Quorum | Split-Brain Safe |
|------------------|--------------------|---------------------|-----------------|------------------|
| Bully            | O(n²)              | No                  | No              | No               |
| Ring             | O(n)               | No                  | No              | No               |
| Raft             | O(n) per term      | No                  | Yes             | Yes              |
| Paxos            | O(n) per ballot    | No                  | Yes             | Yes              |
| ZooKeeper        | Quorum-dependent   | No (quorum needed)  | Yes             | Yes              |
| etcd             | Raft overhead      | No (quorum needed)  | Yes             | Yes              |

**Choosing an algorithm:**
- **Scale:** Ring algorithms are O(n) and scale poorly beyond ~100 processes. Raft/Paxos are optimal from a message perspective.
- **Fault tolerance:** Classical algorithms (Bully, Ring) offer no split-brain protection. Use consensus-based election or external fencing for production systems.
- **Operational complexity:** Bully/Ring are simple but fragile. Raft is easier to reason about than Paxos. Managed services (ZooKeeper, etcd) offload complexity.
- **Partition tolerance:** No leader election algorithm is fully partition-tolerant (CAP theorem). Consensus algorithms trade liveness for safety; a partitioned minority cannot elect a leader.

## Anti-Patterns and Failure Modes

### Preemptive Re-Election

If a process re-elects itself on every heartbeat timeout, the system thrashes. Randomized backoff and heartbeat intervals prevent this. Raft's randomized election timeout (150-300ms) is a key part of stability.

### Cascading Failures

If a leader dies, it should not immediately cause a new leader to be elected until the minority of processes have time to converge on the new leader. Too-fast re-election can cause a flapping cascade. Raft addresses this with random timeouts.

### False Leader Demotion

If the current leader loses contact with a quorum temporarily (but is still running), it should step down to avoid split-brain. Both Raft and Paxos enforce this: a leader that can't replicate to a majority loses the right to claim leadership.

## See Also

- `distributed-consensus.md` — Raft, Paxos, Byzantine consensus
- `distributed-coordination.md` — ZooKeeper, etcd, distributed locks
- `distributed-replication.md` — Leader-follower replication, failover