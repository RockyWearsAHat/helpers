# Data Quality — Validation, Governance & Observability

## Data Quality Dimensions

Data quality is multidimensional — no single metric captures it. Commonly recognized dimensions, each with distinct detection and remediation characteristics:

| Dimension    | Definition                                            | Example Violation                               | Detection Approach                                 |
| ------------ | ----------------------------------------------------- | ----------------------------------------------- | -------------------------------------------------- |
| Accuracy     | Values correctly represent real-world entities        | Customer age recorded as 350                    | Range checks, cross-reference validation           |
| Completeness | Required data is present, not missing                 | Email field null for 40% of records             | Null rate monitoring, required field assertions    |
| Consistency  | Same fact represented the same way across systems     | "USA" in one table, "United States" in another  | Cross-system reconciliation, standardization rules |
| Timeliness   | Data arrives and is available within expected windows | Yesterday's transactions not loaded until noon  | Freshness monitoring, SLA tracking                 |
| Uniqueness   | No unintended duplicate records                       | Same order appearing three times                | Deduplication checks, key uniqueness assertions    |
| Validity     | Values conform to defined formats and domains         | Phone number with letters, date as "2024-13-45" | Schema validation, regex patterns, enum checks     |

These dimensions interact — data can be complete but inaccurate, timely but inconsistent. Quality assessment requires evaluating dimensions relevant to each use case rather than optimizing all dimensions uniformly.

## Schema Validation — Enforcing Contracts at Boundaries

Schema validation verifies that data conforms to a structural contract — expected columns, data types, nullability, and value constraints. The question is where in the pipeline to enforce it.

### Validation Points

- **At ingestion**: Reject or quarantine malformed records before they enter the system. Prevents downstream corruption but may cause data loss if rejection is overly strict.
- **At transformation boundaries**: Validate inputs and outputs of each transformation stage. Catches logic errors within the pipeline itself.
- **At serving layer**: Final gate before data reaches consumers. Last defense but late — errors discovered here may have propagated through multiple stages.

### Schema Evolution Challenges

Schemas change as business requirements evolve. Approaches to managing evolution:

- **Forward compatibility**: New data can be read by old consumers (adding optional fields).
- **Backward compatibility**: Old data can be read by new consumers (new fields have defaults).
- **Full compatibility**: Both directions supported simultaneously.

Schema registries maintain versioned schema definitions and enforce compatibility rules, preventing breaking changes from reaching production. The strictness of compatibility enforcement trades flexibility for safety.

### Structural vs Semantic Validation

Schema validation catches structural issues (wrong type, missing column) but not semantic ones (a valid integer that represents an impossible business value). Semantic validation requires domain-specific rules layered on top of structural checks.

## Data Contracts

Data contracts formalize the agreement between data producers and consumers, specifying:

- **Schema**: Column names, types, nullability
- **Semantics**: What each field means, valid value ranges
- **SLAs**: Freshness guarantees, availability windows
- **Ownership**: Who is responsible when the contract breaks
- **Versioning**: How and when contracts change

Contracts shift quality responsibility leftward — producers commit to delivering data meeting specified criteria, rather than consumers defensively coding around whatever arrives.

Implementation approaches range from lightweight (documented conventions enforced by tests) to formal (machine-readable contract definitions integrated into CI/CD pipelines that block deployments breaking contracts).

Trade-offs: formal contracts add overhead to producer teams and can slow iteration if the change process is heavyweight. Informal contracts provide flexibility but degrade into undocumented assumptions. The appropriate formality level depends on organizational scale and the cost of quality failures.

## Statistical Profiling

Statistical profiling goes beyond schema validation to characterize the distribution and behavior of data values:

### Distribution Checks

- **Value distribution**: Histograms, percentiles, standard deviation. A sudden shift in distribution (e.g., average order value doubling) may indicate upstream changes or errors.
- **Cardinality monitoring**: Tracking the number of distinct values. A dimension that usually has 50 values suddenly having 5,000 suggests a join issue or data corruption.
- **Pattern analysis**: Frequency of value patterns (e.g., email format distribution, phone number prefixes) reveals format drift.

### Anomaly Detection

Statistical anomaly detection identifies deviations from established baselines:

