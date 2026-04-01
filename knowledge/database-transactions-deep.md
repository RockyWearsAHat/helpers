# Database Transactions: ACID Properties, Isolation Levels, MVCC, and Anomalies

## ACID Properties: Implementation Perspectives

**Atomicity** (all-or-nothing) is typically implemented via write-ahead logging (WAL) and undo segments. A transaction's writes are first logged to disk before being applied to the buffer pool. If failure occurs, recovery replays the log to reconstruct the database state, ensuring partial writes never survive. Different systems implement undo differently: PostgreSQL uses a transaction ID (xid) tied to row visibility; MySQL InnoDB uses undo log segments. The cost is disk I/O for every write.

**Consistency** (invariant preservation) is largely application-enforced through constraints (foreign keys, CHECK clauses, uniqueness). The database ensures schema rules fire atomically—a foreign key check either passes or the entire transaction rolls back. However, application-level invariants (e.g., "orders.total must equal sum of order_item.amounts") remain the application's responsibility.

**Isolation** (concurrent transactions appear independent) is the most complex. Different isolation levels provide different degrees of separation. Perfect isolation requires serializable execution, which kills concurrency; practical databases weaken this via MVCC and isolation levels.

**Durability** (committed writes survive failures) is guaranteed by WAL: data is written to persistent storage before a COMMIT returns. However, some systems offer tunable durability (PostgreSQL's `synchronous_commit`). Risk: an OS crash can corrupt both memory and unflushed writes if fsync is disabled.

## Isolation Levels: The SQL Standard and Reality

The SQL:1992 standard defines four levels based on prohibited anomalies:

**Read Uncommitted**: Allows dirty reads (reading uncommitted writes from other transac­tions). In practice, PostgreSQL and MySQL don't truly implement this; PostgreSQL maps it to Read Committed. Worth noting: true Read Uncommitted is rarely used outside specialized analytics workloads with approximate results.

**Read Committed** (PostgreSQL default): Prevents dirty reads but allows non-repeatable reads (re-reading data finds new values from committed transactions) and phantom reads (re-running a query finds new rows). Each statement sees a fresh snapshot of committed data. It's the safest default that doesn't cripple concurrency—most real systems use this.

**Repeatable Read** (MySQL InnoDB default): A transaction gets a single snapshot at BEGIN time. Repeated selects within that transaction see the same data, even if other transactions commit changes. Phantom reads are theoretically allowed but PostgreSQL's implementation prevents them via predicate locking. MySQL InnoDB also prevents phantoms through next-key locking. The tradeoff: slightly higher contention than Read Committed.

**Serializable**: Guarantees that concurrent execution is equivalent to serial execution. Implementations vary: PostgreSQL uses Serializable Snapshot Isolation (SSI), checking for conflicts at commit time; MySQL InnoDB adds gap locks aggressively. Cost: significant performance hit under contention. Most applications don't need it outside financial transactions.

Trade-off asymmetry: Lower isolation levels are faster but riskier; higher levels are safer but slower. No level is uniformly "correct"—the choice depends on application invariants.

## MVCC: Multiversion Concurrency Control

MVCC allows readers and writers to execute concurrently by maintaining multiple versions of each row. Rather than blocking, a transaction sees a consistent snapshot of the database as it existed at a point in time.

**PostgreSQL MVCC**: Each row carries `xmin` (inserting transaction) and `xmax` (deleting transaction). When a transaction begins, it gets a transaction ID (xid) and snapshot of active transactions. Reads check visibility: if xmin is committed and before the snapshot, and xmax is not committed or after the snapshot, the row is visible. Updates don't modify in-place; a new version is inserted with new xmin, and the old row is marked deleted.

**MySQL InnoDB MVCC**: Rows carry implicit transaction IDs in the transaction ID field of the rollback segment. Since MySQL doesn't have explicit transaction snapshots, it constructs a read view when a SELECT or locking statement is executed. READ COMMITTED rebuilds the read view for each statement; REPEATABLE READ uses the same view for the transaction. Undo logs retain old versions for crash recovery and consistent reads.

Both systems face an undo log growth problem: old versions must be retained until all transactions that might read them have finished. Long-running transactions cause "bloat"—excessive dead rows and undo log accumulation.

## MVCC Weaknesses and Serialization Anomalies

MVCC prevents dirty reads and most non-repeatable reads but does NOT prevent all anomalies at higher isolation levels.

**Write Skew** (serialization anomaly): Two transactions each read overlapping data, make independent decisions, and write conflicting results. Example: A doctor-on-call scheduler has two doctors; two concurrent transactions each confirm the other is on call, then both update to take the shift. Both commit, but the invariant "at least one doctor on call" is violated. Read Committed and Repeatable Read don't detect this; it's a serialization anomaly.

**Phantom Reads**: A transaction executes a range query twice and gets different row counts due to concurrent inserts. PostgreSQL REPEATABLE READ prevents this via predicate locking (locks on range conditions, not just individual rows). MySQL InnoDB prevents it via next-key locking (locks the next row after the scan range). Both are conservative—they lock more than necessary.

**Serializable Snapshot Isolation (SSI)** (PostgreSQL's Serializable mode): Rather than locking, SSI detects at commit time whether a transaction's reads and writes conflict with other concurrent transactions. If a conflict is detected, one transaction aborts with a serialization failure. This allows read-only transactions to proceed without blocking.

## Index Visibility and Consistency

Indexes in MVCC systems must handle invisible rows gracefully. An index entry points to a row version that might not be visible to the current transaction. Index scans must still check row visibility via the heap of the table (the main storage), not just the index. This means index-only scans (returning data directly from index) require covering indexes (indexes that include all projected columns) to avoid heap lookups.

## Practical Patterns and Tradeoffs

Most real-world applications use Read Committed + explicit locking (SELECT FOR UPDATE) rather than relying on higher isolation levels. Explicit locks are predictable and often faster than the overhead of Serializable anomaly detection.

Applications that need strong consistency across multiple rows often move logic out of the database: application code reads data, performs computations, and uses optimistic concurrency control (version columns, CAS operations) to detect conflicts. This avoids serialization anomalies at the cost of explicit application logic.

Long-running transactions are problematic in MVCC systems. They extend the oldest snapshot, forcing the system to retain old versions indefinitely. Break long transactions into smaller chunks, or use read-only transaction snapshots explicitly.

## Cross-Database Variation

Most databases (Oracle, SQL Server, PostgreSQL, MySQL) implement similar ACID and isolation concepts but differ in defaults and performance characteristics. Oracle's Read Committed allows non-repeatable reads; SQL Server's default is Read Committed but with range locking under the hood. Always verify isolation behavior rather than assuming portability.