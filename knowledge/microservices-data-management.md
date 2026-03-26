# Microservices Data Management — Database per Service, Sagas, CQRS, Event Sourcing & Consistency

## Database Per Service Principle

**Core rule:** Each microservice owns its database. No sharing.

```
❌ WRONG (Anti-Pattern)
┌─────────────┐
│  Shared DB  │ ← multiple services read/write
└─────────────┘
Service A, B, C

✅ CORRECT
Service A → DB A
Service B → DB B
Service C → DB C
```

### Why This Matters

**Independent scaling:** Service A needs aggressive caching; Service C needs transactional consistency. Each chooses its database.

**Autonomous deployment:** Schema changes in DB A don't require coordinating with Services B and C.

**Technology diversity:** Service A uses PostgreSQL, Service B uses MongoDB, decided independently.

**Decoupled evolution:** Services evolve data models at their own pace.

### The Cost: Data Duplication

Without a shared database, services must replicate data:

*Order Service* maintains customer ID, email. *Notification Service* also needs customer email. When email changes, both services must update.

Options for staying consistent:

| Approach | Mechanism | Tradeoff |
|----------|-----------|----------|
| **CDC (Change Data Capture)** | Database logs replicate to other services' caches | Eventual consistency; infrastructure complexity |
| **Event Stream** | Domain events trigger updates downstream | Explicit; audit trail; needs idempotency |
| **API Pull** | Service B periodically fetches data from Service A | Simple; but polling overhead; stale data |

---

## Saga Pattern: Distributed Transactions

The problem: A single business action spans multiple services. Example: Order fulfillment requires charging payment, reserving inventory, creating shipment. All must succeed or all must fail—but there's no distributed ACID transaction.

**Saga** coordinates this via compensating transactions:

### Orchestration Flavor

A saga orchestrator (central workflow service) drives each step and handles failures:

```
Orchestrator: "Charge payment"
    ↓
PaymentService: succeeds

Orchestrator: "Reserve inventory"
    ↓
InventoryService: FAILS → payment needs to be reversed

Orchestrator: "Issue refund"
    ↓
PaymentService: compensation executed (charge reversed)
```

**Strengths:**
- Centralized logic (easier to understand workflow)
- Strong error handling (orchestrator knows what went wrong, can decide)

**Weaknesses:**
- Orchestrator becomes a bottleneck
- Orchestrator failure needs recovery logic
- Services become dumb (orchestrator makes all decisions)

### Choreography Flavor

Services react to events, each publishing events for others:

```
OrderService: "OrderCreated" → publishes event
    ↓
PaymentService: consumes, charges, publishes "PaymentProcessed"
    ↓
InventoryService: consumes, reserves, publishes "Reserved"
    ↓
ShippingService: consumes, creates shipment, publishes "ShipmentCreated"
```

On failure:

```
ShippingService: fails, publishes "ShipmentFailed"
    ↓
InventoryService: consumes, publishes "ReservationCanceled"
    ↓
PaymentService: consumes, issues refund, publishes "RefundProcessed"
```

