# Data Governance — Catalogs, Contracts & Access Control Patterns

## Overview

**Data governance** is the set of policies, processes, and technologies that manage data as an organizational asset. It answers:
- *Who owns this data?*
- *Can I use it for this purpose?*
- *Is it trustworthy?*
- *What happens if I change its schema?*

Governance spans **metadata management** (catalogs), **data quality**, **access control**, **lineage tracking**, and **policy enforcement**. The tension: governance friction (approval workflows, change control) vs. agility (data teams moving fast). Modern approaches balance both via infrastructure-as-code and contracts.

---

## Data Catalogs: Taxonomy & Discovery

A **data catalog** is a searchable inventory of organizational data assets (tables, datasets, streams, APIs) with metadata: owner, schema, lineage, quality metrics, usage rights.

### Problem Being Solved

In large organizations:
- Hundreds of tables across dozens of systems
- Analysts don't know: "Does marketing_spend table exist? Who maintains it? Is it up-to-date?"
- Duplicate effort: teams independently build "same" metrics
- Governance failures: PII leaks because downstream didn't know sensitive data flowed through

Catalogs make data *findable* and metadata *reusable*.

### Major Platforms

#### DataHub (LinkedIn)

**Architecture**:
- **Metadata store**: graph database (Neo4j) or relational; stores entities (dataset, chart, pipeline, user)
- **Connectors**: Airflow, Spark, dbt, Kafka ingest metadata automatically
- **Search**: Elasticsearch-backed; full-text search on metadata
- **API-first**: full programmatic access for automation
- **Lineage**: tracks transformations; shows data flow from source to consumer

**Strengths**:
- Mature; battle-tested at scale (LinkedIn, large enterprises)
- Strong lineage inference from orchestrators and dbt
- Rich metadata: tags, ownership, SLA agreements, legal basis for use
- API extensibility: custom metadata fields, programmatic updates

**Integrations**: Airflow DAGs → auto-creates datasets and lineage; dbt → tables + transformations; Spark → job lineage; Kafka → streams catalog

#### Apache Atlas

**Architecture**:
- Heavyweight metadata platform with governance model baked in
- **Classification**: data classified by sensitivity (PII, confidential)
- **Business metadata**: glossary terms map to technical entities (business term "customer_id" → database column)
- **Lineage**: table-level to process-level

**Strengths**:
- Fine-grained access control; supports column-level policies
- Part of Hadoop ecosystem; native Hive/Spark integration
- Lineage capture via Hive hooks (hook into query execution)

**Trade-offs**:
- Heavyweight; complex deployment
- Better for Hadoop ecosystems; less seamless for modern cloud (BigQuery, Snowflake, Databricks)

#### OpenMetadata

**Architecture**:
- Open-source competitor to DataHub; simpler onboarding
- **Metadata ingestion**: connectors for 50+ sources (BigQuery, Snowflake, Postgres, etc.)
- **Collaboration**: comments, tasks (similar to Jira), resolution tracking
- **Lineage**: traces via SQL parsing and connector introspection

**Strengths**:
- Easy self-hosted deployment (single Docker container)
- Broader connector coverage (better for multi-cloud environments)
- Built-in data quality framework (dbt tests integration, custom assertions)

**Trade-offs**:
- Smaller ecosystem than DataHub; fewer enterprise features (governance workflows)
- Lineage inference less mature than DataHub's

#### Amundsen (Lyft)

**Architecture**:
- Lightweight microservice: frontend (React), backend (Flask), metadata graph (Neo4j)
- **Search**: faceted search on metadata; emphasis on discovery UX
- **Owner/stats**: tracks data owner, last query time, popularity (used for ranking)

**Strengths**:
- Simplest to deploy (no orchestration deps)
- Beautiful UI; strong emphasis on discoverability

**Trade-offs**:
- Smaller feature set than DataHub/Atlas (lite governance)
- Less lineage inference; requires manual metadata in many cases

### Catalog Metadata Model

Standard catalog schemas include:

```
Dataset
├── name, description, owner, email
├── created_at, last_modified
├── columns: [Column{name, type, description, pii_classification}]
├── lineage: upstream_datasets, downstream_datasets
├── sla: max_age, uptime_target, backup_frequency
├── usage: query_count, fan_out (how many downstream consume this)
└── tags: ["finance", "daily", "incremental"]

Lineage Edge
├── source_dataset, target_dataset
├── transformation: dbt_model, Spark_job, custom_SQL
└── timestamp: when last transformation ran
```

