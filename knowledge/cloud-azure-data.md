# Azure Data Services — Analytics, AI & Data Pipeline Concepts

## Azure SQL Database

Azure SQL Database is a managed relational database engine based on the latest stable SQL Server engine. The management boundary shifts compared to self-hosted SQL Server — patching, backups, high availability, and infrastructure management are handled by the platform.

### Purchasing Models and Tiers

| Model       | Concept                                           | Suited for                                       |
| ----------- | ------------------------------------------------- | ------------------------------------------------ |
| DTU-based   | Bundled compute, I/O, and memory units            | Predictable workloads with stable resource needs |
| vCore-based | Independent selection of compute, memory, storage | Workloads needing fine-grained resource control  |
| Serverless  | Auto-scaling compute with per-second billing      | Intermittent or unpredictable usage patterns     |

The **Hyperscale** tier introduces a fundamentally different storage architecture — a log-based distributed storage system that separates compute from storage, enabling databases to scale to 100 TB, near-instant backups regardless of size, and rapid read scale-out through up to four secondary replicas.

### Elastic Pools

Elastic pools share resources across multiple databases, allowing databases with varying peak times to share a collective DTU or vCore budget. This amortizes cost when managing many databases that individually have low average utilization but occasional spikes.

**Trade-off:** Pooled databases contend for shared resources. A runaway query in one database can affect others in the pool unless resource governance limits are configured.

### Operational Characteristics

- **Geo-replication** — asynchronous readable secondaries in different regions for DR and read offloading
- **Auto-failover groups** — DNS-based failover with grace period policies between primary and secondary regions
- **Transparent Data Encryption (TDE)** — encryption at rest enabled by default
- **Auditing** — tracks database events to storage, Log Analytics, or Event Hubs
- **Intelligent performance** — automatic tuning, query performance insights, adaptive query processing

## Cosmos DB

Cosmos DB is a globally distributed, multi-model database designed for applications that require low latency at any scale across multiple regions.

### Data Models

| API          | Data model               | Migration path from            |
| ------------ | ------------------------ | ------------------------------ |
| NoSQL (Core) | JSON documents           | Custom document stores         |
| MongoDB      | BSON documents           | MongoDB deployments            |
| Cassandra    | Wide-column              | Apache Cassandra clusters      |
| Gremlin      | Graph (vertices/edges)   | Graph databases                |
| Table        | Key-value                | Azure Table Storage            |
| PostgreSQL   | Relational (distributed) | PostgreSQL with sharding needs |

### Consistency Levels — The Spectrum

Cosmos DB exposes five consistency levels, representing a spectrum between strong consistency and maximum availability/performance:

```
Strong ◄──────────────────────────────────────────► Eventual
  │         │              │            │              │
Strong   Bounded       Session      Consistent      Eventual
         Staleness                  Prefix
```

| Level             | Guarantee                                                 | Latency impact                         | Availability impact               |
| ----------------- | --------------------------------------------------------- | -------------------------------------- | --------------------------------- |
| Strong            | Linearizability — reads always see latest committed write | Highest — waits for quorum replication | Limited to single write region    |
| Bounded staleness | Reads lag behind writes by at most K versions or T time   | High — bounded delay                   | Can span regions with constraints |
| Session           | Within a session, reads see own writes, monotonic reads   | Moderate                               | Multi-region capable              |
| Consistent prefix | Reads never see out-of-order writes                       | Lower                                  | Multi-region capable              |
| Eventual          | Reads may see any committed write, no ordering guarantee  | Lowest                                 | Highest availability              |

**Session consistency** is the default and most commonly selected — it provides read-your-own-writes semantics within a client session while allowing multi-region distribution. Choosing stronger consistency incurs higher RU (Request Unit) costs per operation and constrains multi-region write capabilities.

### Partitioning and Request Units

Performance in Cosmos DB is governed by two concepts:

