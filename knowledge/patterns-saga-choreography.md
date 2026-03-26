# Saga Choreography — Event-Driven Distributed Transactions

## Overview

**Saga choreography** is a pattern for coordinating long-running distributed transactions across multiple services by exchanging events, with no central coordinator. Each service acts autonomously, publishing domain events when local transactions complete, and subscribing to events from other services to trigger its part of the workflow.

Unlike saga orchestration (where a coordinator drives each step), choreography distributes responsibility. This creates loose coupling but requires careful design of event semantics, compensation logic, and failure handling.

---

## Event Chain Design

### The Core Flow

A choreographed saga models a distributed workflow as a chain of local transactions triggered by events:

```
Service A: local transaction + publish Event1
    ↓ [Event1 published to broker]
Service B: consume Event1, local transaction + publish Event2
    ↓ [Event2 published to broker]
Service C: consume Event2, local transaction + publish Event3
    ↓ [Event3 published to broker]
Complete
```

Example: Order fulfillment saga
```
1. OrderService: Create order, publish "OrderCreated"
2. PaymentService: Charge customer, publish "PaymentProcessed"
3. InventoryService: Reserve stock, publish "StockReserved"
4. ShippingService: Create shipment, publish "ShipmentCreated"
5. NotificationService: Send confirmation, publish "ConfirmationSent"
```

### Event Naming Conventions

Clear event naming is critical for choreography's maintainability. Use verbs in past tense (naming the fact, not the action):

- **Domain-qualified:** `order.created`, `payment.processed`, `inventory.reserved` (not ambiguous `create_order`)
- **Unambiguous:** Include the aggregate involved: `UserCreated` not `Created` (created what?)
- **Versionable:** `OrderCreated_v2` when semantics change (allows gradual migration)

**Why?** Services subscribing to events need to know which aggregate type triggered them. Ambiguous names cause misrouted logic and data corruption.

---

## Compensation Handling

### Reverse Operations

When a step fails, prior steps must undo their effects. Each participating service defines a **compensation transaction** that reverses its committed change.

```
Service A: Transaction → Success → publish Event1
Service B: Transaction → Success → publish Event2
Service C: Transaction → FAIL → publish CompensationNeeded Event

Compensation chain:
Service B: Compensation (reverse prior transaction) + publish Compensated
    ↓
Service A: Compensation + publish Compensated
```

**Compensation as business logic, not undo:**

Compensation is not a database rollback. It's an explicit business operation that runs in a separate local transaction:

```python
# Forward transaction
def reserve_inventory(order_id, items):
    for item in items:
        update_stock(item, -quantity)  # Decrement stock
    publish("StockReserved", order_id)

# Compensation (business operation, not just undo)
def compensate_inventory(order_id, items):
    for item in items:
        update_stock(item, +quantity)  # Increment stock
    publish("StockReservationCompensated", order_id)
```

### Designing Compensable Transactions

Not all operations are easily compensable. Design services with compensation in mind:

- **Idempotent operations:** Can be safely re-executed (see Idempotency section)
- **Reversible state changes:** Stock, balance, account status can be reversed
- **Audit trail:** Record why compensation occurred and by whom (required for disputes)
- **Avoid permanent side effects:** Notifications, external API calls, physical actions
  - If you must, make them idempotent and log them for manual review

**Hard-to-compensate operations:**
- Email sent → Cannot reliably unsend (design: mark for retraction, log)
- Funds transferred to external bank → Requires manual refund (design: flag for ops team)
- Shipment handed to carrier → Physical reversal required (design: create return order)

---

## Event Ordering

### Single-Service Causality

Within a single service, order matters. If a saga is triggered by events E1 → E2 → E3, they must reach consumers in that order.

**Per-partition ordering** (achieved via event keys in Kafka, partitions in e.g., Azure Service Bus):
```
Order ID as partition key:
  Partition 0: [OrderCreated(order-123), PaymentProcessed(order-123), ...]
  Partition 1: [OrderCreated(order-456), PaymentProcessed(order-456), ...]
```

**Multiple consumers may process events concurrently**, but within a logical aggregate (order ID), events must be ordered.

### Cross-Service Eventual Consistency

Events published by different services may arrive out-of-order at consumers. Services must handle:

```
Publish order:         T=0: OrderCreated
Publish payment:       T=2: PaymentProcessed  
Inventory consumer receives them T=5: PaymentProcessed, T=6: OrderCreated (reversed!)
```

**Solution:** Consumer state machines. Track which step of the saga is complete and only process events applicable to the current state:

```python
class SagaState:
    def __init__(self):
        self.state = "init"
    
    def on_payment_processed(self):
        if self.state == "awaiting_payment":
            self.state = "payment_done"
        else:
            # Ignore or buffer—out of order
            pass
    
    def on_order_created(self):
        if self.state == "init":
            self.state = "awaiting_payment"
        else:
            # Already processed; could be duplicate
            pass
```

