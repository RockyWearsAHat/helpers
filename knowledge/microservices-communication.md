# Microservices Communication — Synchronous, Asynchronous, Messaging, Service Mesh & Resilience

## Communication Spectrum

Microservices communicate via two fundamental modes, each with tradeoffs:

| Mode | Mechanism | Latency | Coupling | Best For |
|------|-----------|---------|----------|----------|
| **Synchronous** | Direct RPC (REST, gRPC) | Low (ms) | Tight (caller waits) | Query-response, immediate feedback |
| **Asynchronous** | Messaging/events (Kafka, RabbitMQ) | High (variable) | Loose (fire-and-forget) | State changes, batch processing, workflows |

---

## Synchronous Communication

### REST over HTTP

Services communicate via HTTP requests/responses. Stateless, cacheable, well-understood.

**Strengths:**
- Browser-based debugging and testing
- Mature ecosystem (proxies, load balancers, CDNs)
- Easy versioning (accept headers, URL paths)

**Weaknesses:**
- Text-based overhead (JSON serialization slower than binary)
- Connection per request (unless HTTP/2 multiplexing)
- No built-in retry logic; client must implement
- Cascading timeouts (A calls B calls C; if C slow, A times out through B)

**Resilience pattern:** Use exponential backoff with jitter for retries. Implement circuit breakers (fail fast after repeated failures). Set aggressive timeouts; fail fast rather than block indefinitely.

### gRPC (HTTP/2 + Protocol Buffers)

Binary RPC framework using HTTP/2 framing. Strongly typed; bidirectional streaming.

**Strengths:**
- Low latency (binary protocol, HTTP/2 multiplexing over single connection)
- Typed contracts (Protocol Buffers)
- Streaming support (useful for large datasets, real-time)
- Language-agnostic

**Weaknesses:**
- Steeper learning curve
- Poor debugging (binary format)
- Requires HTTP/2 (firewalls, older proxies may not support)
- Not as human-friendly as JSON

**When to choose gRPC:**
- High throughput, low-latency requirements (internal microservices)
- Streaming data (logs, metrics, real-time updates)
- Polyglot services (Protocol Buffers enforce contract both sides)

**When to stay with REST:**
- Public APIs (clients may be browsers or third-party tools)
- Prototyping (easier to test manually)
- Team unfamiliar with gRPC operational concerns

---

## Asynchronous Communication

### Event-Driven Messaging

Services publish domain events to a message broker (Kafka, RabbitMQ). Other services subscribe and react:

```
OrderService: "OrderCreated" event published
                    ↓
    ┌───────────────┼───────────────┐
    ↓               ↓               ↓
PaymentService  InventoryService  NotificationService
(charge card)   (reserve stock)   (send confirmation)
```

