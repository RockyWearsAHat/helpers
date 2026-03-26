# Feature Store Architecture — Online, Offline & Point-in-Time Correctness

## Overview

**Feature store** is a centralized system that computes, stores, versions, and serves features (derived input variables) to machine learning models. It bridges data engineering and ML: managing raw data pipelines upstream, providing consistent features to training and inference pipelines downstream.

A feature store solves three problems: **computation** (how to compute features efficiently), **consistency** (same feature definition in training and serving), and **latency** (serve precomputed features at inference time, not compute on request).

---

## The Feature Store Problem

### Training-Serving Skew

A common source of model failure: features computed differently (or from different data) in training vs. production inference.

**Example**:
- Training: Feature `user_purchase_count` = historical user purchases in training data (Dec 2020)
- Serving: Feature `user_purchase_count` = purchases up to request time (Jan 2024)
- Model trained on 2020 data; inference uses 2024 data; performance degrades

Feature store enforces identical computation: one definition, one implementation.

### Feature Reuse

Features are expensive to compute and valuable across many models. Centralized store enables feature sharing: compute once, reuse many times. Without it, each ML team computes overlapping features independently.

### Point-in-Time Correctness

When training on historical data, features must reflect the **state at prediction time**, not current state.

```
Training: For user at 2022-06-01, compute feature = purchases prior to 2022-06-01
Serving: For user at 2024-01-15, compute feature = purchases prior to 2024-01-15
```

Feature store tracks time; queries return feature values as-of a target timestamp.

---

## Dual-Layer Architecture

Feature stores use two storage layers:

### Offline Store (Batch, Historical)

Large-scale historical data for training. Stores feature matrices (entity_id × timestamp × feature).

**Technology**: ParquetRow, data lake table, BigQuery, Snowflake, Hive

**Access pattern**: Full-table scans for training dataset generation

**Computation**: Batch job runs nightly or on-demand to compute historical features

```
Job runs 2024-01-15, computes features for all experiments:
  User 1: [2024-01-14, 2024-01-13, ..., 2022-06-01] → 600 days of daily features
  User 2: [2024-01-14, 2024-01-13, ..., 2022-06-01] → 600 days of daily features
  ...
  Result: 1M users × 600 days = 600M feature rows
  
Stored: Parquet partitioned by date, entity_type
```

**Advantages**:
- High throughput; batch compute is efficient
- Historical time-travel for backtesting
- Cost-effective storage (object store)
- Full feature matrices enable research

**Trade-offs**:
- High latency; new features available hours later
- Recomputation expensive; recalculating 600M rows takes hours
- Schema evolution complex; appending new features to historical table

### Online Store (Low-Latency Serving)

Small subset of recent features for real-time inference. Stores entity_id → latest_feature_values.

**Technology**: Redis, DynamoDB, Firestore, in-memory cache