- **Z-score / sigma-based**: Flag values beyond N standard deviations from the mean. Simple but assumes normal distribution.
- **Interquartile range (IQR)**: Flag values outside 1.5× IQR from quartiles. More robust to non-normal distributions.
- **Time-series decomposition**: Separate trend, seasonality, and residual components. Anomalies in the residual signal unexpected changes.
- **Machine learning approaches**: Isolation forests, autoencoders, or clustering on feature vectors derived from data profiles. More flexible but require training data and introduce model maintenance.

### Null Rate Monitoring

Null rates are among the simplest and most informative quality signals. Tracking null percentages per column over time reveals:

- Source system changes (a previously-required field becoming optional)
- ETL bugs (a join producing unexpected nulls)
- Upstream data quality degradation

A column that is 0.1% null suddenly becoming 15% null warrants investigation regardless of whether the schema permits nulls.

## Referential Integrity Across Distributed Systems

In monolithic databases, foreign key constraints enforce referential integrity automatically. In distributed architectures — microservices, event-driven systems, polyglot persistence — referential integrity becomes an application-level concern.

Challenges:

- **Eventual consistency**: In systems using asynchronous replication or event sourcing, references may temporarily point to not-yet-propagated entities.
- **Cross-system references**: An order service referencing a customer ID owned by a customer service cannot enforce foreign keys across database boundaries.
- **Soft deletes vs hard deletes**: Deleting a referenced entity in one system while references persist in another creates orphaned references.

Approaches to managing distributed referential integrity:

| Approach                    | Mechanism                                                          | Trade-off                         |
| --------------------------- | ------------------------------------------------------------------ | --------------------------------- |
| Event-driven reconciliation | Consume entity lifecycle events to maintain local reference tables | Added latency and complexity      |
| Periodic batch validation   | Regularly compare reference sets across systems                    | Delayed detection                 |
| API-time validation         | Check references via API call before writing                       | Added latency per write, coupling |
| Saga patterns               | Multi-step transactions with compensating actions                  | Complex error handling            |
| Tolerant readers            | Accept potentially invalid references, resolve lazily              | Risk of serving stale references  |

No approach fully replicates the guarantees of a single-database foreign key. The choice depends on the business impact of referential inconsistency and the acceptable latency for detection and correction.

## Master Data Management

Master data management (MDM) addresses the challenge of maintaining a single, consistent representation of core business entities (customers, products, locations) across multiple systems.

### MDM Styles

- **Registry style**: Each system maintains its own version; a central registry maps and links them without storing a golden copy. Lightweight but resolution logic is at read time.
- **Consolidation style**: Systems write independently; a central process periodically merges and deduplicates into a golden record. Batch-oriented, with a delay between source change and master update.
- **Coexistence style**: Bidirectional synchronization between source systems and a central master. Complex synchronization logic, conflict resolution challenges.
- **Centralized style**: The master system is the only write location for entity data. Strongest consistency but imposes constraints on all producing systems.

### Entity Resolution

A core MDM challenge is determining which records across systems represent the same entity. Techniques include:

- Exact matching on stable identifiers (SSN, business registration number)
- Fuzzy matching on name, address, and other attributes (Levenshtein distance, phonetic algorithms)
- Probabilistic matching using weighted scoring across multiple attributes
- Machine learning classifiers trained on labeled match/non-match pairs

False positives (merging distinct entities) and false negatives (failing to link the same entity) have different business costs. Tuning the match threshold requires understanding these asymmetric costs.

## Data Governance

Data governance establishes the organizational framework for managing data as an asset:

### Ownership and Stewardship

- **Data owners**: Business stakeholders accountable for data quality and appropriate use within their domain. Make policy decisions.
- **Data stewards**: Operational roles that implement and enforce data quality rules, resolve issues, and maintain metadata. Bridge between business and technical teams.
- **Technical custodians**: Engineering teams responsible for infrastructure, access controls, and pipeline reliability.

Clear ownership prevents the diffusion-of-responsibility problem where data quality is "everyone's job" and therefore no one's.

### Lineage

Data lineage tracks the origin, movement, and transformation of data through systems:

- **Column-level lineage**: Which source columns contribute to each target column.
- **Pipeline-level lineage**: Which jobs, transformations, and systems data passes through.
- **Business-level lineage**: How business metrics derive from raw data.

Lineage serves multiple purposes: root cause analysis when data quality degrades, impact analysis when source systems change, and compliance evidence for regulatory requirements.

Lineage can be captured through:

- Static analysis of SQL, code, and configuration
- Runtime instrumentation of execution engines
- Manual documentation (low fidelity, high maintenance)

### Access Policies

Data access governance balances availability with security:

- **Role-based access control (RBAC)**: Permissions granted by role. Simple to manage, coarse-grained.
- **Attribute-based access control (ABAC)**: Permissions based on attributes of the user, data, and context. Fine-grained but complex to reason about.
- **Column-level security**: Different users see different columns of the same table.
- **Row-level security**: Different users see different rows based on predicates.
- **Dynamic data masking**: Sensitive values masked at query time based on user privileges.

The principle of least privilege applies — users should access only the data necessary for their function. Enforcement mechanisms range from database-native features to external policy engines that intercept queries.

## Data Catalogs

Data catalogs serve as discovery tools, enabling users to find, understand, and trust available data assets.

Core catalog capabilities:

- **Search and discovery**: Find tables, columns, metrics by name, description, or tag.
- **Technical metadata**: Schema, data types, partitioning, storage location.
- **Business metadata**: Descriptions, ownership, classification, glossary terms.
- **Operational metadata**: Freshness, row counts, quality scores, usage frequency.
- **Social metadata**: User ratings, annotations, tribal knowledge captured in comments.
- **Lineage integration**: How datasets relate to upstream sources and downstream consumers.

Catalog adoption challenges include keeping metadata current (stale catalogs lose trust), motivating contribution (metadata is a public good with free-rider problems), and balancing curation effort with coverage breadth.

## Data Mesh — Domain-Oriented Ownership

The data mesh concept, articulated by Zhamak Dehghani, proposes four principles for scaling analytical data architectures:

### Domain Ownership

Analytical data responsibility shifts from a centralized data team to domain teams that produce the data. Each domain publishes its analytical data as a product, owned and maintained alongside its operational systems.

### Data as a Product

Domain teams treat their analytical data outputs with product thinking — documented, discoverable, trustworthy, with defined SLAs. Consumers should be able to use the data without contacting the producing team.

### Self-Serve Platform

A shared infrastructure platform provides domain teams with tools for publishing, discovering, and consuming data products without requiring deep infrastructure expertise. The platform handles cross-cutting concerns (storage, compute, access control, observability).

### Federated Computational Governance

Global policies (interoperability standards, security requirements, quality baselines) are defined centrally but enforced computationally — through automated validation rather than manual review processes.

### Tensions and Considerations

Data mesh introduces organizational and technical challenges: domain teams need data engineering skills, cross-domain queries require interoperability standards, and the "self-serve platform" is a substantial engineering investment. The approach assumes sufficient organizational scale that centralized data teams have become bottlenecks — for smaller organizations, the overhead may exceed the benefit.

## Testing Data Pipelines

Data pipelines benefit from testing strategies adapted from software engineering, though the nature of what is being tested differs:

### Unit Tests for Transformations

Test individual transformation functions with known inputs and expected outputs:

```
-- Transformation: calculate order total with tax
-- Input: order_amount = 100.00, tax_rate = 0.08
-- Expected: total = 108.00
-- Edge cases: null tax_rate, negative amounts, zero amounts
```

Unit tests for transformations verify business logic correctness independent of infrastructure. They run fast and catch regression when transformation logic changes.

### Integration Tests for Pipeline Stages

Verify that stages connect correctly — output of one stage is valid input for the next. These tests exercise actual infrastructure (databases, file systems, message queues) and catch issues like:

- Schema mismatches between stages
- Encoding/serialization issues
- Permission and connectivity problems
- Timestamp/timezone handling across system boundaries

### Expectation-Based Validation

Rather than testing logic, expectation-based validation asserts properties of the data itself:

- Row count within expected range
- No null values in required columns
- Value distributions within historical norms
- Referential integrity across tables
- Aggregates matching known invariants (e.g., daily totals reconcile with source system)

These expectations run as part of the pipeline — after a stage completes, expectations are evaluated before data propagates downstream. Failures can halt the pipeline, quarantine bad data, or alert operators depending on severity configuration.

### Test Data Management

Testing pipelines requires representative test data. Approaches include:

- **Synthetic generation**: Fabricate data matching expected distributions. Avoids privacy concerns but may miss real-world edge cases.
- **Sampled production data**: Statistically representative subsets. Realistic but requires anonymization.
- **Recorded fixtures**: Snapshots of known-good data for deterministic tests. Stable but may drift from current production patterns.