**Strengths:**
- Loose coupling (publisher doesn't know subscribers)
- Scalability (broker handles fanout)
- Resilience (subscriber down? events queue up)
- Workflow visibility (event stream is audit trail)

**Weaknesses:**
- Eventual consistency (changes propagate with delay)
- Debuggability (distributed asynchronous failures are complex)
- Exactly-once delivery is hard (at-most-once, at-least-once more realistic)
- Message ordering challenges (if order matters)

**Exactly-Once Semantics:** Virtually impossible to guarantee in distributed systems. Instead:
- Use idempotent handlers (applying same event twice has same effect as once)
- Track processed event IDs to skip duplicates
- Accept at-least-once delivery with deduplication logic

### Request-Reply Over Messaging

Combine asynchronous messaging with reply channels:

```
Service A publishes request to broker with reply address
Service B consumes request, processes, publishes response to reply address
Service A consumes response from its reply queue
```

Useful for workflows where async decoupling is desired but direct RPC is wrong (e.g., vendor's external API invocation takes minutes; don't block caller).

---

## Service Mesh

A **service mesh** is a dedicated infrastructure layer handling service-to-service communication concerns: routing, retries, timeouts, circuit breaking, observability.

### Architecture

Each service instance runs a sidecar proxy (Envoy, Linkerd). Proxies intercept all network traffic:

```
Service A → [Envoy Proxy] → [Envoy Proxy] → Service B
            (mTLS, retry, (load-balance,
             circuit-break) observe)
```

The control plane (Istio, Linkerd) configures all sidecars: where to route, retry policies, traffic splitting for canaries.

### Responsibilities

**Mesh handles:**
- Network reliability (retries, circuit breaking, timeouts)
- Load balancing and traffic splitting (canary deployments)
- Encryption (mTLS between services)
- Observability (trace all requests; collect metrics)
- Rate limiting

**Application handles:**
- Business logic errors (400s, domain validation failures)
- Idempotency (mesh can't know if an operation is idempotent)
- High-level workflow (which services to call, in what order)

### Costs

- Operational complexity (sidecars require management, debugging)
- Performance overhead (proxies add latency; usually acceptable)
- Debugging difficulty (another layer in the stack)

**Worth considering when:**
- 10+ services with complex communication patterns
- Cross-cutting concerns (security, observability) are becoming bottleneck
- Teams have infrastructure expertise to operate mesh

**Not worth it when:**
- Fewer than 5 services (manual management is simpler)
- Already have sophisticated load balancers or API gateways
- Team lacks infrastructure engineering maturity

---

## API Composition

When a request requires data from multiple services, the question is: where should composition happen?

### Backend for Frontend (BFF) / API Composition Service

A dedicated service aggregates calls to multiple backend services:

```
Client → BFF (one service) → multiple backend services
              (composes response)
```

Example: Dashboard endpoint needs data from Orders, Users, Metrics services. BFF calls all three, merges responses.

**Strengths:**
- Decouples client from backend service count (client sees one endpoint)
- Can optimize for each client type (web, mobile, TV)
- Centralized logic for cross-cutting concerns

**Weaknesses:**
- Becomes a bottleneck (single point of contention)
- Potential performance degradation (fanning out N requests from BFF)
- Additional latency (extra hop)

### Client-Side Composition

Client directly calls multiple services and composes result:

```
Client → [Service A, Service B, Service C] → compose locally
```

**Strengths:**
- Simpler infrastructure
- Lower latency (direct calls)
- Client controls parallelism

**Weaknesses:**
- Exposes internal service boundaries (client knows about all services)
- Client library management (duplicate retry/timeout logic across clients)
- Harder to coordinate changes across services

### When to Use BFF

- Mobile/web/TV clients with different data needs
- Significant aggregation logic (not just simple merges)
- Need to shield clients from service topology changes

---

## Communication Reliability Patterns

### Retry with Exponential Backoff + Jitter

Simple retries (immediate) can cause "thundering herd"—all clients retry simultaneously, overwhelming recovering service.

```python
delay = base_delay * (exponential_base ** attempt_number)
delay += random(0, jitter)
```

Prevents synchronized retry storms.

### Circuit Breaker

After repeated failures to one service, stop calling it for a window. Then test if it recovered.

States: **Closed** (normal) → **Open** (failing, stop calls) → **Half-Open** (testing if recovered) → **Closed**

Prevents cascading failures (if Service B is down, don't keep sending requests).

### Timeout Strategies

**Overall timeout:** Request deadline from initial caller. All downstream calls must complete within this window.

**Per-hop timeout:** Each proxied call has its own timeout, less than overall. Prevents a single slow service from blocking the entire request.

```
Overall: 5 seconds
├─ Service A timeout: 1.5s
├─ Service B timeout: 1.5s
└─ Service C timeout: 1.5s
```

### Bulkhead Pattern

Isolate resources (thread pools, connection pools) for different services. If one service saturates resources, others continue functioning.

**Example:** Dedicated connection pool for inventory service (10 connections), separate pool for payment service (10 connections). If inventory service hangs, it only consumes its 10 connections; payment service unaffected.

---

## Polyglot Communication

Services can use different protocols internally:
- REST for public-facing APIs
- gRPC for high-performance internal calls
- Messaging for state distribution
- WebSockets for real-time updates

**Key rule:** Make protocols explicit in contracts. Document expectations for latency, throughput, failure modes.

---

## References & Related Concepts

See also: service mesh, circuit breaker pattern, event-driven architecture, eventual consistency, distributed tracing, observability.