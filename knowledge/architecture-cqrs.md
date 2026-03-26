# CQRS — Command Query Responsibility Segregation

## Overview

CQRS (Command Query Responsibility Segregation) separates the data model used for writes (commands) from the model used for reads (queries). Instead of a single conceptual model that handles both CRUD operations, you maintain independent write and read models optimized for their respective responsibilities.

The core insight originates from Command-Query Separation (CQS): methods should either execute commands (state-changing, no return value) or queries (state-retrieving, no side effects), but not both. CQRS extends CQS to the architectural level.

## When CQRS Adds Value

**Complex domains with asymmetric read/write patterns.** Many systems have vastly different read and write logic. CQRS lets you simplify the write model (focused on business rules, validation, consistency constraints) and independently optimize the read model for projection, denormalization, and reporting.

**Performance scaling.** Reads and writes can be scaled independently. If a system sees 100:1 read-to-write ratio, you can use a lightweight command model and multiple read replicas.

**High-frequency write verification.** In financial or audit-heavy systems, CQRS naturally accommodates temporal queries, event replay, and complete domain event history.

**Task-based interfaces.** User interfaces structured around domain actions ("ProcessRefund", "ShipOrder") rather than CRUD operations often pair well with command models that explicitly represent business operations.

## Architecture

**Write side (command model):** Handles domain logic, validation, state mutations. Often uses Domain-Driven Design, aggregates, and entity consistency. Commands are executed, events are emitted.

**Read side (query model):** Denormalized, query-optimized representations. Can be rebuilt from events asynchronously. Multiple read models can coexist for different query patterns.

**Communication:** Models can share the same database (with the database as mediator), use separate databases (query database becomes a real-time reporting database), or communicate via event streams. Eventual consistency typically governs synchronization.

## Trade-offs

**Complexity.** CQRS is a significant mental leap. It requires strong event architecture understanding, careful eventual consistency handling, and more infrastructure. Martin Fowler's original guidance: use CQRS only on portions of your system (bounded contexts), and only when simpler models fail.

**Eventual consistency challenges.** Read models lag behind writes. Stale reads, handling reconciliation, and user expectation management introduce risk. Not suitable for environments requiring strong consistency across all queries.

**Operational overhead.** Separate models create deployment, monitoring, and debugging complexity. Consistency bugs become harder to trace.

**Not a complexity cure-all.** Applying CQRS to a domain where command and query models overlap significantly doesn't simplify—it adds busywork (synchronizing identical models).

## Integration Patterns

**CQRS + Event Sourcing:** Event Sourcing (storing immutable events as the system of record) pairs naturally with CQRS. Commands generate events; events feed read model projections. This enables temporal queries, event replay, and complete auditability.

**CQRS + Event-Driven Architecture:** Separate services communicate via events. Command service publishes domain events; query services subscribe and maintain their own read models.

**Reporting Database Alternative:** If you need read/write separation without full CQRS complexity, maintain a primary transactional database and offload demanding queries to a separate reporting database. This avoids dual-model maintenance for most of your system.

## Anti-Patterns and Red Flags

**Premature CQRS adoption.** Most systems benefit from simpler architectures. If your queries and commands are not significantly different, CQRS adds waste.

**Global CQRS.** Never apply CQRS across an entire system. Use it only in specific bounded contexts where the benefit is clear.

**Ignoring write-side consistency.** The command side should enforce invariants strictly. Pushing all complexity to eventual consistency creates data integrity bugs.

**Read model synchronization without idempotency.** If projections aren't idempotent, duplicate or out-of-order events cause divergence between read models.

## Testing CQRS Systems

**Command testing:** Verify domain logic, invariant enforcement, event generation.

**Projection testing:** Test event-to-read-model transformations reproduce correct query results.

**Integration testing:** Verify commands and queries eventually converge.

**Temporal testing:** Verify events can replay to reconstruct past states accurately.

## When Simpler Models Suffice

If your system is predominantly CRUD with straightforward business logic, a single model is cheaper and less risky. CQRS is not a reflex—it's a targeted tool for specific problem spaces. Many successful systems use a hybrid: simple CRUD for most domains, CQRS only for high-complexity, high-performance-demanding bounded contexts.

See also: Domain-Driven Design, Event Sourcing, event-driven architecture, eventual consistency patterns