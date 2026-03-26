# Real-Time Analytics — OLAP Engines, Streaming SQL & Approximate Algorithms

## Introduction

**Real-time analytics** answers business questions on **fresh data** at **scale** with **sub-second latency**. Traditional OLAP (Online Analytical Processing) systems like Redshift or BigQuery are tuned for batch: queries run on daily snapshots, taking seconds to minutes. Real-time analytics reverses the constraint: data arrives continuously (streaming), and queries must answer on-demand within milliseconds.

This note covers OLAP engines designed for real-time, streaming SQL platforms, approximate algorithms for cardinality/frequency estimation, and architectural patterns for fast analytics.

## OLAP Engines for Real-Time Analytics

### ClickHouse

**ClickHouse** is a columnar OLAP database optimized for analytics on massive datasets (petabyte scale).

**Architecture:**
- **Column-oriented storage:** Values in a column stored sequentially. E.g., to sum a column, CPU reads only that column (not whole rows), enabling compression and vectorized operations.
- **Data compression:** Multiple compression algorithms (LZ4, ZSTD, Delta); typical data sizes 10-50x smaller than row-oriented DBs.
- **Distributed architecture:** Tables can be sharded across multiple nodes; queries are parallelized (scatter-gather).
- **MergeTree table engine:** Immutable blocks; new data arrives in buffers, periodically merged. Supports replicas across nodes for HA.

**Query performance:**
- Typical queries on billions of rows return results in milliseconds to seconds.
- Achieves sub-second latency on pre-aggregated data or selective queries (highly parallelized, CPU-efficient).

**Cons:**
- No transactions (ACID). Inserts/updates are eventual consistent.
- Schema migrations are expensive (rewrite table).
- High memory usage (loads full columns into memory for queries).

**Use cases:** Logs, metrics, analytics events (immutable append-only streams).

### Apache Druid

**Druid** is a real-time OLAP datastore designed for streaming analytics and dashboarding.

**Key characteristics:**
- **Real-time ingestion:** Native connection to Kafka, Kinesis; millisecond latency from event to queryable.
- **Segment-based storage:** Data segments (time-chunked) stored in files; queries prune segments by time range to reduce I/O.
- **Query caching:** Results cached by query hash; identical queries hit cache (sub-millisecond).
- **DShapeIndex:** Sketch data structures for approximate aggregations (cardinality, quantiles) on high-cardinality dimensions.

**Operational model:**
- **Coordinator:** Segment assignment, load balancing.
- **Broker:** Query planner, result aggregation.
- **MiddleManager:** Ingestion from Kafka, real-time indexing.
- **Historical:** Serves historical segments from deep storage (S3, HDFS).

Good for dashboards on high-concurrency reads, time-series analysis, ad-hoc OLAP on streaming data.

**Cons:**
- Steeper operational complexity (many processes).
- Higher memory footprint than ClickHouse (sketch structures).

### Apache Pinot

**Pinot** is similar to Druid: real-time and batch analytics engine. Emphasizes extreme scale (trillions of rows) with low latency.

**Differences from Druid:**
- Native SQL support (Presto/Trino compatible).
- Supports star-join mode (fact + dimension tables; can do joins at query time).
- Optimized for fact tables (many rows, few columns per query).

**Architecture:**
- **Controller:** Cluster coordination.
- **Broker:** Query planner.
- **Server:** Stores segments, serves queries.
- **Segment format (mutable):** Rebuilds periodically; supports schema changes better than immutable formats.

Use cases: Ad-hoc analytics, dashboards, metric queries at massive scale.

### DuckDB

**DuckDB** is an embedded, in-process SQL database optimized for analytical queries.

**Key characteristics:**
- **Embedded:** No separate server process. SQL engine runs in application process (or single-machine analytics).
- **Columnar:** Column-oriented storage and execution.
- **OLAP optimizer:** Pushes predicates, vectorizes execution.
- **Parquet/CSV support:** Native read/write of Parquet, CSV, JSON, Arrow without copying.
- **Extensions:** Can add more data sources (Postgres, MySQL, Iceberg, etc.).

**Performance:**
- Fast for medium datasets (gigabytes to low terabytes) on single machine.
- Latency: tens of milliseconds for typical queries.

**Pros:**
- Zero operational overhead (embedded).
- Great for local analytics, Jupyter notebooks, small-medium OLAP.

**Cons:**
- Single-machine (no distributed query execution).
- Not suitable for multi-user concurrency at scale.

### Trade-offs Among Engines