- **Partition key** — determines data distribution across physical partitions. A poor partition key creates hot partitions and throughput bottlenecks. Ideal keys distribute both storage and request volume evenly.
- **Request Units (RUs)** — a normalized measure of compute cost. Every operation consumes RUs, and throughput is provisioned in RU/s. Provisioned throughput guarantees capacity; serverless mode charges per-RU consumed.

**Autoscale throughput** adjusts between a configured minimum and maximum RU/s, useful for workloads with variable demand. Manual throughput is more cost-efficient when load patterns are predictable.

## Azure Synapse Analytics

Synapse Analytics combines enterprise data warehousing, big data analytics, and data integration into a unified platform. It evolved from SQL Data Warehouse while incorporating Apache Spark and data orchestration capabilities.

### Compute Models

| Pool type           | Engine         | Characteristics                                                                     |
| ------------------- | -------------- | ----------------------------------------------------------------------------------- |
| Dedicated SQL pool  | MPP SQL engine | Provisioned compute, distributed storage, best for predictable analytical workloads |
| Serverless SQL pool | On-demand SQL  | No provisioned resources, queries data in-place (data lake), pay per TB scanned     |
| Apache Spark pool   | Spark engine   | Distributed processing for data engineering, ML, and streaming                      |

**Dedicated SQL pools** distribute data across 60 distributions using hash, round-robin, or replicated table strategies. Distribution key choice significantly impacts query performance — joins between tables co-located on the same distribution key avoid data movement.

**Serverless SQL pools** query Parquet, CSV, Delta, and JSON files directly in Azure Data Lake Storage without loading data into a warehouse. Suitable for exploratory analysis, data lake organization (logical data warehouse), and cost-sensitive workloads where provisioned capacity is unwarranted.

### Data Flow Architecture

```
Sources               Ingestion          Processing          Serving
┌─────────┐          ┌──────────┐       ┌──────────┐       ┌──────────┐
│ SaaS     │──────▶  │ Synapse  │──────▶│ Spark    │──────▶│ Dedicated│
│ IoT      │         │ Pipelines│       │ pools    │       │ SQL Pool │
│ Databases│──────▶  │          │──────▶│          │──────▶│          │
│ Files    │         │ (or ADF) │       │ SQL pools│       │ Power BI │
└─────────┘          └──────────┘       └──────────┘       └──────────┘
                                             │
                                      ┌──────▼──────┐
                                      │  Data Lake  │
                                      │  (ADLS Gen2)│
                                      └─────────────┘
```

Synapse integrates with Azure Data Lake Storage Gen2 as its primary storage layer, using the open Delta Lake format for ACID transactions, time travel, and schema enforcement on data lake files.

## Azure Data Factory

Data Factory is a managed ETL/ELT orchestration service for constructing data movement and transformation pipelines at scale.

### Core Abstractions

| Concept             | Role                                                                              |
| ------------------- | --------------------------------------------------------------------------------- |
| Pipeline            | Container for activities that define a workflow                                   |
| Activity            | A processing step (copy data, run stored procedure, execute Databricks notebook)  |
| Dataset             | Named reference to data structures within linked services                         |
| Linked service      | Connection configuration to data stores or compute                                |
| Integration runtime | Compute infrastructure executing pipeline activities                              |
| Trigger             | Mechanism to initiate pipeline execution (schedule, tumbling window, event-based) |

### Integration Runtimes

| Type        | Where it runs                      | Use case                                 |
| ----------- | ---------------------------------- | ---------------------------------------- |
| Azure       | Azure-managed auto-scaling compute | Cloud-to-cloud data movement             |
| Self-hosted | Customer-managed on-premises or VM | Accessing on-premises data sources       |
| Azure-SSIS  | Managed SSIS runtime in Azure      | Lift-and-shift of existing SSIS packages |

**Mapping data flows** provide a visual, code-free transformation experience that compiles to Spark under the hood. For complex transformation logic, invoking external compute (Databricks, Synapse Spark, Azure Functions) as pipeline activities offers more control.

