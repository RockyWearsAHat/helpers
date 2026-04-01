# Time-Series Databases — Data Model, Storage Engines, and Retention

Time-series databases (TSDBs) are optimized for append-only workloads with bulk queries over time ranges. The core pattern: metric data (sensor readings, application metrics, market tickers, events) arrives as `(timestamp, tags, value)` tuples at high ingestion rates, then gets queried in aggregate or downsampled for long-term storage.

## Data Model Fundamentals

### Metric, Tags, and Value

Time-series records contain:
- **Timestamp** — the time axis, typically nanosecond or microsecond precision
- **Metric name** — the measurement being recorded (e.g., `cpu_usage`, `request_latency`)
- **Tags (dimensions)** — metadata that allows grouping: `{host: "server-01", region: "us-west", env: "prod"}`
- **Value** — the numeric measurement (less commonly, multiple fields: `{memory_used: 4096, memory_available: 8192}`)

The same metric with different tag combinations creates separate time series. A high-cardinality tag (one with many unique values) multiplies storage burden — 10,000 hosts × 50 metrics = 500,000 time series. Query performance degrades with cardinality; most TSDBs suffer performance cliffs when cardinality exceeds their design limits.

## Storage Architecture & Compression

### Columnar Storage and Delta Encoding

TSDBs overwhelmingly use **columnar storage** — values, timestamps, and tags are stored separately.

- **Timestamps**: Delta encoding replaces absolute times with differences. If values arrive every second, store `{epoch, +1, +1, +1, ...}` instead of full timestamps. Compresses to 1-2 bytes per value.
- **Values**: Similar ideas — many metrics exhibit trends (e.g., CPU usage), so storing `{100, +5, -2, +3, ...}` compresses better than raw values. Some systems apply **XOR-based compression** (storing bit differences for floating-point data).
- **Tags**: Stored separately, often deduplicated or indexed.

Compression ratios of 10-100x are common. ClickHouse achieves 10-100x on real-world time-series data; TimescaleDB with compression reaches 5-20x depending on the data.

### Chunk-Based Organization

Data is partitioned into time-based **chunks** (e.g., 1 hour, 1 day). Within a chunk:
- Recent (hot) chunks are memory-resident for fast writes
- Older chunks can be compressed, moved to cheaper storage, or deleted
- Queries that span multiple chunks scan only relevant chunks (partition pruning)

Example: TimescaleDB's hypertables divide data into 24 chunks of 1 hour each (by default). A query for yesterday's last 6 hours scans only 6 chunks.

## Retention and Downsampling

### Retention Policies

Automated retention removes old data to cap storage:
- Raw data: keep for 7-30 days (hot queries are usually recent)
- Downsampled data: keep indefinitely (historical trend analysis)
- TTL columns: some systems mark expires-at timestamps, TTL processes prune automatically

### Downsampling and Continuous Aggregates

**Downsampling** reduces resolution of old data: Replace 1,440 daily minute-granularity points with a single hourly aggregate (sum, avg, max, min, percentiles).

**Continuous Aggregates** (TimescaleDB terminology; similar in other systems as materialized views) pre-compute aggregates in the background:

```sql
CREATE MATERIALIZED VIEW hourly_cpu AS
SELECT
  time_bucket('1 hour', time) AS hour,
  host,
  avg(cpu_usage) AS avg_cpu,
  max(cpu_usage) AS max_cpu,
  percentile_cont(0.95) WITHIN GROUP (ORDER BY cpu_usage) AS p95_cpu
FROM raw_metrics
GROUP BY hour, host;
```

On insertion, raw data flows in; the database incrementally updates aggregates. Queries on old data hit the downsampled view (fast), queries on recent data hit raw data (accurate).

## Query Patterns and Operations

### Typical Workloads

1. **Range queries**: "CPU usage for host X over the last 24 hours" — sequential scan of time range, fast with columnar storage and partition pruning.
2. **Aggregates**: "Average latency across all servers in region Y, bucketed by 5-minute intervals" — GROUP BY time-bucket patterns.
3. **Multi-metric correlation**: "When memory spikes, does disk I/O follow?" — JOIN or correlation analysis.
4. **Anomaly detection**: Rolling windows with statistical functions (stddev, percentiles).

### Time-Bucket Functions

Most modern TSDBs provide time-bucketing operators:

```sql
-- InfluxDB
SELECT mean(value) FROM data WHERE time > now() - 24h GROUP BY time(1h), tag1
```

