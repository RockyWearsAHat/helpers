# SQL Best Practices

## SQL Philosophy

SQL is the universal language for relational data. Modern SQL (SQL:2016+) includes window functions, CTEs, JSON support, and lateral joins. The key to performant SQL is understanding how the query planner works and writing queries that help it.

- **Declarative**: Say *what* you want, not *how* to get it. The optimizer picks the execution plan.
- **Set-based thinking**: Operate on sets of rows, not individual rows. Avoid row-by-row processing.
- **CTEs for readability**: Break complex queries into named, composable steps.

## CTEs (Common Table Expressions)

```sql
-- Named subqueries for readability
WITH active_users AS (
    SELECT id, name, email, created_at
    FROM users
    WHERE status = 'active'
      AND last_login > CURRENT_DATE - INTERVAL '90 days'
),
user_orders AS (
    SELECT u.id AS user_id,
           COUNT(o.id) AS order_count,
           SUM(o.total) AS total_spent
    FROM active_users u
    JOIN orders o ON o.user_id = u.id
    WHERE o.created_at > CURRENT_DATE - INTERVAL '1 year'
    GROUP BY u.id
)
SELECT u.name,
       u.email,
       uo.order_count,
       uo.total_spent,
       uo.total_spent / NULLIF(uo.order_count, 0) AS avg_order_value
FROM active_users u
JOIN user_orders uo ON uo.user_id = u.id
WHERE uo.total_spent > 1000
ORDER BY uo.total_spent DESC;

-- Recursive CTE (hierarchies, graphs)
WITH RECURSIVE org_tree AS (
    SELECT id, name, manager_id, 0 AS depth
    FROM employees
    WHERE manager_id IS NULL  -- root

    UNION ALL

    SELECT e.id, e.name, e.manager_id, t.depth + 1
    FROM employees e
    JOIN org_tree t ON t.id = e.manager_id
)
SELECT * FROM org_tree ORDER BY depth, name;
```

## Window Functions

```sql
-- Row numbering and ranking
SELECT name, department, salary,
       ROW_NUMBER() OVER (PARTITION BY department ORDER BY salary DESC) AS rank,
       DENSE_RANK() OVER (ORDER BY salary DESC) AS overall_rank
FROM employees;

-- Running totals and moving averages
SELECT date, revenue,
       SUM(revenue) OVER (ORDER BY date ROWS UNBOUNDED PRECEDING) AS running_total,
       AVG(revenue) OVER (ORDER BY date ROWS BETWEEN 6 PRECEDING AND CURRENT ROW) AS moving_avg_7d
FROM daily_revenue;

-- Lead/Lag (previous/next row access)
SELECT date, revenue,
       LAG(revenue, 1) OVER (ORDER BY date) AS prev_day,
       revenue - LAG(revenue, 1) OVER (ORDER BY date) AS daily_change,
       LEAD(revenue, 1) OVER (ORDER BY date) AS next_day
FROM daily_revenue;

-- Percentiles
SELECT name, salary,
       PERCENT_RANK() OVER (ORDER BY salary) AS percentile,
       NTILE(4) OVER (ORDER BY salary) AS quartile
FROM employees;
```

## JOIN Patterns

```sql
-- INNER JOIN: only matching rows
SELECT u.name, o.total
FROM users u
JOIN orders o ON o.user_id = u.id;

-- LEFT JOIN: all left rows + matching right (NULL if no match)
SELECT u.name, COALESCE(COUNT(o.id), 0) AS order_count
FROM users u
LEFT JOIN orders o ON o.user_id = u.id
GROUP BY u.id, u.name;

-- LATERAL JOIN (correlated subquery as a join — PostgreSQL)
SELECT u.name, recent.*
FROM users u
CROSS JOIN LATERAL (
    SELECT o.id, o.total, o.created_at
    FROM orders o
    WHERE o.user_id = u.id
    ORDER BY o.created_at DESC
    LIMIT 3
) recent;

-- Anti-join (find rows with NO match)
SELECT u.*
FROM users u
LEFT JOIN orders o ON o.user_id = u.id
WHERE o.id IS NULL;  -- users with no orders
```

## Indexing Strategy

```sql
-- B-tree indexes (default, good for equality and range)
CREATE INDEX idx_users_email ON users (email);
CREATE INDEX idx_orders_user_date ON orders (user_id, created_at DESC);

-- Partial indexes (index a subset of rows)
CREATE INDEX idx_active_users ON users (email) WHERE status = 'active';

-- Covering indexes (include columns to avoid table lookup)
CREATE INDEX idx_orders_covering ON orders (user_id)
    INCLUDE (total, status);

-- Expression indexes
CREATE INDEX idx_users_lower_email ON users (LOWER(email));

-- When to index:
-- DO: foreign keys, WHERE clause columns, JOIN columns, ORDER BY columns
-- DON'T: small tables, rarely queried columns, high-write/low-read tables
```

## Query Optimization

```sql
-- Use EXPLAIN ANALYZE to see actual execution
EXPLAIN ANALYZE
SELECT * FROM orders WHERE user_id = 42 AND status = 'shipped';

-- Avoid SELECT * in production
SELECT id, name, email FROM users;  -- only what you need

-- Use EXISTS instead of IN for correlated checks
-- GOOD
SELECT u.* FROM users u
WHERE EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id);

-- Use UNION ALL instead of UNION when duplicates are impossible
SELECT id FROM active_users
UNION ALL
SELECT id FROM pending_users;  -- no dedup overhead

-- Batch operations
INSERT INTO audit_log (user_id, action, created_at)
SELECT id, 'migration', NOW()
FROM users
WHERE created_at < '2020-01-01';  -- bulk insert from select
```

## Data Integrity

```sql
-- Constraints
CREATE TABLE orders (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id),
    total       NUMERIC(10, 2) NOT NULL CHECK (total >= 0),
    status      TEXT NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending', 'confirmed', 'shipped', 'delivered')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, created_at)  -- no duplicate timestamps per user
);

-- Transactions
BEGIN;
    UPDATE accounts SET balance = balance - 100 WHERE id = 1;
    UPDATE accounts SET balance = balance + 100 WHERE id = 2;
    -- Both succeed or both fail
COMMIT;
```

## Key Rules

1. **Use CTEs for readability.** Break complex queries into named steps.
2. **Always use parameterized queries** in application code. Never concatenate user input into SQL (prevents SQL injection).
3. **Index foreign keys.** Unindexed FKs cause full table scans on JOIN and DELETE.
4. **Use `EXPLAIN ANALYZE`** to understand query plans. Don't guess about performance.
5. **Prefer `NOT EXISTS` over `NOT IN`** — `NOT IN` has surprising behavior with NULLs.
6. **Use transactions** for multi-statement operations that must be atomic.

---

*Sources: PostgreSQL Documentation, SQL Performance Explained (Markus Winand), use-the-index-luke.com, High Performance MySQL (Schwartz)*
