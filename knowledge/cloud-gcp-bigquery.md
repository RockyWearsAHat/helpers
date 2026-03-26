# Google Cloud BigQuery — Dremel Analytics, Cost Optimization & ML

BigQuery is GCP's serverless analytics engine built on Dremel, an in-situ columnar query processor that decouples storage and compute. It powers interactive queries on petabyte-scale datasets with per-GB pricing, making it a fundamental shift from how traditional data warehouses (Redshift, Snowflake) allocate resources.

## Dremel: Query Execution Engine

### Architecture Overview

Dremel executes queries across a distributed tree of hundreds or thousands of nodes in parallel. Each node processes a portion of the data independently, and results merge bottom-up through aggregation.

**Key phases:**
1. **Scan phase** — Leaf nodes read columnar data blocks (from GCS Colossus), apply column selection and pushdown predicates, emit intermediate results
2. **Shuffle phase** — Results flow up the tree; GROUP BY and JOIN operations redistribute data across nodes by key
3. **Aggregation phase** — Higher tree levels combine partial aggregates (e.g. SUM, COUNT) via associative operations
4. **Merge phase** — Root node produces final result set

A query's execution tree is optimized dynamically: fast reads early, expensive operations (joins, window functions) parallelized.

### Column Striping and Record Assembly

Dremel treats each column as an independent entity:

- Columns are **striped** and encoded separately (dictionary compression, run-length encoding, delta encoding)
- Records are **assembled on-demand** during execution when multiple columns are needed
- Null handling uses repetition and definition levels, not materialized NULL markers

This architecture means:
- A query touching 3 columns from a 50-column table reads only 6% of table data
- Compression is highly efficient (correlated columns compress well)
- Projection pushdown is "free" — the execution engine never materializes unused columns

### Slots: Compute as a Resource

BigQuery decouples capacity from individual queries. A **slot** is a unit of processing power (roughly a CPU core equivalent). Slots are pooled across all queries in a project; idle capacity is immediately available.

**Pricing models:**
- **On-demand:** Pay per-TB scanned ($7/TB in US during 2024). No reservation required. Unused capacity → no cost. Suitable for variable, exploratory workloads
- **Annual/monthly commitments:** Buy 100-slot annual commitments; slots are reserved, unused slots are "dead" capacity. Queries run on unlimited slots (burst); cost flattens. Suitable for steady-state 24/7 workloads
- **Flex slots:** Rent slots by the hour, bridge between on-demand and commitments

Slot allocation is **task-aware**. A large multi-stage query reserves more slots; a small single-pass query uses few. This dynamic allocation prevents resource starvation.

## Colossus: Distributed Storage

BigQuery stores data in **Colossus**, GCP's distributed columnar filesystem. Data is replicated 3x across zones for durability, transparently compressed.

