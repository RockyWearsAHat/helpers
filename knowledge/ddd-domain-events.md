# DDD Domain Events — Design, Publishing, Sourcing & Event-Driven Microservices

## Fundamentals

A **domain event** is a fact about something that happened in the business. It's expressed in past tense, is immutable once published, and represents a point in time when the domain state changed.

**Domain events enable:**
- **Decoupling:** Components react to events without direct calls
- **Auditability:** Complete log of what happened, in order
- **Temporal queries:** "What was the state at time T?" with event sourcing
- **Integration:** Microservices communicate asynchronously via events
- **Traceability:** Trace effects of decisions across the system

Domain events describe business facts, not implementation details.

```
// Domain events (business-relevant)
OrderPlaced
PaymentProcessed
InventoryAllocated
ShipmentDispatched

// NOT domain events (implementation details)
DatabaseQueryExecuted
CacheInvalidated
HttpResponseReceived
```

## Event Design

### Past Tense Naming

Events are immutable historical facts, expressed in past tense:

```java
// CORRECT: Past tense, immutable fact
public class OrderPlaced {
  public final String orderId;
  public final Instant timestamp;
  public final List<OrderLine> items;
  public final Money totalAmount;
}

// INCORRECT: Imperative, suggests action
public class PlaceOrder {  // Do NOT use
  public String orderId;
  public List<OrderLine> items;
}

// INCORRECT: Conditional/imperative (something to be done)
public class ShouldConfirmOrder { }  // Future tense implies uncertainty
```

**Why?** An event already happened. It cannot be prevented or cancelled. "OrderPlaced" is fact; "PlaceOrder" is a command that might fail.

### Event Payload

Include sufficient context for subscribers to understand the event:

```java
public class OrderPlaced {
  public final String orderId;           // What was affected
  public final String customerId;        // Who did it
  public final List<OrderLine> items;    // What changed
  public final Money totalAmount;        // Computed state
  public final Instant occurredAt;       // When
  public final String correlationId;     // Track across services
  public final String causationId;       // Trace cause (command ID)
  
  public OrderPlaced(String orderId, String customerId, List<OrderLine> items,
                     Money totalAmount, Instant occurredAt, 
                     String correlationId, String causationId) {
    this.orderId = orderId;
    this.customerId = customerId;
    this.items = List.copyOf(items);     // Defensive copy; immutability
    this.totalAmount = totalAmount;
    this.occurredAt = occurredAt;
    this.correlationId = correlationId;
    this.causationId = causationId;
  }
}
```

**Metadata fields:**
- `correlationId`: Unique ID for the user request; followed across all services and events
- `causationId`: ID of the command or event that caused this event
- `occurredAt`: Timestamp business logic says the event occurred (not system time)
- `source`: Which service/aggregate published this event

### Immutability

Events cannot change after publishing:

```java
// CORRECT: Truly immutable
public final class OrderPlaced {
  public final String orderId;
  public final List<OrderLine> items;
  
  public OrderPlaced(String orderId, List<OrderLine> items) {
    this.orderId = orderId;
    this.items = Collections.unmodifiableList(new ArrayList<>(items));
  }
}

// INCORRECT: Mutable event (violates semantics)
public class OrderPlaced {
  public String orderId;
  public List<OrderLine> items;  // Modifiable after publication!
}
```

Subscribers might cache events, replay them, or process them asynchronously. Mutating an event breaks all these patterns.

## Event Dispatching

### Publishing from Aggregates

Aggregates generate domain events when they change. The aggregate collects events during a transaction; the application layer publishes them:

```java
public class Order {
  private final List<DomainEvent> uncommittedEvents = new ArrayList<>();
  
  public void confirm() {
    if (lines.isEmpty()) throw new DomainException();
    status = OrderStatus.CONFIRMED;
    
    // Raise event
    uncommittedEvents.add(new OrderConfirmed(
      orderId, customerId, totalAmount, Instant.now(), correlationId, causationId
    ));
  }
  
  public List<DomainEvent> getUncommittedEvents() {
    return List.copyOf(uncommittedEvents);
  }
  
  public void markEventsAsCommitted() {
    uncommittedEvents.clear();
  }
}

// Application layer
public class ConfirmOrderUseCase {
  public void execute(String orderId, String correlationId) {
    Order order = repository.findById(orderId);
    order.confirm();
    
    // Persist aggregate
    repository.save(order);
    
    // Get and publish events
    List<DomainEvent> events = order.getUncommittedEvents();
    eventBus.publish(events);
    
    order.markEventsAsCommitted();
  }
}
```

### Event Bus and Handlers

An event bus routes published events to subscribers:

```java
// Event handler (subscriber)
@Component
public class SendOrderConfirmationEmail {
  
  @EventListener(OrderConfirmed.class)
  public void handle(OrderConfirmed event) {
    Customer customer = customerRepository.findById(event.customerId);
    emailService.sendConfirmation(customer.email, event.orderId);
  }
}

// Another handler
@Component
public class ReserveInventory {
  
  @EventListener(OrderConfirmed.class)
  public void handle(OrderConfirmed event) {
    for (OrderLine line : event.items) {
      inventoryService.reserve(line.productId, line.quantity);
    }
  }
}

// Event bus distributes
eventBus.publish(new OrderConfirmed(...));
// Both handlers called; order independent
```

**Publish-subscribe decoupling:**

```
Order aggregate ──> Publishes OrderConfirmed
                        │
                        ├──> SendOrderConfirmationEmail handler
                        ├──> ReserveInventory handler
                        └──> UpdateOrderAnalytics handler

Each handler:
- Runs independently (can fail without blocking others)
- Can be async (listener in separate thread/process)
- Doesn't exist at compile time (loosely coupled)
```

## Event Sourcing vs. Domain Events

**Event Sourcing:** Persistence technique. State is reconstructed by replaying all events.

**Domain Events:** Business semantics. Events represent facts about what happened.

These are orthogonal concepts:

| Pattern                | Use Domain Events | Use Event Sourcing |
| ---------------------- | :-----------:     | :-----------:      |
| **Traditional CRUD**   | Yes               | No                 |
| **With event bus**     | Yes               | No                 |
| **Pure event sourcing**| Yes               | Yes                |
| **CQRS read model**    | Yes (yes)         | Yes (for writes)   |

**Traditional approach with domain events:**

```
Order aggregate changes
    ↓
Domain events raised
    ↓
Events published to bus
    ↓
Handlers react (send email, reserve inventory, etc.)
    ↓
Order saved (current state only) in DB
    ↓
Events discarded after publication
```

**Event sourcing approach:**

```
Order aggregate changes
    ↓
Domain events raised
    ↓
Events appended to event store
    ↓
Projections updated (eventual consistency)
    ↓
Handlers react (cross-service communication)
    ↓
Event stream is the authoritative source of truth
```

See: [event sourcing practice](paradigm-event-sourcing-practice.md)

## Integration Events

When microservices communicate, they exchange **integration events** (or external events). These are domain events published across service boundaries:

```java
// Service A: Order Service
public class OrderPlaced {  // Domain event
  public final String orderId;
  public final String customerId;
  public final Money totalAmount;
  public final Instant occurredAt;
  public final String correlationId;
}

// Service A publishes to central event bus/broker (Kafka, RabbitMQ, etc.)
eventBus.publish(new OrderPlaced(...));

// Service B: Inventory Service (different codebase, different DB)
@EventListener("order-service.OrderPlaced")
public void reserveInventory(OrderPlaced event) {
  // Deserialize from message bus
  for (OrderLine line : event.items) {
    inventoryService.reserve(line.productId, line.quantity);
  }
  
  // Publish its own integration event
  eventBus.publish(new InventoryReserved(event.orderId, event.correlationId));
}

// Service C: Shipping Service
@EventListener("order-service.OrderPlaced")
public void prepareShipment(OrderPlaced event) {
  shipmentService.createShipment(event.orderId, event.customerId);
}
```

