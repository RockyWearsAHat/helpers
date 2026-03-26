# AWS DynamoDB Patterns — Single-Table Design, Access Patterns, and Optimization

## Single-Table Design Fundamentals

Traditional relational database normalization distributes data across multiple tables (Users, Orders, Products, Reviews) to avoid redundancy. DynamoDB single-table design stores multiple entity types in one table using composite keys to distinguish and organize data.

```
PK              | SK                   | Type     | Attributes
USER#alice      | PROFILE              | User     | name, email, created
USER#alice      | ORDER#2025-001       | Order    | total, status, created
USER#alice      | ORDER#2025-002       | Order    | total, status, created
ORG#acme        | METADATA             | Org      | name, plan, created
ORG#acme        | MEMBER#alice         | Member   | role, joinedDate
PRODUCT#widget  | METADATA             | Product  | name, price, sku
PRODUCT#widget  | REVIEW#alice#20241201| Review   | rating, text, created
```

The partition key (PK) groups related entities. The sort key (SK) orders them and creates distinct logical records. Type discriminators (strings like "PROFILE", "ORDER", etc.) enable application logic to parse entity types.

### Benefits and Tradeoffs

**Benefits**: A single table reduces operational overhead (one throughput capacity bucket, one backup, one GSI to maintain). Query patterns that would require joins in SQL can be fetched in one round trip. Scaling is simpler — no shard key design across tables.

**Tradeoffs**: Design is tighter. Adding new entity relationships requires rethinking keys. Scanability is reduced (you cannot easily query all Orders across all users without a GSI or full scan). Denormalization increases storage (data duplication across records).

Single-table design is not universally applicable. Multi-table designs remain valid for workloads with simple, unchanging access patterns or teams unfamiliar with DynamoDB.

## Access Patterns and Global Secondary Indexes (GSIs)

Before designing keys, enumerate all read and write patterns:

```
1. Get user profile by userId          → Direct Query: PK = USER#alice, SK = PROFILE
2. Get user's orders (all)             → Query: PK = USER#alice, SK begins_with ORDER#
3. Get user's orders (by date range)   → Query: PK = USER#alice, SK between ORDER#2025-01-01 and ORDER#2025-01-31
4. Get all users in an org             → Query: PK = ORG#acme, SK begins_with MEMBER#
5. Get reviews for a product           → Query: PK = PRODUCT#widget, SK begins_with REVIEW#
6. Get all products (no direct PK)     → GSI needed: PK = PRODUCT_CATALOG, SK = sku or name
7. Find users by email (no direct PK)  → GSI needed: PK = email, SK = userId
```

Each GSI is a projection of the table with different keys. GSIs count toward read/write capacity. Sparse GSIs (only including items meeting a condition) reduce cost by excluding irrelevant items.

### GSI Overloading

One GSI can serve multiple access patterns by reusing key attributes for different purposes:

```
GSI: InvertedIdx
- PK: email          (search by email)
- SK: userId         (also serves reverse lookup: given userId, list users with that email if multiple)

GSI: TimeIdx
- PK: PRODUCT        (partition by product type)
- SK: createdDate    (sort by time; serves "list products by creation date" pattern)
```

Overloading requires discipline. The GSI key design must support all target access patterns without forced scans or excess capacity.

## Access Pattern Examples

### Hierarchical Data

Representing nested relationships (orgs → teams → members):

```
PK: ORG#acme           SK: METADATA                    (org record)
PK: ORG#acme           SK: TEAM#backend                (team under org)
PK: ORG#acme           SK: TEAM#backend#MEMBER#alice  (member of team)
```

Query `PK = ORG#acme, SK begins_with TEAM#` retrieves all teams. Query `PK = ORG#acme, SK begins_with TEAM#backend` retrieves all members of backend team.

### Time-Series Data

```
PK: SENSOR#device123   SK: 2025-03-01T10:00:00Z       (timestamp)
PK: SENSOR#device123   SK: 2025-03-01T10:00:01Z
```

Query `PK = SENSOR#device123, SK between T1 and T2` retrieves time-series window. Combine with TTL to expire old readings.

