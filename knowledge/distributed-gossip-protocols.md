# Gossip Protocols — Epidemic Algorithms, Failure Detection, and Membership

## Overview

Gossip (epidemic) protocols spread information probabilistically through a network, mimicking the way diseases spread through a population. A node randomly selects peers and exchanges information. Each node receives copies of the same information from multiple peers, ensuring eventual consistency across the network despite failures, delays, and partitions. Trade-offs span message overhead, convergence time, and failure detection speed.

## Core Concept: Epidemic Models

### Push Model

A node that learns new information **pushes** it to a randomly selected peer.

**Mechanism:**
1. Node A learns new state (a value, membership, or heartbeat clock).
2. A selects a random peer B and sends the new state.
3. B receives it, updates its state, and may push to a random peer C.

**Properties:**
- **Fast initial spread:** Information propagates exponentially in the first phase.
- **Tail latency:** The last few nodes take a long time to receive information (exponential slowdown as fewer uninformed nodes remain).
- **Message count:** Roughly $O(n \log n)$ messages per piece of information (each node receives ~log n copies on average).

### Pull Model

A node **pulls** information from a randomly selected peer to check for updates.

**Mechanism:**
1. Node A asks a random peer B: "What state do you have?"
2. B replies with its state.
3. A updates if B's state is newer.

**Properties:**
- **Symmetric:** Both nodes update if either holds newer information.
- **Less overhead during quiescence:** If information stops changing, pull converges and stops naturally.
- **Slow initial spread:** Information propagates roughly linearly before all nodes have seen it.

### Push-Pull (Hybrid)

Nodes both push new information and pull periodically to ensure no stale state remains.

**Properties:**
- Combines fast spread (push) with guaranteed convergence (pull).
- Standard in production systems (Cassandra, Consul, Serf).
- Trade-off: Higher message overhead than pure push or pull, but more robust to transient messages.

## SWIM: Scalable Weak-Consistent Infection-Style Membership Protocol

SWIM (Gupta et al., 2002) addresses the main weakness of naive gossip: **failure detection**. It detects and removes dead nodes while spreading membership changes.

### Protocol Overview

**Periodic rounds** (e.g., every 1 second):
1. **Probe phase:** Node A sends a ping to a random peer B.
   - If B responds with an ack within a timeout (e.g., 200ms), A marks B alive.
   - If B doesn't respond, A asks a random peer C to ping B (indirect probe, tolerating transient network delays).
   - If C also fails to get an ack, A marks B as suspected (not dead, but likely offline).

2. **Gossip phase:** A sends a membership update message to a random peer D, including:
   - Newly suspected or dead nodes.
   - Incarnation numbers (timestamps) to prevent stale information from overwriting recent state.

3. **Failover:** If a suspected node receives a message, it can increment its incarnation number to refute the suspicion.

**State transitions:**
```
alive --(suspect on direct failure)--> suspected --(gossip heard by>1 nodes)--> dead
       <--(alive node refutes suspicion)--> alive
```

### Properties

- **O(log n) failures detected per node per round:** Each node probes O(1) others; each dead node is probed by O(log n) nodes on average.
- **Bounded message overhead:** Constant number of messages per node per round, regardless of n (unlike naive flooding).
- **Eventual consistency:** All alive nodes eventually know about node deaths.
- **Weak consistency:** Temporary disagreement on dead nodes is acceptable; the system doesn't require perfect global knowledge.
- **Refutation support:** A suspected node can refute suspicion by sending an alive message with a higher incarnation number.

### SWIM vs. Heartbeat

| Aspect              | Heartbeat                        | SWIM                              |
|---------------------|----------------------------------|------------------------------------|
| Overhead            | O(n) messages per dead node      | O(log n) messages per dead node    |
| Scalability         | Breaks at ~10K nodes             | Works to 100K+ nodes               |
| Detection time      | O(k) rounds, k = heartbeat freq  | O(1) rounds on average             |
| False positives     | Low (only on packet loss)        | Low (indirect probes catch delays) |

## Rumor Spreading and Convergence

### Rumor Mill Model

A node with new information is the **source**. Each node that receives the rumor pushes it to a random peer with probability p (typically 0.5 to 0.9).

