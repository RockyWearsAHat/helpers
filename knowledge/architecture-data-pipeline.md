# Data Pipeline Architecture — Batch, Streaming, Lambda & Kappa Patterns

## Overview

**Data pipeline architecture** describes the design of systems that reliably transform, move, and process data from sources to destinations at scale. The architecture choice fundamentally shapes latency, consistency guarantees, operational complexity, and fault tolerance.

Modern systems rarely use a single pattern; they compose batch, streaming, and micro-batch approaches depending on latency requirements, data volume, and use-case SLAs.

---

## Processing Models

### Batch Processing

Data arrives in chunks; processed all-at-once on a schedule (hourly, daily, weekly).

```
Source → Collect N hours of data → Process → Store results → Available for analysis
```

**Characteristics**:
- High throughput; process large volumes efficiently
- Ordered processing; deterministic results across runs
- High latency; results available hours later
- Handles late/out-of-order data naturally
- Simple fault recovery; replay entire batch

**Trade-offs**:
- Users see data updates as discrete events (midnight report available next morning)
- Complex exactly-once semantics simpler in batch (transaction boundaries coincide with batch boundaries)
- Infrastructure scales well (Hadoop, Spark batch jobs)

**Example use cases**: Daily reporting, EOD reconciliation, historical analysis, ML training data generation

### Stream Processing

Data processed as individual events or micro-batches immediately upon arrival.

```
Source → Event 1 → Process → Result
      → Event 2 → Process → Result
      → Event 3 → Process → Result
```

**Characteristics**:
- Low latency; results available within seconds
- High throughput (Kafka can handle millions of events/sec)
- Handles unbounded data streams (never ends)
- Harder exactly-once semantics; requires deduplication or idempotency
- Windowing required for aggregations over time (5-minute rolling count, daily total)
- Complex state management (aggregations, joins across streams)

**Windows for aggregations**:
- **Tumbling**: Fixed-size non-overlapping intervals (every 1 minute)
- **Sliding**: Overlapping intervals (every 1 minute, 5-minute window)
- **Session**: Event-driven; close when inactivity exceeds threshold
- **Global**: Aggregate all events (risky for unbounded streams)

**Trade-offs**:
- Immediate results enable real-time dashboards, alerting
- Exactly-once requires careful design; at-least-once easier but requires idempotent sinks
- State explosion risk; storing millionth unique user in session window memory
- Ordering guarantees depend on topology (single partition ordered; multi-partition out-of-order)

**Example use cases**: Real-time dashboards, fraud detection, user activity tracking, metrics aggregation

### Micro-Batch Processing

Hybrid: collect small batches (seconds to minutes) and process each independently.

```
0s-1s:   Events collected → Process batch 1
1s-2s:   Events collected → Process batch 2
...
```

**Advantages**:
- Simpler exactly-once than pure streaming (batch boundaries)
- Lower latency than daily batch
- Easier state management than continuous streams
- Good fit for Apache Spark Streaming, Flink mini-batches

**Trade-off**:
- Latency bounded by micro-batch interval (sub-second micro-batches have overhead)
- Adds queuing delay

---

## Consistency Models

### At-Most-Once

Events processed 0 or 1 times. If failure occurs, event may be lost.

```
Source → Process → Sink
           ↓ (crash)
         Result lost, never reprocessed
```

**Use case**: Loss acceptable; live metrics that decay (user scrolling speed).

**Easy to implement**: No deduplication, no idempotency.

### At-Least-Once

Events processed ≥1 times. Failures cause reprocessing; duplicates possible.

```
Source → Process → Sink
           ↓ (crash, mid-sink)
         Reprocess; sink twice
```

**Use case**: Loss unacceptable; duplicates manageable.

**Requires**: Idempotent sinks (writes safe to repeat) or deduplication.

### Exactly-Once

Events processed exactly once, even across failures. Most complex; rarely free.

