# Event-Driven Architecture

## Event Types

### Domain Events

Record something that happened in the domain. Named in past tense. Immutable.

```json
{
  "type": "OrderPlaced",
  "aggregateId": "order-789",
  "data": { "customerId": "c-123", "items": [...], "total": 99.50 },
  "metadata": { "timestamp": "2025-01-15T10:30:00Z", "correlationId": "req-abc" }
}
```

### Integration Events

Domain events published to other bounded contexts. May be a subset of internal domain events — don't leak internal domain details.

### Event Notification

Thin event that says something happened, with minimal data. Consumers call back for details if needed. Low coupling but adds query load.

```json
{ "type": "OrderPlaced", "orderId": "order-789" }
```

### Event-Carried State Transfer

Event contains all the data consumers need. No callbacks required. Higher coupling to schema but eliminates runtime dependencies.

```json
{ "type": "OrderPlaced", "orderId": "order-789", "customer": { "id": "c-123", "name": "...", "email": "..." }, "items": [...] }
```

**Trade-off matrix**:

| Event Style    | Coupling        | Autonomy              | Payload Size | Consumer Complexity |
| -------------- | --------------- | --------------------- | ------------ | ------------------- |
| Notification   | Low             | Low (must callback)   | Small        | Higher              |
| State transfer | Higher (schema) | High (self-contained) | Larger       | Lower               |

## Event Sourcing

Store state as a sequence of events, not as current state. The event log is the source of truth.

### Event Store

Append-only log of events per aggregate. Core operations:

```
append(aggregateId, expectedVersion, events[])  → success or concurrency conflict
load(aggregateId) → events[]
load(aggregateId, fromVersion) → events[]  // for snapshots
```

Implementations: EventStoreDB (purpose-built), PostgreSQL (with advisory locks), DynamoDB (with conditional writes), Kafka (with compaction caveats).

### Projections

Materialize events into read-optimized views. A projection is a function: `fold(state, event) → state`.

```python
def project_order_summary(state, event):
    match event.type:
        case "OrderPlaced":
            return {"id": event.order_id, "status": "placed", "total": event.total, "items": len(event.items)}
        case "OrderShipped":
            return {**state, "status": "shipped", "tracking": event.tracking_number}
        case "OrderCancelled":
            return {**state, "status": "cancelled"}
```

Multiple projections from the same event stream — one for the customer dashboard, one for analytics, one for search indexing.

### Snapshots

When an aggregate has thousands of events, replaying from the start is slow. Take periodic snapshots:

```
Load: snapshot(v1000) + events[1001..1023] → current state
```

Snapshot every N events (e.g., 100). Store snapshot alongside the event stream.

### Event Versioning and Upcasting

Events are immutable but schemas evolve. Strategies:

| Strategy       | How It Works                                    | Complexity       |
| -------------- | ----------------------------------------------- | ---------------- |
| Weak schema    | Ignore unknown fields, use defaults for missing | Low              |
| Upcasting      | Transform old events to new schema on read      | Medium           |
| Copy-transform | Migrate event store to new schema               | High (but clean) |
| Event adapters | Map old→new at the consumer boundary            | Medium           |

**Upcaster example**: `OrderPlaced_v1` had `price` (single currency). `OrderPlaced_v2` has `amount` + `currency`. Upcaster adds `currency: "USD"` to v1 events.

## CQRS (Command Query Responsibility Segregation)

### Core Concept

Separate the write model (handles commands, enforces invariants) from the read model (optimized for queries).

```
Command → Write Model → Events → Read Model → Query
              │                      │
         (validates,            (denormalized,
          enforces rules)        fast reads)
```

### Read Models

Build purpose-specific read models from events:

- **SQL view**: Denormalized table for dashboards
- **Search index**: Elasticsearch for full-text search
- **Cache**: Redis for hot data
- **Graph**: Neo4j for relationship queries

Each read model can use the optimal storage technology. Rebuild any read model by replaying events.

### Eventual Consistency

Write and read models are not instantly in sync. The delay (typically milliseconds to seconds) must be acceptable. Strategies:

- **Causal consistency**: After a write, read-your-own-writes by querying the write model or checking event version
- **UI optimistic updates**: Assume success, correct on failure
- **Polling/SSE**: Push updates to clients when read model catches up

## Message Brokers Comparison

| Feature        | Kafka                              | RabbitMQ                     | NATS                       |
| -------------- | ---------------------------------- | ---------------------------- | -------------------------- |
| Model          | Distributed log                    | Message queue                | Pub/sub + queue            |
| Ordering       | Per-partition                      | Per-queue                    | Per-subject (JetStream)    |
| Retention      | Time/size-based, durable           | Until consumed/TTL           | JetStream: configurable    |
| Throughput     | Very high (millions/sec)           | High (tens of thousands/sec) | Very high                  |
| Consumer model | Pull (consumer controls pace)      | Push (broker delivers)       | Both                       |
| Replay         | Yes (offset reset)                 | No (once consumed, gone)     | JetStream: yes             |
| Use case       | Event streaming, log aggregation   | Task queues, RPC             | Lightweight messaging, IoT |
| Complexity     | High (ZooKeeper/KRaft, partitions) | Medium                       | Low                        |
| Delivery       | At-least-once (default)            | At-least-once, at-most-once  | At-least-once (JetStream)  |

