# GCP Data Services — Analytics, AI & Data Pipeline Concepts

## BigQuery as Serverless Analytics

BigQuery implements a **separation of storage and compute** architecture where data resides in a distributed columnar store (Capacitor format) and query execution uses a dynamically allocated pool of compute resources called **slots**.

### Storage Model

Data is organized into tables within datasets within projects. The columnar format stores each column independently, enabling:

- **Column pruning** — Queries that reference a subset of columns read only those columns, reducing I/O proportional to the fraction of schema accessed
- **Compression efficiency** — Columnar storage compresses well because values within a column share data type and often exhibit similar patterns
- **Partition and clustering** — Tables can be partitioned (by time, integer range, or ingestion time) and clustered (sorted by specified columns within each partition) to further limit data scanned

```
Storage Hierarchy:
Project → Dataset → Table → Partition → Cluster Block → Column Chunk

Query Cost Model:
  Cost ∝ bytes_scanned (after partition pruning, cluster filtering, column selection)
  NOT ∝ table_size or query_complexity
```

### Execution Model: Slots

A **slot** is a unit of computational capacity — roughly analogous to a virtual CPU for query processing. Two pricing models reflect different slot acquisition strategies:

| Model               | Slot Allocation           | Cost Behavior         | Fit                           |
| ------------------- | ------------------------- | --------------------- | ----------------------------- |
| On-demand           | Dynamic pool, per-query   | Pay per bytes scanned | Sporadic, variable workloads  |
| Capacity (editions) | Reserved pool, guaranteed | Fixed hourly rate     | Steady, predictable workloads |

On-demand pricing bills per terabyte scanned regardless of complexity, making it simple but potentially expensive for frequent large scans. Capacity pricing provides a guaranteed slot pool, making costs predictable but requiring utilization management to avoid paying for idle capacity.

### Materialized Views and BI Engine

- **Materialized views** pre-compute query results and are maintained incrementally as base tables change. The optimizer can automatically rewrite queries to use materialized views even when not explicitly referenced.
- **BI Engine** is an in-memory analysis service that accelerates certain query patterns by caching frequently accessed data in RAM. It operates transparently — no query changes needed — but has memory capacity limits that determine how much data benefits from acceleration.

### Multi-Statement Transactions and DML

BigQuery supports ACID transactions across multiple statements within a session, enabling patterns like read-modify-write that previously required external coordination. DML operations (INSERT, UPDATE, DELETE, MERGE) work on tables but incur different cost and performance profiles than append-only ingestion.

## Dataflow: Stream and Batch Processing

Dataflow implements the **unified programming model** where the same pipeline logic processes both bounded (batch) and unbounded (streaming) data. The core abstractions:

