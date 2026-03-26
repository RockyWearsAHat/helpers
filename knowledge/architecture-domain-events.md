# Domain Events — Event Definition, Publishing Patterns & Eventual Consistency

## Overview

A **domain event** represents something meaningful that occurred in the domain's business logic. It's a **fact about the past**, expressed in past tense, that other parts of the system may care about.

Domain events enable:
- **Loose coupling:** Components react to events without direct calls
- **Auditability:** A complete log of what happened, when, in what order
- **Eventual consistency:** Downstream services can catch up asynchronously
- **Temporal queries:** "What was the state at time T?" (via event sourcing)

**Domain events ≠ UI events** (button clicks, form submissions). Domain events are about business invariants.

---

## Event Definition

### Past Tense Naming

Events describe what *happened*, not what *will happen*.

```
// CORRECT: Past tense
OrderPlaced
PaymentProcessed
InventoryReserved
CustomerEmailUpdated

// INCORRECT: Imperative
CreateOrder
ProcessPayment
ReserveInventory
UpdateCustomerEmail
```

**Why?** An event is immutable historical fact. "OrderPlaced" already happened; it cannot be undone by moving code around.

### Immutable Data

Once published, an event cannot change.

```java
// CORRECT: Immutable event
public class OrderPlaced {
  public final String orderId;
  public final Instant timestamp;
  public final List<OrderItem> items;  // unmodifiable
  
  public OrderPlaced(String orderId, Instant timestamp, List<OrderItem> items) {
    this.orderId = orderId;
    this.timestamp = timestamp;
    this.items = List.copyOf(items);  // defensive
  }
}

// INCORRECT: Mutable event (anti-pattern)
public class OrderPlaced {
  public String orderId;
  public Instant timestamp;
  public List<OrderItem> items;  // can be modified after publication!
}
```

**Consequence:** Consumers never see stale/contradictory state for the same event.

### Event Metadata

Include context for correlation and tracing:

```json
{
  "type": "OrderPlaced",
  "eventId": "evt-789-001",
  "aggregateId": "order-123",
  "aggregateType": "Order",
  "version": 1,
  "timestamp": "2025-03-15T10:30:00Z",
  "correlationId": "req-abc-456",
  "causationId": "cmd-xyz-789",
  "userId": "user-42",
  "data": {
    "customerId": "c-123",
    "items": [...],
    "total": 99.50
  }
}
```

| Field           | Purpose                                                |
| --------------- | ------------------------------------------------------ |
| `type`          | Event class name; used for routing to handlers         |
| `eventId`       | Unique identifier; enables deduplication              |
| `aggregateId`   | Entity this event is about (e.g., order ID)           |
| `version`       | Event version within aggregate (for ordering)         |
| `timestamp`     | When event occurred (immutable after publication)     |
| `correlationId` | Links events from same user request across services   |
| `causationId`   | Links this event to the command that caused it        |
| `userId`        | Who triggered the event (for audits, governance)      |

---

## Event Publishing Patterns

### Immediate Publication (Synchronous)

Event is published as soon as a business transaction commits.

```java
public class OrderService {
  public Order create(CreateOrderRequest req) {
    Order order = new Order(req);
    orderRepository.save(order);          // Committed to DB
    eventBus.publish(new OrderPlaced(order, ...));  // Published
    return order;
  }
}
```

**When it works:** Single-process systems, small subscriber count, subscribers are fast.

**When it breaks:** If a subscriber crashes or is slow, the whole transaction is blocked. Subscribers are tightly coupled.

### Outbox Pattern (Transactional Safety)

Event is saved to a **local outbox table** as part of the same transaction, then published asynchronously.

```java
public class OrderService {
  public Order create(CreateOrderRequest req) {
    Order order = new Order(req);
    OrderPlaced event = new OrderPlaced(order, ...);
    
    // Single transaction:
    // 1. Insert order
    // 2. Insert event to outbox table
    // COMMIT (both or nothing)
    
    orderRepository.save(order);
    outboxRepository.save(event);  // Same DB transaction
    transactionManager.commit();
  }
}

// Separate thread/process:
// 1. Poll outbox table for unpublished events
// 2. Publish to event bus (Kafka, RabbitMQ, pub/sub)
// 3. Mark event published in outbox
// 4. Repeat
```

**Benefit:** If event publication fails, outbox still has the event. Retry mechanism can republish.

**Cost:** Extra database write + polling overhead.

### CDC (Change Data Capture)

Database logs changes (WAL, binlog). A CDC process streams those changes to an event bus.

```
Order table updated
    ↓
Database WAL/binlog records change
    ↓
CDC tool (Debezium, Maxwell) reads log
    ↓
Publishes "OrderPlaced" to Kafka
```

**Benefit:** Events are derived from fact (database state), not application logic. Highly reliable.

**Cost:** Requires database support (PostgreSQL, MySQL support it; not all databases do). CDC tool complexity.

---

## Event Notification vs. Event-Carried State Transfer

### Event Notification

Event is thin; contains only the notification that something happened.

```json
{
  "type": "OrderPlaced",
  "orderId": "order-789"
}
```

Consumer must fetch details:
```
Subscriber receives: OrderPlaced(orderId=789)
    ↓
Subscriber calls: GET /orders/789
    ↓
Publisher responds with full order details
```

**Pros:**
- Payload is small
- Publisher and subscriber schema changes are decoupled
- Publisher can update event schema without breaking old subscribers

