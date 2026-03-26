# Data Lakehouse — Table Formats, ACID Semantics & Data Lake Evolution

## Overview

A **data lakehouse** is an architectural pattern that merges the storage efficiency and flexibility of data lakes with the transactional guarantees and query performance of data warehouses. The enabler is **open table formats** — metadata management systems layered on top of object storage (S3, GCS, Azure Blob) that add structure, versioning, and ACID properties to otherwise schema-free lake storage.

Traditional data lakes stored raw files (Parquet, ORC, CSV) but lacked concurrent update semantics, schema enforcement, and time-travel capability. Warehouses provided these but required expensive proprietary infrastructure. Lakehouses use open formats to deliver both at scale, on cheap object storage.

---

## The Evolution: Lake → Warehouse → Lakehouse

### Data Lake Limitations

Early data lakes were raw file repositories:
- **No schema enforcement**: arbitrary writes could corrupt data contracts
- **No transactions**: concurrent writes from multiple jobs created inconsistency; readers might see partial writes
- **No versioning**: overwriting data lost history; recovery required external snapshots
- **No updates**: fact corrections required full rewrites; schema changes broke downstream consumers
- **No time-travel**: queries froze to a point-in-time only via external snapshots
- Data became unmaintainable swamps without strong metadata governance

### Data Warehouse Approach

Warehouses (Redshift, BigQuery, Snowflake) solved this via:
- **ACID transactions**: serializable or snapshot-isolated mutation semantics
- **Schema evolution**: safe add/drop/rename column operations
- **Version control**: automatic retention of old table versions; time-travel queries via `AS OF` syntax
- **Partition pruning**: metadata-driven query optimization skips irrelevant data

But warehouses required:
- Expensive proprietary hardware, licensing, per-query costs
- Vendor lock-in and format proprietary to each system
- Data duplication: ETL pipelines extracted from lakes to warehouses for queryability
- Inflexible schemas and poor handling of semi-structured data (JSON, Avro)

### Lakehouse Promise

Lake table formats (Delta, Iceberg, Hudi, Paimon) provide warehouse semantics on lake storage:
1. **ACID transactions** on object storage using optimistic concurrency or MVCC
2. **Schema enforcement & evolution** without breaking readers
3. **Time-travel & data versioning** via immutable metadata manifests
4. **Partition & z-order pruning** using statistics encoded in metadata
5. **Open standards**: schemas, APIs, and data are not proprietary — any engine can read
6. **Cost**: object storage ≈ 10x cheaper than warehouse appliances; pay only for compute

---

## The Big Three Table Formats

### Delta Lake (Databricks)

**Architecture:**
- **Metadata storage**: transaction log (JSON files in `_delta_log/`) tracks all mutations
- **Concurrency model**: optimistic locking on metadata; readers see consistent snapshots via transaction log ordering
- **How it works**: every write appends a JSON manifest to the log; readers linearize log from last checkpoint to current version
- **Checkpoint strategy**: every 10 transactions (default), Delta writes a Parquet snapshot to accelerate log replay on large histories

**Key features:**
- **Unified batch + streaming**: same table readable by Spark SQL and Spark streaming
- **ACID forte**: excellent for frequent updates; enforces serializability
- **Schema evolution**: `mergeSchema` option allows schema-on-read
- **Time-travel**: `SELECT * FROM table VERSION AS OF 20` queries historical state
- **Z-ordering**: multidimensional data layout for fast range queries (like clustering)

**Trade-offs:**
- **Lock-free concurrency**: transaction log appends are atomic, but readers must replay entire log on large tables (can be slow for 1000+ transactions without checkpoints)
- **Spark-centric**: Delta was designed for Spark; support in other engines (Flink, Presto, DuckDB) arrived later and some features lag
- **File format coupling**: tightly integrated with Parquet; other columnar formats require workarounds

**Ecosystem:** Databricks (managed Delta Lake SaaS), DeltaRS (Rust implementation), multiple BigQuery/Redshift connectors

### Apache Iceberg (Netflix/Apache)

**Architecture:**
- **Metadata storage**: versioned metadata files (JSON, Avro) track snapshots of table state
- **Concurrency model**: MVCC (multi-version concurrency control); writes create new versions; readers pick a snapshot ID atomically via metadata pointer
- **How it works**: each write creates a new manifest, new manifest list, new metadata file; atomic swap via object storage pointer (e.g., rename `metadata.json.new` → `metadata.json`)
- **Snapshot isolation**: readers always see a consistent table state without replaying transaction logs

