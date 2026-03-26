# ETL/ELT Patterns — Data Extraction, Transformation & Loading

## The ETL vs ELT Spectrum

ETL (Extract-Transform-Load) and ELT (Extract-Load-Transform) represent two philosophies for moving data from operational systems into analytical stores. The distinction centers on _where_ transformation occurs.

| Aspect             | ETL                                               | ELT                                        |
| ------------------ | ------------------------------------------------- | ------------------------------------------ |
| Transform location | Dedicated processing layer before loading         | Inside the target data store after loading |
| Raw data retention | Often discarded after transformation              | Preserved in landing zone                  |
| Compute model      | Separate transform infrastructure                 | Leverages target warehouse compute         |
| Schema flexibility | Schema defined before load                        | Schema can evolve post-load                |
| Debugging          | Harder — raw data may not persist                 | Easier — raw data available for replay     |
| Network cost       | Reduced — only transformed data shipped to target | Higher — raw data shipped, then processed  |

In practice, most pipelines blend both: light transformations during extraction (type coercion, PII masking) with heavier reshaping after loading. The "vs" framing overstates the dichotomy.

The ELT pattern gained traction as cloud warehouses decoupled storage from compute, making it economical to land raw data first and transform on demand. This is less about ELT being "better" and more about infrastructure cost curves shifting.

## Extraction Patterns

### Full vs Incremental Extraction

Full extraction reloads entire datasets on each run. Incremental extraction captures only what changed since the last run.

| Strategy                    | Suited for                                                    | Risks                                           |
| --------------------------- | ------------------------------------------------------------- | ----------------------------------------------- |
| Full extract                | Small tables, reference data, sources lacking change tracking | Expensive at scale, unnecessary load on source  |
| Timestamp-based incremental | Tables with reliable `updated_at` columns                     | Misses deletes, depends on clock accuracy       |
| Sequence-based incremental  | Tables with monotonic IDs or sequence numbers                 | Only captures inserts, not updates              |
| Change Data Capture (CDC)   | High-volume OLTP systems                                      | Infrastructure complexity, log retention limits |
| Snapshot diffing            | Systems without change metadata                               | Computationally expensive comparison step       |

### Change Data Capture (CDC)

CDC reads database transaction logs (write-ahead logs, binlogs, redo logs) to detect inserts, updates, and deletes as they occur. This provides a near-real-time change feed without querying the source tables directly.

Advantages of log-based CDC:

- Minimal impact on source database performance
- Captures deletes (which timestamp-based approaches miss)
- Preserves operation ordering

Challenges:

- Requires database-level permissions and configuration
- Transaction log format varies across database engines
- Log retention windows limit how far back changes can be recovered
- Schema changes in the source can break downstream consumers

### Event Sourcing as Extraction

When source systems use event sourcing natively, the event log itself becomes the extraction source. Every state change exists as an immutable event, making the "extraction" phase a matter of consuming the event stream rather than querying current state.

This collapses the extraction and transport layers — events flow directly to consumers. The trade-off is coupling to the source system's event schema and dealing with event versioning over time.

## Transformation Concepts

Transformations reshape data from source-oriented structures to analysis-oriented structures. Common categories:

### Structural Transformations

- **Mapping** — Renaming fields, changing types, restructuring nested data into flat rows
- **Filtering** — Removing irrelevant records (test accounts, internal traffic, incomplete entries)
- **Pivoting/Unpivoting** — Converting between wide and long formats depending on query patterns
- **Joining** — Combining data from multiple sources using shared keys

### Data Quality Transformations

- **Deduplication** — Identifying and resolving duplicate records. Approaches range from exact-match on primary keys to fuzzy matching on name/address combinations. Deduplication logic can be surprisingly domain-specific — what constitutes a "duplicate" varies by business context.
- **Standardization** — Normalizing formats (dates, phone numbers, addresses, currency codes)
- **Enrichment** — Adding derived fields (geocoding addresses, classifying text, computing aggregates)
- **Null handling** — Deciding per-field whether nulls propagate, get default values, or trigger record rejection

### Aggregation

Aggregation reduces granularity — rolling individual transactions into daily summaries, compressing clickstreams into session metrics. The level of aggregation is a trade-off:

- More aggregation → faster queries, less storage, loss of detail
- Less aggregation → slower queries, more storage, full detail preserved

Pre-aggregation decisions made during transformation are difficult to reverse. A common pattern stores both granular and aggregated forms, accepting the storage cost for flexibility.

## Schema-on-Read vs Schema-on-Write

| Philosophy      | When it tends to work well                                                                             | When it creates friction                                                                       |
| --------------- | ------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------- |
| Schema-on-write | Consumers need consistent structure, data quality is critical, downstream systems are schema-sensitive | Rapid iteration on data formats, exploratory analysis, source schemas change frequently        |
| Schema-on-read  | Exploratory workloads, schema evolves rapidly, raw data has unknown future uses                        | Consumers disagree on interpretation, data quality issues surface late, repeated parsing costs |

