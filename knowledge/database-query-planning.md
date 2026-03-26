# Database Query Planning: Cost-Based Optimization, Plan Nodes, and Adaptive Execution

## Cost-Based Query Optimization

Modern relational engines use cost-based optimization (CBO) rather than rule-based heuristics. The optimizer enumerates candidate physical plans and selects the one with lowest estimated cost. Cost is typically measured in abstract units: sequential disk I/O, random disk I/O, and CPU operations, weighted by constants calibrated to the system's hardware.

**Plan Enumeration**: The optimizer builds a search space of logically equivalent plans. For a three-table join, it considers different join orders (AB then C, AC then B, BA then C, etc.) and different join algorithms (nested loop, hash join, sort-merge). For large queries, the search space explodes—most optimizers use heuristics or dynamic programming to prune the space.

**Cost Estimation**: Each plan node estimates output rows (cardinality) and cost. The sum of costs up the tree gives the total plan cost. Cardinality estimation is the bottleneck: if the optimizer underestimates or overestimates output rows, it may choose a terrible plan. Errors compound—a row count miss of 10x at a join becomes 100x pessimism downstream.

**Statistics and the Histogram**: The optimizer relies on table and column statistics. PostgreSQL stores histograms in `pg_statistic`, bucketing column values and tracking frequencies. For a query `WHERE age > 50`, the optimizer looks up the histogram to estimate how many rows match. With stale statistics (old ANALYZE runs), the optimizer has corrupted input and produces bad plans.

## Join Algorithms: Nested Loop, Hash Join, Merge Join

**Nested Loop Join**: For each row from the outer table, scan the inner table fully or via index. Cost: O(M * N) where M and N are table sizes. Efficient only for small inner tables or highly selective indexes. Used when the inner table can be indexed.

**Hash Join**: Build a hash table from the inner table, then scan the outer table and probe the hash table. Cost: O(M + N) amortized. Efficient for large tables. Fails if memory is insufficient (spills to disk, degrading performance). Used by default for large joins.

**Merge Sort Join**: Sort both tables by join key (or use existing sort order from an index), then scan both sorted streams. Cost: O(M log M + N log N). Efficient when join keys are already sorted (e.g., join on primary key of both tables). Produces sorted output, useful if the next step needs sorting.

Optimizer trade-off: A nested loop join might scan a small filtered result set faster than hashing a large table into memory. Plan selection depends on row count estimates; errors compound.

## Plan Nodes and Execution Concepts

**Seq Scan**: Sequential table scan. Reads table data in physical disk order. Fast for small tables or when filtering must examine most rows.

**Index Scan**: Navigates the index to find rows matching the predicate, then fetches heap rows. Cost depends on index selectivity—if the index returns 10% of rows with only 5 pages, index scan wins. If it returns 80% with many random page fetches, seq scan is faster.

**Index-Only Scan**: Returns rows directly from a covering index without heap access. Requires the index to include all output columns and the Index Visibility Map (PostgreSQL) to confirm visibility. Much faster than Index Scan for hot queries.

**Bitmap Index Scan**: Combines multiple indexes. Scan each index, collect result bitmaps (which rows match), AND/OR the bitmaps, then fetch heap pages in sorted order. Useful for multi-column predicates like `WHERE col1 = 'a' AND col2 > 10`.

**Hash Join**: Build hash table from one input, probe with the other.

**Nested Loop Join**: Loop-and-probe join algorithm.

**Merge Join**: Sort-merge join on pre-sorted or indexed inputs.

**Sort**: Sorts rows by a key. In-memory sort if data fits; external merge sort (disk spilling) otherwise. Major cost driver for large result sets.

**Aggregate** (COUNT, SUM, GROUP BY): Accumulates values into groups. Hash aggregate (partition rows by group key, sum within each partition) vs. sorted aggregate (requires input sorted by group key).

## EXPLAIN and EXPLAIN ANALYZE

**EXPLAIN** shows the planned execution without running the query. Output format: each line is a plan node indented one level per nesting. Costs are estimated.

```
Index Scan on users (cost=0.1..12.4 rows=50 width=40)
  Index Cond: (id = 5)
```

The cost range `0.1..12.4` means: starting cost 0.1 (cpu to begin), total estimated cost 12.4. Rows and width are estimated cardinality and average row size.

**EXPLAIN ANALYZE** executes the query and shows actual runtime statistics: actual row counts, loop iterations, buffer hits vs. misses. Invaluable for debugging bad plans. Compares estimated vs. actual; large discrepancies indicate stale statistics.

```
Index Scan on users (cost=0.1..12.4 rows=50 width=40) (actual time=0.02..0.05 rows=1)
  Index Cond: (id = 5)
```

Actual rows=1 vs. estimated rows=50 suggests the statistic is outdated or the predicate is much more selective than the histogram showed.

## Plan Caching and Prepared Statements

When a query is prepared, the optimizer generates a generic plan using no value constants. On subsequent executions, the plan is reused. This avoids re-planning overhead but may choose a plan that's suboptimal for specific parameter values.

Example: `PREPARE stmt AS SELECT * FROM orders WHERE status = $1`. The first execution with `status='pending'` might use a seq scan. The second with `status='rare_value'` is better served by an index, but the generic plan may ignore this.

Some optimizers (PostgreSQL, SQL Server) support adaptive planning: the first few executions use custom plans, switching to a generic plan only if cost savings justify it. This balances planning overhead against plan quality.

Different databases have different rules: Oracle uses custom plans by default; PostgreSQL caches after the first few executions; SQL Server chooses generically.

## Query Hints and Forcing Plans

Some systems allow query hints to override the optimizer. `/*+ INDEX(table idx_name) */` in Oracle forces index use; `USE INDEX` in MySQL. Hints are a last resort when the optimizer catastrophically mispredicts.

Downsides: Hints become technical debt—they're brittle, don't adapt to schema changes, and obscure intent. If you need hints, first investigate why statistics are stale or the optimizer is mispredicting.

## Adaptive and Runtime Query Execution

Modern engines support *adaptive query execution*: the plan adjusts at runtime based on actual data. If a join predicate returns far more rows than estimated, the engine might switch join algorithms mid-execution.

Systems implementing this (SQL Server Adaptive Joins, Columnar engines like DuckDB) gather feedback during execution and replan or adjust strategies on-the-fly. Trade-off: adaptive execution requires runtime overhead to monitor and decide; it helps worst-case scenarios but adds latency to typical cases.

## Cardinality Estimation Errors and Mitigation

The biggest source of bad plans is row count underestimation. A 10x underestimate might cause the optimizer to choose a nested loop join instead of a hash join—a 100x slowdown.

Causes:
- Stale statistics (old ANALYZE)
- Correlated columns (optimizer assumes independence; it doesn't)
- Predicates with complex conditionals (optimizer uses heuristics, not real statistics)

Mitigation:
- Run ANALYZE frequently, especially after bulk loads
- Monitor for missing or unused indexes
- Use EXPLAIN ANALYZE to verify estimates; if actual rows differ 10x+, investigate
- Consider materialized aggregates or denormalization for predictable queries