**Key features:**
- **Cloud-native design**: built for distributed object storage patterns (no coordination service needed; atomic renames are sufficient on S3/GCS)
- **Format-agnostic**: Parquet, ORC, or Arrow backing formats supported
- **Partition evolution**: change how data is partitioned without rewriting files (hidden partitioning)
- **Column-level statistics**: pruning via min/max/null counts at parquet page level
- **Branching & tagging**: create mutable branches of a table; tag stable versions (used for data governance)

**Trade-offs:**
- **Metadata complexity**: managing many snapshot versions can create metadata bloat; requires periodic cleanup
- **Write latency**: snapshot isolation requires atomic metadata writes; coordinating these at scale is harder than Delta's log appends
- **Spec maturity**: format specification is comprehensive but newer than Delta; ecosystem adoption slower (catching up fast)

**Ecosystem:** Netflix (original creator), Dremio, Trino, Spark 3.3+, DuckDB, Flink

### Apache Hudi (Uber)

**Architecture:**
- **Metadata storage**: timeline-based; each mutation marks a commit/instant in timeline
- **Concurrency model**: copy-on-write and merge-on-read strategies (configurable per write)
- **How it works**:
  - **Copy-on-write (CoW)**: all columns written; read-optimized Parquet files updated per commit; slower writes, instant reads
  - **Merge-on-read (MoR)**: updates stored in delta logs (Avro); reads merge delta log + base Parquet lazily; faster writes, slower reads

**Key features:**
- **Incremental queries**: `BEGIN > last_read_instant` syntax lets consumers query only new/changed records (ideal for CDC)
- **Timeline-based versioning**: every instant (commit, deltacommit, compaction) is versioned; fine-grained control
- **Compaction**: automatic or manual; merges delta log into Parquet base files
- **Index-based upsert**: index structures enable efficient point lookups for upsert (key-based deduplication)

**Trade-offs:**
- **Complexity**: CoW vs MoR choice and compaction strategies require tuning per workload
- **Split metadata model**: base files + delta logs + indexes; more moving parts to manage
- **Query engine support**: Spark support is mature; support in other engines (Presto, Trino) is less battle-tested

**Ecosystem:** Uber, AWS Glue, Spark, Hive

### Comparison Matrix

| Feature | Delta | Iceberg | Hudi |
|---------|-------|---------|------|
| **ACID Model** | Pessimistic log append | Optimistic snapshot isolation | Timeline-based CoW/MoR |
| **Concurrency** | Lock-free; log linearization | MVCC (read correct snapshot ID) | Instant-keyed versioning |
| **Schema Format** | Required: Parquet | Flexible (Parquet, ORC, Arrow) | Flexible |
| **Partition Evolution** | Via ALTER TABLE | Hidden partitions (native) | Via timeline |
| **Time-Travel** | Via transaction ID | Via snapshot ID | Via instant |
| **Incremental Reads** | Not optimized | Via snapshots | Optimized via delta logs |
| **Write Latency** | Low (append-only) | Medium (atomic rename) | Variable (CoW slow, MoR faster) |
| **Read Speed** | Medium (log replay) | Fast (snapshot isolation) | Variable (CoW fast, MoR lazy) |
| **Ecosystem Maturity** | Widest (Databricks investment) | Growing (Netflix backing) | Mature in Spark; others catching up |

---

## ACID Semantics on Object Storage

Object storage lacks traditional database transaction support (locks, write-ahead logs). Lakehouses simulate ACID using:

### Atomicity & Isolation

**Problem**: How to ensure a write (multiple files) is all-or-nothing and readers don't see partials?

**Solution**: All-or-nothing metadata manifest
- Data files (Parquet, Avro) are immutable; writes create new files
- A **metadata pointer** (manifest, transaction log entry, or metadata file) atomically references the new table state
- Readers read metadata first; if consistent, all files referenced exist and are immutable

Example:
```
Transaction: Upsert table (add 2 files, mark 1 obsolete)
Write phase:
  - Write file_1.parquet, file_2.parquet to S3 (uncoordinated)
Commit phase:
  - Atomically append/rename metadata.json referencing new state
  - Ensures atomicity at metadata level only
```

Object stores guarantee atomic single-key writes (S3 PUT, GCS update via version ID), so this pattern works.

### Consistency & Isolation

**Problem**: Concurrent writers might create conflicting metadata updates; readers might see stale state.

