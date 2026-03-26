# Data Mesh

## Principles

Data mesh is an organizational and architectural paradigm that treats data as a product, owned by domain teams rather than centralized data teams.

### The Four Principles

| Principle                              | Core Idea                                                                                                         |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **Domain ownership**                   | Domain teams own and serve their analytical data, not a central data team                                         |
| **Data as a product**                  | Domain data has product qualities: discoverable, addressable, trustworthy, self-describing, interoperable, secure |
| **Self-serve data platform**           | A platform team provides infrastructure so domain teams can build data products without deep data engineering     |
| **Federated computational governance** | Global standards enforced automatically, not by central gatekeepers                                               |

## Why Data Mesh

### Problems with Centralized Data

| Problem                  | Description                                                       |
| ------------------------ | ----------------------------------------------------------------- |
| **Bottleneck**           | Central data team becomes the bottleneck for all data requests    |
| **Domain knowledge gap** | Data engineers don't understand the business domain deeply enough |
| **Pipeline fragility**   | Centralized ETL pipelines break when source systems change        |
| **Slow delivery**        | Weeks/months to add a new data product or answer new questions    |
| **Ownership vacuum**     | Nobody owns data quality — source teams "push and forget"         |
| **Scaling the team**     | Hiring more data engineers doesn't scale linearly with domains    |

### What Data Mesh Changes

```
BEFORE (Centralized):
Domain A → Central ETL → Data Warehouse → BI/Analytics
Domain B →
Domain C →
  (central team owns it all)

AFTER (Data Mesh):
Domain A → [Data Product A] →
Domain B → [Data Product B] → Data consumers (analytics, ML, other domains)
Domain C → [Data Product C] →
  (each domain owns its data product, platform enables them)
```

## Domain Data Products

A data product is a well-defined output of a domain that serves analytical consumers.

### Characteristics

| Quality             | What It Means                                     | Example                                         |
| ------------------- | ------------------------------------------------- | ----------------------------------------------- |
| **Discoverable**    | Listed in a catalog, searchable                   | Registered in data catalog with metadata        |
| **Addressable**     | Stable, unique identifier / location              | `data://orders.order-facts`                     |
| **Trustworthy**     | Meets quality SLOs, documented lineage            | 99.5% completeness, <1hr freshness              |
| **Self-describing** | Schema, semantics, and usage documented           | Schema registry + README + sample queries       |
| **Interoperable**   | Follows global standards for format and semantics | Shared date formats, currency codes, entity IDs |
| **Secure**          | Fine-grained access control                       | Column-level security, PII masking              |

### Data Product Anatomy

```
┌─────────────────────────────────────────────┐
│  Order Facts Data Product (Orders Domain)   │
│                                             │
│  Input Ports:                               │
│    ← Orders database (CDC)                  │
│    ← Payment events (Kafka)                 │
│    ← Customer reference data (API)          │
│                                             │
│  Transformation:                            │
│    Join, aggregate, cleanse, enrich         │
│                                             │
│  Output Ports:                              │
│    → SQL endpoint (analytics queries)       │
│    → Parquet files in S3 (batch consumers)  │
│    → Kafka topic (streaming consumers)      │
│    → REST API (application consumers)       │
│                                             │
│  SLOs:                                      │
│    Freshness: < 1 hour                      │
│    Completeness: > 99.5%                    │
│    Availability: 99.9%                      │
│                                             │
│  Schema: Registered in Schema Registry      │
│  Owner: Orders Domain Team                  │
└─────────────────────────────────────────────┘
```

### Data Product Types

| Type                 | Description                                    | Example                                        |
| -------------------- | ---------------------------------------------- | ---------------------------------------------- |
| **Source-aligned**   | Direct analytical exposure of operational data | Order events, cleaned and served for analytics |
| **Aggregate**        | Pre-computed aggregations within a domain      | Daily order summary, customer lifetime value   |
| **Consumer-aligned** | Tailored for a specific consumer's needs       | ML feature store for recommendation engine     |

## Comparison: Data Mesh vs Data Warehouse vs Data Lake

