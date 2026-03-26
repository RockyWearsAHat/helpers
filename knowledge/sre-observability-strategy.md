# SRE — Observability Strategy: Pillars, Golden Signals & Unified Telemetry

## The Three Pillars plus Profiling

Observability answers: "Why is my system behaving this way?" without needing to modify code. The traditional framework rests on three pillars; modern practice adds a fourth.

### 1. Metrics — System State Over Time

Metrics are numerical measurements aggregated over time. They answer: "What is the overall health of the system?"

Examples: CPU %, memory usage, requests/sec, error rate, latency p50/p99, database connections.

Characteristics:

- **Dimensional**: Each metric has labels (service, instance, region, user_tier)
- **Time series**: Values change over time; stored sequentially for efficient queries
- **Aggregatable**: Can sum, average, or compute percentiles across dimensions
- **Cardinality**: Care needed; too many unique label combinations explodes storage

Tools: Prometheus, Datadog, CloudWatch, InfluxDB

### 2. Logs — Discrete Events with Context

Logs record what happened at specific moments. They answer: "What events occurred when?"

Structured logging (JSON/key-value) is essential:

```json
{
  "timestamp": "2026-03-25T14:30:00Z",
  "level": "ERROR",
  "service": "payment-api",
  "request_id": "abc123",
  "user_id": "user456",
  "error": "payment gateway timeout",
  "latency_ms": 5000,
  "retry_count": 2
}
```

**Cardinality remains a problem**: High-dimensionality logs (one log per request ID, user ID) can generate billions of unique combinations. Storage costs explode.

Pattern: Sample logs (keep 1% of normal requests, 100% of errors) to control volume.

Tools: ELK, Datadog, Splunk, Loki

### 3. Traces — Request Journeys Across Services

Traces follow a single request through multiple services, answering: "What happened to this specific request and where did it get slow?"

A trace contains:

- **Trace ID**: Unique ID for the entire request journey
- **Spans**: Each service records its work as one or more spans (start time, duration, metadata)
- **Parent-child relationships**: Frontend span spawns backend span, backend spawns database span

Example timeline:

- Frontend receives request (span 1: 100ms)
  - Calls backend service (span 2: 150ms — includes network time)
    - Calls database (span 3: 80ms — slow, waterfall problem)
    - Calls cache (span 4: 10ms — miss)
  - Calls recommendation service (span 5: 90ms)
  - Aggregates results (span 6: 20ms)
- Frontend returns response

**Sampling critical**: Sampling 100% of traces is expensive and unnecessary. Common pattern: Sample errors at 100%, normal requests at 0.1%-1%.

Tools: Jaeger, Zipkin, DataDog, AWS X-Ray

### 4. Profiling Plus Metrics/Logs/Traces

Profiling dives deeper than metrics, answering: "What *inside* the service is consuming resources?"

Types:

- **CPU profiling**: Which functions are consuming CPU? Call stacks + counts
- **Memory profiling**: Which allocations are consuming heap? Object counts + sizes
- **I/O profiling**: Which syscalls are slow? Or lock contention?

Unlike metrics (aggregated), profiling shows individual stack traces. Used to investigate specific performance regressions.

Tools: pprof (Go), Java Flight Recorder, py-spy, Linux perf

## Unified Telemetry: OpenTelemetry & OTLP

Rather than vendor silos (Datadog SDK, New Relic SDK, Prometheus exporter), **OpenTelemetry (OTel)** provides vendor-neutral APIs:

```go
import "go.opentelemetry.io/otel"

// Single instrumentation, output to any backend
tracer := otel.Tracer("my-app")
ctx, span := tracer.Start(context.Background(), "my-operation")
defer span.End()

meter := otel.Meter("my-app")
counter := meter.NewInt64Counter("requests.total")
counter.Add(ctx, 1)
```

### OTLP — OpenTelemetry Protocol

Standard protocol for exporting telemetry. Collector receives metrics, logs, traces, and forwards to multiple backends:

```
App (OTel SDK) → OTLP/HTTP → OTel Collector → Prometheus, Jaeger, AWS X-Ray, Datadog, etc.
```

Benefits:

- **Vendor independence**: Swap backends without changing app code
- **Standardization**: Single API across metrics, logs, traces
- **Sampling decisions**: Collector can sample centrally (+10% for backend performance)

### Instrumentation Levels

1. **Automatic**: Language agents (Java Flight Recorder, Node.js APM) capture telemetry without code changes
2. **Library**: App uses OTel SDK; developer calls `tracer.Start()`, `counter.Add()`
3. **Manual**: Custom instrumentation for domain logic ("user_tier" labels, business metrics)

Common pattern: Auto + library for infrastructure; manual for business.

## Service Dependency Mapping

