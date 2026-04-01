# Query Optimization

## Execution Plans

Every query goes through a query planner/optimizer that decides how to retrieve data. Reading execution plans is the single most important skill for optimization.

### PostgreSQL EXPLAIN

```sql
-- Basic plan (estimated costs only)
EXPLAIN SELECT * FROM orders WHERE customer_id = 42;

-- Actual execution (runs the query)
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT * FROM orders WHERE customer_id = 42;

-- JSON format (more detail, good for tools)
EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)
SELECT o.*, c.name
FROM orders o JOIN customers c ON o.customer_id = c.id
WHERE o.created_at > '2025-01-01';
```

**Key metrics in EXPLAIN ANALYZE output**:

| Metric                           | Meaning                                              |
| -------------------------------- | ---------------------------------------------------- |
| `Seq Scan`                       | Full table scan — reading every row                  |
| `Index Scan`                     | Using an index to find rows, then fetching from heap |
| `Index Only Scan`                | Answered entirely from the index (covering index)    |
| `Bitmap Index Scan`              | Build a bitmap of matching rows, then fetch          |
| `cost=0.00..123.45`              | Estimated startup cost..total cost (arbitrary units) |
| `rows=1000`                      | Estimated number of rows                             |
| `actual time=0.1..5.2`           | Actual time in ms (startup..total)                   |
| `Buffers: shared hit=50 read=10` | Pages from cache vs disk                             |
| `loops=1`                        | How many times this node executed                    |

### MySQL EXPLAIN

```sql
EXPLAIN SELECT * FROM orders WHERE customer_id = 42;
EXPLAIN ANALYZE SELECT * FROM orders WHERE customer_id = 42;  -- MySQL 8.0.18+
EXPLAIN FORMAT=TREE SELECT ...;  -- tree format (8.0.16+)
```

| Column  | Key Values                                                                      |
| ------- | ------------------------------------------------------------------------------- |
| `type`  | `ALL` (full scan), `index` (full index scan), `range`, `ref`, `eq_ref`, `const` |
| `key`   | Which index is used (NULL = no index)                                           |
| `rows`  | Estimated rows examined                                                         |
| `Extra` | `Using index` (covering), `Using filesort`, `Using temporary`, `Using where`    |

`type` from worst to best: `ALL` → `index` → `range` → `ref` → `eq_ref` → `const` → `system`

## Index Strategies

### B-Tree Index (Default)

```sql
-- Single column
CREATE INDEX idx_orders_customer ON orders (customer_id);

-- Composite (multi-column) — column order matters!
CREATE INDEX idx_orders_customer_date ON orders (customer_id, created_at);

-- The leftmost prefix rule: composite index on (A, B, C) supports queries on:
-- (A), (A, B), (A, B, C)
-- Does NOT help with: (B), (C), (B, C)
```

### Covering Indexes

An index that contains all columns needed by a query — no heap/table lookup required.

```sql
-- Query: SELECT email FROM users WHERE username = 'alice';
CREATE INDEX idx_users_username_email ON users (username) INCLUDE (email);
-- PostgreSQL: INCLUDE clause adds non-key columns to leaf pages
-- MySQL: covering happens naturally if all selected columns are in the index

-- Verify with EXPLAIN: look for "Index Only Scan" (Postgres) or "Using index" (MySQL)
```

### Partial / Filtered Indexes

```sql
-- PostgreSQL: index only active users
CREATE INDEX idx_active_users ON users (email) WHERE active = true;
-- Smaller index, faster lookups when you always filter by active = true

-- Useful for: status columns with skewed distribution
-- e.g., 5% of orders are "pending" — index only those
CREATE INDEX idx_pending_orders ON orders (created_at) WHERE status = 'pending';
```

### Expression / Functional Indexes

```sql
-- Index on computed expression
CREATE INDEX idx_users_lower_email ON users (LOWER(email));
-- Query must use the same expression: WHERE LOWER(email) = 'alice@example.com'

-- MySQL: generated column + index (pre-8.0.13)
ALTER TABLE users ADD email_lower VARCHAR(255) GENERATED ALWAYS AS (LOWER(email)) STORED;
CREATE INDEX idx_email_lower ON users (email_lower);
```

### Index Types Beyond B-Tree

