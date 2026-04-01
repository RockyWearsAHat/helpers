# Saga Pattern — Distributed Transactions & Compensation

## Overview

A **saga** is a distributed transaction expressed as a sequence of compensating transactions, each with an associated compensation action. When a step fails, previously completed steps are undone via compensation.

Instead of ACID transactions locked across a database, sagas model long-running business processes where:
- Each step is a local transaction in one service
- If any step fails, prior steps are compensated (rolled back)
- No global lock; eventual consistency replaces atomicity

**When sagas matter:** Multi-service workflows (order → payment → inventory → shipment) where atomic isolation across databases is impossible.

---

## Two Models: Choreography vs. Orchestration

### Choreography: Event-Driven, Peer-to-Peer

Each service publishes events when steps complete. Other services subscribe and react. No central coordinator.

```
OrderService publishes "OrderCreated"
    ↓
PaymentService subscribes, charges card, publishes "PaymentProcessed"
    ↓
InventoryService subscribes, reserves stock, publishes "StockReserved"
    ↓
ShippingService subscribes, creates shipment, publishes "ShipmentCreated"

If InventoryService fails (stock unavailable):
    ↓
InventoryService publishes "StockReservationFailed"
    ↓
PaymentService subscribes, refunds card (compensation)
    ↓
OrderService subscribes, cancels order
```

**Pros:**
- Decentralized; no bottleneck coordinator
- Services remain loosely coupled
- Easy to add new steps (just subscribe to events)

**Cons:**
- Hard to see the flow (implicit control); distributed debugging is painful
- Requires careful event naming and versioning
- Compensation logic is scattered across services
- Risk of infinite loops if compensation triggers cascading failures
- Poor auditability of the full saga's current state

### Orchestration: Centralized Coordinator

A **saga execution coordinator** (SEC) drives the flow. When a step completes, the SEC tells the next service what to do. On failure, SEC triggers compensations.

```
SagaCoordinator receives OrderRequest
    ↓
"Call PaymentService.charge, if success → go to InventoryService"
    ↓
PaymentService.charge() → success → event published
    ↓
SagaCoordinator receives success, "Call InventoryService.reserve"
    ↓
InventoryService.reserve() → failure → event published
    ↓
SagaCoordinator receives failure, "Compensate PaymentService.refund"
    ↓
PaymentService.refund() → success
```

Often expressed as a **state machine** (stored in a database or in-memory):

| State             | Action                    | On Success           | On Failure              |
| ----------------- | ------------------------- | -------------------- | ----------------------- |
| PENDING           | Call PaymentService       | → PAYMENT_COMPLETED  | → COMPENSATING         |
| PAYMENT_COMPLETED | Call InventoryService    | → INVENTORY_RESERVED | → COMPENSATING         |
| INVENTORY_RESERVED| Call ShippingService     | → SHIPMENT_CREATED   | → COMPENSATING         |
| SHIPMENT_CREATED  | Saga complete            | ✓ Done               | —                       |
| COMPENSATING      | Undo prior steps (order)  | —                    | → FAILED               |

**Pros:**
- Explicit control flow; easy to understand and audit
- Central view of saga state simplifies monitoring
- Compensation logic is centralized and orchestrated
- Can make conditional decisions based on outcomes

**Cons:**
- Coordinator becomes a bottleneck or single point of failure
- Tighter coupling: coordinator knows all service interfaces
- Adding new services to the saga requires coordinator changes
- Coordinator state must be persisted for fault tolerance

---

## Compensation Transactions

Compensation is not the inverse of the original transaction—it's a corrective action.

| Original                  | Compensation                 | Note                                                      |
| ------------------------- | ---------------------------- | --------------------------------------------------------- |
| Create order              | Cancel order                 | Mark as cancelled; don't delete (audit trail)            |
| Charge credit card        | Refund card                  | Refund may post after charge due to settlement lag       |
| Reserve inventory         | Release reservation          | Update inventory availability                            |
| Create shipment trigger   | Cancel shipment              | Depends on shipment state; may not be reversible         |

