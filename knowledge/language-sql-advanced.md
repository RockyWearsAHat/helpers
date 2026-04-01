# SQL Advanced: Window Functions, CTEs, and Analytical Queries

## Overview

Advanced SQL unlocks analytical capabilities beyond basic aggregation. **Window functions** compute across row sets without collapsing results. **Recursive CTEs** traverse hierarchies. **GROUPING SETS**, **CUBE**, and **ROLLUP** generate multi-level summaries. **MERGE** and **UPSERT** handle complex inserts. Generated columns and table partitioning optimize schema design. Mastery here separates competent SQL from expert data engineering.

## Window Functions: The Deep Dive

### Architecture

A window function applies an aggregate-like operation (SUM, AVG, ROW_NUMBER, etc.) over a **frame** of rows, returning a value for each input row rather than collapsing.

```sql
SELECT 
  employee,
  salary,
  AVG(salary) OVER (PARTITION BY department) AS dept_avg,
  RANK() OVER (ORDER BY salary DESC) AS salary_rank
FROM employees;
```

**PARTITION BY** defines groups; **ORDER BY** defines sort order within the frame.

### Frame Specification

By default, the frame extends from the first row to the current row (for ORDER BY without explicit frame).

```sql
SUM(revenue) OVER (
  PARTITION BY region
  ORDER BY date
  ROWS BETWEEN 2 PRECEDING AND CURRENT ROW
)
```

**Frame modes**:

- **ROWS**: Fixed number of rows (always N literal rows)
- **RANGE**: Logical range based on values (e.g., all rows within 100 of current)
- **GROUPS**: Groups of rows with same ORDER BY value

Common frames:

- `UNBOUNDED PRECEDING TO UNBOUNDED FOLLOWING`: entire partition
- `CURRENT ROW`: just this row
- `N PRECEDING TO N FOLLOWING`: sliding window

### Categories

**Aggregate window functions** (work like GROUP BY, but non-aggregating):

```sql
AVG(x) OVER (...), SUM(x) OVER (...), COUNT(*) OVER (...)
```

**Ranking functions**:

```sql
ROW_NUMBER()       -- 1, 2, 3, ... (ties get different numbers)
RANK()             -- 1, 2, 2, 4 (ties get same rank, gaps follow)
DENSE_RANK()       -- 1, 2, 2, 3 (ties same rank, no gaps)
PERCENT_RANK()     -- [0, 1] normalized rank
CUME_DIST()        -- cumulative distribution: count <= current / total
```

**Offset functions** (access previous/next rows):

```sql
LAG(column, offset, default) OVER (ORDER BY ...)    -- previous row's value
LEAD(column, offset, default) OVER (ORDER BY ...)   -- next row's value
FIRST_VALUE(column) OVER (...)                      -- first in frame
LAST_VALUE(column) OVER (...)                       -- last in frame
NTH_VALUE(column, n) OVER (...)                     -- nth in frame
```

### Performance Considerations

- Window functions compute after WHERE but before ORDER BY final results (execution order matters)
- Multiple windows with same PARTITION BY/ORDER BY can share a single pass; reorder clauses for efficiency
- RANGE BETWEEN with value offsets can force full table scans; use ROWS for large data
- Modern databases (PostgreSQL 13+, MySQL 8.0+, SQL Server) incrementally compute windows; earlier versions materialize

## Recursive CTEs: Hierarchies and Traversal

### Anatomy

Recursive CTEs have two parts: **anchor** (base case) and **recursive member** (inductive step).

```sql
WITH RECURSIVE tree AS (
  -- Anchor: starting rows
  SELECT id, parent_id, name, 1 AS level
  FROM nodes
  WHERE parent_id IS NULL  -- roots
  
  UNION ALL
  
  -- Recursive: iterative expansion
  SELECT n.id, n.parent_id, n.name, tree.level + 1
  FROM nodes n
  INNER JOIN tree ON n.parent_id = tree.id
)
SELECT * FROM tree;
```

