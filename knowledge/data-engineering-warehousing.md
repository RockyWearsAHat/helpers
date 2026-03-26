# Data Warehousing — Dimensional Modeling, OLAP & Analytical Architecture

## OLTP vs OLAP — Different Optimization Goals

Transactional (OLTP) and analytical (OLAP) workloads impose fundamentally different demands on storage engines, indexing strategies, and query planners.

| Characteristic | OLTP                                      | OLAP                                         |
| -------------- | ----------------------------------------- | -------------------------------------------- |
| Access pattern | Point lookups, small row ranges           | Full-table scans, large aggregations         |
| Write pattern  | High-frequency single-row inserts/updates | Bulk loads, append-mostly                    |
| Normalization  | Highly normalized (3NF+)                  | Denormalized or dimensional                  |
| Concurrency    | Many short transactions                   | Fewer long-running analytical queries        |
| Indexing       | B-tree on primary/foreign keys            | Columnar encoding, bitmap indexes, zone maps |
| Latency goal   | Sub-millisecond per transaction           | Seconds to minutes per query acceptable      |
| Data volume    | Operational window (recent)               | Historical depth (months to years)           |

Attempting to serve both workloads from a single system creates contention — long analytical scans compete with latency-sensitive transactional writes. This tension motivates separating operational stores from analytical stores, though the degree of separation varies by architecture.

Some systems blur the boundary by offering hybrid transactional/analytical processing (HTAP), maintaining row-oriented and columnar representations simultaneously. These reduce data movement latency at the cost of storage overhead and operational complexity.

## Dimensional Modeling — Star and Snowflake Schemas

Dimensional modeling organizes analytical data around business processes, distinguishing between measurable events (facts) and the descriptive context surrounding them (dimensions).

### Star Schema

The star schema places a central fact table surrounded by denormalized dimension tables, each joined via foreign keys:

```
        dim_customer
             |
dim_date --- fact_sales --- dim_product
             |
        dim_store
```

Advantages of the star shape include simpler queries (fewer joins), predictable performance characteristics, and intuitive mapping to business questions. The denormalization in dimension tables trades storage efficiency for query simplicity.

### Snowflake Schema

Snowflake schemas normalize dimension tables into sub-dimensions:

```
dim_date --- fact_sales --- dim_product --- dim_category --- dim_department
```

This reduces redundancy in dimensions but increases join complexity. The trade-off matters more or less depending on the query engine — some optimizers handle multi-level dimension joins efficiently, others degrade noticeably.

### Choosing Between Them

Contexts favoring star schemas: query simplicity is valued, dimension cardinality is moderate, end users write ad-hoc queries directly. Contexts favoring snowflake schemas: dimension tables are extremely large with high redundancy, storage costs matter, ETL processes benefit from normalized staging.

In practice, many warehouses use a hybrid — some dimensions denormalized, others partially normalized where it reduces maintenance burden without materially affecting query performance.

## Fact Tables and Dimension Tables

### Grain

The grain of a fact table defines what one row represents — a single transaction, a daily summary, a session event. Grain decisions cascade through the entire model:

- **Transaction grain**: one row per atomic event (e.g., one line item in an order). Maximum flexibility for aggregation but highest row count.
- **Periodic snapshot grain**: one row per entity per time period (e.g., daily account balance). Pre-aggregated, reducing query cost at the expense of detail.
- **Accumulating snapshot grain**: one row per process instance tracking milestones (e.g., order lifecycle from placement to delivery). Rows are updated as milestones are reached.

Mixing grains in a single fact table creates ambiguity in aggregation and is a frequent source of incorrect results.

### Measures

Measures in fact tables fall into categories that determine how they can be aggregated:

| Measure Type  | Example                          | Aggregation Behavior                 |
| ------------- | -------------------------------- | ------------------------------------ |
| Additive      | Revenue, quantity sold           | Sum across any dimension             |
| Semi-additive | Account balance, inventory level | Sum across some dimensions, not time |
| Non-additive  | Unit price, ratio, percentage    | Cannot be summed meaningfully        |

### Dimension Hierarchies

Dimensions often contain natural hierarchies (date → month → quarter → year; city → state → country). These hierarchies enable drill-down and roll-up operations. Representing them as attributes within a single dimension table (the star approach) vs. separate normalized tables (the snowflake approach) affects query patterns and maintenance.

Slowly changing dimensions (SCDs) address the problem of dimension attributes that evolve over time:

- **Type 1**: Overwrite the old value. Simple but loses history.
- **Type 2**: Add a new row with versioning columns (effective dates, current flag). Preserves history but increases dimension size.
- **Type 3**: Add a column for the previous value. Limited history but no row growth.
- **Type 6 (hybrid)**: Combines types 1, 2, and 3 for different use cases within the same dimension.

The appropriate SCD strategy depends on whether historical analysis requires point-in-time accuracy or only current-state reporting.