**Challenges with integration events:**

| Challenge                       | Mitigation                                   |
| ------------------------------- | -------------------------------------------- |
| **Eventual consistency**        | Accept delays; design for idempotence        |
| **Lost events**                 | Persist events before publishing              |
| **Duplicate delivery**          | Handlers must be idempotent                  |
| **Version skew**                | Use versioning; handle old events gracefully |
| **Schema evolution**            | Additive fields only (backward compatible)   |

## Event Versioning

Events evolve over time. Use versioning to handle changes:

```java
// V1 (original)
public class OrderPlaced_v1 {
  public final String orderId;
  public final String customerId;
  public final Money totalAmount;
}

// V2 (new field added)
public class OrderPlaced_v2 {
  public final String orderId;
  public final String customerId;
  public final Money totalAmount;
  public final String shippingAddress;  // New field
}

// Event store tracks version
{
  "type": "OrderPlaced",
  "version": 2,
  "data": {
    "orderId": "O123",
    "customerId": "C456",
    "totalAmount": 99.99,
    "shippingAddress": "123 Main St"
  }
}

// Deserializer handles both versions
public OrderPlaced deserialize(Map<String, Object> data, int version) {
  if (version == 1) {
    // V1 had no shippingAddress; use default
    return new OrderPlaced(data.get("orderId"), data.get("customerId"), 
                           data.get("totalAmount"), "UNKNOWN");
  } else if (version == 2) {
    return new OrderPlaced(data.get("orderId"), data.get("customerId"),
                           data.get("totalAmount"), data.get("shippingAddress"));
  }
}
```

**Versioning rules:**
- Add fields only; never remove or rename
- Removed fields are handled by default values
- Use `@Deprecated` with explanation for old versions
- Test old events can be deserialized to current version

## Event Storming

**Event storming** is a collaborative modeling technique to discover domain events and understand workflows:

**Process:**
1. **Orange sticky notes:** Events (past tense: OrderPlaced, PaymentProcessed, etc.)
2. **Blue sticky notes:** Commands (PlaceOrder, ProcessPayment)
3. **Yellow sticky notes:** Aggregates (Order, Payment, Inventory)
4. **Red sticky notes:** Hotspots/problems (contradictions, unclear flow)
5. **Green sticky notes:** Policies (automatic reactions: "When OrderPlaced, send confirmation email")

**Example output (timeline):**
```
Command: CreateOrder
    ↓
Aggregate: Order
    ↓
Event: OrderCreated
    ↓
Event: OrderLineAdded (×N)
    ↓
Event: OrderConfirmed
    ├──> Policy: Send confirmation email
    └──> Event: InventoryReserved (in Inventory aggregate)
```

Benefits: Reveals missing events, clarifies dependencies, identifies bounded contexts.

## Event-Driven Microservices

In event-driven architectures, services communicate via events:

```
Service A (Orders)
    │ Publishes OrderPlaced
    ├────────────────────────┬─────────────────────┬──────────────────┐
    │                        │                     │                  │
    ▼                        ▼                     ▼                  ▼
Service B            Service C                Service D         Message queue
(Billing)        (Inventory)              (Shipping)         (event store)
    │                    │                     │
    └──────────┬─────────┴─────────┬──────────┘
               │                   │
               ▼                   ▼
            Events                Events
           (async,           (may replay,
            fanout)           may fail)

Service C publishes InventoryReserved
    │
    ├──> Service A reacts (updates order status)
    └──> Service D reacts (prepares shipment)
```

**Benefits:**
- Loose coupling: services don't call each other directly
- Scalability: async processing; back pressure via queues
- Resilience: one service slow doesn't block others
- Auditability: event log is permanent record

**Challenges:**
- Distributed debugging (trace correlation IDs)
- Eventual consistency (temporary inconsistency is normal)
- Replay complexity (idempotence required)

See also: [event-driven architecture](architecture-event-driven.md), [CQRS](architecture-cqrs.md), [aggregates](ddd-aggregate-design.md)