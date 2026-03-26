# SQL Window Functions — Analytical Queries Without Aggregation

## Overview

Window functions compute values across a set of rows related to the current row, without collapsing the result set like `GROUP BY` does. Every row retains its identity while gaining context from its "window" — a conceptual frame of neighboring rows.

The `OVER` clause defines the window. Without it, aggregate functions return a single row per group.

```sql
-- Without window function (GROUP BY collapses rows)
SELECT dept, COUNT(*) FROM employees GROUP BY dept;  -- one row per dept

-- Window function (retains all rows)
SELECT dept, salary, COUNT(*) OVER (PARTITION BY dept) FROM employees;
```

## Frame Specification

### PARTITION BY and ORDER BY

**`PARTITION BY`** divides rows into logical groups (like `GROUP BY`). Omitting it treats the entire result set as one partition.

**`ORDER BY`** within the window defines row order and implicitly defines the **frame** (the set of rows included in the computation).

```sql
-- Salary rank within each department
SELECT name, dept, salary,
  RANK() OVER (PARTITION BY dept ORDER BY salary DESC)
FROM employees;

-- Running total (frame: all rows up to current, sorted by hire_date)
SELECT name, hire_date, salary,
  SUM(salary) OVER (ORDER BY hire_date)
FROM employees;
```

### Frame Modes: ROWS, RANGE, GROUPS

When `ORDER BY` is present, the default frame is `RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW`. Explicit frames refine this:

- **`ROWS`** — Physical row offsets. `ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING` includes the previous row, current row, and next row.
- **`RANGE`** — Logical value ranges. `RANGE BETWEEN 100 PRECEDING AND 100 FOLLOWING` includes all rows with values within 100 of the current row. Ties are included together.
- **`GROUPS`** — Peer groups. `GROUPS BETWEEN 1 PRECEDING AND 1 FOLLOWING` includes the previous peer group, current peer group, and next peer group (peers have equal `ORDER BY` values).

```sql
-- Moving average (3-row window: current ± 1)
SELECT id, value, AVG(value) OVER (
  ORDER BY id
  ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING
) FROM measurements;

-- Range frame with date offsets
SELECT order_date, amount, SUM(amount) OVER (
  ORDER BY order_date
  RANGE BETWEEN INTERVAL '7 days' PRECEDING AND CURRENT ROW
) FROM orders;  -- PostgreSQL extension; standard SQL uses ROWS
```

## Ranking and Numbering Functions

| Function       | Behavior                                           | Example Output       |
| -------------- | -------------------------------------------------- | -------------------- |
| `ROW_NUMBER()` | Sequential row number within partition             | 1, 2, 3, 4, 5        |
| `RANK()`       | Rank with gaps on ties                             | 1, 2, 2, 4, 5        |
| `DENSE_RANK()` | Rank without gaps on ties                          | 1, 2, 2, 3, 4        |
| `NTILE(n)`     | Divide partition into `n` buckets (nth percentile) | 1, 1, 1, 2, 2, 3, 3  |

```sql
SELECT name, salary,
  ROW_NUMBER() OVER (ORDER BY salary DESC),
  RANK() OVER (ORDER BY salary DESC),
  DENSE_RANK() OVER (ORDER BY salary DESC),
  NTILE(4) OVER (ORDER BY salary DESC)  -- quartile
FROM employees;
```

Usage: **`ROW_NUMBER`** for pagination; **`RANK`/`DENSE_RANK`** for competitions (ties break equally); **`NTILE`** for bucketing percentiles.

## Offset Functions: LAG and LEAD

Access prior or subsequent rows without a self-join.

```sql
SELECT date, value,
  LAG(value) OVER (ORDER BY date) AS prev_value,
  LAG(value, 2) OVER (ORDER BY date) AS prev_prev_value,
  LEAD(value) OVER (ORDER BY date) AS next_value,
  value - LAG(value) OVER (ORDER BY date) AS day_over_day_change
FROM daily_metrics
ORDER BY date;
```

**Parameters**: `LAG(expr [, offset [, default]])`. Offset defaults to 1; default value (when out of range) defaults to NULL.

## First/Last Value and Aggregates

```sql
SELECT name, salary, hire_date,
  FIRST_VALUE(salary) OVER (ORDER BY hire_date) AS first_hire_salary,
  LAST_VALUE(salary) OVER (
    ORDER BY hire_date
    ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
  ) AS last_hire_salary,
  MAX(salary) OVER (PARTITION BY dept ORDER BY hire_date) AS max_dept_salary,
  AVG(salary) OVER (PARTITION BY dept) AS avg_dept_salary
FROM employees;
```

**Critical**: `LAST_VALUE` and `FIRST_VALUE` require explicit frame bounds. The default frame (rows up to current) will return the current value for `LAST_VALUE`.

## Running Totals and Moving Averages

Common use case: time-series aggregations.

```sql
-- Cumulative sales
SELECT date, daily_sales,
  SUM(daily_sales) OVER (ORDER BY date) AS cumulative_sales
FROM sales;

-- 30-day moving average
SELECT date, price,
  AVG(price) OVER (
    ORDER BY date
    ROWS BETWEEN 29 PRECEDING AND CURRENT ROW
  ) AS ma_30
FROM stock_prices;

-- Running average (unbounded)
SELECT date, value,
  AVG(value) OVER (ORDER BY date) AS running_avg
FROM metrics;
```

## Gaps and Islands Detection

Identify contiguous ranges (e.g., consecutive days without activity or outliers).

```sql
-- Assign group numbers to consecutive working days
SELECT date, status,
  SUM(CASE WHEN prev_date IS NULL OR date - prev_date > 1 THEN 1 ELSE 0 END)
    OVER (ORDER BY date) AS island_group
FROM (
  SELECT date, status,
    LAG(date) OVER (ORDER BY date) AS prev_date
  FROM logs
  WHERE status = 'active'
) sub;
```

Use case: segment time series into continuous blocks for analysis.

## Performance Considerations

- Frames restrict computation scope. **`ROWS BETWEEN ... AND ...`** is fastest; **`RANGE`** requires additional sorting logic.
- Multiple window functions with the same partition/order are combined into one computation pass.
- Window functions are computed after `WHERE`, `GROUP BY`, and before `ORDER BY` (final sort). Use subqueries or CTEs to filter or exclude window results.
- Indexing on partition columns and `ORDER BY` columns helps, but not as dramatically as it does for GROUP BY aggregations.

```sql
-- Anti-pattern: filter window results in WHERE clause
SELECT * FROM (
  SELECT name, rank() OVER (...) AS rnk FROM employees
) WHERE rnk <= 10;  -- CTE/subquery required; WHERE doesn't see window results

-- Avoid: computing the same window multiple times
SELECT
  SUM(...) OVER w,
  AVG(...) OVER w,
  COUNT(...) OVER w
FROM table
WINDOW w AS (PARTITION BY dept ORDER BY date);  -- reuse with WINDOW clause
```

## See Also

- [SQL Conventions and Idioms](language-sql.md)
- [Database Query Optimization](database-query-optimization.md)
- [Database Query Planning](database-query-planning.md)