# dbt (Data Build Tool) — SQL Transformation and Analytics Engineering

## Overview

dbt is a framework for managing SQL data transformations with software engineering practices. It enables **analytics engineers** to develop, test, version control, and deploy SQL transformations as reusable, documented models with transparent lineage.

Rather than running ad-hoc SQL scripts, dbt models are versioned code treated as first-class artifacts in data pipelines. The tool handles dependency management, orchestration, testing, and documentation generation for analytics workflows.

---

## Core Concepts

### Models

A **model** is a SQL query saved as a `.sql` file in the `models/` directory. dbt executes each model against the warehouse, storing the result as a table or view.

Model types:
- **Staging models** (`staging/`): Clean, rename, and deduplicate raw data from source systems. One-to-one or light aggregations
- **Intermediate models** (`intermediate/`): Combine staging models; compute derived logic (e.g., user lifetime value calculations)
- **Mart/dimensional models** (`marts/`): Final aggregated tables for end users and BI tools. Joined dimensions with denormalization for query performance
- **Ephemeral models** (`ephemeral: true`): Temporary queries not materialized as tables; inlined into dependent models to reduce physical tables

### ref() and source()

**ref()** references another dbt model, creating an explicit DAG dependency:

```sql
SELECT * FROM {{ ref('stg_users') }}
```

dbt recognizes this dependency and executes `stg_users` before the referencing model. `ref()` also provides test isolation: each model can be tested independently.

**source()** references raw data from external systems (databases, data warehouses). It defines the contract with upstream systems:

```sql
SELECT * FROM {{ source('raw_data', 'users_table') }}
```

Source definitions include schema validation, freshness checks, and documentation, decoupling source definitions from multiple models.

---

## Materialization Strategies

Models can be materialized in different ways:

### Table
Full recreation on each run. Fastest for downstream queries but expensive for large models. Use for final marts.

### View
No data persisted; query re-executes on reference. Cheap to maintain but slower for downstream queries and harder to debug. Use for intermediate logic or rarely-queried models.

### Incremental
Only process new/changed data. Maintains an existing table and appends/upserts new rows:

```sql
{{ 
  config(
    materialized='incremental',
    unique_key='id',
    on_schema_change='fail'
  )
}}

SELECT * FROM {{ source('raw', 'events') }}
{% if execute %}
  WHERE created_at > (SELECT MAX(created_at) FROM {{ this }})
{% endif %}
```

Incremental models reduce compute but require careful state management (idempotent updates, watermarking). Key tradeoff: speed vs. complexity.

### Snapshot
Captures a **Slowly Changing Dimension (SCD) Type 2** snapshot, preserving history with effective dates. Useful for tracking user attributes, account statuses over time.

---

## Testing

Models are automatically tested with schema tests (column presence, types, constraints):

```yaml
models:
  - name: users
    columns:
      - name: user_id
        tests:
          - unique
          - not_null
      - name: email
        tests:
          - unique
```

Custom data tests validate business logic:

```sql
-- tests/assert_user_created_at.sql
SELECT *
FROM {{ ref('users') }}
WHERE created_at IS NULL
```

If the query returns rows, the test fails. dbt runs all tests in the `tests/` directory, providing a safety net for data quality.

Tests are essential for maintaining models; without them, silent data corruption goes undetected.

---

## Documentation

dbt generates documentation from YAML configuration and SQL comments:

```yaml
models:
  - name: users
    description: Canonical user dimension
    columns:
      - name: user_id
        description: Primary key
```

`dbt docs generate` produces an interactive HTML site with:
- Model lineage (DAG visualization)
- Column descriptions and types
- Data freshness indicators
- SQL source code
- Test results

Documentation-as-code means docs stay in sync with models (no separate wiki to maintain).

---

## dbt Cloud

dbt Cloud is a SaaS platform for hosting and running dbt projects:
- **Scheduler**: dbt jobs run on a schedule (hourly, daily, etc.) without external orchestration
- **API**: Trigger dbt jobs from Airflow, Gitlab CI, or other orchestrators
- **Notifications**: Slack/email alerts on job failures
- **Partial parsing**: Incremental project parsing, speeding up large projects
- **Integrated IDE**: Browser-based SQL editor with auto-complete and lineage preview
- **Multi-tenancy**: Multiple dev/staging/production environments

Tradeoff: managed convenience vs. operational control. dbt Cloud is simpler for small teams; large organizations often run dbt-core as a job within Airflow for tighter control.

---

## Packages and Macros

**Packages** are reusable dbt modules published to the dbt package registry. Popular examples:
- `dbt_utils`: Helper macros (pivot, null check aggregations)
- `dbt_expectations`: Data quality tests
- `metrics`: Normalized metric definitions

Packages are versioned and installed via `packages.yml`.

**Macros** are Jinja templating functions enabling code reuse:

```sql
{% macro custom_aggregation(table_name, column_name) %}
  SELECT 
    {{ column_name }},
    COUNT(*) as cnt
  FROM {{ table_name }}
  GROUP BY {{ column_name }}
{% endmacro %}

SELECT * FROM {{ custom_aggregation('users', 'country') }}
```

Macros allow dynamic SQL generation, reducing duplication across models.

---

## dbt-core vs dbt Cloud

**dbt-core** is the open-source CLI tool. Install locally or in a container; run via cron or orchestrator. Full control; minimal SaaS overhead.

**dbt Cloud** adds scheduler, IDE, and collaboration UI on top of dbt-core. Simplified operations but vendor lock-in and additional cost.

Most organizations use dbt-core run within Airflow, treating dbt as a parameterized job that an orchestrator invokes.

---

## Warehouse Support

dbt supports all major warehouses:
- **Snowflake**: Native dbt-snowflake adapter with zero-copy clones for development
- **BigQuery**: Baked into cloud architecture; cheap to run dbt jobs
- **Redshift**: Mature support; common in AWS-centric orgs
- **Postgres/DuckDB**: Lighter-weight for local development and small teams
- **Spark/Databricks**: Growing adoption for lakehouse stacks

Adapter choice locks you into that warehouse's SQL dialect and performance characteristics. Switching warehouses requires rewriting models.

---

## Philosophy and Tradeoffs

dbt enforces a specific philosophy:
1. Transformations are SQL (not Python, not Spark jobs)
2. Models are DAG-based with explicit dependencies
3. Testing and documentation are mandatory, not optional
4. Version control is the source of truth

**Strengths**: Reduced code duplication, strong testing story, rapid iteration on SQL models, excellent lineage tracking, mature ecosystem.

**Weaknesses**: SQL-only (some logic is better expressed in Python/Spark); opinionated conventions may feel restrictive; incremental models add complexity; orchestration still requires external tools (Airflow).

dbt is best for **SQL analytics** and **data warehouse transformations**. It's not suitable for complex ML feature engineering, real-time processing, or non-tabular data. For those, use Spark or orchestrate with custom Python scripts.

Combined with Spark (for complex transformations) and Airflow (for orchestration), dbt forms a modern data stack for building reliable, maintainable pipelines.