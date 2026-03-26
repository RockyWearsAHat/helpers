# Message Broker Architecture — Partitioning, Consumer Groups, Delivery Semantics & Schema Evolution

## Overview

Message brokers decouple producers from consumers: producers write messages to a broker without knowing who consumes them, and consumers read at their own pace. This asynchronous, persistent model enables scalable distributed systems. Modern message brokers organize messages into **topics** (logical streams), partition topics for parallelism, and coordinate multiple consumers to share work. This note covers the architectural patterns underlying systems like Kafka, RabbitMQ, and Pulsar: partitioning, consumer groups, delivery semantics, message ordering, and the challenges of schema evolution and exactly-once processing.

## Topic Partitioning & Parallelism

### Single-Queue Model (Traditional)

A single queue holds all messages for a topic. Consumers connect and retrieve messages sequentially. Each consumer sees each message exactly once; scales only vertically (larger queue, faster consumer).

**Bottleneck:** The queue becomes a serialization point; producer throughput is limited by queue speed and consumer throughput.

### Partitioned Model (Kafka, Pulsar)

Topic is split into multiple independent partitions, each a separate ordered log. Producers hash a message key to determine which partition receives the message. Every consumer in a consumer group is assigned one or more partitions.

**Example:** Topic "orders" with 3 partitions:
```
Partition 0: order-1, order-4, order-7, ...  (key % 3 == 0)
Partition 1: order-2, order-5, order-8, ...  (key % 3 == 1)
Partition 2: order-3, order-6, order-9, ...  (key % 3 == 2)
```

**Parallelism:**
- 3 consumer instances can run in parallel, each consuming 1 partition
- Producers can write to all 3 partitions in parallel
- Throughput scales linearly with partition count

**Ordering Guarantee:** Messages with the same key always go to the same partition, so they're processed in order. Messages with different keys may be processed out of order (across partitions).

**Scaling:** To increase throughput, add more partitions (and scale consumers to match). To decrease latency (per-message), keep partition count low (fewer consumers compete).

## Consumer Groups & Offset Management

### Consumer Group Pattern

Multiple consumers running the same application (e.g., 3 instances of order-processor) are grouped together under a consumer group. The message broker coordinates which partitions each consumer reads from, balancing the workload.

**Mechanics:**
1. Consumers join the group and announce subscriptions (e.g., topics: "orders")
2. Group coordinator (elected from brokers) triggers **rebalancing**: reassigns partitions to consumers
3. Example: 3 partitions, 1 consumer → consumer-1 handles all 3 partitions
4. Add consumer-2 → rebalance: consumer-1 handles partitions 0-1, consumer-2 handles partition 2
5. Add consumer-3 → rebalance: consumer-1 handles partition 0, consumer-2 partition 1, consumer-3 partition 2

**Rebalancing Mechanism:**
- All consumers in the group pause processing
- Broker revokes current assignments (generators stop)
- Broker assigns new partitions (generators restart at previously committed offsets)
- Processing resumes

**Duration:** Rebalancing typically takes seconds; time increases with group size and topic complexity.

### Offset Management

Each partition is an ordered log. **Offset** is the position: partition-0 message-1, partition-0 message-2, etc. Consumer group tracks the current offset per partition; offset is persistent (usually committed to topic or broker store).

**Mechanics:**
1. Consumer reads message at offset N
2. Consumer processes message
3. Consumer commits offset N+1 (tells broker "I've processed up to N")
4. If consumer crashes and restarts, it resumes from the committed offset N+1

**Commit Strategies:**

**auto-commit:** Broker automatically commits offsets after a time interval (default 5s). Fast, but if consumer crashes after reading but before processing, messages are skipped.

**manual-commit (sync):** Consumer explicitly calls `commit()` after processing. Safe (no message skipping) but slower; commit blocks until broker acknowledges. Throughput is limited by commit latency.

**manual-commit (async):** Consumer calls `commit()` without waiting for ack. Fast but risky; if consumer crashes before offset is committed, messages are reprocessed.

**Unique offset topic:** Brokers store offset metadata in a special internal topic (e.g., `__consumer_offsets`). Replicated and persistent; survives broker failures.

## Delivery Semantics

### At-Most-Once

Message is delivered 0 or 1 time. If consumer crashes after reading but before committing, message is lost.

**Implementation:** Commit offset before processing. Consumer reads message, commits, then processes. Crash after commit means message is skipped.

**Use:** Non-critical data (analytics, duplicates acceptable); high throughput is more important than reliability.

**Example:** Counting clicks on a website; losing occasional clicks is acceptable.

### At-Least-Once

Message is delivered 1 or more times. If consumer crashes, message is reprocessed.

**Implementation:** Process message, then commit. Consumer reads message, processes (persists to database or file), commits. Crash before commit means message is reprocessed.

**Implication:** **Idempotency Required.** If message is reprocessed, results must be identical. Two-phase commit or idempotent operations are required.

**Use:** Order processing, billing, financial transactions; duplicates must be detected and deduplicated.

