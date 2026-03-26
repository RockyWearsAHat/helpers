# Microservices Architecture

## Decomposition Strategies

### By Business Capability

Map services to business functions — what the organization _does_, not how it's structured. A retail system decomposes into: Catalog, Ordering, Shipping, Billing, Inventory. Each capability is a service boundary.

### By Subdomain (DDD-aligned)

Use bounded contexts from Domain-Driven Design. Core domains get dedicated services with the best engineers. Supporting subdomains can be simpler. Generic subdomains (auth, notifications) use off-the-shelf solutions when possible.

### Strangler Fig Migration

Incrementally decompose a monolith by routing new traffic to microservices while legacy handles existing flows. Use an anti-corruption layer between old and new.

```
Client → API Gateway → [Router]
                         ├── New: Order Service
                         ├── New: Inventory Service
                         └── Legacy: Monolith (shrinking)
```

## Service Boundaries

Good boundaries exhibit:

| Property         | Description                                              |
| ---------------- | -------------------------------------------------------- |
| High cohesion    | Everything in the service changes together               |
| Loose coupling   | Changes in one service rarely require changes in another |
| Autonomous       | Can be developed, deployed, and scaled independently     |
| Owns its data    | No shared databases between services                     |
| Business-aligned | Maps to a business capability or bounded context         |

**Litmus test**: If changing a feature requires coordinated deployment of multiple services, the boundary is wrong.

## Communication Patterns

### Synchronous

| Protocol  | When to Use                                             | Trade-offs                                                       |
| --------- | ------------------------------------------------------- | ---------------------------------------------------------------- |
| REST/HTTP | CRUD operations, public APIs, simple request-response   | Simple but couples caller to callee's availability               |
| gRPC      | Internal service-to-service, high throughput, streaming | Fast (binary/HTTP2) but tighter schema coupling, harder to debug |
| GraphQL   | API aggregation, client-driven queries                  | Flexible queries but complex caching, N+1 risk                   |

### Asynchronous

| Pattern         | Implementation          | When to Use                                             |
| --------------- | ----------------------- | ------------------------------------------------------- |
| Message queue   | RabbitMQ, SQS           | Work distribution, load leveling, guaranteed delivery   |
| Event bus       | Kafka, SNS, EventBridge | Fan-out notifications, event-driven workflows           |
| Event streaming | Kafka, Kinesis          | Event sourcing, real-time processing, replay capability |

**Rule of thumb**: Use sync for queries that need immediate answers. Use async for commands/events where the caller doesn't need to wait.

## Service Discovery

### Client-Side Discovery

Services query a registry (Consul, Eureka) and load-balance themselves. More control but every client needs discovery logic.

### Server-Side Discovery

A load balancer (ALB, Kubernetes Service) handles discovery. Clients hit a single DNS name. Simpler for clients but adds a hop.

### Kubernetes-Native

```yaml
# Services get DNS automatically: <service>.<namespace>.svc.cluster.local
apiVersion: v1
kind: Service
metadata:
  name: order-service
spec:
  selector:
    app: order-service
  ports:
    - port: 80
      targetPort: 8080
```

## API Gateway Patterns

The gateway sits between clients and services, handling:

- **Routing**: Direct requests to the correct service
- **Authentication**: Validate tokens once at the edge
- **Rate limiting**: Protect services from overload
- **Response aggregation**: Combine data from multiple services for one client call
- **Protocol translation**: REST externally, gRPC internally

**BFF (Backend for Frontend)**: Separate gateways per client type (web, mobile, IoT) to optimize payloads and workflows.

## Data Management

### Database Per Service

Each service owns its database exclusively. No shared tables. Period.

| Approach                   | Use Case                                                  |
| -------------------------- | --------------------------------------------------------- |
| Private tables per service | Shared DBMS but strict logical separation                 |
| Database per service       | Full isolation, independent scaling and technology choice |
| Schema per service         | Middle ground on shared infrastructure                    |

### Saga Pattern