| Type   | Database                   | Use Case                                                    |
| ------ | -------------------------- | ----------------------------------------------------------- |
| Hash   | PostgreSQL, MySQL (Memory) | Equality lookups only. Not range queries.                   |
| GIN    | PostgreSQL                 | Full-text search, JSONB containment, arrays                 |
| GiST   | PostgreSQL                 | Geometric data, range types, full-text                      |
| BRIN   | PostgreSQL                 | Very large tables with naturally ordered data (time-series) |
| Bitmap | Oracle                     | Low-cardinality columns, warehouse queries                  |

```sql
-- GIN index for JSONB queries
CREATE INDEX idx_users_metadata ON users USING GIN (metadata jsonb_path_ops);
-- Supports: WHERE metadata @> '{"role": "admin"}'

-- BRIN index for time-series (extremely compact)
CREATE INDEX idx_events_time ON events USING BRIN (created_at);
-- Only useful when physical row order correlates with column values
```

### When NOT to Index

- Low-cardinality columns on small tables (< a few thousand rows)
- Columns that are rarely queried in WHERE/JOIN/ORDER BY
- Write-heavy tables where index maintenance overhead exceeds read benefit
- Already-indexed: adding a redundant index wastes space and slows writes

## Join Optimization

### Join Algorithms

| Algorithm   | How It Works                                         | Best For                                  |
| ----------- | ---------------------------------------------------- | ----------------------------------------- |
| Nested Loop | For each row in outer, scan inner                    | Small outer set, indexed inner            |
| Hash Join   | Build hash table from smaller set, probe with larger | Equi-joins, large tables, no index        |
| Merge Join  | Sort both sides, merge                               | Pre-sorted data (indexed), large datasets |

```sql
-- PostgreSQL: hint join order (rarely needed)
SET join_collapse_limit = 1;  -- use query order, don't reorder

-- Force specific join type (PostgreSQL)
SET enable_hashjoin = off;  -- for testing only, not production
SET enable_mergejoin = off;

-- MySQL: force index
SELECT * FROM orders FORCE INDEX (idx_customer_id)
JOIN customers ON orders.customer_id = customers.id;
```

### Join Order

The optimizer reorders joins to minimize intermediate result sizes. Usually correct, but:

```sql
-- If the optimizer gets it wrong, use CTEs to force order
-- (PostgreSQL < 12 materializes CTEs; 12+ may inline them)
WITH small_set AS MATERIALIZED (
    SELECT id FROM customers WHERE region = 'US'
)
SELECT o.* FROM orders o
JOIN small_set s ON o.customer_id = s.id;
```

## N+1 Queries

The most common ORM performance antipattern: loading a list, then issuing one query per item.

```python
# N+1 problem: 1 query for users + N queries for orders
users = User.query.all()                    # SELECT * FROM users
for user in users:
    print(user.orders)                       # SELECT * FROM orders WHERE user_id = ? (per user!)

# Fix 1: Eager loading (joinedload)
users = User.query.options(joinedload(User.orders)).all()
# Single query with JOIN

# Fix 2: Subquery loading
users = User.query.options(subqueryload(User.orders)).all()
# Two queries: all users, then all orders for those users

# Fix 3: selectinload (SQLAlchemy 1.4+)
users = User.query.options(selectinload(User.orders)).all()
# SELECT * FROM orders WHERE user_id IN (1, 2, 3, ...)
```

```javascript
// Prisma: include related data
const users = await prisma.user.findMany({
  include: { orders: true }, // eager load
});

// DataLoader pattern (GraphQL)
const orderLoader = new DataLoader(async (userIds) => {
  const orders = await db.query(
    "SELECT * FROM orders WHERE user_id = ANY($1)",
    [userIds],
  );
  return userIds.map((id) => orders.filter((o) => o.userId === id));
});
```

**Detection**: Enable query logging, look for patterns of repeated queries. Many ORMs have N+1 detection libraries (e.g., `bullet` for Rails, `nplusone` for Django).

## Pagination

### OFFSET-Based (Simple, Flawed)

```sql
SELECT * FROM orders ORDER BY created_at DESC LIMIT 20 OFFSET 1000;
-- Problem: database must scan and discard 1000 rows to get to page 51
-- Gets slower linearly as offset increases
```

### Keyset Pagination (Cursor-Based)