**Convergence:** After round j, the number of uninformed nodes drops as $(1-p)^k$ where k is the number of informed nodes. Convergence is **logarithmic in system size**; reaching 99% of nodes requires $O(\log n)$ rounds regardless of n.

### Crumbling Walls Effect

In a push model, the last node to receive information may receive it from very few sources. If that node fails before spreading the rumor, information loss is possible (rare in practice due to high redundancy).

**Mitigation:** Pull phase ensures information persists even if last-mile sources fail.

## Applications

### Cassandra Gossip

Cassandra uses a modified SWIM protocol to maintain a **membership view** (which nodes are alive). Nodes gossip about:
- Heartbeat state (alive vs. suspected).
- Endpoint metadata (schema version, boot ID, IP address).
- System load (for load balancing).

Each node is pinged every 5 seconds; suspected nodes are pinged indirectly. Failure detection is tunable (e.g., suspected after 15 seconds, removed after 30 seconds).

### Consul

Consul implements SWIM-like gossip for cluster membership and health state. Agents gossip status at 200ms intervals. Failed agents are marked down after a few missed gossip rounds. Consul applies **anti-entropy** (full state syncs) periodically to catch up lagging nodes.

### Serf

Serf is an explicit gossip library using SWIM. It spreads events (node joins, leaves, custom user events) through the cluster and detects node failures. Common in distributed configuration and orchestration tools.

## Advanced Topics

### Version Vectors in Gossip

Gossip of mutable state (e.g., key-value updates) requires version tracking to avoid old values overwriting new ones. **Version vectors** (see `distributed-clocks-ordering.md`) attach a per-node timestamp to each state update. Node A ignores an update from B if A's version vector includes a higher timestamp from B.

### Causality Preservation

In eventual consistency systems, gossip ensures state converges, but not necessarily in a causal order. A transaction write "A set x=1, then set y=2" might arrive at a replica out of order. Causal delivery requires vector clocks or explicit dependency tracking in the gossip payload.

### Density and Message Volume

Gossip message overhead scales as $O(n \log n)$ per information piece per round. In large clusters, traffic can be substantial. Techniques to reduce overhead:
- **Fanout reduction:** Push to 2-3 peers instead of random selection.
- **Exponential backoff:** Reduce gossip frequency if the cluster is converged (no new changes).
- **Selective gossip:** Only spread changes that affect local interests (regional gossip, topic-based).

## Comparison: Gossip vs. Centralized Coordination

| Property            | Gossip              | Centralized (ZooKeeper, etcd) |
|---------------------|---------------------|--------------------------------|
| Fault tolerance     | High (any n-1 fail) | Limited (needs quorum)         |
| Latency             | ~log n RTTs         | 1-2 RTTs                       |
| Consistency         | Eventual            | Strong                         |
| Message overhead    | O(n log n)          | O(n) on update                 |
| Partition handling  | Works in all parts  | Minority partition blocked     |
| Operational model   | Peer-to-peer        | Leader-follower               |

**When to use gossip:**
- Eventual consistency acceptable (e.g., membership views, DNS TTL updates).
- Partition tolerance required (esp. wide-area networks).
- High availability valued over strong consistency.

**When to use centralized:**
- Strong consistency required (e.g., metadata for distributed transactions).
- Bounded latency important .
- Simpler operational reasoning preferred.

## Failure Modes and Considerations

### Message Loss

Gossip is resilient to single message loss (redundant delivery from other peers), but repeated message loss on an edge can isolate nodes. Periodic pulls catch these.

### Byzantine Nodes

Naive gossip trusts peers' state directly. A malicious node can spread lies (e.g., "node B is dead" when it isn't). Byzantine gossip protocols (not covered here) require cryptographic signatures or quorum verification.

### Backward Compatibility

Rolling updates where old and new nodes use different gossip formats can fragment the cluster. Gossip protocols should include version negotiation or multi-version support.

## See Also

- `distributed-clocks-ordering.md` — Version vectors, causal delivery
- `distributed-coordination.md` — Centralized coordination vs. gossip trade-offs
- `distributed-consensus.md` — Why consensus is needed despite gossip