**Strengths:**
- Loose coupling (services don't know about orchestrator)
- Scalable (each service works independently)
- Events are audit trail

**Weaknesses:**
- Implicit workflow (hard to track what's supposed to happen)
- Complex testing (event chains are hard to verify)
- Compensation logic hidden across services

### Idempotency Requirement

Both orchestration and choreography require **idempotent operations**: applying the same operation twice has the same effect as once.

```python
# Charged payment twice? Idempotency prevents duplicate charge
def charge_payment(order_id, amount, idempotency_key):
    # Check if this idempotency_key was already processed
    if db.charge_exists(idempotency_key):
        return db.get_charge(idempotency_key)
    
    charge = payment_api.charge(order_id, amount)
    db.store_charge(idempotency_key, charge)
    return charge
```

Without idempotency, retries (necessary for reliability) cause duplicate charges, double reservations, double shipments.

---

## CQRS: Command Query Responsibility Segregation

**CQRS** separates the write model (commands, state changes) from the read model (queries).

```
Command side:        Query side:
write database  ←→   read cache/view
(normalized)        (denormalized)
```

### Why Separate Models?

**Write model optimized for:** Consistency, transactions, ACID properties.

**Read model optimized for:** Fast queries, denormalization, indexing for common query patterns.

Example: E-commerce

**Write model:** Orders table (normalized), Payments table (normalized). Commands enforce constraints ("don't double-charge").

**Read models:**
- Dashboard view: (user_id, total_spent, order_count)
- Admin reporting: (month, category, total_revenue)
- Search: (product_name, price, inventory_count)

All read models are denormalized views over the write model, updated either synchronously or via event stream.

### Implementation Options

**Synchronous:** During command execution, update both write and read models in same transaction. Simple but write latency scales with read model complexity.

**Asynchronous:** Command updates write model; event triggers eventual updates to read models. Write is fast; reads slightly stale.

### Read Model Rebuilding

Read models are derivable—computed from write model. If read model logic changes, rebuild from scratch:

```
Delete all read model data
Replay all events (or re-query write model)
Rebuild denormalized views
Switch to rebuilt version
```

This enables zero-downtime migrations of query logic.

---

## Event Sourcing: Event Store as System of Record

Instead of storing current state, store every state change as an immutable event.

```
Traditional: [current state] → update → [new current state]
Event Sourcing: [event log] → replay → [current state derived from events]
```

### Event Stream as Audit Trail

Every change to the system has intent and consequences recorded:

```
OrderCreated(order_id=123, customer=Alice, items=[...], ts=2025-01-01)
PaymentProcessed(order_id=123, amount=50, ts=2025-01-02)
ItemShipped(order_id=123, carrier=FedEx, ts=2025-01-05)
OrderCompleted(order_id=123, ts=2025-01-10)
```

Reconstruct state at any point: What was this order's status on 2025-01-03? Replay events up to that timestamp.

### Projections

Multiple views can be computed from the same event stream:

```
Events: [OrderCreated, PaymentProcessed, ItemShipped]

Projection 1 (Financial): totals revenue by month
Projection 2 (Operational): inventory reserved vs. actual
Projection 3 (Customer): shipment status with tracking
```

Each projection is independently rebuildable.

### Snapshots for Performance

Replaying 10 years of events to get current state is slow. Take periodic snapshots:

```
Snapshot (year 2020): [state at 2020-12-31]
Then replay only events after that: [2021 events]
```

---

## Eventual Consistency: The Reality of Distributed Data

When Service A updates its database, Service B doesn't immediately see the change. There's a delay:

```
Time:  0s           2s           4s
       │           │             │
Service A: update DB A
                    ↓ CDC/event propagation
Service B: receives update, updates cache
                                ↓
Client queries Service B: now sees change
```

**Accepting this requires:**

**Idempotent reads:** Queries should be robust to seeing partially-updated state.

**Causal reads:** If you just wrote to A, you can explicitly read from A (not from B's stale cache).

**Conflict resolution:** If two services update conflicting data, what wins? (Last-write-wins? Merge? Manual intervention?)

**Monitoring staleness:** Track how far behind read models are from source database. Alert if delays exceed acceptable threshold.

---

## Cross-Service Queries

Problem: Fetch user data (Service A), their orders (Service B), and shipping status (Service C). Single query doesn't work.

### Solutions

**API Composition:** Caller (or BFF) fetches from all three services, merges locally. Simple but caller becomes orchestrator.

**Denormalization:** Service A proactively caches user + their latest order data. Simple reads; but cache invalidation is complex.

**Materialized Views:** Dedicated service maintains a view combining data from A, B, C. Updated via CDC or events. More infrastructure.

**Accept the limitation:** Don't try to join across services. Design APIs so most queries need one service.

**Recommendation:** Start with API composition. Move to denormalization/materialized views if composition becomes bottleneck.

---

## Data Consistency: Which Level?

| Level | Mechanism | When |
|-------|-----------|------|
| **Strong** | ACID transaction within one service | Critical operations (payments, inventory reservations) |
| **Eventual** | Asynchronous propagation via events/CDC | Non-critical state (caches, analytics, UI projections) |
| **Weak** | Accept stale data indefinitely | Display-only data; accepts "best-effort" correctness |

**Note:** Even eventual consistency shouldn't take days. Design your propagation pipeline so eventual becomes consistent within seconds to minutes.

---

## References & Related Concepts

See also: saga pattern, CQRS, event sourcing, change data capture, choreography, two-phase commit (to avoid).