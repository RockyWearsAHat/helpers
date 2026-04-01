# Multi-Tenancy Architecture — Database Isolation, Tenant Scoping & SaaS Patterns

## Overview

**Multi-tenancy** describes applications serving multiple independent customers (tenants) from shared infrastructure. The core architectural challenge: isolated data, isolated configuration, and isolated performance _without_ replicating the entire system for each tenant.

Multi-tenancy enables SaaS economics—spreading infrastructure costs across customers while maintaining isolation boundaries. The trade-off: increased complexity in data scoping, performance noisy-neighbor prevention, and compliance segregation.

---

## Database Isolation Models

The choice of tenant isolation at the database level shapes the entire stack. Three primary approaches dominate:

### Shared Database, Shared Schema (Row-Level Isolation)

All tenants store data in the same database within the same tables. A `tenant_id` column on every table filters data at the application layer.

```sql
-- customers table
CREATE TABLE customers (
  id BIGINT,
  tenant_id BIGINT,
  name VARCHAR,
  PRIMARY KEY (tenant_id, id),  -- composite key for scoping
  INDEX idx_tenant (tenant_id)
);

SELECT * FROM customers WHERE tenant_id = ? AND id = ?;
```

**Strengths**:
- Simplest infrastructure; one database instance to operate
- Easy backups, monitoring, maintenance
- Cost-efficient at large scale
- Connection pooling shared across tenants

**Weaknesses**:
- Query isolation is application-enforced; SQL errors or ORM mistakes leak data
- Debugging harder; every query requires `tenant_id` filtering
- Noisy neighbor risk severe; one tenant's workload impacts all others
- Enforcement requires discipline; subqueries or joins easily omit the filter
- Compliance harder (data residency, regulatory isolation)
- Scaling requires horizontal sharding; unsharded single database has throughput ceiling

**Mitigation strategies**:
- Enforce `tenant_id` in application ORM layer or database policies
- Connection pooling with per-tenant credential mapping (rare; mostly application-side scoping)
- Row-level security (RLS) policies in PostgreSQL/Oracle to enforce filtering at the database
- Strict query review in code

### Shared Database, Separate Schema (Schema-Level Isolation)

Each tenant owns a separate schema within the same database. Data is logically isolated; queries are tenant-specific.

```sql
-- create schema per tenant
CREATE SCHEMA tenant_acme_corp;
CREATE TABLE tenant_acme_corp.customers (...);

-- queries scoped to schema
SET SEARCH_PATH TO tenant_acme_corp;
SELECT * FROM customers WHERE id = ?;
```

**Strengths**:
- Schema-level isolation; accidental cross-tenant queries prevented by search_path
- Easier compliance (data residency visible at schema layer)
- Schema can be customized per tenant (add columns, indexes)
- Tenants can be restored individually without affecting others
- Connection pooling still shared, reducing overhead

**Weaknesses**:
- Single database still has throughput ceiling
- Schema creation, migration, and cleanup at scale requires orchestration
- Monitoring becomes complex; hundreds/thousands of schemas
- Backup/restore more complex than shared schema
- Still vulnerable to noisy neighbor at the instance level

**Migration complexity**:
- Migrating schema structure across hundreds of tenants is a challenge (idempotent schema migrations essential)
- Requires careful coordination to avoid lock contention during peak hours

### Dedicated Database (Per-Tenant Isolation)

Each tenant has their own database instance (or cluster). Complete isolation at the database level.

```
Tenant A: RDS instance A (pgbench, backups, reads)
Tenant B: RDS instance B (independent configuration)
Tenant C: RDS instance C (different region for GDPR)
```

**Strengths**:
- Maximum isolation; noisy neighbor impossible
- Regulatory compliance easy (tenant A's data never touches tenant B's infrastructure)
- Per-tenant configuration, backup frequency, read replicas, failover strategy
- Performance SLAs enforceable independently
- Data residency enforceable (EU tenant in EU region)
- Scaling horizontally trivial

**Weaknesses**:
- Infrastructure cost per tenant; expensive at scale unless tenants are large
- Operational overhead; hundreds of database instances to manage, monitor, patch
- Resource utilization inefficient; small tenants have underutilized instances
- Backup, recovery, and failover management complex
- Connection pooling inefficient; many small connection pools
- Migration and upgrades slow (upgrade each tenant independently or orchestrate)

---

## Hybrid Models & Tiering

Production SaaS systems often tier tenants:

- **Starter/free tier**: Shared database, row-level isolation (cost-optimized)
- **Standard tier**: Shared schema within database (better isolation, moderate cost)
- **Enterprise tier**: Dedicated database or multi-region cluster (maximum isolation, compliance)

Tenants can migrate between tiers as they grow. This balances cost and isolation.

---

## Tenant Onboarding & Offboarding

### Onboarding

**Shared schema approach**:
1. Create schema for tenant (or reserve one if pooled)
2. Run migration scripts (DDL) in tenant's schema
3. Bootstrap seed data (configuration, templates)
4. Register tenant in routing/discovery service
5. Activate billing

**Dedicated database approach**:
1. Provision database instance (RDS, CloudSQL, etc.)
2. Run all migrations
3. Bootstrap seed data
4. Update DNS/routing
5. Run smoke tests
6. Activate billing

### Offboarding

**Data retention policies**:
- Soft delete (mark tenant deleted, retain data for compliance/recovery window)
- Hard delete (zero-knowledge; cryptographically destroy)
- Export (offer tenant data export before deletion)
- Archive (move to S3 for regulatory retention)

Database cleanup:
- **Shared schema**: DELETE FROM all_tables WHERE tenant_id = ? (wrapped in transaction)
- **Dedicated database**: DROP SCHEMA CASCADE or terminate entire instance

---

## Data Partitioning & Performance

### Noisy Neighbor Prevention

A single tenant's query spike can impact others in shared infrastructure.

**Strategies**:

1. **Query limits per tenant**: Rate limit or abort queries exceeding time thresholds
2. **Connection pooling limits**: Cap connections per tenant
3. **Resource isolation**: Database resource groups (limit CPU/memory per tenant)
4. **Caching**: Cache per-tenant aggregates to reduce query load
5. **Read replicas**: Offload analytics queries from primary

### Sharding for Scale

Shared row-level or schema-level isolation hits throughput ceiling at single database. Solution: partition tenants across multiple database instances.

```
Tenants 1-100 → Database A
Tenants 101-200 → Database B
Tenants 201-300 → Database C
```

Requires:
- Consistent hashing or shard map to route queries
- Backfill/rebalancing when adding shards
- Cross-shard joins become distributed, complex
- Transactions across tenants now difficult

---

## Tenant-Aware Caching

Cache multi-tenant data carefully:

```python
# Cache key includes tenant_id
cache_key = f"tenant:{tenant_id}:customers"
result = cache.get(cache_key)
if not result:
    result = db.query(f"SELECT * FROM customers WHERE tenant_id = {tenant_id}")
    cache.set(cache_key, result, ttl=300)
```

**Considerations**:
- Invalidation: One tenant's write invalidates only that tenant's caches
- Capacity planning: Cache size grows with number of tenants
- TTL tuning: Fresh data vs. cache hit rate
- Consistency: Eventually consistent caches may serve stale data during updates

---

## Configuration & Feature Flags

Tenants often need custom features, workflows, or integrations:

- **Configuration per tenant**: Database table storing tenant preferences
- **Feature flags**: Gradual rollout of features to subset of tenants
- **Custom fields**: EAV (entity-attribute-value) tables for tenant-specific columns
- **Webhooks & API extensions**: Allow tenants to integrate external systems

```sql
CREATE TABLE tenant_features (
  tenant_id BIGINT,
  feature_flag VARCHAR,
  enabled BOOLEAN,
  created_at TIMESTAMP
);
```

---

## Compliance & Data Residency

**SOC 2, GDPR, HIPAA considerations**:
- Data residency: Tenant data must stay in specific regions (EU for GDPR, US for HIPAA)
- Audit logs: Tenant-specific audit trails
- Encryption: Per-tenant encryption keys (or shared with careful key rotation)
- Access control: Segregate tenant data from support/operations teams (careful debugging)

---

## When to Choose Each Model

| Model | Use Case |
|-------|----------|
| Row-level shared | High-volume SaaS, small-medium tenants, cost optimization priority |
| Schema-level shared | Medium mix of tenant sizes, moderate isolation needed, schema customization acceptable |
| Dedicated database | Enterprise customers, regulatory isolation mandatory, performance SLAs critical |
| Hybrid tiering | Freemium model, range of customer sizes and budgets |

---

## See Also

- architecture-data-mesh.md — Federated ownership of data domains (tenant ownership model)
- database-sharding.md — Horizontal scaling techniques
- security-cloud-security.md — Compliance and isolation
- architecture-patterns.md — General system design patterns