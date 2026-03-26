# Event Sourcing — Event Store Design, Projections & Temporal Queries

## Fundamentals

Event Sourcing inverts traditional persistence: instead of storing current state, store every state change as an immutable event. The system of record is the event stream; current state is derived by replaying events.

**Capture rule:** Every domain-relevant change originates as an event object, persisted in sequence, with the same lifetime as application state itself.

**Key capabilities:**
- **Complete audit trail.** Every change is logged with intent and consequences.
- **Temporal queries.** Reconstruct state at any point in time by replaying events up to that moment.
- **Debugging clarity.** Trace exactly why state changed, not just its current value.
- **Event replay.** Correct past mistakes by reversing incorrect events and replaying corrected sequences.

## Event Store Design

**Write model:** Event append-only log. Events are immutable once persisted. Storage options: databases (PostgreSQL JSON, MongoDB), specialized event stores (EventStoreDB, Apache Kafka), or cloud services (AWS DynamoDB Streams, Azure Event Hubs).

**Partitioning:** Events are typically partitioned by aggregate ID (or stream ID), enabling:
- Fast retrieval of all events for a specific aggregate
- Concurrency guarantees within an aggregate stream
- Independent snapshots and compaction per stream

**Consistency guarantees:** All events for an aggregate must be stored in order with strong ordering semantics. Race conditions between command execution and event persistence must be eliminated (optimistic locking on version numbers, or single-threaded command processing per aggregate).

**Query efficiency:** The event stream itself is inefficient for many queries (e.g., "find all orders over $1000 placed today"). This is why projections exist.

## Projections: Building Query Models

**Concept:** A projection reads events from the event stream and builds a denormalized, query-optimized view. Multiple projections can exist—different views for different query patterns.

**Types:**
- **Inline projections:** Updated synchronously during command processing. Simpler but blocks writes.
- **Async projections:** Updated asynchronously after events are persisted. Higher throughput, eventual consistency tradeoff.
- **Streaming projections:** Continuously updated as events arrive (used by event streaming platforms).

**Idempotency requirement:** Projections must be idempotent—applying the same event twice produces the same result as applying it once. Failure handling and event replay become safe only with idempotent projections. Track processed event sequence numbers and skip duplicates.

**Rebuild capability:** Projections should be rebuildable from scratch by replaying the event stream. This enables migration to new query models, bug fixes in projection logic, and zero-downtime updates.

## Snapshots: Optimization

**Problem:** Replaying ten years of events to get current state is slow.

**Solution:** Snapshot the aggregate's state at regular intervals (e.g., every N events). When reconstructing state, load the latest snapshot and replay only subsequent events.

**Tradeoffs:**
- **Storage overhead:** Snapshots duplicate data already in events.
- **Synchronization complexity:** Snapshots must stay consistent with events during concurrent updates.
- **Rebuild risk:** If snapshot generation logic is buggy, errors propagate.

**Pattern:** Combine snapshots with compression or archival of old events for long-lived aggregates.

## Event Versioning & Upcasting

**Challenge:** Domain models evolve. Old events in the store may not match the current schema.

**Two approaches:**

1. **Upcasting (schema evolution):** When reading old events, transform them on-the-fly to match the new schema. The event store remains immutable; transformation happens in application code. This is the standard approach in production systems.

2. **Compensation events:** Append new events that correct or override effects of old events. Keeps history intact but requires careful sequencing and idempotency.

**Upcaster implementation:** Maps old event format to new. For example, if an old `UserCreated` event lacks a `timezone` field, an upcaster injects a default. Upcasters form a chain—each version upgrade applies the next transformation.

**Idempotency requirement:** Upcasters must be deterministic and composable. Running an upcaster twice should produce identical results to running it once.

**Migration strategy:** Version events explicitly. Track which events use which schema. Gradually migrate storage, or use lazy upcasting on read and periodically batch-write upgraded events.

## Temporal Queries

**Mechanism:** Any state query can be time-shifted by replaying events only up to a specific timestamp.

**Examples:**
- What was the account balance on January 1?
- How many active users existed at launch?
- What was the full order history just before the bug fix?

**Implementation patterns:**
- Maintain snapshots at key time boundaries for fast lookups.
- Use event timestamps (verified servers-side, not client-side, for correctness).
- Cache historical snapshots or use read-optimized time-series stores as an alternative.

**Analytics integration:** Temporal queryability makes event sourcing attractive for analytics—you can replay events into a warehouse with full historical accuracy, no data loss from overwrites.

## Idempotent Event Handling

**Challenge:** Distributed systems often deliver events multiple times (network retries, reprocessing).

**Solution:** Handlers must be idempotent—processing the same event twice yields the same result as processing it once.

**Patterns:**
- **Deduplication table:** Track processed event IDs; skip re-processing.
- **Deterministic state transitions:** If applying event E to state S always produces state S', idempotency is guaranteed.
- **Sequence number tracking:** Projections track the last applied event sequence number and skip older duplicates.

## Practical Challenges

**Storage growth:** Event streams grow unbounded. Archive old events after snapshots are taken, or implement compaction—periodically rebase the stream on snapshots and discard old events (losing history, but freeing storage).

**Debugging complexity:** With years of events, understanding why a bug occurred requires replaying large sequences. Event versioning and clear event naming help.

**Data migration:** Changing aggregates, event structures, or storage systems is complex. Plan migrations carefully—consider running dual writes or shadow systems.

**Event discovery:** With many event types and projections, discoverability and impact analysis become harder. Maintain event registry and documentation.

## CQRS Pairing

Event Sourcing pairs naturally with CQRS but is independent. Event Sourcing alone is valid for audit-heavy systems. CQRS + Event Sourcing optimally combines: commands generate events, projections subscribe to events and maintain read models, separating write and read concerns entirely.

See also: CQRS, event-driven architecture, architecture-patterns, Domain-Driven Design