---

## Data Contracts: Schema, SLAs, and Expectations

A **data contract** is an explicit agreement between data producer (upstream) and consumer (downstream):
- *"I will provide table X with columns Y, Z"*
- *"...updated by time T daily"*
- *"...with NULL rate < 1%, duplicate PKs = 0"*

Contracts prevent silent breakage: if producer changes schema or SLA fails, consumer notified.

### Design Patterns

#### Schema-First Contracts (dbt)

```yaml
# schema.yml in dbt project
models:
  - name: customers
    description: "Deduplicated customer master"
    contract:
      enforced: true
      columns:
        - name: customer_id
          data_type: int
          constraints:
            - type: not_null
            - type: unique
        - name: email
          data_type: string
          constraints:
            - type: not_null
        - name: created_at
          data_type: timestamp
    tests:
      - not_null
      - relationships: {to: ref('accounts'), field: customer_id}
    columns:
      - name: customer_id
        tests:
          - unique
          - not_null
```

**Enforcement**: dbt tests run on every transformation; failures block production deployment

**Strengths**:
- Git-tracked; version control on schema
- Integrated into CI/CD; pull requests include data contract changes
- Tests catch violations early

**Weaknesses**:
- Test-based, not schema-enforced (downstream could still write invalid data)
- Manual governance: reviewers must approve schema changes

#### Schema Registry & Topic Contracts (Kafka)

```json
{
  "type": "record",
  "namespace": "com.acme.events",
  "name": "UserSignup",
  "fields": [
    {"name": "user_id", "type": "string"},
    {"name": "email", "type": "string"},
    {"name": "timestamp", "type": "long"}
  ]
}
```

**Enforcement**: Kafka rejects messages not matching schema (producer-side validation)

**Schema Evolution**:
- **Backward compatibility**: new schema can read old data (e.g., new optional field with default)
- **Forward compatibility**: old schema can read new data (e.g., ignore new required field)
- Registry validates evolution rules; prevents breaking changes

**Strengths**:
- Enforced at write time; invalid data never reaches downstream
- Multi-schema versions coexist; gradual migration

**Weaknesses**:
- Event-stream centric; less applicable to batch transforms

#### SLA Contracts

```yaml
# Data contract
dataset: daily_revenue
owner: finance-team@acme.com
sla:
  max_age: 2 hours       # Must be fresh within 2h of ETL trigger
  uptime: 99%            # Available 99% of days (allow 3 missed days/month)
  data_quality:
    null_rate: < 1%      # <1% NULL values on critical columns
    duplicate_rate: 0    # No duplicate keys
    volume_trend: within 20% of historical  # Alert if volume drops >20% (data quality check)
  compliance:
    pii_columns: [email, phone]  # These columns marked PII; audited quarterly
    retention: 365 days  # Kept ≥ 1 year for compliance
```

**Monitoring**: Alert if contract breached (e.g., 2+ hour delay, >2% nulls)

**Escalation**: Contract breach → page owner → rollback/remediation decision

---

## PII Detection & Sensitive Data Management

### Classification Layers

**Automated detection** (heuristic + ML):
- Column name patterns: *_email, *_phone, ssn, credit_card_* → likely PII
- Data patterns: "123-45-6789" format → likely SSN
- Statistical: entropy analysis (credit cards have low entropy structure)
- ML: semantic models trained on known PII columns

**Manual annotation**:
- Catalog UX: mark column as PII; propagate to lineage (all downstream transform data)
- Tags: ["PII.email", "PII.phone", "confidential", etc.]

### Data Control Patterns

**Column-Level Access Control**:
```
Dataset: customers
├── Column: customer_id → Reader role (analysts can query)
├── Column: email       → Finance role only
├── Column: ssn         → Stripe integration only
└── Column: created_at  → Reader role
```

**Row-Level Security**:
```
Dataset: employee_expenses
├── Employee sees only their own rows (WHERE employee_id = context.user_id)
├── Manager sees team rows (WHERE manager_id = context.user_id)
└── Finance sees all rows
```