- **PCollections** — Distributed datasets that can be bounded (finite) or unbounded (continuously arriving)
- **Transforms** — Operations that take PCollections as input and produce PCollections as output
- **Windowing** — Divides unbounded data into finite chunks for aggregation. Window types include fixed, sliding, session, and global.
- **Triggers** — Policies for when to emit results from a window. Interact with watermarks (the system's estimate of input completeness) to balance latency against completeness.
- **Watermarks** — Heuristic timestamps tracking how far behind real-time the system believes unprocessed data might be

```
Unified Model Concepts:

  What are you computing?    → Transforms (ParDo, GroupByKey, Combine)
  Where in event time?       → Windowing
  When in processing time?   → Triggers + Watermarks
  How do refinements relate? → Accumulation (discarding, accumulating, retracting)
```

### Autoscaling and Resource Management

Dataflow automatically scales worker count based on pipeline backlog and throughput. Streaming pipelines scale between a configured minimum and maximum. Batch pipelines scale based on work remaining.

Trade-offs in pipeline design:

- **Fusion** — Dataflow may fuse sequential transforms into a single stage for efficiency. This reduces serialization overhead but means the combined stage is a single scaling unit. Explicit side outputs or reshuffles can break fusion when independent scaling is needed.
- **Shuffle** — Data redistribution (GroupByKey, CoGroupByKey) determines parallelism boundaries. Skewed key distributions can cause hot spots where one worker processes disproportionate data.
- **Exactly-once vs at-least-once** — Streaming Dataflow provides exactly-once processing semantics by default through checkpointing and deduplication, at the cost of higher latency compared to at-least-once systems.

## Dataproc: Managed Hadoop/Spark Ecosystems

Dataproc provides managed clusters for workloads built on the Hadoop ecosystem — Spark, Hive, Pig, Presto, and related tools. Its design philosophy centers on **ephemeral clusters**: spin up a cluster, run a job, tear it down.

Key concepts:

- **Cluster lifecycle** — Clusters can be created with initialization actions (scripts that install custom software), used for one or more jobs, and deleted. This differs from persistent cluster models where clusters run continuously.
- **Autoscaling** — Secondary (preemptible/spot) workers can scale based on YARN metrics. Primary workers hold HDFS data and are not scaled.
- **Storage decoupling** — Using Cloud Storage (via the gs:// connector) instead of HDFS enables ephemeral clusters because data persists independently of cluster lifecycle.
- **Component gateway** — Provides web access to cluster UIs (Spark UI, YARN, Jupyter) without SSH tunnels.
- **Dataproc Serverless** — Submits Spark batch or interactive (via notebooks) workloads without cluster management. Infrastructure is fully managed at the cost of less customization.

### When Managed Spark vs. Serverless Analytics

The choice between Dataproc (or Dataproc Serverless) and BigQuery often involves:

| Factor              | Dataproc / Spark                                 | BigQuery                    |
| ------------------- | ------------------------------------------------ | --------------------------- |
| Existing codebase   | Strong Spark/Hadoop investment                   | SQL-centric teams           |
| Processing model    | Complex DAGs, ML pipelines, iterative algorithms | Analytical SQL queries      |
| Data format control | Custom formats, existing data lakes              | Managed columnar storage    |
| Cost model          | Cluster-hours (ephemeral or persistent)          | Bytes scanned or slot-hours |
| Ecosystem needs     | Specific Hadoop ecosystem tools                  | Self-contained analytics    |

Neither is universally superior; the fit depends on team capabilities, existing code, and workload characteristics.

## Pub/Sub: Messaging Backbone

Pub/Sub is a fully managed messaging service that decouples producers and consumers. Messages flow from publishers to topics, then to subscriptions where subscribers consume them.

### Delivery Semantics

- **At-least-once delivery** — Pub/Sub guarantees every message is delivered at least once. Duplicate delivery can occur, so consumers should be idempotent or implement deduplication.
- **Ordering guarantees** — Messages with the same ordering key within a topic are delivered in order to a subscriber. Without ordering keys, delivery order is not guaranteed. Ordering adds constraints on throughput because ordered messages in a group are processed sequentially.
- **Message retention** — Unacknowledged messages are retained for a configurable period (default 7 days). Acknowledged messages can optionally be retained, enabling seek/replay operations.

### Subscription Types

| Type          | Consumer Model                  | Delivery                                    |
| ------------- | ------------------------------- | ------------------------------------------- |
| Pull          | Consumer polls for messages     | Consumer-controlled rate                    |
| Push          | Pub/Sub sends to HTTPS endpoint | Server-driven, backpressure via HTTP status |
| BigQuery      | Direct write to BigQuery table  | Automatic, managed                          |
| Cloud Storage | Write to Cloud Storage buckets  | Batched, file-based                         |

### Dead-Letter Topics

Messages that fail processing after a configurable number of delivery attempts are forwarded to a dead-letter topic. This prevents a single problematic message from blocking consumption of subsequent messages (the "poison pill" problem). Dead-letter handling requires monitoring to ensure failed messages are investigated, not silently accumulated.

### Flow Control and Backpressure

Pull subscribers configure flow control parameters (max outstanding messages, max outstanding bytes) to bound memory usage. Without flow control, a fast topic can overwhelm a slow subscriber, leading to memory exhaustion or cascading failures.

## Cloud Composer: Workflow Orchestration

Cloud Composer is a managed implementation of Apache Airflow for authoring, scheduling, and monitoring workflows (DAGs — Directed Acyclic Graphs).

Core concepts:

- **DAGs** define task dependencies as directed graphs. Each task wraps an operator that performs work (run a query, trigger a Dataflow pipeline, call an API).
- **Operators** abstract execution targets. Sensor operators wait for conditions; transfer operators move data between systems; action operators execute commands.
- **Scheduling** — DAGs run on configurable schedules or in response to triggers. Execution dates, catchup behavior, and backfill policies control how missed or historical runs are handled.
- **XComs** (cross-communications) pass small data between tasks within a DAG. They suit metadata (row counts, file paths) but not bulk data transfer.

Environment sizing involves balancing Airflow scheduler performance, worker capacity, and database backend resources. Under-provisioned environments exhibit scheduler lag (DAG run delays) or worker starvation (queued tasks waiting for free workers).

The conceptual tension in orchestration: thin orchestration (Composer only triggers external systems, which do the heavy processing) vs thick orchestration (Composer workers perform significant data processing directly). Thin orchestration is generally more scalable and maintainable; thick orchestration can be simpler for lightweight tasks.

## Vertex AI Platform Concepts

Vertex AI consolidates ML infrastructure into a unified platform spanning the model lifecycle:

### Training

- **Custom training** — User-provided training code runs on managed compute (VMs with optional GPUs/TPUs). Container-based — bring any framework.
- **AutoML** — Automated model architecture search and hyperparameter tuning for tabular, image, text, and video tasks. Trades control for convenience.
- **Training pipelines** — Orchestrated sequences of training steps, data preprocessing, and evaluation.

### Serving

- **Endpoints** — Managed HTTP endpoints that host deployed models for online prediction. Support traffic splitting between model versions for canary deployments or A/B testing.
- **Batch prediction** — For high-throughput, latency-tolerant inference. Submits data, receives results asynchronously.
- **Model monitoring** — Detects feature skew (training/serving data distribution mismatch) and prediction drift (model output distribution changing over time).

### Feature Store

A centralized repository for ML features that serves both training (historical feature values via point-in-time lookups) and serving (low-latency feature retrieval). Key concepts:

- **Feature freshness** — How recently feature values were updated. Fresh features improve prediction quality but require more frequent computation.
- **Point-in-time correctness** — Training data must use feature values as they existed at the time of each training example, avoiding temporal leakage where future information contaminates historical features.
- **Online vs offline serving** — Online serving optimizes for latency (milliseconds); offline serving optimizes for throughput (batch retrieval for training data generation).

### Pipelines

Vertex AI Pipelines orchestrate ML workflows using a container-based execution model. Each pipeline step runs in an isolated container, enabling reproducibility and caching of intermediate results. Pipelines capture lineage — tracking which data, code, and parameters produced each model artifact.

## Data Catalog: Metadata Management

Data Catalog provides a centralized inventory of data assets across GCP services. It addresses the discovery problem: in organizations with many datasets across many projects, finding relevant data becomes a challenge.

Concepts:

- **Automatic discovery** — BigQuery datasets, Pub/Sub topics, and other GCP resources are automatically registered
- **Custom entries** — Non-GCP data assets can be manually registered to create a unified catalog
- **Tag templates** — Define structured metadata schemas (data owner, PII classification, freshness SLA) that can be attached to any catalog entry
- **Search** — Full-text and faceted search across all cataloged assets
- **Policy tags** — Integration with BigQuery column-level security for fine-grained access control based on data classification

The organizational value of metadata management scales with data estate size. Small environments can rely on tribal knowledge; larger environments need systematic cataloging to prevent data silos and redundant computation.

## Dataform: SQL-Based Transformation

Dataform provides a framework for managing SQL-based data transformations in BigQuery, addressing the gap between raw ingested data and analysis-ready tables.

Core concepts:

- **SQLX files** — SQL with extensions for dependency declaration, assertions, and incremental processing
- **Dependency graph** — Transformations declare their dependencies explicitly, enabling Dataform to determine execution order and parallelize independent transforms
- **Incremental tables** — Process only new/changed data rather than recomputing the entire table, reducing cost and execution time for large datasets
- **Assertions** — Data quality checks (uniqueness, non-null, referential integrity) embedded alongside transformations. Failures can block downstream processing.
- **Environments** — Separate development, staging, and production execution contexts using schema/dataset suffixes

The conceptual pattern is **ELT** (Extract, Load, Transform) — data is loaded into BigQuery first, then transformed using SQL within the warehouse, rather than transforming data before loading (traditional ETL).

## The Data Lifecycle

Data services compose along a lifecycle path. Understanding this path helps in selecting and integrating services:

```
┌──────────┐    ┌──────────┐    ┌────────────┐    ┌──────────┐    ┌──────────┐
│ Ingestion│───→│ Storage  │───→│ Processing │───→│ Analysis │───→│ Serving  │
└──────────┘    └──────────┘    └────────────┘    └──────────┘    └──────────┘
     │               │               │                │               │
  Pub/Sub        Cloud Storage    Dataflow          BigQuery       Vertex AI
  IoT Core       BigQuery        Dataproc          Looker         Endpoints
  Transfer       Bigtable        Dataform          Data Studio    Cloud Run
  Service        Spanner         Cloud Composer                   Functions
  Streaming      Firestore       (orchestration)
  inserts
```

### Ingestion Patterns

| Pattern                            | Latency            | Throughput        | Complexity                                   |
| ---------------------------------- | ------------------ | ----------------- | -------------------------------------------- |
| Streaming (Pub/Sub → sink)         | Seconds            | High, sustained   | Moderate — requires handling ordering, dedup |
| Micro-batch (periodic small loads) | Minutes            | Moderate          | Low — simpler error handling                 |
| Batch (scheduled bulk transfer)    | Hours              | Very high per run | Low — simple but delayed                     |
| Change data capture (CDC)          | Seconds to minutes | Variable          | High — schema evolution, ordering            |

The choice depends on how fresh downstream consumers need data to be versus the engineering complexity budget. Real-time ingestion sounds appealing but adds complexity in error handling, ordering, and exactly-once guarantees.

### Storage Tier Selection

Different storage services optimize for different access patterns:

| Service       | Model                            | Strength                                | Consideration                                   |
| ------------- | -------------------------------- | --------------------------------------- | ----------------------------------------------- |
| Cloud Storage | Object/blob                      | Unstructured data, data lake foundation | Not queryable directly without external compute |
| BigQuery      | Columnar analytical              | SQL analytics, ad-hoc queries           | Append-optimized, DML has higher overhead       |
| Bigtable      | Wide-column, sorted key-value    | High-throughput read/write, time-series | Requires careful key design to avoid hotspots   |
| Spanner       | Relational, globally distributed | Strong consistency, horizontal scaling  | Higher cost, operational complexity             |
| Firestore     | Document                         | Application state, hierarchical data    | Designed for application-tier, not analytics    |

Data often flows between storage tiers: raw data lands in Cloud Storage, structured data loads into BigQuery for analytics, hot application data lives in Firestore or Bigtable, and aggregated results serve from Memorystore or application caches.

### Processing Model Selection

```
Decision Factors:

  Latency requirement?
    ├── Real-time (seconds) ───→ Dataflow streaming, Bigtable
    ├── Near-real-time (minutes) ───→ Dataflow, Spark Streaming
    └── Batch (hours) ───→ Dataflow batch, Dataproc, BigQuery scheduled queries

  Programming model?
    ├── SQL-centric ───→ BigQuery, Dataform
    ├── Beam/pipeline SDK ───→ Dataflow
    └── Spark/Hadoop ecosystem ───→ Dataproc

  Schema complexity?
    ├── Structured, stable ───→ BigQuery, Dataform
    ├── Semi-structured, evolving ───→ Dataflow with schema handling
    └── Unstructured ───→ Dataflow + custom transforms, Vertex AI
```

## Cost Optimization for Analytical Workloads

Cost management in data services involves architectural decisions, not just resource tuning:

### Storage Costs

- **Lifecycle policies** — Automatically transition Cloud Storage objects from Standard to Nearline to Coldline to Archive based on access patterns
- **Long-term storage pricing** — BigQuery tables not modified for 90 days shift to lower storage pricing automatically
- **Compression and format** — Columnar formats (Parquet, ORC) in Cloud Storage reduce both storage cost and query cost when accessed by BigQuery external tables or Dataflow
- **Partition expiration** — Automatically drop old partitions to prevent unbounded storage growth

### Compute Costs

- **Slot reservations vs on-demand** — For predictable BigQuery workloads, capacity pricing can be significantly cheaper than on-demand when utilization is high. For unpredictable workloads, on-demand avoids paying for idle capacity.
- **Ephemeral clusters** — Spinning Dataproc clusters for jobs rather than running persistent clusters avoids paying for idle time between jobs
- **Preemptible/spot workers** — Secondary Dataproc workers using spot VMs reduce cost by 60-90% but can be reclaimed, requiring fault-tolerant job design
- **Materialized views** — Pre-computed results reduce repeated scan costs in BigQuery when the same aggregations are queried frequently

### Query Cost Management

- **Partition and cluster pruning** — Well-designed table schemas dramatically reduce bytes scanned. A time-partitioned, clustered table can reduce scan cost by orders of magnitude compared to unpartitioned.
- **Dry runs** — BigQuery dry runs estimate bytes scanned without executing, enabling cost checks before running expensive queries.
- **Query result caching** — Identical queries within 24 hours (same query text, same table data) return cached results at no cost.
- **Authorized views** — Provide controlled access to subsets of data without materializing copies, reducing storage duplication.

### Data Movement Costs

- **Ingress is free; egress is not** — Data entering GCP incurs no charge, but cross-region and internet egress carries per-GB costs. Architecture choices that minimize cross-region data movement directly reduce cost.
- **Co-location** — Processing and storage in the same region eliminates inter-region transfer costs. BigQuery datasets, Cloud Storage buckets, and Dataflow regional endpoints should align.
- **Transfer vs transformation** — Moving raw data and transforming at destination (ELT) may cost less in data transfer than transforming at source and moving refined data (ETL), depending on transformation ratios and egress pricing.

The fundamental cost optimization principle: minimize data movement and data scanned. Architecture decisions — where data lives, how it's partitioned, which processing model runs where — determine cost structure more than operational tuning of individual services.
