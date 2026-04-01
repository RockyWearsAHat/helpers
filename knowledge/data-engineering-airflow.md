# Apache Airflow — Workflow Orchestration and DAG Execution

## Overview

Apache Airflow is a platform for programmatically authoring, scheduling, and monitoring data workflows. Workflows are expressed as directed acyclic graphs (DAGs) of tasks, with explicit dependencies defining execution order. The Airflow scheduler executes tasks across a cluster of workers, retrying failures, managing state, and exposing a web UI for monitoring.

Airflow is best suited for **mostly static workflows** where task relationships don't change between runs. For highly dynamic pipelines or streaming workloads, other tools (Temporal, Prefect, Kafka Streams) may be more appropriate.

---

## Core Concepts

### DAG (Directed Acyclic Graph)

The fundamental abstraction: a collection of tasks with explicit dependencies forming a DAG. Each task runs once the tasks it depends on complete. The scheduler renders the DAG and executes tasks in topological order.

DAGs are defined in Python code, enabling dynamic composition (DAGs can generate themselves based on configuration) and version control. Every DAG is parameterized by execution date (the logical "as of" time for the run).

```python
# Pseudocode
from airflow import DAG
from airflow.operators import BashOperator, PythonOperator

with DAG("my_pipeline", schedule="@daily") as dag:
    task_a = BashOperator(task_id="extract", bash_command="...")
    task_b = PythonOperator(task_id="load", python_callable=func)
    task_a >> task_b  # Dependencies: task_a then task_b
```

### Tasks and Operators

A **task** is an instance of an **operator**. Operators are the reusable abstractions:

- **BashOperator**: Execute shell commands
- **PythonOperator**: Call Python functions; can return XCom
- **SqlOperator** (database-specific variants): Execute SQL against a database
- **HttpOperator**: Make HTTP requests
- **KubernetesPodOperator**: Launch Kubernetes pods
- **DummyOperator/NoOpOperator**: Placeholder for DAG structure without work
- **SensorOperator** subclasses: Wait for external conditions (file existence, database query result)

Custom operators extend a base class to wrap tools or internal services. The Airflow ecosystem includes 300+ providers with specialized operators.

### Scheduling

Airflow schedules DAG runs via cron-like expressions or interval strings:
- `"@daily"` — run once daily at midnight UTC
- `"0 2 * * *"` — run at 2 AM UTC daily
- `schedule_interval=timedelta(hours=1)` — run hourly
- `schedule_interval=None` — manual triggering only

The scheduler interprets a DAG concurrently based on **max_active_runs** and **max_active_tasks_per_dag** settings.

---

## Executor Types

The **executor** determines how and where tasks execute:

### LocalExecutor
Single-machine, multi-process executor. Tasks run as parallel processes on the scheduler's host. No remote workers. Suitable for development, small datasets, or single-node deployments with moderate parallelism.

### CeleryExecutor
Distributed executor using Celery (a task queue). Tasks are published to a message broker (RabbitMQ, Redis) and workers poll for work. Multiple workers can run on different machines, scaling horizontally. Requires external infrastructure (broker, workers) but handles high concurrency.

### KubernetesExecutor
Each task runs as a Kubernetes pod on a Kubernetes cluster. Pods are created on demand and cleaned up after completion. Provides strong isolation, automatic scaling, and cloud-native orchestration. Higher overhead per task than Celery but cleaner operations for teams already using Kubernetes.

### SequentialExecutor
Single-threaded; runs tasks sequentially. Useful for testing or constrained environments. No parallelism.

---

## Task Dependencies and Control Flow

Dependencies are expressed via operators:
- `task_a >> task_b` — task_b depends on task_a
- `task_a >> [task_b, task_c]` — task_b and task_c both depend on task_a
- `task_a << task_b` — bitwise reverse; equivalent to `task_b >> task_a`

Task states: **queued** → **running** → **success** or **failed** → optionally **upstream_failed** (if a dependency failed) or **skipped** .

By default, if a task fails, downstream tasks are not scheduled (marked upstream_failed). Conditional branching via **BranchOperator** allows dynamic task selection based on logic.