Schema-on-write enforces structure at load time — malformed data is rejected or corrected before entering the warehouse. This shifts quality problems upstream but can create pipeline fragility when sources change.

Schema-on-read accepts data as-is and interprets structure at query time. This offers flexibility but pushes data quality concerns to every consumer independently, potentially leading to inconsistent interpretations across teams.

Many systems layer both: raw data lands schema-on-read, then curated tables enforce schema-on-write for production reporting.

## Idempotency in Data Pipelines

An idempotent pipeline produces the same result whether run once or multiple times for the same input window. This property is critical because pipelines fail and must be retried.

### Why Idempotency Matters

Without idempotency, retrying a failed pipeline risks:

- Duplicate records in the target
- Double-counted metrics
- Corrupted aggregations
- Inconsistent state across dependent tables

### Approaches to Idempotency

**Partition-based overwrite** — Each run writes to a date-partitioned segment. Reruns overwrite the entire partition, guaranteeing consistent state regardless of how many times the job runs.

```
-- Conceptual: Replace entire partition on each run
DELETE FROM target WHERE date_partition = '2025-03-25';
INSERT INTO target SELECT * FROM staged WHERE date_partition = '2025-03-25';
```

**Merge/Upsert** — Match on primary key and insert-or-update. Works when source provides natural keys. Sensitive to ordering if multiple updates exist for the same key within a batch.

**Deduplication at load** — Accept potential duplicates during extraction, then deduplicate during transformation using row hashing or event IDs. Requires a reliable unique identifier per source record.

**Write-ahead tracking** — Track which source offsets or timestamps have been processed. Only process unprocessed ranges. Requires reliable watermarking infrastructure.

Each approach trades off between complexity, performance, and correctness guarantees.

## Backfilling and Reprocessing

Backfilling reconstructs historical data — filling gaps from pipeline failures, onboarding new data sources retroactively, or reprocessing after transformation logic changes.

### Patterns

- **Timeline replay** — Rerun the pipeline for each historical time window sequentially. Simple but slow for deep history.
- **Bulk parallel backfill** — Launch parallel jobs across historical partitions. Fast but resource-intensive and can stress source systems.
- **Snapshot restoration** — If raw data is preserved, reload from snapshots and retransform. Requires raw data retention policy.

### Challenges

- Source systems may not support historical queries (APIs with no `since` parameter, databases without point-in-time recovery)
- Transformation logic changes mean old and new data were processed with different rules — introducing potential discontinuities
- Backfills at scale can overwhelm downstream systems that expect incremental loads
- Dependency ordering — if table B depends on table A, backfilling B requires A to be backfilled first

## Data Quality Checks as Pipeline Stages

Embedding quality checks directly into the pipeline — rather than as a separate post-hoc process — allows failures to halt propagation of bad data.

### Categories of Checks

| Check type            | Examples                                                                  | When it fires                      |
| --------------------- | ------------------------------------------------------------------------- | ---------------------------------- |
| Schema validation     | Expected columns present, types match, no unexpected nulls                | After extraction                   |
| Volume checks         | Row count within expected range, not empty, not suspiciously large        | After extraction or transformation |
| Freshness checks      | Source data is recent enough, timestamps are not stale                    | Before processing begins           |
| Statistical checks    | Value distributions within expected bounds, no sudden spikes/drops        | After transformation               |
| Referential integrity | Foreign keys resolve, join fan-out within expected range                  | After joining/loading              |
| Business rules        | Revenue not negative, dates not in the future, status values in valid set | After transformation               |

### Handling Failures

Quality check failures present a routing decision:

- **Hard fail** — Stop the pipeline, alert, require manual intervention. Appropriate when downstream consumers cannot tolerate bad data.
- **Quarantine** — Route failing records to a dead-letter table for investigation. Pipeline continues with clean records.
- **Warn and proceed** — Log the anomaly but continue. Appropriate for advisory checks where the issue may be legitimate.

The choice depends on the cost of bad data reaching consumers vs. the cost of pipeline delays.

## Orchestration Concepts

### DAGs as Workflow Representations

Directed Acyclic Graphs (DAGs) model pipeline stages as nodes and dependencies as edges. A task runs only when all its upstream dependencies have succeeded.

```
extract_orders ──→ transform_orders ──→ load_orders
                                           ↓
extract_products ─→ transform_products ─→ build_summary
```

DAG-based orchestration provides:

- Dependency management — tasks execute in correct order
- Parallelism — independent branches run concurrently
- Failure isolation — one branch failing doesn't block unrelated branches
- Retry granularity — individual tasks can be retried without rerunning the entire pipeline

