# Apache Iceberg — Open Table Format for Distributed Data

## Overview

Apache Iceberg is an open table format designed for large-scale distributed analytics. It layers metadata management on top of object storage (S3, GCS, Azure Blob) to provide ACID transaction semantics, schema evolution, time-travel queries, and partition management—historically only available in proprietary data warehouses.

The format enables multiple compute engines (Spark, Flink, Presto, Trino) to read and write the same tables safely, without coordination overhead. Iceberg is built on the principle that object storage should replicate data warehouse capabilities: structure, versioning, and transactional safety.

---

## The Problem: Raw Object Storage Limitations

Traditional data lakes stored Parquet/ORC files directly on S3 without metadata:
- **No transactions**: Concurrent writes from Spark and other jobs created inconsistency; readers might see partially written files
- **No schema enforcement**: No mechanism to prevent schema drift or invalid mutations
- **No versioning**: Overwriting files lost history; recovery required external snapshots
- **No safe updates**: Fact corrections required full rewrites; bad deletes had no rollback
- **Poor partition management**: Partition pruning relied on directory naming; repartitioning was manual and expensive
- **Read consistency issues**: Listing object storage is not atomic; readers could miss files or read deleted files

Result: data lakes became unmaintainable, with silent data corruption and no audit trails.

---

## Iceberg's Architecture

### Metadata Tables

Iceberg stores metadata in three layers:

1. **Catalog**: The metastore entry point. Points to the latest table metadata. Implementations: AWS Glue, Hive Metastore, JDBC, REST
2. **Manifest List**: A file listing all manifest files for a specific table snapshot; includes partition statistics
3. **Manifests**: Files listing actual data files in the table, with partition values, record counts, column stats
4. **Data Files**: Parquet/ORC/Arrow files containing actual data

A **snapshot** is an immutable view of the table at a point in time, identified by a snapshot ID. Snapshots are linked in a timeline; new writes commit new snapshots while keeping old ones available for time-travel queries.

### Partition and Data Structure

Iceberg stores data as **data files** (Parquet, ORC) partitioned by user-defined columns. Unlike Hive, partitions are logical; Iceberg tracks which data files belong to which partition value, reducing the number of files the query engine must scan.

**Hidden partitioning** is a key innovation: users don't arrange data into directory hierarchies. Instead, Iceberg manages file organization based on partition columns. This decouples logical partitions from physical file layout, enabling efficient repartitioning without rewriting data.

---

## Schema Evolution

Iceberg supports safe schema changes:
- **Add columns**: New columns added with default values; read-time backfill for old files
- **Rename columns**: Column IDs remain stable; only metadata updates
- **Drop columns**: Logical deletion; data still present on disk but unmarked
- **Reorder columns**: Pure metadata change; no data touched

The key insight: Iceberg assigns immutable **column IDs** at creation. As the schema evolves, column IDs remain the same while names/types change. This enables safe evolution: old files with old schemas are still readable by new code because IDs map correctly.

---

## Partition Evolution

Traditional partitioned tables (Hive) are brittle: changing the partition schema requires full rewrites. Iceberg enables **partition evolution**: repartition data without stopping reads or writes.

Example: A table initially partitioned by `(year, month)` can be evolved to partition by `(day)` without rewriting existing data. Queries use the old partition scheme for old data and new scheme for new data. The query engine handles both transparently.

---

## Time Travel and Versioning

Every write to an Iceberg table creates a new snapshot. Snapshots are immutable and retain their data files, enabling queries "as of" any previous snapshot:

```sql
SELECT * FROM table VERSION AS OF 12345
SELECT * FROM table TIMESTAMP AS OF '2025-03-01 10:00:00'
SELECT * FROM table AT VERSION 12345
```

This enables:
- **Data debugging**: Inspect tables at any point in time
- **Recompute at old state**: Rerun BI dashboards as if data hadn't changed
- **Regulatory compliance**: Retain data history for audits
- **Recovery**: Rollback accidental deletes

Old snapshots are retained by default; expiration policies can clean up snapshots older than N days to manage storage.

---

## Merge-on-Read vs Copy-on-Write (CoW)