---

## XCom (Cross-Communication)

XCom allows tasks to exchange metadata (not bulk data):

```python
# Task A: Push a value
context['task_instance'].xcom_push(key='user_count', value=42)

# Task B: Pull it
count = context['task_instance'].xcom_pull(
    task_ids='task_a', key='user_count'
)
```

XCom rows are serialized (default: JSON, Pickle) and stored in the Airflow database. Limited by serialization overhead and database size. **Best practice**: pass filenames via XCom, not gigabytes of data. For data pipelines, persist results to object storage (S3, GCS) and reference paths in XCom.

---

## Connections and Variables

**Connections** store credentials for external systems (databases, APIs, cloud services). Defined in the UI or environment variables, they avoid hardcoding secrets.

**Variables** store configuration values (dataset paths, thresholds, feature flags). Also managed via UI or environment. Both are looked up at task runtime.

```python
from airflow.models import Connection, Variable

db_conn = Connection.get_connection_from_secrets('postgres_db')
data_dir = Variable.get('DATA_DIRECTORY')
```

---

## Airflow 2.x Improvements

Airflow 2.0+ introduced:
- **TaskFlow API**: Decorator-based DAG construction with automatic dependency inference and XCom handling
- **Provider plugins**: Separated operators into installable packages (e.g., `apache-airflow-providers-amazon`)
- **DAG serialization**: DAGs stored as JSON for easier auditioning and versioning
- **REST API**: Programmatic DAG/run management
- **Improved UI**: Better monitoring and debugging

---

## Dynamic DAGs

DAG structure can be generated at parse time (every time the scheduler parses the DAG file):

```python
with DAG("dynamic_pipeline") as dag:
    for dataset in config['datasets']:
        task = PythonOperator(
            task_id=f"process_{dataset}",
            python_callable=process_func,
            op_kwargs={'dataset': dataset}
        )
```

Every Airflow scheduler restart parses all DAG files, so dynamic generation scales with O(n) parse time. Excessive dynamic DAG generation (e.g., generating 10,000 tasks at parse time) can slow the scheduler. Better practice: keep DAG structure mostly static; parameterize via Variables or task config.

---

## Monitoring and Observability

The web UI displays:
- **DAG view**: Graph of task dependencies
- **Calendar view**: Historical success/failure dates
- **Gantt view**: Task execution timeline, showing parallelism and duration
- **Tree view**: Hierarchical task and run status

Logs for each task run are captured by the executor and stored in the backend (file system, S3, etc.). Failed tasks are visible with stack traces, making debugging straightforward.

The REST API enables programmatic querying of DAG status, triggering manual runs, and integrating with external systems.

---

## Architecture Considerations

Airflow has a single metadata database (PostgreSQL, MySQL) storing DAG definitions, run history, and XCom. This centralizes state but introduces a bottleneck. For 1000+ DAGs in a large organization, sharding or federation strategies become necessary.

Scaling concerns:
- **Parser bottleneck**: Too many DAGs cause slow scheduler parse cycles
- **Database bottleneck**: High-concurrency task updates can saturate database connections
- **Worker scale**: Use KubernetesExecutor or Celery with many workers; LocalExecutor maxes out at single machine

Airflow is not a stream processor. While it can poll streams (Kafka) and run mini-batch jobs frequently, streaming workloads are better served by Kafka Streams, Apache Flink, or Spark Structured Streaming.

---

## Integration with Data Tools

Airflow is commonly paired with:
- **dbt**: dbt models are run as tasks via `DbtRunOperator`; Airflow manages DAG scheduling
- **Spark**: Spark jobs submitted as tasks via `SparkSubmitOperator` or `SparkKubernetesOperator`
- **SQL warehouses**: Queries executed via `SqlOperator` variants for Redshift, BigQuery, Snowflake
- **Data catalogs**: Metadata registered to Unity Catalog, Hive Metastore, or OpenMetadata

Airflow excels at orchestrating many small to medium tasks with clear dependencies. For single large jobs (e.g., one Spark compute per day), simpler systems (cron + monitoring) may be overkill.