Loop detection: most databases limit recursion depth (default: 100) to prevent infinite loops.

### Use Cases

**Tree/hierarchical queries** (org charts, categories):

```sql
WITH RECURSIVE hierarchy AS (
  SELECT emp_id, manager_id, name, 0 AS depth FROM employees WHERE manager_id IS NULL
  UNION ALL
  SELECT e.emp_id, e.manager_id, e.name, h.depth + 1
  FROM employees e
  INNER JOIN hierarchy h ON e.manager_id = h.emp_id
  WHERE h.depth < 10  -- limit depth
)
SELECT REPLICATE('  ', depth) || name AS tree FROM hierarchy ORDER BY emp_id;
```

**Graph traversal** (paths, shortest paths):

```sql
WITH RECURSIVE paths AS (
  SELECT from_id, to_id, 1 AS hops FROM edges WHERE from_id = 1
  UNION ALL
  SELECT p.from_id, e.to_id, p.hops + 1
  FROM paths p
  INNER JOIN edges e ON p.to_id = e.from_id
  WHERE p.hops < 5  -- limit path length
)
SELECT DISTINCT to_id FROM paths;
```

### Performance Warnings

- Recursive term multiplies result size every iteration; exponential for dense graphs
- Join conditions crucial; inefficient joins cause explosion
- Set `MAXRECURSION` explicitly to prevent runaway queries

## GROUPING SETS, CUBE, and ROLLUP

### Standard GROUP BY Limitations

GROUP BY aggregates to one level of granularity. Reporting requires multiple queries joined manually. **GROUPING SETS** unifies this.

```sql
-- Without GROUPING SETS: three queries
SELECT region, NULL AS product, SUM(sales) FROM sales GROUP BY region
UNION ALL
SELECT NULL, product, SUM(sales) FROM sales GROUP BY product
UNION ALL
SELECT NULL, NULL, SUM(sales) FROM sales;

-- With GROUPING SETS: one query
SELECT region, product, SUM(sales)
FROM sales
GROUP BY GROUPING SETS ((region), (product), ())
ORDER BY region, product;
```

### ROLLUP vs CUBE

**ROLLUP** (hierarchical superset):

```sql
GROUP BY ROLLUP (year, month, day)
-- Generates: (year, month, day), (year, month), (year), ()
```

Useful for time-series drill-down.

**CUBE** (all combinations):

```sql
GROUP BY CUBE (region, product, channel)
-- Generates: all 8 combinations of (region), (product), (channel)
```

More expensive; use ROLLUP when hierarchy is natural.

### Identifying Aggregate Levels

**GROUPING()** function returns 1 if column is part of GROUP BY, 0 if aggregated (subtotal row):

```sql
SELECT 
  CASE WHEN GROUPING(region) = 1 THEN 'All Regions' ELSE region END AS region,
  CASE WHEN GROUPING(product) = 1 THEN 'All Products' ELSE product END AS product,
  SUM(sales),
  GROUPING(region) * 2 + GROUPING(product) AS grouping_id
FROM sales
GROUP BY ROLLUP (region, product);
```

## MERGE: Conditional Insert/Update/Delete

**MERGE** atomically handles complex upsert logic: insert new, update existing, delete obsolete.

```sql
MERGE INTO target t
USING source s
ON t.id = s.id
WHEN MATCHED AND s.status = 'ACTIVE' THEN
  UPDATE SET t.value = s.value, t.updated = NOW()
WHEN MATCHED AND s.status = 'DELETED' THEN
  DELETE
WHEN NOT MATCHED THEN
  INSERT (id, value, created) VALUES (s.id, s.value, NOW());
```

Syntax varies by database (SQL Server, Oracle, PostgreSQL). PostgreSQL uses **ON CONFLICT** instead (more flexible):

