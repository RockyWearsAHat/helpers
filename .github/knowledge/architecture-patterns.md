# Architecture Patterns

## Layered (N-Tier) Architecture
The classic default. Separates code into horizontal layers:
- **Presentation** (UI, API controllers)
- **Business Logic** (domain rules, services)
- **Data Access** (repositories, ORM, database)

Each layer depends only on the layer below it. Simple, well-understood, good for CRUD-heavy apps. Risk: becomes a "big ball of mud" in large systems because layers encourage wide, unfocused classes.

## Clean Architecture (Robert C. Martin)
Concentric circles of dependency — dependencies point inward:
- **Entities** (core business objects) — innermost, no dependencies
- **Use Cases** (application logic) — orchestrates entities
- **Interface Adapters** (controllers, presenters, gateways)
- **Frameworks & Drivers** (web, DB, external APIs) — outermost

The inner layers know nothing about the outer layers. Dependency inversion enforced at every boundary. Highly testable — business logic has zero framework dependencies.

## Hexagonal Architecture (Ports and Adapters)
Application core defines **ports** (interfaces). External systems connect via **adapters**.
- **Driving ports** (left side): how the outside triggers the app (HTTP, CLI, message queue).
- **Driven ports** (right side): how the app reaches the outside (database, email, payment).

Swap adapters freely — replace PostgreSQL with MongoDB, REST with GraphQL, without touching business logic. The application is a hexagon; ports are its edges.

## Microservices
Each business capability is a separately deployable service with its own database.

**When to use:** Large teams, independent deployment needs, polyglot tech stacks, different scaling requirements per service.

**When NOT to use:** Small teams, simple domains, when you don't have operational maturity (monitoring, tracing, CI/CD). Start with a modular monolith; extract microservices when the pain of the monolith outweighs the pain of distribution.

**Key principles:**
- Each service owns its data (no shared databases).
- Communicate via APIs (REST/gRPC) or async events (message queues).
- Design for failure (circuit breakers, retries with backoff, bulkheads).
- Distributed tracing is non-optional (OpenTelemetry).

## Modular Monolith
A monolith with strict internal module boundaries. Each module has:
- Its own domain model.
- A clear public API (façade).
- Ideally its own database schema.

Modules communicate through defined interfaces, not direct database access. Deploy as one unit but structured for future extraction into microservices if needed. Best of both worlds for small-to-medium teams.

## Event-Driven Architecture
Components communicate through events rather than direct calls.

**Event types:**
- **Domain events**: Something happened in the domain (`OrderPlaced`, `UserRegistered`).
- **Integration events**: Cross-service communication via message broker.

**Patterns:**
- **Event sourcing**: Store events as the source of truth. Rebuild state by replaying events. Great for audit trails, temporal queries.
- **CQRS (Command Query Responsibility Segregation)**: Separate read models from write models. Write side processes commands; read side is optimized for queries.
- **Saga/Choreography**: Manage distributed transactions across services via event chains.

**Tools:** Kafka, RabbitMQ, NATS, AWS SQS/SNS, Redis Streams.

## Service-Oriented Architecture (SOA)
Predecessor to microservices. Larger, coarser-grained services communicating through an Enterprise Service Bus (ESB). Still relevant in enterprise environments. Key difference from microservices: shared infrastructure (ESB), often shared databases.

## Serverless / FaaS
Functions deployed individually, triggered by events. No server management.
- **Pros:** Zero-to-scale automatically, pay-per-invocation, minimal ops.
- **Cons:** Cold starts, vendor lock-in, hard to test locally, 15-minute execution limits.
- **Best for:** Event processing, webhooks, scheduled tasks, API backends with variable load.

## Choosing an Architecture

| Factor | Monolith | Modular Monolith | Microservices |
|--------|----------|-------------------|---------------|
| Team size | 1-10 | 5-30 | 20+ |
| Deployment | Simple | Simple | Complex |
| Data consistency | Easy | Moderate | Hard |
| Tech diversity | Low | Low | High |
| Operational cost | Low | Low | High |
| Scaling flexibility | Limited | Limited | High |

**Golden rule:** Start simple. Add complexity only when the current architecture demonstrably fails to meet requirements.

---

*Sources: Robert C. Martin (Clean Architecture), Alistair Cockburn (Hexagonal Architecture), Martin Fowler (Microservices, Event Sourcing), Sam Newman (Building Microservices)*
