# Event-Driven Patterns

Event-driven patterns model systems as a stream of domain events (things that happened). Rather than direct service calls, services communicate by publishing and consuming events. This decouples services in time and space, enabling loose coupling, scalability, and auditability.

## Event Types and Payload Strategies

### Domain Events

Events that represent something meaningful that occurred in the domain. Named in past tense, immutable.

```json
{
  "type": "CustomerCreated",
  "aggregateId": "customer-123",
  "eventId": "evt-456",
  "timestamp": "2025-03-15T10:30:00Z",
  "data": {
    "name": "Alice Smith",
    "email": "alice@example.com",
    "tier": "gold"
  }
}
```

### Integration Events

Domain events published to external systems or other bounded contexts. Often a subset of internal domain events — don't leak internal schema details. Versioned explicitly.

```json
{
  "type": "CustomerCreated",
  "version": 1,
  "customerId": "customer-123",
  "email": "alice@example.com"
  // tier not included; that's internal
}
```

### Event Notification vs. Event-Carried State Transfer

Two strategies for event payloads, each with trade-offs:

**Event Notification** — Thin event that signals something happened, with minimal data:

```json
{ "type": "OrderPlaced", "orderId": "order-789" }
```

Consumer must call back to get details:

```
Producer publishes "OrderPlaced: orderId=789"
   ↓
Consumer sees it
   ↓
Consumer calls GET /orders/789
   ↓
Producer responds with order details
```

**Pros:** Low payload size, schema changes don't ripple to consumers, producer and consumer remain loosely coupled.
**Cons:** Eventual consistency delay, extra query per consumer, risk of referential integrity (order might be deleted before consumer queries).

**Event-Carried State Transfer** — Event contains all data consumers need:

```json
{
  "type": "OrderPlaced",
  "orderId": "order-789",
  "customer": {
    "id": "customer-123",
    "name": "Alice Smith",
    "email": "alice@example.com"
  },
  "items": [
    { "sku": "WIDGET", "quantity": 2, "price": 29.99 }
  ],
  "total": 59.98
}
```

**Pros:** Consumers have all data immediately, no round-trips, no referential integrity issues.
**Cons:** Larger payloads, schema tightly couples producer and consumer, updates require re-emitting full state.

**Trade-off choice:**
- Use **notification** for high-volume events where the full entity is large
- Use **state transfer** for low-volume, business-critical events where consumers can't afford delays
- Hybrid: notification + eventual consistency read, then cache the state

## Event Sourcing

Event sourcing persists state as an immutable sequence of events. The event log is the source of truth; current state is derived by replaying events.

### Event Store

The core data structure: an append-only log of events per aggregate.

| Operation                        | Purpose                                              |
| -------------------------------- | ---------------------------------------------------- |
| `append(aggregateId, version, events)` | Append new events; fail if version mismatch (optimistic locking) |
| `getEvents(aggregateId)`               | Load all events for an aggregate                     |
| `getEvents(aggregateId, fromVersion)`  | Load events starting from a version (for snapshots) |
| `subscribe(type)`                      | Subscribe to events of a type (for projections)    |

### Immutability & Audit Trail

Every state change is recorded as an event:

```
BankAccount aggregate:
  - Opened(id=123, owner="Alice", balance=0) [v1]
  - Deposit(amount=1000) [v2]
  - Withdrawal(amount=200) [v3]
  - InterestEarned(amount=15) [v4]
  Current balance = 1000 - 200 + 15 = 815
```

To compute balance at version 2, replay events 1-2. Audit trail is automatic: redrive any state at any point in time.

### Event Store Implementations

| Option        | Pros                                  | Cons                                |
| ------------- | ------------------------------------- | ----------------------------------- |
| **EventStoreDB** | Purpose-built, efficient subscriptions | Additional infrastructure          |
| **PostgreSQL**    | Known system, ACID guarantees        | Rows, not optimized for log append |
| **DynamoDB**      | Fully managed, scalable              | Eventually consistent reads        |
| **Kafka**         | High-throughput event streaming      | Log compaction doesn't preserve history |

### Snapshots

Replaying 10 years of events to compute state is slow. Snapshots store the state at a point in time:

```
Event log: [Event-0, Event-1, ..., Event-1000, ..., Event-5000]
                                       ↑ Snapshot(state at 1000)

To load state at Event-4500:
  1. Load snapshot at 1000 → state
  2. Replay events 1001-4500 → final state
  Much faster than replaying 5000 events.
```

## CQRS (Command Query Responsibility Segregation)

CQRS separates read and write models:

- **Write (Command)** side: accepts commands, applies business logic, generates domain events
- **Read (Query)** side: projections derived from events, optimized for reads

```
Command-side (writes):
  UserService(CreateUserCommand) → validate → emit UserCreated event → event store

Event stream:
  UserCreated → UserEmailVerified → UserSubscribed

Query-side (reads):
  Projection: UserReadModel (id, email, name, subscription_level) ← consumed from events
  GET /users/123 → UserReadModel (fast, simple query)
```

### Benefits

- **Scalability:** Separate scaled independently. Many read replicas, small write database.
- **Optimization:** Reads stored in denormalized form (e.g., MongoDB for fast queries, relational DB for strong consistency).
- **Temporal data:** Events allow querying historical state; read model is "current" snapshot.
- **Complex domain logic:** Write model focuses on business rules; read model is dumb projection.

### Trade-offs

- **Eventual consistency:** Event processing adds latency between write and read model. User updates record, but query might return old state momentarily.
- **Dual-write problem:** If both write and read models are separate systems, a failure between them leaves inconsistency.
  
