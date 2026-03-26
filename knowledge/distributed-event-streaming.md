# Event Streaming Platforms — Kafka, Pulsar, Redpanda, and Alternatives

## Overview

Event streaming platforms are distributed publish-subscribe systems optimized for **durability, replay, and scale**. Unlike traditional message queues that discard messages after delivery, event streams persist data, allowing consumers to replay history, catch up after downtime, and support multiple independent subscribers. Kafka dominates but the landscape includes Apache Pulsar, Redpanda, NATS JetStream, AWS Kinesis, and others. The distinguishing factors: partitioning strategy, consistency guarantees, tiered storage, and operational complexity.

## Apache Kafka Architecture

Kafka's core: **brokers** (servers) organize messages in **topics**, split into **partitions** (ordered, immutable logs).

### Partitions and Consumer Groups

- **Partition:** Single ordered log. Messages immutable once written. Replicated across brokers.
- **Consumer Group:** Multiple consumers read from the same topic's partitions. Kafka assigns partitions to consumers; each partition goes to exactly one consumer in the group.
- Parallelism = number of partitions. 10 partitions = up to 10 consumers can read in parallel.

### Exactly-Once Semantics

Kafka provides three guarantees:

1. **At-most-once:** Message sent once; may be lost if broker fails before replication. Fast but risky.
2. **At-least-once (default):** Messages replicated before ack. Consumer may reprocess if it crashes mid-commit. Common.
3. **Exactly-once (Kafka 0.11+):** Transactional writes + idempotent consumer tracking. Producer deduplication via sequence numbers; consumer offset commits atomic with processing.

Exactly-once requires: `enable.idempotence=true`, transactional processing, and state store snapshots (Kafka Streams). Higher latency but suitable for critical workflows (financial transactions, inventory).

### Replication Topology

- **replication.factor=3 (typical):** Write to leader + 2 followers (min.insync.replicas=2 default).
- **In-sync Replicas (ISR):** Replicas caught up with the leader. Acks only from ISR ensure durability.
- **Unclean leader election:** If ISR is empty (all replicas down), Kafka can elect an out-of-sync replica (data loss risk) or go unavailable. Configuration choice in `unclean.leader.election.enable`.

## Apache Pulsar

Pulsar separates **brokers** (temporary) from **BookKeepers** (persistent ledger storage). This decoupling enables features Kafka lacks:

### Tiered Storage and Geo-Replication

- **Tiered storage:** Brokers cache hot data; old data moved to cheap object storage (S3). Consumers can "replay" messages years old without redownloading the cluster.
- **Active-active geo-replication:** Write in region A, appears in region B, C immediately. Concurrent writes handled by unique producer IDs and application-level conflict resolution.

### Topics vs. Subscriptions

Pulsar separates topics (event stream) from subscriptions (consumption position). Multiple subscriptions independently track progress on the same topic.

### Consistency and Ordering

- **Partitioned topics:** Like Kafka—partition-level ordering.
- **Non-partitioned topics:** Messages delivered in order to all consumers (stricter guarantee than Kafka's consumer-group model).

## Redpanda

Redpanda reimplements Kafka's API in **C++** (vs. Kafka's Java/Scala), aiming for lower latency and operational simplicity:

- **No ZooKeeper:** Redpanda uses Raft internally for coordination (vs. Kafka's ZooKeeper dependency).
- **Single binary:** Bundles broker, controller, coordination logic in one process (Kafka splits these after KIP-500).
- **Tuning:** Lower garbage collection pauses (C++ vs. JVM GC), tighter latency percentiles (p99 in sub-milliseconds).
- **API compatibility:** Supports Kafka client libraries and protocols (protocol translation layer).

**Trade-off:** Redpanda's simplicity comes at the cost of ecosystem maturity. Fewer third-party integrations, smaller community debugging resources.

## NATS JetStream

Lightweight event streaming overlay on NATS messaging:

- **Lightweight:** Pure Go, minimal resource footprint. Suitable for edge, embedded, or resource-constrained deployments.
- **Subjects as topics:** Hierarchical subject naming (e.g., `orders.created.us-west`); consumers subscribe via patterns.
- **Consumer model:** Similar to Kafka groups but more flexible (ephemeral vs. durable consumers); simpler configuration.

**Limitations:** Smaller scale (viable up to hundreds of gigabytes on a single server; Kafka scales to terabytes). Smaller ecosystem.

## AWS Kinesis

Managed event streaming (no self-hosted option):

- **Shards (partitions):** Users provision shard count upfront; each shard has provisioned throughput.
- **Managed:** AWS handles replication, upgrades, operational overhead.
- **Cross-region replication:** Async via managed replication. Not active-active; data flows one direction.
- **Pricing:** Per-shard-hour + API calls. Expensive at scale vs. self-hosted Kafka.

**Use case:** Organizations standardizing on AWS; low-ops tolerance; predictable workload.

## Stream Processing: Kafka Streams vs. Apache Flink

### Kafka Streams

- Embedded library (Java/Scala). Applications link Kafka Streams into their runtime.
- **Topology:** DAG of processors (source → filter → aggregate → sink).
- **State stores:** Changelog topics back local state. Rebalancing replays state from changelog.
- **Semantics:** Exactly-once when combined with Kafka transactions.
- **Scaling:** Redistribute processors across instances; Kafka handles rebalancing.

**Pros:** Deployed with application; minimal separate infrastructure.
**Cons:** State stores duplicated per instance (disk usage); debugging distributed topologies is harder.

### Apache Flink

- Standalone cluster (Java/Scala; Python APIs emerging).
- **Dataflow:** Streaming DAG with checkpoint intervals for fault tolerance.
- **Exactly-once:** Built-in via distributed snapshots (two-phase commit with sources/sinks).
- **Stateful processing:** Managed state backend (in-memory, RocksDB for scale).
- **Windowing, joining, complex aggregations:** Native support (vs. Kafka Streams "GlobalKTable" workarounds).

**Pros:** Purpose-built for streaming; richer API; handles complex workflows.
**Cons:** Separate cluster to operate; higher resource overhead; configuration complexity.

## Consistency and Ordering Trade-offs

| Platform     | Ordering    | Exactly-Once | Geo-Replication | Tiered Storage | Operational Burden |
|-------------|-----------|----|----|----|---|
| **Kafka**    | Per-partition | Yes (0.11+) | None (app-side) | No | High (ZK, multiple tools) |
| **Pulsar**   | Per-partition or global | Yes | Active-active | Yes | Moderate (BookKeeper adds complexity) |
| **Redpanda** | Per-partition | Yes (Raft-based) | No | No | Low (single binary) |
| **NATS JetStream** | Subject-based | Yes | No | No | Very low (lightweight) |
| **Kinesis**  | Per-shard | Yes (with Lambda/managed) | One-way async | No | None (fully managed) |

## Kafka Deep Dive: Exactly-Once in Practice

### Idempotent Producer

Kafka producers can set `enable.idempotence=true`. Producer assigns monotonic sequence numbers; broker deduplicates based on **{producer_id, sequence_number}**. If producer crashes and restarts, duplicate sends are detected and dropped server-side.

**Window:** Deduplication window = 5 minutes by default (`transactional.id.expiration.ms`). Sequence numbers stored in memory per partition leader.

### Transactional Writes + Read Committed

For atomic multi-partition writes:

```
producer.beginTransaction()
producer.send(topic1, record1)
producer.send(topic2, record2)
producer.commitTransaction()  // atomic: both or neither
```

On commit, writes become visible to consumers reading with `isolation.level=read_committed`. In-flight (uncommitted) writes hidden.

**Cost:** Additional latency; transaction markers written to each partition's log.

### Consumer-Side Offset Management

Offset tracking determines what message a consumer re-reads after crash :

1. **Automatic offset commit:** Consumer calls `commitSync()` or `commitAsync()` after processing. If crash immediately after fetch but before commit, message re-processed (at-least-once).
2. **Manual offset commit:** Application decides when to commit (after downstream persisting to DB, etc.). More control but complex.
3. **Transactional read:** Consumer offset commit + downstream write in one transaction (via Confluent's connectors). Exactly-once for source→DB patterns.

## Event Streaming Configuration Patterns

### At-Least-Once (Safe Default)

```
producer: acks=all, retries=infinite
consumer: auto.commit=false, process → commitSync() → error handling
```

Suitable for most workloads. Consumer idempotency required (dedup on key or database unique constraints).

### Exactly-Once (High Correctness Cost)

```
producer: enable.idempotence=true, transactional.id="{app-instance-id}"
consumer: isolation.level=read_committed, auto.commit=false
          process → commitSync() in transaction block
```

Latency overhead ~5-10% vs. at-least-once. Requires consistent produce-process-commit cycle.

### At-Most-Once (Risky, Fast)

```
producer: acks=1 or 0
consumer: auto.commit=true (default), process immediately
```

Throughput maximum; data loss acceptable. Examples: analytics pipelines, log aggregation where sampling is okay.

## Practical Positioning

**Choose Kafka if:** Mature ecosystem, large scale (terabytes/day), complex producer/consumer patterns, or existing Kafka investment.

**Choose Pulsar if:** Geo-active-active replication required, replaying ancient data (tiered storage), or requiring non-partitioned ordered delivery.

**Choose Redpanda if:** Operational simplicity (no ZK), lower latency tails, or team prefers C++-native tools.

**Choose NATS JetStream if:** Lightweight deployment (embedded, edge, IoT), subject-based routing, or already in NATS ecosystem.

**Choose Kinesis if:** All-in on AWS, minimal ops tolerance, or predictable cost/scale.

## Common Pitfalls

**Partition imbalance:** Uneven data distribution across partitions leads to hot partitions (throughput capped). Use partitioning key strategically (hash-based, time-based, etc.).

**Consumer lag explosion:** Slow consumer falls behind; under-provisioned in-memory buffers. Monitor lag; scale up consumer replicas or increase partition count.

**Exactly-once misconfiguration:** Forgetting `enable.idempotence=true` or committing offsets before downstream work completes = data loss or duplication.

**Geo-replication latency:** Active-active geo-replication introduces cross-region latency; conflicts on concurrent writes must be resolved (LWW, application-side, etc.).

## Practical Positioning

**Choose Kafka if:** Mature ecosystem, large scale (terabytes/day), complex producer/consumer patterns, or existing Kafka investment.

**Choose Pulsar if:** Geo-active-active replication required, replaying ancient data (tiered storage), or requiring non-partitioned ordered delivery.

**Choose Redpanda if:** Operational simplicity (no ZK), lower latency tails, or team prefers C++-native tools.

**Choose NATS JetStream if:** Lightweight deployment (embedded, edge, IoT), subject-based routing, or already in NATS ecosystem.

**Choose Kinesis if:** All-in on AWS, minimal ops tolerance, or predictable cost/scale.

## See Also

- **architecture-event-driven.md** — Event-driven system patterns
- **architecture-event-sourcing.md** — Event store design and projections
- **patterns-event-driven.md** — Domain events and event notification patterns
- **distributed-data-consistency.md** — Exactly-once semantics and consistency models