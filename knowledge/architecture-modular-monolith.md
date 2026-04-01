# Modular Monolith — Structured Monoliths with Module Boundaries

## Overview

A **modular monolith** is a monolithic codebase organized into **explicit modules** with clear boundaries, mimicking microservice organization without distribution overhead.

Instead of:
- A single blob of code (bad monolith)
- Fully distributed microservices (good architecture, expensive operations)

A modular monolith:
- Runs as a single process (one binary, one database initially)
- Divided into modules by domain (e.g., Orders, Payments, Inventory, Shipping)
- Modules have internal APIs and can eventually extract into microservices
- Enables team scaling and independent reasoning without operational complexity

**When it fits:** Growing startups, teams new to distributed systems, systems without hyperscale demands, or projects deciding between monolith and microservices.

---

## Module Boundaries

### Domain Alignment

Modules map to **business domains** or bounded contexts (DDD). A user interface, API handlers, business logic, and data access exist within a module, not scattered.

```
orders/
  ├── api/          (OrderController, OrderDTO)
  ├── domain/       (Order aggregate, OrderService)
  ├── persistence/  (OrderRepository, OrderSchema)
  └── internal/     (order-specific utilities)

payments/
  ├── api/          (PaymentController)
  ├── domain/       (Payment aggregate, PaymentService)
  ├── persistence/  (PaymentRepository)
  └── internal/

inventory/
  [similar structure]
```

**Key rule:** A module owns one **business capability**, not a technical layer. Wrong: `persistence/`, `services/`, `models/` at top level. Right: `orders/`, `payments/`, `catalog/`.

### Shared Kernel

Modules share a small amount of *code (not data). Common types, utilities, base classes.

```
shared/
  ├── domain/       (ValueObject, Entity base classes)
  ├── exceptions/   (BusinessException, ValidationException)
  ├── events/       (DomainEvent interface)
  └── utils/        (serialization, logging)
```

**Boundary:** A module can depend on `shared`, but not on other modules' code. This keeps modules loosely coupled.

```java
// OK: OrderService depends on shared
public class OrderService extends DomainService {
  public Order create(OrderDTO dto) { ... }
}

// NOT OK: OrderService directly uses PaymentService
public class OrderService {
  @Autowired private PaymentService paymentService;  // VIOLATION
}
```

### Internal APIs

Modules publish a **public interface** for cross-module communication. Everything else is private.

```java
// payments/api/PaymentService.java (PUBLIC)
public interface PaymentService {
  PaymentResult charge(PaymentRequest req);
  void refund(String paymentId);
}

// payments/domain/PaymentAuthenticator.java (PRIVATE)
class PaymentAuthenticator {
  // implementation detail, not called from other modules
}
```

**Enforcement:** Use package visibility, Access Control Lists (ACLs), or conventions (prefix private classes with underscore, linters, documentation).

---

## Anti-Corruption Layer

When modules have different models for the same concept, one module translates for the other to prevent the "upstream" module's logic from leaking downstream.

**Example:**
```
Payments context sees: Card(cardNumber, cvv, expiryDate) — PCI concerns
Orders context needs: PaymentMethod(token, last4Digits)
```

The **anti-corruption translator** lives in Orders:

```java
// orders/anticorruption/PaymentDTOTranslator.java
public class PaymentDTOTranslator {
  public PaymentMethod toOrdersModel(Card card) {
    // Mask card; extract token; hide PCI details
    return new PaymentMethod(tokenize(card), card.getLast4());
  }
}
```

**Benefit:** Orders code is clean and only knows about tokens, not raw card data.

---

## Eventual Consistency Within Monolith

Even in a monolith, modules can communicate asynchronously via **domain events**, achieving loose coupling.

```java
public class OrderService {
  public Order create(CreateOrderRequest req) {
    Order order = new Order(req.getItems(), req.getCustomer());
    order.recordEvent(new OrderCreatedEvent(order.getId(), ...));
    orderRepository.save(order);
    
    // Publish events to internal event bus
    eventBus.publishAll(order.getDomainEvents());
  }
}

// Elsewhere (another module, same JVM)
public class InventoryService implements OrderEventListener {
  @EventHandler
  void onOrderCreated(OrderCreatedEvent event) {
    // Async: reserve inventory
    // If fails, raises CompensationNeeded event
  }
}
```