| Aspect                       | Data Warehouse                | Data Lake                     | Data Mesh                   |
| ---------------------------- | ----------------------------- | ----------------------------- | --------------------------- |
| **Ownership**                | Central data team             | Central data team             | Domain teams                |
| **Schema**                   | Schema-on-write (star schema) | Schema-on-read (raw files)    | Schema-on-write per product |
| **Governance**               | Centralized                   | Often weak/absent             | Federated                   |
| **Technology**               | Snowflake, BigQuery, Redshift | S3 + Spark, ADLS              | Varies per domain           |
| **Data quality**             | Central team responsible      | Often poor                    | Domain team SLOs            |
| **Scaling model**            | Scale the central team        | Scale storage                 | Scale teams independently   |
| **Time to new data product** | Weeks-months (ticket queue)   | Days-weeks (data engineering) | Days (self-serve)           |

**Data mesh doesn't replace data warehouses.** A domain team might use a warehouse as an output port. The difference is _who owns and builds_ the data product.

## Implementation Patterns

### Domain-Aligned Data Teams

Each domain team includes "data product developers" — engineers who build and maintain the domain's data products alongside operational features.

```
Orders Domain Team:
  - Backend engineers (operational APIs)
  - Data product developer(s) (analytical data products)
  - Shared: same codebase, same deployments, same domain knowledge

NOT:
  - Backend team builds APIs
  - Separately, central data team pulls data out with ETL
```

### Data Product API

Each data product exposes standardized interfaces:

```yaml
# data-product.yaml (metadata)
name: order-facts
domain: orders
owner: orders-team@company.com
version: 2.1.0
description: Enriched order data for analytical consumption

output_ports:
  - type: bigquery_table
    address: project.orders.order_facts_v2
  - type: kafka_topic
    address: data.orders.order-facts.v2
  - type: s3_path
    address: s3://data-products/orders/order-facts/v2/

schema:
  registry: schema-registry.internal
  subject: order-facts-v2-value

slos:
  freshness_minutes: 60
  completeness_percent: 99.5
  availability_percent: 99.9

classification: internal
pii_columns: [customer_email, shipping_address]
```

### Self-Serve Data Platform

The platform team provides:

| Capability                  | What It Provides                                            |
| --------------------------- | ----------------------------------------------------------- |
| **Data product templating** | Scaffold new data products with standard structure          |
| **Schema registry**         | Central schema management with compatibility checks         |
| **Data catalog**            | Discovery, search, lineage visualization                    |
| **Quality monitoring**      | Automated SLO checks, alerting                              |
| **Access management**       | Self-serve access requests, RBAC/ABAC                       |
| **Compute provisioning**    | Pipeline infrastructure (Spark, Flink, dbt) on demand       |
| **Storage**                 | Managed data lake storage with standard formats             |
| **CI/CD for data**          | Testing, versioning, deployment pipelines for data products |

### Mesh Platform Architecture

```
┌────────────────────────────────────────────────────────────┐
│  Self-Serve Data Platform                                  │
│                                                            │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │
│  │ Catalog  │ │ Schema   │ │ Quality  │ │ Access       │  │
│  │ Service  │ │ Registry │ │ Monitor  │ │ Manager      │  │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────┘  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │
│  │ Compute  │ │ Storage  │ │ CI/CD    │ │ Governance   │  │
│  │ (Spark)  │ │ (S3/GCS) │ │ Pipeline │ │ Policies     │  │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────┘  │
└────────────────────────────────────────────────────────────┘
         ↑              ↑              ↑
   ┌─────┴─────┐  ┌────┴────┐  ┌─────┴─────┐
   │ Orders    │  │ Payments│  │ Logistics │
   │ Domain    │  │ Domain  │  │ Domain    │
   │ Data Prod │  │ Data Prod│ │ Data Prod │
   └───────────┘  └─────────┘  └───────────┘
```

## Technology Choices

### Data Catalog & Discovery

| Tool                         | Type        | Key Feature                            |
| ---------------------------- | ----------- | -------------------------------------- |
| **DataHub** (LinkedIn)       | Open source | Metadata graph, lineage, governance    |
| **OpenMetadata**             | Open source | Standards-based, data quality built-in |
| **Atlan**                    | Commercial  | Business glossary, collaboration       |
| **Databricks Unity Catalog** | Platform    | Unified governance for Databricks      |
| **Google Dataplex**          | Managed     | Discovery, quality, security for GCP   |

### Data Contracts

