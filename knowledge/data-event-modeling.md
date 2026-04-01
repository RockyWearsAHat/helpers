# Event Modeling — Methodology, Storming, and Temporal Design

Event modeling is a collaborative practice and design discipline for understanding and documenting systems through the lens of immutable events. It complements [Domain-Driven Design](architecture-ddd.md) and [Event Sourcing](architecture-event-sourcing.md) by providing a structured methodology for discovery and design.

## Event Storming

Event Storming is a collaborative workshop technique that uses sticky notes (or digital equivalents) to map a domain through events. Iterations progress from broad discovery to detailed design.

### Big Picture (2-4 hours)

Goal: Identify all domain events and high-level flow without implementation details.

**Process:**
1. Facilitator explains domain to group (problem statement, business drivers)
2. Each participant adds sticky notes with events observed in the domain (orange for events, arbitrary colors for actors)
3. Group arranges events left-to-right in sequence; no implementation model yet
4. Discuss and add missing events; merge duplicates

**Outcome:**
- Comprehensive event timeline for the domain
- Stakeholder alignment on what "happens"
- Identification of blind spots (missing processes, unclear sequences)

**Example: E-commerce order system**
```
Events (left-to-right timeline):
1. ProductViewed
2. ItemAddedToCart
3. CartAbandoned (alternative path)
4. CheckoutStarted
5. PaymentProcessed
6. OrderPlaced
7. InventoryReserved
8. ShipmentInitiated
9. TrackingNumberAssigned
10. DeliveryAttempted
11. DeliveryFailed (alternative)
12. DeliverySucceeded
13. RefundRequested
14. RefundProcessed
```

### Process Level (4-8 hours)

Dive into a single business process. Map command-event-reaction flow, identify actors and systems.

**Sticky note legend:**
- **Blue (Commands)**: User or system actions triggering events. Named imperative ("PlaceOrder")
- **Orange (Events)**: Things that happened, immutable. Named past tense ("OrderPlaced")
- **Yellow (Actors/Roles)**: Who triggered the command (Customer, Admin, System)
- **Pink (Policies/Rules)**: Reaction logic ("If payment accepted, create shipment")
- **Red (Problems)**: Unknowns, edge cases, conflicts

**Process:**
1. Start with a trigger event or command
2. Add command → ask who triggers it (actor)
3. Ask what event results from this command
4. Ask what policies/consequences follow; loop
5. Identify failures, exceptions, alternative flows
6. Mark unknowns, complex logic, system boundaries

**Example: "PlaceOrder" process**
```
Actor (Customer) → Command (PlaceOrder) → Event (OrderPlaced)
↓
Policy: "Check inventory; if available, reserve"
↓
Event (InventoryReserved)
↓
Policy: "If inventory unavailable, publish OutOfStockEvent"
↓
Event (OutOfStockNotified)
↓
Alternative: Customer can cancel → Command (CancelOrder) → Event (OrderCancelled)
```

**Outcomes:**
- Clear separation of commands (requests) and events (results)
- Identification of policies and automated reactions
- Alternative paths, edge cases, business rules
- Conversation about system boundaries (which system handles which event?)

### Design Level (2-4 hours per aggregate)

Create technical design for a specific aggregate (bounded context responsibility). Map event structure, snapshots, projections.

**Focus on:**
1. **Aggregate identity**: What is the root entity? (Order, Account, Shipment)
2. **Events within the aggregate**: What state changes? (OrderPlaced, PaymentProcessed, OrderCancelled, OrderShipped)
3. **Commands accepted**: What commands alter state? (PlaceOrder, ProcessPayment, CancelOrder, ShipOrder)
4. **Invariants**: What rules must always hold? (Payment must clear before shipping; Order must have items)
5. **Projections**: What read models do downstream services need?
6. **Event snapshots**: At what point snapshot to avoid replaying 10k events?

**Example: Order aggregate**
```
Identity: OrderId
Events:
  • OrderPlaced(orderId, customerId, items[], timestamp)
  • PaymentProcessed(orderId, amount, transactionId)
  • InventoryReserved(orderId, [itemId, quantity]...)
  • ShipmentInitiated(orderId, shipmentId, trackingNumber)
  • OrderCancelled(orderId, reason, refundAmount, timestamp)
  • OrderCompleted(orderId, deliveryTimestamp)

Invariants:
  - Payment must succeed before ShipmentInitiated
  - InventoryReserved must occur before ShipmentInitiated
  - After OrderCancelled, no further state changes allowed

Projections:
  - OrderSummary: {orderId, status, total, estimatedDelivery}
  - CustomerOrders: List of orders per customer
  - RevenueReport: OrderPlaced events grouped by date, product, region
```

**Pitfalls:**
- Over-designing: Too many events; break into smaller aggregates
- Missing events: Forgot refunds, amendments, cancellations
- Vague event data: "OrderUpdated" vs. specific events like "ShippingAddressChanged"

---

## Event-First Design Principles

Event-first design prioritizes **what happened** over **what the system does**. This inverts traditional imperative design.

### Immutability and Append-Only

Events are immutable facts. Never modify or delete events; only append.

**Benefits:**
- Audit trail built-in; no need for separate audit logging
- Temporal queries: "What was state at time T?" via event replay
- Recovery: Rebuild state from events
- Conflict resolution: Multiple sources can merge events by timestamp

**Trade-off:** Storage grows over time; must eventually snapshot or purge old events.

### Command-Event-State Pattern

1. **Command**: Request to change state. Named imperative. May fail. Example: `PlaceOrder(customerId, items[])`
2. **Event**: Immutable fact confirming change. Named past tense. May have metadata (timestamp, actor, version). Example: `OrderPlaced(orderId, customerId, items[], total, timestamp)`
3. **State**: Current summary (read model). Derived from events. Ephemeral; can be rebuilt. Example: `Order { id, status="placed", items, total }`