### Dependency Management Patterns

- **Temporal dependencies** — "Run after midnight UTC" — time-based scheduling
- **Data dependencies** — "Run when the upstream partition is available" — sensor/trigger patterns
- **External dependencies** — "Run when the vendor file lands in the SFTP server" — event-driven triggers

### Retry Semantics

Retry strategies balance between recovering from transient failures and avoiding infinite loops:

- **Fixed retry** — Wait N seconds, try again, up to M attempts
- **Exponential backoff** — Increase delay between retries to reduce pressure on failing systems
- **Per-task retry limits** — Different tasks tolerate different retry counts (a slow API call might retry 5 times; a transformation step might retry once)
- **Upstream cascade** — When a task fails after all retries, optionally trigger upstream reruns in case the input was the problem

## Slowly Changing Dimensions (SCD)

Dimension tables (customers, products, locations) change over time. How these changes are captured affects historical query accuracy.

| SCD Type | Approach                                                          | Trade-off                                                               |
| -------- | ----------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Type 0   | Never update — retain original value                              | Simple, but dimension data becomes stale                                |
| Type 1   | Overwrite with current value                                      | Simple, but loses history                                               |
| Type 2   | Add new row with version tracking (effective dates, current flag) | Preserves full history, but table grows and queries become more complex |
| Type 3   | Add columns for previous and current values                       | Limited history (usually just one prior version), simple queries        |
| Type 4   | Separate current and historical tables                            | Clean separation, but requires join logic                               |
| Type 6   | Hybrid of Types 1, 2, and 3                                       | Most flexible, most complex                                             |

Type 2 is the most common choice when historical accuracy matters, but the query complexity cost is real — every join to a Type 2 dimension must include date-range filtering to get the correct version.

## The ELT Shift and Warehouse-Side Transformation

The migration from ETL to ELT reflects changes in infrastructure economics:

- Cloud warehouses separate storage and compute, making it cheap to store raw data and spin up compute on demand
- Transformations expressed as SQL run inside the warehouse using its optimizer, often outperforming external transformation engines
- Raw data preservation enables reprocessing without re-extracting from source systems

This shifts the pipeline from:

```
Source → Transform Engine → Warehouse (curated)
```

To:

```
Source → Landing Zone (raw) → Warehouse SQL (transform) → Curated Layer
```

The trade-off: warehouse compute costs scale with transformation complexity. Heavy transformations on massive datasets can generate significant compute bills. The cost equation depends on the specific warehouse pricing model and data volumes.

## Pipeline Observability

### Data Lineage

Lineage tracks the provenance of data — which sources fed which tables, through which transformations. This enables:

- Impact analysis — "If this source changes, what downstream reports are affected?"
- Root cause analysis — "This metric looks wrong — where did the bad data enter?"
- Compliance — "Can we prove this report only uses consented data?"

Lineage can be captured at different granularities: table-level (coarse but easy), column-level (precise but expensive to track), or row-level (used in regulated industries).

### Freshness Monitoring

Freshness checks answer "how old is the data in this table?" by comparing the latest record timestamp or pipeline run time against expectations. Stale data may indicate:

- Pipeline failure or delay
- Source system outage
- Backfill in progress (temporarily replacing fresh data with historical data)

### Anomaly Detection

Statistical monitoring of pipeline metrics — row counts, null rates, value distributions, processing times — can detect issues before they become visible in downstream reports. Approaches range from simple threshold alerts to statistical process control to ML-based anomaly detection.

The challenge is alert fatigue: overly sensitive anomaly detection generates noise that teams learn to ignore, defeating the purpose. Tuning detection sensitivity to the business impact of the data is an ongoing operational concern.

## Common Anti-Patterns and Their Contexts

| Pattern                      | Why it happens                                            | The resulting friction                                                      |
| ---------------------------- | --------------------------------------------------------- | --------------------------------------------------------------------------- |
| Monolithic pipeline          | Simpler to build initially                                | Single failure blocks everything; difficult to test components in isolation |
| No idempotency               | "It'll only run once" assumption                          | Retries corrupt data; manual cleanup required after failures                |
| Transformation in extraction | Blending concerns for convenience                         | Source coupling — extraction changes break transformations and vice versa   |
| Ignoring late-arriving data  | Pipeline assumes data arrives on time                     | Metrics silently drift as late data is missed                               |
| Over-aggregation early       | Performance optimization before understanding query needs | Granular data lost; new questions require re-extraction from source         |
| No raw data retention        | Storage cost concerns                                     | Inability to reprocess when transformation logic changes                    |

These patterns are not universally wrong — each emerges from a rational trade-off in specific contexts. A small team may reasonably choose a monolithic pipeline for simplicity, accepting the coupling cost.