**Mitigation:** Use idempotent projections. Re-running the projection from the event log should produce the same result. Persist a checkpoint (last event version processed) so retries are safe.

## Event Sourcing + CQRS

When combined, they form a powerful pattern:

```
Command: PlaceOrder
  ↓
Order aggregate processes command → emits [OrderPlaced, InventoryReserved, PaymentProcessed]
  ↓
Events written to event store
  ↓
Read projections consume events:
  - OrderReadModel (for customer order history)
  - InventoryProjection (for stock levels)
  - RevenueProjection (for analytics)
  ↓
Query: GET /orders/123 → OrderReadModel (always fast, denormalized)
```

## Idempotent Event Consumers

Events may be delivered more than once (network retries, reprocessing). Consumers must be idempotent: applying the same event multiple times produces the same result.

### Strategies

**Deduplication via event ID:**

```python
def handle_order_placed(event):
    if db.get(f"processed:{event.id}"):
        return  # Already processed this event
    
    # Process
    inventory.reserve(event.items)
    
    # Mark as processed
    db.set(f"processed:{event.id}", True)
```

Downside: requires a persistent store to track processed events. Scaling across instances means shared state.

**Idempotent operations:**

```python
def handle_order_placed(event):
    # Inventory.reserve() is idempotent: 
    # if called twice with same order ID, second call is a no-op
    inventory.reserve(order_id=event.orderId, items=event.items)
```

Better: make the business operation itself idempotent using unique constraints or conditional updates.

**Outbox Pattern:**

Emit events and update local state in a single transaction. A separate process polls the outbox and publishes to the event broker:

```
BEGIN TRANSACTION
  INSERT into orders (id, status) VALUES (order-123, 'pending')
  INSERT into outbox (event_type, payload) VALUES ('OrderPlaced', {...})
COMMIT

-- Outbox relay process
SELECT * FROM outbox WHERE published = false
  PUBLISH to event broker
  UPDATE outbox SET published = true
```

Ensures events don't get lost, and local state updates atomically with event recording.

## Event Schema Evolution

Events are immutable and must live forever in the store. Updating an event's schema breaks old consumers. Strategies:

| Strategy     | How                                       | Pros                                    | Cons                                 |
| ------------ | ----------------------------------------- | --------------------------------------- | ------------------------------------ |
| **Versioning**   | `type` includes version: `OrderPlaced.v2` | Clear, explicit                         | Consumers must handle multiple versions |
| **Upcasting**    | Transform old events to new schema on read | Old events unchanged in store           | Upcasting logic required              |
| **New types**    | Deprecated event unchanged, new type added | No old-version code needed             | Event log cluttered with historical versions |
| **Expansion**    | Add optional fields; old events have nulls | Non-breaking                            | Consumers must handle missing fields |

### Example: Versioning

```json
// Old event (v1)
{ "type": "OrderPlaced.v1", "orderId": "123", "total": 100 }

// New event (v2) adds tax breakdown
{ "type": "OrderPlaced.v2", "orderId": "123", "total": 100, "subtotal": 90, "tax": 10 }

// Consumer handles both
if event.type == "OrderPlaced.v1":
    subtotal = event.total * 0.9  # Approximate
    tax = event.total * 0.1
else:  # v2
    subtotal = event.subtotal
    tax = event.tax
```

## Ordering Guarantees

Event ordering matters. A customer can't be charged twice if refund events arrive out of order.

| Requirement                | How to Provide                                              | Cost                         |
| -------------------------- | ----------------------------------------------------------- | ---------------------------- |
| **Per-aggregate ordering**  | Partition by aggregate ID (Kafka partitioning)              | Automatic via most brokers   |
| **Global ordering**        | Single partition (no parallelism) or consensus              | Single partition = bottleneck |
| **Causal ordering**        | Track causality with correlation/causation IDs              | Application-level logic      |

Most systems require only per-aggregate ordering. Orders for customer-123 arrive in order; orders for customer-456 can be interleaved.

## Dead Letter Queues

When event processing fails repeatedly, don't lose the event. Divert to a dead letter queue:

```
Event broker Topic: orders
  ↓ [Consumer fails 3 times]
  ↓
Dead Letter Queue: orders-dlq
```

Operators can inspect DLQ events, fix the consumer or external dependency, and replay from the queue.

**Implementation:**
```python
max_retries = 3
for attempt in range(max_retries):
    try:
        process_event(event)
        break
    except Exception as e:
        if attempt == max_retries - 1:
            dlq.send(event, reason=str(e))
        else:
            time.sleep(2 ** attempt)  # Exponential backoff
```

## Tools & Platforms

| Platform/Library | Language | Strengths                                     | Limits                        |
| --- | --- | --- | --- |
| **Kafka** | Polyglot | High-throughput, durable, partitioned, fanout | Operational complexity        |
| **Event Hubs** (Azure) | Polyglot | Managed Kafka alternative, integrations      | Vendor lock-in                |
| **Kinesis** (AWS) | Polyglot | Managed, auto-scaling, analytics integration | Limited features vs. Kafka    |
| **EventStoreDB** | Runtime-agnostic | Purpose-built event store, projections       | New ecosystem, smaller community |
| **MassTransit** | .NET | Distributed middleware, CQRS support        | .NET only                     |
| **Axon** | Java | Event sourcing framework, DDD support       | Framework overhead            |

## See Also

- **architecture-event-driven** — Architectural patterns for event-driven systems
- **database-kafka** — Kafka internals and operational details
- **patterns-circuit-breaker** — Resilience patterns needed in event systems (retries, timeouts, bulkheads)