| Engine | Latency | Scale | Ingestion | ACID | Operational Complexity | Best For |
|--------|---------|-------|-----------|------|-------------------------|----------|
| **ClickHouse** | 100ms-1s | Petabytes | Batch/CDC | No | Moderate | Append-only events, logs |
| **Druid** | 10-100ms | Trillions (sketches) | Real-time Kafka | No | High | Dashboard, high-concurrency |
| **Pinot** | 100ms-1s | Trillions | Real-time Kafka | No | High | Fact table analytics, SQL |
| **DuckDB** | 10-100ms | Terabytes | Local/embedded | Yes | None | Single-machine, analytics |

## Materialized Views & Pre-Aggregation

Real-time queries can't always scan raw data (too expensive). **Pre-aggregation** trades space for query latency.

### Materialized Views

A materialized view is a **pre-computed aggregate** stored as a table:

```sql
-- Raw events table (immutable, append-only, millions of rows/sec)
CREATE TABLE events (
  customer_id UUID,
  event_type VARCHAR,
  amount DECIMAL(18,2),
  ts TIMESTAMP
);

-- Materialized view: daily revenue per customer
CREATE MATERIALIZED VIEW daily_revenue AS
SELECT 
  customer_id,
  DATE(ts) as day,
  SUM(amount) as revenue,
  COUNT(*) as event_count
FROM events
WHERE event_type = 'purchase'
GROUP BY 1, 2;
```

**Maintenance:**
- **Full rebuild:** Recompute entire MV from scratch. Expensive for large tables. Typically scheduled nightly.
- **Incremental (delta) refresh:** Only recompute for new/changed data since last refresh. Requires tracking of ETL state (checkpoint, watermark).
- **Real-time MV:** Push updates to MV as fact data arrives. Lowest latency but highest operational cost.

### MV Challenges

1. **Freshness lag:** MV lags behind raw data by refresh interval (hourly, daily). Queries see stale results.
2. **Dimension explosion:** MV with dimensions {week, country, product, customer_type} has millions of combinations; storage explodes.
3. **Maintenance burden:** Each MV requires monitoring, error handling, tuning.

### Pre-Aggregation Patterns

**Partial aggregation:** Instead of materializing every dimension combination, pre-aggregate at coarser granularity:

```
Raw events → Hourly aggregates → Daily rollups → Monthly summaries
(billions)      (millions)         (thousands)       (hundreds)
```

Queries on granular data (specific hour) hit hourly table; queries on daily data hit daily table (fewer rows to scan).

**Adaptive aggregation:** System automatically determines which granularities to pre-aggregate based on query patterns (learned).

## Lambda and Kappa Architectures

### Lambda Architecture

Two parallel paths for analytics:

```
Source → [Batch Pipeline] -----> Batch View (truth, accurate) ---→
                                                                    → Serving Layer
         [Streaming Pipeline] → Speed View (low-latency, approx) →
```

- **Batch**: Processes all historical + new data periodically (daily); produces accurate results.
- **Speed**: Processes streaming data in near real-time; lower latency but may have duplicates or out-of-order.
- **Serving layer**: Union results from both (batch takes precedence, fills gaps).

**Advantages:**
- Batch provides accuracy/correctness; speed provides freshness.
- Failures in either path don't block both (resilience).

**Disadvantages:**
- Complex to maintain (two code paths, two state machines, debugging is harder).
- Requires careful conflict resolution (same query from batch vs. speed may disagree).
- Cost: duplicate processing and storage.

### Kappa Architecture

**Single path** (streaming replaces both batch and speed):

```
Source → [Streaming Pipeline] → Store (with full history/replayability) → Serving Layer
```

- All processing happens in stream (Kafka, Flink, ksqlDB).
- Store retains full event history (immutable log); new analytics can be added by replaying.
- No separate batch pipeline.

**Advantages:**
- Simpler operational model (one code path).
- No lambda merge logic.
- Replayable: bugs in past analytics can be fixed by replaying stream.

**Disadvantages:**
- Requires streaming platform maturity (Kafka, Flink, etc.).
- Stream processing must be production-grade (exactly-once semantics, error handling).
- Historical reprocessing can be slow (streaming is often slower than batch for bulk work).

**Reality:** Most companies use **hybrid**: critical paths use Kappa (streaming); batch used selectively for expensive one-time analytics (e.g., ML training data prep).

## Streaming SQL Platforms

### ksqlDB

**ksqlDB** is a streaming SQL database built on Kafka. Write SQL to define streams and materialized views:

```sql
-- Define stream from Kafka topic
CREATE STREAM events (
  customer_id UUID,
  event_type VARCHAR,
  amount DECIMAL(18,2)
) WITH (
  kafka_topic='events',
  value_format='json'
);

-- Materialized view (continuous aggregation)
CREATE TABLE hourly_revenue AS
SELECT 
  WINDOWSTART as hour,
  customer_id,
  SUM(amount) as revenue
FROM events
WINDOW TUMBLING (SIZE 1 HOUR)
WHERE event_type = 'purchase'
GROUP BY WINDOWSTART, customer_id;
```

**Benefits:**
- SQL familiar to analysts (low barrier).
- Stateful processing (windows, joins, aggregations) managed by platform.
- Scales to high throughput Kafka topics.

**Limitations:**
- Limited to Kafka (source and sink).
- Window operations (TUMBLING, HOPPING, SESSION) are Kafka Streams concepts; not all SQL engines support.
- Debugging is harder than batch SQL (state is distributed).

### Apache Flink SQL

**Flink** is a distributed stream processing engine; Flink SQL is the SQL layer.

```java
StreamExecutionEnvironment env = StreamExecutionEnvironment.getExecutionEnvironment();
StreamTableEnvironment tableEnv = StreamTableEnvironment.create(env);

// Source from Kafka
tableEnv.executeSql(
  "CREATE TABLE events (" +
  "  customer_id STRING," +
  "  event_type STRING," +
  "  amount DECIMAL(18,2)," +
  "  ts TIMESTAMP(3) WATERMARK FOR ts AS ts - INTERVAL '5' SECOND" +
  ") WITH (" +
  "  'connector' = 'kafka'," +
  "  'topic' = 'events'" +
  ")"
);

// Streaming SQL
Table result = tableEnv.sqlQuery(
  "SELECT TUMBLE_START(ts, INTERVAL '1' HOUR) as hour, " +
  "       SUM(amount) as revenue " +
  "FROM events " +
  "GROUP BY TUMBLE_START(ts, INTERVAL '1' HOUR), customer_id"
);
```

**Advantages:**
- More expressive than ksqlDB (supports complex joins, UDFs).
- Scalable to very high throughput.
- Watermarking support (handles out-of-order data).
- Works with multiple sources (Kafka, S3, Postgres, etc.).

**Disadvantages:**
- Steeper learning curve (JVM-based, requires Java/Scala knowledge).
- Operationally complex (cluster management, state backend config).

### Spark Structured Streaming

**Spark** is batch-oriented, but Structured Streaming adds streaming API:

```python
spark = SparkSession.builder.appName("analytics").getOrCreate()

# Read from Kafka
df = spark.readStream \
  .format("kafka") \
  .option("kafka.bootstrap.servers", "localhost:9092") \
  .option("subscribe", "events") \
  .load()

# Parse JSON
events = df.select(F.from_json(F.col("value"), schema).alias("data")).select("data.*")

# Stateful aggregation (micro-batches, checkpointing)
result = events.groupBy(
  F.window("ts", "1 minute"),
  "customer_id"
).agg(F.sum("amount"))

# Write to console/storage
query = result.writeStream \
  .format("console") \
  .option("checkpointLocation", "/tmp/checkpoint") \
  .start()

query.awaitTermination()
```

**Advantages:**
- Familiar (Spark syntax for batch and streaming).
- Scales horizontally (Spark cluster).

**Disadvantages:**
- Micro-batching (minimum latency ~500ms determined by batch interval).
- Less ideal for very low-latency (10-100ms) requirements.

## Time-Windowed Aggregation

A critical operation: **windowed aggregation** (sum/count/avg over time windows).

### Window Types

**Tumbling (fixed, non-overlapping):**
```
Events: ----*----*------*---*---------*--*--
        [0-1h)  [1h-2h) [2h-3h)  [3h-4h)
```
Use case: Reports by hour, day.

**Hopping (fixed, overlapping):**
```
Window size 1h, step 30m:
        [0-1h)
          [30m-1.5h)
               [1h-2h)
```
Use case: Sliding window metrics (last hour, rolling).

**Session (grouped by inactivity):**
```
Events: activity--gap(30m)--activity--gap(30m)--activity
        [session 1]        [session 2]
```
Use case: User sessions (grouped interactions with gaps).

**Handling late data (out-of-order):**

In distributed systems, events arrive out-of-order. E.g., event at 3pm may arrive at 3:05pm. Windows must close at **watermark** (latest guaranteed timestamp):

```
Watermark: 3:01pm
Events received: 2:50pm, 2:55pm, 3:00pm, 3:02pm (late)
Window [2:30-3:00): gets events 2:50, 2:55, 3:00 → closed
Late arrivals (3:02pm): ignored or re-added to next window (depends on allowed lateness config)
```