**Example:** Payment message is processed; consumer crashes before committing. On restart, message is reprocessed, but the payment already exists in the database (idempotent update or duplicate detection).

### Exactly-Once

Message is delivered and processed precisely once. No duplicates, no losses.

**Ideal:** But expensive to implement. Requires:
1. **Idempotent Writes:** Downstream system must be idempotent (or have a deduplication key)
2. **Transactional Processing:** Producer, broker, consumer operations are atomic.

**Implementation (Complex):**

**Approach 1: Idempotent Producer + Consumer Deduplication**
- Producer retries are idempotent (same messageID sent multiple times results in same broker state)
- Consumer stores message ID; skips if seen before
- Works if consumer state is durable (crash-resistant database)

**Approach 2: Kafka Transactions**
Kafka provides transactional reads/writes:
```
begin-transaction
  read offset state
  process message
  write results
  commit offset
end-transaction
```

If any step fails, the entire transaction is rolled back. Atomicity guarantees exactly-once processing.

**Approach 3: Kafka + Transactions + Exactly-Once Semantics (EOS)**

Kafka provides **end-to-end exactly-once semantics:**
- Idempotent producer (no duplicate sends)
- Transactional consumer (atomic write + offset commit)
- Transactional producer (atomic writes if chaining consumers)

Together, they guarantee exactly-once from source to sink.

**Overhead:**
- Transactional writes are slower (extra coordination, logging)
- Complexity increases (coordinator state must be tracked)
- Idempotency has operational overhead (deduplication state)

**When to Use:** Critical financial transactions, inventory systems, strong SLA requirements where duplicates are unacceptable.

**When Not:** Throughput-critical, non-critical data (analytics).

## Message Ordering

### Per-Partition Ordering

Messages within a single partition are **always** ordered. If producer sends msg-1, msg-2 with the same key, both enter partition-0 in order; consumer reads them in order.

**Implementation:** Single producer, single reader; no concurrency within partition.

**Caveat:** Messages with different keys (different partitions) are **not** ordered globally. Processing order depends on partition parallelism and consumer speed.

### Global Ordering

All messages in a topic are processed in the exact order they were produced, regardless of key.

**Trade-Off:** Single partition required. Only one consumer can read (no parallelism). Throughput is severely limited; this is rarely acceptable at scale.

**When Acceptable:** Small, critical workflows where total order is essential (e.g., state machine replicas in consensus systems).

### Offset-Based Ordering

Consumer is responsible for reading offsets sequentially. If consumer reads offset 0, then offset 2 (skipping 1), the broker doesn't care; ordering is preserved within each partition naturally.

**Use:** Out-of-order consumption for reprocessing (e.g., replay topic from specific offset).

## Acknowledgment Modes

### Broker Acknowledgment

Producer sends message; broker responds with ACK when message is persisted to disk and replicated to N in-sync replicas.

**Modes:**
- **acks=0:** No ACK; fire-and-forget. Fastest. Message loss possible.
- **acks=1 (default):** ACK from leader broker. Fast. Message loss if leader crashes before replication completes (rare but possible).
- **acks=all:** ACK only after all in-sync replicas persist. Slowest. Maximum safety.

**Trade-Off:** Throughput vs. durability. Choose based on use case importance.

### Consumer Acknowledgment

Consumer receives message and sends ACK to broker when processing completes. Broker doesn't consider message "delivered" until ACK.

**Acknowledgment Methods:**
- **Auto-ACK:** Broker auto-acks after consumer receives (within configured time). Fast but risky.
- **Manual ACK:** Consumer explicitly acks after processing. Safe but slower.
- **Batch ACK:** Consumer acks every N messages. Balance between safety and throughput.

## Dead Letter Routing

Messages that can't be processed after N retries are sent to a **dead letter queue (DLQ)** for manual inspection.

**Example Workflow:**
```
Producer → Topic (orders) → Consumer
                              ├─ Process successfully → downstream system
                              ├─ Transient error (network timeout) → Retry (up to 3x)
                              └─ Persistent error (invalid format) → DLQ
```

**DLQ Topic:** Separate topic (e.g., `orders-dlq`) where problematic messages accumulate. Observability tools monitor DLQ depth; human operators investigate and requeue or discard.

**Retry Logic:**
- Exponential backoff (retry after 1s, then 10s, then 100s)
- Max retries threshold (default 3)
- DLQ redirect on final failure

**Implementation:** Custom retry logic in consumer, or broker-supported retry (e.g., RabbitMQ queues with TTL).

## Priority Queues

Some brokers support message priorities; higher-priority messages are consumed before lower-priority ones.

**Example:** Urgent orders processed before routine orders.

**Implementation:**
- **Named Queues:** Separate high-priority and low-priority topics; consumer pulls from high-priority first (check high, then low).
- **Broker Priority:** Broker internally orders messages by priority field.

