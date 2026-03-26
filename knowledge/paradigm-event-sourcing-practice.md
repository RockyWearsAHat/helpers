# Event Sourcing in Practice — Implementation, Evolution & Operational Concerns

## Event Store Design Decisions

**Append-only storage.** The foundational commitment: events are immutable once committed. This simplifies concurrency (no locks for reads) but requires deletion via scavenging, encryption for secrets, and retraction patterns for "undo" scenarios.

**Partitioning strategy.** Events typically partition by aggregate ID (stream ID), enabling fast retrieval of an entity's history and local snapshots. Coarse-grained stream names (e.g., "all-orders") scale poorly under write load; fine-grained (per-aggregate) is standard. Some systems use multiple partition schemes: audit log (stream per entity type) plus operational streams (by tenant or time window).

**Consistency model.** Strong per-stream ordering is non-negotiable (all events for an aggregate in sequence). Cross-stream consistency is eventual — reading event N from stream A and event M from stream B may momentarily see inconsistent state. Command handlers must handle optimistic concurrency (version numbers per stream) or single-threaded processing per aggregate.

**Storage technology trade-offs:**
- **Relational databases** (PostgreSQL, MySQL): ACID guarantees, JSON columns for event payloads, proven operational tooling. Competing writes on the same aggregate may cause contention.
- **Document databases** (Couchbase, RavenDB): Document-level transactions fit stream semantics. Query flexibility varies.
- **Specialized event stores** (EventStoreDB, Axon Framework): Purpose-built subscription management, projection tooling, built-in snapshots. Operational learning curve and vendor lock-in.
- **Streaming platforms** (Apache Kafka, AWS Kinesis): Higher throughput, temporal log guarantees, multi-consumer subscriptions. Less suitable for aggregate-level consistency guarantees.

## Event Schema Design

**Payload structure.** Events record **what happened**, not **why** or **how the system reacted**. An "OrderPlaced" event contains order ID, items, customer, total—not the resulting projection updates.

**Versioning immutability.** Once events are persisted, their schema is permanent. Breaking a field contract breaks history. Strategies for evolution:
- **Additive fields.** Add optional fields; old events lack them (consumers use defaults).
- **New event types.** Introduce "OrderPlacedV2" for incompatible changes; replay logic handles both versions.
- **Upcasting at read-time.** Transform old events into new schema when replaying projections (costs replay performance but delays schema changes).
- **Migration events.** Run a one-time process reading old events and writing new "equivalent" events to the stream.

**Metadata and causality.** Include correlation IDs (linking request → commands → events), event timestamp, aggregate version, originating user/system. These enable debugging and audit trails but add storage.

## Projections and Read Models

**Single-model projections.** Simple many-to-one: events build a single read model (e.g., "Customer Account" projection tracks balance, tier, activity). Fastest for single-entity reads.

**Multi-model projections.** One event stream feeds multiple read models simultaneously. "OrderPlaced" updates inventory projection, customer-orders-list projection, and analytics projection. Enables denormalization for different query patterns.

**Subscription semantics.** Projections consume events from a subscription:
- **At-least-once delivery.** Event redelivered if processor crashes mid-handling; idempotency required (deduplication by event ID + version).
- **Exactly-once (exactly-up-to-the-last-successful):** Track last processed event version per projection; detect and ignore duplicates.
- **Ordering guarantees.** Within a stream (per aggregate) usually sequential; across aggregates, ordering is not guaranteed unless using a single append-only log.

**Transient vs. persistent projections.** Transient projections rebuild on startup (suitable for derived views like search indexes); persistent store intermediate state (suitable for read models served to clients). Some systems support both: keep a persistent projection as the primary read model and regenerate transient ones for performance.

## Snapshots and Performance

**Why snapshots.** Replaying 10 years of events to reconstruct current state is slow. Snapshots store a point-in-time aggregate state, allowing replay from that point forward.

**When to snapshot.** Common triggers: every N events (e.g., 100), every time interval, or manually on large aggregates. The cost is storage (duplicate of aggregate state) vs. benefit (faster reads).

**Consistency risk.** If a snapshot is stale or incorrect, derived state diverges from truth. Versioning snapshots and validation during replay mitigates this.

**Alternatives to snapshots:** Event compaction (rewrite the stream, removing intermediate events), or materialized aggregates (continuously write aggregate state to a side table). Each trades different resources.

## Schema Evolution and Event Migration

**Breaking migrations.** Renaming a field in a persisted event breaks all existing events. Strategy: introduce a new event type, migrate events via background job, deprecate old type.

**Upcasting gradients.** Some frameworks support event upcasting policies: define transformations between schema versions. Expensive during projection rebuild, valuable for clean old-event handling.

**Replaying vs. migrating.** Two approaches: (1) leave events as-is, handle both versions at read-time; (2) migrate events in-place, rewriting the stream. (1) scales better (lazy transformation); (2) costs upfront but simplifies reader logic.

**Testing migrations.** Schema changes are risky. Test that events -> old schema -> new schema produce identical results to events -> new schema.

## CQRS + Event Sourcing

**Separation asymmetry.** Command side (event sourcing) and query side (projections) evolve independently. Commands remain strict aggregates; queries scatter across denormalized read models.

**Consistency lag.** Projections reflect history with delay. UI must either accept stale data or request eventual consistency (e.g., "refresh" buttons for fresh-as-of points). Some patterns use optimistic updates (render locally before event completes).

**Coordination.** Commands trigger events; all views update asynchronously. Failures mid-event-dispatch require compensating commands or manual reconciliation.

## Operational Challenges

**Observability.** Track projection lag (how far behind the event stream is the read model). Alert if lag grows unbounded—indicates processor bottleneck or crash.

**Rebuilds.** Periodically re-run projections from scratch to verify consistency or change projection logic. Full rebuild costs I/O and processor time; design for incremental rebuild where possible.

**Deletion and compliance.** Events are immutable, but GDPR/compliance may require deletion. Strategies: encrypt sensitive fields, truncate events, introduce "erasure events" (pseudo-events marking deletion). None are clean—this is hard.

**Backup and recovery.** Event store is the source of truth. Backup it like your primary database. Replay from backup loses events after backup point; some systems support log shipping.

## Testing Event-Sourced Systems

**Property-based tests on aggregates.** Given a sequence of commands, verify resulting state is deterministic and consistent with aggregate invariants. Test edge cases: duplicate commands, out-of-order delivery.

**Projection testing.** Test that projections correctly transform event sequences into query models. Use snapshot testing: verify projection state at key points.

**Saga testing.** If using sagas (orchestration of commands across aggregates), test that multi-step flows complete or roll back correctly under failures.

**Event versioning tests.** For every schema version, verify that upcasting or on-read transformation produces expected results.

## Common Pitfalls

**Over-domain-driven event naming.** "OrderPlaced" vs "DepositTransferred"—naming reveals intent. But vague event names ("StateChanged", "Updated") hide history. Use domain language.

**Storing commands as events.** Events are facts (immutable outcomes); commands are intents (may fail). Conflating them complicates replay and audit trails.

**Ignoring concurrency.** Multiple commands on the same aggregate concurrently—must enforce stream ordering and version checks, or replay becomes non-deterministic.

**Projection cascades.** One projection feeds another (projection A -> event B -> projection C). Cascading failures propagate; consider whether coupling is necessary.

**Premature snapshots.** Snapshots add complexity. Build without them first; add if profiling shows replay time as bottleneck.