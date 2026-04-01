# DDD Bounded Context Mapping — Strategic Patterns, Team Topologies & Integration

## Fundamentals

A **bounded context** is a linguistic and organizational boundary within which a consistent ubiquitous language applies. The same concept ("Account") means different things in different contexts.

**Context map** describes relationships between bounded contexts. It answers: How do contexts interact? What are the dependencies? How does data flow?

```
Example: E-commerce company

┌────────────────────┐    ┌────────────────────┐    ┌────────────────────┐
│ Ordering Context   │    │ Billing Context    │    │ Inventory Context  │
│                    │    │                    │    │                    │
│ Concepts:          │    │ Concepts:          │    │ Concepts:          │
│ - Order            │    │ - Invoice          │    │ - Product          │
│ - OrderLine        │    │ - Payment Method   │    │ - Stock            │
│ - Customer         │    │ - Bill / Tax       │    │ - Allocation       │
│                    │    │                    │    │                    │
│ "Customer" =       │    │ "Customer" =       │    │ "Customer" =       │
│ Who ordered        │    │ Bill recipient     │    │ Shipping address   │
└────────────────────┘    └────────────────────┘    └────────────────────┘
        │                        │                         │
        └────────────┬───────────┴─────────────────────────┘
                     │
          Data flows across contexts;
          BUT "Customer" means different things
          in each context.
          Models are separate; translation required.
```

**Strategic design:** Consciously define context boundaries and integration patterns rather than letting architecture emerge chaotically.

## Context Mapping Patterns

Context maps show relationships between teams/contexts. Each relationship has:
- **Direction:** Who is upstream (provider of data/services)? Who is downstream (consumer)?
- **Translation:** Does data translate, or is it passed through?

### Pattern: Shared Kernel

Two teams share a small, carefully managed model:

```
┌─────────────────────────────┐
│   Shared Kernel             │  Common code/types:
│  (shared library)           │  - User (email, name)
│  - User type definition     │  - Permission enum
│  - Permission enum          │  - Role definitions
└─────────────────────────────┘
       ▲            ▲
       │            │
   Shared by both Auth Context
                and User Management Context
```

**When to use:**
- Two closely collaborating teams
- Shared types are stable and infrequent change
- High communication overhead is acceptable

**Pitfalls:**
- Becomes bloated; grows beyond "kernel"
- Tight coupling; changes ripple across teams
- Shared library release cycles complicate deployments

**Example:**
```java
// shared-kernel library (version-controlled together)
public enum UserRole {
  ADMIN, USER, GUEST
}

public class User {
  public final String userId;
  public final String email;
  public final Set<UserRole> roles;
}

// Both contexts import and use
// Auth context uses User to check permissions
// User management context uses User to store profile
```

### Pattern: Open Host Service

Upstream provides a published API; downstream consumes it:

```
┌──────────────────────────────────────────┐
│  Product Catalog Service (Upstream)      │
│                                          │
│  Public API: /products, /inventory       │  Well-defined protocol
│  - RESTful endpoints                     │  - Versioned
│  - Stable contracts                      │  - Documented
└──────────────────────────────────────────┘
       ▲              ▲              ▲
       │              │              │
  Ordering      Recommendation    Analytics
  Service       Service           Service
  (Downstream)  (Downstream)      (Downstream)
```

**When to use:**
- One service has multiple consumers
- Provider and consumers have different priorities
- Consumers need stable, versioned interface

**Example:**
```java
// Upstream: Product Service
@RestController
@RequestMapping("/api/v1/products")
public class ProductApi {
  
  @GetMapping("/{productId}")
  public ProductDto getProduct(@PathVariable String productId) {
    // Stable contract; versioned endpoint
    return new ProductDto(id, name, price, inventory);
  }
}

// Downstream: Ordering Service
public class OrderService {
  
  @Autowired private ProductClient productClient;
  
  public void placeOrder(List<String> productIds) {
    for (String id : productIds) {
      ProductDto product = productClient.getProduct(id);
      // Use product data; rely on stable contract
    }
  }
}
```

**Challenges:**
- API evolves; versioning becomes necessary
- Upstream may not prioritize downstream needs
- Breaking changes require coordination

### Pattern: Published Language

Contexts exchange data using a shared schema (Avro, Protobuf, JSON Schema):

```
┌──────────────────────────────────┐
│  Shared Schema Registry          │
│  - OrderCreated (Avro)           │  Contracts defined
│  - PaymentProcessed (Protobuf)   │  independently
│  - ShipmentDispatched (JSON)     │  of any service
└──────────────────────────────────┘
       ▲         ▲         ▲
       │         │         │
   Order     Payment    Shipping
   Service   Service    Service
```