Formalized agreements between data producers and consumers:

```yaml
# data-contract.yaml
dataContractSpecification: 0.9.3
id: orders-order-facts
info:
  title: Order Facts
  version: 2.0.0
  owner: orders-team

servers:
  production:
    type: bigquery
    project: analytics
    dataset: orders

models:
  order_facts:
    description: One row per order with enriched attributes
    fields:
      order_id:
        type: string
        required: true
        unique: true
      customer_id:
        type: string
        required: true
        pii: true
      order_total:
        type: decimal
        required: true
      order_date:
        type: date
        required: true

quality:
  type: SodaCL
  specification:
    checks for order_facts:
      - row_count > 0
      - missing_percent(order_id) = 0
      - invalid_percent(order_total) < 1%
```

### Schema Registry

Schema Registry (Confluent, AWS Glue, Apicurio) enforces schema evolution rules:

| Compatibility | Rule                         | Safety                      |
| ------------- | ---------------------------- | --------------------------- |
| BACKWARD      | New schema can read old data | Consumers can upgrade first |
| FORWARD       | Old schema can read new data | Producers can upgrade first |
| FULL          | Both backward and forward    | Safest, most restrictive    |
| NONE          | No compatibility check       | Dangerous in production     |

## Federated Governance

### What's Federated

| Concern            | Global Standard                       | Domain Decision             |
| ------------------ | ------------------------------------- | --------------------------- |
| Naming conventions | Company-wide naming rules             | Domain-specific field names |
| Date/time format   | ISO 8601 everywhere                   | —                           |
| PII handling       | Masking/encryption requirements       | Which fields are PII        |
| Schema evolution   | Compatibility rules                   | Schema details              |
| Quality thresholds | Minimum SLO framework                 | Specific SLO values         |
| Access model       | RBAC/ABAC framework                   | Who gets access to what     |
| Data formats       | Parquet for batch, Avro for streaming | —                           |

### Automated Policy Enforcement

Govern through automation, not review boards:

```python
# CI pipeline for data products
def validate_data_product(config):
    assert config.schema_registered, "Schema must be registered"
    assert config.slos_defined, "SLOs must be defined"
    assert config.owner_specified, "Owner team must be specified"
    assert config.pii_classified, "PII columns must be classified"
    assert config.access_policies_set, "Access policies required"
    # Fail the pipeline if governance requirements aren't met
```

## Challenges

### Organizational Change

Data mesh is primarily an organizational shift, not a technology change. Challenges:

- Domain teams resist owning data products ("that's the data team's job")
- Data engineering skills need to be distributed to domain teams
- Central data team transitions to platform role (identity crisis)
- Management must realign incentives and OKRs

### Cross-Domain Queries

When analytics needs data from multiple domains:

| Approach                      | How                                             | Trade-off                    |
| ----------------------------- | ----------------------------------------------- | ---------------------------- |
| Federated query               | Query engine spans domains (Trino, BigQuery)    | Performance limits, coupling |
| Consumer-aligned data product | A domain creates a joined/enriched product      | Ownership of the join logic  |
| Data marketplace              | Central catalog discovers and composes products | Complexity                   |

### Data Quality

Without central control, quality can vary. Mitigations:

- **Automated quality checks** in CI/CD pipelines
- **SLO monitoring** with alerts to domain teams
- **Data contracts** between producers and consumers
- **Quality scores** visible in the data catalog

## When Data Mesh Makes Sense

### Good Fit

- Large organizations (100+ engineers) with distinct business domains
- Multiple data consumers with different needs
- Central data team is a bottleneck
- Domain expertise is critical for data quality
- Teams are mature enough for distributed ownership

### Poor Fit

- Small organizations (< 50 engineers) — overhead isn't justified
- Single domain — just build a good data pipeline
- No analytical data consumers — solve the problem when you have it
- Teams lack data engineering capability — build the platform first
- "We want data mesh" without organizational will to change ownership

### Adoption Path

```
1. Start with one domain → Prove the model works
2. Build platform primitives → Catalog, schema registry, quality monitoring
3. Expand to 2-3 domains → Refine governance and platform
4. Federate governance → Automate policy enforcement
5. Scale to all domains → Ongoing organizational evolution
```

**Data mesh is a journey, not a migration. It takes years, not months.**
