# Distributed Messaging Systems — Architecture, Ordering, and Semantics

## Overview

Distributed message brokers decouple producers from consumers, enabling asynchronous, scalable communication. Messaging systems trade complexity and operational overhead for resilience and separation of concerns. The choice of broker (Kafka, RabbitMQ, Pulsar, NATS) fundamentally shapes ordering guarantees, throughput, and failure semantics.

## Kafka Architecture

Kafka is a distributed commit log designed for high-throughput, durable, real-time data streaming.

### Core Concepts

**Topics:** Named message streams. Producers write to topics; consumers read from topics.

**Partitions:** A topic is divided into partitions (shards). Each partition is an ordered log on a specific broker. Partitions enable parallelism: consumers process different partitions concurrently.

**Partition Leadership:** Each partition has a leader broker and zero or more replica brokers. The leader handles all reads and writes. Replicas replicate the log asynchronously for fault tolerance.

**Consumer Groups:** A set of consumers sharing a topic's partitions. Kafka assigns partitions to group members such that each partition is read by exactly one consumer in the group (no duplication, automatic load balancing).

**Offsets:** A consumer tracks its position in the log (offset). Offsets are committed (durably stored) to allow resuming after failures.

### Log Compaction

Kafka can retain all messages forever (high disk cost) or compact the log: for each key, retain only the latest message. Useful for state snapshots (e.g., current user profile per user_id, not all profile updates).

### Ordering Guarantees

- **Partition-level:** All messages produced to a partition appear to consumers in the order produced.
- **Topic-level:** No ordering across partitions. Messages from partition 0 and partition 1 can be interleaved in consumption.
- **Key-based guarantee:** If producers always send messages with the same key to the same partition (via hashing), all messages for that key are ordered and consumed in order.

**Replication ordering (ISR - In-Sync Replicas):**
- If `min.insync.replicas=1`, only the leader has the message (fastest, low durability).
- If `min.insync.replicas=2`, the message is replicated to at least one follower before the leader acknowledges (slower, higher durability).

## RabbitMQ Architecture

RabbitMQ is a traditional message broker with complex routing and acknowledgment semantics.

### Core Concepts

**Exchanges:** Publishers send messages to exchanges (not directly to queues). Exchanges have routing rules (type-dependent).

**Exchange Types:**
- **Direct:** Message routed to queues matching the routing key exactly.
- **Topic:** Message routed via wildcard matching (e.g., `user.*.profile` matches `user.123.profile`).
- **Fanout:** Message broadcast to all bound queues.
- **Headers:** Message routed based on headers, not routing keys.

**Queues:** Consumers receive messages from queues (not exchanges). Multiple consumers can share a queue; RabbitMQ distributes messages between them (load balancing).

**Acknowledgments (Acks):** Consumers must acknowledge messages. If unacknowledged, RabbitMQ re-queues and retries. Enables the "at-least-once" guarantee.

### Ordering Guarantees

- **Per-queue:** Messages in a single queue to a single consumer are ordered.
- **Multiple consumers:** If a queue is shared by multiple consumers, ordering is not guaranteed (messages can be distributed out of order).
- **Across queues/exchanges:** No ordering. Different routing rules can cause messages to arrive in arbitrary order.

### Durability

Messages can be marked as "durable," persisted to disk. If the broker crashes, durable messages survive. Transient messages are lost.

## Pulsar Architecture

Apache Pulsar is a modern multi-tenant message broker combining Kafka-like durability with RabbitMQ-like routing flexibility.

### Core Concepts

**Topics:** Named streams, similar to Kafka.

**Partitions (Ledgers):** Each topic partition is stored as multiple ledgers (distributed across a cluster of BookKeepers). Ledgers are immutable and replicated for durability.

**Subscriptions (Consumer Groups):** Multiple subscribers can independently consume a topic. Subscriptions track position separately (unlike Kafka consumer groups, which share a position).

**Multi-Tenancy:** Tenants are isolated namespaces with separate quotas, authentication, and authorization.

### Tiered Storage

Pulsar can offload old ledgers to cheaper storage (S3, GCS, HDFS) while keeping recent ledgers in a fast message broker. Enables long retention at low cost.

