# Apache Spark — Distributed Computing Engine for Large-Scale Data Processing

## Overview

Apache Spark is a distributed computation engine designed for large-scale data processing across clusters. It emerged from research at UC Berkeley to overcome Apache Hadoop MapReduce's latency bottlenecks. While Hadoop optimizes for very large batch jobs by reading/writing to disk, Spark prioritizes in-memory computation for interactive queries, machine learning, and stream processing.

Spark abstracts cluster coordination and task parallelism, allowing developers to write transformations that look like single-machine operations but execute in parallel across hundreds of machines and thousands of CPU cores.

---

## Core Architecture

### Driver and Executors

A Spark application consists of a **driver program** and distributed **executors**:

- **Driver**: The JVM process running user code. It creates the `SparkContext/SparkSession` object, defines transformations on data, and coordinates task execution. The driver's main thread launches jobs in response to actions (like `collect()` or `write()`).
- **Executors**: Worker processes launched on cluster nodes. Each executor runs tasks in parallel threads, manages in-memory storage for RDDs/DataFrames, and communicates results back to the driver. Every application gets its own executor processes, isolating it from other applications.
- **Cluster Manager**: External service (Standalone, YARN, Kubernetes) that allocates executor resources to Spark applications.

Task scheduling occurs on the driver via a DAG scheduler, which translates user operations into a stage-wise physical plan. Executors receive task assignments and execute them in parallel.

### RDD vs DataFrame vs Dataset

**RDD (Resilient Distributed Dataset)** is the low-level abstraction:
- Collection of objects partitioned across the cluster
- Immutable; operations return new RDDs
- Supports arbitrary transformations via `map`, `flatMap`, `filter` (functional programming primitives)
- No built-in schema; the driver must track what data looks like
- Slower execution because optimizer cannot reason about data structure
- Appropriate for unstructured data, fine-grained control, or legacy code

**DataFrame**: A distributed table abstraction (conceptually equivalent to Pandas DataFrames or SQL tables):
- Rows organized into named columns with known types
- Immutable; transformations return new DataFrames
- API in Python, Scala, Java, SQL, and R
- Operations (`select`, `filter`, `groupBy`) look like SQL but compose as expressions
- Spark can optimize execution because structure is explicit
- Preferred modern API for structured data

**Dataset**: JVM-only type-safe abstraction combining RDD and DataFrame benefits:
- Scala/Java only (Python/R use dynamic typing, so DataFrames already provide Dataset ergonomics)
- Leverages static typing for compile-time safety
- Same optimized execution engine as DataFrames
- Rare in practice; use DataFrames unless you need JVM typing

---

## Query Optimization: Catalyst and Tungsten

### Catalyst Optimizer

Spark SQL translates DataFrames and SQL queries into an optimized execution plan:

1. **Parsing**: SQL/DataFrame expressions → abstract syntax tree (AST)
2. **Analysis**: Resolve column names, infer types, check for semantic errors
3. **Logical Optimization**: Apply rule-based transformations (predicate pushdown, constant folding, null propagation reduction)
4. **Physical Planning**: Convert logical plan into executable RDD operations; estimate costs
5. **Code Generation**: Generate JVM bytecode for the physical plan

Predicate pushdown is the most impactful optimization: filter conditions are pushed down to data source scans, reducing the data that flows through the pipeline. For example, `df.filter(col("year") == 2025).select("name")` will scan only 2025 partitions, not all partitions.

### Tungsten Execution Engine

Tungsten is Spark's low-level columnar execution layer that replaced RDD interpretation:

- **Off-heap memory management**: Allocates memory outside the JVM garbage collector, reducing GC pauses on large datasets
- **Column-oriented storage**: Data stored by column rather than row, improving CPU cache locality and enabling vectorized operations
- **Codegen**: Compiles physical plans directly to Java bytecode rather than interpreting object-at-a-time operations
- **SIMD operations**: Leverages CPU vector instructions for fast operations on batches of data

