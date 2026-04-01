# DDD Aggregate Design — Roots, Boundaries, Invariants & Consistency

## Fundamentals

An **aggregate** is a cluster of objects bound by consistency rules. The aggregate root is the single entry point for external access. Think of it as a micro-transaction boundary: operations inside an aggregate are atomic; operations across aggregates are eventually consistent.

**Why aggregates exist:** Domain models are graphs of interconnected entities and value objects. Without boundaries, a single transaction might touch hundreds of objects (slow, deadlock-prone, violates domain logic). Aggregates enforce a consistency boundary: within the boundary, strong consistency; across boundaries, eventual consistency.

## Aggregate Roots

The **aggregate root** is a designated entity that owns all objects in the aggregate:

- External code holds references **only** to the root
- External code calls methods **only** on the root
- The root is responsible for enforcing invariants
- The root is the only object persisted directly; children are saved via the root

```java
// Order aggregate
public class Order {  // Root
  private final String orderId;
  private final Customer customer;
  private final List<OrderLine> lines;  // Internal; no external refs
  private final Money totalPrice;
  private OrderStatus status;
  
  public Order(String orderId, Customer customer) { /* ... */ }
  
  // External code calls methods on root only
  public void addLine(Product product, Quantity quantity) {
    if (status != OrderStatus.DRAFT) throw new DomainException();
    lines.add(new OrderLine(product, quantity));
    recalculateTotalPrice();  // Root maintains invariant
  }
  
  public void confirm() {
    if (lines.isEmpty()) throw new DomainException("Cannot confirm empty order");
    if (totalPrice.isNegative()) throw new DomainException();
    status = OrderStatus.CONFIRMED;
  }
}

// OrderLine is NOT an aggregate root
public class OrderLine {
  private final Product product;
  private final Quantity quantity;
  
  public OrderLine(Product product, Quantity quantity) { /* ... */ }
  // External code never instantiates or modifies OrderLine directly
}
```

## Consistency Boundaries

An aggregate enforces **strong consistency**: all invariants must hold when the transaction commits.

Invariants are rules the domain requires:

```
// Order invariant examples:
- An order must have at least one line before confirming
- An order cannot exceed the customer's credit limit
- An order line quantity must be a positive integer
- Total price must equal sum of line prices
```

The root's methods guarantee these invariants:

```java
public class Order {
  public void addLine(Product product, Quantity quantity) {
    // Invariant: quantity must be positive
    if (quantity.isZeroOrNegative()) 
      throw new DomainException("Quantity must be positive");
    
    lines.add(new OrderLine(product, quantity));
    recalculateTotalPrice();  // Restore invariant
  }
  
  public void confirm() {
    // Invariant: cannot confirm empty order
    if (lines.isEmpty()) 
      throw new DomainException("Order must have at least one line");
    
    // Invariant: total price must be valid
    if (totalPrice.isNegative()) 
      throw new DomainException("Total price is negative");
    
    status = OrderStatus.CONFIRMED;
  }
}
```

**Scope of guarantee:** Only within a single transaction. If two clients modify the same aggregate concurrently, optimistic locking or pessimistic locking prevents one from losing updates. The invariant guarantee applies to the serialized transaction, not to concurrent views.

## Aggregate Size

Aggregates should be **small to medium sized**. Common mistakes:

| Mistake                    | Problem                                               | Solution                                |
| -------------------------- | ----------------------------------------------------- | --------------------------------------- |
| **Giant aggregate**        | Entire domain in one root; slow transactions; locks   | Split into multiple aggregates          |
| **Aggregate with all refs** | Loads entire object graph to modify one object        | Use IDs to reference other aggregates   |
| **Circular refs between**  | Difficult to persist and load; violates boundaries    | Yes, circles with IDs are fine          |
| **Lazy loading inside**    | Hidden I/O; breaks single transaction guarantee       | Load children upfront or use separate   |