**When to use:**
- Multiple contexts need to communicate
- Data format is well-understood (e.g., industry standard)
- Services are independent and may be replaced

**Example:**
```protobuf
// shared-schema/order-created.proto
syntax = "proto3";

message OrderCreated {
  string order_id = 1;
  string customer_id = 2;
  int64 total_cents = 3;
  string currency = 4;
  int64 occurred_at_ms = 5;
}

// Message is published to event broker (Kafka)
// All subscribers use same schema for deserialization
```

### Pattern: Anti-Corruption Layer (ACL)

Downstream translates upstream's model so it doesn't infect the downstream domain:

```
┌─────────────────────────────────────┐
│  Legacy System (Upstream)           │
│  Crufty model; strange semantics    │
│  - customer_id (string, inconsistent)
│  - order_stat (enum, 10+ values)    │
└─────────────────────────────────────┘
       │ (messy data)
       │ https://legacyapi.com/orders
       │
       ▼ [ACL Translation Layer]
       │ - Validate input
       │ - Transform to clean model
       │ - Map legacy statuses to domain events
       │
┌─────────────────────────────────────┐
│  New Service (Downstream)           │
│  Clean domain model                 │
│  - OrderId (value object)           │
│  - OrderStatus (enum, 3 values)     │
└─────────────────────────────────────┘
```

**When to use:**
- Integrating with legacy/external systems
- Upstream model is unpredictable or changes frequently
- Protecting domain model from corruption

**Example:**
```java
// Anti-Corruption Layer: translates legacy to clean model
@Component
public class LegacyOrderAdapter {
  
  private final RestTemplate legacyClient;
  
  public Order fetchOrder(String legacyOrderId) {
    // Call legacy API
    LegacyOrderDto dto = legacyClient.getForObject(
      "https://legacy.api/orders/{id}", 
      LegacyOrderDto.class,
      legacyOrderId
    );
    
    // TRANSLATE: Validate and clean
    if (dto.customer_id == null || dto.customer_id.isEmpty()) {
      throw new InvalidOrderException("Legacy order missing customer");
    }
    
    // Map legacy enum to domain enum
    OrderStatus status = mapLegacyStatus(dto.order_stat);
    
    // Construct clean domain model
    return new Order(
      new OrderId(legacyOrderId),
      new CustomerId(dto.customer_id),
      status,
      Money.of(dto.amount / 100, USD)
    );
  }
  
  private OrderStatus mapLegacyStatus(String legacyStatus) {
    return switch(legacyStatus) {
      case "ST_NEW" -> OrderStatus.PENDING;
      case "ST_PROCESSING", "ST_WAITING" -> OrderStatus.CONFIRMED;
      case "ST_SHIPPED" -> OrderStatus.SHIPPED;
      case "ST_DELIVERED" -> OrderStatus.DELIVERED;
      case "ST_CANCELLED", "ST_FAILED" -> OrderStatus.CANCELLED;
      default -> throw new UnknownOrderStatusException(legacyStatus);
    };
  }
}
```

### Pattern: Conformist

Downstream adopts upstream's model as-is. No translation:

```
┌────────────────────────┐
│ Upstream Service       │
│ - User model (specific)
│ - Payment API shape    │
└────────────────────────┘
       │
       │ (NO translation layer)
       │
       ▼
┌────────────────────────┐
│ Downstream Service     │
│ Uses upstream model    │
│ directly; conforms to  │
│ upstream's design      │
└────────────────────────┘
```

**When to use:**
- Upstream is dominant, well-designed
- Translation overhead not justified
- Upstream changes rarely, predictably

**Advantages:** Simpler; less code.
**Disadvantages:** Downstream tightly coupled; upstream changes force downstream changes.

### Pattern: Customer-Supplier

Upstream (supplier) actively considers downstream's (customer's) needs:

```
┌────────────────────────────────────┐
│  Platform Team (Supplier)          │
│  Owns: Authentication, User DB     │
│  Committed to supporting           │
│  Auth clients with stable API      │
└────────────────────────────────────┘
       │
       │ (Supplier considers
       │  customers' priorities)
       │
       ▼
┌────────────────────────────────────┐
│  Product Team (Customer)           │
│  Dependent on Auth API             │
│  Provides feature requests         │
│  Has voice in platform roadmap     │
└────────────────────────────────────┘
```

**When to use:**
- Supplier depends on customers' success
- Long-term partnership (e.g., platform + products)
- Formal communication channel (e.g., steering committee)

**Example:**
- Platform team publishes quarterly feature plan
- Customer teams provide requirements
- Supplier team negotiates priorities
- API contracts reviewed by both parties