- **Format:** Capacitor (Google's columnar format, similar to Parquet but optimized for Dremel)
- **Partitioning:** Logical division of a table into independent blocks
- **Clustering:** Sort order within partitions for locality

### Partitioning Strategies

BigQuery supports three partitioning modes:

**1. Time partitioning** — Partition by a TIMESTAMP/DATE column (most common). Partition granularity: day, hour, month, year.
```
CREATE TABLE events (
  timestamp TIMESTAMP,
  user_id INT64,
  event_type STRING
)
PARTITION BY DATE(timestamp)
– Results in partitions: 2024-01-01, 2024-01-02, etc.
– Queries with WHERE timestamp >= '2024-03-01' scan only Jan–Mar partitions
```

**2. Range partitioning** — Partition by integer column ranges. Useful for IDs or numeric identifiers.
```
PARTITION BY RANGE_BUCKET(user_id, GENERATE_ARRAY(0, 1000000, 100000))
– Creates ranges: [0, 100K), [100K, 200K), etc.
```

**3. Ingestion-time partitioning** — Partition by table load time (\_PARTITIONTIME pseudocolumn). Useful when source already ordered; doesn't require explicit column.

**Partition pruning example:**
- Table: 100 GB total, partitioned by date (100 partitions, ~1 GB each)
- Query: `SELECT * FROM events WHERE timestamp BETWEEN '2024-03-01' AND '2024-03-10'`
- Data scanned: ~10 partitions = 10 GB; cost reduced 10x

Partition elimination is **automatic** during query planning—no hint required.

### Clustering

Clustering is **co-locating rows with similar values for specified columns,** within each partition. Unlike partitioning, clustering doesn't reduce the number of scanned blocks—it enables **block-level pruning** during execution.

```
CREATE TABLE sales (
  date DATE,
  region STRING,
  amount INT64
)
PARTITION BY date
CLUSTER BY region, amount;
```

When queries filter on **clustered columns**, BigQuery skips unnecessary blocks:
- Query `WHERE region = 'US-West' AND amount > 1000` skips blocks not containing that region + amount range
- Result: Lower data scanned within each partition

Choose clustering columns based on:
- **High-cardinality filters** (avoid clustering on low-cardinality Boolean fields)
- **JOIN keys** (clustering by the key on both sides reduces shuffle)
- **Query frequency pattern** (profile slow queries for candidates)

**Partitioning + Clustering synergy:** Partition by time, cluster by geography → prunes both temporal and geographic ranges.

## Materialized Views

A **materialized view** is a **pre-aggregated snapshot** of a query, stored as a new table. BigQuery auto-triggers refresh when base tables change.

```
CREATE MATERIALIZED VIEW daily_sales AS
SELECT
  DATE(timestamp) as date,
  region,
  SUM(amount) as total_sales,
  COUNT(*) as count
FROM sales
GROUP BY date, region;
```

**When materialized views accelerate queries:**
- Repeated aggregations (same GROUP BY/SUM across many queries)
- Pre-joined fact tables
- Expensive transformations (complex string parsing, JSON extraction)

**When they don't help:**
- One-off queries (view refresh cost exceeds query time saved)
- Highly volatile base tables (frequent refreshes = wasted compute)
- Low-cardinality results (most aggregations already fit in memory)

Views are **transparently substitutable:** If you query the base table with the same GROUP BY, BigQuery's optimizer detects and rewrites to use the materialized view. On-demand updates prevent staleness.

## BigQuery ML (BQML)

BQML embeds machine learning **inside SQL**, eliminating data export:

```
CREATE OR REPLACE MODEL fraud_model
OPTIONS(model_type='linear_reg') AS
SELECT amount, num_transactions, days_active
FROM transactions
WHERE label IS NOT NULL;

-- Predict on new data
SELECT predicted_amount, amount, SQRT(POW(predicted_amount - amount, 2)) as error
FROM ML.PREDICT(fraud_model, (
  SELECT amount, num_transactions, days_active
  FROM new_transactions
));
```

Supported models:
- **Linear/Logistic regression** — baseline models; fast training
- **Time series forecasting (ARIMA+)** — seasonal data; trend extrapolation
- **Clustering (K-means)** — unsupervised segmentation
- **Deep neural networks (DNNs)** — tabular classification/regression
- **Boosted/Random Forest trees** — XGBoost integration
- **Tensor Flow models** — import external models; run inference in BigQuery

**Cost model:** Training = compute cost (slot-based). Batch prediction = scan cost (pay for scanned tables). Queries on predictions = normal query cost.

## BI Engine

BI Engine is an **in-memory acceleration layer** that caches query results and intermediate aggregations. For repeated queries (dashboards, automated reports), BI Engine intercepts the query, checks the cache, and returns results in milliseconds without touching Colossus.

**How it works:**
- Allocate BI Engine capacity per dataset (typically 0.5–50 GB per project)
- Run queries normally; BI Engine automatically caches SELECT subqueries, GROUP BY aggregations
- Identical queries within the cache window hit 0ms latency

**Use cases:**
- BI dashboards with fixed report set (0ms latency for executives)
- Automated monitoring dashboards (same 20 queries every 5 min)
- Interactive BI tools querying fixed dimensions (Tableau, Looker)

**Tradeoff:** Cached results are slightly stale (updated on write). For real-time data, disable caching.

## Streaming Inserts vs Batch

### Streaming Inserts

**Definition:** Write individual rows or small batches in real-time to BigQuery \_PARTITIONS. Rows are immediately queryable (propagation ~seconds).

```bash
bq insert --skip_leading_rows=1 --autodetect events.json my_table
```

**Cost:** $6.25 per TB ingested (separate from storage/query costs). **6.25x more expensive** than batch loading.

**When to use:**
- Event streams (Kafka → BigQuery connectors like Dataflow)
- Real-time dashboards needing fresh data
- Unbounded data sources (logs, IoT sensors)

**Gotchas:**
- High volume = expensive. A 1-week 1-MB/sec stream = 600 GB/week = $3,750/week just for ingestion
- Rows in _PARTITIONS are not yet in Colossus; queries are slower until consolidation (async background job)
- Hard quota: 100k rows per second per project (soft limit, throttled beyond)

### Batch Loading

**Definition:** Load tables from GCS files (Avro, Parquet, JSON, CSV) or from other BigQuery tables. Rows are grouped, compressed, and written to Colossus atomically.

```bash
bq load --source_format=PARQUET my_table gs://bucket/data.parquet
```

**Cost:** $6.25 per TB, same as streaming. BUT: Data is loaded into Colossus directly, no _PARTITIONS overhead, faster queries.

**When to use:**
- Scheduled ETL (hourly, daily data ingestion)
- Data warehouse migration (`COPY TABLE` from other engines)
- High-volume ingestion (terabytes/hour without streaming tax)

**Optimization:** Load from Parquet files (compressed) rather than JSON/CSV; compression reduces ingestion size 5-10x.

## Cost Optimization Patterns

### Query Cost (Bytes Scanned)

1. **Partition pruning** — Always include partition column filters. Cost difference: 10x+.
   ```sql
   – AVOID: SELECT * FROM events; – Scans entire table
   – GOOD: SELECT * FROM events WHERE DATE(timestamp) >= CURRENT_DATE() - 7; – 7 partitions only
   ```

2. **Column selection** — Query only needed columns.
   ```sql
   – AVOID: SELECT * FROM billion_row_table;
   – GOOD: SELECT user_id, event_type FROM billion_row_table; – Costs proportional to column count
   ```

3. **Clustering on filter columns** — For high-cardinality filters not appropriate for partitioning.

4. **Materialized views** — Pre-aggregate common aggregations.

### Cost Avoidance

- **Avoid UNION ALL of many large tables** — Scanned separately, costs additive
- **Avoid SELECT * unless necessary** — Column pruning is free but requires explicit projection
- **Use APPROX_* functions** — `APPROX_COUNT_DISTINCT()` scans same data but faster
- **Use BI Engine for dashboards** — Caches reduce repeated scan cost to zero

### Pricing Models Comparison

| Model           | Base Cost | Use Case                    | Commitment |
|-----------------|-----------|-----------------------------| ------------|
| On-demand       | $7/TB     | Unpredictable, exploratory  | None       |
| Annual slots    | $0.04/slot/hour | Steady >100 TB/month   | 1 year     |
| Monthly slots   | $0.05/slot/hour | Steady <100 TB/month   | 1 month    |
| Flex slots      | $0.04/slot/hour | 100-hour bursts        | Hourly     |

**Crossover point:** At ~3 PB/month queried, annual slots break even vs on-demand.

## See Also

- `cloud-gcp-data` — GCP data services overview; Dataflow, Dataproc
- `data-engineering-warehousing` — Snowflake, Redshift comparisons
- `data-engineering-formats` — Parquet, Avro encoding tradeoffs
- `database-timeseries` — Time-series optimization (events, metrics)