# Microservices Decomposition — Bounded Contexts, Strangler, Data Separation & Team Topology

## Decomposition Strategies

### By Business Capability

Map service boundaries to what the organization **does**, not its internal structure. A retail system decomposes into Catalog, Ordering, Shipping, Billing, Inventory—each representing a distinct business function customers experience.

**Strengths:** Clear stakeholder ownership, aligns with business metrics, survives competitive landscape shifts.

**Weaknesses:** Requires understanding business domain (not technical); departments might disagree on boundaries; capability boundaries shift as business evolves.

### By Subdomain (Domain-Driven Design)

Use bounded contexts from DDD. A domain consists of core domains (revenue-generating), supporting domains (required but generic), and generic domains (off-the-shelf).

- **Core domains** get dedicated teams and services with the best engineers
- **Supporting domains** can share infrastructure; simpler patterns acceptable
- **Generic domains** (auth, notification, logging) use third-party solutions when viable

Example: A lending platform has *Credit Analysis* (core), *Customer Data* (supporting), *Email Notifications* (generic).

**Strengths:** Strategic focus on competitive advantage; reduces complexity for supporting/generic areas; enables team scaling.

**Weaknesses:** Requires domain expertise upfront; boundaries shift with strategy changes; shared services create coupling.

### By Data Entity / Aggregate

Each microservice owns a primary entity and its relationships. Order service owns orders and line items. Inventory service owns stock levels and reservations.

**Strengths:** Clear data ownership; natural fit for DDD aggregates; reduces cross-service queries.

**Weaknesses:** Creates duplicate entities across services (customer ID replicated); needs API composition for queries touching multiple entities; incentivizes denormalization.

## Decomposition Anti-Patterns

### Shared Database

Multiple services reading/writing the same database violates the microservice principle. Creates tight coupling—schema changes affect all consumers. Prevents independent scaling and deployment. Can be a temporary strangler step but shouldn't be the destination.

### Per-Function Services

Creating one service per REST endpoint (one for /users, one for /orders). Leads to excessive overhead, no clear data ownership, chatty inter-service communication. Not actually modular—just distributed complexity.

### Anemic Services (Getters/Setters Only)

Services that exist only to expose CRUD operations on a data entity, with business logic elsewhere. Defeats the purpose of decomposition—logic should live with data it operates on.

---

## Strangler Fig Migration

The **strangler pattern** (or "strangler fig") incrementally replaces a monolith by routing traffic to new microservices while the legacy system shrinks. Not a big-bang rewrite.

### How It Works

```
┌─────────────────────────┐
│   API Gateway/Router    │
└────────────┬────────────┘
             │
      ┌──────┴──────┐
      │             │
   New Services   Legacy Monolith
  (Strangler)    (Shrinking)
```

Phase progression:
1. **Route new feature requests** to new service; legacy otherwise
2. **Gradually move feature flags** from legacy to new  
3. **Dual-write during transition** (write to both old and new; read from new)
4. **Flip switch** when confident; deprecate legacy flow
5. **Sunsetting:** Remove legacy code once monitoring confirms no fallback

### Key Implementation Details

**Anti-Corruption Layer:** Translate between legacy data models and new service models. Prevents legacy baggage from infecting new services.

```
Client → Router → Anti-Corruption Layer → New Service
                  (transforms request/response)
```

**Data Consistency During Dual-Write:** Write to both old and new systems during transition. Read from new (which may be slightly behind). Idempotent operations are critical—retries shouldn't duplicate state.

**Confidence Signals:** Monitor new service quality metrics (latency, error rates) against legacy. Only route more traffic if new service outperforms or matches legacy SLAs. Problems detected during strangling are cheaper than problems in full decomposition.

---

## Service Boundaries: The Litmus Tests

### High Cohesion

Everything in a service changes together—features are localized. Changes to one service rarely require coordinated changes elsewhere.

**Question:** If I ship a new version of this service, can another service continue unchanged? If no, boundary is wrong.

### Loose Coupling

Services are independent. One service's internal changes don't force others to update.

Tight coupling failures:
- Service A changes its response schema; Service B breaks
- One service's database schema change requires coordination with all consumers
- Deploying Service A requires deploying Service B

### Autonomous Deployment

A service can be deployed independently without coordinating with other services. Requires:
- Clear contracts (API versioning, backwards compatibility)
- Graceful degradation when dependencies are temporarily down
- No shared databases

### Data Ownership

Each service owns its primary data and is responsible for its integrity. Other services access it read-only through APIs.

**Violation:** Two services writing to overlapping data sets → conflict resolution nightmares.

**Boundary test:** If changing one service's data model requires migrations in another service's code, the boundary is wrong.

---

## Team Topology Alignment

Microservice boundaries must align with team structure and communication patterns (Conway's Law applies—system architecture mirrors organization).

### One Team, One Service

Ideal case. Single team owns a service end-to-end: development, deployment, on-call. Eliminates handoff delays.

### Multiple Teams, One Service

Shared ownership. Works with clear ownership rules and explicit communication channels. Risk: unclear responsibility becomes diffused responsibility. Useful for large, complex services where specialization is needed.

### One Team, Multiple Services

One team owns several related services. Common for infrastructure teams (observability, auth, API gateway). Risk: team becomes bottleneck; services lack clear ownership.

---

## Decomposition Challenges & Trade-offs

| Challenge | Implication | Mitigation |
|-----------|-------------|-----------|
| **Eventual consistency** | Changes in one service propagate asynchronously to others | Accept delays; use event streams; implement compensation patterns |
| **Distributed debugging** | Failures span multiple services; tracing complexity grows | Distributed tracing (OpenTelemetry); correlation IDs across logs |
| **Network latency** | Inter-service calls are orders of magnitude slower than function calls | Minimize chattiness; batch requests; cache aggressively |
| **Data duplication** | Avoiding shared databases necessitates denormalization | Explicit cache invalidation; CDC (Change Data Capture) pipelines |
| **Operational complexity** | More moving parts; more deployment coordination required | Invest in automation, CI/CD, observability early |
| **Testing difficulty** | Integration tests span process boundaries | Contract testing; test doubles; pyramid strategy |

---

## When to Decompose (And When Not To)

### Good Candidates for Early Decomposition

- Feature teams with conflicting deployment schedules
- Scaling requirements that differ across features (e.g., one needs aggressive caching)
- Security/compliance boundary (PII handling isolated)
- Technology diversity (one service needs a different language/runtime)
- Business capability with clear stakeholder ownership

### Keep Monolithic

- Early-stage products where boundaries are still uncertain
- High-velocity features being shipped faster than clarifications emerge
- Team still learning the domain
- Performance requirements necessitate synchronous RPC (keep tightly coupled)

**Red flag:** Decomposing to solve team politics. Misaligned teams will struggle with either architecture.

---

## References & Related Concepts

See also: strangler-fig pattern, domain-driven design, team topology, service boundaries, anti-corruption layers.