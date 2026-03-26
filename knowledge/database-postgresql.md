# PostgreSQL Deep Dive

## Data Types

### JSONB

Binary JSON with indexing support. Preferred over `json` for nearly all use cases.

```sql
-- JSONB operators
SELECT data->'name' FROM users;          -- returns JSON
SELECT data->>'name' FROM users;         -- returns text
SELECT data#>'{address,city}' FROM users; -- nested path (JSON)
SELECT data#>>'{address,city}' FROM users; -- nested path (text)
SELECT data @> '{"role":"admin"}' FROM users; -- containment
SELECT data ? 'email' FROM users;        -- key exists
SELECT data ?| array['email','phone'] FROM users; -- any key exists
SELECT data ?& array['email','phone'] FROM users; -- all keys exist

-- JSONB modification
UPDATE users SET data = data || '{"verified":true}';
UPDATE users SET data = data - 'temp_field';
UPDATE users SET data = jsonb_set(data, '{address,zip}', '"90210"');
```

### Arrays

```sql
CREATE TABLE posts (tags text[]);
INSERT INTO posts VALUES ('{go,backend,api}');
SELECT * FROM posts WHERE tags @> '{go}';      -- contains
SELECT * FROM posts WHERE tags && '{go,rust}'; -- overlap (any match)
SELECT unnest(tags) FROM posts;                -- expand to rows
SELECT array_agg(name) FROM users GROUP BY dept; -- aggregate to array
```

### Range Types

```sql
CREATE TABLE reservations (
  during tstzrange NOT NULL,
  EXCLUDE USING gist (during WITH &&)  -- no overlapping ranges
);
INSERT INTO reservations VALUES ('[2024-01-01, 2024-01-05)');
SELECT * FROM reservations WHERE during @> '2024-01-03'::timestamptz;
```

Built-in: `int4range`, `int8range`, `numrange`, `tsrange`, `tstzrange`, `daterange`.

### Other Notable Types

| Type                   | Use Case                                                 |
| ---------------------- | -------------------------------------------------------- |
| `hstore`               | Simple key-value pairs (prefer jsonb for new code)       |
| `tsvector` / `tsquery` | Full-text search tokens and queries                      |
| `citext`               | Case-insensitive text                                    |
| `uuid`                 | Use `gen_random_uuid()` (built-in since PG13)            |
| `inet` / `cidr`        | IP addresses and networks                                |
| `composite`            | Custom structured types (`CREATE TYPE address AS (...)`) |
| `enum`                 | Static sets — hard to modify, use sparingly              |

## Indexing

### Index Types

| Type    | Best For                                    | Operators                                 |
| ------- | ------------------------------------------- | ----------------------------------------- |
| B-tree  | Equality, range, sorting (default)          | `=`, `<`, `>`, `BETWEEN`, `IN`, `IS NULL` |
| Hash    | Equality only                               | `=`                                       |
| GiST    | Geometric, range, full-text, ltree          | `&&`, `@>`, `<@`, `<<`, `>>`              |
| GIN     | Multi-valued (arrays, jsonb, tsvector)      | `@>`, `?`, `?&`, `?\|`, `@@`              |
| BRIN    | Large naturally-ordered tables (timestamps) | `<`, `>`, `=`                             |
| SP-GiST | Non-balanced structures (phone numbers, IP) | varies                                    |

### Advanced Index Techniques

```sql
-- Partial index: only index what you query
CREATE INDEX idx_active_users ON users (email) WHERE active = true;

-- Expression index: index computed values
CREATE INDEX idx_lower_email ON users (lower(email));

-- Covering index (INCLUDE): avoid heap fetches
CREATE INDEX idx_user_lookup ON users (email) INCLUDE (name, created_at);

-- Multicolumn: leftmost prefix rule applies
CREATE INDEX idx_multi ON orders (customer_id, created_at DESC);

-- GIN for JSONB
CREATE INDEX idx_data ON users USING gin (data);
CREATE INDEX idx_data_path ON users USING gin (data jsonb_path_ops); -- containment only, smaller

-- BRIN for time-series
CREATE INDEX idx_ts ON events USING brin (created_at) WITH (pages_per_range = 32);
```

### Index Maintenance