## Approximate Algorithms

Real-time analytics often uses **approximate computation** for speed, sacrificing perfect accuracy:

### HyperLogLog (Cardinality Estimation)

Estimates **distinct count** (cardinality) using probabilistic sketch.

**Problem:** SELECT COUNT(DISTINCT customer_id) over massive data is expensive (large hash set needed).

**Solution:** HyperLogLog stores sketch (~1-2KB regardless of dataset size). Estimates cardinality with configurable error (e.g., 2% error).

**Example:**
```python
import hyperloglog

# Process events one-by-one, updating sketch
sketch = hyperloglog.HyperLogLog(precision=14)  # precision controls memory/accuracy
for event in massive_stream:
  sketch.add(event['customer_id'])

distinct_customers = sketch.cardinality()  # ~cardinality ± 2%
```

**Accuracy vs. size trade-off:** Higher precision = larger sketch, lower error.

### Count-Min Sketch

Estimates **frequency of elements** (how many times does each customer_id appear?).

**Problem:** Full count map requires O(N) memory. Frequent queries ("top 100 customers by event count") scale poorly.

**Solution:** Count-Min Sketch: fixed-size 2D array, k hash functions. Each element hashed k times; increment all k cells. Cardinality estimated by minimum across k cells. False positives possible; underestimation is minimal.

**Example:**
```
Element 'customer_123' hashed to cells [5][2], [7][8], [9][1]
Increment count at all three cells.
Query: count['customer_123'] = min([5][2], [7][8], [9][1])  # conservative estimate
```

### T-Digest (Percentile/Quantile Estimation)

Estimates **percentiles** efficiently (p50, p95, p99 latencies).

**Problem:** Storing all values (for exact percentile computation) is expensive. Sorting is expensive.

**Solution:** T-Digest is a sketch that approximates percentiles. Useful for streaming metrics (latency histograms, value distributions).

```python
from tdigest import TDigest

digest = TDigest()
for latency in stream:
  digest.add(latency)

p50 = digest.percentile(50)  # median latency
p95 = digest.percentile(95)  # 95th percentile
p99 = digest.percentile(99)  # 99th percentile (approximate ±1-2% error)
```

### When to Use Approximate

- **Dashboards:** Approximate is sufficient; users rarely notice 1-2% error.
- **Alerting:** May need exact (e.g., alerting on exact thresholds). Approx suitable for trending.
- **Real-time ranking:** "Top 100 customers by spend" — approximate order is usually acceptable.
- **High-frequency ingestion:** 1M events/sec; exact cardinality every second is expensive. Approximate + periodic exact.

**Downsides:** Not suitable for financial reporting, audit trails, precise allocations (billing).

## Real-Time Analytics Architecture

Typical stack:

```
Source Systems (transactional DBs, APIs)
        ↓ [CDC / Event streaming]
Message Broker (Kafka)
        ↓ [Streaming processing: Flink, ksqlDB]
Real-time OLAP (ClickHouse, Druid, Pinot)
        ↓ [SQL / Dashboards]
Analytics / Dashboards / Monitoring
```

**Data flow:**
1. Source emits changes (CDC or event logs).
2. Broker buffers and distributes.
3. Streaming processor cleanses, enriches, aggregates (windowed operations).
4. OLAP engine stores pre-aggregated data, indexed for fast queries.
5. Dashboards query OLAP with low latency.

**Operational concerns:**
- **Replayability:** Kafka retains data (days/weeks); if processor fails, can replay and rebuild state.
- **Exactly-once semantics:** Processor must checkpoint state; on recovery, resume from checkpoint (avoid duplicate processing).
- **Late data handling:** Windowed operations must tolerate out-of-order events (watermarks, allowed lateness).
- **Scaling:** Processor should auto-scale to match throughput; OLAP should support read replicas for high query concurrency.

## See Also

- [data-engineering-etl.md](data-engineering-etl.md) — Batch ETL and data pipelines
- [data-engineering-spark.md](data-engineering-spark.md) — Apache Spark for large-scale data processing
- [data-engineering-airflow.md](data-engineering-airflow.md) — Workflow orchestration (scheduling batch + streaming)
- [data-change-data-capture.md](data-change-data-capture.md) — CDC patterns for streaming data
- [infrastructure-message-broker-patterns.md](infrastructure-message-broker-patterns.md) — Kafka, Pulsar architecture
- [algorithms-randomized.md](algorithms-randomized.md) — Probabilistic algorithms (HyperLogLog, Count-Min)