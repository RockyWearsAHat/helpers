# etcd — Raft Consensus, Watch API & Kubernetes Control Plane Storage

etcd is a distributed key-value store using Raft for consensus. Designed for cluster coordination, configuration, and leader election. Primary use: backing store for Kubernetes control plane. Core feature: the **watch API** (push notifications on key changes) paired with Raft linearizability.

## Problem Space: Coordinated Distributed State

Systems need shared, consistent state across machines:

- **Leader election**: Only one primary at a time (active-passive failover)
- **Cluster membership**: Who's in the cluster? When does a node join/leave?
- **Configuration**: Feature flags, secrets, shared settings
- **Distributed locks**: Mutual exclusion across services

Solutions:
- ZooKeeper (Java, complex Paxos)
- Consul (similar to etcd, more opinionated)
- etcd (simpler, embeds Raft, HTTP API)

etcd designed for **simple deployment** and **strong guarantees** in Kubernetes.

## Core Data Model: Flat Key-Value Store

```
Key-Value pairs:
/kubernetes/pods/default/nginx-pod-1 → { "image": "nginx", "status": "Running" }
/kubernetes/services/default/api    → { "type": "ClusterIP", "port": 8080 }
```

All keys have **versions** (incremented on each modify):

| Key | Revision | Value | ModVersion |
|-----|----------|-------|-----------|
| `/app/config` | 1 | `"old"` | 1 |
| `/app/config` | 2 | `"new"` | 2 |

**MVCC (Multi-Version Concurrency Control):** All versions retained in etcd (with automatic compaction). Non-blocking reads of historical state.

### Hierarchical Keys (by Convention)

No actual hierarchy; keys are flat strings. Clients use `/` as separator and tools infer structure:

```
prefix=/kubernetes/pods/default/ → Lists all pods in "default" namespace
```

etcd treats this as "keys starting with prefix", not a tree. This flat design enables watch on prefixes efficiently.

## Raft Consensus & Linearizability

etcd uses Raft consensus for all writes. Raft guarantees:

1. **Safety**: At most one leader at a time; non-leaders reject writes
2. **Liveness**: Cluster tolerates ⌊N/2⌋ failures (3-node cluster: 1 failure, 5-node: 2 failures)
3. **Linearizability**: All reads/writes appear in a single consistent order

### Write Path (Put)

```
Client → etcd Leader ───────────────────────────────────────
                      ↓ Replicate log entry
                   etcd Follower 1  etcd Follower 2
                      ↓ (persist)      ↓ (persist)
            [Quorum: 2/3 persisted] ← Returns ACK
                      ↓
           Apply to state machine (increment revision)
                      ↓
         Returns: { "revision": 42 }
```

Write blocking until quorum acknowledges. Raft ensures all followers converge on same state.

### Read Path: Linearizable vs Stale Reads

**Linearizable read** (default):

```
Client → Leader: "Read key X at current version"
Leader → Confirms it's still leader (Raft no new entries from majority)
    ↓
  Returns X at current committed revision
```

Guarantees: Read reflects all writes acknowledged before read request.

**Stale read** (local read, no Raft check):

```
Client → Any follower: "Read key X"
Follower → Returns X at last known committed revision (may be lagging)
```

Lower latency, weaker guarantee (may return stale data). Acceptable for non-critical queries.

## Watch API: Push Notifications

etcd's defining feature compared to other KV stores: **Watches**.

Client registers interest in key changes:

```bash
watch /kubernetes/services/default/api
```

etcd pushes notifications as changes occur:

```json
{
  "type": "PUT",
  "key": "/kubernetes/services/default/api",
  "value": { "port": 8081 }
}
```

Client app reacts **immediately** without polling.

### Implementation

1. Client sends WATCH request, opens persistent stream
2. etcd server maintains watch subscriptions
3. On PUT/DELETE:
   - Apply to key-value store
   - Step through all watches
   - Send event to matching subscribers
4. Server buffers events if subscriber slow (ring buffer); drop if full

Watch is not reliable (events after drops are lost); clients must check state on reconnect.

### Watch on Prefixes

```bash
watch /kubernetes/pods/default/
```

Receives events for all keys under that prefix.

Used by Kubernetes:
- Watch all pods: client gets events when pod starts/stops
- Watch all services: load balancer reconfigures on service change

## Lease Mechanism: Automatic Expiry

Keys can be bound to **leases**. Lease has TTL; must be kept alive by client sending heartbeats.