## Capacity Modes: On-Demand vs. Provisioned

**On-Demand** — Pay per request. No capacity planning. Automatic unlimited throughput (subject to account limits). Best for unpredictable workloads, new applications, or development/testing.

**Provisioned** — Reserve read/write capacity (RCUs/WCUs). Pay for reserved capacity whether or not you use it. Cheaper per request at scale. Requires accurate capacity forecasting. Underprovisioned tables are throttled; overprovisioned capacity is waste.

Tradeoff: On-demand costs more per request but requires no forecasting. Provisioned costs less per request but requires planning and monitoring. Switching between modes incurs a 24-hour cooldown.

## DynamoDB Transactions

Transactions ensure ACID guarantees across up to 25 items using `TransactWriteItems` and `TransactGetItems`. Writes within a transaction succeed or all fail atomically. Useful for maintaining consistency across related records (e.g., decrement inventory and create order atomically).

Tradeoff: Transactions consume double the provisioned capacity (reads/writes are charged twice). All items must be in the same table (in older API versions; newer versions support cross-table transactions in limited scenarios). Complex transaction logic can become a bottleneck.

## Global Secondary Indexes (GSIs)

GSIs replicate table data with different keys. They are eventually consistent with the base table (updates propagate asynchronously). GSIs consume separate read/write capacity (on provisioned mode) or are billed per request (on-demand mode).

Sparse indexes include only items where a particular attribute exists, reducing size and cost. Example: a `UserEmailGSI` that only includes users who have provided an email address.

GSIs cannot be queried with consistency guarantees; all GSI queries are eventually consistent from the client perspective (though replicas are consistent with each other).

## DynamoDB Streams and Change Data Capture

DynamoDB Streams capture insert, update, and delete events on a table. Streams are ordered by item and can be consumed by Lambda (event mapping) or applications reading the stream API.

Two stream views: `NEW_IMAGE` (new item state after mutation) and `OLD_IMAGE` (old item state before mutation). Both enable change data capture, audit logging, and downstream system synchronization.

Streams have a 24-hour retention window. If consumers lag, events are lost.

## Time-to-Live (TTL)

Designate an attribute as TTL (Unix timestamp). DynamoDB automatically deletes items when the current time exceeds the TTL value. Deletions eventually happen (not instant) and do not consume write capacity.

TTL is ideal for temporary data (sessions, cache entries, time-bound subscriptions). Tradeoff: Eventual deletion; you cannot rely on immediate removal for strict compliance use cases.

## DAX (DynamoDB Accelerator)

DAX is a managed, in-memory cache for DynamoDB. Queries and scans are cached; subsequent requests return from DAX without hitting DynamoDB, reducing latency and throughput consumption.

DAX is a separate cluster managed by AWS. It charges separately and adds operational complexity. Useful for read-heavy workloads with repetitive queries. Invalidation strategy is important; DAX has a TTL-based eviction policy.

## Migration from Relational Databases

Migrating from RDBMS (PostgreSQL, MySQL) to DynamoDB requires rethinking data structure:

- **Denormalize aggressively** — Joining tables in DynamoDB is expensive; embed related data in single items when practical.
- **Design for access patterns** — RDBMS can answer arbitrary queries via SQL; DynamoDB requires pre-defined patterns via careful key design.
- **Plan for data duplication** — The same logical entity may appear in multiple items/GSIs to support different queries.
- **Handle transactions carefully** — RDBMS transactions span tables; DynamoDB transactions are limited to single table and 25 items. Implement application-level coordination for complex workflows.

Lift-and-shift migrations often fail. Successful migrations require re-architecting schema based on application access patterns.

## Query Performance Tuning

- Use consistent reads only when necessary (stronger consistency costs double the capacity).
- Filter after querying, not before (filters are applied after items are returned, wasting capacity).
- Leverage sparse indexes to reduce data transfer.
- Monitor consumed capacity vs. provisioned capacity; scale up if approaching limits.
- Use batch operations (`BatchGetItem`, `BatchWriteItem`) to amortize overhead.

## See Also

Related: `database-patterns`, `database-internals-query`, `architecture-event-sourcing` (for using streams)