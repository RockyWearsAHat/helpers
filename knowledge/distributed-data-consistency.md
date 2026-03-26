# Distributed Data Consistency — Models, Levels, and Client-Side Helpers

## Overview

**Consistency** specifies what data a client sees after operations. In distributed systems, data is replicated across machines, and failures, network delays, and concurrency make consistency non-trivial. A spectrum of models exists—from strong consistency (global view) to eventual consistency (local views temporarily diverge). Applications choose based on correctness requirements, latency tolerance, and partition resilience. This note covers consistency model taxonomy, database-specific consistency levels, and client-side patterns for weaker models.

## Consistency Model Definitions

### Strong Consistency (Linearizability)

**Definition:** Reads and writes appear to happen instantly and atomically, in a single global order. Every read sees the most recent write globally.

**Implementation:** Quorum reads/writes + leader-based replication. Read goes to leader or a quorum of replicas to confirm recency.

**Cost:** High latency (geographically distributed quorum may take 100ms+). Partition → minority unavailable.

**Use:** Financial balances, critical counters, distributed locks.

### Sequential Consistency

**Definition:** Writes are globally ordered, but reads can lag. All clients see writes in the same order; each client's reads are ordered, but absolute recency isn't guaranteed.

**Simpler than linearizability:** Doesn't require instantaneity; allows for replication lag so long as all replicas eventually see the same write order.

**Example:** Cassandra at QUORUM consistency (not linearizable but sequential-like with monotonic reads enforcement).

### Causal Consistency

**Definition:** If write W1 happens-before write W2 (causally dependent), every client seeing W2 also sees W1. Non-causal operations may be concurrent.

**Mental model:** Order follows dependency chain, not wall-clock time.

**Implementation:** Version vectors or hybrid logical clocks. Track which nodes have seen which writes; don't deliver a write until its dependencies are visible locally.

**Cost:** Moderate overhead; not as expensive as quorum for every operation, but requires metadata tracking.

**Example:** Some replicated key-value stores (Dynamo, Cosmos DB with causal consistency tier).

### Read-Your-Writes (Session Consistency)

**Definition:** A client's own writes are always visible to its own reads. Other clients' writes may not be immediately visible.

**Simpler than causal:** Only guarantees the client's perspective; other clients may see different orders temporarily.

**Implementation:** Client tracks its write version; reads go to replicas with at least that version. Sticky session approach (pin client to a replica).

**Cost:** Low compared to quorum reads.

**Example:** AWS DynamoDB via "consistent read" flag per-client or session tokens.

### Monotonic Reads

**Definition:** If a client reads value X, then later reads the same key, the second read returns X or a more recent value, never an older value.

**Prevents:** Regression to stale data (reading an older version after a newer one).

**Implementation:** Client pins to a replica or caches version metadata; doesn't read from replicas lagging behind its highest-seen version.

**Cost:** Low; mostly client-side tracking.

**Example:** Applications reading user profiles; want to see profile improvements over time, not rollbacks.

### Monotonic Writes