## The Cost of Bad Data

Data quality failures compound through downstream systems:

- **Direct costs**: Incorrect reports, failed automated processes, manual remediation effort.
- **Decision costs**: Business decisions based on inaccurate data — mispriced products, misallocated resources, incorrect forecasts.
- **Trust costs**: When users encounter quality issues, they lose confidence in the data platform and revert to spreadsheets and tribal knowledge. Rebuilding trust is harder than building it.
- **Compliance costs**: Regulatory penalties for incorrect reporting, privacy violations from misclassified data.
- **Opportunity costs**: Engineering time spent firefighting quality issues instead of building new capabilities.

The "1-10-100 rule" (attributed to George Labovitz and Yu Sang Chang) suggests it costs $1 to prevent a quality issue, $10 to detect and correct it, and $100 to deal with its downstream consequences. While the specific ratios vary by context, the exponential cost growth with detection delay is consistently observed.

Quality is a continuous investment: data sources change, business definitions evolve, pipeline complexity grows. A one-time quality initiative degrades without ongoing monitoring, maintenance, and organizational commitment.

## Privacy Considerations

Data quality and data privacy intersect in multiple ways:

### PII Detection

Personally identifiable information (PII) may appear in unexpected places — free-text fields, log messages, error outputs, derived columns. Detection approaches include:

- **Pattern matching**: Regular expressions for structured PII (email addresses, phone numbers, social security numbers). High recall for known patterns, no coverage for unstructured PII.
- **Named entity recognition (NER)**: NLP models that identify names, addresses, and other entities in free text. Broader coverage but with false positive/negative rates.
- **Classification models**: Machine learning classifiers trained on labeled PII/non-PII datasets. Adaptable to domain-specific PII patterns.
- **Metadata-based inference**: Column names, descriptions, and data types suggest PII presence (e.g., a column named "ssn" or "date_of_birth").

### Data Masking

Masking replaces sensitive values with realistic but non-identifying substitutes:

| Technique                    | Mechanism                                                                | Reversibility                        | Utility Preservation                                    |
| ---------------------------- | ------------------------------------------------------------------------ | ------------------------------------ | ------------------------------------------------------- |
| Substitution                 | Replace with fake but format-consistent values                           | Irreversible (if mapping not stored) | High — format and distribution preserved                |
| Shuffling                    | Rearrange values within a column across rows                             | Reversible if seed known             | Moderate — distribution preserved, relationships broken |
| Nulling                      | Replace with null or constant                                            | Irreversible                         | Low — column becomes unusable for analysis              |
| Tokenization                 | Replace with opaque tokens, mapping stored securely                      | Reversible with mapping              | Low for analysis, high for system testing               |
| Format-preserving encryption | Encrypt while maintaining format (e.g., 16-digit number stays 16 digits) | Reversible with key                  | Moderate — format preserved, values meaningless         |

### Anonymization and Pseudonymization

Anonymization removes the ability to identify individuals from the dataset. Pseudonymization replaces identifiers with artificial ones, maintaining linkability with a separate mapping.

Key considerations:

- **Re-identification risk**: Even "anonymized" datasets can be re-identified through linkage attacks combining quasi-identifiers (zip code + birth date + gender). k-anonymity, l-diversity, and t-closeness are frameworks for quantifying and mitigating this risk.
- **Utility-privacy trade-off**: Stronger anonymization reduces data utility. Differential privacy provides a mathematical framework for this trade-off, adding calibrated noise that bounds individual disclosure risk while preserving aggregate statistical properties.
- **Regulatory requirements**: Different jurisdictions define anonymization differently. What qualifies as anonymous under one regulation may be considered pseudonymous under another.

### Privacy by Design in Data Pipelines

Integrating privacy considerations into pipeline design rather than applying them retroactively:

- Minimize collection — ingest only what is necessary for defined purposes
- Separate PII from analytical attributes early in the pipeline
- Apply masking/anonymization at ingestion or the earliest feasible stage
- Implement retention policies — automated deletion when data exceeds its purpose window
- Audit access — log who accessed what data and when, with sufficient granularity for compliance reporting

The tension between data quality improvement (which benefits from access to raw, complete data) and privacy protection (which benefits from minimization and restriction) requires explicit organizational decisions about acceptable trade-offs.