---

## Idempotent Event Handlers

### The Core Requirement

Event brokers (Kafka, RabbitMQ) offer **at-least-once** delivery semantics: events may be delivered multiple times if processing crashes, consumer restarts, or timeouts occur.

Consumers **must process the same event multiple times and produce the same effect**:

```
First delivery:  OrderCreated(id=123) → Reserve stock → Succeeds
Consumer crashes
Redelivery:      OrderCreated(id=123) → Reserve stock → Same effect (stock already reserved)
```

### Idempotency Key Pattern

Store a deduplication set (e.g., a set of processed event IDs) in local storage:

```python
def handle_order_created(event):
    idempotency_key = f"saga-{event.id}-{event.event_type}"
    
    # Check if already processed
    if db.get_processed_events(idempotency_key):
        return  # Idempotent: skip if already done
    
    # Execute the business operation
    reserve_stock(event.items)
    publish("StockReserved", event.id)
    
    # Mark as processed
    db.mark_processed(idempotency_key, event)
```

**Store idempotency state alongside the business effect:** In the same transaction, write both the stock reservation AND the processed event ID. This ensures atomicity—if the service crashes, both occur or neither.

### Deterministic IDs

Events must carry **globally unique, deterministic IDs** so consumers can recognize duplicates:

```json
{
  "id": "order-123-20260325-001",  // Deterministic: order ID + timestamp + sequence
  "event_type": "OrderCreated",
  "timestamp": "2026-03-25T10:30:00Z"
}
```

Avoid randomly generated UUIDs unless the source system guarantees uniqueness per business transaction.

---

## Dead Letter Handling

### The Problem

Compensations can fail too. If Service A publishes a compensation event but Service B is down:

```
Service A: Publishes "StockReservationCompensated"
Service B: DOWN (cannot receive)
Service A: Attempts retry, then gives up
Result: Compensation never executed; data is inconsistent
```

### Dead Letter Queue (DLQ) Pattern

Configure a message broker (Kafka, RabbitMQ) to route messages that cannot be delivered to a **dead letter queue**:

```
Handler subscription → Message delivery attempt
  ├─ Success → process and continue
  └─ Failure (error, timeout, crash)
    └─ Retry 3x with backoff
      └─ Final failure
        → Route to DLQ
```

**DLQ characteristics:**
- Topic/queue with a distinct name: `saga-compensation-dlq`
- Holds messages indefinitely (configurable retention)
- Enables operational monitoring and debugging
- Typically reviewed and resolved manually

**Processing a DLQ:**

```python
def handle_dlq_message(event):
    """Manually inspect and resolve dead letter events"""
    logger.error(f"DLQ: {event.id} - {event.event_type}")
    
    # Attempt automated recovery
    try:
        retry_with_backoff(process_event, event)
        move_to_processed_queue(event)
    except Exception as e:
        # Create operational alert for manual intervention
        alert_operations_team(event, reason=str(e))
```

### Saga-Specific Dead Letter Strategy

For compensations, a failed compensation is catastrophic—the saga is stuck. Options:

1. **Retry with exponential backoff** → alerting after N retries
2. **Saga compensations go to a priority DLQ** (more urgent than regular messages)
3. **Human review required** → compensation cannot proceed without ops team approval (some sagas are too critical to fail silently)

---

## Monitoring Saga Progress

### Saga State Tracking

Without a central coordinator, tracking saga progress requires **distributed tracing** and a shared event log:

```
Saga instance: order-123

Timeline:
  T=0s: OrderCreated event → Stored in saga event log
  T=0.5s: PaymentService processes, publishes PaymentProcessed
  T=1s: InventoryService processes, publishes StockReserved
  T=2s: ShippingService processes, publishes ShipmentCreated
  T=3s: Saga COMPLETE or ERROR (timeout after 5min + auto-compensation)
```

**Operational dashboard:**
- Current state of each saga instance: Pending, Complete, Compensating, Failed
- Duration from start to completion
- Event chain: which services have processed, which are pending
- Compensation status: pending, in-progress, complete, failed

**Implementation:** Event store (Kafka log, EventStoreDB, or database event table):

```sql
CREATE TABLE saga_events (
  saga_id VARCHAR(255),
  event_id VARCHAR(255),
  event_type VARCHAR(255),
  event_timestamp TIMESTAMP,
  service_name VARCHAR(255),
  status VARCHAR(50),  -- processed, failed, compensated
  payload JSON,
  PRIMARY KEY (saga_id, event_timestamp)
);
```

### Correlation IDs

Every event in a choreographed saga must carry a **correlation ID** linking all events to the originating request:

```json
{
  "correlation_id": "saga-order-123-20260325",
  "event_id": "payment-evt-001",
  "event_type": "PaymentProcessed",
  "saga_id": "order-123"
}
```

Logs and metrics use correlation ID to trace the full saga across all services:

```
// Service A logs
2026-03-25 10:30:01 [correlation_id=saga-order-123] OrderCreated

// Service B logs
2026-03-25 10:30:02 [correlation_id=saga-order-123] PaymentProcessed

// Tracing aggregates all by correlation_id
```

---

## Testing Choreographed Sagas

### Unit Testing

Test each service's event handler in isolation:

```python
def test_inventory_handles_stock_reserved_event():
    """Inventory service processes a stock reservation event"""
    event = OrderCreated(order_id=123, items=[{"sku": "A", "qty": 5}])
    
    handler = InventoryEventHandler()
    result = handler.on_order_created(event)
    
    assert result.status == "reserved"
    assert result.stock_change == -5
    assert "StockReserved" in result.published_events
```

### Integration Testing: Event Chain

Test the chain of services publishing and consuming events via a test message broker:

```python
@pytest.fixture
def test_broker():
    return InMemoryMessageBroker()

def test_order_saga_happy_path(test_broker):
    """Full saga: order → payment → inventory → shipping"""
    order_service = OrderService(test_broker)
    payment_service = PaymentService(test_broker)
    inventory_service = InventoryService(test_broker)
    shipping_service = ShippingService(test_broker)
    
    # Trigger saga
    order_service.create_order(items=[...])
    
    # Assert event chain
    assert test_broker.get_events_for("PaymentProcessed")  # Payment executed
    assert test_broker.get_events_for("StockReserved")     # Inventory executed
    assert test_broker.get_events_for("ShipmentCreated")   # Shipping executed
    
    # Assert final outcome
    assert order_service.get_order(123).status == "completed"
```

### Chaos Testing: Failure Scenarios

Test compensation paths by simulating failures:

```python
def test_order_saga_inventory_failure_triggers_compensation(test_broker):
    """Inventory fails; other services compensate"""
    order_service = OrderService(test_broker)
    payment_service = PaymentService(test_broker)
    inventory_service = InventoryService(test_broker)
    
    # Inventory will fail on this event
    inventory_service.fail_on_order_id(123)
    
    # Trigger saga
    order_service.create_order(order_id=123, items=[...])
    
    # Assert compensation chain
    assert test_broker.get_events_for("PaymentRefunded")  # Payment compensated
    assert not test_broker.get_events_for("ShipmentCreated")  # Never reached
    
    # Assert final outcome
    assert order_service.get_order(123).status == "cancelled"
```

### Testing Idempotency

Verify handlers process duplicate events safely:

```python
def test_payment_handler_idempotent():
    """Processing same event twice has no side effects"""
    event = PaymentProcessed(id="payment-001", order_id=123, amount=50.00)
    handler = PaymentEventHandler()
    
    # First delivery
    balance_before = db.get_customer_balance(123)
    handler.on_payment_processed(event)
    balance_after_1 = db.get_customer_balance(123)
    
    # Redelivery (duplicate)
    handler.on_payment_processed(event)
    balance_after_2 = db.get_customer_balance(123)
    
    # Same result
    assert balance_after_1 == balance_after_2
    assert balance_after_1 == balance_before + 50.00
```

---

## Choreography vs. Orchestration

| Aspect | Choreography | Orchestration |
|--------|---|---|
| **Control flow** | Distributed, event-driven; services react | Centralized; coordinator drives each step |
| **Coupling** | Low: services only know events they consume | High: services know the coordinator |
| **Visibility** | Implicit; debugging requires log aggregation | Explicit; coordinator is the single authority |
| **Scalability** | Decentralized; no bottleneck | Coordinator can become a bottleneck |
| **Failure handling** | Compensation logic scattered; harder to reason about | Centralized recovery; easier to reason about |
| **Testing** | Complex; must test event chains and async failures | Simpler; coordinator controls the flow |
| **Add a step** | Subscribe to events (loosely coupled) | Often requires coordinator changes |

**Use choreography when:**
- Services are loosely coupled and event-driven
- Many independent services participate in sagas
- Scalability of event processing is critical
- You have observability infrastructure (tracing, event logs)

**Use orchestration when:**
- The saga is complex with conditional branching
- Debugging and visibility are critical
- Services are tightly integrated anyway
- The coordinator isn't a bottleneck

---

## See Also

- [architecture-saga-pattern.md](architecture-saga-pattern.md) — General saga concepts and orchestration model
- [patterns-event-driven.md](patterns-event-driven.md) — Event-driven architecture principles
- [patterns-idempotency.md](patterns-idempotency.md) — Idempotent operations
- [architecture-event-sourcing.md](architecture-event-sourcing.md) — Event store backends for sagas