# Couchbase — Distributed KV, N1QL, Full-Text Search & Mobile Sync

Couchbase is a distributed document database designed for high-throughput operational workloads, combining key-value store efficiency with SQL query capability and full-text search. Originally derived from Memcached + CouchDB, it powers real-time applications requiring sub-millisecond latency with multi-region replication and mobile synchronization.

## Architecture: Layered Services Model

Couchbase decouples workload handling into specialized services, allowing selective scaling:

- **Data Service** — Manages KV operations, caching layer, and persistence via B+ trees. Handles SET/GET/DELETE with automatic in-memory buffering (managed cache) and disk persistence to RocksDB. Each node maintains a subset of shard replicas, coordinated per-bucket.

- **Index Service** — Maintains secondary indexes (Global Secondary Indexes, GSI) in memory or disk. Stores index metadata and statistics separately from data, enabling index-only queries and covering indexes. Uses Forest-of-trees structures optimized for online index building without blocking writes.

- **Query Service** — Parses N1QL (SQL for JSON), plans execution, and coordinates distributed queries. Stateless query nodes route to Data and Index services. Supports prepared statements and query caching.

- **Search Service** — Full-text indexing powered by Bleve (Go-based inverted index library). Builds indexes asynchronously on data mutations. Handles tokenization, stemming, phrase queries, faceting, and range filtering on analyzed fields.

- **Eventing Service** — Triggers user-defined functions on data changes for reactive workflows. Functions run in V8 JavaScript engine with access to KV operations, timers, and buckets. Guarantees at-least-once execution per mutation.

Nodes self-organize via Erlang-like supervisor trees. Rebalancing redistributs vBucket (virtual bucket) replicas across the cluster without downtime.

## KV Operations & Consistency

- **GET/SET/DELETE** — Direct memory-resident operations with optional persistence. Durability levels: persisted-to-disk, replicated-to-N-nodes, or both. Client specifies desired consistency before reading (read-your-own-writes, strong consistency).

- **CAS (Compare-And-Swap)** — Optimistic locking via version token. Prevent lost updates in high-contention scenarios without distributed locks.

- **Sub-document API** — Atomically manipulate fragments of large JSON documents (e.g., increment a counter in an embedded object) without full-document rewrite.

- **Multi-Document Transactions** — ACID guarantees across multiple docs in same bucket via snapshot isolation. Reads lock docs, writes validate against snapshot. Aborts on conflict.

## N1QL: SQL for JSON

N1QL extends SQL with JSON-aware semantics:

```javascript
SELECT airline.name, route.distance 
FROM bucket AS airline 
UNNEST airline.routes AS route 
WHERE airline.country = "US" AND route.distance > 1000
```

**Features:**
- UNNEST/NEST for JSON arrays and objects (see database-graph-database.md for relational join analogs)
- LET clauses for computed expressions
- Subqueries and CTEs
- CASE, DISTINCT, GROUP BY, aggregate functions
- SCAN consistency levels: NOT_BOUNDED (eventual), AT_PLUS (read-your-write), REQUEST_PLUS (strong)

Execution uses columnar processing for aggregations; pushes filtering to Index Service when scan range available.

## Full-Text Search (FTS) & Eventing

**FTS Indexes** — Bleve-backed inverted indexes on document fields. Support fuzzy matching, n-grams, phrase queries, date/numeric range queries, and faceting. Indexed asynchronously; queries search either index or full bucket fallback.

**Analytics Queries** — Cross-bucket analytical SQL routed to shadow datasets, separate from operational data. Useful for BI workloads without impacting KV latency.

**Eventing** — User-defined functions respond to mutations:
```javascript
function OnUpdate(doc, meta) {
  if (doc.status === "completed") {
    var archive_doc = JSON.parse(JSON.stringify(doc));
    couchbase.insert(archive_bucket, meta.id, archive_doc);
  }
}
```

Guarantees: each mutation triggers handler at least once, ordered per doc, but may execute multiple times on node failure (requires idempotence).

## XDCR: Cross-Datacenter Replication

Asynchronous replication of buckets to peer clusters for geographic distribution and disaster recovery. **Unidirectional** by default (conflicts resolved by retaining source):

- Bi-directional XDCR — Applications must handle convergence (vector clocks or last-write-wins).
- Filtering — Replicate subset of keyspace via regex on doc ID.
- Compression — Optional as replication traffic.

No single source of truth across regions; all writes accepted locally and propagated eventually.

## Couchbase Lite & Sync Gateway

**Couchbase Lite** — Embedded document database for mobile/IoT, stores JSON locally with SQLite-like query interface. Syncs with server via Sync Gateway.

**Sync Gateway** — Node.js-based sync service mediating Lite clients and main cluster. Handles authentication, channel-based access control (docs assigned to channels), conflict resolution, and log-based replication (`_changes` feed). Clients receive push notifications of document changes.

## Comparison to MongoDB

| Aspect | Couchbase | MongoDB |
|--------|-----------|---------|
| **Query** | N1QL (SQL-like) with JOIN | Aggregation pipeline (imperative) |
| **Replication** | Asynchronous XDCR, no sync gateway analogue | Replica sets (synchronous majority quorum) |
| **Indexing** | Separate GSI service, covered queries | Embedded indexes in shard |
| **Transactions** | Multi-doc (bucket scoped) | Multi-doc (cross-collection) |
| **Mobile Sync** | Lite + Sync Gateway | Realm (separate product) |
| **KV Ops** | Millisecond (in-memory) | Document-level atomicity only |
| **Full-Text Search** | Native FTS service | Text indexes only |

Couchbase targets **operational workloads with mobile first-class citizen + analytics**; MongoDB targets **document-centric applications with aggregation flexibility**.

## Trade-offs

**Strengths:**
- Sub-millisecond GET/SET with managed cache
- Multi-model (KV + SQL + FTS) in one cluster
- Mobile-native story via Lite + Sync Gateway
- Eventing for reactive workflows

**Weaknesses:**
- Scaling write throughput requires manual partitioning (no auto-sharding like MongoDB)
- N1QL adoption slower than native document APIs
- Operational complexity: separate services add maintenance burden
- XDCR lacks total ordering (bi-directional XDCR requires app-level conflict resolution)

See also: database-distributed-sql.md, data-replication-patterns.md, database-query-planning.md