```sql
INSERT INTO target (id, value) VALUES (1, 100)
ON CONFLICT (id) DO UPDATE SET value = EXCLUDED.value
WHERE target.updated < NOW() - '1 day'::interval;
```

## Generated Columns and Virtual Expressions

**Generated columns** compute from other columns, reducing redundancy and bugs.

```sql
CREATE TABLE products (
  id INT PRIMARY KEY,
  price DECIMAL(10, 2),
  tax_rate DECIMAL(5, 4),
  total_price DECIMAL(10, 2) GENERATED ALWAYS AS (price * (1 + tax_rate)) STORED
);
```

- **STORED**: computed and physically stored (disk cost, no re-computation)
- **VIRTUAL**: computed on read (storage savings, slight query cost)

Use cases: derived metrics, denormalization, simplifying queries. Trade off consistency (stored: always current, always rows synced) vs. storage.

## Table Partitioning

Partition large tables by range, list, or hash for performance and manageability.

```sql
CREATE TABLE sales (
  id INT,
  sale_date DATE,
  amount DECIMAL(10, 2)
)
PARTITION BY RANGE (YEAR(sale_date)) (
  PARTITION p2023 VALUES LESS THAN (2024),
  PARTITION p2024 VALUES LESS THAN (2025),
  PARTITION pmax VALUES LESS THAN MAXVALUE
);
```

Benefits:

- **Pruning**: WHERE on partition key skips partitions (query optimizer, index range scans)
- **Parallel queries**: different partitions processed in parallel
- **Maintenance**: vacuum, analyze, reindex per partition
- **Archival**: drop old partitions instead of DELETE

Trade-offs: planner overhead (ineffective on non-partition keys), complexity (manual maintenance in some databases).

## CTE Optimization

### Materialization

Some databases materialize (scan-once, cache, join from cache) CTEs; others inline (repeated scan). Write CTEs assuming inlining—avoid high-cost scans.

```sql
-- Good: CTE filters aggressively
WITH filtered AS (
  SELECT * FROM big_table WHERE date > '2024-01-01'
)
SELECT * FROM filtered WHERE id IN (...)
```

### Chaining CTEs

Multiple CTEs chain sequentially; only reference earlier CTEs:

```sql
WITH cte1 AS (...),
     cte2 AS (SELECT * FROM cte1 ...),
     cte3 AS (SELECT * FROM cte2 ...)
SELECT * FROM cte3;
```

Each step filters through the previous; stack filters for efficiency.

## Analytical Functions: Advanced Patterns

### Year-over-Year (YoY) and Month-over-Month (MoM)

```sql
SELECT 
  DATE_TRUNC('month', date) AS month,
  SUM(revenue) AS revenue,
  LAG(SUM(revenue)) OVER (
    PARTITION BY EXTRACT(MONTH FROM date)
    ORDER BY DATE_TRUNC('month', date)
  ) AS prev_year_same_month
FROM sales
GROUP BY DATE_TRUNC('month', date);
```

### Running Totals and Cumulative Distribution

```sql
SELECT 
  id,
  value,
  SUM(value) OVER (ORDER BY id ROWS UNBOUNDED PRECEDING) AS cumulative,
  PERCENT_RANK() OVER (ORDER BY value) AS percentile
FROM transactions;
```

### Gaps and Islands

Identify contiguous sequences (islands) and gaps:

```sql
WITH numbered AS (
  SELECT 
    id, 
    date,
    ROW_NUMBER() OVER (ORDER BY date) - ROW_NUMBER() OVER (PARTITION BY category ORDER BY date) AS island
  FROM events
)
SELECT 
  island,
  MIN(date) AS start,
  MAX(date) AS end,
  COUNT(*) AS event_count
FROM numbered
GROUP BY island;
```

Advanced SQL shifts focus from **retrieval** (SELECT WHERE) to **analysis** (window functions, hierarchies, complex aggregations). Mastery enables elegant, performant analytics where naive approaches require multiple round-trips or procedural code.