# Data Contracts — Schema Versioning, Quality Agreements & Data Product Interfaces

## Introduction

A **data contract** is a formal agreement between a data producer and consumers: "I will provide data with this schema, this freshness, this quality, over this period." Contracts codify expectations, enable detection of breaking changes, and establish SLOs for data quality and timeliness.

Data contracts are to data products what API contracts are to microservices—a mutually understood interface.

## Core Concepts

### Schema as Contract

A contract begins with **schema**: the structure of data. Schema defines column names, types, nullability, ordering, and constraints.

```yaml
# canonical_customer_contract.yaml
product: canonical_customer
version: 2.1.0
owner: customer_platform_team
updated: 2024-03-20

schema:
  - name: customer_id
    type: string
    description: "Unique customer identifier (UUID)"
    required: true
    primary_key: true
    
  - name: email
    type: string
    description: "Customer email address"
    required: true
    constraints:
      - type: "regex"
        pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
        message: "Must be a valid email"
    
  - name: lifetime_value_usd
    type: decimal(18, 2)
    description: "Total customer spend (USD)"
    required: false
    constraints:
      - type: "range"
        min: 0
        message: "LTV must be non-negative"
    deprecated_in_version: "3.0.0"
    deprecation_replacement: "lifetime_value_cents"
```

**Advantages of schema-first contracts:**
- Machine-readable; tooling can validate automatically.
- Detection of incompatibilities at producer-consumer bind time (not runtime).
- Documentation is enforced (schema with no description is caught in review).

**Challenges:**
- Schema alone doesn't capture semantic invariants (e.g., "this column is sorted by date").
- Type systems are shallow; constraints (range, regex) require custom logic.
- Schema evolution is expected; contract versioning is administrative overhead.

### Breaking vs. Non-Breaking Changes

| Change | Type | Impact | Example |
|--------|------|--------|---------|
| Add optional column | Non-breaking | Consumers unaffected (new readers ignore it) | `schema: [{name: "new_field"}]` |
| Add required column with default | Non-breaking | Producers fill with default; consumers unaffected | `required: true, default: null` |
| Remove optional column | Breaking | Consumers expecting it fail | Removing `email` column |
| Rename column | Breaking | Consumers hardcoded to old name fail | `email` → `email_address` |
| Change type | Breaking | Incompatible deserialization | `string` → `integer` |
| Add constraint | Breaking if narrower | Producers may now fail validation | Adding `NOT NULL` to nullable column |
| Relax constraint | Non-breaking | Producers have more freedom | Expanding range constraint |

**Production reality:**

- Producers often change schemas without coordinating (easy in self-serve systems).
- Consumers often ignore or are unaware of schema changes (notification is manual).
- Breaking changes sometimes go undetected for weeks (data corruption in log, queries fail silently, or results are wrong).

### Breaking Change Detection

Tools that detect breaking changes **at publish time**:

- **Schema registries** (Confluent Schema Registry, Apicurio, AWS Glue Schema Registry): when producer publishes new schema, registry validates against previous. Forward/backward/full compatibility rules enforced.

- **dbt contracts** (dbt v1.8+): `contract: {enforced: true}` on models; schema drift detected in CI/CD.

- **Great Expectations / Soda / dbt tests:** Runtime data validation; if data violates contract (e.g., nulls in required column), test fails.

**Enforcement depends on where validation happens:**

