# MySQL 8

## InnoDB Architecture

### Buffer Pool

The central caching mechanism for data and indexes. Pages are 16KB by default.

```ini
# my.cnf — set to ~70-80% of available RAM for dedicated DB servers
innodb_buffer_pool_size = 12G
innodb_buffer_pool_instances = 8  # one per GB (max 64)
```

Monitor hit ratio — should be >99%:

```sql
SHOW STATUS LIKE 'Innodb_buffer_pool_read%';
-- hit ratio = 1 - (Innodb_buffer_pool_reads / Innodb_buffer_pool_read_requests)
```

The buffer pool uses a modified LRU with a midpoint insertion strategy: new pages go to the 3/8 mark (not the head), preventing full table scans from evicting hot pages.

### Redo Log (WAL)

Write-ahead log for crash recovery. Transactions write to the redo log before modifying data pages.

```ini
innodb_redo_log_capacity = 4G       # MySQL 8.0.30+ (replaces log file size/count)
innodb_flush_log_at_trx_commit = 1  # ACID-compliant (fsync every commit)
                                     # = 2: fsync every second (better perf, ~1s data risk)
                                     # = 0: log written to buffer only
```

### Undo Log

Stores previous versions of rows for MVCC and rollback. Lives in the system tablespace or dedicated undo tablespaces.

```sql
-- Check undo space usage
SELECT * FROM information_schema.innodb_tablespaces WHERE space_type = 'Undo';
```

Long-running transactions prevent undo purge → undo log growth → disk pressure. Monitor `trx_rows_modified` in `information_schema.innodb_trx`.

### Clustered Index

InnoDB stores table data in the primary key's B+tree — the table IS the index. Implications:

- **Defining a PK is strongly recommended**: InnoDB creates a hidden 6-byte row ID otherwise
- **Short PKs save space**: secondary indexes store the PK value (not a pointer)
- **Auto-increment PKs**: sequential inserts → no page splits, great write performance
- **UUID PKs**: random inserts → page splits, ~30-40% overhead vs auto-increment
- **UUID v7** (time-ordered): best of both worlds if you need UUIDs

## Indexing

### B+Tree Indexes (default)

```sql
-- Composite index — leftmost prefix rule
CREATE INDEX idx_name ON orders (customer_id, status, created_at);
-- Usable for: (customer_id), (customer_id, status), (customer_id, status, created_at)
-- NOT usable for: (status), (status, created_at)

-- Covering index (all columns in query satisfied by index)
-- InnoDB secondary indexes include PK columns automatically
CREATE INDEX idx_covering ON orders (customer_id, status) /* implicitly includes PK */;

-- Descending index (MySQL 8.0+)
CREATE INDEX idx_recent ON events (user_id, created_at DESC);

-- Invisible index (test dropping without actually dropping)
ALTER TABLE orders ALTER INDEX idx_name INVISIBLE;
-- Check if queries still perform, then:
ALTER TABLE orders ALTER INDEX idx_name VISIBLE; -- or DROP INDEX
```

### Other Index Types

| Type       | Syntax                                                           | Use Case                          |
| ---------- | ---------------------------------------------------------------- | --------------------------------- |
| Fulltext   | `FULLTEXT INDEX (col)`                                           | Natural language text search      |
| Spatial    | `SPATIAL INDEX (col)`                                            | Geometry/geography data           |
| Functional | `CREATE INDEX idx ON t ((CAST(j->>'$.price' AS DECIMAL(10,2))))` | Index expressions (8.0.13+)       |
| Hash       | Only in MEMORY engine                                            | Exact-match lookups (not on disk) |

### Index Hints

```sql
SELECT * FROM orders USE INDEX (idx_customer) WHERE customer_id = 42;
SELECT * FROM orders FORCE INDEX (idx_customer) WHERE customer_id = 42;
SELECT * FROM orders IGNORE INDEX (idx_status) WHERE status = 'pending';
```

## Query Optimizer

### EXPLAIN and EXPLAIN ANALYZE

```sql
EXPLAIN FORMAT=TREE SELECT ...;    -- cost-based tree (8.0.16+)
EXPLAIN ANALYZE SELECT ...;        -- actually executes, shows real times (8.0.18+)
```

