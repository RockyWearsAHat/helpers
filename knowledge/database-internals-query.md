# Database Query Processing Internals — Optimization, Execution, and Plan Selection

## The Query Pipeline: Parsing to Execution

Every SQL query undergoes a multi-stage pipeline:

1. **Lexical/Syntax Analysis**: Parser tokenizes SQL string, builds abstract syntax tree (AST)
2. **Semantic Analysis**: Verifies schema (tables, columns exist), resolves names
3. **Query Rewriting**: Simplifies and transforms the query (flatten subqueries, push down filters)
4. **Logical Planning**: Builds a tree of logical operators (Scan, Filter, Join, Aggregate, Sort)
5. **Cost Estimation**: Attaches estimated row counts and CPU/I/O costs to each operator
6. **Physical Planning**: Chooses concrete algorithms (which join method, index, sort strategy)
7. **Code Generation/Optimization**: Compiles or interprets the physical plan
8. **Execution**: Runs the plan, producing result rows

## Logical vs Physical Plans

**Logical plan** is algorithm-independent: "filter orders where customer_id = 5, then join with customers, then project order_id". Has multiple equivalent forms.

**Physical plan** is concrete: "use index idx_orders_customer_id to find matching rows (15 estimated rows), hash join with customers table broadcast from memory, project using expression evaluator". Embeds:
- Specific index choices
- Join algorithm (nested loop vs hash vs sort-merge)
- Sort method (in-memory vs external)
- Parallelization strategy

## Cost-Based Optimization (CBO)

A cost-based optimizer enumerates multiple physical plans, estimates cost (CPU + I/O + memory), and selects the cheapest.

**Cost model inputs**:
- **Table statistics**: Row count, page count, column distributions, distinct values
- **Index statistics**: Selectivity, cardinality
- **Join selectivity**: Estimated output rows of join (input rows * selectivity)
- **Hardware costs**: CPU cycles per operation, disk I/O latency, memory cost

**Cost model output**: Estimated cost in arbitrary units (PostgreSQL uses cost units that loosely approximate milliseconds). Typically models: tuples processed, disk page fetches, CPU operations.

**Limitations**: Statistics decay when data changes. Optimizer can choose badly if stats are stale. Cardinality estimation (predicting output rows) is notoriously difficult for complex filters and multi-table joins.

## Join Algorithms: Core Choices

The optimizer must choose which algorithm to use when joining two tables.

### Nested Loop Join (NLJ)

```
for each row r1 in table1:
    for each row r2 in table2:
        if r1.key == r2.key:
            emit (r1, r2)
```

- **Cost**: O(n₁ × n₂) comparisons
- **I/O**: Outer table scanned once, inner table scanned n₁ times
- **Memory**: Minimal (one row buffered)
- **Best for**: Small inner table (~<1000 rows), outer table very large. Inner table fits in buffer pool.
- **Variant**: Index nested loop — use index on inner table's join column (reduces inner scans from full table scans to index lookups)

### Hash Join

```
1. Build phase: scan table2, hash all rows into in-memory hash table (key → list of rows)
2. Probe phase: scan table1, for each row, hash and lookup in hash table
```

- **Cost**: O(n₁ + n₂) comparisons (linear if hash table fits in memory)
- **I/O**: Both tables scanned once each
- **Memory**: Must hold larger table's hash table in RAM (~8-16 bytes per row overhead)
- **Best for**: Larger tables where hash table fits in memory; equality joins
- **Variant**: Grace hash join (spill to disk) if hash table doesn't fit

### Sort-Merge Join

```
1. Sort table1 on join key (if not already sorted)
2. Sort table2 on join key (if not already sorted)
3. Merge: scan both sorted streams, match rows
```

- **Cost**: O((n₁ + n₂) log n) for sorting + O(n₁ + n₂) for merge
- **I/O**: Both tables sorted, single pass merge (good for disk I/O)
- **Memory**: O(k) buffer for merge (k = run merge factor)
- **Best for**: Both tables already sorted or indexes exist on join keys; OUTER joins; `k-way merge of pre-sorted input
- **Natural advantage**: Produces sorted output (good for subsequent GROUP BY or ORDER BY)

**Optimizer decision rule of thumb**:
- Small inner table → NLJ
- Both tables large, memory available → Hash
- Pre-sorted or index available → Sort-merge
- Unknown selectivity → Hash (safer linear behavior)

## Index Selection and Selectivity

The optimizer chooses whether to use an index or full table scan.

**B-tree index scan cost**:
- Estimated rows to fetch: selectivity × total rows
- Cost ≈ log₂(total rows) [tree traversal] + selectivity × total rows [row fetches]

**Full scan cost**:
- Must read all pages sequentially
- Cost ≈ total pages

**Selectivity thresholds**: Typically, if selectivity > 5-10% of table, full scan is cheaper than index. Exact threshold depends on page costs vs random I/O costs.

**Composite indexes**: Index on (customer_id, order_date) can answer queries on (customer_id) or (customer_id, order_date), but not (order_date) alone. Optimizer uses **leftmost-prefix rule** to determine applicability.

## Query Rewriting and Transformation

The optimizer applies rewrites to simplify or improve logical plans.

**Subquery flattening**:
```sql
-- Original
SELECT * FROM orders WHERE customer_id IN (SELECT id FROM customers WHERE country = 'US')