---

## Strategic Design: Team Topologies

Bounded contexts map to teams. Conway's Law: organization structure is reflected in system architecture.

**Principle:** Each service should be owned by one team; each team should own one or a few related services.

```
Scenario 1 (Anti-pattern: Shared ownership)
┌─────────────────┐       ┌──────────────────┐
│  Order Service  │◄─────►│  Payment Service │
└─────────────────┘       └──────────────────┘
       △                           △
       │                           │
       └───────┬──────────┬────────┘
               │          │
            Team A    Team B
               (Both teams touch both services)

Result: Slow, brittle; blame-shifting

Scenario 2 (Aligned: Clear ownership)
┌─────────────────┐       ┌──────────────────┐
│  Order Service  │──────►│  Payment Service │
└─────────────────┘       └──────────────────┘
       △                           △
       │                           │
       │                           │
    Team A                      Team B
    (owns Order)              (owns Payment)
    (can change freely)       (publishes stable API)

Result: Fast, clear accountability
```

**Team topology patterns:**

| Pattern             | When                                          | Structure                                  |
| ------------------- | --------------------------------------------- | ------------------------------------------ |
| **Platform + users**| One team builds infrastructure; others use it | Platform exports API; users conform        |
| **Stream-aligned**  | Each team owns end-to-end feature stream      | Cross-functional, minimal inter-team deps  |
| **Enabling team**   | Specialists help other teams                  | Temporary; teaches then steps back         |
| **Complicated**     | Shared expertise required (e.g., security)    | Centralized; accessed by others as service |

## Context Discovery

**How to find bounded contexts:**

1. **Talk to domain experts:** Where do they naturally divide the problem?
2. **Language shifts:** When does vocabulary change? (Inventory says "stock"; Finance says "GL account")
3. **Data flows:** Where are the boundaries of state changes?
4. **Team structure:** Where could a team own independently?
5. **Event storming:** Collaborate on timeline; gaps suggest context boundaries

**Example discovery session:**
```
Facilitator: "Tell me about an order."
Domain expert (Ordering): "Customer places order; we confirm."
Domain expert (Billing): "Once confirmed, we bill."
Domain expert (Inventory): "We reserve stock; if unavailable, backorder."
Domain expert (Shipping): "When paid, we dispatch."

Observations:
- Different vocabularies: "order" vs "bill" vs "allocation" vs "shipment"
- Different timing: Ordering is synchronous; Billing is async
- Different triggers: Billing depends on Ordering success

Conclusion: **Four distinct contexts**
- Ordering context (entry point)
- Billing context (post-order)
- Inventory context (parallel to Ordering)
- Shipping context (post-Billing)
```

## Mapping the Context Map

Document relationships:

```
Context Map (diagram or text)

[Customer-Supplier]
Ordering ──────► Billing
  │                (Supplier provides API;
  │                 Customer pulls invoice data)
  │
  ├─────────────────────────┐
  │ [Open Host Service]     │
  ▼                         ▼
Inventory             Product Catalog
  │                      (Open API;
  └─────────► Shipping       multiple consumers)
  │         [Conformist]
  │          (uses Inventory
  │           model directly)
  │
  └──────────► Analytics
             [ACL]
             (translates Business
              events to Analytics
              schema)
```

**In text:**
```
- **Ordering** ──[Customer-Supplier]──► **Billing**
  (Ordering is customer; Billing is supplier)

- **Ordering** ──[Open Host Service]──► **Inventory**
  (Inventory publishes stable API)

- **Inventory** ──[Conformist]──► **Shipping**
  (Shipping adopts Inventory's allocation model)

- **Ordering** ──[ACL]──► **Analytics**
  (Analytics has separate schema; ACL translates events)
```

## Pitfalls

| Pitfall                          | Consequence                                     | Mitigation                          |
| -------------------------------- | ----------------------------------------------- | ----------------------------------- |
| **No context map**               | Unclear ownership; hidden coupling               | Discover and document patterns      |
| **Too many shared kernels**      | Coupling creep; changes ripple                  | Minimize kernel; revisit boundaries |
| **Unclear upstream/downstream**  | Ambiguous dependencies; hard to debug            | Explicitly name direction           |
| **ACL too thin**                 | Upstream model leaks into downstream            | Enforce translation; validate input |
| **Context spans multiple teams** | Slow decisions; blame-shifting; duplicated work | Merge contexts or split teams       |

See also: [microservices decomposition](microservices-decomposition.md), [architecture DDD](architecture-ddd.md), [aggregates](ddd-aggregate-design.md)