Two strategies for implementing updates/deletes:

### Copy-on-Write
Writing modified records creates new data files. Because data files are immutable, the entire affected file is rewritten with modifications. This is write-heavy but read-fast (readers only see committed data files).

### Merge-on-Read
Writing creates new **delete files** (delta files) marking which records are deleted. Readers must merge data + delete files at runtime. This is write-light but read-heavy; queries must check delete files before returning results.

Tradeoff: CoW suits workloads with rare updates (data warehouse inserts-only). MoR suits workloads with frequent changes (operational databases). Iceberg supports both; Spark typically uses CoW.

---

## Catalog Integration

Iceberg requires a **catalog** to store table metadata (instance location, current snapshot). Implementations:

| Catalog | Backend | Use Case |
|---------|---------|----------|
| **AWS Glue** | AWS metadata service | AWS-native; integrates with Athena, EMR |
| **Hive Metastore** | JDBC database | Existing Hadoop/Presto environments |
| **Nessie** | Git-like versioning | Advanced time-travel, branching/tagging |
| **REST** | Custom server | Multi-cloud, custom implementations |
| **JDBC** | Any SQL database | Self-hosted, vendor-independent |

Catalog choice affects where metadata lives and how table discovery works. Most organizations use AWS Glue or Hive Metastore for maturity.

---

## Iceberg vs Competitors

### Iceberg vs Delta Lake
Both enable ACID on object storage. Delta uses JSON transaction logs; Iceberg uses hierarchical metadata. Iceberg's advantage: partition evolution, multi-engine support (Flink, Presto), better statistics. Delta's advantage: tighter Databricks integration, broader adoption in ML workflows.

### Iceberg vs Hudi
Apache Hudi focuses on incremental processing (insert/update/delete) with upsert primitives. Iceberg focuses on table management and query optimization. Hudi is better for operational workloads; Iceberg for analytics.

### Iceberg vs Hive
Hive is an older metastore + query engine. Iceberg is a format specification; query engines implement readers. Iceberg has superior performance (partition pruning, schema evolution, atomic writes). Hive is simpler but brittle at scale.

---

## Performance Characteristics

Iceberg optimizes query performance:

1. **Partition pruning via manifest**: Manifest files list data files with partition values; query engine skips entire files without scanning
2. **Column-level statistics**: Min/max values per column per file enable predicate pushdown at file level
3. **Bloom filters**: Optional per-column Bloom filters enable fast non-match elimination
4. **Sorted files**: Iceberg tracks file sort order, enabling efficient range scans

Result: Queries on large tables scan only relevant files, not all data.

---

## Consistency Model

Iceberg provides **optimistic concurrency**:

Writers perform read → modify → write atomically at snapshot level. If two writers both modify the table, the second write validates that its base snapshot is still consistent and applies changes without replay. Conflicts are rare (different rows modified) but possible; write failures are explicit, requiring application-level retry logic.

Readers see consistent snapshots; there's no dirty reads or partial writes. The tradeoff: write conflicts require retry vs. pessimistic locking at the cost of write throughput.

---

## Adoption and Ecosystem

Iceberg adoption is growing, particularly in cloud data warehouses:
- **AWS**: EMR supports Iceberg; used in data lakes with Athena/Redshift
- **Databricks**: Competitive with Delta Lake; plans to support both formats
- **Streaming platforms**: Flink, Kafka Connect with Iceberg sink for CDC pipelines
- **SQL engines**: Presto, Trino, DuckDB support Iceberg reads

Iceberg is most relevant for organizations migrating from Hive or building multi-engine analytics stacks. For small teams or single-engine Spark shops, Delta Lake maturity may be preferable.

---

## Practical Considerations

**Strengths**: Multi-engine compatibility, schema evolution, time-travel, strong consistency, efficient partitioning, mature spec.

**Weaknesses**: Complex format (metadata trees) requires careful catalog management; overhead for small tables; library ecosystems less mature than Spark/Pandas.

Iceberg is best for **large-scale multi-engine analytics** requiring reliable schema management. For small datasets or single-engine shops, simpler formats (Parquet) or Delta Lake may be sufficient.