**Implementations**:
1. **Transactional writes**: Source, processing, sink all in single transaction. (Only for small systems.)
2. **Idempotent processing + deduplication**: Sink stores event ID; rejects duplicates. (Kafka exactly-once, Flink)
3. **Distributed snapshots**: Periodically checkpoint state; on failure restore to checkpoint + reprocess since checkpoint. (Flink's Chandy-Lamport algorithm)

**Cost**: Added latency, state overhead, complexity.

---

## Architectural Patterns

### Lambda Architecture

Combine batch and streaming layers to reconcile latency and correctness.

```
Source → Batch Layer  → Batch Views  ↘
      ↘                              → Serving Layer → Query
         Speed Layer (Stream) → Stream Views ↗
```

**How it works**:
1. **Batch layer** (nightly): Process all historical data exhaustively, store in batch views
2. **Speed layer** (continuous): Process recent events, maintain in-memory approximate views
3. **Serving layer**: Query returns `batch_view + stream_corrections`

**Example**: Suppose yesterday's totals are in batch views; today's estimated total is batch + today's streaming sum

**Pros**:
- Correctness from batch; freshness from stream
- Independent failure modes; stream failure doesn't invalidate historical batch

**Cons**:
- Operational complexity; two code paths (batch logic + stream logic) must stay synchronized
- Debugging hard; results come from both layers
- Resource overhead; running both continuously
- Often contradicted by experience; streaming layer lags, reconciliation messy

**Verdict**: Lambda architecture has fallen out of favor; most systems are stream-first now.

### Kappa Architecture

Replace batch layer with a replayable event log; single streaming layer processes all events.

```
Source → Event Log (Kafka, Event Hubs) ↔ Stream Processor → Views
```

All data stored in log. Stream processes in real-time. To recalculate: replay log, recompute views.

**Pros**:
- Single code path; batch reprocessing and stream processing use same logic
- Event log is source of truth; recovering from failures = replaying log
- Simpler operational model than Lambda

**Cons**:
- Requires event log to be append-only and replayable (Kafka works; OLTP database doesn't)
- Recalculating views from scratch = full replay (hours for large logs)
- State management more complex; streaming system must handle state rebuilds efficiently

**When it works well**:
- Events naturally append-only (user clicks, transactions, sensor readings)
- Re-derivable views (don't need historical intermediate states)
- Storage cheap enough to keep logs long-term

---

## ETL vs ELT

See data-engineering-etl.md for detailed comparison. Brief summary:

| ETL | ELT |
|-----|-----|
| Extract → Transform (external) → Load | Extract → Load → Transform (in warehouse) |
| Compute-intensive transformation before loading | Warehouse does transformation |
| Network cost low, compute cost high | Network cost high, warehouse compute cost lower |
| Schema defined before load | Schema evolves post-load |

ELT emerged as cloud warehouses cheapened compute and decoupled compute from storage.

---

## Data Quality & Correctness

### Schema Evolution

Data sources change schema over time (new fields, renamed columns). Pipeline must adapt.

**Strategy**: 
- Preserve raw data in landing zone (ELT approach)
- Detect schema changes (monitoring, version tracking)
- Update downstream views incrementally
- Version schema in lineage

### Late & Out-of-Order Data

Real-world streams have delays (network latency, clock skew, batching upstream).

**Handling**:
- Grace period: Window waits for late arrivals (5-minute window accepts data 10 seconds late)
- Side outputs: Capture data outside assigned window, preserve for manual review
- Recomputation: If late data is important, batch recalculate and merge

### Data Quality Gates

Filter, validate, and reject malformed data.

```
Pipeline:
  → Schema validation (required fields, types)
  → Anomaly detection (impossible values)
  → Freshness check (data too old?)
  → Drop or quarantine invalid records
  → Count metrics for monitoring
```

---

## Pipeline Orchestration & Lineage

See data-engineering-orchestration.md for detailed discussion.

**Orchestration** coordinates complex multi-step pipelines: dependencies, retries, SLA monitoring.

**Lineage** tracks which data tables depend on which upstream tables. Enables impact analysis (if customer table changes, which reports break?).

---

## Tool Landscape

| Category | Examples | Fit |
|----------|----------|-----|
| Batch | Spark, Hadoop, dbt, SQL | Scheduled jobs, historical reprocessing |
| Streaming | Kafka, Flink, Spark Streaming, Kinesis | Real-time processing, windowing |
| Orchestration | Airflow, Dagster, Prefect | Complex multi-step workflows |
| ELT | Stitch, Fivetran, dbt | Cloud warehouse ELT |

---

## When to Choose Each

**Batch enough?**: Data freshness acceptable at daily/hourly cadence; cost is constraint.

**Stream needed?**: Sub-second latency required, or continuous aggregate updates.

**Lambda?**: Rare; added complexity usually not worth it.

**Kappa?**: Events replayable, infrastructure supports event log, recalculation acceptable.

---

## See Also

- data-engineering-etl.md — ETL/ELT patterns
- data-engineering-orchestration.md — DAG scheduling, asset tracking
- data-engineering-quality.md — Validation and governance
- architecture-event-sourcing.md — Event-based state management
- data-engineering-streaming.md — Streaming systems design