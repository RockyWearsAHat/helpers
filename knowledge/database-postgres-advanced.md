# PostgreSQL Advanced — MVCC, Vacuuming, Locks, and Extensions

## MVCC: Multi-Version Concurrency Control

PostgreSQL uses MVCC to enable concurrent reads and writes without blocking. Every tuple (row) is tagged with transaction IDs (`xmin` and `xmax`).

- **`xmin`**: Transaction ID that inserted the tuple. Row visible to transactions with ID ≥ xmin AND where xmin is committed.
- **`xmax`**: Transaction ID that deleted/updated the tuple (or 0 if not deleted). Row visible only to transactions before xmax.

### MVCC Visibility Rules

```
Row visible to transaction T if:
  xmin(row) <= T AND xmin(row) is committed
  AND
  (xmax(row) = 0 OR xmax(row) > T OR xmax(row) is not committed)
```

Each transaction sees a **snapshot** — a consistent view of which transactions are committed — at its start. Long-running transactions hold this snapshot and keep old tuple versions alive.

### Transaction Isolation Levels

| Level               | Dirty Read | Non-Repeatable Read | Phantom | Implementation      |
| ------------------- | ---------- | ------------------- | ------- | ------------------- |
| Read Uncommitted    | —          | —                   | —       | Not available (PG enforces Repeatable Read minimum) |
| Read Committed      | No         | Yes                 | Yes     | Snapshot at stmt start (default) |
| Repeatable Read     | No         | No                  | Yes*    | Snapshot at txn start; "phantom" edge case |
| Serializable        | No         | No                  | No      | Serialization conflict detection (SSI) |

PostgreSQL's **Serializable** isolation (SQL:2008 standard) uses **Serialization Snapshot Isolation (SSI)**, not explicit locks. It detects read-write conflicts and aborts txns as needed.

## Vacuuming and Autovacuum

Every `UPDATE` or `DELETE` creates a new tuple version or marks the old one for deletion (via `xmax`). Old versions accumulate, bloating the table. **Vacuuming** reclaims space.

### VACUUM Modes

```sql
-- Remove dead tuples; mark space for reuse (not returned to OS)
VACUUM some_table;

-- Aggressive vacuum with index cleanup
VACUUM ANALYZE some_table;

-- FULL: rewrite table, return space to OS (locks table, slow)
VACUUM FULL some_table;

-- Check bloat without modifying
SELECT schemaname, tablename, pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname NOT IN ('pg_catalog', 'information_schema')
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

### Autovacuum Tuning

PostgreSQL runs `autovacuum` daemon to vacuum tables based on threshold. Tuning parameters:

```sql
-- View current settings
SELECT name, setting FROM pg_settings WHERE name LIKE 'autovacuum%';

-- Per-table settings (in pg_class):
ALTER TABLE some_table SET (
  autovacuum_vacuum_scale_factor = 0.05,      -- vacuum when 5% of tuples are dead
  autovacuum_vacuum_threshold = 1000,         -- or after 1000 dead tuples
  autovacuum_analyze_scale_factor = 0.01,     -- reanalyze at 1% dead tuples
  autovacuum_naptime = '10s'                  -- vacuum check interval
);
```

### TOAST: Out-of-Line Storage

Large values (text, JSONB, arrays > ~2KB) are stored in a separate **TOAST** table (The Oversized-Attribute Storage Technique). Automatic; no configuration needed.

```
PRIMARY TABLE: user_id (int), name (text), email (text)
TOAST TABLE: used only when values exceed TOAST_TUPLE_THRESHOLD
```

TOAST is transparent but important for understanding disk usage and vacuum behavior — TOAST tables can bloat independently.

## Locking

### Lock Types and Modes

| Mode       | Use Case                                  | Conflicts With |
| ---------- | ----------------------------------------- | -------------- |
| AccessShare | SELECT; non-blocking reads default        | ExclusiveLock  |
| RowShare   | SELECT FOR SHARE (share row locks)        | Exclusive      |
| RowExclusive | UPDATE, DELETE, INSERT                  | All write modes |
| Share      | CREATE INDEX                              | RowExclusive, Exclusive |
| ExclusiveLock | DDL (ALTER TABLE); txn lock             | Most modes     |

### Advisory Locks (Application-Level)

Explicit, user-defined locks for coordinating work across connections.

```sql
-- Acquire lock (blocking)
SELECT pg_advisory_lock(key INT8);

-- Try lock (non-blocking, returns boolean)
SELECT pg_advisory_xact_lock(key INT8);  -- transaction-scoped
SELECT pg_advisory_lock(key INT8);       -- session-scoped

-- Release (session-scoped must be explicit; xact-scoped auto-release on commit/rollback)
SELECT pg_advisory_unlock(key INT8);
SELECT pg_advisory_unlock_all();

-- Example: Distributed job locking
-- Only one worker acquires the lock and processes the job
IF pg_try_advisory_lock(12345) THEN
  -- Process job
  UPDATE jobs SET status = 'completed' WHERE id = 12345;
  SELECT pg_advisory_unlock(12345);
ELSE
  -- Another worker already has it
  RAISE NOTICE 'Job already being processed';
END IF;
```

## Row-Level Security (RLS)

Enable fine-grained access control per row based on session context.

```sql
CREATE TABLE documents (
  id uuid PRIMARY KEY,
  owner_id uuid NOT NULL,
  title text,
  content text
);

-- Enable RLS on the table
ALTER TABLE documents ENABLE ROW LEVEL SECURITY;

-- Policy: users can only see their own documents
CREATE POLICY user_documents ON documents
  FOR SELECT
  USING (owner_id = current_user_id);  -- session variable