## Kimball vs Inmon — Warehouse Design Philosophies

### Kimball (Bottom-Up)

Ralph Kimball's approach builds the warehouse as a collection of dimensional data marts, each modeled around a business process. Data marts are integrated through conformed dimensions — shared dimension tables with consistent keys and attributes across marts.

Characteristics: faster time-to-value per business area, business-user-friendly star schemas, incremental delivery. Risks: without governance, data marts can diverge, creating inconsistency. Conformed dimensions require organizational discipline.

### Inmon (Top-Down)

Bill Inmon's approach starts with a centralized, normalized enterprise data warehouse (EDW) in third normal form. Departmental data marts are derived from the EDW as needed.

Characteristics: single source of truth, strong data consistency, enterprise-wide integration. Risks: longer initial delivery timeline, higher upfront modeling effort, the EDW can become a bottleneck for new requirements.

### Convergence

Modern warehouse practice often blends elements — using a lightly normalized staging/integration layer (Inmon-influenced) that feeds dimensional models for consumption (Kimball-influenced). The choice is less binary than the original debate suggested; organizational maturity, team skills, and time constraints shape the practical approach.

## Columnar Storage

Row-oriented storage writes all columns of a row contiguously on disk. Column-oriented storage writes all values of a single column contiguously.

For analytical queries that typically access a subset of columns across many rows, columnar storage provides:

- **I/O reduction**: Only columns referenced in the query are read from disk.
- **Compression efficiency**: Values within a column share the same data type and often similar distributions, enabling dictionary encoding, run-length encoding, and delta encoding with high compression ratios.
- **Vectorized execution**: Processing a column of homogeneous values enables SIMD operations and cache-friendly access patterns.

Trade-offs: single-row lookups require reading from multiple column files. Write patterns that insert individual rows incur overhead assembling across columns. This is why columnar formats pair naturally with batch/bulk loading rather than row-at-a-time insertion.

Encoding strategies within columnar formats include:

| Encoding    | When Effective                       | Mechanism                                    |
| ----------- | ------------------------------------ | -------------------------------------------- |
| Dictionary  | Low-cardinality columns              | Replace values with integer codes            |
| Run-length  | Sorted or repeated values            | Store value + count                          |
| Delta       | Sequential or slowly changing values | Store differences between consecutive values |
| Bit-packing | Small integer ranges                 | Use minimum bits per value                   |

## Materialized Views and Pre-Aggregation

Materialized views store the result of a query physically, trading storage and refresh cost for query speed. Pre-aggregation is a broader pattern — any mechanism that computes aggregates ahead of query time.

Benefits: dramatically faster query response for known access patterns, reduced compute cost for repeated aggregations.

Costs and considerations:

- **Staleness**: Materialized views require refresh — synchronous (blocking writes until the view is updated) or asynchronous (allowing temporary inconsistency).
- **Maintenance overhead**: Each materialized view is a dependency on base tables. Schema changes cascade.
- **Storage multiplication**: Heavy pre-aggregation can multiply storage requirements significantly.
- **Query routing complexity**: The query planner or application must know when to use the materialized view vs. the base tables.

OLAP cubes represent an extreme of pre-aggregation — computing aggregates along every combination of dimensions. The combinatorial explosion limits this approach to relatively low-dimensionality datasets.

A middle path involves aggregate-aware query layers that maintain a catalog of available pre-aggregations and transparently route queries, falling back to base tables when no suitable aggregate exists.

## Data Vault Modeling

Data vault modeling, originated by Dan Linstedt, structures the warehouse for auditability and adaptability rather than query performance:

- **Hubs**: Business keys (the unique identifiers of business entities). Loaded once, never modified.
- **Links**: Relationships between hubs. Capture associations without encoding business rules.
- **Satellites**: Descriptive attributes and their change history, attached to hubs or links. New attributes are added as new satellites without altering existing structures.

Strengths: full insert-only auditability (no updates, no deletes), resilience to source system changes, parallel loading of independent structures. The model accommodates new data sources without restructuring existing tables.

Limitations: the raw vault structure is unfriendly for direct querying — a business vault or dimensional layer on top is typically necessary for consumption. The additional abstraction layer increases modeling and ETL complexity.

Data vault is most compelling in environments with regulatory audit requirements, highly volatile source systems, or where multiple conflicting sources must coexist with full traceability.

## The Modern Data Stack — Separation of Concerns

The label "modern data stack" describes an architectural pattern separating:

| Layer          | Concern                            | Examples of Approach                       |
| -------------- | ---------------------------------- | ------------------------------------------ |
| Ingestion      | Extract data from sources          | CDC, API polling, event streaming          |
| Storage        | Persist raw and processed data     | Object storage, cloud warehouse storage    |
| Compute        | Transform and query data           | Decoupled compute engines, elastic scaling |
| Transformation | Business logic, cleaning, modeling | SQL-based transformation frameworks        |
| Orchestration  | Scheduling, dependency management  | DAG-based workflow engines                 |
| Serving        | BI, dashboards, ML feature stores  | Semantic layers, caching layers            |

