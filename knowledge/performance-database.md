# Performance: Database Optimization — Queries, Indexes, Scaling, and Tuning

Database performance is often the limiting factor in system scale. Optimizing queries, choosing indexes wisely, and scaling architecture can improve throughput by 10-100×.

## Query Optimization Fundamentals

### Understanding Execution Plans

A **query plan** is the database's roadmap for executing a query. It shows:
- Which tables are scanned (full table scan vs index scan)
- How tables are joined (nested loop, hash join, merge join)
- Filter application order
- Estimated vs actual row counts

```sql
EXPLAIN ANALYZE SELECT * FROM orders WHERE customer_id = 123 AND status = 'shipped';
```

Postgres output (simplified):
```
Seq Scan on orders (cost=0.00..35.50 rows=5 width=64)
  Filter: (customer_id = 123 AND status = 'shipped')
  Actual rows: 5
```

**Full seq scan** means the database scanned every row and filtered. This scales linearly with table size (O(n)). For a billion-row table, unacceptable.

With an index:
```
Index Scan using orders_customer_id on orders (cost=0.29..8.31 rows=5)
  Index Cond: (customer_id = 123)
  Filter: (status = 'shipped')
  Actual rows: 5
```

**Index scan** is logarithmic in table size (O(log n)), then applies the filter. Much better.

### Key Metrics

**Rows estimated vs actual**: If the optimizer estimates 100 rows but gets 1000, cardinality estimation is wrong (stale statistics). The plan chosen for 100 rows (e.g., hash join) might be terrible for 1000.

**Plan cost**: Arbitrary units (disk block accesses, roughly). Not walltime. Use `EXPLAIN ANALYZE` to see actual execution time.

**Sequential vs random I/O**: Sequential I/O (reading consecutive disk blocks) is 100-1000× faster than random I/O (seeking). An index on a column with poor locality (e.g., a boolean) might force random reads, losing advantage.

### Query Anti-Patterns

**N+1 Queries**: Fetch a list, then for each item, fetch related data. Instead of 1 query, 1 + N queries.

```python
# Bad: N+1
users = db.query("SELECT * FROM users LIMIT 100")
for user in users:
    profile = db.query(f"SELECT * FROM profiles WHERE user_id = {user.id}")  # 100 more queries
```

**Solution:** JOIN or explicit batch loading:
```python
# Good: 1 query
users_with_profiles = db.query("""
  SELECT u.*, p.* FROM users u
  LEFT JOIN profiles p ON u.id = p.user_id
  LIMIT 100
""")
```

**Implicit type conversion**: Searching a string column with a number:

```sql
SELECT * FROM products WHERE sku = 123  -- sku is VARCHAR
```

The optimizer might be forced to convert all 10M SKUs to numbers, defeating indexes. Use explicit types:
```sql
SELECT * FROM products WHERE sku = '123'
```

**OR with multiple columns**: ORs often defeat indexes because the optimizer can't use a single index efficiently:

```sql
SELECT * FROM users WHERE (first_name = 'Alice' OR last_name = 'Alice')
-- Might do full table scan; can't use a (first_name, last_name) index efficiently
```

**Solution:** UNION:
```sql
SELECT * FROM users WHERE first_name = 'Alice'
UNION
SELECT * FROM users WHERE last_name = 'Alice'
```

## Index Strategy

### B-Tree Indexes (Default)

Most databases default to B-tree indexes (balanced tree structure). Efficient for range queries and equality. Cost: write amplification (every insert/update/delete modifies the index).

**Index selectivity**: An index is useful only if it dramatically reduces rows examined. An index on `gender` (2 values) has low selectivity and often isn't used. An index on `user_id` (millions of values) has high selectivity.

**Composite indexes**: A single index on multiple columns:

```sql
CREATE INDEX idx_customer_status ON orders(customer_id, status);
```

Queries matching the **leftmost prefix** use the index:
- `WHERE customer_id = 123` ✓ uses index
- `WHERE customer_id = 123 AND status = 'shipped'` ✓ uses index
- `WHERE status = 'shipped'` ✗ does NOT use index (status is not leftmost)
- `WHERE customer_id = 123 AND status = 'shipped' AND created_at > now()` ✓ uses index for first two, filters created_at

**Cost of extra indexes**: Each index consumes disk space, slows writes, and complicates the optimizer's decision-making. Don't index everything; choose strategically.

### Partial Indexes

Index only a subset of rows:

```sql
CREATE INDEX idx_active_users ON users(email) WHERE status = 'active';
```

Smaller, faster. Useful when queries frequently filter the same way (e.g., "find active user by email").

### JSONB Indexes

Modern databases support indexing into JSON. PostgreSQL's GIN (Generalized Inverted Index) indexes JSON keys and values:

```sql
CREATE INDEX idx_metadata_id ON logs USING GIN (data);
SELECT * FROM logs WHERE data->>'user_id' = '123';  -- Uses index
```

## Connection Pooling

A database connection is expensive to create (TCP handshake, authentication, negotiation). Opening a new connection for each request causes:
- Connection exhaustion (database limits concurrent connections)
- Latency (10-100ms per connection)
- Resource waste

**Connection pooling** maintains a pool of open connections, reusing them across requests.