**Key insight:** Compensation can fail or be partial. If a refund fails, you have a customer with a charge but no product — now you need escalation procedures, not more compensations. Sagas don't solve this; they make it visible and structured.

---

## Semantic Lock Pattern

A service performing a preliminary action acquires a lock that hints to other clients what's in progress, without fully locking the resource.

**Example:** PaymentService creates an order at `status: PAY_RESERVED` (not yet charged, but payment is being processed). Other clients can see the order exists but is in flux. If payment fails, compensations clear the lock's state.

**Benefit:** Prevents concurrent sagas from trampling each other. Doesn't enforce atomicity—it's a visibility signal.

---

## Commutative Retry & Idempotency

Saga steps must be **idempotent**: calling them multiple times is safe.

- PaymentService.charge(orderId, amount) idempotent? Check if we already charged this orderId for this amount. If so, return success.
- InventoryService.reserve(orderId, items) idempotent? If reservation record already exists, return success.

Without idempotency: Network timeout causes coordinator to retry. Service receives duplicate charge requests → double charge.

Idempotency is usually implemented via a **deduplication cache** (Redis, database) keyed on (service, operation, idempotence_key). Entries TTL after success to avoid unbounded memory.

---

## Pivot Transaction

The **pivot** is the step after which compensation is no longer safe or reversible. Beyond the pivot, a saga commits implicitly (enters a state where forward-only roll-forward is required, not compensation).

**Example:**
```
1. Create Order          (reversible)
2. Charge Card           (reversible via refund)
3. PIVOT: Shipment dispatched  (partially reversible—physical goods en route)
4. Notify Customer       (not easily reversible; customer already contacted)
```

After the pivot, if step 4 fails, you don't undo step 3 (goods are already shipping). Instead, you notify the customer of the failure and arrange a manual resolution (return shipment, refund).

**Implication:** Design sagas so the pivot occurs as late as possible, and understand what happens after it.

---

## SEC (Saga Execution Coordinator) Properties

A sage execution coordinator in orchestration mode should:

1. **Persistent state** — SEC state (current step, outcomes) stored in a durable log, not memory. On crash, SEC resumes from the last checkpoint.
2. **Idempotent** — Multiple instances of SEC can safely process the same saga; deduplication ensures correctness.
3. **Timeout handling** — If a service doesn't respond, SEC has a timeout and decides to retry, compensate, or escalate.
4. **Compensate in reverse order** — If step 3 fails, undo steps 2, 1 in that order (reverse topological order).
5. **Observability** — Log every step, outcome, and compensation. Make saga state queryable for support and debugging.

---

## Failure Modes & Tradeoffs

| Mode              | Choreography | Orchestration |
| ----------------- | ------------ | ------------- |
| Visibility        | Low          | High          |
| Coupling          | Low          | High          |
| Debugging         | Hard         | Easier        |
| Scaling           | Better       | Bottleneck    |
| Implicit loops    | Yes          | No            |
| Single SPOF       | No           | SEC           |

**Reality:** Most teams start with choreography (simpler mental model, loosely coupled) and hit visibility pain. Then migrate to orchestration (more infrastructure, explicit control, but manageable).

---

## When NOT to Use Sagas

- **Few services, strong consistency needed:** Use a monolith or distributed ACID (expensive but possible with 2PC in some systems).
- **Compensation is impossible:** E.g., "send email" has no meaningful undo. For such steps, accept eventual reconciliation outside the saga framework.
- **Latency-sensitive workflows:** Sagas add latency due to event propagation and confirmation loops. Synchronous RPC is faster.

---

## See Also

- Event-Driven Architecture
- Domain-Driven Design (for saga boundaries aligned with bounded contexts)
- Distributed Transactions (2PC, quorum commit, compare/contrast)
- Architecture Patterns