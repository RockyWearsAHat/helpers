# Distributed Coordination — Services, Protocols, and Patterns

## Overview

Distributed coordination services solve the problem of multiple machines agreeing on state and configuration in the presence of failures. They enable leader election, distributed locking, configuration management, and service discovery. Coordination services trade throughput for strong consistency: writes are sequentially consistent or linearizable, making them suitable for metadata rather than high-volume data.

## ZooKeeper

ZooKeeper (from Hadoop) is the foundational distributed coordination service. It models state as a tree of znodes (similar to filesystems) and provides watches for change notifications.

### Core Abstractions

**Znodes:** Nodes in the ZooKeeper tree (e.g., `/app/config`, `/services/db-primary`). Each znode holds data (a byte string up to 1MB) and metadata (version, creation time, ACLs).

**Znode Types:**
- **Persistent:** Remains until explicitly deleted.
- **Ephemeral:** Deleted automatically when the session ends (client crashes or network partition). Useful for leader election and service discovery.
- **Sequential:** Name includes an auto-incrementing counter (e.g., `/queue/job-0000001`). Useful for distributed queues and locks.

**Watches:** Clients set one-time triggers on a znode. If the znode changes or children change, the client receives a notification. Watches are critical for building responsive coordination patterns.

### ZAB Protocol (ZooKeeper Atomic Broadcast)

ZooKeeper replicates state using the ZAB consensus protocol, a variant of Paxos.

**Phases:**
1. **Discovery:** A new leader is discovered; followers sync with the leader's state.
2. **Sync:** Followers apply all leader transactions to catch up.
3. **Ready:** Followers are ready to accept reads and writes.

**Ordering Guarantees:**
- All writes go to the leader; the leader assigns increasing transaction IDs.
- Followers apply writes in order.
- Reads from a follower see a prefix of the write history (may lag behind the leader, but monotonic for a client).

**Fault Tolerance:** Tolerates $f$ failures with $2f + 1$ servers (quorum-based).

### Recipes (Common Patterns)

**Distributed Lock (shared, non-exclusive):**
1. Client creates an ephemeral sequential znode (e.g., `/lock/client-0000001`).
2. Client lists all znodes under `/lock` and watches the znode with the next-lower sequence number.
3. If the watched znode is deleted, the client re-checks if it's now the lowest; if so, the lock is acquired.
4. On process crash, the ephemeral znode is automatically deleted, releasing the lock.

**Leader Election:**
1. Multiple candidates create ephemeral znodes in a designated directory.
2. The candidate with the lowest sequence number is the leader.
3. Other candidates watch the leader's znode.
4. If the leader crashes, its znode is deleted; the next candidate in sequence becomes the leader.

**Barrier (synchronization point):**
1. All participants create persistent znodes under `/barrier`.
2. Participants wait for N children under `/barrier`.
3. When N children are present, the barrier is satisfied; all participants proceed.

### Limitations

- **Write throughput:** All writes serialize through the leader, limiting throughput (typically 1000s of operations/second).
- **Network sensitivity:** Extended network partitions can lead to unavailability during leader re-election.
- **Manual operation:** Rebalancing and cluster topology changes require manual intervention or external tools.

## etcd

etcd (from CoreOS) is a modern distributed key-value store designed for configuration management and service discovery in Kubernetes.

### Core Abstractions

**Key-Value Pairs:** Keys are strings; values are arbitrary byte strings. Keys are hierarchical (e.g., `/kubernetes.io/services/default/web-api`).

**Leases:** A lease grants a TTL to a key. When the lease expires, the key is deleted. Clients can renew leases with periodic heartbeats (similar to ephemeral znodes in ZooKeeper).

**Watches:** Clients watch keys or prefixes. Changes trigger notifications.

**Transactions:** Multi-key operations (compare-and-set): if conditions hold, execute a transaction; otherwise, execute an alternative.

### Raft Consensus

etcd uses the Raft consensus protocol, chosen for simplicity and understandability compared to Paxos.

**Raft Safety:**
- A leader is elected via majority vote.
- All writes go to the leader, which replicates to followers.
- Followers ack the write; the leader waits for a quorum.
- Committed entries are applied to the state machine.
- Reads can be served by the leader (always consistent) or by followers (if linearized by checking with the leader).

**Performance:** etcd achieves strong consistency with good write throughput (10,000s of operations/second in production clusters).

### Kubernetes Integration

etcd is the data store for Kubernetes. API objects (pods, services, deployments) are stored as key-value pairs. Watches on these objects trigger controller reconciliation loops.

## Consul

Consul (from HashiCorp) is a service mesh and coordination platform combining service discovery, health checking, and configuration management.

### Key Components

