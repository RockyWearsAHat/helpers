# AWS Database Services

## RDS (Relational Database Service)

Managed relational databases: MySQL, PostgreSQL, MariaDB, Oracle, SQL Server.

### Multi-AZ Deployments

**Multi-AZ Instance (classic)**:

- Synchronous replication to standby in different AZ
- Automatic failover (60-120 seconds) on primary failure
- Standby is NOT readable — failover only
- DNS endpoint doesn't change on failover

**Multi-AZ Cluster (newer)**:

- One writer + two readable standbys across 3 AZs
- Semi-synchronous replication (transaction committed when at least 1 reader confirms)
- Failover under 35 seconds
- Reader endpoint for read scaling
- Supports MySQL 8.0 and PostgreSQL 13+

### Read Replicas

- Asynchronous replication (eventually consistent)
- Up to 15 read replicas (5 for Oracle/SQL Server)
- Can be cross-region
- Can be promoted to standalone database (breaks replication)
- Same or different instance class as primary
- Read replicas can have their own read replicas (adds latency)

Cross-region read replica: Used for disaster recovery, migration to another region, or reducing read latency for global users. Data transfer charges apply.

### Performance Insights

Built-in database performance monitoring:

- **DB Load**: Active sessions broken down by wait events (CPU, I/O, lock waits)
- **Top SQL**: Identify slow or resource-heavy queries
- **Top waits**: Understand bottlenecks (io/data_reads, lock/relation, CPU)
- 7-day retention free, up to 2 years paid
- Accessible via console, API, CloudWatch

Key wait events to monitor:

- `io/data_reads`: Storage I/O bottleneck → increase IOPS or switch to io2
- `lock/relation`: Table-level locks → review application locking strategy
- `CPU`: Compute bound → right-size instance or optimize queries
- `synch/mutex`: Internal engine contention → check engine-specific tuning

### RDS Proxy

Fully managed database proxy for connection pooling:

- Pools and shares database connections
- Reduces failover time by 66% (maintains connections during failover)
- IAM authentication support
- MySQL and PostgreSQL
- Critical for Lambda → RDS (Lambda can exhaust connection limits)
- Enforces TLS, integrates with Secrets Manager for credentials
- No application code changes (just change endpoint)

### Blue/Green Deployments

Zero-downtime database upgrades and schema changes:

1. Creates green (staging) copy from blue (production)
2. Green replicates from blue continuously
3. Make changes on green (engine upgrade, parameter changes, schema changes)
4. Switchover: Promotes green to production, redirects DNS
5. Switchover time: Usually under 1 minute
6. Rollback: Switch back to blue if issues

Supported changes: Major version upgrades, parameter group changes, schema modifications, instance class changes.

## Aurora

### Storage Architecture

Fundamentally different from standard RDS:

- **6 copies of data across 3 AZs** (2 copies per AZ)
- Handles loss of 2 copies for writes, 3 copies for reads without downtime
- Storage auto-scales in 10 GB increments up to 128 TB
- Write operations: Only redo log records sent to storage layer (not full pages)
- 4/6 write quorum, 3/6 read quorum
- Continuous backup to S3 (no performance impact)
- Point-in-time recovery to any second within retention period (up to 35 days)

Performance vs standard RDS:

- Up to 5x MySQL throughput, 3x PostgreSQL throughput (AWS claims)
- Real-world: 2-3x improvement for write-heavy, less for read-heavy
- Sub-10ms replica lag (vs 100ms+ for standard RDS read replicas)

### Aurora Replicas

- Up to 15 read replicas
- Same storage layer as primary (no replication lag for reads of committed data)
- Automatic failover (promoted to writer): Priority tiers 0-15
- Reader endpoint load balances across all replicas
- Custom endpoints for routing specific queries to specific replicas

### Aurora Serverless v2

Scales compute capacity automatically:

- Scales in 0.5 ACU increments (1 ACU ≈ 2 GB RAM + proportional CPU)
- Min 0.5 ACU, Max 256 ACU per instance
- Scales in seconds, not minutes
- Can mix serverless and provisioned instances in same cluster
- Costs more per ACU-hour than provisioned — break-even at ~40-50% utilization

When to use:

- Variable/unpredictable workloads
- Dev/test environments
- Multi-tenant with spiky per-tenant load
- NOT for sustained high-throughput (provisioned is cheaper)

### Global Databases

Cross-region replication with < 1 second replication lag:

- 1 primary region (read/write) + up to 5 secondary regions (read-only)
- Storage-based replication (not binlog) — minimal primary impact
- Promote secondary to primary for disaster recovery (RPO < 1 sec, RTO < 1 min with managed failover)
- Up to 16 read replicas per secondary region
- Write forwarding: Secondary region can forward writes to primary (adds latency but simplifies app)

