# SQL CTEs and Recursive Queries — Named Queries, Hierarchies, and Traversal

## Overview

Common Table Expressions (CTEs) are temporary named result sets defined within a query using the `WITH` clause. They improve readability and enable recursion.

```sql
WITH employee_list AS (
  SELECT id, name, manager_id FROM employees
)
SELECT * FROM employee_list WHERE manager_id IS NULL;
```

**Non-recursive CTEs** are functionally equivalent to subqueries but more readable and reusable within the same query. **Recursive CTEs** enable graph traversal and hierarchical data processing.

## Non-Recursive CTEs (WITH Clause)

Multiple CTEs can reference each other (forward references are not allowed);later CTEs see earlier ones.

```sql
WITH dept_summary AS (
  SELECT dept, COUNT(*) AS emp_count, AVG(salary) AS avg_sal
  FROM employees
  GROUP BY dept
),
high_salary_employees AS (
  SELECT e.name, e.salary, d.avg_sal
  FROM employees e
  JOIN dept_summary d ON e.dept = d.dept
  WHERE e.salary > d.avg_sal * 1.5
)
SELECT * FROM high_salary_employees;
```

**Materialization**: Most databases materialize (compute once, store temporarily) non-recursive CTEs, making them efficient for multiple references in the main query. Some databases (e.g., PostgreSQL with `WITH ... AS MATERIALIZED`) allow explicit control.

## Recursive CTE Structure

A recursive CTE has two parts, joined by `UNION ALL`:

1. **Anchor member** (base case): Non-recursive SELECT defining the starting rows.
2. **Recursive member** (inductive step): SELECT that references the CTE, producing new rows.

Recursion terminates when the recursive member produces no new rows.

```sql
WITH RECURSIVE hierarchy AS (
  -- Anchor: all top-level nodes (no parent)
  SELECT id, name, parent_id, 0 AS depth
  FROM categories
  WHERE parent_id IS NULL

  UNION ALL

  -- Recursive: join hierarchy to categories to get children
  SELECT c.id, c.name, c.parent_id, h.depth + 1
  FROM categories c
  INNER JOIN hierarchy h ON c.parent_id = h.id
)
SELECT * FROM hierarchy;
```

**Critical**: The recursive member must join the CTE (not a subquery or different table), or recursion doesn't work.

## Hierarchical Data: Trees and Org Charts

Organizations, file systems, and taxonomies are naturally hierarchical.

```sql
-- Organization hierarchy with path strings
WITH RECURSIVE org_tree AS (
  SELECT id, name, manager_id, 1 AS level, CAST(name AS TEXT) AS path
  FROM employees
  WHERE manager_id IS NULL

  UNION ALL

  SELECT e.id, e.name, e.manager_id, ot.level + 1,
    ot.path || ' > ' || e.name
  FROM employees e
  INNER JOIN org_tree ot ON e.manager_id = ot.id
)
SELECT * FROM org_tree ORDER BY path;
```

**SQL Standard** (PostgreSQL):
```sql
WITH RECURSIVE org_tree AS (
  SELECT id, name, manager_id, 1 AS level, ARRAY[id] AS path
  FROM employees
  WHERE manager_id IS NULL

  UNION ALL

  SELECT e.id, e.name, e.manager_id, ot.level + 1, ot.path || e.id
  FROM employees e
  INNER JOIN org_tree ot ON e.manager_id = ot.id
)
SELECT * FROM org_tree;
```

## Bill of Materials (BOM)

Hierarchical product breakdowns (assembly ← component ← subcomponent).

```sql
WITH RECURSIVE bom AS (
  -- Top-level product
  SELECT id, part_id, name, quantity, 0 AS level, CAST(part_id AS TEXT) AS path
  FROM products
  WHERE parent_id IS NULL

  UNION ALL

  -- Recursive: fetch subcomponents
  SELECT p.id, p.part_id, p.name, bom.quantity * p.quantity,
    bom.level + 1, bom.path || '/' || p.part_id
  FROM products p
  INNER JOIN bom ON p.parent_id = bom.id
)
SELECT * FROM bom;
```