**Delta solution (optimistic)**: Serialize all writers through log append
- Every write appends a JSON line to `_delta_log/` (immutable append-only log)
- Readers replay log from last checkpoint to present; log order is serialization order
- Conflicts (two writes to same object) are detected; one aborts

**Iceberg solution (pessimistic snapshot isolation)**: Metadata versioning
- Each write creates new metadata file `v123.json` referencing immutable manifest files
- Atomic pointer (renamed to `metadata.json`) selects active version
- Readers pick snapshot ID; get snapshot's manifest list; query only referenced files
- No log replay needed; each snapshot is self-contained

### Durability & Recoverability

Metadata is versioned, enabling:
- **Recovery on failure**: if a write crashed mid-flight, next reader skips incomplete version
- **Audit trail**: all versions remain queryable; GDPR-compliant deletion requires version cleanup
- **Garbage collection**: orphaned files (unreferenced after N versions) are removable; metadata history can be compacted

---

## Schema Evolution

Schemas change without disrupting readers or requiring rewrites:

### Add/Drop/Reorder Columns

**Add**: new column gets default or null; files missing it (written before change) filled on read
**Drop**: column removed from schema; old files still have it but it's ignored
**Reorder**: affects read order and downstream code but not storage; manifests track canonical column order

### Type Evolution

**Promotion**: `int` → `long`, `float` → `double` (safe; larger type holds old values)
**Demotion** (Delta, Iceberg support): `string` → `int` (risky; old string data may not parse; disallowed by default)

### Schema Enforcement

Writes validate against current schema (Column type mismatch causes rejection). Readers can:
- **Schema-on-read**: ignore unknown columns; fill missing columns with nulls
- **Strict mode**: fail if file structure doesn't match schema

---

## Time-Travel & Temporal Queries

Every table version is queryable:

**Delta**:
```sql
SELECT * FROM table VERSION AS OF 5
SELECT * FROM table TIMESTAMP AS OF '2025-03-01'
```

**Iceberg**:
```sql
SELECT * FROM table SNAPSHOT AS OF 1234567890
SELECT * FROM table FOR TIMESTAMP AS OF '2025-03-01'
```

**Use cases**:
- Audit: reconstruct data state at specific point (compliance, debugging)
- Rollback: restore prior version without deep restores
- A/B testing: compare query results pre/post schema change
- Data quality: identify when a quality issue was introduced

---

## Partition Evolution

Traditional partitioning locks the table into `/year=2020/month=03/day=15/...` structure. Changing it requires rewriting all files.

Lakehouse formats support **hidden partitioning** (Iceberg) or evolution:
- Schema specifies partitioning logic (date bucketed by day, region bucketed by geography)
- Existing files keep old layout; new writes use new layout
- Queries transparently prune across both (partition column kept in metadata)
- Gradual migration: compact old files to new layout on compaction, not on writes

---

## Merge-on-Read vs. Copy-on-Write

Trade-off between write cost and read cost (most acute in Hudi):

**Copy-on-Write**:
- Every upsert rewrites affected Parquet files (expensive write, instant read)
- Best for append-heavy workloads or infrequent updates
- Example: daily batch updates to historical fact table

**Merge-on-Read**:
- Upserts append to delta log (fast write)
- Reads lazily merge delta log + base Parquet (slower read, but incremental reads are fast)
- Best for streaming ingest with many small updates; downstream readers can choose to read base or incl. deltas
- Example: real-time CDC feed into a table

---

## Ecosystem Considerations

- **Spark SQL**: All support; Delta tightest integration
- **DuckDB, Datafusion**: Strong Iceberg support; Delta/Hudi catching up
- **Presto/Trino**: All support; Iceberg most mature
- **Streaming**: Delta handles Spark Streaming natively; others require connectors (Flink, Kafka)
- **Governance**: Iceberg's branching/tagging gives stronger data governance primitives
- **Cost & Performance**: Benchmarks vary by workload; no universal winner currently (Delta & Hudi trade performance based on query pattern; Iceberg's MVCC adds latency but reads are consistent)

---

## See Also

- [Data Warehousing](data-engineering-warehousing.md) — dimensional modeling, OLAP
- [Data Serialization Formats](data-serialization-formats.md) — Parquet, ORC, Arrow details
- [ETL/ELT Patterns](data-engineering-etl.md) — pipeline design
- [Data Mesh](architecture-data-mesh.md) — federated data governance