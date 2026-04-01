# Azure Cosmos DB — Multi-Model Database, Consistency & Global Distribution

## Overview

**Azure Cosmos DB** is Microsoft's globally distributed, multi-model database service. Unlike single-model databases, Cosmos DB natively supports multiple data paradigms (document, graph, column-family, key-value) within a single account, enabling schema flexibility and complex workloads in a unified service.

## Multi-Model Architecture

Cosmos DB exposes multiple APIs for the same underlying storage:

### Document (SQL/Core API)

- JSON documents stored in collections
- SQL-like query language
- Schema-flexible (no pre-defined structure)
- Suitable for most application data (user profiles, orders, configs)

### MongoDB API

- Wire-protocol compatible with MongoDB 4.0+
- Reuse existing MongoDB drivers and code
- Collections, indexes, aggregation pipelines
- Enables easy migration from self-hosted MongoDB

### Cassandra API

- Apache Cassandra wire protocol
- Column-family data model
- Partition keys and clustering keys
- Time-series, wide-column workloads

### Gremlin (Graph API)

- Graph query language for relationship traversal
- Vertices and edges with properties
- Suitable for social networks, recommendation engines, knowledge graphs

### Table API

- REST API for key-value access
- Flat schema (PartitionKey + RowKey)
- Drop-in replacement for Azure Table Storage

All models operate on the same distributed core: same RU pricing, consistency levels, replication, and failover.

## Partitioning & Distribution

### Partition Keys (Critical Design Choice)

Every container must define a partition key:
- Cosmos DB distributes data across physical partitions based on key values
- All documents with the same partition key reside together
- Queries within a partition are local (single-partition queries)
- Queries across partitions are broadcast (multi-partition queries, higher latency)

**Good partition key**: distributes load evenly, isolates hot data, aligns with access patterns. Example: `userId` for user-centric applications.

**Bad partition key**: skewed distribution (e.g., `country` if 90% users in one country). Results in partition hot spots and throttling.

### Physical Partitions

- Cosmos DB manages physical partitions internally (users don't allocate them)
- Each physical partition stores ~100 GB
- Logical partitions map to physical partitions as data grows
- As document count rises, logical partitions may split across multiple physical partitions

### Replication & Global Distribution

- Every collection replicated across all enabled regions
- **Strong consistency**: write region updates before replication to read regions (higher latency)
- **Eventual consistency**: written to write region, asynchronously replicated (lower latency)
- Multi-master replication: write to any region (addresses partition tolerance)
- Automatic failover to read replicas if write region unavailable

## Consistency Levels

Cosmos DB offers **five** well-defined consistency levels (beyond NoSQL's typical "eventual"):

1. **Strong**: Write + all read replicas synchronized before request returns. Globally consistent but highest latency.

2. **Bounded Staleness**: Replicas fall behind < N updates or < T seconds. Balances consistency and latency for most applications.

3. **Session**: Consistency within a client session (monotonic reads, consistent prefix writes). Default. Sufficient for single-user sessions.

4. **Consistent Prefix**: Writes seen in order globally, but stale reads possible. Respects causality.

5. **Eventual**: Reads may see stale data. Lowest latency, highest throughput.

**Trade-off**: strong consistency requires coordination; eventual consistency maximizes availability. Session/Bounded Staleness suit most interactive apps.

## Throughput & Pricing Model

### Request Units (RUs)

All operations billed in **Request Units**:
- 1 RU ≈ 1 KB document create
- 1 RU ≈ 4 KB document read
- 1 RU ≈ cost to execute 1 JS UDF
- Can be fractional: 0.5 RU for small operations

Provisioned throughput is guaranteed; excess consumption throttles requests.

### Provisioned vs. Serverless

**Provisioned Throughput**:
- Reserve RU/s in advance (e.g., 1,000 RU/s)
- Pay for reserved capacity regardless of usage
- Suitable for predictable, sustained workloads
- Scales from 400 RU/s to millions

**Serverless**:
- Pay per operation (RUconsumed)
- No capacity reservation
- Auto-scales from zero
- Suitable for unpredictable or bursty workloads
- Higher price per RU but no waste for sparse usage

### Throughput Sharing & Autoscale

**Autoscale**:
- Automatically scales between min/max RU/s
- Reacts to load patterns
- Cost savings if usage fluctuates

**Shared Throughput** (database-level):
- Multiple containers share an RU/s allocation
- Useful for many small containers
- Prevents one hot container from blocking others

## Change Feed

Event stream of all mutations (creates, updates, deletes) in a container:

- Ordered by partition (not globally)
- Change Streams API exposes as a cursor
- Consumers can subscribe and process changes
- Enables event sourcing, caching invalidation, downstream synchronization
- Change feed processor library handles checkpoints and retries

Example: Product catalog updates trigger cache invalidation and search index updates.

## Advanced Features

### Analytical Store

Separate columnstore optimized for analytics (OLAP):
- Decouples transactional (OLTP) and analytical workloads
- Synapse Spark pool or Power BI can query analytically without impacting RU consumption
- Time-to-live policies control retention

### Managed Identities

Pods/functions authenticate to Cosmos DB via managed identity (no connection strings). Works with Kubernetes Workload Identity.

### Backup & Point-in-Time Recovery

- Automatic backups to geo-redundant storage
- Continuous backup mode: restore to any point in last 30 days
- Periodic backup mode: retention windows

### Compliance & Encryption

- Encryption at rest (customer-managed or service-managed keys)
- Encryption in transit (TLS)
- Compliance certifications available (SOC 2, ISO, HIPAA)

## Common Pitfalls

- **Hot partitions**: uneven key distribution causes throttling on one partition
- **Cross-partition queries**: latency hidden from metrics; measure latency explicitly
- **Over-provisioning**: fixed RU/s costs money even if unused
- **Weak partition keys**: avoid timestamps (all new data concentrated); avoid low-cardinality fields
- **Unbounded result sets**: pagination required; cursors remain valid only for limited time

## Related

See also: [database-partitioning.md](database-partitioning.md), [database-consistency-models.md](database-consistency-models.md), [cloud-azure-data.md](cloud-azure-data.md), [architecture-event-sourcing.md](architecture-event-sourcing.md)