**Data Factory vs Synapse Pipelines:** Synapse Pipelines are essentially Data Factory embedded within the Synapse workspace. The authoring experience and activity types are nearly identical. Standalone Data Factory remains relevant for organizations that need pipeline orchestration without the full Synapse analytics platform.

## Event Hubs

Event Hubs is a distributed, partitioned streaming platform for high-throughput event ingestion. Conceptually comparable to Apache Kafka, with a managed infrastructure layer.

**Architecture:**

- **Namespace** — the management container providing DNS identity and access policies
- **Event Hub** — analogous to a Kafka topic; the stream of events
- **Partitions** — ordered, immutable sequences of events within an Event Hub
- **Consumer groups** — independent read views of the event stream
- **Throughput units / Processing units** — capacity controls governing ingress and egress

**Kafka compatibility:** Event Hubs provides a Kafka-compatible endpoint, allowing existing Kafka producers and consumers to connect with minimal configuration changes. This eases migration paths from self-managed Kafka clusters.

**Capture** writes arriving events directly to Azure Blob Storage or Data Lake Storage in Avro format, creating a permanent archived stream without custom consumer code.

**Event Hubs vs Service Bus:** Event Hubs is optimized for high-volume event streaming with partitioned consumption. Service Bus is a message broker for transactional message processing with queues, topics, sessions, and dead-letter support. They serve complementary rather than competing roles.

## Azure Databricks

Azure Databricks provides a managed Apache Spark environment optimized for collaborative data engineering, data science, and ML workloads. It runs on Azure infrastructure but is operated in partnership with Databricks.

### Workspace Concepts

| Concept    | Description                                                           |
| ---------- | --------------------------------------------------------------------- |
| Workspace  | Organizational container for notebooks, libraries, and configurations |
| Cluster    | Auto-scaling Spark compute with configurable node types               |
| Notebook   | Interactive document combining code, visualization, and narrative     |
| Job        | Scheduled or triggered execution of notebooks or JAR/Python tasks     |
| Delta Lake | Open-source storage layer providing ACID transactions on data lakes   |

### Cluster Types

- **All-purpose clusters** — interactive analysis, shared across users, persist until manually terminated
- **Job clusters** — created for a specific job run, terminated upon completion
- **SQL warehouses** — optimized for SQL analytics workloads (Databricks SQL)

**Photon engine** accelerates Spark SQL queries through a vectorized execution engine written in C++, offering significant speedups for analytical queries on Delta Lake tables.

### Unity Catalog

A unified governance layer for data and AI assets across Databricks workspaces. Provides:

- Centralized access control with fine-grained permissions (table, column, row level)
- Data lineage tracking across notebooks, jobs, and pipelines
- Audit logging of data access patterns
- Metastore federation with external catalogs

## Azure Cognitive Services and Azure OpenAI Service

### Cognitive Services

Pre-built AI models exposed as REST APIs, designed for developers without deep ML expertise. Organized into capability domains:

| Domain   | Capabilities                                                            |
| -------- | ----------------------------------------------------------------------- |
| Vision   | Image classification, object detection, OCR, spatial analysis           |
| Language | Sentiment analysis, entity recognition, text summarization, translation |
| Speech   | Speech-to-text, text-to-speech, speech translation, speaker recognition |
| Decision | Anomaly detection, content moderation, personalizer                     |

Services can be deployed in Azure regions or as containers for on-premises and edge scenarios where data must not leave the local environment.

### Azure OpenAI Service

Provides access to large language models (GPT-series, DALL-E, Whisper, embeddings) with Azure's enterprise features layered on:

- **Content filtering** — configurable safety filters on inputs and outputs
- **Private networking** — accessible via Private Endpoints within VNets
- **Managed identity** — Azure AD-based authentication without API key management
- **Regional deployment** — data residency controls
- **Provisioned throughput** — reserved capacity for predictable latency

**Deployment model:** Models are deployed to specific regions within an Azure OpenAI resource. Each deployment has its own scaling configuration, content filter policies, and version pinning. Managing model versions across deployments requires attention as newer model versions may change behavior.

