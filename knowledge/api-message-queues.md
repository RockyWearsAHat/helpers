# Message Queues: Patterns, Delivery Semantics & Architecture

## Overview

Message queues decouple producers from consumers via durable, asynchronous messaging. This note covers point-to-point vs. pub/sub topologies, delivery semantics (exactly-once is impossibly hard), message ordering, deduplication, dead letter queues, competing consumers, priority queues, and architecture comparisons (RabbitMQ, Kafka, AWS SQS, NATS).

## Fundamentals

### Queue vs. Pub/Sub

#### Point-to-Point (Queue)

A producer sends a message; exactly one consumer receives it. After consumption, the message is removed.

```
Producer → Queue → Consumer A (receives, processes, removes)
           (message gone)
```

Used for: Tasks (email, image processing), job distribution, work balancing.

#### Pub/Sub (Topic)

A producer publishes a message; multiple subscribers receive independent copies. Subscription model determines lifetime.

```
Producer → Topic → Subscriber A (receives)
                → Subscriber B (receives)
                → Subscriber C (receives)
```

Used for: Events (user registered, order shipped), logging, metrics, fan-out.

**Hybrid systems** (Kafka) blur this distinction: Topics act as persistent logs; consumers pull independently.

## Delivery Semantics

### At-Most-Once

A message is delivered 0 or 1 times. If the broker fails after sending but before acknowledging the producer, the message is lost.

```
Producer sends → Broker receives → Broker sends to Consumer → Consumer lost
(no ack back to producer, broker crashes)
→ Producer assumes success, message gone forever
```

- Fastest (no durability cost)
- **Dangerous**: Silent data loss
- Rarely justified for production systems

### At-Least-Once

A message is delivered 1+ times. Producers and consumers use acknowledgments; on failure, the broker retransmits.

```
Producer sends → Broker persists → Broker sends to Consumer → Consumer processes
  (waits for ack)  (durable)         (waits for ack)          → Sends ACK
    ← ack ←────────────────────────────────────────────────── ack
```

If Consumer crashes before ACK: broker retransmits the same message later (duplicate delivery possible).

- **Most common** in production
- Requires consumer idempotency (handle duplicates)

### Exactly-Once (Impossible Without Constraints)

