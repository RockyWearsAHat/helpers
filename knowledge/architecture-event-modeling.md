# Event Modeling — Architecture, Design Discipline, and Implementation Mapping

Event modeling is a design discipline that views systems through the immutable facts of what happened. Unlike data modeling (which represents current state) or behavior modeling (which describes processes), event modeling uses business events as the primary design vocabulary. It bridges analysis, architecture, and implementation.

## Event Modeling vs. Other Approaches

| Approach | Focus | Design Artifact | Best For |
|----------|-------|-----------------|----------|
| **Data Modeling** | Current state | Entity-relationship diagram | Schema design |
| **Process Modeling** | Workflows | Flowchart, swimlanes | Business processes |
| **Behavior Modeling** | Actions | State machine | State transitions |
| **Event Modeling** | Historical facts | Event timeline + commands/projections | End-to-end flows, auditability, evolution |

Event modeling excels at making implicit business rules explicit, preventing communication gaps between domain experts and engineers.

## The Blueprint Format

The core artifact of event modeling is the **event blueprint**: a visual timeline showing the command → event → read model progression for a business flow.

### Components

**1. Commands (Blue)**
- User or system actions that *request* a state change
- Named imperative: *PlaceOrder*, *CancelSubscription*, *ApproveRefund*
- Always happen before events
- Can fail (rejected by domain logic)

**2. Events (Orange)**
- Immutable facts: "this happened"
- Named past tense: *OrderPlaced*, *SubscriptionCancelled*, *RefundApproved*
- Recorded permanently (audit trail)
- Cannot be deleted, only corrected with compensating events

**3. Read Models (Green)**
- Queryable views derived from events
- Answer specific questions: "What's the order status?" "How much inventory do we have?"
- Optimized for reads, not writes
- Rebuilt when events arrive

**4. Timeline**
- Horizontal axis = time (left to right)
- Shows decision points, alternative paths, parallel processes
- Exposes waiting periods and dependencies

### Example: E-Commerce Order Fulfillment

```
[Command] → [Event] → [Read Model]

[PlaceOrder] → [OrderPlaced] → {Order Status} = "confirmed"
                               → {Order Items} added
                               → {Customer Orders} list updated

[CheckInventory] → [InventoryChecked] → {Inventory} = reserved amount
                 ↓ (if insufficient)
              [InsufficientInventory] → {Inventory Status} = "backorder"

[ChargePayment] → [PaymentProcessed] → {Account Balance} = updated
              or [PaymentFailed] → {Order Status} = "payment_failed"
                                → {Failed Charge Log} recorded

[ShipOrder] → [OrderShipped] → {Order Status} = "shipped"
                             → {Tracking Events} = timestamp + location
                             → {Shipment Timeline} = updated

[DeliveryAttempt] → [DeliverySucceeded] → {Order Status} = "delivered"
                 or [DeliveryFailed] → {Failed Delivery Log} recorded
                 → [Retry Scheduled] → {Next Attempt} = tomorrow
```

The blueprint makes visible:
- Which events always follow commands
- When events cause new commands (triggered events)
- What information each read model needs
- Where systems interact (waiting for external responses)

## Given-When-Then Structure

Event modeling pairs with **Given-When-Then** scenarios describing behavior:

```
Given: An order for 5 units when inventory has only 3
When: PlaceOrder command arrives
Then: OrderPlaced event includes quantity, but InventoryShortageFlagged event emitted
  and Order Status read model shows "awaiting_restock"
```

Each Given-When-Then becomes a test and a specification:
- **Given** = initial state (derived from previous events)
- **When** = command received
- **Then** = event(s) emitted and read models updated

Scenarios expose edge cases and business rules naturally.

## Information Completeness

Events must contain all information needed for:
1. **Audit** — reconstruct what happened (history)
2. **Projections** — rebuild read models if needed
3. **Side effects** — downstream systems know what to do

```python
# Incomplete: no context
class PaymentProcessed:
    amount: float
    timestamp: datetime

# Complete: auditable and actionable
class PaymentProcessed:
    payment_id: str
    order_id: str
    customer_id: str
    amount: Money
    currency: str
    processor: str  # "stripe", "paypal", etc.
    processor_transaction_id: str
    timestamp: datetime
    metadata: dict  # for extensions
```

Every field answers: "Would I need to know this to handle side effects or debug?"

## Command-Event Pairs and Invariants

Not every command succeeds. The relationship between commands and events encodes **business invariants**.

```
[Command] ↘︎
          → (validation) →
[Event]   ↙︎
[Rejection]

[CancelOrder] → {Order.status in ["pending", "confirmed"]}?
             → Yes: [OrderCancelled]
             → No: [CancellationRejected]
```

Invariants become explicit in the blueprint:

- *CreateAccount* → *AccountCreated* only if email not already registered
- *WithdrawFunds* → *FundsWithdrawn* only if balance ≥ amount (else *WithdrawalFailed*)
- *ApproveRefund* → *RefundApproved* only if original transaction was charged

This clarity prevents bugs: business rules are obvious, not buried in conditional logic.

## Read Models and Projections

A **read model** is a cached view of event data, optimized for queries. Event sourcing typically uses event handlers to build read models asynchronously.