## Azure Machine Learning

A platform for managing the full ML lifecycle from experimentation through production deployment.

### Core Components

| Component         | Role                                                                       |
| ----------------- | -------------------------------------------------------------------------- |
| Workspace         | Top-level resource containing all ML assets                                |
| Compute instances | Developer VMs for notebook-based experimentation                           |
| Compute clusters  | Auto-scaling training clusters for large-scale jobs                        |
| Datastores        | Registered connections to storage (Blob, Data Lake, SQL)                   |
| Environments      | Reproducible runtime definitions (Docker-based)                            |
| Models            | Registered trained model artifacts with versioning                         |
| Endpoints         | Deployment targets for real-time or batch inference                        |
| Pipelines         | Multi-step ML workflows (data prep → training → evaluation → registration) |

### MLOps Patterns

```
Code Commit → CI Pipeline → Training Pipeline → Model Registry
                                                      │
                                               ┌──────▼──────┐
                                               │  Validation  │
                                               │  (accuracy,  │
                                               │   fairness,  │
                                               │   drift)     │
                                               └──────┬──────┘
                                                      │
                                              ┌───────▼───────┐
                                              │  CD Pipeline  │
                                              │  (deploy to   │
                                              │   endpoint)   │
                                              └───────────────┘
```

**Responsible AI dashboard** integrates model interpretability, fairness analysis, error analysis, and causal inference tools, supporting governance requirements around ML model transparency.

## Azure Stream Analytics

A real-time analytics engine that processes streaming data using a SQL-like query language. Ingests from sources like Event Hubs, IoT Hub, and Blob Storage, and outputs to destinations including databases, dashboards, and storage.

**Windowing functions** are central to stream processing:

| Window type | Behavior                                        |
| ----------- | ----------------------------------------------- |
| Tumbling    | Fixed-size, non-overlapping time intervals      |
| Hopping     | Fixed-size, overlapping intervals (size + hop)  |
| Sliding     | Outputs when events enter/leave window duration |
| Session     | Groups events arriving within a gap duration    |
| Snapshot    | Groups events with identical timestamps         |

**Exactly-once processing:** Stream Analytics guarantees exactly-once delivery to supported outputs (e.g., SQL Database, Cosmos DB), though source replay behavior requires understanding when designing end-to-end processing guarantees.

**Event ordering:** Late-arriving events, out-of-order data, and temporal skew between partitions are practical challenges. The service provides configurable late arrival and out-of-order tolerance windows, but the chosen values affect both accuracy and latency.

## The Data Estate — Composing Services Along the Data Lifecycle

Azure data services are rarely used in isolation. They compose along the data lifecycle, with each service addressing a stage:

```
Ingestion         Storage          Processing       Serving          Consuming
─────────         ───────          ──────────       ───────          ─────────
Event Hubs        ADLS Gen2        Databricks       Synapse          Power BI
IoT Hub           Blob Storage     Synapse Spark    Cosmos DB        Custom Apps
Data Factory      Azure SQL        Stream Analytics Azure SQL        Azure OpenAI
Kafka Connect     Cosmos DB        AML Pipelines    Redis Cache      Cognitive Svc
```

### Common Architectural Patterns

**Lambda architecture** — batch and speed layers operating in parallel, merging results at the serving layer. Batch processing (Synapse, Databricks) provides comprehensive historical analysis while stream processing (Stream Analytics, Event Hubs) handles real-time insights. Complexity in maintaining two codebases for the same logic is the chief drawback.

**Kappa architecture** — treating all data as streams, reprocessing historical data by replaying events. Event Hubs with long retention (or Capture to storage) enables this pattern. Simplifies the codebase at the cost of potentially higher compute for reprocessing.

**Medallion architecture** (Bronze/Silver/Gold) — organizing the data lake into raw (bronze), cleansed and conformed (silver), and business-aggregated (gold) layers using Delta Lake. Each layer adds data quality, and consumers choose the layer matching their quality/latency requirements.

