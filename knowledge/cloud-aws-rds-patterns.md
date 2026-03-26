# AWS RDS Patterns — Multi-AZ, Read Replicas, Aurora, and Operational Excellence

## Overview

Amazon RDS (Relational Database Service) abstracts database management—patching, backups, failover—while leaving control of the database engine. RDS patterns cluster around **availability** (Multi-AZ, read replicas), **scalability** (Aurora's storage-compute separation, connection pooling), **operational visibility** (Performance Insights), and **migration** (Database Migration Service).

## Multi-AZ Deployments

**Multi-AZ** provisions a primary database instance in one availability zone and a **synchronous standby replica** in a different AZ. Data committed to primary is written to standby before acknowledgment. On primary failure (hardware, network, patching), AWS initiates **automatic failover** (typically 1-2 minutes) to promote the standby to primary.

### Key Properties

- **RPO (Recovery Point Objective)**: near-zero; standby reflects primary state
- **RTO (Recovery Time Objective)**: 1-2 minutes; DNS update required for connection rerouting
- **Read traffic**: only primary accepts reads; standby is warm standby only
- **Cost**: roughly 2x single-instance cost (runs two instances)
- **Engine coverage**: MySQL, PostgreSQL, MariaDB, Oracle, SQL Server

### Failover Mechanics

When primary fails, AWS detects loss of heartbeat (20-60 seconds), promotes standby, updates Route 53 configuration, and restarts writes. Applications must implement **retry logic** with exponential backoff (connection resets redirect to new primary). Some frameworks (e.g., Java JDBC with Aurora) have built-in retry handlers.

Failover does **not** trigger if application makes a bad query or slow query; only infrastructure failures trigger automatic promotion.

## Read Replicas

**Read replicas** are asynchronous copies of a database instance. Writes go to the primary; replicas pull changes via **binary log replication** (MySQL/MariaDB) or **logical replication** (PostgreSQL). Read traffic can route to replicas to distribute query load.

### Replica Topology

- **Same-region replicas**: low latency, high throughput; typically for read scaling
- **Cross-region replicas**: high latency, isolated backups, disaster recovery; also enable **read locality** (serve local users from geographically closer database)
- **Cascading replicas**: replica-of-replica (one primary → replica A → replica B); adds latency but reduces primary replication load

### Replica Promotion

Replicas can be promoted to standalone primary instances (breaking replication). Promotion is manual and takes ~1 minute. Use cases include **blue-green deployments** (replica as new primary) or **sharding on demand** (shard off a replica).

### Read Replica Lag

Replicas lag behind primary; write-heavy workloads (10k+ commits/sec) can incur 100ms+ lag. Applications reading time-sensitive data (e.g., just-written payment status) should query the primary or implement **read-after-write consistency** patterns (write to primary, read from primary for short window, then route to replica).

## Aurora Architecture

**Aurora** decouples compute from storage. Traditional RDS couples them: EC2 instance generates pages, writes to EBS volume. Aurora separates: compute cluster (read/write instance, optional read replicas) uses a **shared distributed storage layer** (Aurora Storage) that replicates data across 3 AZs.

### Storage Layer

- **Copy-on-write**: committed pages are immutable; new writes create new page versions
- **Quorum-based writes**: writes acknowledged when 2 of 3 replicas in shared storage confirm (write quorum = 2, read quorum = 2, total replicas = 3; overlap ensures consistency)
- **Automatic repair**: if storage node fails, Aurora detects and rebuilds from quorum

### Compute Flexibility

- **Primary reader-writer**: reads and writes
- **Read replicas**: up to 15; all read from same shared storage (no replication lag, though internal consistency may differ)
- **Auto failover**: if primary fails, fastest replica promoted; ~30 seconds typical

### Advantages Over Traditional RDS

- **No storage replication overhead on compute**: compute freed from log shipping
- **Fast failover**: other replicas already have data
- **Instant read replicas**: no data copy; replica points to existing storage
- **Storage autoscaling**: grows automatically; no pre-allocation

### Disadvantages

- **Vendor lock-in**: Aurora MySQL and Aurora PostgreSQL diverge from vanilla engines (proprietary page format, replication model)
- **Shared storage cost**: paid per GB stored + I/O requests; workloads with many small writes (logging) can be expensive
- **Limited engine versions**: fewer minor versions supported than self-managed

## Aurora Serverless v2

**Aurora Serverless v2** replaces capacity-based provisioning (choose instance size like `db.r6g.xlarge`) with **automatic scaling based on demand**. Compute scales up/down in fine-grained increments (Aurora Capacity Units, ACUs); billing per ACU-second.

### Behavior

- **Burst scaling**: spikes in CPU/memory trigger rapid scaling; new reads directed to provisioned instances, existing queries continue on scaled instance
- **Minimum/maximum bounds**: set `MinCapacity` and `MaxCapacity` in ACUs; cluster scales within bounds
- **Sub-second response**: scaling typically takes 1-2 seconds; not instantaneous
- **Warm standby**: failover to replica still automatic

### Tradeoffs

- **Simpler capacity planning**: no guessing instance size
- **Cost variability**: predictable for uniform workloads, unpredictable for bursty ones; can exceed provisioned instance cost if sustained high demand
- **Startup latency**: cold starts (serverless suspended below minimum) require 30-60 seconds to warm
- **Compatible engines**: Aurora MySQL, Aurora PostgreSQL

## RDS Proxy

**RDS Proxy** pools database connections between application and database, multiplexing many app connections into fewer database connections. Useful when applications create short-lived connections (e.g., Lambda functions, web containers).

### Connection Pooling Strategies

- **Session mode**: RDS Proxy maintains connection to database for duration of client session; low overhead but less multiplexing
- **Transaction mode**: RDS Proxy reuses database connection across multiple client transactions; higher multiplexing but requires clients not to hold connections between transactions
- **Statement mode**: RDS Proxy reuses connection within single statement; maximum multiplexing, but application must not use prepared statements across statements

### Benefits

- **Reduced database connection count**: 1000 app connections → 100 database connections
- **Faster failover**: proxy manages failover; applications reconnect to proxy, not directly to database
- **IAM authentication**: proxy supports IAM database authentication without applications handling credentials

### Overhead

RDS Proxy adds ~5ms latency per query for connection management. Worthwhile when connection count is bottleneck or when applications spawn many short-lived connections.

## Performance Insights

**Performance Insights** is a diagnostics tool showing database load (active sessions, wait events, top SQL queries) over time. Visualizes **active sessions** (sessions executing queries) across CPU, lock waits, I/O, and other wait classes.

### Load Chart

- Y-axis: active session count (1 session = 1 CPU core worth of work)
- X-axis: time
- Colored regions: wait event types

If active sessions exceed CPU count, queries are queued or waiting. Clicking on a wait event shows top queries and tables involved.

### Use Cases

- **Identify slow queries**: top SQL shows queries consuming most active session time
- **Diagnose contention**: lock waits indicate table locking or row locks
- **Spot capacity issues**: active sessions approaching core count signals scaling need

## Encryption

### At-Rest Encryption

RDS uses **AWS KMS (Key Management Service)** to encrypt storage. Keys can be AWS-managed (default, no cost) or customer-managed (pay ~$1/month per key). Encryption is transparent; data encrypted at storage layer, decrypted in memory. Performance impact: roughly 0-5% depending on workload I/O patterns.

### In-Transit Encryption

RDS supports **SSL/TLS** encryption between application and database. Certificate validation prevents man-in-the-middle attacks. Some engines (RDS Proxy) enforce SSL/TLS.

### Authentication

- **Database user password**: traditional username/password stored in database
- **IAM database authentication**: temporary credentials (tokens valid 15 minutes) issued via IAM; avoids storing passwords; works with RDS Proxy

## Database Migration Service (DMS)

**AWS DMS** migrates data from on-prem or other cloud databases to RDS. Creates a **replication instance** (an EC2-like VM running DMS software) that reads from source database, applies transformation rules, and writes to target RDS.

### Full Load + CDC Pattern

1. **Full load**: replication instance reads entire source table, writes to target
2. **Change data capture (CDC)**: replication instance tails source transaction logs (MySQL binlog, PostgreSQL WAL), applies changes to target
3. **Cutover**: once target is caught up, application switches to target

### Transformation

DMS supports column mapping, filtering, and type conversion. Examples: rename columns, exclude PII, convert Oracle CLOB to PostgreSQL TEXT.

### Validation

DMS can run **data validation** comparing source and target row counts, checksums, and sample rows during or after migration.

## Operational Patterns

### Read Scaling

Route increasing read queries to read replicas or Aurora read replicas. Requires application logic to distinguish read vs. write queries (or use query routing proxy like ProxySQL). Aurora read replicas add complexity (no replication lag to manage) but couples you to Aurora.

### Write Scaling

RDS does not scale writes horizontally (all writes go to primary). Options:

- **Vertical scaling**: larger instance (more CPU, more concurrent writers)
- **Sharding**: split data across multiple RDS instances by shard key; application routes queries to correct shard
- **Switching to DynamoDB or other data stores**: if write volume demands distributed writes

### Disaster Recovery

Multi-AZ provides **high availability** (quick failover). For geographic DR (multi-region):

- **Read replica with promotion**: cross-region read replica can be promoted to primary; replication lag means potential data loss
- **RDS Automated Backups + restore**: backup snapshots shared across regions; restore in new region on disaster

### Version Upgrades

RDS supports **major version upgrades** (MySQL 5.7 → 8.0) and **minor version upgrades** (security patches, bug fixes). Upgrades can occur during maintenance window or immediately. Multi-AZ deployments upgrade in coordinated fashion: standby first, then primary (involves downtime).

## Cost Considerations

- **On-demand instances**: hourly charge per instance size
- **Aurora storage**: charged per GB stored + per 1M I/O requests
- **Data transfer**: outbound to internet, cross-region replication incur charges; within AZ is free
- **Backups**: automated backups free up to 35 days; manual snapshots charged per GB
- **Burst credits**: some instance types accumulate burst credits; credit exhaustion throttles performance

See also: [database-replication-patterns](database-replication-patterns.md), [cloud-disaster-recovery](cloud-disaster-recovery.md), [cloud-aws-databases](cloud-aws-databases.md)