**Cons:**
- Extra query per subscriber (N+1 problem)
- Latency (subscriber can't process until it fetches)
- Risk of referential integrity (order might be deleted before query)

### Event-Carried State Transfer

Event contains all data subscribers need.

```json
{
  "type": "OrderPlaced",
  "orderId": "order-789",
  "customerId": "c-123",
  "items": [
    { "productId": "p-1", "quantity": 2, "price": 49.99 }
  ],
  "total": 99.50,
  "shippingAddress": { "street": "123 Main St", ... },
  "timestamp": "2025-03-15T10:30:00Z"
}
```

Consumer has all data:
```
Subscriber receives: OrderPlaced event with all details
    ↓
Subscriber uses data directly (no callback needed)
    ↓
Subscriber updates its own projection/cache
```

**Pros:**
- No callbacks required; autonomous consumer
- Lower latency; consumer can proceed immediately
- Works offline; consumer can queue and process later

**Cons:**
- Larger payload
- Subscriber and publisher schema are tightly coupled
- If field is removed from event, subscribers break (versioning required)

**Reality:** Most systems use **hybrid**: notification for high-volume events, state transfer for business-critical events. Versioning and backward compatibility are required either way.

---

## Integration Events vs. Domain Events

### Domain Events

Internal to a bounded context. Other modules in the same service consume them.

```
OrderService publishes: OrderPlaced
    ↓
Internal subscribers (same monolith):
  - NotificationService (send email)
  - InventoryService (reserve stock)
  - AccountingService (record revenue)
```

**Characteristics:**
- Not versioned (changed freely within context)
- Contains detailed internal state
- Schema is loose (optional fields OK)

### Integration Events

Published to external systems or other bounded contexts. Must be stable.

```
OrderService publishes: OrderPlaced
    ↓
External systems:
  - SalesAnalytics (third-party data warehouse)
  - WMS (warehouse management, different org)
  - CustomerLoyal (loyalty program)
```

**Characteristics:**
- Versioned explicitly (v1, v2)
- Contains only business-relevant data (no internal details)
- Schema is strict (breaking changes = new version)
- May be renamed on publication (e.g., internal "OrderPlaced" → external "OrderCreatedV1")

```java
// Domain event (internal)
public class OrderPlaced {
  public String orderId;
  public Customer customer;  // full object
  public List<OrderItem> items;
  public Address billingAddress;
  public Address shippingAddress;
  public ShippingMethod shippingMethod;
  public PromoCode promoCode;
  // ... 30 fields, all internal details
}

// Integration event derived from domain event
public IntegrationEvent toIntegrationEvent(OrderPlaced domain) {
  return new OrderCreatedV1(
    orderId: domain.orderId,
    customerId: domain.customer.id,  // ID only, not full object
    items: domain.items,
    total: domain.total
    // ... only 5 fields, stable for 2+ years
  );
}
```

---

## Event Schema Design

### Schema Evolution

How do you add a field without breaking consumers?

**Approach 1: Additive (Safe)**
- Add optional fields to event
- Old consumers ignore new fields
- New consumers can use new field with default fallback

```json
{
  "type": "OrderPlaced",
  "orderId": "order-789",
  "customerId": "c-123",
  "items": [...],
  "loyaltyPointsEarned": 42  // NEW FIELD
}

// Old subscriber still works (ignores loyaltyPointsEarned)
// New subscriber uses it if present, or uses default if absent
```

**Approach 2: Versioning**
- New schema = new type or version field
- Old and new types coexist
- Subscribers opt-in to new version

```json
// v1 (old)
{ "type": "OrderPlacedV1", "orderId": "...", "items": [...] }

// v2 (new, added bundle field)
{ "type": "OrderPlacedV2", "orderId": "...", "items": [...], "bundleId": "..." }

// Subscribers pick which version(s) they handle
public class LoyaltyService implements OrderPlacedV1Subscriber, OrderPlacedV2Subscriber {
  @EventHandler void on(OrderPlacedV1 e) { /* handle v1 */ }
  @EventHandler void on(OrderPlacedV2 e) { /* handle v2 */ }
}
```

### Ordering

Events within an **aggregate** must be ordered causally.

```
OrderPlaced (version 1)
    ↓
PaymentProcessed (version 2)
    ↓
OrderFulfilled (version 3)
```

Version numbers ensure ordering even if events arrive out-of-order from event bus.

**Cross-aggregate ordering:** No guarantee. OrderPlaced from Aggregrate A and OrderPlaced from Aggregate B have no defined order. This is eventual consistency.

---

## SEC (Strict Event Consistency) Property

When events are **causally ordered** and no events are lost, the system has strong consistency guarantees. Consumers can safely build accurate projections:

1. **Causality preserved:** Version numbers or timestamps order events correctly
2. **No gaps:** Every event is delivered or explicitly marked as failed
3. **Exactly-once semantics:** Duplicate events are deduplicated (via eventId)

**Implication:** A data warehouse, reporting database, or cached read model built from events is *guaranteed accurate* (up to the latest event consumed).

```
Event log: [OrderPlaced-v1, OrderPlaced-v2, OrderPlaced-v3]
    ↓
Analytics DB (exactly-once processing)
    ↓
Analytics DB has accurate state (same as production at event-v3 time)
```

If the event bus doesn't guarantee ordering or can lose events, SEC is violated and projections become eventually inconsistent or permanently inaccurate.

---

## See Also

- Event Sourcing (event storage, projections, temporal queries)
- Event-Driven Architecture (system-wide event flow)
- Saga Pattern (events trigger compensating transactions)
- Domain-Driven Design (domain modeling, bounded contexts)
- Distributed Transactions (compare with eventual consistency via events)