**Data Masking**:
```python
# Dynamic masking at read time
class_name: "PII"
transform: MASK_FIRST_5_CHARS  # SELECT SUBSTRING(email, 6) AS email
```

### Legal Basis Tracking

GDPR/CCPA require:
- **Processing justification**: why is PII collected?
- **Retention period**: how long kept?
- **Right to deletion**: can data subject request removal?

Catalog annotations:
```yaml
dataset: customer_interactions
pii_fields: [email, ip_address]
legal_basis: "legitimate_interest"  # or consent, contract, legal_obligation
retention_period: 3 years
supports_deletion: true  # Can data be purged on request
```

---

## Schema Registries

A **schema registry** is a versioned, queryable store of schemas for topics, tables, or datasets.

### Use Cases

1. **Type safety across systems**:
   - Kafka producer publishes events in Avro format
   - Consumer automatically deserializes using registered schema
   - Type mismatch caught early (missing field, wrong type)

2. **Schema versioning**:
   - Schema v1.0 (released Q1): [id, name]
   - Schema v1.1 (released Q2): [id, name, email] (backward compatible)
   - Both versions coexist; registry knows which messages use v1.0 vs. v1.1

3. **Governance**:
   - Schema changes require PR approval (pull request + review)
   - Audit trail: who changed, when, why

### Implementation

**Confluent Schema Registry** (Kafka-centric):
- REST API: PUT /subjects/{subject}/versions → register new schema
- Compatibility mode: BACKWARD, FORWARD, FULL
- Language support: Python, Java, Go bindings

**Iceberg/Delta Schema Definition**:
- Schema stored alongside table metadata
- Changes via ALTER TABLE tracked with versions
- Time-travel queries see historical schemas

---

## Data Quality Frameworks

Embedded in governance:

### Great Expectations

```python
validator = context.run_checkpoint("validation_checkpoint")
# Checkpoint includes suite of expectations:
validator.expect_column_values_to_not_be_null("customer_id")
validator.expect_column_values_to_be_unique("email")
validator.expect_column_values_to_match_regex("phone", r"^\d{3}-\d{3}-\d{4}$")
validator.expect_column_pairs_to_be_equal("created_at", "updated_at")
```

Used in:
- **Pre-load validation**: reject bad data before warehouse ingestion
- **Continuous monitoring**: run checks hourly; alert on regressions

### dbt Tests

```sql
# tests/customers_key_uniq.sql
select customer_id, count(*) as n
from {{ ref('customers')}}
group by customer_id
having n > 1
```

Run as part of CI/CD; failed test blocks merge to main.

---

## Ownership & Governance Workflows

### Clear Ownership

Every dataset has:
- **Owner**: data steward (typically engineering or analytics lead)
- **Stakeholders**: who consumes, who has SLA stake
- **Review board**: must approve schema/SLA changes

Catalog tracks:
- Owner contact
- Request/review ticketing (Jira, GitHub issues)
- Change history with approval chain

### Change Workflows

Schema change approval:
```
1. Engineer: "I want to add email column"
   → Creates pull request in dbt project, updates schema.yml, tests
2. Data steward: Reviews PR
   → "Approved; 2-week notice period before deploy"
3. Consumer notification: Email sent to downstream teams
   "customers table schema changing: adding email column (nullable)"
4. Deploy: After notice period, merge to main
5. Rollout: Backfill email column with nulls; downstream sees new column
```

---

## Data Mesh Governance

In federated ownership (data mesh), governance must account for autonomy:

**domain-first governance**:
- Each domain (Finance, Marketing) owns own data products
- Central governance team defines standards (schema format, test coverage)
- Domain teams self-serve; central team audits/enforces

**Cross-domain discovery**:
- Catalog shows which domain owns what data
- Contracts are negotiated between domains (producer/consumer)
- Lineage shows data flows across domain boundaries

**Standards without bottleneck**:
- Publish reusable patterns (e.g., "customer entity must have these columns")
- Domains adopt standards asynchronously, not synchronized by central team

---

## See Also

- [Data Quality](data-engineering-quality.md) — validation and observability
- [ETL/ELT Patterns](data-engineering-etl.md) — enforcing contracts in pipelines
- [Data Mesh](architecture-data-mesh.md) — governance at scale with federated teams
- [Data Lakehouse](data-engineering-lakehouse.md) — schema enforcement in table formats