```sql
-- TimescaleDB / PostgreSQL
SELECT time_bucket('1 day', time) as day, avg(value) FROM data GROUP BY day
```

```sql
-- QuestDB (SAMPLE BY)
SELECT avg(value) FROM data SAMPLE BY 1h ALIGN TO CALENDAR
```

These avoid expensive window-function computation for uniform-width buckets.

## Storage Engine Comparison

### InfluxDB

- **Model**: InfluxDB 3.x is Parquet-based (columnar); earlier versions used custom WAL + in-memory index
- **Writes**: High ingestion throughput via buffering and batch flushing
- **Compression**: Parquet codec (typically Zstandard or gzip)
- **Query language**: InfluxQL (old, SQL-like) or Flux (new, functional)
- **Trade-off**: Simpler operations, but scaling beyond single node historically required clustering complexity

### TimescaleDB

- **Model**: PostgreSQL extension with hypertables (auto-partitioned regular tables)
- **Advantage**: Full SQL compatibility, ACID semantics, consistent operational model with PostgreSQL DBAs
- **Compression**: Optional per-chunk; recent versions add WAL compression
- **Continuous aggregates**: Materialized views backed by hypertables — automatic incremental updates
- **Trade-off**: Bounded single-node performance (PostgreSQL doesn't shard); excellent for sub-million-series scale

### QuestDB

- **Model**: Written in zero-GC Java and C++, designed for ultra-low-latency writes
- **Storage**: Column-oriented, with O(1) append semantics (data lands sequentially on disk)
- **Signature feature**: ASOF JOIN for correlating time series without exact timestamp matches
- **SQL extensions**: `SAMPLE BY`, `FILL`, `WHERE` with timestamp predicates optimized for partition pruning
- **Trade-off**: Optimized for write volume and latency, narrower ecosystem than PostgreSQL-based systems

### ClickHouse

- **Model**: Columnar, designed for analytical queries over massive datasets
- **Compression**: Pluggable codecs; LZ4/ZSTD standard; special encoding for time-series (delta + XOR)
- **Partitioning**: By key (e.g., date) with automatic pruning
- **Scaling**: Horizontal scaling via sharding + replication
- **Trade-off**: Slower per-row insertion than write-optimized TSDBs; faster aggregation queries at scale; steeper learning curve

## Use Cases and Limits

### When to Use a TSDB

- **Metrics collection**: Prometheus scrapes services every 15 seconds; InfluxDB/QuestDB handle the buffer
- **Financial data**: Tick data, OHLC candles, order book snapshots
- **Observability**: Metrics, traces (as time-series), logs (as events with timestamp)
- **IoT sensors**: Temperature, pressure, location data streaming in continuously

### When NOT to Use a TSDB

- **Sparse data**: Few readings per series per day → regular SQL works fine, TSDB optimizations add overhead
- **Complex OLTP transactions**: Distributed transactions across unrelated data → use distributed SQL, not TSDB
- **Unstructured text search**: Logs with keywords → use Elasticsearch or a search index, combine with TSDB for correlation

## Architectural Trade-offs

### Write Performance vs. Query Flexibility

- **InfluxDB 3.x / QuestDB**: Optimize for ingestion velocity (millions of events/sec). Modern InfluxDB 3.x (Parquet-based) sacrifices real-time querying for compression and columnar efficiency.
- **TimescaleDB**: Balances write and read performance; full SQL shifts complexity to the query planner, not to custom DSLs.

### Single Node vs. Distributed

- **TimescaleDB, QuestDB**: Single-node datastores; replication is external (WAL streaming, dedicated replicas). Simpler operations, deterministic latency.
- **ClickHouse**: Native sharding; distributed queries; added operational complexity (shard key selection, cross-shard aggregations).

### Cardinality Limits

High-cardinality dimensions (millions of unique tag values) cause index explosion and memory pressure. Each TSDB has a practical limit (ClickHouse ~100 million series per node; InfluxDB 3.x rethinks the problem with Parquet). Design schemas to bound cardinality: avoid user IDs as tags, prefer bucketed values or hashes.

## One More Thing: Continuous Queries

Some systems (InfluxDB historically, Esper, Kafka Streams) add **continuous queries** — standing queries that emit results as new data arrives:

```
SELECT avg(value) INTO downsampled FROM raw_data GROUP BY time(1h)
```

This is distinct from continuous aggregates (materialized views). Different systems have different names; understand your system's model.