Key `type` values (best to worst): `system` > `const` > `eq_ref` > `ref` > `range` > `index` > `ALL`.

### Optimizer Hints (inline)

```sql
SELECT /*+ JOIN_ORDER(o, c) */ o.id, c.name
FROM orders o JOIN customers c ON o.customer_id = c.id;

/*+ BNL(t1, t2) */       -- block nested loop
/*+ NO_BNL(t1, t2) */    -- avoid BNL
/*+ HASH_JOIN(t1, t2) */ -- use hash join (8.0.18+)
/*+ INDEX(t idx_name) */ -- force specific index
/*+ NO_INDEX(t idx) */   -- exclude index
/*+ SET_VAR(sort_buffer_size=16M) */ -- per-query variable
```

### Optimizer Trace

```sql
SET optimizer_trace = 'enabled=on';
SELECT ...;
SELECT * FROM information_schema.optimizer_trace\G
SET optimizer_trace = 'enabled=off';
```

## Window Functions (MySQL 8.0+)

```sql
SELECT
  name, department, salary,
  ROW_NUMBER() OVER w AS rownum,
  RANK() OVER w AS rnk,
  DENSE_RANK() OVER w AS dense_rnk,
  NTILE(4) OVER w AS quartile,
  LAG(salary, 1) OVER w AS prev_salary,
  LEAD(salary, 1) OVER w AS next_salary,
  FIRST_VALUE(name) OVER w AS top_earner,
  SUM(salary) OVER (PARTITION BY department) AS dept_total,
  CUME_DIST() OVER w AS cumulative_dist,
  PERCENT_RANK() OVER w AS pct_rank
FROM employees
WINDOW w AS (PARTITION BY department ORDER BY salary DESC);

-- Frame clause
SUM(amount) OVER (ORDER BY dt ROWS BETWEEN 6 PRECEDING AND CURRENT ROW) -- 7-day rolling sum
AVG(amount) OVER (ORDER BY dt RANGE BETWEEN INTERVAL 7 DAY PRECEDING AND CURRENT ROW) -- range-based
```

## Common Table Expressions

```sql
-- Non-recursive
WITH regional_sales AS (
  SELECT region, SUM(amount) AS total FROM orders GROUP BY region
)
SELECT * FROM regional_sales WHERE total > 100000;

-- Recursive (graph traversal, hierarchies)
WITH RECURSIVE category_tree AS (
  SELECT id, name, parent_id, 0 AS depth, CAST(name AS CHAR(1000)) AS path
  FROM categories WHERE parent_id IS NULL
  UNION ALL
  SELECT c.id, c.name, c.parent_id, ct.depth + 1, CONCAT(ct.path, ' > ', c.name)
  FROM categories c JOIN category_tree ct ON c.parent_id = ct.id
)
SELECT * FROM category_tree ORDER BY path;
```

CTE limit: `cte_max_recursion_depth` (default 1000).

## JSON Support

```sql
-- JSON column
CREATE TABLE events (
  id BIGINT AUTO_INCREMENT PRIMARY KEY,
  payload JSON NOT NULL
);

-- Extraction
SELECT payload->'$.user.name' FROM events;       -- returns JSON
SELECT payload->>'$.user.name' FROM events;      -- returns text (8.0.21+)
SELECT JSON_EXTRACT(payload, '$.user.name') FROM events;
SELECT JSON_UNQUOTE(JSON_EXTRACT(payload, '$.user.name')) FROM events;

-- Modification
UPDATE events SET payload = JSON_SET(payload, '$.processed', true);
UPDATE events SET payload = JSON_REMOVE(payload, '$.temp');
UPDATE events SET payload = JSON_ARRAY_APPEND(payload, '$.tags', 'new_tag');

-- Multi-valued index (8.0.17+) — index array elements
CREATE INDEX idx_tags ON events ((CAST(payload->'$.tags' AS UNSIGNED ARRAY)));
SELECT * FROM events WHERE 42 MEMBER OF (payload->'$.tags');

-- JSON_TABLE: expand JSON to relational rows
SELECT jt.* FROM events,
  JSON_TABLE(payload, '$.items[*]' COLUMNS (
    item_name VARCHAR(100) PATH '$.name',
    qty INT PATH '$.quantity'
  )) AS jt;
```