**Access pattern**: Single entity lookup (get user 42's features)

**Computation**: Streaming or scheduled syncs from offline store

```
User 42: {
  purchase_count_7d: 5,
  avg_purchase_value: 95.32,
  last_purchase_time: 2024-01-14T15:32:00Z,
  risk_score: 0.12
}

Latency: <10ms lookup
```

**Advantages**:
- Sub-millisecond latency for inference
- Scalable; horizontal sharding by entity_id
- Real-time updates possible (streaming)

**Trade-offs**:
- Limited historical data
- Time-travel queries difficult (only recent state)
- Storage expensive per-entity; can't store full history

---

## Feature Computation Strategies

### Batch Computation

Compute features on a fixed schedule (nightly, hourly), store in offline/online stores.

```
Data source → Aggregation job → Offline store (persist)
                             ↓
                        Sync to online store (sample/recent)
```

**Fit**: Historical patterns, windowed aggregates (7-day purchase count)

**Latency**: Features available next batch interval (hours)

**Freshness**: Acceptable for most models; updates lag

### Streaming Computation

Real-time aggregators process events as they arrive, maintain online store state.

```
User event stream → Aggregation operator → State (Redis) → Query
                        (rolling count, avg)
```

Using Kafka Streams, Flink, or Spark Structured Streaming:

```python
# Flink: 7-day sliding window of purchase value
events.keyBy('user_id') \
    .window(SlidingEventTimeWindows.of(7 days, 1 day)) \
    .aggregate(sum_purchase_value) \
    .sink(redis)
```

**Fit**: Real-time aggregates, low-cardinality entities

**Latency**: Sub-second updates

**Challenges**: State explosion (unique users = memory); order-of-magnitude more complex

### On-Demand Computation

Compute features on-the-fly at inference time.

```
Request: user_id=42 → compute_feature(user_id) → Return feature → Inference
Latency: ≤100ms if computation is fast
```

**Fit**: Complex computations that can't be pre-aggregated; rare queries

**Risk**: Latency spike if computation slow; CPU overhead; cache required

---

## Point-in-Time Correctness

The core mechanism ensuring training-serving consistency.

### Training Phase

For each training example (entity, target_timestamp):
1. Query offline store: fetch feature values as-of target_timestamp
2. Combine features into feature vector
3. Include label (did user purchase in next 7 days?)

```sql
-- Pseudo-query: Get features for user 1 as of 2023-06-01
SELECT purchase_count_7d, avg_purchase_value, risk_score
FROM feature_store_offline
WHERE entity_id = 1 AND timestamp = '2023-06-01'
```

**Result**: Training data reflects how model would have seen data in June 2023.

### Serving Phase

For inference request (entity, now):
1. Query online store: fetch current feature values
2. Pass feature vector to model
3. Return prediction

**Consistency**: Same feature definition, computed identically (batch → sync → serve)

### Handling Delays

Real-world feature values may have delays (upstream data late, aggregation window).

```
User purchase at 2024-01-14 20:00 UTC
Logged to data warehouse at 2024-01-14 22:00 UTC (2hr delay)
Aggregated in batch job at 2024-01-15 02:00 UTC
Synced to online store at 2024-01-15 02:15 UTC
Model inference at 2024-01-15 15:00 UTC sees early observation

Depending on model sensitivity, may be acceptable or requires buffering/explicit delay
```

---

## Feature Versioning & Lineage

Features evolve: definition changes, bugs fixed, new implementations.

**Versioning strategy**:

```yaml
feature: "purchase_count_7d"
versions:
  v1: "Simple 7-day sum (2023-01-01 → 2024-01-01)"
  v2: "Fixed: exclude cancelled orders (2024-01-02 →)"
  v3: "Optimized: batch compute instead of streaming (2024-02-01 →)"
```

**Registration**: Feature catalog stores definition, owner, SLA, lineage

```
purchase_count_7d ← events table ← raw.events
                 ← config.window_size
                 ↓ uses aggregation→_utils.sum_7d
```

---

## Popular Feature Store Systems

| System | Approach | Fit |
|--------|----------|-----|
| Feast (open-source) | Unified batch + streaming, modular orchestration | Self-hosted, modular, open |
| Tecton (commercial) | Batch + streaming, end-to-end, no-code UI | Fully managed, enterprise SLAs |
| Hopsworks | Distributed offline + online, built on Spark/Hive | Feature eng collaboration, complex transformations |
| Databricks Feature Store | Integrated with Databricks workspace, Delta Lake | Existing Databricks users |
| BigQuery ML Feature Store | Offline-only, built on BigQuery | BigQuery-centric teams |

---

## Operational Considerations

### Monitoring Features

- Freshness: How recent is online store data? (should match SLA)
- Completeness: What % of entities have features? Missing = inference stalls
- Distribution shift: Feature values changing unexpectedly (stale upstream data?)
- Staleness: Online store lags offline store; mismatch at serving time

### Backfill & Recomputation

When adding new feature or fixing bug: recompute historical values.

```
Job: Backfill purchase_count_7d for all users, 2022-01-01 to 2024-01-01
Cost: 600M rows × compute time (~hours)
Risk: If calculation buggy, all historical training data corrupted
```

**Safeguards**: Version-tag features during backfill, keep old version until validated

### Consistency Between Stores

Offline and online stores can drift. Reconciliation needed:
- Offline updated nightly; online gets updates via batch sync lag
- Streaming updates online; batch updates offline; temporary divergence
- Validation: Query both stores, assert feature values match (within tolerance)

---

## When to Build vs. Buy

**Build**: Small team, simple features (basic SQL aggregates), control required

**Buy (Feast, Tecton, Hopsworks)**: Complex features, need multiple teams, compliance/versioning critical

---

## See Also

- ml-feature-engineering.md — Feature engineering principles and transforms
- ml-operations.md — Model serving, inference infrastructure
- data-engineering-etl.md — Feature computation pipelines
- data-engineering-orchestration.md — Scheduling and orchestrating computations
- data-engineering-streaming.md — Real-time aggregation systems