```sql
-- First page
SELECT * FROM orders ORDER BY created_at DESC, id DESC LIMIT 20;

-- Next page: use last row's values as cursor
SELECT * FROM orders
WHERE (created_at, id) < ('2025-03-15 10:00:00', 12345)
ORDER BY created_at DESC, id DESC
LIMIT 20;

-- With a single column (requires unique sort key)
SELECT * FROM orders
WHERE id < 12345
ORDER BY id DESC
LIMIT 20;
```

**Advantages**: Constant performance regardless of page depth. Uses indexes efficiently.
**Disadvantages**: Can't jump to arbitrary page. Requires stable sort order with a tiebreaker.

### Comparison

| Aspect                    | OFFSET                  | Keyset                 |
| ------------------------- | ----------------------- | ---------------------- |
| Jump to page N            | Yes                     | No                     |
| Deep page performance     | O(offset)               | O(1)                   |
| Handles inserts/deletes   | May skip/duplicate rows | Stable                 |
| Implementation complexity | Simple                  | Moderate               |
| API design                | `?page=5&per_page=20`   | `?cursor=abc&limit=20` |

## Batch Operations

### Batch Inserts

```sql
-- Bad: individual inserts
INSERT INTO logs (message) VALUES ('event 1');
INSERT INTO logs (message) VALUES ('event 2');
-- ... 10,000 times

-- Good: batch insert
INSERT INTO logs (message) VALUES
    ('event 1'), ('event 2'), ('event 3'), ...;
-- Group into batches of 1,000-5,000

-- PostgreSQL COPY (fastest bulk load)
COPY logs (message) FROM '/tmp/events.csv' WITH (FORMAT csv);

-- MySQL LOAD DATA
LOAD DATA INFILE '/tmp/events.csv' INTO TABLE logs;
```

### Batch Updates

```sql
-- Bad: update one at a time in application loop

-- Good: single UPDATE with CASE
UPDATE products SET price = CASE id
    WHEN 1 THEN 19.99
    WHEN 2 THEN 29.99
    WHEN 3 THEN 39.99
END WHERE id IN (1, 2, 3);

-- Good: UPDATE from VALUES (PostgreSQL)
UPDATE products AS p SET price = v.price
FROM (VALUES (1, 19.99), (2, 29.99), (3, 39.99)) AS v(id, price)
WHERE p.id = v.id;

-- Large batch update in chunks (avoid long locks)
DO $$
DECLARE batch_size INT := 5000;
BEGIN
    LOOP
        UPDATE orders SET status = 'archived'
        WHERE id IN (
            SELECT id FROM orders WHERE status = 'completed' AND created_at < '2024-01-01'
            LIMIT batch_size FOR UPDATE SKIP LOCKED
        );
        EXIT WHEN NOT FOUND;
        COMMIT;
    END LOOP;
END $$;
```

## Statistics

The query optimizer relies on table statistics to estimate row counts and choose plans.

```sql
-- PostgreSQL: update statistics
ANALYZE orders;                          -- specific table
ANALYZE;                                 -- all tables
SET default_statistics_target = 1000;    -- more detailed stats (default 100)
ALTER TABLE orders ALTER COLUMN status SET STATISTICS 1000;  -- per-column

-- View statistics
SELECT * FROM pg_stats WHERE tablename = 'orders' AND attname = 'status';
-- Shows: null_frac, n_distinct, most_common_vals, most_common_freqs, correlation

-- MySQL: update statistics
ANALYZE TABLE orders;
-- View:
SELECT * FROM information_schema.statistics WHERE table_name = 'orders';
SHOW INDEX FROM orders;

-- PostgreSQL autovacuum handles ANALYZE automatically, but after large data changes:
-- Run ANALYZE manually to avoid stale plans
```

**Stale statistics** are a common cause of bad query plans. After bulk loads, large deletes, or schema changes — always run ANALYZE.

## Materialized Views

Pre-computed query results stored as a table. Trade storage and freshness for read speed.