**Outcome:** Modules are decoupled in time (don't block each other) and can fail independently (if InventoryService crashes, Orders still saved).

---

## Database-Per-Module Patterns

### Shared Database (Common Starting Point)

All modules write to the same monolithic database. Simple but couples schema. A schema change in one module can ripple.

```
monolith_db
  ├── orders_orders (Orders module)
  ├── orders_order_items
  ├── payments_charges (Payments module)
  ├── inventory_stock (Inventory module)
```

**Trade-off:** Fast local queries (`SELECT * FROM orders_* JOIN payments_*`), but schema boundaries are weak. Modules can accidentally bypass APIs and query each other directly.

### Separate Databases (Loose Coupling)

Each module has its own database. Cross-module queries require API calls or denormalization.

```
orders_db
  ├── orders
  ├── order_items

payments_db
  ├── charges
  ├── refunds

inventory_db
  ├── stock_levels
```

Orders module needs payment status for an order:
```java
// orders/OrderService.java
Order getOrderWithPaymentStatus(String orderId) {
  Order order = orderRepository.find(orderId);
  PaymentStatus status = paymentServiceClient.getStatus(order.paymentId);
  order.paymentStatus = status;  // Denormalized, may be stale
  return order;
}
```

**Trade-off:** Modules are independent (can deploy separately, schema changes isolated), but queries are slower and require denormalization or eventual consistency.

**Transition:** Start with shared database. As modules mature, split databases incrementally using the **strangler fig** pattern: replicate module data to new database, run CDC or dual-write during transition, decommission shared schema.

---

## Module Testing

### Unit Tests

Test business logic (OrderService, PaymentService) in isolation. Mock module dependencies.

```java
@Test
public void testOrderCreation() {
  OrderService svc = new OrderService(mockRepository, mockEventBus);
  Order order = svc.create(new OrderRequest(...));
  assert order.status == OrderStatus.PENDING;
  verify(mockEventBus).publish(any(OrderCreatedEvent.class));
}
```

### Integration Tests

Test module + its persistence layer + internal workflows.

```java
@Test
public void testOrderCreationE2E() {
  Order order = orderService.create(...);
  Order retrieved = orderRepository.find(order.id);  // Actual DB
  assert retrieved.status == OrderStatus.PENDING;
}
```

### System Tests (Contract Tests)

Test module APIs from the perspective of a client module.

```java
@Test
public void testOrderServiceAPIContract() {
  PaymentServiceClient client = new PaymentServiceClient(orderModule.getPaymentAPI());
  PaymentResult result = client.charge(new PaymentRequest(...));
  assert result.success;
}
```

This catches API contract changes before they break other modules.

---

## Extraction to Microservices

As a modular monolith grows, extracting a module into a microservice is straightforward:

1. **Module is isolated.** Has own database schema and APIs. No tight coupling.
2. **Extract code.** Move module code to new service repository.
3. **Replace with RPC.** Module's internal API becomes a network service (HTTP, gRPC).
4. **Deploy separately.** New service runs independently.

```
Before (Monolith):
monolith/
  ├── orders/
  ├── payments/
  ├── inventory/

After (Microservices):
payment-service/  (extracted from monolith, now separate)
monolith/         (orders, inventory remain)
```

**Why the monolith->modular->microservices path works:**
- Modular boundaries are already clear; extraction is mechanical
- Team familiarity; no architectural re-learning
- Can extract one module at a time, testing each transition

---

## Drawbacks & Trade-offs

| Aspect                    | Monolith          | Modular | Microservices |
| ------------------------- | ----------------- | ------- | ------------- |
| Deployment unit           | Whole app         | Whole   | Individual    |
| Team independence         | Low               | Med     | High          |
| Operational complexity    | Low               | Low     | High          |
| Data consistency          | ACID (single DB)  | Eventual| Eventual      |
| Debugging                 | Easy (one process)| Easy    | Hard          |
| Scaling per module        | No; scale all     | No      | Yes           |

**Reality:** A modular monolith stays a monolith if the team doesn't need independent deployment. That's OK—don't extract to microservices for architecture's sake.

---

## Module Organization in Practice

### Maven/Gradle Multi-Module

```
pom.xml (root)
├── shared/pom.xml
├── orders/pom.xml
├── payments/pom.xml
├── monolith/pom.xml (depends on orders, payments, shared)
```

Each module can be versioned, tested, and released independently while still built together.

### Single Monolithic Codebase (by convention)

```
src/
  ├── orders/
  ├── payments/
  ├── inventory/
  ├── shared/
  └── monolith.py (main application, imports orders, payments, ...)
```

Enforce boundaries via linting (e.g., `orders` code cannot import `payments`), documentation, and code review.

---

## See Also

- Domain-Driven Design (bounded contexts, modules as contexts)
- Strangler Fig Pattern (strategy for extracting modules into services)
- Anti-Corruption Layer (protecting modules from upstream changes)
- Event-Driven Architecture (eventual consistency, domain events)
- Architecture Patterns