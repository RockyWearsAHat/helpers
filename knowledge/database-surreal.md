# SurrealDB — Multi-Model Document, Graph & Relational Database

SurrealDB is a modern multi-model database engine written in Rust, unifying document storage, graph relationships, and relational queries in a single system. It targets developers seeking SQL-like querying on flexible schemas with embedded deployment and live query subscriptions.

## Multi-Model: Document, Graph, Relational

Unlike databases specializing in one model, SurrealDB treats documents, relationships, and tables as first-class citizens:

**Document model** — Records stored as JSON-like structures with flexible fields:
```javascript
{
  id: "user:alice",
  name: "Alice",
  email: "alice@example.com",
  settings: {
    theme: "dark",
    notifications_enabled: true
  }
}
```

**Graph model** — Relationships are **record links**, not separate edge documents:
```javascript
{
  id: "user:alice",
  follows: ["user:bob", "user:charlie"],
  created_by: "admin:root"
}
```

Links are **typed** (user → follows → user implies edge type) and queryable as relations. No separate edges table needed.

**Relational model** — Tables define schema constraints and relationships, enabling traditional normalized queries:
```sql
SELECT user.name, COUNT(post.id) as post_count
FROM user
LEFT JOIN post ON user.id = post.author_id
GROUP BY user.id;
```

SurrealDB permits mixing: a document may have unstructured fields AND typed links AND participate in normalized joins in the same query.

## SurrealQL: Query Language

SurrealQL extends SQL with graph traversal and document operations:

```sql
SELECT 
  user.name, 
  user->follows->name as friends,
  count(post) as post_count
FROM user
WHERE user.age > 18
GROUP BY user.name;
```

**Key features:**
- `->` operator for graph traversal (e.g., `user->posts` returns all posts linked from user)
- Array/object field projections: `SELECT user.settings.theme`
- RELATE statement to create edges: `RELATE user:alice->follows->user:bob SET created_at = now()`
- Recursive traversal (though depth must be bounded): `TRAVERSE(node, -> edge_type, <depth>)`
- LET clauses for computed fields
- Transactions: full ACID support for multiple statements in single transaction

SurrealQL reads like SQL but treats relationships as first-class navigation primitives (similar to query language in database-graph-database.md).

## Permissions Model: Record & Field Level

Fine-grained access control at multiple levels:

**Database/table scope:**
```sql
DEFINE SCOPE user_scope SESSION 24h
  CREATE USER:new WHEN $auth.role = 'admin';
```

**Record-level permissions:**
```sql
DEFINE TABLE access_log
  PERMISSIONS
    FOR CREATE ALLOW $auth.role = 'operator'
    FOR READ ALLOW $auth.role = 'operator' OR $auth.id = author_id
    FOR UPDATE ALLOW $auth.id = author_id
    FOR DELETE ALLOW $auth.role = 'admin';
```

**Field-level permissions:**
```sql
DEFINE FIELD password ON user
  PERMISSIONS
    FOR READ ALLOW false;
```

Field-level rules prevent unauthorized reads; queries omit restricted fields automatically. Permissions checked before query execution (not as post-filter).

## Live Queries & Subscriptions

Clients subscribe to query results; SurrealDB pushes updates when data changes:

```javascript
const subscription = await db.live("SELECT * FROM user WHERE age > 18");

subscription.on("update", (record) => {
  console.log("Updated:", record);
});

subscription.on("delete", (id) => {
  console.log("Deleted:", id);
});
```

Internal implementation uses **change tracking** — when a write modifies a row, SurrealDB evaluates affected subscriptions (re-runs WHERE clauses) and sends deltas to subscribers. Subscriptions maintain state server-side; client sockets receive only changes affecting their query.

## Embedded Deployment

SurrealDB can run **embedded in applications** (in-process) or as a server:

**Embedded mode:**
- RocksDB storage engine local to app (no network round-trip)
- Link SurrealDB Rust library into binary
- Single-process instance; replication unavailable
- Ideal for IoT, offline-first mobile, or local-first web

**Server mode:**
- Multiple clients connect via WebSocket or HTTP REST API
- Clustering and replication planned (as of 2025, single-node primary)
- Suitable for traditional client-server deployments

Unified query language across embedded and server modes eases development.

## Record Links & Type System

Records are identified by fully qualified IDs: `table:identifier` (e.g., `user:alice`, `post:123`).

```sql
CREATE user:alice SET
  name = "Alice",
  created_by = user:root,
  manages = [team:dev, team:infra];
```

Types:
- **String** — UTF-8 text
- **Number** — Integer or float (64-bit)
- **Boolean** — true/false
- **DateTime** — ISO8601 or Unix timestamp
- **Array** — Homogeneous or heterogeneous (no type enforcement)
- **Object** — Nested key-value
- **Record link** — Typed reference to another record (checked on query)
- **Geometry** — GeoJSON (Point, Polygon, etc.)

Validation prevents invalid links; attempting to link to non-existent table raises error (configurable: allow or reject).

## Array & Object Field Operations

Atomic operations on nested fields without full-document rewrite:

```sql
UPDATE user:alice
  SET favorites[+] = movie:interstellar,
      settings.theme = "light",
      tags += ["admin", "contributor"];
```

- `[+]` — Append to array
- `+=` — Union with array (no duplicates)
- `-=` — Remove elements matching array
- `*=` — Intersect arrays

Optimized internally to avoid full-document fetch-modify-write cycles.

## Internal Architecture & Storage

- **Storage engine** — RocksDB (LSM tree) for embedded; pluggable backends for server
- **Indexing** — B-tree indexes on fields; range scans optimized
- **Query planner** — Cost-based optimizer generates execution plan (similar to traditional RDBMS)
- **Execution** — Iterator-based (pull model); parallelism within single query

No columnar storage (not optimized for OLAP); bulk analytical queries slower than ClickHouse.

## Comparison to Traditional Databases

| Aspect | SurrealDB | MongoDB | PostgreSQL | Neo4j |
|--------|-----------|---------|-----------|-------|
| **Graph queries** | Native with `->` | Aggregation pipeline | No (JOINs laborious) | Native Cypher |
| **Schema** | Flexible schema + type defs | Flexible | Rigid schema | Schema per node type |
| **Transactions** | Multi-statement ACID | Multi-doc ACID | Full ACID | Single statement |
| **Permissions** | Record + field level | Role-based only | Row-level via policies | Role-based only |
| **Embedding** | Native Rust library | N/A | pg_embed (limited) | N/A |
| **Live queries** | Native subscriptions | Change streams | Listen/Notify | No |
| **Full-text search** | Basic (no tokenization) | Text Indexes only | Full-text indexes | N/A |

SurrealDB unifies use cases requiring both relational joins AND graph traversal + embedding flexibility, filling gap between MongoDB (documents only) and PostgreSQL (relational only).

## Trade-offs

**Strengths:**
- Multi-model queries in single transaction (no Join fragmentation between systems)
- Permissions at record/field granularity
- Embedded deployment with same query language
- Live queries for real-time UI updates

**Weaknesses:**
- Early-stage (single-node primary in 2025; clustering roadmap)
- Graph queries unoptimized vs. Neo4j (no parallel traversal, limited indexes on edges)
- No full-text search parity with Elasticsearch
- Fewer proven production deployments vs. MongoDB/PostgreSQL
- Performance: embedded mode slower than purpose-built systems due to generality

See also: database-graph-database.md, database-document-oriented.md, auth-access-control.md