```python
pool = psycopg2.pool.SimpleConnectionPool(1, 20, dbname="mydb")
conn = pool.getconn()
try:
    cursor = conn.cursor()
    cursor.execute("SELECT ...")
finally:
    pool.putconn(conn)
```

**Key settings:**
- **Min pool size**: Keep this many connections warm at all times
- **Max pool size**: Hard limit; excess requests wait for a connection to free
- **Idle timeout**: Close unused connections after N seconds (prevents stale connections)
- **Queue timeout**: How long to wait for a connection before failing the request

Most frameworks (Django, SQLAlchemy, Spring Data JPA) bundle pooling. External poolers (PgBouncer for PostgreSQL) multiplex thousands of app connections onto a smaller pool to the database.

## Read Replicas and Scaling Reads

A **read replica** is an asynchronous copy of the primary database, updated via replication stream (thousands of writes/second lag is common).

**Use replicas for:**
- Offloading read traffic (primary handles writes; replicas handle reads)
- Geographic distribution (replicas in different data centers)
- Analytics (point replicas at a separate analytics database)

**Tradeoff:** replicas can fall slightly behind the primary (eventual consistency). If your application binds queries to a replica immediately after a write, stale reads are possible. Mitigate with:
- Read-your-writes consistency: immediately after a write, read from the primary
- Causal consistency: track LSN (log sequence number) and ensure the read replica has reached at least that LSN before querying

## Materialized Views and Denormalization

A **materialized view** is a precomputed query result stored as a table. Faster than computing the result repeatedly but requires refresh:

```sql
CREATE MATERIALIZED VIEW user_order_totals AS
  SELECT user_id, COUNT(*) as order_count, SUM(amount) as total
  FROM orders
  GROUP BY user_id;

REFRESH MATERIALIZED VIEW user_order_totals;  -- Update when underlying data changes
```

Useful when the view is queried frequently but refreshed rarely (e.g., daily).

**Denormalization**: Store derived data in the same table to avoid joins:

```sql
-- Normalized:
SELECT SUM(amount) FROM orders WHERE user_id = 123;

-- Denormalized:
SELECT total_spent FROM users WHERE id = 123;  -- Total maintained in the users table
```

**Tradeoff:** denormalization speeds reads but complicates writes (updates must maintain consistency). Use sparingly and only when the read performance gain is critical.

## Partitioning for Performance

Partitioning divides a large table into smaller pieces, each stored separately. Benefits:
- **Parallelism**: queries on different partitions can execute in parallel
- **Faster scans**: scanning 1/10 of the data is ~10× faster
- **Index efficiency**: indexes on smaller tables are smaller and faster

**Partition strategies:**

**Range partitioning**: Divide by value ranges (e.g., orders by year):
```sql
CREATE TABLE orders (id INT, user_id INT, created_at DATE) 
  PARTITION BY RANGE (YEAR(created_at)) (
    PARTITION p2023 VALUES LESS THAN (2024),
    PARTITION p2024 VALUES LESS THAN (2025)
  );
```

Queries on specific years automatically route to the right partition.

**Hash partitioning**: Divide by hash of a column:
```sql
CREATE TABLE users (id INT) PARTITION BY HASH (id) PARTITIONS 10;
```

Even distribution but loses ordering properties.

## Maintenance: Vacuum, Bloat, and Autovacuum

In MVCC (Multi-Version Concurrency Control) databases like PostgreSQL, deletes don't immediately free space; they mark rows as deleted. Over time, tables accumulate dead rows (bloat).

**Autovacuum**: Background process that periodically reclaims space. Important to tune:
- Too frequent: wastes CPU
- Too infrequent: tables bloat, queries slow

**Manual VACUUM ANALYZE**: Force reclamation and update statistics:
```sql
VACUUM ANALYZE users;
```

Bloat is detectable via `pgstattuple()` extension or by observing table size growth.

## Memory Tuning and Buffer Pool

The **buffer pool** (or page cache) is in-memory storage for frequently accessed disk blocks. Hit rate determines performance:
- **High hit rate** (90%+): most data queries find in RAM; effectively instant
- **Low hit rate** (10%): most queries miss, forcing disk reads; 100-1000× slower

**Tuning:**
- Allocate 25% of system RAM to the buffer pool (but max 40GB; DRAM doesn't scale linearly for huge pools)
- Monitor hit rate; if consistently <80%, either increase pool size or reduce working set

For workloads with unpredictable access patterns, hit rate is hard to improve; focus on query optimization instead.

## IO Optimization

**SSD vs HDD**: SSDs have lower latency (~1-5ms vs 10-50ms) and higher IOPS (100K+ vs 1K). If the database is IO-bound, SSDs provide dramatic speedup.

**RAID configuration**: RAID 1 (mirroring) for safety without performance loss. RAID 5/6 for capacity efficiency but write overhead. Preference: RAID 1 for databases (safety > capacity).

**Concurrent I/O**: Multiple queries reading/writing simultaneously stress I/O subsystem. Use storage QoS to isolate workloads or distribute queries across servers.

## See Also

- database-internals-query.md — execution engine deep dive
- database-query-optimization.md — PostgreSQL-specific optimization details
- database-patterns.md — denormalization and schema patterns
- patterns-pagination.md — efficient pagination strategies