### Ordering Guarantees

- **Partition-level (per-ledger):** Messages are ordered per partition.
- **Subscriptions share the partition:** Unlike Kafka groups that round-robin partitions, Pulsar subscriptions can all read all partitions. Ordering is per subscription for ordered consumption type.

## NATS Architecture

NATS (from Synadia) is a lightweight, distributed messaging infrastructure focused on simplicity and latency.

### Core Concepts

**Subjects:** Topic-like names with hierarchical structure (e.g., `orders.created.west`, `orders.created.east`).

**Publish-Subscribe:** Publishers send messages to subjects; subscribers listen. A subject's message goes to all active subscribers (fanout by default).

**Request-Reply:** A subscriber can reply to a publisher's request. NATS correlation and routing handle the reply path automatically.

**JetStream (durable streaming):** An add-on providing durability, consumers, and consumer groups similar to Kafka. Introduced for stateful streaming workloads.

### Ordering Guarantees

- **No guaranteed ordering** in core pub-sub. Messages can be lost if no subscribers are connected.
- **JetStream:** Per-stream (partition) ordering with consumer groups and offset management similar to Kafka.

## Messaging Guarantees

### Delivery Semantics

**At-Most-Once:** A message is delivered 0 or 1 times. If the producer doesn't see an ack, it doesn't retry; the message may be lost. Fast but risky.

**At-Least-Once:** A message is delivered 1 or more times. If the producer doesn't see an ack, it retries. Risks duplicates if the broker crashes after applying but before ack'ing.

**Exactly-Once:** A message is delivered exactly once. Combines at-least-once delivery with idempotence: the broker or consumer deduplicates retries.

### Exactly-Once Implementation (e.g., Kafka 0.11+)

1. **Idempotent Producers:** Producer includes a sequence number with each message. If a producer retries, the sequence number is the same. The broker deduplicates (rejects duplicates with the same sequence).
2. **Transactional Messages:** Producer atomically writes to multiple partitions within a transaction. All or nothing commitment.
3. **Consumer Offsets:** Consumer offset commits are transactional. If a consumer processes a message and commits its offset atomically, a crash before commit replays the message (at-least-once), and a crash after commit ensures the message isn't replayed.

### Write Semantics and Ordering

**Producer Ordering:** If a producer sends messages sequentially, does the broker guarantee they arrive in order to the consumer?
- Kafka: Yes, within a partition (if producer sends to the same partition).
- RabbitMQ: Yes, within a queue.
- Pulsar: Yes, within a partition.
- NATS: No (unless using ordered consumer type in JetStream).

## High-Level Message Flow

```
Producer -> Broker -> Partition/Queue -> Consumer
  (key)     (route)   (persist, replicate) (ack/offset)
```

Producers choose a topic and (optionally) a partition key. The broker routes the message, persists it, and replicates it. Consumers read, process, and ack. Acks and offsets enable resumption and exactly-once semantics.

## Comparison Summary

| System | Throughput | Ordering | Durability | Routing | Use Case |
|--------|-----------|----------|-----------|---------|----------|
| Kafka | High (1M msgs/s) | Partition-level | High (replicated log) | Simple (topic + partition key) | Event streaming, data pipelines |
| RabbitMQ | Medium (100K msgs/s) | Queue-level | Medium (configurable) | Complex (exchanges, topic patterns) | Task queues, pub-sub with flexibility |
| Pulsar | High (1M msgs/s) | Partition-level | High (tiered storage) | Moderate (topics + subscriptions) | Multi-tenant cloud streaming |
| NATS | Very High (5M msgs/s) | None (pub-sub) / Per-stream (JetStream) | Low (in-memory) / High (JetStream) | Simple (subjects, hierarchical) | Low-latency, microservices, edge |

## See Also

- [distributed-consensus.md](distributed-consensus.md) — consensus for broker leader election
- [distributed-transactionssystemdesign.md](distributed-transactions.md) — transactional semantics across messages
- [architecture-event-driven.md](architecture-event-driven.md) — event-driven architecture patterns
- [database-kafka.md](database-kafka.md) — deep dive into Kafka concepts and operations