## Partitioning

```sql
-- Range partitioning (time-series data)
CREATE TABLE logs (
  id BIGINT AUTO_INCREMENT,
  created_at DATETIME NOT NULL,
  message TEXT,
  PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (YEAR(created_at)) (
  PARTITION p2023 VALUES LESS THAN (2024),
  PARTITION p2024 VALUES LESS THAN (2025),
  PARTITION pmax VALUES LESS THAN MAXVALUE
);

-- Prune old data instantly
ALTER TABLE logs DROP PARTITION p2023;

-- List partitioning
PARTITION BY LIST (region) (
  PARTITION p_us VALUES IN ('us-east','us-west'),
  PARTITION p_eu VALUES IN ('eu-west','eu-central')
);

-- Hash partitioning (even distribution)
PARTITION BY HASH (customer_id) PARTITIONS 8;
```

Limitations: foreign keys not supported with partitioning, all unique indexes must include the partition key.

## Replication

### Async Replication (default)

Source writes binlog → replica IO thread fetches → replica SQL thread applies. Replica can lag.

### Semi-Synchronous Replication

Source waits for at least one replica to ACK receiving the event before committing. Reduces data loss risk.

```sql
-- Source
INSTALL PLUGIN rpl_semi_sync_source SONAME 'semisync_source.so';
SET GLOBAL rpl_semi_sync_source_enabled = 1;
SET GLOBAL rpl_semi_sync_source_timeout = 1000; -- ms, falls back to async
```

### Group Replication

Multi-source replication with built-in conflict detection. Basis for MySQL InnoDB Cluster.

- **Single-primary mode**: one writer, rest are read-only (recommended)
- **Multi-primary mode**: all members accept writes, conflict detection via certification

### GTID (Global Transaction Identifiers)

```ini
gtid_mode = ON
enforce_gtid_consistency = ON
```

Each transaction gets a UUID:sequence_number. Simplifies failover — replicas know exactly which transactions they have.

## Performance Schema and Monitoring

```sql
-- Top queries by total time
SELECT DIGEST_TEXT, COUNT_STAR, AVG_TIMER_WAIT/1e12 AS avg_sec
FROM performance_schema.events_statements_summary_by_digest
ORDER BY SUM_TIMER_WAIT DESC LIMIT 10;

-- Current running queries
SELECT * FROM performance_schema.events_statements_current
WHERE END_EVENT_ID IS NULL;

-- Lock waits
SELECT * FROM performance_schema.data_lock_waits;

-- Memory usage by component
SELECT * FROM performance_schema.memory_summary_global_by_event_name
ORDER BY CURRENT_NUMBER_OF_BYTES_USED DESC LIMIT 10;
```

### Slow Query Log

```ini
slow_query_log = ON
slow_query_log_file = /var/log/mysql/slow.log
long_query_time = 0.5          # seconds
log_queries_not_using_indexes = ON
min_examined_row_limit = 100   # ignore trivial queries
```

Parse with `mysqldumpslow` or `pt-query-digest` (Percona Toolkit).

## Key Configuration Parameters

| Parameter                                | Default | Recommendation                   |
| ---------------------------------------- | ------- | -------------------------------- |
| `innodb_buffer_pool_size`                | 128M    | 70-80% of RAM                    |
| `innodb_log_file_size`                   | 48M     | 1-2G for write-heavy             |
| `innodb_flush_method`                    | `fsync` | `O_DIRECT` on Linux              |
| `innodb_io_capacity`                     | 200     | Match disk IOPS (SSD: 2000+)     |
| `innodb_flush_neighbors`                 | 1       | 0 for SSD                        |
| `max_connections`                        | 151     | Size for actual need + buffer    |
| `table_open_cache`                       | 4000    | Match `Opened_tables` status     |
| `join_buffer_size`                       | 256K    | 1-4M for complex joins           |
| `sort_buffer_size`                       | 256K    | 1-2M (per-connection allocation) |
| `tmp_table_size` / `max_heap_table_size` | 16M     | 64-256M (match both)             |