```sql
REINDEX INDEX CONCURRENTLY idx_name;  -- rebuild without locking
SELECT pg_size_pretty(pg_indexes_size('tablename'));
-- Check bloat: pgstattuple extension
SELECT * FROM pgstatindex('idx_name');
```

## Query Planner

### EXPLAIN ANALYZE

```sql
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) SELECT ...;
```

Key metrics: `actual time` (first row..last row in ms), `rows`, `loops`, `Buffers: shared hit/read`.

### Join Strategies

| Strategy    | When Used                              |
| ----------- | -------------------------------------- |
| Nested Loop | Small outer table, indexed inner table |
| Hash Join   | No useful index, medium-large tables   |
| Merge Join  | Both inputs sorted (or can be)         |

### Planner Hints (indirect)

```sql
SET enable_seqscan = off;          -- force index use (debugging only)
SET random_page_cost = 1.1;        -- SSD-appropriate (default 4.0)
SET effective_cache_size = '8GB';   -- hint about OS cache
SET work_mem = '256MB';            -- per-sort/hash memory
```

## CTEs and Recursive Queries

```sql
-- Recursive CTE: org chart traversal
WITH RECURSIVE org_tree AS (
  SELECT id, name, manager_id, 0 AS depth
  FROM employees WHERE manager_id IS NULL
  UNION ALL
  SELECT e.id, e.name, e.manager_id, ot.depth + 1
  FROM employees e JOIN org_tree ot ON e.manager_id = ot.id
)
SELECT * FROM org_tree ORDER BY depth, name;
```

**CTE materialization** (PG12+): CTEs are optimization fences by default. Use `NOT MATERIALIZED` to let the planner inline:

```sql
WITH cte AS NOT MATERIALIZED (SELECT ...) SELECT * FROM cte WHERE ...;
```

## Window Functions

```sql
SELECT
  name,
  department,
  salary,
  ROW_NUMBER() OVER w AS row_num,
  RANK() OVER w AS rank,
  DENSE_RANK() OVER w AS dense_rank,
  NTILE(4) OVER w AS quartile,
  LAG(salary) OVER w AS prev_salary,
  LEAD(salary) OVER w AS next_salary,
  SUM(salary) OVER (PARTITION BY department) AS dept_total,
  SUM(salary) OVER (ORDER BY hire_date ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS running_total
FROM employees
WINDOW w AS (PARTITION BY department ORDER BY salary DESC);
```

## LISTEN / NOTIFY

Lightweight pub/sub built into PostgreSQL.

```sql
-- Session 1
LISTEN order_events;

-- Session 2
NOTIFY order_events, '{"order_id": 42, "status": "shipped"}';

-- Or from a trigger:
CREATE FUNCTION notify_order_change() RETURNS trigger AS $$
BEGIN
  PERFORM pg_notify('order_events', row_to_json(NEW)::text);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;
```

Caveats: payload max 8000 bytes, messages lost if no listener, not durable — use as a signal layer, not a message queue.

## Advisory Locks

Application-level distributed locks without table rows.

```sql
-- Session-level (held until release or disconnect)
SELECT pg_advisory_lock(hashtext('import_job'));
-- ... do work ...
SELECT pg_advisory_unlock(hashtext('import_job'));

-- Transaction-level (auto-released at commit/rollback)
SELECT pg_advisory_xact_lock(42);

-- Try without blocking
SELECT pg_try_advisory_lock(42); -- returns bool
```

Use case: prevent duplicate cron jobs, serialize resource access, implement idempotent operations.

## Table Partitioning

```sql
-- Declarative partitioning (PG10+)
CREATE TABLE measurements (
  id bigint GENERATED ALWAYS AS IDENTITY,
  ts timestamptz NOT NULL,
  value double precision
) PARTITION BY RANGE (ts);

CREATE TABLE measurements_2024_q1 PARTITION OF measurements
  FOR VALUES FROM ('2024-01-01') TO ('2024-04-01');

-- List partitioning
CREATE TABLE orders (...) PARTITION BY LIST (region);
CREATE TABLE orders_us PARTITION OF orders FOR VALUES IN ('us-east', 'us-west');

-- Hash partitioning (even distribution)
CREATE TABLE sessions (...) PARTITION BY HASH (user_id);
CREATE TABLE sessions_0 PARTITION OF sessions FOR VALUES WITH (MODULUS 4, REMAINDER 0);
```

