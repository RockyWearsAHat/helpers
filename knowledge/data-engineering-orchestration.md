# Data Pipeline Orchestration — Scheduling, State Management & Lineage

## Overview

**Data orchestration** automates the execution, scheduling, and monitoring of data transformation pipelines. At its core it solves: *How do I define a complex pipeline, schedule it reliably, track what data flows where, handle failures, and understand what's running?*

The orchestration layer sits between raw compute (Spark, dbt, SQL) and data storage (lake, warehouse, message queue). It coordinates task dependencies, retries failed steps, maintains audit trails, and exposes data lineage for governance.

The field has evolved from simple cron + shell scripts → Azkban/Oozie → Airflow (still dominant) → modern alternatives emphasizing asset/data-first models (Dagster, Prefect), triggering frameworks (Temporal), and specialized data workflows (Kafka Streams, Flink).

---

## Scheduling & Execution Models

### Cron-Based: Time-Triggered

Traditional approach: run jobs on schedule (daily 2am, hourly catch any new data).

**Strengths**:
- Simple; scales to many jobs via standard OS cron

**Weaknesses**:
- No dependency tracking; jobs may start before upstream completes
- Hard to distribute across many machines or cloud environments
- No visibility into what succeeded/failed; only logs
- Fragile retry logic (exponential backoff in shell is error-prone)

### DAG-Based: Task Dependency Graphs

Modern standard (Airflow, Oozie): define workflows as directed acyclic graphs where:
- **Nodes** = tasks/jobs (Spark job, SQL query, Python script)
- **Edges** = dependencies (task_B runs only after task_A succeeds)
- **Scheduler** = renders graph, executes nodes in topological order across workers

**Strengths**:
- Explicit dependency tracking; scheduler enforces order
- Distributable: tasks farmed to executors (local, Kubernetes, cloud)
- Monitoring: dashboard shows task status, retries, SLA violations
- Fault tolerance: scheduler can rerun failed nodes; idempotent tasks are re-executable

**Weaknesses**:
- DAG must be static (no dynamic task generation mid-run, though workarounds exist)
- Tasks are imperative (shell, Python, SQL) — no strong contract about what data they produce
- Lineage tracking requires manual documentation or post-hoc inference

### Asset-Based: Data-Centric

Newer paradigm (Dagster, some Prefect workflows): define pipelines as transformations between **assets** (tables, files, ML models) not *tasks*.

```
Pseudocode (Dagster):
@asset
def customers(bank_db):
    return load_table(bank_db, 'customers')

@asset
def customers_clean(customers):
    return deduplicate(customers)

@asset
def customer_segments(customers_clean):
    return kmeans(customers_clean, k=5)
```

**Strengths**:
- Explicit data contracts: each asset declares its upstream dependencies
- Scheduler infers dependencies from asset graph; same ordering but clearer semantics
- Lineage is intrinsic: query "what feeds monthly_revenue?" directly from code
- Incremental runs: system can skip already-computed assets or re-materialize specific ones
- Type checking: asset outputs/inputs have schemas; type mismatches caught at definition time

**Weaknesses**:
- Requires paradigm shift from jobs to assets; legacy task-heavy pipelines harder to port
- Not all computations map neatly to assets (side effects, notifications, fan-outs)
- Distributed execution still requires explicit partitioning logic

---

## Airflow Architecture

Apache Airflow is the dominant open-source orchestrator. Understanding its design illuminates trade-offs:

### Components

**Scheduler**:
- Central daemon; reads DAG definitions from `dags/` folder
- Periodically parses DAGs to detect new tasks, changed dependencies, or newly-triggered interval schedules
- Maintains task database (PostgreSQL, MySQL); stores task instances and execution history
- Assigns tasks to executors when available; monitors completion; logs metrics

**DAG Parser**:
- Lightweight process; scans Python DAG files (typically 1000s of lines of imperative code)
- Must complete in seconds; slow DAGs create scheduler bottleneck
- Creates task instances for each schedule interval (e.g., hourly for past 30 days = 30 tasks)

**Executor**:
- Pluggable component handling task execution
- **LocalExecutor**: squential on same machine (dev/CI only; not HA)
- **CeleryExecutor**: distributed; pushes tasks to message queue (RabbitMQ, Redis); workers pull and execute
- **KubernetesExecutor**: each task runs in a new pod; compute isolation; auto-scales (popular for cloud)
- **SequentialExecutor**: one task at a time; deterministic but slow

**WebUI**:
- Dashboard showing DAG runs, task logs, SLA violations, backfill history
- Manual trigger interface; useful for re-running failed runs

