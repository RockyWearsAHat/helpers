# Distributed State Machines — Replication, Raft, Paxos, and Reconfiguration

## Overview

A **replicated state machine** is a set of identical state machines (servers) that execute the same sequence of commands, producing identical outputs despite independent failures. This is the foundation for fault-tolerant services: databases (replication), coordination services (Zookeeper), configuration managers, distributed locks. The challenge is ensuring all replicas execute the same commands in the same order despite asynchrony, failures, and network partitions. Consensus protocols (Raft, Paxos) solve this.

## Replicated State Machine Architecture

**Goal:** Multiple servers, each with identical state. Client sends command to any server; it reaches consensus, executes, produces result, returns to client.

### Basic Flow

1. **Client sends command** to a server (the leader in Raft, proposer in Paxos).
2. **Leader replicates** the command to followers/acceptors via append-entries or propose messages.
3. **Majority confirms** (quorum acknowledgment).
4. **Leader commits** the command (safe to execute; won't be lost even if leader crashes).
5. **All replicas execute** the command deterministically in the agreed order.
6. **Result returned** to client after execution.

### Determinism Requirement

All replicas see the same input in the same order → identical execution → identical output. This requires:
- **No system time** (use logical clocks)
- **No randomness** (deterministic algorithms)
- **No concurrency** (single-threaded execution or carefully synchronized)

Violations silently diverge replicas.

## Raft Consensus Algorithm

Raft is the dominant consensus protocol for practical systems (etcd, Consul, CockroachDB). It's more understandable than Paxos, with formal safety proofs.

### Raft Overview

Three roles: **Leader** (receives client requests, manages replication), **Followers** (replicate state), **Candidates** (temporary role during leader election).

### Leader Election

**Problem:** Elect a single leader to coordinate. Solved by **terms**—logical clock numbers that increase monotonically and are the basis for authority.

**Algorithm:**
1. Followers wait for heartbeats from leader. If heartbeat expires (election timeout), become candidate.
2. **Candidate increments term, votes for itself**, sends RequestVote RPCs to peers.
3. Peers respond: grant vote if candidate's term ≥ their term and candidate's log is at least as up-to-date.
4. **Majority votes → elected leader**; informs peers via AppendEntries heartbeat.

**Split votes (no majority):** Random backoff; retry election.

### Log Replication (Normal Operation)

1. **Client sends command to leader.**
2. **Leader appends command to its log,** sends AppendEntries RPC to followers.
3. **Followers append entry to their logs** (not yet executed), acknowledge.
4. **Leader receives majority acknowledgment → commits entry** (safe point; won't revert even if leader crashes).
5. **Leader notifies followers of new commit index.**
6. **All replicas execute committed entries** in order.

### Safety / Correctness Property: Log Matching

**Invariant:** If two logs have entries at the same index with the same term,  entries before that index are identical.

**Mechanism:** Leader only sends AppendEntries if the follower's previous entry matches. Followers reject mismatches. This ensures logs remain consistent.

**On leader crash + restart:** New leader has *at least* the committed suffix of the old log (elected via majority vote), so no committed entry is ever lost.

### Follower/Candidate Crashes

Logs persist on disk. On restart, a follower re-reads its log and continues replication from where it left off. Takes time to catch up but eventually consistent.

### Network Partitions

Minority partition: followers don't hear from leader, start elections, but can't get majority votes. Remain followers; no writes possible.

Majority partition: leader remains; can still commit entries (majority present). Minority partition stalled with outdated data until partition heals.

## Multi-Paxos

Paxos is another consensus algorithm, older (Lamport, 1998) and the theoretical foundation for Raft. Multi-Paxos runs Paxos continuously, electing a leader implicitly after the first consensus.

### Basic Paxos (Single Value)

Three roles: Proposer, Acceptor, Learner.

**Two rounds:**
1. **Prepare phase:** Proposer sends (n, X) to acceptors, asking if they'll accept proposals with number ≥ n.
2. **Accept phase (if majority agrees in prepare):** Proposer sends value V to acceptors; they store (n, V) if n is highest they've seen.

**Safety:** Majority protocol ensures if a value is accepted, it will be the value proposed.

### Multi-Paxos

Run Basic Paxos repeatedly. Elect a designated **proposer (leader)** who only learns its own proposals from previous rounds. Trivial prepare phase if leader stable; most rounds skip to accept.

**Complexity:** Paxos is harder to understand than Raft but theoretically equivalent. Raft won because of understandability; both are in production.

## Viewstamped Replication (VSR)

Earlier protocol (Liskov et al., 1988) similar to Raft: leader (primary), followers (backups), terms (views). Less commonly deployed but foundational for understanding replica coordination.

## ZooKeeper Atomic Broadcast (ZAB)

ZooKeeper's replication protocol for configuration/coordination:

- Simpler than Raft; no client-visible log.
- **Quorum writes:** Leader replicates to followers; waits for quorum ack.
- **Atomic broadcast:** Guarantees all or nothing atomicity for transaction sequences.
- **Follower crash + restart:** Catches up from leader via state transfer.

## Configuration Changes: Adding/Removing Replicas

Initial problem: changing cluster membership (add replica for scalability, remove faulty node) is tricky. Applying old and new configs at different times risks split-brain (two leaders elected from different quorums).

### Raft's Solution: Joint Consensus

Two-phase approach:

1. **C-old,new:** Jointly run new config but require majority from both C-old and C-new. Neither subset can unilaterally act.
2. **C-new:** Once C-old,new committed, execute C-new alone.

**Ensures:** At no point can two leaders be elected from disjoint majorities.

### Simpler Alternative: Majority Quorum

Add replicas one at a time (usually safe). Remove only non-leader replicas, never the leader directly; elect a new leader first.

## Practical Distributed State Machine Examples

| System | Protocol | Use Case | Notes |
|--------|----------|----------|-------|
| **etcd** | Raft | Kubernetes config, service discovery | 3-5 nodes typical; linearizable reads |
| **Consul** | Raft | Service registry, health checks | Stronger consistency than eventual |
| **CockroachDB** | Raft | Distributed SQL | Per-range Raft groups; highly replicated |
| **ZooKeeper** | ZAB | Hadoop, Kafka coordination | Hierarchical configuration store |
| **Chubby** (Google) | Paxos | Internal service discovery, locks | Inspired Zookeeper; proprietary |

## Operational Challenges

1. **Snapshot + log truncation:** Full replay of entire log on follower restart is slow. Systems periodically snapshot state, discard old log entries. Snapshot transfer (rsync-like) faster than replay.

2. **Split-brain prevention:** Ensure minority partition can't elect a leader. Requires stable quorum. Don't reduce cluster to 2 nodes; 3 or 5 typical.

3. **Clock skew:** Some systems use wall-clock timeouts for leader election. Clock skew can cause unnecessary elections. Better: logical clocks (monotonic; immune to NTP shifts).

4. **Linearizable reads:** Reads from followers may be stale. Linearizable read protocols from leader ensure freshness; add latency.

## Advanced Topics: Reconfiguration and Membership Changes

**Coordinated reconfiguration challenge:** Changing cluster membership (add node, remove node, resize quorum) without allowing split-brain requires careful state transitions.

### Single-Node Changes (Simplest)

Add one node at a time to avoid quorum confusion:

1. New node joins as non-voting member; catches up from leader's log snapshot.
2. Once caught up (committed index matches leader), promoted to voting member.
3. Cluster now has N+1 nodes; quorum recalculated.

Removing: Step down current leader first (if being removed), run new election, then remove from config.

### Raft v3 Fast Reconfiguration

First Raft implementations used two-phase "joint consensus" (old + new config majorities simultaneously) to prevent split-brain. Raft v3 (post-2018) improved:

- **Single-step commit:** New configuration committed directly if membership change log entry committed under old majority.
- Faster, less operational complexity.

### ZooKeeper's Approach

ZooKeeper freezes writes during reconfiguration (to prevent split-brain from config inconsistency). All servers must ack new config before writes resume. High latency (seconds) but safety is strict.

## Debugging and Observability

**Key metrics for state machine health:**

- **Replication lag:** (leader.committed_index - follower.committed_index). Alert if > threshold.
- **Election count:** Frequent elections indicate cluster instability (clock skew, network flakes, resource starvation).
- **Snapshot duration:** Long snapshots = slow catch-up after restart.
- **RPC error rates:** Follower→leader grpc failures indicate network problems.
- **Log divergence:** Followers' logs should match leader's (after commit). Divergence = inconsistency bug.

**Testing:** Run chaos experiments: kill nodes, delay messages, introduce clock skew, partition the network. Verify quorum behavior, leadership transition, eventual consistency.

## Trade-offs Summary

| Aspect | Raft | Paxos | ZAB |
|--------|------|-------|-----|
| **Understandability** | High | Low | Medium |
| **Performance** | Good (steady-state) | Similar | Good |
| **Configuration Changes** | Joint consensus / fast reconfiguration | Manual protocol extension | Frozen writes |
| **Adoption** | Widespread (etcd, Consul, etc.) | Theoretical / Google Chubby | Niche (Zk) |
| **Failure tolerance** | F failures = 2F+1 nodes | F failures = 3F+1 nodes | F failures = 2F+1 nodes |
| **Linearizable reads** | Quorum read / lease | Quorum consensus | Leader read |

Raft's simplicity and performance made it the consensus algorithm of choice for modern systems. Paxos's three-phase structure offers theoretical elegance but higher implementation burden.

## See Also

- **distributed-consensus.md** — Consensus impossibility and protocol comparisons
- **distributed-leader-election.md** — Leader election algorithms and fencing
- **distributed-cap-in-practice.md** — Consistency and availability trade-offs
- **architecture-state-machines.md** — Finite state machines in single-process systems