**Definition:** If a client writes W1, then W2, all servers see W1 before W2 (from any client's perspective).

**Prevents:** Out-of-order writes breaking application logic (e.g., password reset sequence).

**Implementation:** Client routes all its writes to the same replica or uses logical timestamps.

**Cost:** Low; mostly routing discipline.

### Bounded Staleness

**Definition:** Reads return data at most Δ time old or k writes behind the leader.

**Options:**
- **Time-bounded (Δ seconds):** Data is at most 100ms old.
- **Version-bounded (k writes):** Data reflects at most 10 uncommitted writes on the leader.

**Implementation:** Replicas lag by known amount; client waits for lag to drop below threshold if necessary.

**Cost:** Predictable; suitable for interactive applications tolerating short staleness windows.

**Example:** Google Spanner multi-region (staleness bounds configurable). DynamoDB global secondary indexes (eventual consistency + bounded staleness options).

### Eventual Consistency

**Definition:** All writes eventually propagate to all replicas; no ordering guarantee about when or relative to other writes.

**Weakest guarantee:** Clients may see conflicting writes (last-write-wins, multi-value, or application-level resolution).

**Implementation:** Gossip replication, one-way replication, deferred sync.

**Cost:** Lowest; no cross-replica coordination.

**Suitable for:** Read-heavy workloads, tolerating temporary divergence (user preferences, counts, caches, social feeds).

## Consistency Levels in Popular Databases

### DynamoDB (AWS)

**Per-request consistency:**
- `ConsistentRead=false` (default): Eventually consistent read from any replica. ~1ms latency, low cost.
- `ConsistentRead=true`: Strongly consistent read from leader. ~10ms latency, higher cost (2 read units vs. 1).

**Multi-item transactions (v2 API):** All-or-nothing atomicity; strongly consistent reads/writes bundled.

**Global tables:** Eventual consistency across regions (async replication).

### Cassandra

**Per-operation consistency level** (read and write independently):

| Level | Replicas Acknowledged | Guarantees |
|-------|-----|-----------|
| ONE | 1 | Fast, risky (may read deleted data mid-repair) |
| QUORUM | ⌈rf/2⌉ + 1 | Majority protocol; more durable |
| ALL | all | Strict but fails if any replica down |
| LOCAL_QUORUM | ⌈rf/2⌉ + 1 in local DC | DC-local majority (lower cross-DC latency) |

**Typical:** write=QUORUM, read=QUORUM (quasi-linearizable if tuned, but not guaranteed).

**Read repair:** Replicas return different data; seen discrepancies heal via background repair.

### PostgreSQL with Replication

**Single-leader mode:**
- **Synchronous replication:** Leader waits for follower disk sync before ack. Strong durability, higher latency.
- **Asynchronous replication:** Leader acks immediately; followers catch up. Eventual consistency on follower reads.

**Multi-master (logical replication cross-DC):** Application must resolve conflicts or use append-only schemes.

### Cosmos DB (Azure)

**Consistency tiers** (tunable per-account):

| Tier | Guarantees | Latency | Availability |
|------|-----------|---------|--------------|
| Strong | Linearizability | High (quorum write + read) | Lower (minority partition stalls) |
| Bounded Staleness | Data ≤ Δ time old or k writes behind | Medium | Higher |
| Session | Read-your-writes + causal client isolation | Low | High |
| Consistent Prefix | Writes seen in order but may lag | Low | High |
| Eventual | No ordering | Lowest | Highest |

## Client-Side Consistency Helpers

When the system provides weak consistency, clients implement patterns:

### Version Tracking (Session Consistency)

```
write(key, value) → server returns version_v1
read(key) → client sends "version ≥ v1"
   → server waits until replica at lease v1, then returns
```

Used by DynamoDB, Cosmos DB session consistency.

### Sticky Sessions

Pin client to a replica. Reads see own writes; other client writes may lag.

**Trade-off:** Simpler than version tracking; limits read distribution if client-replica affinity breaks (failover restarts from different replica).

### Causal Consistency Tokens

Client tracks version vector of all seen (server, version) pairs.

```
write(key, value) → server returns token = {S1: v3, S2: v1, ...}
read(key) → client sends token; server waits until it's seen all those versions
```

**Complex but strong:** Guarantees causal order across reads/writes.

### Read-Your-Writes Pattern

Client stores write version; routes reads to replica with at least that version.

```
write(key, v) → track local_write_version = v
read(key) → find replica.version ≥ local_write_version
```

Suitable for applications dominating their own traffic.

### Retry with Exponential Backoff

For bounded staleness, if read returns stale data:

```
for attempt = 0 to max:
  try:
    data = read(key)
    if data.version ≥ min_required_version:
      return data
  except staleness_error:
    sleep(2^attempt + jitter)
```

**Simple but inefficient:** Adds latency on staleness misses.

## Practical Positioning

| Application | Consistency | Rationale |
|-------------|-------------|-----------|
| **Social feed** | Eventual | Reads dominate; brief lag acceptable |
| **Shopping cart** | Session (read-your-writes) | User needs to see own items immediately |
| **Financial balance** | Strong | Overdraft impossible; critical invariant |
| **Search index** | Bounded staleness (few seconds) | Near-real-time index; acceptable lag |
| **User preferences** | Eventual→Session | Eventual on read; strong write-read sync if critical |
| **Collaborative edit** | Causal | Collaborators must see edits in dependency order |

## Monitoring Consistency Violations

1. **Replication lag:** Track replica max_lsn vs. leader write_lsn; alert if lag > threshold (e.g., >1s).
2. **Read-write conflicts:** Log cases where read returns unexpected version (staleness violation).
3. **Divergence detection:** Periodic replica hash checks; flag mismatches.
4. **Client-side inconsistency:** Instrument write_version tracking; alert on version regression.

## See Also

- **distributed-cap-in-practice.md** — CAP theorem, linearizability vs. sequential, system positioning
- **distributed-replication.md** — Replication architectures and leader-follower patterns
- **distributed-event-streaming.md** — Exactly-once semantics in event platforms
- **distributed-clocks-ordering.md** — Logical clocks for causal ordering