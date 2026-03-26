# Database Patterns & Design Considerations

## Normalization

Reduce data redundancy by organizing tables according to normal forms.

- **1NF**: Each column holds atomic values (no arrays, no repeating groups).
- **2NF**: Every non-key column depends on the entire primary key (not just part of a composite key).
- **3NF**: No transitive dependencies — non-key columns depend only on the primary key, not on other non-key columns.

**3NF is the sweet spot for most applications.** Denormalize intentionally (and document why) when performance requires it.

## Indexing Strategies

Indexes speed up reads at the cost of slower writes and more storage.

### When to Index

- Columns in WHERE clauses.
- Columns in JOIN conditions.
- Columns in ORDER BY / GROUP BY.
- High-cardinality columns (many unique values) benefit most.

### When NOT to Index

- Tables with very few rows (full scan is faster).
- Columns with low cardinality (boolean flags — index doesn't help much).
- Columns that are frequently updated (index maintenance overhead).
- Write-heavy tables where read performance isn't critical.

### Index Types

- **B-Tree** (default): Good for equality and range queries. Most common.
- **Hash**: Only equality comparisons. Faster than B-Tree for exact matches.
- **GIN/GiST**: Full-text search, JSONB, arrays, geometry (PostgreSQL).
- **Composite index**: Multi-column. Order matters — leftmost columns are usable independently.
- **Partial index**: Index only rows meeting a condition (`WHERE active = true`).
- **Covering index**: Includes all columns needed by a query — avoids table lookup.

### Use EXPLAIN

Analyzing query plans before and after adding indexes reveals whether they're helping:

```sql
EXPLAIN ANALYZE SELECT * FROM orders WHERE user_id = 42 AND status = 'pending';
```

Look for: Seq Scan (bad on large tables), Index Scan (good), Bitmap Index Scan (acceptable).

## The N+1 Query Problem

The most common ORM performance anti-pattern.

**Problem:** Loading a list of items, then issuing a separate query for each item's related data.

```python
# N+1: 1 query for users + N queries for orders
users = User.objects.all()         # SELECT * FROM users (1 query)
for user in users:
    orders = user.orders.all()     # SELECT * FROM orders WHERE user_id = ? (N queries)
```

**Solution:** Eager loading / JOINs.

```python
# 1 query (or 2 with prefetch)
users = User.objects.prefetch_related('orders').all()
```

Every ORM has this problem. Django: `select_related`/`prefetch_related`. SQLAlchemy: `joinedload`/`subqueryload`. ActiveRecord: `includes`. Hibernate: `@Fetch(FetchMode.JOIN)`.

## Query Optimization

1. **SELECT only needed columns** — `SELECT name, email` not `SELECT *`.
2. **Avoid functions on indexed columns in WHERE** — `WHERE YEAR(created_at) = 2024` can't use an index. Use `WHERE created_at >= '2024-01-01' AND created_at < '2025-01-01'`.
3. **Use LIMIT** — Never return unbounded result sets.
4. **Avoid OR on different columns** — Often prevents index use. Consider UNION instead.
5. **Use EXISTS over IN for correlated subqueries** — `WHERE EXISTS (SELECT 1 FROM ...)` is often faster.
6. **Batch inserts** — One `INSERT INTO ... VALUES (...), (...), (...)` beats 1000 individual inserts.

## Connection Pooling

Creating database connections is expensive (TCP handshake, authentication, SSL negotiation).

- **Connection pools are strongly recommended** in production.
- A common starting point for pool size: 2 × CPU cores + disk spindles (adjust empirically).
- Set connection timeouts and idle timeouts.
- Tools: PgBouncer (PostgreSQL), HikariCP (Java), SQLAlchemy pool (Python).

## Transactions and Consistency

- Use transactions for operations that must be atomic.
- Keep transactions short — long transactions hold locks and block other operations.
- Understand isolation levels:
  - **Read Uncommitted**: Fastest, dirty reads possible.
  - **Read Committed**: Default in PostgreSQL. No dirty reads.
  - **Repeatable Read**: No phantom reads. Default in MySQL InnoDB.
  - **Serializable**: Full isolation. Slowest.
- Use optimistic locking (version column) for low-contention updates.
- Use pessimistic locking (`SELECT ... FOR UPDATE`) only when contention is high.

## Migration Patterns

- **Version all schema changes** in migration files (Flyway, Alembic, Knex, ActiveRecord migrations are common tools).
- **Make migrations reversible** when possible.
- **Prefer additive changes in production**: Add new columns as nullable, backfill, then add constraints. Renaming or dropping columns benefits from a deprecation period.
- **Test migrations** on a copy of production data before deploying.
- **Separate deploy from migrate**: Deploy code that works with both old and new schema, then migrate, then clean up.

## Data Modeling Patterns

### Soft Deletes

`deleted_at` timestamp instead of `DELETE`. Preserves audit trail. Add to all queries: `WHERE deleted_at IS NULL`. Consider: filtered indexes, periodic hard-delete of old records.

### Audit Trail

Track who changed what and when. Options:

- Trigger-based: database triggers write to audit table.
- Application-level: middleware logs changes.
- Event sourcing: events ARE the audit trail.

### Multi-Tenancy

- **Shared database, shared schema**: Tenant ID column on every table. Simplest. Risk: data leaks if you forget the WHERE clause.
- **Shared database, separate schemas**: Each tenant has own schema. Better isolation.
- **Separate databases**: Full isolation. Most complex operationally.

---

_Sources: PostgreSQL Documentation, Use The Index Luke (Markus Winand), High Performance MySQL (Baron Schwartz), Django ORM documentation, Martin Fowler (Patterns of Enterprise Application Architecture)_