**Guideline:** An aggregate should change as often as a business transaction. If only part of it changes independently, refactor it into a separate aggregate.

## Cross-Aggregate References

Objects in different aggregates reference each other **by ID only**, not by direct object references:

```java
// CORRECT: Reference by ID
public class Order {
  private final CustomerId customerId;  // ID, not Customer object
  
  // Repository retrieves the Customer separately if needed
}

// INCORRECT: Direct reference across aggregates
public class Order {
  private final Customer customer;  // Violates boundary!
}
```

**Why by ID?**
- Decouples loading: Order and Customer can be persisted independently
- Prevents lazy-loading surprises: accessing `order.customer.email` doesn't trigger hidden DB queries
- Enables eventual consistency: two aggregates can be out of sync temporarily
- Supports distributed systems: aggregates can live in different services/databases

**Loading cross-aggregate data:**

```java
// Service/application layer coordinates
public class PlaceOrderUseCase {
  public void execute(String orderId, String customerId) {
    Order order = orderRepository.findById(orderId);
    Customer customer = customerRepository.findById(customerId);
    
    // Do not pass customer into order aggregate
    // Instead, value objects or DTOs cross boundaries
    order.confirm(customer.creditLimit);  // Pass data as values
  }
}
```

## Aggregate Lifecycle

Aggregates transition through states driven by domain events:

```
┌─────────────┐
│   Created   │
└──────┬──────┘
       │ (constructor or factory)
       ▼
┌─────────────┐
│   Modified  │ ◄─── addLine, removeDiscount, etc.
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Persisted  │ ◄─── Repository.save()
└──────┬──────┘
       │
       ├─► Archived / Deleted
       │
       └─► Events emitted to event bus
```

Change methods return domain events; the repository/application layer publishes them:

```java
public class Order {
  public List<DomainEvent> confirm() {
    if (/* invariant violation */) throw exception;
    status = OrderStatus.CONFIRMED;
    return List.of(new OrderConfirmed(orderId, timestamp));
  }
}

// Application layer
public void confirmOrder(String orderId) {
  Order order = repository.findById(orderId);
  List<DomainEvent> events = order.confirm();
  repository.save(order);
  eventBus.publish(events);  // Async handlers react
}
```

## Event Sourcing with Aggregates

With event sourcing, aggregates are reconstructed from event streams instead of current state:

```
Events (past):
  OrderCreated(id=123, customerId="C1", timestamp=T1)
  OrderLineAdded(orderId=123, product="Widget", qty=5, timestamp=T2)
  OrderLineAdded(orderId=123, product="Gadget", qty=3, timestamp=T3)
  OrderConfirmed(orderId=123, timestamp=T4)

Replay:
  1. Start with empty Order(orderId=123)
  2. Apply OrderCreated → init fields
  3. Apply OrderLineAdded twice → populate lines
  4. Apply OrderConfirmed → status = CONFIRMED

Result: Order aggregate fully reconstructed
```

Aggregate roots define `apply()` methods for each event:

```java
public class Order {
  private List<DomainEvent> uncommittedEvents = new ArrayList<>();
  
  public void applyOrderCreated(OrderCreated event) {
    this.orderId = event.orderId;
    this.customerId = event.customerId;
    this.status = OrderStatus.DRAFT;
  }
  
  public void applyOrderLineAdded(OrderLineAdded event) {
    lines.add(new OrderLine(event.product, event.quantity));
    recalculateTotalPrice();
  }
  
  public void applyOrderConfirmed(OrderConfirmed event) {
    status = OrderStatus.CONFIRMED;
  }
  
  public void raise(DomainEvent event) {
    apply(event);  // Apply to in-memory state
    uncommittedEvents.add(event);  // Track for persistence
  }
}
```

See also: [event sourcing practice](paradigm-event-sourcing-practice.md), [domain events](architecture-domain-events.md), [CQRS](architecture-cqrs.md)