True exactly-once delivery in a distributed system is impossible (see: Tyler Treat's paper). The fundamental problem:

1. If the consumer ACKs, crashes before persisting: broker delivers again (duplicate)
2. If the broker doesn't wait for consumer ACK: consumer may crash, losing the message (loss)
3. Round-trip coordination introduces more failure modes: timing, partitions, etc.

**Practical exactly-once** requires constraints outside the message queue:

1. **Idempotent operations**: Consumer side-effects are idempotent (e.g., upsert instead of insert)
2. **Transactional databases**: Consumer writes and ACK in a single transaction
3. **Deduplication**: Consumer tracks message IDs; ignores duplicates

Example:
```javascript
function processMessage(msg) {
  const db = new Transaction();
  
  // Atomically: write state AND acknowledge
  db.upsert(msg.id, msg.data);  // Idempotent: same result if called twice
  db.ack(msg.id);
  db.commit();
  
  // If crash between ACK and commit: next attempt re-processes same message
  // But upsert is idempotent, so correct result
}
```

This is called **effectively exactly-once**.

## Message Ordering

### Guarantees by Queue Type

#### Kafka

- **Per-partition ordering**: Messages in the same partition maintain order, but different partitions may be out of order
- **Global ordering**: Only if using a single partition (kills scalability)

```
Topic: orders
  Partition 0: [order-1, order-5, order-9]      (for customer A)
  Partition 1: [order-2, order-6, order-10]     (for customer B)
  Partition 2: [order-3, order-7, order-11]     (for customer C)
```

Order is guaranteed within partitions (1→5→9 for customer A), but partition 0 events may process before partition 1.

**Pattern**: Use the business key (customer_id) as the partition key:
```
Partition key = customer_id → All events for a customer route to the same partition
```

#### RabbitMQ

- No ordering guarantee by default (messages are distributed to multiple consumers)
- **Ordering with single consumer per queue**: If one consumer, messages process sequentially

```
Producer → Queue message-1 → Consumer A processes in order: 1, 2, 3, 4
          message-2
          message-3
          message-4
```

Add a second consumer → no ordering guarantee (messages dispatched round-robin).

#### AWS SQS

- **Standard queues**: No ordering guarantee
- **FIFO queues**: Strict FIFO within a message group (but one consumer per group)

```
Queue: orders.fifo
  MessageGroupId: "customer-123" → [order-1, order-2, order-3] (FIFO)
  MessageGroupId: "customer-456" → [order-4, order-5]          (FIFO)
```

### Out-of-Order Delivery

Most systems accept out-of-order arrivals and handle causally:

```javascript
function processEvent(event) {
  if (event.type === "payment") && !hasOrder(event.orderId) {
    // Order hasn't arrived yet; queue for later
    eventBuffer.defer(event);
    return;
  }
  
  // Process
  apply(event);
}
```

## Message Deduplication

Detecting and handling duplicate messages:

### Deduplication Cache

Consumer caches message IDs; ignores seen messages:

```javascript
const dedup = new Set();  // or Redis, DB

function handleMessage(msg) {
  if (dedup.has(msg.id)) {
    return;  // Already processed
  }
  
  processMessage(msg);
  dedup.add(msg.id);
  
  // May need to clean old IDs from cache (TTL)
}
```

**Trade-off**: Cache size vs. retention period (keep IDs for how long?).

### Idempotent Operations

Don't track; just make all side effects idempotent:

```javascript
function processPayment(msg) {
  // Upsert: if the payment already exists, same result
  db.upsert(
    { paymentId: msg.paymentId },
    { accountId: msg.accountId, amount: msg.amount }
  );
}

// Called twice with same msg → DB has one payment, correctly recorded
```

Idempotent operations include: upserts (insert or update), PUT (replace), DELETE (if resource is already gone, OK).

**Non-idempotent**: INSERT (creates duplicate rows), increments, appends.

## Dead Letter Queues (DLQ)

After N retries fail, move the message to a DLQ for manual inspection:

```
Message → Queue → Consumer attempts to process
                    ↓ (fails)
                    Retry (requeue)
                    ↓ (fails N times)
                    → DLQ (dead letter queue)
```

Consumer code examines DLQ messages, classifies errors:

- **Transient**: Retry later (e.g., database temporarily down)
- **Permanent**: Fix code, replay (e.g., schema mismatch)
- **Unknown**: Alert engineer

Example flow:
```javascript
function processMessage(msg) {
  try {
    // Process
    result = apiCall(msg.data);
    ack();
  } catch (error) {
    if (isTransient(error)) {
      nack(requeue=true);  // Retry later
    } else if (isPermanent(error)) {
      sendToDLQ(msg);      // Manual intervention
    }
  }
}
```

## Competing Consumers

Multiple consumer instances competing for messages from a queue:

```
Queue: tasks
  Task 1
  Task 2
  Task 3
  Task 4

  Consumer A (processes Task 1)
  Consumer B (processes Task 2)
  Consumer C (processes Task 3)
  Consumer D (processes Task 4)
```

**Benefits**: Parallelism, load distribution, automatic scale-out.

**Challenges**:

1. **Duplicate processing**: If Consumer A crashes mid-message, task is requeued; Consumer B picks it up (already half-done)
   - Solution: Make processing idempotent
2. **Ordering**: No guarantee which consumer gets which task
   - Solution: Use partition keys (Kafka) or message group IDs (SQS FIFO)
3. **Backpressure**: Too many consumers / slow consumption
   - Solution: Prefetch limits (consumer pulls N messages at a time, stops at N)

## Priority Queues

Process high-priority messages before low-priority:

```
Queue: notifications
  Priority 1 (high): [urgent-alert-1, urgent-alert-2]
  Priority 2 (normal): [email-1, email-2, email-3]
  Priority 3 (low): [daily-digest, weekly-report]
```

**Implementation**:
- Multiple queues (separate queue per priority, consumer polls in order: high → normal → low)
- Priority field in message (consumer sorts or broker handles)

**Trade-off**: Starvation risk (low-priority messages never processed if high-priority stream is constant).

Solution: Mixed polling (consume 90% high, 10% low; or fair scheduling).

## Architecture Comparison

### Kafka

**Model**: Distributed append-only log. Brokers organize messages into topics/partitions. Consumers track offsets (cursor).

```
Broker 1: Partition 0 [msg-1, msg-2, msg-3, ...]
Broker 2: Partition 1 [msg-4, msg-5, msg-6, ...]
Broker 3: Partition 2 [msg-7, msg-8, msg-9, ...]

Consumer Group A: Reads P0 (offset 5), P1 (offset 3), P2 (offset 8)
Consumer Group B: Reads P0 (offset 10), P1 (offset 5), P2 (offset 1)
```

**Strengths**:
- **Durability**: Messages persisted to disk; replayed indefinitely
- **Ordering per partition**: Parallelism with guaranteed order (key-based routing)
- **Replay**: Consumers restart, re-read history

**Weaknesses**:
- Operational complexity (cluster management, rebalancing)
- Overkill for simple task queues
- Offset management critical (consumer must track position)

### RabbitMQ

**Model**: Queue-based. Messages routed to queues by exchanges (rules). Consumers acknowledge after processing.

```
Producer → Exchange (fanout/topic/direct rules) → Queue → Consumer
```

**Strengths**:
- Simpler operational model (single node, clusterable)
- Flexible routing (fanout, topic patterns, direct)
- TTL, DLQ, priority queue built-in
- Good for task distribution

**Weaknesses**:
- No replay (consumed messages are deleted)
- Ordering hard to guarantee with multiple consumers
- Less suitable for high-volume streaming

**Use case**: Job queues, work distribution, task processing.

### AWS SQS

**Model**: Fully managed, serverless queue service. Standard and FIFO variants.

**Strengths**:
- No ops (AWS manages)
- Cheap at scale
- Visibility timeout (auto-retry if consumer crashes)
- FIFO guarantees ordering

**Weaknesses**:
- Limited ordering (FIFO only, one consumer per message group)
- No replay (processed messages deleted)
- Message body size limit (256 KB)
- Visibility timeout can cause duplicate processing

**Use case**: Decoupling, fan-out notifications, task queues at AWS scale.

### NATS

**Model**: Pub/sub with optional persistence (JetStream). In-memory first, optional durability.

```
Publisher → Subject (topic name) → Multiple Subscribers
```

**Strengths**:
- Simple, lightweight (C++ core, microsecond latency)
- Pub/sub and request/reply built-in
- JetStream provides durable streaming (Kafka-like)
- Excellent for messaging infrastructure

**Weaknesses**:
- Smaller ecosystem than Kafka/RabbitMQ
- JetStream (durability) adds complexity
- Less transactional guarantees

**Use case**: Microservices messaging, real-time events, lo-latency systems.

## Comparison Table

| Feature | Kafka | RabbitMQ | SQS | NATS |
|---------|-------|----------|-----|------|
| Throughput | Very high | High | High | Very high |
| Latency | Low | Very low | Low | Ultra-low |
| Durability | Persistent log | Message-level | Managed | Optional (JetStream) |
| Ordering | Per partition | Per queue | FIFO only | Best effort |
| Replay | Yes | No | No | Yes (JetStream) |
| Complexity | High | Medium | Low | Low–Medium |
| Operations | Complex | Medium | None (managed) | Medium |
| Routing | Topic/Partition | Exchange (fanout/topic/direct) | N/A | Subject patterns |

## Best Practices

1. **Idempotent consumers**: All side effects idempotent
2. **Use message IDs**: Track duplicates where needed
3. **Implement DLQ**: Catch poison pills, failed messages
4. **Monitor lag**: Alert if consumer falls behind
5. **Partition by business key**: Ensure ordering where needed
6. **Test failures**: Simulate broker crashes, network partitions
7. **Timeout handling**: Set prefetch limits to prevent resource exhaustion
8. **Version messages**: Include schema version for evolution
9. **Use connection pooling**: Don't create new connections per message
10. **Log correlation IDs**: Trace messages through the system

## Cross-References

See also: [architecture-event-driven.md](architecture-event-driven.md), [database-kafka.md](database-kafka.md), [cloud-aws-messaging.md](cloud-aws-messaging.md), [paradigm-actor-model.md](paradigm-actor-model.md)