Manages distributed transactions without 2PC (which doesn't scale):

**Choreography**: Each service emits events triggering the next step. Simple but hard to follow, debug, and add compensating transactions.

```
Order Created → Payment Charged → Inventory Reserved → Shipping Scheduled
     ↓ (fail)        ↓ (fail)          ↓ (fail)
  (nothing)    Refund Payment    Release Inventory + Refund
```

**Orchestration**: A coordinator service directs the saga. Easier to understand and manage failures but introduces a central point of coordination.

### CQRS

Separate read and write models. Write side enforces business rules, read side is optimized for queries. Connected via events. Useful when read and write patterns differ significantly (e.g., complex writes, many read views).

## Distributed Transactions

| Approach             | Consistency | Complexity  | When to Use                               |
| -------------------- | ----------- | ----------- | ----------------------------------------- |
| Saga (choreography)  | Eventual    | Medium      | Simple workflows, few steps               |
| Saga (orchestration) | Eventual    | Medium-High | Complex workflows, many compensations     |
| Outbox pattern       | Eventual    | Medium      | Reliable event publishing from DB changes |
| 2PC (avoid)          | Strong      | High        | Almost never in microservices             |

### Outbox Pattern

Write the event to an outbox table in the same DB transaction as the business data. A separate process polls/CDC the outbox and publishes to the message broker. Guarantees at-least-once delivery.

```sql
BEGIN;
  INSERT INTO orders (id, ...) VALUES (...);
  INSERT INTO outbox (aggregate_type, aggregate_id, event_type, payload)
    VALUES ('Order', '123', 'OrderCreated', '{"id":"123",...}');
COMMIT;
```

## Testing Strategies

| Level       | What It Tests                           | Tools                           |
| ----------- | --------------------------------------- | ------------------------------- |
| Unit        | Business logic in isolation             | Standard unit test frameworks   |
| Integration | Service + its database/dependencies     | Testcontainers, docker-compose  |
| Contract    | API compatibility between services      | Pact, Spring Cloud Contract     |
| Component   | Single service end-to-end (deps mocked) | WireMock, in-memory DBs         |
| End-to-end  | Full system flows                       | Cypress, Playwright (sparingly) |

**Contract testing is critical**: Without it, services break each other silently at deployment. Consumer-driven contracts let the consumer define what it needs, and the provider verifies it can deliver.

## Observability

### Three Pillars

| Pillar  | Purpose                                | Tools                                    |
| ------- | -------------------------------------- | ---------------------------------------- |
| Logs    | What happened (structured, correlated) | ELK, Loki, CloudWatch Logs               |
| Metrics | How the system is performing           | Prometheus, Datadog, CloudWatch          |
| Traces  | Request flow across services           | Jaeger, Zipkin, AWS X-Ray, OpenTelemetry |

**Correlation IDs**: Every request gets a unique trace ID at the edge. All services propagate it in headers. Without this, debugging distributed issues is impossible.

### Key Metrics (RED Method)

- **Rate**: Requests per second
- **Errors**: Error rate (4xx, 5xx)
- **Duration**: Latency percentiles (p50, p95, p99)

## Deployment Patterns

### Sidecar

Deploy a helper container alongside the service container in the same pod. Handles cross-cutting concerns (logging, networking, security) without service code changes.

### Service Mesh

Sidecars (Envoy proxies) on every service, managed by a control plane (Istio, Linkerd). Provides mTLS, traffic management, observability without application changes.

```
┌─────────────────────────┐
│ Pod                     │
│  ┌────────┐ ┌────────┐ │
│  │Service │←→│ Envoy  │←→ Other pods' sidecars
│  └────────┘ └────────┘ │
└─────────────────────────┘
```

**Cost**: Significant operational complexity and resource overhead. Use when you actually need mesh features across many services, not as a default.

## Organizational Alignment

### Conway's Law

> "Organizations design systems that mirror their communication structures."

Use this deliberately: structure teams around service boundaries. A team owns 1-3 services end-to-end (build, deploy, operate).

### Team Topologies

| Type                  | Role                                                                         |
| --------------------- | ---------------------------------------------------------------------------- |
| Stream-aligned        | Owns a business capability end-to-end                                        |
| Platform              | Provides self-serve infrastructure (CI/CD, observability, service templates) |
| Enabling              | Coaches stream-aligned teams on specific capabilities                        |
| Complicated-subsystem | Owns technically complex components (ML, crypto)                             |

## Anti-Patterns

### Distributed Monolith

Services are deployed independently but must be changed and deployed together. Symptoms: shared libraries with business logic, synchronous call chains, shared databases.

### Chatty Services

Too many inter-service calls per user request. Fix: coarsen service granularity, use async messaging, aggregate at the gateway, or reconsider boundaries.

### Shared Database

Multiple services reading/writing the same tables. Eliminates independent deployability and schema evolution. Every schema change requires coordinating all services.

### Nano-Services

Services so small they have more infrastructure overhead than business logic. A function that's 50 lines of code and only called by one other service probably doesn't need to be a separate service.

### Premature Decomposition

Starting with microservices before understanding domain boundaries. Start with a well-structured monolith. Extract services when you have clear, proven boundaries and a real need for independent scaling or deployment.

**The first rule of microservices: don't start with microservices.**
