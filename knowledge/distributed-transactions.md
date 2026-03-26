# Distributed Transactions — Coordination, Atomicity, and Failure Handling

## Overview

A distributed transaction either commits atomically across multiple databases/services or aborts entirely, leaving no partial state. Unlike a single-database transaction (ACID is straightforward), a distributed transaction must coordinate multiple independent agents that can fail, network might partition, and no global lock exists. This section covers atomic commitment protocols, saga patterns for long-running workflows, and practical patterns for exactly-once semantics.

## Two-Phase Commit (2PC)

2PC is the classic protocol for atomic commitment across multiple databases (participants).

### Protocol Flow
**Phase 1 (Prepare/Voting):**
1. Coordinator sends "prepare to commit" to all participants.
2. Each participant executes the transaction, locks resources, and responds with "yes" (ready to commit) or "no" (abort).
3. If all say yes, move to phase 2. If any say no, abort.

**Phase 2 (Commit/Abort):**
1. If all voted yes, coordinator sends "commit" to all participants; they commit and release locks.
2. If any voted no, coordinator sends "abort"; participants roll back.

### Safety Guarantees
- **Atomicity:** All participants commit or all abort; no partial commits.
- **Durability:** Once committed, data persists even if participants crash after phase 2.

### Blocking Problem
2PC is **blocking**. A participant that votes yes is locked until the coordinator confirms the commit/abort decision. If the coordinator crashes:
- The participant remains locked, blocking other transactions.
- If the coordinator fails after a phase 1 vote but before sending phase 2, the participant is indefinitely blocked.

**Real-world impact:** Database connection pools exhaust; throughput collapses.

### Limitations
- Synchronous, expensive coordination.
- Susceptible to network partitions: if a partition separates coordinator from participants, transactions block.
- Participants must be able to lock resources for an unpredictable duration.
- Not feasible for very large, heterogeneous systems (multiple independent databases fail independently).

### When 2PC Is Used
- Single enterprise with heterogeneous databases (Oracle, PostgreSQL, etc.).
- Financial systems requiring precise atomicity (though often used in conjunction with saga patterns).
- Most cloud-native systems **avoid** 2PC, using sagas or eventual consistency instead.

## Three-Phase Commit (3PC)

3PC adds an extra phase to reduce blocking.

### Protocol Flow
**Phase 1 (Prepare):** Same as 2PC.

**Phase 2 (Pre-Commit/Ack):**
1. If all voted yes, coordinator sends "prepare to commit" (a promise that the commit will follow).
2. Participants acknowledge; they're still locked but can flush data and prepare for fast commit.

**Phase 3 (Commit/Abort):**
1. Coordinator sends final "commit" or "abort."
2. Participants commit/abort and release locks.

### Advantages Over 2PC
- Non-blocking in certain failure scenarios: If the coordinator crashes after phase 2, participants know a commit will eventually happen and can proceed.
- Reduces lock duration: Phase 2 is an acknowledgment, not the final decision.

### Limitations
- More complex; three round-trips instead of two.
- **Still not fully non-blocking:** If a network partition separates coordinator from participants after phase 2, participants are still stuck (should they commit or abort?).
- FLP impossibility (from consensus theory) shows no protocol is guaranteed to be non-blocking with crash failures in async systems.
- Rarely used in practice; most systems use 2PC if coordination is needed, or avoid blocking altogether via sagas.

## Saga Pattern — Long-Running Transactions

A saga is a sequence of local transactions (one per service) coordinated to achieve a distributed transaction's semantics without blocking.

### Choreography (Event-Driven)
Services publish events for each step. Other services listen and react.

**Example:** Order saga (Order Service → Payment Service → Inventory Service)
1. Order Service creates an order, publishes "OrderCreated."
2. Payment Service listens, processes payment, publishes "PaymentProcessed."
3. Inventory Service listens, reserves inventory, publishes "InventoryReserved."
4. Order Service listens, completes the order.

If a step fails (e.g., payment declined), it publishes a failure event. All services listening to that event orchestrate rollback.