## ElastiCache

In-memory caching with Redis or Memcached:

### Redis vs Memcached

| Feature         | Redis                                              | Memcached                  |
| --------------- | -------------------------------------------------- | -------------------------- |
| Data structures | Strings, hashes, lists, sets, sorted sets, streams | Strings only               |
| Persistence     | RDB snapshots + AOF                                | None                       |
| Replication     | Yes (read replicas)                                | No                         |
| Clustering      | Cluster mode (sharding)                            | Auto-discovery, multi-node |
| Failover        | Multi-AZ with auto-failover                        | No automatic failover      |
| Pub/Sub         | Yes                                                | No                         |
| Lua scripting   | Yes                                                | No                         |
| Transactions    | MULTI/EXEC                                         | No                         |
| Max memory      | Up to 500+ GB per node                             | Up to 300+ GB per node     |

Choose Redis unless you specifically need Memcached's multi-threaded architecture for simple key-value caching at extreme throughput.

### ElastiCache Redis Patterns

**Lazy loading (cache-aside)**:

```
1. App checks cache → miss
2. App reads from DB
3. App writes to cache
4. Next request → cache hit
```

Pro: Only caches what's needed. Con: Cache miss = 3 round trips. Stale data until TTL expires.

**Write-through**:

```
1. App writes to DB
2. App writes to cache
3. Reads always hit cache
```

Pro: Data always fresh. Con: Write penalty, caches unused data.

**TTL strategy**: Combine both — write-through for consistency, TTL for eviction. Short TTL (seconds-minutes) for volatile data, longer (hours) for stable data.

### Cluster Mode

- Partition data across up to 500 shards
- Each shard: 1 primary + up to 5 replicas
- Automatic slot redistribution on scaling
- Online resharding without downtime
- Cross-AZ replication within each shard

## MemoryDB for Redis

Redis-compatible, durable in-memory database:

- Multi-AZ durability with transaction log
- Microsecond read latency, single-digit millisecond write latency
- Can replace Redis + database pattern with single service
- Supports Redis data structures and APIs
- Data persists across restarts (unlike ElastiCache Redis without persistence)

Use case: When you need Redis as a primary database (not just cache), durable session store, leaderboards, real-time analytics.

## Neptune

Managed graph database:

- Supports Gremlin (property graph) and SPARQL (RDF)
- openCypher support
- Up to 15 read replicas across 3 AZs
- Storage auto-scales up to 128 TB (Aurora-like architecture)
- Full-text search integration with OpenSearch
- Neptune Serverless: Automatic capacity scaling
- Neptune Analytics: Graph analytics on large datasets
- Neptune ML: Graph neural networks for predictions

Use cases: Social networks, knowledge graphs, fraud detection, recommendation engines, identity graphs.

Query example (Gremlin):

```groovy
// Find friends-of-friends who like the same movies
g.V().has('person', 'name', 'Alice')
  .out('knows').out('knows')
  .where(neq('alice'))
  .out('likes').has('type', 'movie')
  .dedup()
  .values('title')
```

## Timestream

Serverless time-series database:

- Automatic tiering: In-memory store (recent/hot) → magnetic store (historical/cold)
- Built-in time-series functions: interpolation, smoothing, approximation
- SQL-compatible query interface with time-series extensions
- Ingestion: millions of data points per second
- Retention: Separate policies for memory and magnetic stores
- Scheduled queries for continuous aggregation
- Integrations: Grafana, QuickSight, SageMaker

Query example:

```sql
-- Average CPU over 5-minute windows for last hour
SELECT BIN(time, 5m) AS binned_time,
       AVG(measure_value::double) AS avg_cpu,
       hostname
FROM "metrics"."cpu"
WHERE time > ago(1h)
  AND measure_name = 'cpu_usage'
GROUP BY hostname, BIN(time, 5m)
ORDER BY binned_time DESC
```

Use cases: IoT sensor data, DevOps metrics, application telemetry, financial tick data. Don't use for: general-purpose OLTP, complex joins, non-time-series data.

## Database Selection Guide

| Requirement                                 | Service                         |
| ------------------------------------------- | ------------------------------- |
| Relational, standard SQL                    | RDS (MySQL/PostgreSQL)          |
| Relational, high availability + performance | Aurora                          |
| Key-value, microsecond latency              | DynamoDB                        |
| In-memory cache                             | ElastiCache Redis               |
| In-memory durable database                  | MemoryDB                        |
| Document store                              | DocumentDB (MongoDB-compatible) |
| Graph relationships                         | Neptune                         |
| Time-series data                            | Timestream                      |
| Full-text search                            | OpenSearch                      |
| Ledger/immutable                            | QLDB                            |
| Data warehouse                              | Redshift                        |
