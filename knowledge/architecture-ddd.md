# Domain-Driven Design

## Strategic Design

### Bounded Contexts

A bounded context is a linguistic and model boundary. The same word ("Account") means different things in different contexts:

| Context  | "Account" Means                              |
| -------- | -------------------------------------------- |
| Banking  | Financial account with balance, transactions |
| Identity | User login credentials, profile              |
| CRM      | Customer record, contact information         |

Each bounded context has its own model, its own ubiquitous language, and ideally its own codebase/service. Models don't leak across boundaries.

### Context Mapping

Relationships between bounded contexts:

| Pattern                   | Relationship                                    | When to Use                                                    |
| ------------------------- | ----------------------------------------------- | -------------------------------------------------------------- |
| **Shared Kernel**         | Two contexts share a small common model         | Tightly collaborating teams, shared core types                 |
| **Anti-Corruption Layer** | Translates between external and internal models | Protecting your model from a legacy or external system         |
| **Open Host Service**     | Published API with well-defined protocol        | Serving many consumers with a stable interface                 |
| **Published Language**    | Shared schema (Avro, Protobuf, OpenAPI)         | Standard data exchange format between contexts                 |
| **Conformist**            | Downstream adopts upstream's model as-is        | When upstream won't change and fighting isn't worth it         |
| **Customer-Supplier**     | Upstream serves downstream's needs              | Two teams where downstream can influence upstream's priorities |
| **Partnership**           | Two contexts evolve together                    | Two teams coordinating closely on shared goals                 |
| **Separate Ways**         | No integration                                  | When the cost of integration exceeds the benefit               |

### Context Map Visualization

```
┌──────────────┐    ACL      ┌──────────────┐
│   Ordering   │←────────────│   Shipping   │
│   Context    │             │   Context    │
└──────┬───────┘             └──────────────┘
       │ OHS/PL
       ↓
┌──────────────┐  Conformist ┌──────────────┐
│   Billing    │────────────→│   Payment    │
│   Context    │             │   Gateway    │
└──────────────┘             └──────────────┘
```

## Tactical Design

### Entities

Objects defined by identity, not attributes. Two customers with the same name are different entities.

```python
class Order:
    def __init__(self, order_id: OrderId):
        self._id = order_id  # Identity
        self._items: list[OrderItem] = []
        self._status = OrderStatus.DRAFT

    @property
    def id(self) -> OrderId:
        return self._id

    def __eq__(self, other):
        return isinstance(other, Order) and self._id == other._id
```

### Value Objects

Defined by attributes, not identity. Immutable. Two `Money(100, "USD")` are the same.

```python
@dataclass(frozen=True)
class Money:
    amount: Decimal
    currency: str

    def add(self, other: "Money") -> "Money":
        if self.currency != other.currency:
            raise ValueError("Cannot add different currencies")
        return Money(self.amount + other.amount, self.currency)

    def __post_init__(self):
        if self.amount < 0:
            raise ValueError("Amount cannot be negative")
```

**Prefer value objects over primitives** (Money over float, EmailAddress over str, OrderId over UUID). They carry validation and domain meaning.

### Aggregates

A cluster of entities and value objects with a consistency boundary. One entity is the **aggregate root** — all access goes through it.

```python
class Order:  # Aggregate root
    def add_item(self, product_id: ProductId, quantity: int, price: Money):
        if self._status != OrderStatus.DRAFT:
            raise DomainError("Cannot modify a submitted order")
        item = OrderItem(product_id, quantity, price)
        self._items.append(item)
        self._updated_at = datetime.utcnow()

    def submit(self) -> list[DomainEvent]:
        if not self._items:
            raise DomainError("Cannot submit empty order")
        self._status = OrderStatus.SUBMITTED
        return [OrderSubmitted(self._id, self.total, self._items)]
```

### Aggregate Design Rules

1. **Reference other aggregates by ID**, not by object reference
2. **Keep aggregates small** — protect a single consistency boundary
3. **Use eventual consistency between aggregates** — don't try to update two aggregates in one transaction
4. **One aggregate per transaction** — if you're updating multiple, your boundaries are wrong

```python
class Order:
    customer_id: CustomerId  # Reference by ID, not Customer object
    items: list[OrderItem]   # OrderItem is inside the aggregate boundary

# Tightly coupled: Order holds a reference to Customer aggregate
# Loosely coupled: Order stores customer_id and queries Customer separately
```

### Repositories

Abstraction over data access. One repository per aggregate root. Returns and persists whole aggregates.

```python
class OrderRepository(Protocol):
    def find_by_id(self, order_id: OrderId) -> Order | None: ...
    def save(self, order: Order) -> None: ...
    def next_id(self) -> OrderId: ...

# Infrastructure implementation
class SqlOrderRepository:
    def find_by_id(self, order_id: OrderId) -> Order | None:
        row = self.session.query(OrderRow).get(str(order_id))
        return self._to_domain(row) if row else None

    def save(self, order: Order) -> None:
        row = self._to_row(order)
        self.session.merge(row)
```