```
┌─────────────────────────────────────┐
│ Event Stream (immutable)            │
│ [OrderPlaced] [ItemAdded]           │
│ [PaymentProcessed] [OrderShipped]   │
└──────────────↓─────────────────────┘
               │
      ┌────────┴─────────┬─────────┐
      ↓                   ↓         ↓
   [Order Status]  [Customer   [Revenue
    Projection]     Orders]     Totals]
    
"What's the    "All orders   "Sales by
 status of      for customer  region +
 order #123?"   #456?"        product?"
```

Each read model answers a specific question. Multiple projections from the same event stream serve different access patterns.

**Projection strategies:**

1. **Sync projection** — Update read models during event application (simplest, highest latency)
2. **Async projection** — Event handler queues work; separate worker updates read models (higher throughput, eventual consistency)
3. **CQRS** — Separate read and write databases; read replicas updated via message bus (best scale, complex)

## Event Timeline and Causality

The event timeline reveals dependencies and bottlenecks:

```
Time →

Customer    T1: [SearchProduct]
App         T2: [AddToCart]
            T3: [Checkout]
            T4: [PlaceOrder]
                    ↓
Payment     T5: [ProcessPayment] (waits for T4)
System           (can take 2-5s)
                    ↓ success
Fulfillment T6: [ReserveInventory]
System      T7: [PrintLabel]
                ↓
Warehouse   T8: [ShipOrder] (manual, often delayed)
```

Timelines expose:
- **Critical path** — events that block downstream progress
- **Parallel events** — independent; can optimize
- **Wait states** — where the system is idle (integration delays, manual work)
- **Cascading failures** — one delay ripples downstream

This guides architecture: move slow operations off the critical path, parallelize where possible, use events to decouple. A timeline showing Checkout → PlaceOrder → PaymentProcessing sequential might suggest async payment processing instead.

## Event Storming to Event Modeling

**Event storming** is a workshop technique for discovering events collaboratively. **Event modeling** formalizes storming findings into design artifacts.

### Storming Phase (1-2 days)
- Participants (domain experts, developers, stakeholders) gather
- Sticky notes: orange for events, blue for commands, green for actors, pink for policy
- Build timeline of events left-to-right
- Discuss hot-spots and unknowns

### Modeling Phase (post-workshop, 1-2 weeks)
- Formalize events with complete schemas
- Define read models each event affects
- Write Given-When-Then scenarios
- Map to implementation (aggregates, projections, side effect handlers)
- Review with stakeholders for validation

The output: a detailed, executable specification.

## Implementation Mapping

Event models must map to actual code: aggregates, events, handlers, projections.

### DDD Aggregate Example

```python
class Order(Aggregate):
    def __init__(self, order_id, customer_id):
        self.id = order_id
        self.customer_id = customer_id
        self.items: list[OrderItem] = []
        self.status = "pending"
        self.events: list[Event] = []
    
    def place_order(self, items: list[OrderItem]) -> None:
        if not items:
            raise ValueError("Order must have items")
        
        self.items = items
        self.status = "confirmed"
        
        # Event emitted
        self.events.append(OrderPlaced(
            order_id=self.id,
            customer_id=self.customer_id,
            items=items,
            timestamp=now()
        ))
    
    def cancel(self) -> None:
        if self.status not in ["pending", "confirmed"]:
            raise InvalidStateError(f"Cannot cancel {self.status} order")
        
        self.status = "cancelled"
        self.events.append(OrderCancelled(
            order_id=self.id,
            reason="customer_request",
            timestamp=now()
        ))
```

### Projection Example

```python
class OrderStatusProjection:
    def __init__(self, db):
        self.db = db
    
    def handle(self, event: Event):
        if isinstance(event, OrderPlaced):
            self.db.insert("order_status", {
                "order_id": event.order_id,
                "status": "confirmed",
                "items_count": len(event.items),
                "created_at": event.timestamp
            })
        
        elif isinstance(event, OrderCancelled):
            self.db.update("order_status",
                where={"order_id": event.order_id},
                set={"status": "cancelled"}
            )
```

### Handler Example (Side Effects)

```python
class OrderPlacedHandler:
    def __init__(self, inventory_service, payment_service):
        self.inventory = inventory_service
        self.payment = payment_service
    
    def handle(self, event: OrderPlaced):
        # Triggered by OrderPlaced event
        self.inventory.reserve(event.order_id, event.items)
        self.payment.charge(event.customer_id, event.total_amount)
```

## When to Use Event Modeling

**Ideal for:**
- Complex domains with many stakeholders (retail, finance, logistics)
- Systems requiring audit trails and compliance
- Projects evolving rapidly (explicit rules easier to change)
- Microservices needing clear contracts

**Consider when:**
- Business requirements unclear or frequently changing
- Debugging mysterious state is recurring problem
- Multiple teams maintaining same domain

**Skip if:**
- Simple CRUD with stable requirements
- Real-time constraints (event latency matters)
- Aggregate sizes are massive (event streams grow huge)

Event modeling is an investment upfront. Returns compound: fewer bugs, easier onboarding, natural scalability.

See also: [data-event-modeling.md](data-event-modeling.md), [architecture-event-sourcing.md](architecture-event-sourcing.md), [ddd-domain-events.md](ddd-domain-events.md)