Flow:
```
Command (request) → Validate → Emit Event (if valid) → Apply Event to State
                                        ↓
                                    Invalid: Return error
                                    Valid: Emit event
```

**Key insight:** State is never the source of truth; events are. Read models (state) are always derived, always up-to-date or eventually consistent.

### Polymorphic Events

Different event types for different state transitions. Avoid generic events.

```
Generic (bad):
  OrderEvent { orderId, type: "updated", data: {...} }

Specific (good):
  OrderPlaced { orderId, customerId, items[], total }
  PaymentProcessed { orderId, transactionId, amount }
  OrderCancelled { orderId, reason, refundAmount }
```

Benefits: Type safety, handler clarity, easier testing and documentation.

---

## Temporal Modeling

Events carry temporal semantics. Design event payloads to support time-based queries.

### Event Timestamp Fields

**Event Time** (actual time of occurrence): When the business event truly happened.
- Immutable; set when event is committed
- Enable: "Orders placed on this date?"

**Processing Time** (when system processed): When the system received and processed the event.
- May differ from event time due to delays
- Enable: "When did we learn about this?" (compliance, audit)

**Correlation ID / Causality Chain**: Link events caused by the same command or request.
```json
{
  "eventId": "evt_12345",
  "eventType": "OrderPlaced",
  "eventTime": "2026-03-25T14:30:00Z",
  "processingTime": "2026-03-25T14:30:02Z",
  "causationId": "cmd_99999",
  "correlationId": "transaction_abc"
}
```

### Versioning and Schema Evolution

Events accumulate over time; schema must evolve without breaking history.

**Field additions**: Safe; old events default to null or use fallback logic.

**Field removals**: Risky; require migration. Mark fields deprecated, then remove.

**Type changes**: Avoid if possible. Create new events (`OrderAmended` vs. `OrderUpdated`) if logic differs.

**Upcasting**: Convert old event versions to current schema during replay.

```python
def upcaster(event):
    if event.version == 1:
        # Old schema: "address" → split into "street", "city", "zip"
        event.street = event.address.split(',')[0]
        event.city = event.address.split(',')[1]
        event.zip = event.address.split(',')[2]
        del event.address
        event.version = 2
    return event
```

---

## Compliance and GDPR

Events create audit trails; they also create compliance obligations.

### Right to Forget (GDPR Article 17)

Users can request deletion of personal data. Events containing PII must be handled carefully.

**Approaches:**

1. **Event Deletion**: Physically delete events mentioning the user. Breaks event stream integrity; require replay of downstream effects.

```json
Before: [OrderPlaced(..., customerId="user123"), PaymentProcessed(...)]
After:  [PaymentProcessed(...)]  // Missing context; replays are incomplete
```

2. **Pseudonymization**: Replace PII with tokens; store mapping separately (also delete on request).

```json
{
  "eventType": "OrderPlaced",
  "orderId": "order_abc",
  "customerId": "cust_token_xyz",  // Replaced; mapping in separate store
  "amount": 100
}

PII Store (delete on request):
  cust_token_xyz → {name, email, address}
```

3. **Encryption + Key Deletion**: Encrypt PII fields; delete encryption key. Data is unrecoverable without key.

```json
{
  "eventType": "OrderPlaced",
  "orderId": "order_abc",
  "encryptedCustomerData": "base64...",  // Encrypted with key_user123
  "amount": 100
}

// On deletion request: delete key_user123 → data unrecoverable
```

### Event Retention Policies

**Types:**
- **Legal hold**: Retain indefinitely (financial, healthcare, tax requirements)
- **Automatic purge**: Delete after TTL (GDPR: can delete after statutes of limitations)
- **Selective retention**: Keep aggregated data, discard details

**GDPR Exceptions to right-to-forget:**
- Contractual obligations (must keep to fulfill orders)
- Legal comply (tax, accounting)
- Public interest (research, statistics if anonymized)
- Fraud prevention (retain signals without PII)

### Audit Trail Design

Maintain compliance-safe audit logs distinct from operational events.

```json
Operational Event (immutable):
  OrderPlaced { orderId, customerId, items, amount, timestamp }

Audit Event (separate store, immutable):
  OrderAccessed { orderId, userId, action: "viewed", timestamp, ipAddress, reason }
```

Separating operational and audit events allows:
- Delete operational data (compliance) while keeping audit trail (investigation)
- Different retention policies
- Role-based access (audit governed by compliance team, operational by domain team)

---

## Event Modeling Tools & Artifacts

**Miro, FigjJam, or physical whiteboard**: Real-time collab for workshops

**EventStorming.com**: Dedicated platform; templates for big picture, process, design levels

**AsyncAPI**: Document event schemas, channels, and operations (like OpenAPI for events)

```yaml
asyncapi: "2.0.0"
info:
  title: "Order Service"
  version: "1.0.0"

channels:
  order.events:
    publish:
      message:
        oneOf:
          - $ref: "#/components/messages/OrderPlaced"
          - $ref: "#/components/messages/PaymentProcessed"

components:
  messages:
    OrderPlaced:
      payload:
        type: object
        properties:
          orderId: { type: string }
          customerId: { type: string }
          items: { type: array }
          total: { type: number }
          timestamp: { type: string, format: date-time }
```

---

## See Also
- [Domain-Driven Design](architecture-ddd.md) — Strategic design and bounded contexts
- [Event Sourcing](architecture-event-sourcing.md) — Event store design and projections
- [Event-Driven Patterns](patterns-event-driven.md) — System patterns using events
- [Event-Driven Architecture](architecture-event-driven.md) — Microservices coordination