**Kafka** when you need event replay, high throughput, or stream processing. **RabbitMQ** when you need flexible routing, work queues, or traditional messaging. **NATS** when you need lightweight, low-latency pub/sub.

## Event Schemas

### Schema Formats

| Format      | Encoding    | Schema Evolution                | Ecosystem                               |
| ----------- | ----------- | ------------------------------- | --------------------------------------- |
| Avro        | Binary      | Excellent (compatibility rules) | Kafka-native, Confluent Schema Registry |
| Protobuf    | Binary      | Good (field numbering)          | gRPC ecosystem, language codegen        |
| JSON Schema | Text        | Acceptable (manual validation)  | Universal, human-readable               |
| CloudEvents | Text/Binary | Spec for envelope, not payload  | CNCF standard, vendor-neutral           |

### CloudEvents Specification

Standard envelope format for events across systems:

```json
{
  "specversion": "1.0",
  "type": "com.example.order.placed",
  "source": "/orders/service",
  "id": "evt-456",
  "time": "2025-01-15T10:30:00Z",
  "datacontenttype": "application/json",
  "data": { "orderId": "order-789", "total": 99.5 }
}
```

## Idempotency

Events may be delivered more than once. Consumers must handle duplicates:

| Strategy             | How                                                          | Trade-off                     |
| -------------------- | ------------------------------------------------------------ | ----------------------------- |
| Idempotency key      | Store processed event IDs, skip duplicates                   | Requires storage for seen IDs |
| Natural idempotency  | Operations that are inherently idempotent (SET vs INCREMENT) | Not always possible           |
| Deduplication window | Track recent event IDs in a time window                      | Window size trade-off         |
| Conditional writes   | `UPDATE ... WHERE version = expected`                        | Optimistic concurrency        |

## Ordering Guarantees

- **Total order**: All consumers see events in the same order. Expensive, usually unnecessary.
- **Partition order**: Events with the same key are ordered. Default in Kafka (per-partition).
- **Causal order**: If event A caused event B, all consumers see A before B.
- **No order**: Events may arrive in any order. Consumers must handle reordering.

**Partition key strategy**: Use the aggregate ID as the partition key. All events for one order go to the same partition → guaranteed order per order.

## Dead Letter Queues

Failed messages go to a DLQ for investigation instead of blocking the pipeline:

```
Consumer → Process → Success → Ack
              ↓
           Failure (after N retries)
              ↓
           Dead Letter Queue → Alert → Manual investigation
```

DLQ messages need: original message, error details, retry count, timestamp. Build tooling to inspect, replay, or discard DLQ messages.

## Event Replay

Re-process historical events to:

- **Rebuild projections**: Fix a bug in a read model, replay all events to rebuild
- **Backfill new projections**: Add a new read model and populate from history
- **Debug issues**: Replay a specific aggregate's events to understand state transitions
- **Test changes**: Replay production events in a staging environment

**Replay safety**: Ensure side effects (emails, payments) are not re-triggered during replay. Use flags or separate replay infrastructure.

## Saga Patterns

### Orchestration

A central coordinator directs the saga:

```
Saga Orchestrator
  → Command: ReserveInventory → Inventory Service → InventoryReserved
  → Command: ChargePayment → Payment Service → PaymentCharged
  → Command: ScheduleShipment → Shipping Service → ShipmentScheduled

On failure at any step:
  → Compensating commands in reverse order
```

**Pros**: Clear flow, easy to add steps, centralized failure handling.
**Cons**: Coordinator is a single point of knowledge, risk of becoming a god service.

### Choreography

Services react to events without central coordination:

```
OrderPlaced → Inventory listens → InventoryReserved → Payment listens → PaymentCharged → ...
```

**Pros**: Loose coupling, no coordinator.
**Cons**: Hard to visualize, debug. Compensating flows are implicit and scattered.

## Outbox Pattern

Solve the dual-write problem: writing to DB and publishing an event must be atomic.

```
┌─────────────────────────────────┐
│ Database Transaction            │
│  1. UPDATE orders SET status=...│
│  2. INSERT INTO outbox (event)  │
│  COMMIT                         │
└─────────────────┬───────────────┘
                  │ CDC or polling
                  ↓
           Message Broker → Consumers
```

### Change Data Capture (CDC)

Instead of polling the outbox table, use CDC (Debezium) to stream database changes directly to Kafka. Lower latency, no polling overhead, captures the exact change.

```
PostgreSQL WAL → Debezium → Kafka → Consumers
```

**Outbox + CDC is the gold standard** for reliable event publishing from services that use relational databases.