**Database**:
- Stores DAG definitions (metadata), task instances (run history, state), connections/credentials, XComs (task-to-task message passing)
- Bottleneck at scale (100k+ task instances create query pressure)

### Execution Model: Imperative

```python
with DAG('daily_etl, default_args=defaults) as dag:
    extract = BashOperator(task_id='extract', bash_command='spark-submit extract.py')
    transform = SparkSQLOperator(task_id='transform', sql='SELECT ...', table='staging')
    load = PostgresOperator(task_id='load', sql='INSERT INTO fact ...')
    
    extract >> transform >> load
```

**Semantics**:
- DAG defined once; scheduler instantiates tasks for each schedule interval
- Tasks are **imperative**: shell commands, SQL statements, or Python code
- No strong contract on what each task produces; downstream must know (fragile)
- **XCom passing**: tasks can exchange JSON/pickle via `task.xcom_push()` / `task.xcom_pull()` (lightweight message bus)

### Failure Handling

**Retry logic**:
- Failed tasks retry up to N times with exponential backoff (configurable)
- Entire DAG doesn't restart; only failed task retries
- If a task retries exhausted, DAG run marked as failed

**Backfill**:
- Retrospectively run a DAG for past dates/intervals
- Example: DAG scheduled daily but added new transformation; backfill last 30 days to populate
- `airflow dags backfill -s 2025-01-01 -e 2025-03-01 my_dag`

**SLA & Alerting**:
- Tasks can have SLA (e.g., max_execution_time, timeout)
- SLA violations trigger alerts (email, Slack)
- Monitoring plugins hook into failure callbacks

### Scalability Challenges

- **DAG parsing**: 1000s of Python files; parsing every scheduler loop (default 30s) is expensive
- **Database**: task instance table grows linearly with run history; queries slow
- **Executor saturation**: if Celery queue backs up, new tasks wait; poor predictability
- **No dynamic DAGs**: can't spawn tasks mid-run based on data (old Airflow; newer versions have dynamic task mapping)

---

## Dagster: Asset-Centric Orchestration

Dagster inverts the model: define **assets** and let the system infer orchestration.

### Core Concepts

**Asset**:
- Definition of a data object: a table, file, or ML model
- **IO**: input assets (upstream dependencies) and output asset (this table)
- **Computation**: Python function computing the asset from inputs
- **Metadata**: schema, freshness policy, owner, tags

```python
@asset
def staging_orders(raw_orders_bucket: S3Resource) -> pd.DataFrame:
    """Loads raw orders, applies basic validation."""
    df = raw_orders_bucket.read_parquet('orders.parquet')
    return df[df['amount'] > 0]

@asset
def orders_mart(staging_orders) -> pd.DataFrame:
    """Models orders into facts and dimensions."""
    return staging_orders.groupby('customer_id').agg(...)
```

**Asset Graph**:
- Directed edges: inputs → outputs
- Scheduler auto-generates execution order (topological sort)
- Same guarantee as Airflow DAG but clearer semantics

**Partitioning**:
- Assets can be **partitioned** (time-partitioned: daily, monthly; or dimension: by region, customer)
- Orchestrator tracks which partitions are stale/missing
- Backfill: recompute specific partitions in parallel

**Freshness Policy**:
- Define SLA: "orders_mart must be ≤ 1 hour old"
- System checks asset age; marks stale; triggers re-runs
- Different from simple schedule; actual freshness-driven

**Sensors**:
- Trigger assets on external events (new file in S3, message in Kafka, HTTP request)
- More flexible than fixed schedules
- Exactly-once semantics: don't re-trigger if asset already fresh

**Jobs**:
- Group assets for coordinated execution
- Different jobs = different schedules (daily revenue job, hourly click job)

### Execution & Materialization

**Materialization**:
- Running an asset's computation; producing its output data
- Dagster records: timestamp, compute duration, inputs consumed, lineage

**Selective Materialization**:
- Recompute only stale or newly-changed assets
- Avoids recomputing expensive assets if inputs unchanged

**Versioning & Caching**:
- Assets can declare version pinning: "use v2 of customers even if v3 is available"
- Pre-computed assets linkable to prevent redundant computation

### Lineage & Data Awareness

**First-class lineage**:
- Every asset tracks its upstream dependencies explicitly
- UI shows: "revenue_mart depends on orders_mart (partitioned by date, updated hourly)"
- Easy to trace impact of upstream changes

**Data Contracts**:
- Assets define output schema; violations caught on write
- Example: `staging_orders` must have columns [id, amount, customer_id]; runtime check fails if missing
- Prevents silent data quality regressions

