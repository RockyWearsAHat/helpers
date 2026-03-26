# Database Concurrency Control: Locking Strategies, Deadlocks, and Patterns

## Pessimistic vs. Optimistic Concurrency Control

**Pessimistic** assumes conflicts are likely, so transactions lock resources before accessing them. A lock is held for the entire duration, blocking other transactions. PostgreSQL and MySQL default to pessimistic locking.

**Optimistic** assumes conflicts are rare, so transactions proceed without locks. At commit time, a conflict check verifies that no other transaction modified the data. If a conflict is detected, the transaction aborts and retries. Most effective when contention is low.

Choice depends on workload: high contention favors pessimism (reduce expensive retries); low contention favors optimism (avoid lock overhead). Many systems support both—explicit locks (SELECT FOR UPDATE) for pessimistic sections, version columns for optimistic checks.

## Lock Types and Granularity

**Row-Level Locks**: Lock individual rows. Fine-grained control; many transactions can modify different rows concurrently. PostgreSQL and MySQL use row locks by default. Overhead: more lock entries in memory, more lock management.

**Table-Level Locks**: Lock the entire table. Coarse-grained; no two transactions can modify the same table concurrently (though readers may not conflict). Older systems preferred this; modern systems use it sparingly (DDL typically acquires table locks).

**Page-Level Locks**: Intermediate granularity; lock a disk page (multiple rows). Rare in modern systems; InnoDB historically used page locks before adopting row locks.

**Advisory Locks**: Application-defined locks outside the data model. PostgreSQL supports `SELECT pg_advisory_lock(key)`, allowing applications to lock arbitrary resources. Useful for coordinating cross-table operations or external systems.

## Lock Modes

**Shared (Read) Lock**: Multiple transactions can hold. Prevents exclusive locks but allows other reads. `SELECT ... LOCK IN SHARE MODE` in MySQL.

**Exclusive (Write) Lock**: Only one transaction can hold. Blocks all others. `SELECT ... FOR UPDATE` in both PostgreSQL and MySQL.

**Update Lock**: Held by a statement that may update (e.g., `WHERE col = 5` finds the row first, then may update it). PostgreSQL uses `FOR SHARE` (shared) and `FOR UPDATE` (exclusive). MySQL distinguishes `FOR SHARE` and `FOR UPDATE`.

Conflict Matrix: Shared locks don't conflict with each other; exclusive locks conflict with all.

## Gap Locks and Next-Key Locks (InnoDB-Specific)

MySQL InnoDB uses gap locks to prevent phantom reads at REPEATABLE READ isolation. A gap lock locks the space *between* rows, not the rows themselves. When a transaction's WHERE clause scans rows 10-20, InnoDB locks the gap between all scanned values to prevent other transactions from inserting rows that would match the scan.

**Situation**: Transaction A scans `WHERE id BETWEEN 10 AND 20`. InnoDB locks rows 10 and 20, and the gap between. Transaction B tries to insert id=15. It's blocked by the gap lock.

**Next-Key Lock**: InnoDB combines row locks and gap locks—locks the row and the gap after it. On a scan that finds rows at positions 5, 15, 25, InnoDB locks [5-15), [15-25), and the gap after 25.

Trade-off: Gap locks prevent phantoms but reduce concurrency. They're conservative—they lock ranges the application may never collide on.

PostgreSQL's Serializable isolation uses a different approach: predicate locking (logically track which rows each transaction accessed) and detect conflicts at commit time rather than blocking preemptively.

## Deadlocks: Detection and Prevention

A deadlock occurs when two transactions wait for each other's locks. Transaction A holds lock on row 1 and waits for row 2; transaction B holds row 2 and waits for row 1. Neither can proceed.

**Detection**: Most systems have a deadlock detector that periodically checks for cycles in the wait-for graph. When detected, one transaction is chosen as the victim and rolled back. This breaks the cycle.

**Prevention** (application level):
- Lock rows in a consistent order (always lock user_id before order_id)
- Minimize lock duration (release locks early, don't hold across network I/O)
- Use lower isolation levels when possible (Read Committed has fewer lock conflicts than Repeatable Read)
- Break long transactions into smaller chunks

**Lock Timeout**: Instead of waiting indefinitely, a transaction can timeout if a lock isn't acquired within N seconds. PostgreSQL's `lock_timeout` parameter sets this. MySQL's `innodb_lock_wait_timeout` similarly. Timeout errors alert the application; it can retry or give up.

## Lock Escalation

Some systems automatically escalate locks when a transaction acquires too many row locks. Example: SQL Server escalates from row locks to page locks to table locks if lock count exceeds a threshold. This saves memory and simplifies locking but reduces concurrency.

PostgreSQL doesn't escalate; it stays granular regardless of lock count. Trade-off: PostgreSQL uses more memory under high contention but maintains fine-grained concurrency.

## SELECT FOR UPDATE Patterns

`SELECT ... FOR UPDATE` acquires an exclusive lock for the current transaction's duration. Common pattern for optimistic-to-pessimistic upgrades:

```sql
BEGIN;
SELECT * FROM account WHERE id = 5;  -- read-only, no lock
-- if modification needed:
SELECT * FROM account WHERE id = 5 FOR UPDATE;  -- exclusive lock
UPDATE account SET balance = ... WHERE id = 5;
COMMIT;
```

Also used for coordinating across multiple rows:
```sql
SELECT * FROM accounts WHERE type = 'checking' ORDER BY id FOR UPDATE;
UPDATE accounts SET interest = interest + 0.005 WHERE type = 'checking';
```

Locks are held until END TRANSACTION. Be careful: long-lived code between lock and commit can deadlock other transactions waiting for those rows.

PostgreSQL variants: `FOR NO KEY UPDATE` (lock without preventing row movement during updates), `FOR SHARE` (shared lock), `FOR KEY SHARE` (shared lock allowing structural changes). Most systems use FOR UPDATE for writes.

## Optimistic Concurrency: Version Columns and CAS

Instead of locking, check for conflicts at commit:

```sql
UPDATE account SET balance = 100, version = version + 1
WHERE id = 5 AND version = 3;  -- check version changed
```

If version has changed (another transaction updated the row), the UPDATE affects zero rows. The application detects this and retries.

Trade-off: No locks are held, so concurrency is high. But if conflicts are frequent, many transactions abort and retry—work is wasted. Optimistic schemes work best when contention is low.

Compare-and-swap (CAS) is a CPU primitive that atomically updates a value only if it matches an expected value. Databases emulate CAS via WHERE conditions. Application libraries (e.g., ORMs) often abstract this with optimistic version attributes.

## Monitoring Locks and Contention

PostgreSQL: `SELECT * FROM pg_locks` shows all held locks. Join with `pg_stat_statements` to identify which queries hold locks longest.

MySQL: `SHOW PROCESSLIST` shows transactions and their locked tables. `InnoDB MONITOR` (via `SHOW ENGINE INNODB STATUS`) details lock waits and deadlocks.

Long-running transactions are the #1 cause of lock contention. They hold locks longer than necessary, forcing others to wait. Query logs should flag transactions running > N seconds.

## Distributed Transactions and Consistency

When data spans multiple databases, locking must coordinate across systems. Two-Phase Commit (2PC) is the traditional approach: perform a prepare phase (all databases lock and verify they can commit), then a commit phase (all write). Cost: high latency (multiple round-trips), and failures mid-commit require recovery.

Saga pattern (compensating transactions) or event-driven consistency (async eventually consistent) are alternatives that avoid distributed locking but require careful design to handle partial failures and ensure application-level invariants.