**Service Discovery:** Services register with Consul; clients query Consul to find service instances. Health checks ensure only healthy instances are returned.

**Distributed State (Key-Value Store):** Similar to etcd, Consul provides a replicated key-value store (used for configuration management).

**Gossip Protocol:** Services communicate with Consul agents via HTTP; agents gossip with each other to propagate updates. Decentralized (no central coordinator required, unlike ZooKeeper/etcd).

**Consensus:** Consul uses Raft for leader election and consistent state replication.

### Service Mesh (Envoy Integration)

Consul integrates with Envoy proxies to provide service-to-service mesh communication:
- Automatic proxy injection.
- Traffic control (routing, load balancing, retries).
- mTLS enforcement.

## Coordination Patterns and Trade-offs

### Leader Election

**Problem:** Multiple servers compete; only one should act as leader at a time.

**Approaches:**
1. **Lease-based (etcd, Consul):** Each candidate creates a lease with a short TTL. The holder of a lease is the leader. To remain leader, the candidate renews the lease periodically.
2. **Sequence-based (ZooKeeper):** Candidates create sequential znodes; the one with the lowest sequence is leader.

**Consensus guarantees:** If the leader crashes or network-partitions, a new leader is elected. Election takes $O(\text{heartbeat timeout})$ time.

### Distributed Locks

**Problem:** Multiple processes need exclusive access to a resource (e.g., a file, a database transaction).

**Approaches:**
1. **Lock server:** A centralized server holds the lock; clients request/release. Simple but creates a single point of failure (needs redundancy).
2. **Quorum-based:** Clients request acknowledgment from a majority of servers. If a quorum acks, the lock is acquired. Tolerates minority failures.
3. **Mutual exclusion (Bakery algorithm, Lamport):** Clients use logical clocks to order requests; the client with the lowest clock value acquires the lock. No centralized server, but requires communication with all servers.

**Deadlock risk:** If a lock holder crashes without releasing, the lock is held forever. Mitigation: leases with auto-expiration (TTL).

### Configuration Management

**Problem:** Multiple services need the same configuration; when it changes, all services must be updated.

**Approaches:**
1. **Polling:** Services periodically query the coordination service for updated config.
2. **Watches:** Services set watches on config keys. Changes trigger notifications, enabling reactive updates.
3. **Broadcast:** The coordination service pushes changes to all subscribers (Consul's gossip).

Watch-based approaches are more efficient (reactive) but require clients to handle watch resets (if the connection drops, watches are lost).

## Consistency Models

### Strong vs. Eventual Consistency

**Strong Consistency (ZooKeeper, etcd, Consul):** All reads reflect the most recent write. Achieved via consensus; trades write throughput for consistency.

**Eventual Consistency (Cassandra, DynamoDB):** Reads may see stale data. Writes replicate asynchronously; replicas converge over time. Higher throughput, lower latency, but complex application logic.

For coordination metadata, strong consistency is typically required (leader must be globally visible immediately). For application data at scale, eventual consistency is often acceptable.

### Linearizability

A system is linearizable if every operation appears to take effect at a single instant between its invocation and response. All reads see all preceding writes.

ZooKeeper, etcd, and Consul support linearizable reads, though followers may require coordination with the leader to guarantee serialization.

## Comparison Summary

| System | Throughput | Consistency | Fault Tolerance | Primary Use |
|--------|-----------|-------------|-----------------|------------|
| ZooKeeper | Low (1K ops/s) | Strong (via ZAB) | $f$ failures in $2f+1$ servers | Hadoop, Kafka metadata |
| etcd | High (10K ops/s) | Strong (via Raft) | $f$ failures in $2f+1$ servers | Kubernetes, cloud-native |
| Consul | Medium (5K ops/s) | Strong (via Raft) + eventually consistent gossip | Decentralized gossip + quorum Raft | Service mesh, service discovery |

## Pitfalls

1. **Lease Management:** If clients forget to renew leases, keys expire unexpectedly. Leases should be set to $\text{network latency} \times 2$ to avoid false expirations.
2. **Watch Semantics:** Watches are one-time events. If a client misses a notification (crash or backlog), it must re-sync manually.
3. **Split-brain Prevention:** Network partitions can isolate a partition-holder from the quorum. The quorum continues (preventing split-brain); the isolated partition halts. This requires $2f+1$ servers to tolerate $f$ failures.
4. **Operational Burden:** Scaling a coordination cluster is complex and disruptive. Typically, clusters are sized for peak load upfront.

## See Also

- [distributed-consensus.md](distributed-consensus.md) — Raft, Paxos, consensus protocols
- [distributed-replication.md](distributed-replication.md) — replication strategies underpinning coordination
- [system-design-distributed.md](system-design-distributed.md) — integrating coordination into system design