## Graph Traversal

Shortest paths, reachability, and social networks.

```sql
-- Breadth-first search (BFS): shortest path from source
WITH RECURSIVE path_search AS (
  SELECT id, source_id, target_id, 1 AS hops, ARRAY[source_id, target_id] AS path
  FROM edges
  WHERE source_id = 'start_node'

  UNION ALL

  SELECT e.id, ps.source_id, e.target_id, ps.hops + 1,
    ps.path || e.target_id
  FROM edges e
  INNER JOIN path_search ps ON e.source_id = ps.target_id
  WHERE ps.hops < 10 AND NOT e.target_id = ANY(ps.path)  -- stop on cycle or depth limit
)
SELECT * FROM path_search WHERE target_id = 'end_node'
ORDER BY hops LIMIT 1;
```

Use **arrays** or **strings** to track visited nodes and prevent infinite loops.

## Pagination with ROW_NUMBER and Recursion

Fetch result pages without computing entire result set (though this is rare vs. `LIMIT/OFFSET`).

```sql
WITH RECURSIVE paginated AS (
  SELECT *, ROW_NUMBER() OVER (ORDER BY id) AS rn FROM items
  WHERE rn BETWEEN (? - 1) * 10 + 1 AND ? * 10
)
SELECT * FROM paginated;
```

More common: Use window functions directly instead of recursion.

## Cycle Detection using SET

Prevent infinite loops by tracking visited nodes.

```sql
-- PostgreSQL-specific: CYCLE clause (SQL:2023 standard)
WITH RECURSIVE traversal AS (
  SELECT id, parent_id, 1 AS depth
  FROM categories
  WHERE parent_id IS NULL

  UNION ALL

  SELECT c.id, c.parent_id, t.depth + 1
  FROM categories c
  INNER JOIN traversal t ON c.parent_id = t.id
  WHERE NOT t.depth > 100  -- simple depth limit instead of explicit cycle detection
) CYCLE id SET is_cycle USING last_path
SELECT * FROM traversal WHERE NOT is_cycle;
```

Alternatives (most databases):
- **Array check**: `NOT id = ANY(path)` (PostgreSQL)
- **String containment**: Check if path contains current node
- **Depth limit**: `AND depth < max_allowed`

## CTE vs. Subquery Performance

CTEs are logically equivalent to nested subqueries but offer different optimization paths:

| Aspect               | CTE                                      | Subquery                           |
| -------------------- | ---------------------------------------- | ---------------------------------- |
| **Readability**      | Superior for complex queries             | Nested/hard to follow              |
| **Reusability**      | Defined once, used multiple times        | Must repeat or nest                |
| **Materialization**  | Database chooses (often materialized)    | Inline in many optimizers          |
| **Recursion**        | Supported (WITH RECURSIVE)               | Not possible                       |
| **Performance**       | Database-dependent; may be slower if materialized | May be inlined and optimized better |

Modern databases optimize both similarly. **Use CTEs for clarity and recursion; use subqueries for fine-grained inlining control**.

## Recursive CTE Depth and Performance

- **Iteration limit**: Most DBs have `max_recursive_iterative` settings (e.g., PostgreSQL sets no hard limit by default, but infinite loops are possible).
- **Memory**: Each iteration produces new rows; deep trees grow quickly.
- **Termination**: Recursive member must **always** produce fewer rows over time, or add explicit `LIMIT` / cycle detection.

```sql
-- Pattern: always terminate
UNION ALL
SELECT ...
WHERE depth < 100  -- safety bound
  AND NOT target_id = ANY(visited_path)  -- cycle check
```

## See Also

- [SQL Window Functions](sql-window-functions.md)
- [SQL Conventions and Idioms](language-sql.md)
- [Database Query Optimization](database-query-optimization.md)