-- Restrictive policy: users can only update their own docs
CREATE POLICY user_update_documents ON documents
  FOR UPDATE
  USING (owner_id = current_user_id)
  WITH CHECK (owner_id = current_user_id);

-- Superusers bypass RLS; roles with BYPASSRLS attribute bypass it
ALTER ROLE app_user SET app.user_id = 'uuid-here';  -- set session var
```

## LISTEN/NOTIFY: Pub/Sub in PostgreSQL

Real-time notification system for inter-process communication.

```sql
-- Publisher: send notification
NOTIFY channel_name, 'payload as string';

-- Subscriber: listen in another connection
LISTEN channel_name;

-- Receive notifications (blocking call in libpq or drivers)
SELECT pg_sleep(3600);  -- simulate listening; wait for notifications

-- Example: invalidate cache on data change
-- Trigger fires on insert/update/delete
CREATE FUNCTION notify_data_change() RETURNS TRIGGER AS $$
BEGIN
  PERFORM pg_notify('cache_invalidate', NEW.id::text);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER data_changed AFTER INSERT OR UPDATE ON documents
FOR EACH ROW EXECUTE FUNCTION notify_data_change();

-- Application listens and clears cache on notification
```

## Foreign Data Wrappers (FDW)

Query remote databases, APIs, or filesystems as if they were local tables.

```sql
-- Create foreign server connection
CREATE EXTENSION IF NOT EXISTS postgres_fdw;

CREATE SERVER remote_db FOREIGN DATA WRAPPER postgres_fdw
  OPTIONS (host 'remote.example.com', dbname 'remote_db', port '5432');

CREATE USER MAPPING FOR current_user SERVER remote_db
  OPTIONS (user 'remote_user', password 'secret');

-- Create foreign table (schema must match remote table)
CREATE FOREIGN TABLE remote_users (
  id int,
  name text,
  email text
) SERVER remote_db
OPTIONS (schema_name 'public', table_name 'users');

-- Query as local table (Postgres pushes WHERE/LIMIT down)
SELECT * FROM remote_users WHERE id > 100;

-- Common FDWs: postgres_fdw, mysql_fdw, mongodb_fdw (community), redis_fdw (community)
```

## Extensions

Extend PostgreSQL with additional types, functions, or operators.

| Extension   | Purpose                              |
| ----------- | ------------------------------------ |
| `uuid-ossp` | UUID generation (`gen_random_uuid()` now built-in since PG13) |
| `hstore`    | Key-value type (prefer JSONB)        |
| `PostGIS`   | Spatial/GIS types and functions      |
| `pg_trgm`   | Trigram indexing for fuzzy text search |
| `ltree`     | Tree (hierarchical path) type        |
| `pgcrypto`  | Cryptographic functions             |
| `intarray`  | Array functions                      |

### PostGIS Example

```sql
CREATE EXTENSION postgis;

CREATE TABLE locations (
  id serial PRIMARY KEY,
  name text,
  geom geography(POINT, 4326)  -- lon/lat
);

INSERT INTO locations VALUES (1, 'NYC', ST_Point(-74.0060, 40.7128));

-- Distance in meters
SELECT name, ST_Distance(geom, ST_Point(-74.0, 40.71)::geography) AS dist_m
FROM locations
WHERE ST_Distance(geom, ST_Point(-74.0, 40.71)::geography) < 10000;
```

### pg_trgm for Fuzzy Search

```sql
CREATE EXTENSION pg_trgm;
CREATE INDEX idx_name_trgm ON users USING gin (name gin_trgm_ops);

SELECT name FROM users WHERE name % 'jon';  -- similarity operator, uses index
SELECT name, similarity(name, 'jon') FROM users ORDER BY similarity DESC;
```

## Logical Replication

Stream changes from a **publication** (primary) to **subscribers** (replicas).

```sql
-- Primary: create publication (one or more tables)
CREATE PUBLICATION pub_main FOR TABLE users, orders;

-- Subscriber: create subscription
CREATE SUBSCRIPTION sub_main CONNECTION 'connstr to primary'
  PUBLICATION pub_main;

-- Subscriber can have local tables; replication is one-way (primary → subscriber)
```

Unlike physical streaming replication (WAL-based), logical replication:
- Is **database-level** (not instance-level); can replicate to different schema
- Supports **multi-master** (each subscriber can publish its own changes)
- Is **asynchronous** (slight lag acceptable)
- Works across versions and platforms (more portable)

## Performance Tuning

### Key Parameters

```sql
-- Memory
shared_buffers = 256MB    -- 25% of RAM for most workloads
effective_cache_size = 8GB  -- helps planner; set close to OS cache + buffers
work_mem = 64MB           -- per operation (GROUP BY, sort, hash join)

-- Parallelism
max_parallel_workers_per_gather = 4
max_parallel_workers = 8

-- WAL and Checkpoints
checkpoint_timeout = 15min
max_wal_size = 4GB

-- Autovacuum (see earlier section)
autovacuum = on
autovacuum_naptime = 10s
```

### Query Analysis

```sql
-- Explain with actual runtime stats (costs are estimates)
EXPLAIN ANALYZE SELECT ...;

-- Check for sequential scans on large tables
SELECT schemaname, tablename, idx_scan, seq_scan
FROM pg_stat_user_tables
WHERE seq_scan > 1000
ORDER BY seq_scan DESC;

-- Missing indexes
SELECT schemaname, tablename, attname FROM pg_stat_user_columns
WHERE null_frac > 0.5 AND n_distinct > 100
  AND indexrelname IS NULL;
```

## See Also

- [Database Transactions: ACID, Isolation Levels, MVCC](database-transactions-deep.md)
- [Database Concurrency Control](database-concurrency.md)
- [Database Indexing Strategies](database-indexing-strategies.md)
- [PostgreSQL Deep Dive](database-postgresql.md)