Attach/detach without locking the parent: `ALTER TABLE measurements DETACH PARTITION old_part CONCURRENTLY;`

## Performance Monitoring

### pg_stat_statements

```sql
CREATE EXTENSION pg_stat_statements;
SELECT query, calls, mean_exec_time, rows, shared_blks_hit, shared_blks_read
FROM pg_stat_statements ORDER BY total_exec_time DESC LIMIT 20;
```

### Key System Views

| View                   | Shows                                                    |
| ---------------------- | -------------------------------------------------------- |
| `pg_stat_user_tables`  | Seq scans, index scans, dead tuples, last vacuum/analyze |
| `pg_stat_user_indexes` | Index usage counts                                       |
| `pg_stat_activity`     | Active queries, wait events                              |
| `pg_locks`             | Current locks                                            |
| `pg_stat_replication`  | Replication lag                                          |

## Vacuuming and MVCC

PostgreSQL uses **Multi-Version Concurrency Control**: readers never block writers, writers never block readers. Dead tuples accumulate from updates/deletes.

- **VACUUM**: reclaims dead tuple space for reuse (doesn't return to OS)
- **VACUUM FULL**: rewrites table, returns space to OS (locks table exclusively)
- **autovacuum**: background process, tune `autovacuum_vacuum_scale_factor` (default 20%)

```sql
-- Check bloat
SELECT relname, n_dead_tup, n_live_tup,
       round(n_dead_tup::numeric / greatest(n_live_tup, 1) * 100, 1) AS dead_pct
FROM pg_stat_user_tables ORDER BY n_dead_tup DESC;
```

**Transaction ID wraparound**: PostgreSQL uses 32-bit XIDs. Autovacuum's anti-wraparound mode freezes old tuples. If it falls behind, the database will refuse writes at ~2 billion XIDs remaining. Monitor `age(relfrozenxid)`.

## Logical Replication

```sql
-- Publisher
CREATE PUBLICATION my_pub FOR TABLE orders, customers;
-- or FOR ALL TABLES;

-- Subscriber
CREATE SUBSCRIPTION my_sub
  CONNECTION 'host=pub_host dbname=mydb'
  PUBLICATION my_pub;
```

- Replicates DML (INSERT, UPDATE, DELETE), not DDL
- Tables need `REPLICA IDENTITY` (default uses primary key)
- Supports cross-version replication
- Use cases: zero-downtime upgrades, analytics replicas, selective replication

## Row-Level Security

```sql
ALTER TABLE documents ENABLE ROW LEVEL SECURITY;

CREATE POLICY user_isolation ON documents
  USING (owner_id = current_setting('app.user_id')::int);

CREATE POLICY admin_all ON documents
  TO admin_role USING (true);

-- Force RLS even for table owners
ALTER TABLE documents FORCE ROW LEVEL SECURITY;

-- Set context per request
SET app.user_id = '42';
SELECT * FROM documents; -- only sees own rows
```

## Full-Text Search

```sql
-- Create tsvector column with index
ALTER TABLE articles ADD COLUMN search_vec tsvector
  GENERATED ALWAYS AS (to_tsvector('english', coalesce(title,'') || ' ' || coalesce(body,''))) STORED;
CREATE INDEX idx_fts ON articles USING gin (search_vec);

-- Query
SELECT title, ts_rank(search_vec, q) AS rank
FROM articles, to_tsquery('english', 'postgres & replication') q
WHERE search_vec @@ q
ORDER BY rank DESC;

-- Phrase search (PG9.6+)
SELECT * FROM articles WHERE search_vec @@ phraseto_tsquery('english', 'logical replication');
```

## Connection Pooling with PgBouncer

| Mode        | Description                         | Best For                           |
| ----------- | ----------------------------------- | ---------------------------------- |
| Session     | 1:1 mapping for connection lifetime | LISTEN/NOTIFY, prepared statements |
| Transaction | Returns to pool after transaction   | Most web apps (default choice)     |
| Statement   | Returns after each statement        | Simple read loads, pgbench         |

Critical settings: `max_client_conn`, `default_pool_size`, `reserve_pool_size`, `server_idle_timeout`.

Rule of thumb: set PG `max_connections` to `(CPU cores * 2) + effective_spindle_count`, let PgBouncer handle thousands of app connections.