```
etcd Leader
  [Lease ID=123, TTL=60s]
       ↓ (client heartbeat every 30s keeps it alive)
   TTL advances: 60s → 0s
       ↓ (if no heartbeat)
   Lease expires → All keys bound to lease deleted
```

Use cases:

1. **Session state** — Login token expires after 60s inactivity
2. **Ephemeral locks** — If process dies, lock auto-released
3. **Cluster membership** — Node registers with lease; crashes → auto-deregistered

```bash
# Acquire lease (60s TTL)
lease_id=$(etcdctl lease grant 60 | jq .ID)

# Bind key to lease
etcdctl put /app/workers/instance-1 data --lease=$lease_id

# Keep alive
etcdctl lease keep-alive $lease_id  &

# If process dies, shell terminates, keep-alive stops
# → Lease expires → /app/workers/instance-1 deleted
```

## MVCC & Compaction

### MVCC: Historical Versions

All writes create new revisions; old versions kept:

```
Get /app/config, Revision=1 → "old"
Get /app/config, Revision=2 → "new"
```

Useful for:
- Auditing (all values ever set)
- Debugging (watch events show old/new)
- Distributed snapshots (consistent read at revision N)

### Compaction: Reclaiming Space

Without compaction, etcd grows indefinitely (every write adds a revision). Compaction removes old versions; keeps GC'able.

```bash
etcdctl compact 1000   # Remove all revisions < 1000
```

After compaction:
```
Get /app/config, Revision=1 → Error (compacted away)
Get /app/config, Revision=1000 → Success (retained)
```

Trade-off: Smaller etcd size vs. less historical data.

## Cluster Management & Discovery

### Initial Cluster Bootstrap

Starting a 3-node cluster:

```bash
# Node 1
etcd --name=node1 --initial-cluster=node1=http://localhost:2380,node2=http://localhost:2381,node3=http://localhost:2382

# Node 2
etcd --name=node2 --initial-cluster=node1=http://localhost:2380,node2=http://localhost:2381,node3=http://localhost:2382

# Node 3
etcd --name=node3 --initial-cluster=node1=http://localhost:2380,node2=http://localhost:2381,node3=http://localhost:2382
```

Each node knows initial members; performs election → leader emerges.

### Cluster Reconfiguration

Adding/removing nodes dynamically:

```bash
etcdctl member add node4 --peer-urls=http://localhost:2384
```

Cluster rebalances automatically; old quorum no longer needed.

### Kubernetes: etcd as Control Plane Store

Kubernetes uses etcd as the single source of truth:

```
API Server → etcd writes: /kubernetes/pods/, /kubernetes/services/, etc.
   ↓
Kubelet → Watch /kubernetes/pods/my-node/ (deployed on this node)
   ↓
If new pod appears → Kubelet creates container
```

etcd is **the** critical component. If etcd is down, cluster is read-only. etcd failure = cluster halt.

## Performance Tuning

### Write Throughput

etcd writes limited by Raft quorum sync. Typical:
- **Single-node**: 10k writes/sec
- **3-node cluster**: 3-5k writes/sec (quorum overhead)

### Read Throughput

Reads can be distributed to followers (stale) or sent to leader (linearizable):
- **Stale reads**: All nodes (~10k reads/sec per node)
- **Linearizable reads**: Leader only (~5k reads/sec)

### Latency

- **Write latency (p99)**: 50-200ms (Raft quorum is synchronous)
- **Linearizable read latency (p99)**: 10-50ms (leader checks with followers)
- **Stale read latency (p99)**: 1-10ms (no quorum check)

High-frequency updates conflict with Raft's consensus overhead.

## Limitations

1. **Write throughput bounded by consensus** — Always < 10k writes/sec per node
2. **All data in memory** — Scales to hundreds of GB, not multi-TB
3. **No sharding** — Single logical cluster (can federate multiple clusters)
4. **Stale reads unpredictable** — No way to guarantee data freshness without linearizable read
5. **Watch reliability** — No guarantees after disconnect; client must poll

## Use Cases

**Kubernetes cluster state**: API server writes all pod/service/node state to etcd. All components watch changes.

**Distributed coordination**: Services use leases for ephemeral registration; watch for updates to discovered peers.

**Feature flags**: Engineer changes flag in etcd; all services watch, reconfigure without restart.

**Cluster elections**: Service instances acquire lock via lease; winner becomes leader; followers watch for leader failure.

## See Also

- [Distributed Consensus](distributed-consensus.md) — Raft protocol basics
- [Distributed Coordination](distributed-coordination.md) — Leader election, distributed locks
- [Apache Cassandra](database-cassandra.md) — Contrasting replica consistency model