**Advantages:**
- Decentralized; no orchestrator.
- Services are loosely coupled (publish events, don't know who consumes them).
- Scales naturally.

**Disadvantages:**
- Implicit flow; tracing who does what is hard.
- Each service must define compensating transactions (rollback logic).
- Chains of events can become tangled; debugging is difficult.
- Cycles or deadlocks can occur if choreography rules are not carefully designed.

### Orchestration (Centralized Coordinator)
A saga orchestrator service controls the flow. It's a state machine: "do X, then do Y, then do Z." If Y fails, run compensating transaction for X.

**Example:** Order orchestrator
```
1. Call Order Service → create order.
2. Call Payment Service → process payment.
   If fail → call Order Service compensate (cancel order).
3. Call Inventory Service → reserve inventory.
   If fail → call Payment Service compensate + Order Service compensate.
4. If all succeed → Order Service commit.
```

**Advantages:**
- Explicit, easy to trace and debug.
- Central point for handling failures and retries.
- Can encode complex business logic clearly.

**Disadvantages:**
- Centralized orchestrator is a bottleneck and single point of failure.
- Tight coupling between orchestrator and services.
- Orchestrator must be highly available (needs replication, failover).

## Compensation and Rollback

A compensating transaction undoes a previous transaction's effect. Unlike ACID rollback (automatic), sagas require explicit compensation code.

**Example:**
- Transaction: Debit account by $100.
- Compensation: Credit account by $100.

**Challenges:**
- A debit is easy to compensate. A "send email" transaction cannot be undone (you can't unsend an email; best you can do is send a correction).
- Irreversible operations (e.g., deleting data permanently) cannot be compensated; they must not occur in a saga or must be idempotent (running twice has the same effect as once).
- Compensations can fail. If Account Service is down, compensation fails. The system must detect this and retry.

**Strategies for non-compensable operations:**
- Don't perform them in the saga; do them in a final committed state.
- Use soft-deletes or archival instead of hard deletion.
- Design operations to be re-triggerable (idempotent) so retries are safe.

## Transactional Outbox Pattern

The outbox pattern solves the dual-write problem: writing to a database and publishing an event atomically is hard without 2PC.

### The Problem
```
Service 1 writes to database.
Service 1 publishes event to message broker.
```
If the publish fails, the database has the data but no one knows (event got lost). If the database write fails, the event shouldn't have been published.

### The Solution
1. Service 1 writes to database **and** writes an entry to an "outbox" table in the same transaction.
2. A separate process (poller or transaction log tailing) reads the outbox, publishes events to the broker, and marks entries as published.

**Atomicity:** Database write and outbox write are atomic (same transaction). Outbox write to broker is decoupled (can be retried).

**Exactly-once semantics:** If the event broker is idempotent (same event published twice has the same effect), republishing on process failure is safe.

### Implementation
- **Polling:** Periodically query the outbox table and publish unpublished entries. Simple but adds polling latency (seconds of delay).
- **Transaction log tailing:** Capture events from the database's transaction log (CDC - Change Data Capture). More efficient; seconds to milliseconds of latency. Examples: Debezium (for CDC), DynamoDB Streams, Postgres WAL.

## Idempotence and Exactly-Once Semantics

**Idempotence:** Running an operation multiple times has the same effect as running it once.

**Problem:** In a distributed system with retries, messages can be delivered multiple times. Without idempotence, retries cause duplicates (e.g., charging a customer twice).

### Exactly-Once Semantics (EOS)
EOS means each message is processed exactly once end-to-end, even with failures and retries.

**Three components:**

**1. Idempotent Producer:**
- Producer assigns unique IDs to messages.
- Broker deduplicates based on ID; each ID is stored once.
- If the producer retries sending the same message, the broker recognizes the duplicate and doesn't store it twice.
- Used in Kafka, some queue systems.

**2. Idempotent Consumer:**
- Consumer checks if it's already processed a message (via an idempotent key).
- If yes, skips processing; if no, processes and records the ID.
- Idempotent key can be message ID or a hash of message content.
- Storage of processed IDs requires a database or side-effect-free computation.

**3. Exactly-Once Processing with Outbox:**
- Process message (write to database).
- Write processed message ID to outbox (same transaction).
- Publish all changes via outbox.
- On retry, consumer queries the database; if the message ID is there, it skips (idempotent).

### Practical Trade-offs
- Perfect exactly-once is expensive (requires coordination, state tracking).
- At-least-once semantics (message delivered ≥ 1 time) with idempotent consumer is a practical compromise.
- At-most-once semantics (message delivered ≤ 1 time) loses messages on failure; rarely acceptable.

Most systems aim for "**effectively exactly-once**": at-least-once delivery + idempotent processing.

## Isolation Levels in Distributed Transactions

A single database offers isolation levels (serializable, snapshot, read committed, etc.). Distributed transactions face additional challenges because participants execute in isolation locally.

### Snapshot Isolation
Each participant operates on a snapshot of data at the transaction's start time. Participants don't see changes from concurrent transactions.

**Advantage:** Prevents dirty reads and non-repeatable reads.
**Disadvantage:** Snapshot at participant A is from time T_A; snapshot at participant B is from time T_B ≠ T_A (clock skew). Transactions lose causality relationships.

### Serializable Isolation Across Nodes
Ensuring serializability across multiple participants requires that the global order of all transactions is consistent with local orders at each participant. This is expensive.

**Methods:**
- 2PC with strict two-phase locking: Participants lock all needed rows before the coordinator commits. Serializable but blocking.
- Spanner's approach (Google): Use TrueTime to assign exact timestamps to transactions globally. Serves consistent snapshots based on timestamps. Expensive (requires atomic clocks and GPS).
- Calvin's approach: Deterministic ordering of transactions via consensus. All participants execute transactions in the same order. Strong consistency without locking or TrueTime.

### Practical Isolation
Most distributed systems offer **snapshot isolation** per participant (each sees a consistent view locally) but **eventual consistency** globally (replicas converge gradually). Strong global serialization is rarely used because of cost.

## Calvin and Spanner Models

**Calvin (2012):** A database for partitioned data with deterministic transaction ordering.

- A global consensus layer (Paxos) orders transactions.
- All partitions execute transactions in the same order, deterministically.
- No 2PC needed; once ordered, transactions commit locally.
- Guarantees: Serializability + high throughput (no coordination delays).
- Trade-off: Transactions must be deterministic (no time.now() calls; use transaction timestamp instead).

**Spanner (Google):** A globally-distributed database with strong consistency.

- Uses Paxos for each data shard; coordinates across shards with 2PC.
- TrueTime (GPS + atomic clocks) provides globally consistent timestamps.
- Transactions use snapshots tagged with TrueTime, ensuring linearizability across data centers.
- Trade-off: Atomically synchronized clocks are expensive; not practical for most non-Google scale systems.

Both show extremes of the trade-off: Calvin prioritizes throughput (consensus then deterministic execution); Spanner prioritizes consistency and global timestamps (expensive, low latency).

## Coordination Overhead and Scalability

Distributed transactions incur coordination costs: round-trips for voting, consensus, etc.

**For 2PC:** 2 round-trips = 4 network hops (minimum). Each hop adds latency.

**For Saga:** Multiple requests to services over time; latency compounds, but no global blocking.

**Scaling:** As systems grow, coordination costs dominate. Most large-scale systems (Uber, Netflix, etc.) use **event-driven architecture** with eventual consistency rather than transactional coordination.

## Practical Guidance

| Scenario | Approach | Rationale |
|----------|----------|-----------|
| Small monolith + few external resources | 2PC or 3PC | Acceptable latency, explicit atomicity |
| Microservices, eventual consistency ok | Saga + Outbox | Decoupled, scalable |
| Critical financial transactions | 2PC + Saga hybrid | 2PC for critical path, saga for compensations |
| Global strong consistency required | Spanner-like (TrueTime) or Calvin (consensus) | Highest cost but maximum guarantees |
| High throughput, strict atomicity | Calvin | Deterministic ordering, no locking |

## See Also

- [distributed-consensus](distributed-consensus.md) — Coordination protocols underlying distributed transactions
- [distributed-replication](distributed-replication.md) — Data durability and replicas in transactional systems
- [distributed-clocks-ordering](distributed-clocks-ordering.md) — Timestamps and ordering for transaction ordering