1. **Schema registry:** Pre-publish (producer can't publish incompatible schema).
2. **Consumer deserialization:** At consumer read time (may be delayed; data already corrupt in broker/lake).
3. **Contract tests:** Continuous validation (Great Expectations, dbt tests run on fresh data).

**Ideal flow:** Schema registry catches incompatibilities before publish; contract tests verify runtime quality.

## SLOs and Quality Agreements

Beyond schema, contracts specify **service-level objectives (SLOs)** for data quality and timeliness.

### Freshness SLO

**Freshness:** the lag between when data changes in the source and when it appears in the contract product.

```yaml
slo:
  freshness: 5m          # Data must be no more than 5 minutes old
  window: daily          # SLO applies during business hours (alternatively: 24/7)
  alert_threshold: 10m   # Alert on warning if freshness > 10m
```

**Measurement:**
- For CDC-based products: max timestamp in product vs. max timestamp in source.
- For batch: time since last successful load.

**Challenges:**
- **Perception gap:** Consumers often expect "real-time" (< 1 second) but contract is 5m. Managing expectations requires communication.
- **Source variability:** Upstream systems have their own SLOs; upstreaming failures causes downstream breach.
- **Cost vs. freshness:** Tighter freshness (1m instead of 5m) costs more (infrastructure, operational overhead). Trade-off explicit in contract negotiation.

### Completeness & Accuracy SLO

**Completeness:** expected rows, distributions.

```yaml
slo:
  completeness:
    - metric: "row_count"
      expected: "> 1000000"          # At least 1M rows
      window: daily
    - metric: "null_rate"
      column: "email"
      expected: "< 0.01"             # Less than 1% nulls
      window: daily
```

**Accuracy:** domain-specific business logic constraints.

```yaml
slo:
  accuracy:
    - constraint: "lifetime_value_usd >= 0"
    - constraint: "customer_created_at <= NOW()"
    - constraint: "email is valid format"
```

**Tools for enforcement:**
- **Great Expectations:** Python-based data validation; extensive constraint library (ranges, uniqueness, custom functions).
- **dbt tests:** SQL-based (dbt built-in tests + custom SQL tests).
- **Soda:** YAML-based SLO definitions; integrates with orchestrators (dbt, Airflow).

**Production reality:**
- Defining correct SLOs is hard (requires historical data, domain knowledge).
- SLOs are often too loose initially, then tightened after data maturity increases.
- Enforcement is often **advisory** (alerting) not **blocking** (prevents bad data release). Blocking requires manual review, adds latency to releases.

### Availability SLO

**Availability:** percentage of time the product is queryable and accurate.

```yaml
slo:
  availability: 99.9%    # Product up and correct 99.9% of the time
```

Harder to measure. Usually tracked via monitoring: did the last load succeed? Is the table non-empty? Does data pass quality checks?

## Data Product Ownership & Accountability

Contracts establish **accountability**:

- **Producer**: Owns schema, SLO delivery, change notifications.
- **Consumer**: Agrees to use data contract as documented; responsibly handles version changes.
- **Platform**: Provides tooling (schema registry, monitoring, contract enforcement).

### Ownership Markers

```yaml
# part of contract metadata
producer:
  team: "customer_platform"
  slack_channel: "#customer-data"
  on_call_schedule: "pagerduty-customer-platform"
  escalation_contact: "cpo@company.com"

consumers:
  - team: "analytics"
    use_case: "customer dashboards"
    contact: "analytics-lead@company.com"
  - team: "ml_platform"
    use_case: "customer segmentation model training"
```

**Purpose:**
- Clear communication channels (who to contact for issues).
- Dependency tracking (analytics can't release if customer data is late).
- Impact assessment (if producer changes schema, known consumers can be notified).

## Semantic Versioning for Data

Treat data products as software with versions:

```
MAJOR.MINOR.PATCH

1.0.0: Initial release
  schema: {customer_id, email, created_at}
  
1.1.0: Add optional column (backward compatible)
  schema: {customer_id, email, created_at, lifetime_value_usd}
  changelog: "Added lifetime_value_usd for new FP&A requirements"
  
1.1.1: Fix data quality (patch)
  changelog: "Corrected null handling for email; now catches missing values"
  
2.0.0: Breaking change (rename + deprecation)
  schema: {customer_id, email_address, created_timestamp_utc}
  changelog: "Deprecated created_at; use created_timestamp_utc (UTC, not local)"
  breaking_changes:
    - "Renamed 'email' to 'email_address'"
    - "Renamed 'created_at' to 'created_timestamp_utc' (changed timezone)"
    - "Removed non_conforming_addresses column"
  migration_guide: "See https://wiki/customer-data-v2-migration"
```

**Enforcement:**
- Tags in schema registry (Confluent: version in subject name or metadata).
- Git tags / release workflows (dbt semantic versioning framework in progress).
- Manual discipline (document breaking changes in changelog).

### Deprecation Lifecycle

```
v1.0: Created
v1.1: Deprecation announced
      - Dual-publish: old_field and new_field both present
      - Mark old_field as deprecated
      - Notification to consumers

v1.1 + 6 months: Grace period, consumers migrate
                - Monitor consumption of old_field
                - Encourage migration

v2.0: Removal
      - Old_field removed
      - All consumers have migrated (verified via monitoring)
      - Breaking change requires major version bump
```

**Challenges:**
- Discipline: domains skip grace period to ship faster.
- Monitoring: tracking which consumers still use which fields requires instrumentation.
- Legacy systems: some consumers can't upgrade easily (legacy code, budget constraints).

## Contract Testing

**Contract tests** are automated checks that data product meets contract obligations.

### Test Categories

1. **Schema conformance:** Data matches declared schema (type checks, presence).
   ```sql
   -- dbt test
   SELECT * FROM {{ ref('canonical_customer') }}
   WHERE customer_id IS NULL
      OR NOT REGEXP_LIKE(email, '^[a-zA-Z0-9._%+-]+@')
   ```

2. **Uniqueness/Primary keys:**
   ```sql
   SELECT customer_id, COUNT(*) as cnt
   FROM {{ ref('canonical_customer') }}
   GROUP BY customer_id
   HAVING cnt > 1
   ```

3. **Referential integrity:** Foreign keys exist in referenced tables (expensive; often skipped).

4. **Business logic constraints:**
   ```sql
   -- Customers can't have future created_at dates
   SELECT * FROM {{ ref('canonical_customer') }}
   WHERE created_at > CURRENT_TIMESTAMP()
   ```

5. **Distribution / statistical** (dbt-expectations, Great Expectations):
   ```yaml
   # Great Expectations
   - expectation_type: "expect_column_value_z_score_between"
     column: "lifetime_value_usd"
     thresholds: [-3, 3]   # Alert if z-score > 3 (outliers)
   ```

### Test Execution & Failure Handling

**When to run tests:**
- After every data load (blocking: bad data doesn't get published).
- Periodically (monitoring: catch degradation over time).
- Before consumer query (rejection: prevent downstream corruption).

**On failure:**
- **Log & alert:** Non-blocking, allows investigation before deciding to block.
- **Block release:** Prevents bad data shipment; requires incident response.
- **Quarantine:** Isolate bad data, route consumers to previous good version.

## Tooling

### Schema Registries

**Confluent Schema Registry** (most mature):
- Topic-level schema management.
- Compatibility modes: BACKWARD, FORWARD, FULL, NONE.
- Subject naming strategies (TopicNameStrategy, RecordNameStrategy).
- Version tracking; schema IDs embedded in Kafka message headers.

```
curl -X POST http://localhost:8081/subjects/customer-value/versions \
  -H "Content-Type: application/vnd.schemaregistry.v1+json" \
  -d '{"schema":"{\"type\":\"record\",\"name\":\"Customer\",\"fields\":[{\"name\":\"id\",\"type\":\"string\"}]}"}'
```

**Apicurio Registry** (alternative OSS):
- Supports Avro, JSON Schema, Protobuf, GraphQL.
- Rules engine for governance (compatibility, content validation).

### Data Quality & Testing Tools

- **dbt** (SQL-based): tests are queries; contracts in YAML.
- **Great Expectations** (Python): extensive constraint library, integrates with Airflow/dbt.
- **Soda** (YAML): SLO definitions, monitoring integrations.
- **Elementary**: dbt-native observability (schema changes, anomalies, lineage).

### Data Catalogs

- **Collibra, Alation**: Commercial; strong governance, contract tracking.
- **Datahub** (LinkedIn, OSS): Contract metadata, lineage, impact analysis.
- **Apache Atlas**: Hadoop ecosystem lineage and governance.

**Integration pattern:** Catalog indexes contracts; CI/CD validates against catalog.

## Production Patterns

### Rolling Deployments

Change schema gradually to minimize blast radius:

```
1. Add new column (optional) in v1.1
   → Consumers can ignore it safely
   
2. After grace period, mark old column as deprecated
   → Consumers are warned to migrate
   
3. Remove old column in v2.0
   → Only breaking to consumers who didn't migrate
   → Minor blast radius if migration was tracked
```

### Contract Tests as CI/CD Gates

```yaml
# .github/workflows/contract_validation.yaml
name: Data Contract Validation
on:
  pull_request:
    paths:
      - 'data/schemas/**'
      - 'data/models/**'

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Validate schema compatibility
        run: |
          python scripts/check_schema_compatibility.py \
            data/schemas/canonical_customer.yaml
      - name: Run dbt tests
        run: dbt test --select contract:enforced
      - name: Run Great Expectations
        run: great_expectations checkpoint run contract_checks
```

### Consumer Onboarding

New consumers should verify compatibility before consuming:

```python
# consumer_verify.py
import schema_registry
registry = schema_registry.SchemaRegistry(url="http://registry:8081")

# Fetch latest schema
latest_schema = registry.get_latest_schema("canonical_customer-value")

# Verify local code is compatible
local_schema = parse_schema_from(MyCustomerClass)
compatibility = registry.check_compatibility(
    local_schema, latest_schema, mode="BACKWARD"
)
assert compatibility, "Consumer code is incompatible with latest schema"
```

## Failure Modes & Lessons

### Contracts as Bureaucracy

**Failure:** Contracts become checklist; teams add soulless SLOs ("99.9% uptime"), tests, versioning without actual business understanding or enforcement.

**Symptom:** Contracts exist but are ignored (data breaks, alerts are muted, no investigation).

**Fix:** Contracts must be tied to **observable consequences** (downstream job failures, user-facing impacts). SLOs picked based on actual consumer needs, not arbitrary percentiles.

### Schema Drift

**Failure:** Producers add/remove columns without versioning; consumers break silently.

**Fix:** Schema registry mandatory (can't publish without compatibility check). Monitoring on schema change events.

### Over-Versioning

**Failure:** Every bugfix or minor column addition triggers MAJOR version bump; versioning becomes noisy.

**Fix:** Establish clear semantic versioning discipline. Non-breaking changes (new optional columns) don't require consumer action—use MINOR bumps. Monitor adoption of new versions before removing old ones.

### Orphaned Contracts

**Failure:** Old data product versions have no known consumers; version chains are unclear.

**Fix:** Contract metadata includes consumer registry. Automated removal of unused versions (with grace period). Catalog tracking and alerts for stale contracts.

## See Also

- [data-engineering-governance.md](data-engineering-governance.md) — Data governance, catalogs, and lineage
- [data-data-mesh-practice.md](data-data-mesh-practice.md) — Data mesh implementation patterns and challenges
- [data-engineering-quality.md](data-engineering-quality.md) — Data quality validation and observability
- [api-versioning.md](api-versioning.md) — API versioning patterns (analogous to data contracts)
- [testing-contract.md](testing-contract.md) — Consumer-driven contract testing (related pattern)