**Data mesh** — decentralized ownership where domain teams own their data as products, published through a self-serve data platform. Azure Purview (now Microsoft Purview) provides the data catalog and governance layer. Synapse, Databricks, and Data Factory provide the compute and integration capabilities domain teams consume.

### Integration Patterns Between Services

| Source → Destination                | Mechanism             | Considerations                                             |
| ----------------------------------- | --------------------- | ---------------------------------------------------------- |
| Event Hubs → Databricks             | Structured Streaming  | Near real-time processing with checkpoint-based recovery   |
| Data Factory → Synapse              | Pipelines + COPY INTO | Bulk loading with transformation in ELT pattern            |
| Cosmos DB → Synapse                 | Synapse Link          | No-ETL analytical access via HTAP bridge                   |
| Event Hubs → Stream Analytics → SQL | Streaming pipeline    | Sub-second latency for operational dashboards              |
| ADLS Gen2 → Databricks → Synapse    | Medallion layers      | Progressive data refinement                                |
| AML → Cosmos DB/SQL                 | Managed endpoints     | Model inference results stored for application consumption |

**Synapse Link** deserves special note — it creates a bridge between Cosmos DB's operational store and Synapse's analytical engine without ETL pipelines, enabling HTAP (Hybrid Transactional/Analytical Processing) scenarios where analytical queries run against a near real-time copy of operational data without impacting transactional workload performance.

## Cost Optimization Across Analytical Workloads

Data workloads can accumulate significant costs across compute, storage, and data movement. The optimization strategies differ by service and workload pattern.

### Compute Cost Strategies

| Strategy                | Applicable services                          | Mechanism                                                              |
| ----------------------- | -------------------------------------------- | ---------------------------------------------------------------------- |
| Auto-pause / Serverless | SQL Database, Synapse serverless, Databricks | Eliminate idle compute costs                                           |
| Reserved capacity       | Synapse dedicated, Cosmos DB, SQL Database   | 1-year or 3-year commitments for discount                              |
| Spot instances          | Databricks, AML compute                      | Interruptible VMs at significant discount for fault-tolerant workloads |
| Right-sizing            | All compute services                         | Monitoring utilization to match provisioned capacity to actual need    |
| Cluster policies        | Databricks                                   | Constrain maximum cluster sizes and auto-termination timeouts          |

### Storage Cost Strategies

- **Tiering** — Azure Data Lake Storage supports hot, cool, and archive tiers with lifecycle management policies to automatically transition data based on age
- **Compression and columnar formats** — Parquet and Delta Lake compress data significantly versus CSV/JSON, reducing both storage cost and query scan cost
- **Retention policies** — defining how long data persists at each medallion layer prevents unbounded storage growth
- **Partitioning** — partitioning data by date or other high-cardinality keys enables partition pruning in queries, reducing scanned data volume

### Data Movement Costs

Cross-region data transfer incurs per-GB charges that can be surprising at scale. Architectural decisions about where data lands, how it moves between services, and how frequently pipelines run all influence the data transfer component of cost.

**Cost monitoring** through Azure Cost Management, resource tagging, and budget alerts provides visibility into spending patterns. Tags applied at the resource group or individual resource level enable cost attribution to business units, projects, or workload types — a prerequisite for governed cost optimization.

### The Provisioned vs Consumption Spectrum

Most Azure data services offer both provisioned (pay-for-capacity) and consumption (pay-per-use) models. The optimal choice depends on workload predictability:

- **Stable, high-utilization** workloads benefit from provisioned capacity with reserved pricing
- **Intermittent, exploratory** workloads benefit from serverless/consumption models
- **Variable but growing** workloads often start serverless and migrate to provisioned as patterns stabilize

This spectrum applies across services — Cosmos DB (provisioned RU/s vs serverless), Synapse (dedicated vs serverless pools), Databricks (always-on clusters vs job clusters), and SQL Database (provisioned vCores vs serverless tier). Understanding where each workload falls on this spectrum is foundational to cost governance.