### Domain Services

Operations that don't naturally belong to a single entity or value object. Stateless. Named after domain operations.

```python
class PricingService:
    def calculate_discount(self, order: Order, customer: Customer) -> Money:
        """Cross-aggregate business logic that doesn't belong in either aggregate."""
        if customer.is_premium and order.total > Money(100, "USD"):
            return order.total.multiply(Decimal("0.10"))
        return Money(0, order.total.currency)
```

**Not a domain service**: Application services (orchestration), infrastructure services (email sending). Domain services contain pure domain logic.

### Domain Events

Record meaningful things that happened in the domain:

```python
@dataclass(frozen=True)
class OrderSubmitted:
    order_id: OrderId
    total: Money
    item_count: int
    occurred_at: datetime = field(default_factory=datetime.utcnow)
```

Aggregates return domain events from command methods. The application layer dispatches them.

```python
# Application service
def submit_order(self, order_id: OrderId):
    order = self.order_repo.find_by_id(order_id)
    events = order.submit()       # Domain logic returns events
    self.order_repo.save(order)
    for event in events:
        self.event_bus.publish(event)  # Application layer dispatches
```

### Factories

Encapsulate complex aggregate creation:

```python
class OrderFactory:
    def create_from_cart(self, cart: ShoppingCart, customer_id: CustomerId) -> Order:
        order = Order(self.id_generator.next(), customer_id)
        for cart_item in cart.items:
            order.add_item(cart_item.product_id, cart_item.quantity, cart_item.price)
        return order
```

## Ubiquitous Language

The language of the domain — used by developers, domain experts, in code, documentation, and conversation. If the code says `Order.submit()` but the business says "confirm the order," someone is wrong.

Rules:

- Code uses domain terms: `submitOrder()` not `processData()`
- Method names reflect business operations: `order.cancel()` not `order.setStatus(CANCELLED)`
- No technical jargon in the domain model: No `OrderDTO`, no `OrderManager`
- Glossary maintained and referenced: Disagreements about terms are design discussions

## Event Storming

A workshop technique for discovering domain events, commands, aggregates, and bounded contexts:

### Process

1. **Domain events** (orange stickies): Brainstorm everything that happens. Past tense: "Order Placed", "Payment Received"
2. **Commands** (blue): What triggers each event? "Place Order", "Process Payment"
3. **Aggregates** (yellow): What entity handles each command?
4. **Bounded contexts**: Group related aggregates. Gaps and overlaps reveal boundaries
5. **Policies** (lilac): Automation rules — "When Order Placed, then Reserve Inventory"
6. **Read models** (green): What information does someone need to issue a command?

### Event Storming to Code

```
Sticky: "Order Placed" (event) ← "Place Order" (command) ← Order (aggregate)

↓ Translates to:

class Order:
    def place(self, items, customer_id) -> list[OrderPlaced]:
        # validate, enforce invariants
        return [OrderPlaced(self.id, items, customer_id)]
```

## Anti-Patterns

### Anemic Domain Model

Entities are just data bags with getters/setters. All logic lives in service classes. This is procedural programming disguised as OOP.

```python
# ANEMIC (bad)
class Order:
    id: str
    status: str
    items: list

class OrderService:
    def submit(self, order):
        if not order.items:
            raise Error("empty")
        order.status = "submitted"  # Logic outside the entity

# RICH (good)
class Order:
    def submit(self) -> list[DomainEvent]:
        if not self._items:
            raise DomainError("Cannot submit empty order")
        self._status = OrderStatus.SUBMITTED
        return [OrderSubmitted(self._id)]
```

### God Aggregate

An aggregate that owns too much. Symptoms: large transaction scopes, frequent contention, huge event payloads. Split along consistency boundaries — what actually needs to be atomically consistent?

### Leaking Domain Logic

Domain rules scattered across controllers, services, or infrastructure:

```python
# Validation in the controller (anemic domain)
@app.post("/orders/{id}/submit")
def submit_order(id):
    order = repo.find(id)
    if order.status != "draft":  # Domain logic leaked to controller
        return 400
    if len(order.items) == 0:    # Domain logic leaked to controller
        return 400
    order.status = "submitted"

# Domain logic in the aggregate (rich domain model)
@app.post("/orders/{id}/submit")
def submit_order(id):
    order = repo.find(id)
    events = order.submit()  # All validation inside
    repo.save(order)
```

### Ignoring Bounded Context Boundaries

Using one model everywhere. "Order" in the warehouse doesn't need customer email. "Order" in billing doesn't need shipping address. Shared models become bloated and coupled.