---

## Other Platforms

### Prefect

Combines simplicity with dynamic task generation:
```python
@flow
def my_pipeline():
    orders = extract()
    for region in ['US', 'EU', 'APAC']:
        transform(orders, region)  # Dynamic: 3 tasks spawned at runtime
    load_all()
```

**Strengths**:
- Pythonic; easier learning curve than Airflow
- Dynamic task generation (generate tasks mid-run based on data)
- Built-in retry/caching; less boilerplate

**Trade-offs**:
- Smaller ecosystem; fewer integrations than Airflow
- Managed offering (Prefect Cloud) tightly integrated

### Apache Temporal

Purpose-built for **long-running, distributed workflows** with state machine semantics:
```java
@WorkflowMethod
public void dataRetentionWorkflow(String jobId) {
    // Long-running: weeks of retries, manual interventions, reminders
    archiveOldData(jobId);
    scheduleNextWeek();
}
```

**Use case**: Data retention policies, ETL with manual review steps, multi-day workflows with human intervention

**Strengths**:
- Excellent for workflows spanning days/weeks (not just minute-scale batches)
- Built-in durable state; survives restarts
- Temporal locality: related tasks run on same worker (lower latency)

**Weaknesses**:
- Steeper learning curve; not purely data-driven
- Smaller community than Airflow or Dagster

---

## Idempotency & Retry Semantics

A critical requirement: **pipelines must be idempotent** (running twice = running once).

### Why Idempotency Matters

In large scale pipelines with hundreds of tasks, failures are common:
- Task crashes due to OOM or timeout
- Network hiccup during write
- Upstream data incomplete but partial results written
- Human re-runs a failed task

If a task retries and writes duplicate rows or overwrites correct data incorrectly, cascading corruption.

### Patterns

**1. Upsert with primary key**:
```sql
INSERT INTO customers VALUES (1, 'Alice')
ON DUPLICATE KEY UPDATE name='Alice'
```
Rerunning produces same state.

**2. Partition overwrite**:
```python
# Write to temp; rename into place only after all data loaded
df.write.mode('overwrite').partitionBy('date').save('/tmp/new_partition')
mv('/tmp/new_partition', '/data/date=2025-03-01')  # Atomic rename
```

**3. External state version tracking**:
```python
# Only process if we haven't seen this batch before
batch_id = extract_batch_id(source)
if batch_id > last_processed_batch_id:
    process(batch_id)
    save_state(batch_id)
```

### Orchestrator Support

- **Airflow**: Idempotency is task implementer's responsibility; no framework primitive
- **Dagster**: Emphasizes idempotent assets; tracks output versions; re-materialization is safe
- **Temporal**: Durable execution + idempotency key = exactly-once semantics

---

## Data Lineage & Observability

### Lineage Capture

**Intrinsic** (Dagster, modern Airflow):
- Asset graph or DAG definition contains lineage
- "revenue" depends on "orders" and "customers" — metadata is in code

**Post-hoc** (log analysis):
- Parse task logs / query execution plans
- Infer: this SQL query read table X and wrote table Y
- Less reliable; misses data flows outside the orchestrator (ad-hoc scripts, manual exports)

### Lineage Use Cases

- **Impact analysis**: "I'm dropping this column; what jobs will break?"
- **Freshness tracking**: "Is this table fresh? All upstream dependencies met?"
- **Compliance**: "How many steps does PII traverse?"
- **Debugging**: "Where did this data quality issue originate?"

---

## Operational Challenges & Trade-offs

| Aspect | Airflow | Dagster | Prefect | Temporal |
|--------|---------|---------|---------|----------|
| **Learning Curve** | Moderate | Moderate | Easy | Hard |
| **Scalability** | Good (proven at scale) | Growing | Good | Very Good |
| **Dynamic Tasks** | Limited | Full | Full | Full |
| **Lineage** | Implicit | Explicit | Implicit | Implicit |
| **Asset-First** | No (task-first) | Yes | No | No |
| **Community** | Largest; most tutorials | Growing fast | Medium | Niche (workflow-focused) |
| **Cloud Support** | All (multi-cloud) | All + managed | Managed (Prefect Cloud) | All |

---

## See Also

- [ETL/ELT Patterns](data-engineering-etl.md) — pipeline design and transformation
- [Data Quality](data-engineering-quality.md) — validation in orchestrated pipelines
- [Data Mesh](architecture-data-mesh.md) — orchestration at domain boundaries
- [Distributed Systems Concepts](systems-distributed-consensus.md) — understanding scheduler consensus