Result: 100x speedup vs. Hadoop MapReduce on in-memory workloads; 10x faster on disk.

---

## Data Abstractions and APIs

### Spark SQL

SQL interface for structured queries. Functionally equivalent to DataFrame API; same optimization engine applies. Useful for teams familiar with SQL, enabling a subset of Airflow/dbt users to write queries without learning a new DSL.

Spark SQL integrates with Hive metastores, allowing queries over existing warehouse tables and metadata.

### Structured Streaming

Real-time stream processing built on the Spark SQL engine. Conceptually: an unbounded DataFrame that grows as new data arrives. Streaming queries can use the same DataFrame operations as batch queries; Spark incrementally processes arriving micro-batches.

Supports event time windows, stateful operations, and exactly-once semantics. Often paired with Kafka or other event streams in data pipelines.

### MLlib and GraphX

- **MLlib**: Machine learning library with algorithms for classification, regression, clustering, and dimensionality reduction. High-level Pipelines API (feature engineering → model training) mirrors scikit-learn. Lower-level RDD-based APIs are discouraged.
- **GraphX**: Graph computation library for analyzing networks and relationships. Less commonly used than standalone graph engines like Neo4j.

---

## Partitioning and Performance Tuning

Data is divided into partitions (one per executor task). The number of partitions affects parallelism and network overhead:

- **Too few partitions**: Underutilizes cluster; large shuffles; slow
- **Too many partitions**: Task scheduling overhead; small shuffles; potential bottleneck on fan-out
- **Heuristic**: 1 task per core; scale partitions with data size and cluster size

Repartitioning via `repartition()` triggers a full shuffle (expensive). `coalesce()` reduces partitions without shuffling (removes empty partitions). Bucketing organizes data by hash values to optimize joins.

**Shuffle** is the most expensive operation: data is redistributed across the cluster to group keys. Minimizing shuffles (via early filtering, bloom filters) is a primary tuning lever.

---

## Execution Model: Lazy Evaluation

Spark uses **lazy evaluation**: transformations (`map`, `filter`, `join`) are not executed immediately; they build up a DAG of operations. Execution occurs only when an **action** is called (`collect()`, `count()`, `write()`, `show()`).

This allows Spark to optimize the entire pipeline before execution rather than optimizing each step independently.

---

## Deployment Models

| Mode | Driver | Executors | Use |
|------|--------|-----------|-----|
| **Local** | Same JVM as executors | Local threads | Development/testing |
| **Standalone** | Cluster node | Cluster nodes | Simple clusters without Kubernetes/YARN |
| **YARN** | Hadoop resource manager | Hadoop worker nodes | Legacy Hadoop environments |
| **Kubernetes** | Kubernetes pod | Kubernetes pods | Modern cloud-native deployments |

Deploy mode (client vs. cluster) controls where the driver runs. Client mode runs the driver on the submitter's machine; cluster mode runs it on a cluster node.

---

## Integration with Data Ecosystems

Spark integrates with Delta Lake (ACID transactions on object storage), Apache Iceberg (multi-engine table format), and Apache Hudi (incremental processing). These enable lakehouse architectures: the efficiency of lakes with warehouse transactionality.

Spark also connects to SQL warehouses (Redshift, BigQuery), data catalogs (Unity Catalog, Hive Metastore), and orchestration tools (Airflow, dbt).

---

## Tradeoffs

**Strengths**: In-memory speed, multiple APIs (SQL, Python, Scala, R), ecosystem maturity, fault tolerance, excellent for iterative workloads.

**Weaknesses**: High memory overhead; not ideal for inherently sequential algorithms; requires cluster infrastructure; steep learning curve for optimization; less suitable for single-machine analysis than pandas.

Spark is best for **distributed batch** and **stream** compute on structured data at scale. For small data, single-machine tools (pandas, DuckDB) are faster and simpler. For specialized workloads (graph, ML), domain-specific engines may be more efficient.