```sql
-- PostgreSQL
CREATE MATERIALIZED VIEW monthly_sales AS
SELECT
    date_trunc('month', created_at) AS month,
    product_id,
    SUM(quantity) AS total_qty,
    SUM(total) AS total_revenue
FROM orders
GROUP BY 1, 2;

-- Create index on materialized view
CREATE INDEX idx_monthly_sales_product ON monthly_sales (product_id);

-- Refresh (full rebuild)
REFRESH MATERIALIZED VIEW monthly_sales;

-- Concurrent refresh (allows reads during refresh, requires unique index)
REFRESH MATERIALIZED VIEW CONCURRENTLY monthly_sales;

-- MySQL: use actual tables + scheduled refresh
CREATE TABLE monthly_sales AS SELECT ...;
-- Refresh via cron or event scheduler:
CREATE EVENT refresh_monthly_sales
ON SCHEDULE EVERY 1 HOUR
DO REPLACE INTO monthly_sales SELECT ...;
```

**When to use**: Expensive aggregations, complex joins, dashboard queries, reporting. When data freshness of minutes/hours is acceptable.

## Common Anti-Patterns

### Functions on Indexed Columns

```sql
-- Bad: function prevents index use
SELECT * FROM users WHERE YEAR(created_at) = 2025;
SELECT * FROM orders WHERE UPPER(status) = 'PENDING';

-- Good: rewrite to use the indexed column directly
SELECT * FROM users WHERE created_at >= '2025-01-01' AND created_at < '2026-01-01';
SELECT * FROM orders WHERE status = 'pending';  -- store normalized
```

### Implicit Type Conversion

```sql
-- Bad: phone is VARCHAR, comparing to integer
SELECT * FROM users WHERE phone = 5551234;
-- Database converts every row's phone to integer — full scan

-- Good: use matching types
SELECT * FROM users WHERE phone = '5551234';
```

### SELECT \*

```sql
-- Bad: fetches all columns, prevents covering index
SELECT * FROM orders WHERE customer_id = 42;

-- Good: only fetch what you need
SELECT id, total, status FROM orders WHERE customer_id = 42;
```

### OR Conditions on Different Columns

```sql
-- Bad: can't use a single index efficiently
SELECT * FROM orders WHERE customer_id = 42 OR product_id = 99;

-- Better: UNION (each branch uses its own index)
SELECT * FROM orders WHERE customer_id = 42
UNION ALL
SELECT * FROM orders WHERE product_id = 99 AND customer_id != 42;
```

### Correlated Subqueries

```sql
-- Bad: executes subquery once per outer row
SELECT o.*, (SELECT COUNT(*) FROM items i WHERE i.order_id = o.id) AS item_count
FROM orders o;

-- Better: JOIN with aggregation
SELECT o.*, COALESCE(ic.cnt, 0) AS item_count
FROM orders o
LEFT JOIN (SELECT order_id, COUNT(*) AS cnt FROM items GROUP BY order_id) ic
    ON o.id = ic.order_id;

-- Or lateral join (PostgreSQL)
SELECT o.*, ic.cnt
FROM orders o
LEFT JOIN LATERAL (
    SELECT COUNT(*) AS cnt FROM items WHERE order_id = o.id
) ic ON true;
```

### NOT IN with NULLs

```sql
-- Dangerous: if subquery returns any NULL, NOT IN returns empty set
SELECT * FROM users WHERE id NOT IN (SELECT user_id FROM banned_users);
-- If any banned_users.user_id is NULL, this returns 0 rows!

-- Safe: use NOT EXISTS
SELECT * FROM users u
WHERE NOT EXISTS (SELECT 1 FROM banned_users b WHERE b.user_id = u.id);
```

### Missing LIMIT on Existence Checks

```sql
-- Bad: scans potentially millions of rows
SELECT COUNT(*) FROM orders WHERE status = 'pending';
-- Then: if count > 0 ...

-- Good: stop after finding one
SELECT EXISTS (SELECT 1 FROM orders WHERE status = 'pending');
-- Or: SELECT 1 FROM orders WHERE status = 'pending' LIMIT 1;
```

## Query Performance Checklist

1. **Run EXPLAIN ANALYZE** — measure rather than guess
2. **Check for sequential scans** on large tables — add missing indexes
3. **Verify index usage** — functions, type mismatches, and OR can prevent index use
4. **Check row estimates vs actuals** — large discrepancy = stale statistics
5. **Look for sort/temp files** — add indexes that match ORDER BY, or increase work_mem
6. **Check for nested loops on large sets** — might need hash/merge join
7. **Verify pagination approach** — OFFSET on deep pages is a red flag
8. **Count round trips** — N+1 queries, chatty application code
9. **Review data types** — oversized columns waste buffer cache
10. **Monitor slow query log** — track regressions over time