-- Rewritten (semi-join)
SELECT o.* FROM orders o, customers c 
WHERE o.customer_id = c.id AND c.country = 'US'
```

**Predicate pushdown**: Move filters lower in the plan tree to reduce data early.
- Original: Scan table → Filter → Join
- Rewritten: Filter on table1 → Join filtered results

**Constraint propagation**: If a filter on a join key exists, propagate equivalent filters to both sides.

**View merging**: Inline materialized view definitions into the query.

## Execution Models: Three Approaches

### Volcano (Iterator) Model

Each operator implements `next()` returning one row at a time:

```
class FilterOperator:
    def next(self):
        while True:
            row = child.next()
            if row matches filter:
                return row
```

Operators form a callable tree. Root repeatedly calls next(), pulling rows up the tree.

**Pros**: Simple, composable, supports correlated subqueries, pipelining of operations
**Cons**: Function call overhead per row, poor CPU cache usage, can't exploit SIMD

### Vectorized Execution

Operators process **batches of rows at once** (e.g., 1000 rows):

```
class FilterOperator:
    def next_batch(self):
        rows = child.next_batch()  # 1000 rows
        return [row for row in rows if row matches filter]
```

Each operator works on arrays (vectors), enabling:
- Loop-unrolling and SIMD instructions
- Better CPU cache utilization
- Reduced function call overhead

Used by: CockroachDB, DuckDB, Databend (analytics)

**Pros**: 5-10x speedup from cache/SIMD, good for OLAP workloads
**Cons**: Still operates on pre-computed plans (less flexible than Volcano)

### Compiled Execution

Query plan compiled to native machine code before execution:

```c
// Pseudocode compiled from: SELECT o.amount FROM orders WHERE o.customer_id = 5
for (int i = 0; i < orders.count; i++) {
    if (orders[i].customer_id == 5) {
        output(orders[i].amount);
    }
}
```

Compiler can:
- Inline everything (no function calls)
- Apply aggressive optimizations (branch prediction, loop unrolling)
- Generate SIMD code

Used by: HyPer, Umbra, SQL Server (in newer versions)

**Pros**: Best performance for analytical queries; JIT compilation overhead amortized over large queries
**Cons**: Compilation latency (milliseconds), memory overhead, harder to debug

**Trade-off**: Volcano is most flexible but slowest. Compiled fastest but least flexible. Vectorized is practical middle ground.

## Materialized Views and Query Caching

**Materialized view**: Pre-computed result of a query stored as a table. Queries matching the view's definition use the pre-computed result instead of re-computing.

Example: Heavy analytics queries on (customers JOIN orders GROUP BY country) can be materialized hourly, speeding repeated queries.

Cost: Update overhead. If base tables change frequently, view maintenance is expensive.

**Query caching**: Cache query results by hash(query_text, parameters). On identical query, return cached result without planning/execution.

Invalidation challenge: Must clear cache when any underlying table changes.

## EXPLAIN: Understanding the Optimizer's Decisions

Every database provides EXPLAIN to inspect chosen plans:

```sql
EXPLAIN SELECT o.* FROM orders o JOIN customers c ON o.customer_id = c.id WHERE c.country = 'US';

-- PostgreSQL output:
Hash Join  (cost=100.50..500.00 rows=100)
  Hash Cond: (o.customer_id = c.id)
  ->  Seq Scan on orders o  (cost=0.00..200.00 rows=10000)
  ->  Hash  (cost=50.00..50.00 rows=50)
        ->  Seq Scan on customers c  (cost=0.00..50.00 rows=1000)
              Filter: (country = 'US')
```

Key metrics:
- `cost=startup..total`: Estimated startup cost (first row) and total cost
- `rows`: Estimated output rows
- `Buffers`: Pages read from cache vs disk (ANALYZE mode)
- `actual time`: Real execution time (ANALYZE mode)

## Key Insights for Practitioners

- **Stats matter**: Stale statistics lead to bad plans. `ANALYZE` or `VACUUM ANALYZE` regularly.
- **Join order matters**: n-way joins have (n-1)! possible orderings. Optimizer searches heuristically (not exhaustively).
- **Selectivity estimation is hard**: Database can't always predict filter cardinality without histograms; EXPLAIN is invaluable for verification.
- **Indexes don't always help**: High-selectivity filters (> 10%) may be better served by full table scans.
- **Watch for sequential access patterns**: Compiled and vectorized models favor loops over random branching.
- **No one-size-fits-all**: OLTP favors Volcano (flexible, online). OLAP favors vectorized/compiled (throughput).

## See Also

- database-query-optimization (practical index strategies and EXPLAIN usage)
- database-internals-storage (how indexes and tables are stored)
- performance-optimization (profiling and bottleneck identification)