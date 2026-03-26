# Change Data Capture (CDC) — Patterns, Architectures & Trade-offs

## Overview

**Change Data Capture (CDC)** is the pattern of capturing and propagating modifications to data—inserts, updates, deletes—from a source system in real time or near-real time. CDC decouples source systems from consumers: rather than tight coupling between applications or ETL jobs that poll for changes, consumers subscribe to a stream of changes and respond independently.

CDC powers event-driven architectures, data replication, data warehousing, caching invalidation, audit trails, and analytics pipelines. The core tension: **accuracy vs. performance vs. operational overhead**.

## CDC Mechanisms

### Log-Based CDC

The source database writes all changes to a transaction log (e.g., MySQL binlog, PostgreSQL WAL, SQL Server transaction log). A CDC log reader tails this log, parses committed transactions, and emits change events.

**Advantages:**
- Non-invasive; no application changes required
- Captures all writes (even from legacy systems, direct SQL)
- Minimal source system performance impact
- Transactional consistency (can replay exact state at any point)
- Works with read replicas (no reader performance penalty on primary)

**Disadvantages:**
- Log format is database-specific; each database needs a connector (Debezium, Maxwell, Kafka Connect)
- Logical vs. physical logs create parsing complexity (some databases expose only physical logs)
- Lagging replica risk: log reader falls behind writes during high volume
- Schema changes in logs may not be fully described; context is implicit
- Operational complexity: managing log retention, offset tracking, failover coordination

**When to use:** High-velocity sources (millions of events/sec), strict ordering requirements, zero-downtime requirement.

### Trigger-Based CDC

The database executes a trigger on INSERT, UPDATE, DELETE. The trigger writes change metadata to a CDC table. A polling process (or daemon) reads this table and emits events.

**Advantages:**
- Works on any database (even older systems)
- Database-agnostic reader logic
- Simple to implement (just SQL)
- Can enrich changes with application context in the trigger

**Disadvantages:**
- Adds latency and overhead to every write (trigger execution cost)
- Missed changes if polling falls behind (gaps in event stream)
- Hard to achieve exactly-once semantics (offset tracking is manual)
- CDC table grows unbounded without cleanup
- Triggers on all tables = operational noise; selective triggering = manual configuration overhead
- No transactional atomicity guarantee between main table and CDC table in older databases

**When to use:** Legacy databases without log access, low-frequency changes, when application has direct schema control.

### Timestamp/Version-Based CDC

Applications record a **last-modified timestamp** (or version number) on each row. A poller queries for rows where `modified_at > last_checkpoint`, emits them, and advances the checkpoint.

**Advantages:**
- Simple, requires only application field discipline
- No special database features needed
- Works with any query interface (REST API, JDBC, GraphQL)
- Easy to implement client-side

**Disadvantages:**
- Clock skew creates missed or double-counted changes (rows backfilled after checkpoint, clock rewind)
- No deletion semantics (deletes disappear)
- Polling lag = change lag (not true streaming)
- Doesn't scale: scanning all rows with modified_at > T becomes expensive at high volume
- No ordering guarantee across shards (distributed systems issue)
- Two-phase problem: row may be visible between polling cycles but not yet indexed

**When to use:** Low-frequency snapshots, append-only systems, when latency tolerance is high (hours/days).

### Query-Based CDC (Snapshot)

Full table snapshot at interval: query entire table, diff against previous snapshot, emit changes.

**Advantages:**
- Works on any source (even APIs)
- No database changes required

**Disadvantages:**
- Expensive (full table scan each run)
- Deletion semantics depend on diff algorithm
- High latency (changes batched until next snapshot interval)
- Doesn't scale to large tables

**When to use:** Very low-frequency data, small tables, or as fallback when no better option exists.

## Event Structure & Semantics

CDC events typically contain:

```
{
  "op": "c|u|d",                    // create, update, delete
  "ts_ms": 1234567890,              // transaction timestamp
  "table": "schema.table_name",
  "before": {...},                   // previous row state (update/delete)
  "after": {...},                    // new row state (create/update)
  "source": {
    "db": "postgres",
    "txId": 12345,
    "lsn": 67890,
    "snapshot": false
  }
}
```

**Exactly-once semantics** is the goal but hard to achieve. Log-based CDC with broker offset management (Kafka, Pulsar) approximates it. Trigger-based and timestamp-based CDC are usually **at-least-once** (duplicates possible on retry).

## Schema Change Handling

A critical challenge: **schema evolves**. A column is added, renamed, or dropped. How does CDC respond?

### Approaches

1. **Snapshot-resume**: Detect schema change, pause CDC, take full snapshot with new schema, resume. Downtime for consumers.

2. **Evolve schema version in events**: Include schema version or full schema in each event. Consumers handle versioned deserialization. Adds event size; complexity in consumer code.

3. **Schema registry**: Debezium + Confluent Schema Registry (or Protobuf/Avro registry): schema tagged by ID, published separately, consumers fetch schema by ID. Cleaner but adds registry dependency.

4. **Outbox pattern supplement**: For structural changes, emit explicit "schema change" events to outbox table. Consumers can process and adapt.

5. **Database history topic** (Debezium): All DDL statements published to a Kafka topic. Consumers can process DDL and adapt incremental schema.

**Trade-offs:** Schema registry centralization vs. event self-description; latency (instant propagation vs. batch schema publish); consumer complexity (handle multiple schema versions in-flight).

## Outbox Pattern Integration

The **outbox pattern** pairs CDC with a local transactional outbox table:

```sql
BEGIN;
  UPDATE orders SET status = 'shipped' WHERE id = 123;
  INSERT INTO outbox (event_type, aggregate_id, payload, created_at)
    VALUES ('OrderShipped', 123, '{"qty":5}', NOW());
COMMIT;
```

CDC reads from `outbox`, emits events to broker, replays this pattern across services.

**Advantages:**
- Guarantees: business operation and event emission are transactional (no lost notifications)
- Events are replayable (outbox is the source of truth; broker is cache)
- No dual-write problem (single transaction)

**Disadvantages:**
- Requires application changes (must write to outbox)
- Extra table = extra I/O
- Outbox polling/CDC needs operational care (backlog monitoring, cleanup)
- Still eventual consistency; ordering across outbox instances (shards) requires careful consumer grouping

**When to use:** Orders, payments, events requiring guaranteed delivery with application transaction boundaries.

## Event Sourcing vs. CDC

Event sourcing is an **event store**: all state changes are immutable events. The aggregate is reconstructed by replaying events. CDC is read from databases that don't store events—events are derived from state diffs.

| Aspect | Event Sourcing | CDC |
|--------|---|---|
| **State storage** | Event log is source of truth | Database tables are source of truth |
| **Audit trail** | Built-in (all events recorded) | Derived (changes inferred from snapshots/logs) |
| **Temporal queries** | Reconstruct any past state | Possible but not primary use case |
| **Operational model** | Write to event store, project to views | Write to database, CDC propagates |
| **Schema evolution** | Event upcasting; versioning | Schema migration + CDC evolution |
| **Use cases** | Audit, temporal analysis, complex domains | Data replication, caching, real-time warehousing |

**In practice:** Many systems use both. Event sourcing for write models (orders, transactions); CDC for read model replication and analytics.

## CDC Platforms & Tools

### Debezium (Log-Based)

Open-source, Kafka-centric CDC platform. Connectors for PostgreSQL (WAL), MySQL (binlog), MongoDB (oplog), SQL Server, Oracle, Cassandra, Vitess. 

- Exports changes as JSON/Avro messages to Kafka Topics
- Decimal precision, large numbers, temporal types all supported
- High-volume handling in production (millions of events/sec) at companies like Uber, LinkedIn
- Snapshot mode for initial bulk load, then incremental capture
- Schema registry integration (built-in Confluent/Apicurio support)

**Operational model:** Kafka Connect worker runs Debezium connector task. Connector maintains source offsets in Kafka; crash recovery resumes from checkpoint. Horizontal scaling by adding more workers.

**Limitations:** Only emits JSON/Avro (no native RPC); requires Kafka (or Kafka-compatible systems like AWS MSK, Confluent Cloud); connector-specific quirks (PostgreSQL replication slot management, MySQL GTID, Oracle LogMiner complexity).

### Kafka Connect Ecosystem

Kafka Connect is the integration framework; Debezium is one implementation. Also: Maxwell (MySQL → JSON), Mongo Connector for Kafka, AWS DMS, etc.

### Commercial/Cloud CDC

- **AWS DMS**: Replicates to RDS, Redshift, S3. Native CDC capture for many databases.
- **Fivetran**: SaaS CDC + data warehouse loading. Abstracts complexity; subscription cost.
- **Stitch** (Talend): SaaS CDC, similar model.

### Specialized CDC

- **MongoDB Ops Manager**: Native change stream API
- **PostgreSQL Logical Decoding**: Subscriptions (native to 10+)
- **Streaming SQL**: ksqlDB, Spark Structured Streaming can read from Kafka topics emitted by CDC

## Performance and Latency Considerations

**Latency sources:**

1. **Source database**: Write commits to WAL but CDC reader may lag (especially under high volume). Log rotation can cause reader backpressure.
2. **CDC process**: Parsing, filtering, batching, serialization.
3. **Broker**: Kafka broker commit latency (fsync if acks=all).
4. **Network**: Various RTT delays.

**Typical latencies:** Log-based CDC: second to tens of seconds in steady state (depending on batch sizes, producer config). Trigger-based: polling interval + trigger overhead = seconds to minutes. Timestamp: polling interval = seconds to minutes.

**Scaling constraints:**

- Single CDC process can handle millions of events per second from a single source, but at cost of resource consumption.
- Horizontal scaling: partition source by table, shard, or time range, run multiple CDC processes (Debezium workers, Spark tasks).
- Bottleneck often shifts to broker (Kafka brokers unable to absorb all inbound events at target throughput).

**Schema change storms:** If many source tables change schema simultaneously, CDC event generation may spike. Distribute schema migrations to avoid thundering herd.

## Storage Impact

CDC log readers create **read-only load on database replicas**. Under high volume, replica lag may increase, impacting applications using the replica for read scaling.

**Mitigation:**
- Run CDC reader against a dedicated read replica.
- Tune replica max_connections, shared_buffers, work_mem.
- Use log streaming replication (PostgreSQL) to reduce replica lag for CDC reader.
- Rate-limit CDC batches to prevent reader from consuming excessive resources.

## See Also

- [data-engineering-etl.md](data-engineering-etl.md) — ETL/ELT patterns and trade-offs
- [architecture-event-driven.md](architecture-event-driven.md) — Event-driven architecture patterns
- [architecture-event-sourcing.md](architecture-event-sourcing.md) — Event sourcing as alternative to CDC
- [data-engineering-governance.md](data-engineering-governance.md) — Data quality and lineage tracking
- [infrastructure-message-broker-patterns.md](infrastructure-message-broker-patterns.md) — Kafka, Pulsar, and broker architectures