The key architectural insight is decoupling storage from compute — allowing independent scaling and cost optimization. Raw data persists cheaply in object storage; compute spins up only for transformation and query workloads.

Trade-offs: the composability of separate tools introduces integration complexity, version management across components, and potential inconsistency between layers. A monolithic warehouse trades flexibility for operational simplicity.

## Data Lakehouse

The lakehouse concept merges data lake characteristics (open formats, schema-on-read, cheap storage) with warehouse features (ACID transactions, schema enforcement, indexing):

| Feature            | Data Lake                            | Data Warehouse           | Lakehouse                             |
| ------------------ | ------------------------------------ | ------------------------ | ------------------------------------- |
| Storage format     | Open (Parquet, ORC)                  | Proprietary              | Open with metadata layer              |
| Schema enforcement | Optional (schema-on-read)            | Strict (schema-on-write) | Configurable per table                |
| ACID transactions  | Typically absent                     | Built-in                 | Added via metadata/log layer          |
| Query performance  | Variable                             | Optimized                | Optimized via statistics and indexing |
| Data types         | Any (structured, semi, unstructured) | Primarily structured     | Any                                   |

Table format layers add transaction logs, time travel (querying historical snapshots), schema evolution, and partition management on top of open file formats. This metadata layer is what distinguishes a lakehouse from a raw data lake.

The convergence is not without tension — the metadata and indexing overhead of lakehouse formats approaches warehouse complexity, and the performance gap with purpose-built warehouses, while narrowing, persists for certain query patterns.

## Partitioning and Clustering

### Partitioning

Partitioning divides a table into segments based on column values, enabling partition pruning — skipping irrelevant partitions during query execution.

Common partitioning strategies:

- **Time-based**: By day, month, or year. Natural for event data and time-series analysis.
- **Hash-based**: Distribute rows across partitions by hash of a key. Balances partition sizes but does not support range pruning.
- **List-based**: Explicit assignment of values to partitions (e.g., by region or category).

Over-partitioning creates many small files, degrading metadata management and query planning. Under-partitioning fails to provide scan reduction. The sweet spot depends on data volume, query patterns, and the storage engine's file-handling characteristics.

### Clustering (Sort Order)

Clustering orders data within partitions by one or more columns, co-locating related values physically. This enables:

- Efficient range scans on the clustering key
- Better compression (similar values adjacent)
- Zone map / min-max index effectiveness (each data block covers a narrow value range)

Clustering is most effective when queries consistently filter on the clustering columns. Multi-column clustering keys face diminishing returns — the first column dominates physical ordering.

## Query Optimization in Analytical Contexts

Analytical query optimization focuses on reducing the volume of data scanned and minimizing data movement:

### Scan Reduction

- **Partition pruning**: Eliminate entire partitions based on filter predicates.
- **Column pruning**: Read only referenced columns (inherent in columnar formats).
- **Zone maps / min-max indexes**: Skip data blocks where the filter value falls outside the block's value range.
- **Bloom filters**: Probabilistically skip blocks that cannot contain the filtered value.

### Predicate Pushdown

Moving filter evaluation closer to the storage layer — into the storage engine or even the file format reader — reduces the volume of data that enters the query processing pipeline. The effectiveness depends on how deeply the storage format supports predicate evaluation.

### Join Strategies

| Strategy        | When Used                          | Characteristics                                       |
| --------------- | ---------------------------------- | ----------------------------------------------------- |
| Hash join       | Large-to-large table joins         | Builds hash table on smaller side, probes with larger |
| Broadcast join  | One small, one large table         | Replicates small table to all nodes                   |
| Sort-merge join | Pre-sorted inputs                  | Efficient for already-ordered data                    |
| Nested loop     | Small outer table or indexed inner | Fallback; expensive for large inputs                  |

In distributed analytical systems, join ordering interacts with data distribution — co-located data avoids network shuffles, while non-co-located joins require redistribution. The optimizer's cardinality estimates drive these decisions; inaccurate statistics lead to suboptimal plans.

### Adaptive Query Execution

Some engines adjust execution plans mid-query based on runtime statistics — switching join strategies, adjusting parallelism, or re-partitioning data when actual cardinalities diverge from estimates. This partially mitigates the impact of stale or inaccurate statistics but adds execution overhead.

### Cost-Based vs Rule-Based Optimization

Cost-based optimizers estimate the computational cost of alternative plans using table statistics, column histograms, and hardware cost models. Rule-based optimizers apply heuristic transformations (e.g., push predicates down, eliminate redundant sorts). Most modern analytical engines use cost-based optimization with rule-based transformations as a preprocessing step. The quality of optimization depends heavily on the freshness and accuracy of collected statistics.