Large systems have many services. Dependency maps (from traces) reveal:

- Which services call which
- Latency contribution of each hop
- Failure correlation (when service X fails, which others are affected?)

Generated from spans:

```
API Gateway
├── User Service
├── Product Service
│   └── Inventory Service
└── Payment Service
    └── Banking Gateway
```

Queries: "What is the latency contribution of Inventory Service? If Inventory is down, which critical paths are affected?" Enables blameless diagnosis.

## Business KPIs in Observability

Product-driven KPIs belong alongside infrastructure metrics. Examples:

- **Conversion rate**: Orders completed / checkout starts (% for key step)
- **Cart abandonment**: Carts opened → Payment page → Order confirmed (drop-off points)
- **Feature adoption**: % of users using new feature within first week
- **User retention**: % of users in cohort returning after day/week/month

Tied to technical signals:

- High checkout latency correlates with cart abandonment
- Search service errors → lower conversion rate
- New feature with missing telemetry → can't measure adoption

### Implementation

Instrument app to emit business metrics alongside technical ones:

```python
# Technical metric
request_latency.observe(elapsed_ms)

# Business metric
if checkout_step == "payment":
    checkout_stage.labels(stage="payment").inc()
    
if order_completed:
    revenue.observe(order_total)
    orders_by_region.labels(region=region).inc()
```

## Golden Signals: Key Metrics Every Service Should Track

Google's SRE book standardizes four golden signals for every service:

1. **Latency**: Response time of requests (p50, p99)
2. **Traffic**: Request volume (requests/sec, transactions/sec)
3. **Errors**: Error rate (5xx, timeouts, failed transactions)
4. **Saturation**: Resource utilization (CPU, memory, disk, network, queue depth)

Any service dashboard should show these four. If any is abnormal, you can rapidly diagnose:

- High latency + low saturation = external dependency slow
- High latency + high saturation = resource-bound (scale or optimize)
- High errors + normal latency = code bug or test traffic
- High traffic + errors = load spike (trigger autoscaling / load shedding)

Alerts typically trigger on:

- Latency p99 > 200ms (depends on SLO)
- Error rate > 1% (1% burn of error budget/hour)
- Saturation > 70% (headroom for spikes)

## Observability vs. Monitoring

**Monitoring**: Predefined queries (dashboards, alerts). You know in advance what to measure.

**Observability**: Arbitrary queries on high-dimensional data. You can ask questions post-incident without re-instrumenting.

Distinction:

- Monitoring: "Is CPU < 70%?" → dashboard
- Observability: "Why did this specific user's checkout take 5s?" → trace, logs, metrics together

Modern stacks support both. Prometheus (monitoring) + Jaeger (observability) + Loki (high-cardinality logs) = comprehensive.

## Cost Control

Telemetry is expensive. Strategies to manage cost without losing visibility:

### Sampling

- **Metrics**: Always collect (low cardinality)
- **Logs**: Sample high-volume streams (1% of normal requests, 100% of errors)
  - Query: sample_rate = error ? 1.0 : 0.01
- **Traces**: Sample errors at 100%, normal at 0.1%, tail-based sampling for slow traces
  - Head-based: Sampling decision at ingestion (before seeing full trace)
  - Tail-based: Wait for full trace, sample if slow or has errors

### Cardinality Limits

Cap dimensions on high-volume metrics:

- User ID as dimension: "requests_by_user" has high cardinality → ❌
- User tier as dimension: "requests_by_tier" has 3-5 values → ✓
- Request path: "requests_by_path" has finite set → ✓
- Request query string: High cardinality → ❌ (aggregate; don't store)

### Retention Tiers

- Metrics: 13 months (annual patterns important)
- Traces: 7 days (recent issues matter; older traces rarely helpful)
- Logs: 30 days (legal, support reasons) → archive to cold storage after
- Profiles: On-demand, not continuous (run during investigation)

## Observability Challenges at Scale

1. **Cardinality explosion**: Running services generate millions of unique log sequences/trace paths
2. **Cost**: Exabytes of data generated per day; storage + querying is expensive
3. **Spike detection**: Millions of metrics; which changes are anomalous? (requires ML)
4. **Correlation**: Incident spans multiple services; finding root cause among billions of events is hard
5. **Freshness vs. cost**: High-cardinality real-time data is expensive; delayed analysis is cheaper but less useful

## See Also

- [Logging & Observability](logging-observability.md) — Detailed logging strategies
- [Devops — OpenTelemetry](devops-opentelemetry.md) — OTel architecture and configuration
- [Observability: Distributed Tracing](observability-distributed-tracing.md) — Tracing patterns and sampling
- [Observability: Metrics Systems](observability-metrics.md) — Prometheus, cardinality management