**Limitations:**
- Priority can starve low-priority messages (live-lock)
- Partition ordering may be violated (higher-priority message in different partition arrives later)
- Not universally supported (Kafka doesn't have native priority; RabbitMQ does)

## Message Deduplication

Exactly-once requires deduplication: detecting and skipping duplicate messages.

### Strategies

**Producer Side (Idempotent Sends):**
- Producer assigns unique IDs to messages (e.g., UUID, sequence ID)
- Producer retries with same ID; broker stores seen IDs
- Broker deduplicates: if same ID received twice, returns ACK but doesn't store duplicate

**Consumer Side (Deduplication State):**
- Consumer stores seen message IDs in a durable store (database, cache) with TTL
- When consuming, check if ID was seen; skip if so
- Requires external state (database query overhead)

**Broker Side (Implicit Deduplication):**
- Broker detects duplicate message IDs (via producer's unique ID field) within a time window
- Broker stores, deduplicates, and returns idempotent response

# Schema Evolution in Messages

As systems evolve, message schemas change: old fields removed, new fields added, type changes.

### Challenges

**Backwards Compatibility:** Old consumers should handle messages from new producers (new fields ignored).

**Forwards Compatibility:** New consumers should handle messages from old producers (missing fields have defaults).

**Compatibility Across Versions:** Producers and consumers are updated at different times; N versions of each may be running simultaneously.

### Schema Registry Pattern

Use a schema registry (e.g., Confluent Schema Registry) separate from the broker.

**Mechanics:**
1. Producer registers schema (JSON Schema, Avro, Protobuf)
2. Schema assigned a version ID
3. Producer includes schema ID in message header (not full schema, just ID)
4. Consumer downloads schema via ID, deserializes message
5. Registry enforces compatibility rules (new schema must be compatible with previous)

**Benefits:**
- Schemas are versioned, centralized, and validated
- Backward/forward compatibility checked automatically
- Messages are compact (schema ID, not full schema)

### Avro (Apache)

Language-independent format with built-in schema evolution.

**Schema:**
```json
{
  "type": "record",
  "name": "Order",
  "fields": [
    {"name": "id", "type": "int"},
    {"name": "customer", "type": "string"},
    {"name": "amount", "type": "double"},
    {"name": "timestamp", "type": "long", "default": 0}
  ]
}
```

**Evolution:** Add field with default. Old messages don't have the field; consumers use default. New messages have the field; old consumers ignore it.

### Protobuf (Google)

Binary format, efficient for serialization.

**Evolution:** Field numbers are immutable; new fields get new numbers. Old schema ignores unknown field numbers. New schema uses default for missing fields.

### JSON Schema

Human-readable, web-native.

**Evolution:** More flexible than Avro/Protobuf; allows custom compatibility rules.

## Architecture Patterns

### Fan-Out Pattern

One topic (input stream) feeds multiple consumer groups, each processing independently.

**Example:** "events" topic; one consumer group runs real-time dashboards, another does batch analytics, a third populates data warehouse.

**Benefit:** Decouples consumers; each group can crash/scale without affecting others.

### Log Compaction

Broker retains only the latest message per key, discarding old values. Acts as a distributed cache/state store.

**Example:** Topic "customer-state" with messages:
```
customer-1 → {"name": "Alice", "balance": 100}
customer-2 → {"name": "Bob", "balance": 50}
customer-1 → {"name": "Alice", "balance": 150}  (updated balance)
```

After compaction:
```
customer-1 → {"name": "Alice", "balance": 150}
customer-2 → {"name": "Bob", "balance": 50}
```

**Use:** Event sourcing, CQRS (Command Query Responsibility Segregation), distributed state machines.

### Transactional Outbox

Ensure message is sent if and only if a local database transaction succeeds.

**Pattern:**
```
begin-transaction
  1. Save business data to database
  2. Write message to "outbox" table (same transaction)
commit-transaction

Separate process polls outbox table, sends messages to broker, deletes from outbox.
```

**Guarantee:** If transaction commits, message is guaranteed to be sent (eventually). If transaction rolls back, no message is sent. Avoids dual-write problem (failing to send message if app crashes after transaction commits).

## Broker Comparison

| **Broker** | **Partitioning** | **Ordering** | **Delivery** | **Exactly-Once** | **Priority** | **Setup** |
|---|---|---|---|---|---|---|
| **Kafka** | Yes, hash-based | Per-partition | At-least-once (default) | Yes (transactions) | No | Complex |
| **RabbitMQ** | Via exchanges/queues | Per-queue | Configurable (acks) | Difficult | Yes (native) | Medium |
| **Pulsar** | Yes | Per-partition | At-least-once | Yes (transactions) | Via queues | Complex |
| **Redis Streams** | Not built-in | Per-stream | At-least-once | No | No | Simple |
| **NATS** | Subject-based | Per-subject | At-most-once (default) | No | No | Very Simple |

## See Also

- [Message Queues: Patterns, Delivery Semantics & Architecture](api-message-queues.md) — point-to-point messaging patterns
- [Distributed Messaging Systems](distributed-messaging.md) — Broader messaging architecture context
- [Event-Driven Architecture](architecture-event-driven.md) — Patterns built on message brokers
- [Event Sourcing](architecture-event-sourcing.md) — Message brokers as event logs
- [Distributed Transactions](distributed-transactions.md) — Transactional guarantees in distributed systems