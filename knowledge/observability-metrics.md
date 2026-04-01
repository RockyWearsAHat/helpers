# Observability: Metrics Systems

Metrics are numerical measurements aggregated over time. Unlike logs (discrete events) and traces (request journeys), metrics capture _system state_ at scale: CPU utilization, request count, queue depth, error rates.

## Dimensional vs Hierarchical Metrics

**Dimensional metrics** attach labels (dimensions) to each observation. Same metric with different label values creates separate time series.

```
http_requests_total{method="GET",endpoint="/api/users",status="200"} = 1000
http_requests_total{method="GET",endpoint="/api/users",status="404"} = 5
http_requests_total{method="POST",endpoint="/api/users",status="200"} = 100

Query: http_requests_total{endpoint="/api/users"} returns all three series
Query: http_requests_total{endpoint="/api/users",status="200"} returns first series
```

**Hierarchical metrics** embed dimensions in the metric name (statsd-style):

```
http.requests.get.api_users.200 = 1000
http.requests.get.api_users.404 = 5
http.requests.post.api_users.200 = 100

Query: must specify exact name; aggregation requires post-processing
```

**Dimensional advantages:**
- Flexible queries; discover patterns retrospectively
- Aggregation at query time (`sum by (status)`)
- Same storage for all dimensions
- Dimensionality enables alerting on derived metrics

**Dimensional disadvantages:**
- High cardinality explosion: if each user's request is a dimension, storage explodes exponentially
- Query complexity; teams need PromQL literacy
- Requires discipline: limit cardinality by dropping or aggregating high-volume dimensions

**Hierarchical advantages:**
- Simple naming, no query language
- Guaranteed cardinality by design (name is fixed)
- Lower barrier to entry

**Hierarchical disadvantages:**
- Retroactive aggregation impossible; must pre-compute
- Adding new dimension requires name change (schema evolution)
- Dashboards rigid; can't pivot on new attributes

**Recommendation:** Dimensional metrics for primary systems (Prometheus). Hierarchical for high-volume event streams (StatsD to StatsD backends or pre-aggregated).

## Prometheus Data Model

Prometheus is **pull-based dimensional metrics**.

### Architecture

```
┌────────────────────────────────────────────┐
│  Application Targets                       │
│  ┌─ Instance A: /metrics endpoint ──────┐  │
│  │  http_requests_total 1500             │  │
│  │  request_duration_seconds 0.45        │  │
│  └─────────────────────────────────────┘  │
└────────────────────────────────────────────┘
              ▲ HTTP GET scrape (every 15s)
              │
┌────────────────────────────────────────────┐
│  Prometheus Server                         │
│  ┌──────────────────────────────────────┐  │
│  │ TSDB (Time-Series Database)          │  │
│  │ Stores samples: (timestamp, value)   │  │
│  │ for each series                      │  │
│  └──────────────────────────────────────┘  │
│  ┌──────────────────────────────────────┐  │
│  │ Rules Evaluator                      │  │
│  │ (Evaluation of recording & alert)    │  │
│  └──────────────────────────────────────┘  │
└────────────────────────────────────────────┘
         │ alert rules
         ▼
┌────────────────────────────────────────────┐
│  Alertmanager                              │
│  Routes to PagerDuty, Slack, email         │
└────────────────────────────────────────────┘
```

**Pull model advantages:**
- Simple deployment; targets don't need to know Prometheus exists
- Load shedding: if Prometheus scraper is slow, target continues normally
- Service discovery integration (Kubernetes, EC2)

### Text Exposition Format

```
# HELP http_requests_total Total HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",endpoint="/api/orders",status="200"} 1234 1234567890
http_requests_total{method="GET",endpoint="/api/orders",status="404"} 5 1234567890

# TYPE request_duration_seconds histogram
request_duration_seconds_bucket{instance="10.0.0.1",le="0.1"} 50
request_duration_seconds_bucket{instance="10.0.0.1",le="0.5"} 150
request_duration_seconds_bucket{instance="10.0.0.1",le="1.0"} 200
request_duration_seconds_bucket{instance="10.0.0.1",le="+Inf"} 210
request_duration_seconds_sum{instance="10.0.0.1"} 90.5
request_duration_seconds_count{instance="10.0.0.1"} 210
```

**Metrics endpoint:** Every Prometheus-instrumented service exposes `GET /metrics` with this format.

## Metric Types

### Counter

Monotonically increasing value; resets welcome (process restart).

```python
REQUEST_COUNT = Counter('http_requests_total', 'Total requests', ['method', 'status'])
REQUEST_COUNT.labels(method='GET', status='200').inc(1)
REQUEST_COUNT.labels(method='GET', status='200').inc(5)  # +5
# Total: 6
```

**Semantics:** "How many times has this happened?" Total bytes transferred, total errors, total jobs completed.

**PromQL:**
```
rate(http_requests_total[5m])  # requests per second over last 5 minutes
increase(http_requests_total[1h])  # total increase over 1 hour
```

### Gauge

Value that goes up and down; represents current state.

```python
QUEUE_SIZE = Gauge('job_queue_depth', 'Current queue size')
QUEUE_SIZE.set(42)
QUEUE_SIZE.set(40)  # decreased
```

**Semantics:** "How much right now?" Memory usage, temperature, active connections, pool utilization.

**PromQL:**
```
job_queue_depth  # current value
delta(job_queue_depth[5m])  # change over 5 minutes
deriv(job_queue_depth[5m])  # rate of change (derivative)
```

**Caveat:** Cannot use `rate()` on gauges; rate() assumes counter semantics (always increase).

### Histogram

Observations bucketed into configurable ranges. Enables percentile calculation.

```python
REQUEST_DURATION = Histogram(
    'http_request_duration_seconds',
    'Request duration',
    buckets=[0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0, float('inf')]
)

with REQUEST_DURATION.time():
    handle_request()  # duration recorded

# For 0.23s request:
# Increments buckets 0.5, 1.0, 5.0, +Inf
# bucket{le="0.23"} is NOT incremented
```

**Backend datastore:**
```
http_request_duration_seconds_bucket{le="0.01"} 10
http_request_duration_seconds_bucket{le="0.05"} 25
http_request_duration_seconds_bucket{le="0.1"} 50
http_request_duration_seconds_bucket{le="0.5"} 200
http_request_duration_seconds_bucket{le="1.0"} 210
http_request_duration_seconds_bucket{le="+Inf"} 212
http_request_duration_seconds_sum 145.3
http_request_duration_seconds_count 212
```

**PromQL:**
```
# p95 latency over 5 minutes
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Average (sum / count)
rate(http_request_duration_seconds_sum[5m]) / rate(http_request_duration_seconds_count[5m])
```

**Disadvantage:** Bucket boundaries must be chosen upfront. Mis-calibrated buckets hide detail.

### Summary

Client-side percentile calculation; Prometheus stores pre-computed quantiles.

```python
RPC_DURATION = Summary('rpc_duration_seconds', 'RPC latency', quantile=[0.5, 0.9, 0.99])
# SDK computes and stores p50, p90, p99 at scrape time
```

**Why avoid:** Quantiles are not aggregatable across instances. Histograms are preferred for server-side aggregation.

## Recording Rules and Aggregation

**Recording rule:** Pre-computed query stored as a metric.

```yaml
# prometheus.yml rules section
rule_files:
  - /etc/prometheus/rules.yml

---
# rules.yml
groups:
  - name: aggregation
    interval: 30s  # evaluate every 30 seconds
    rules:
      - record: service:http_requests:rate1m
        expr: rate(http_requests_total[1m])
      
      - record: service:http_errors:ratio
        expr: sum by (service) (rate(http_requests_total{status=~"5.."}[5m]))
             / sum by (service) (rate(http_requests_total[5m]))
      
      - record: node:cpu:ratio
        expr: 100 - (avg by (node) (rate(node_cpu_seconds_total{mode="idle"}[5m])) * 100)
```

**Effect:** Prometheus evaluates the `expr` every 30 seconds and stores the result as a new time series.

```
service:http_requests:rate1m{service="checkout"} 50  # requests/sec
service:http_requests:rate1m{service="auth"} 200
service:http_errors:ratio{service="checkout"} 0.01  # 1% error rate
```

**Benefit:** Faster dashboards (pre-aggregated). Reduce cardinality explosion (aggregate high-cardinality series into low-cardinality summaries).

## Cardinality Management

**Cardinality:** Number of unique label combinations for a metric.

```
http_requests_total with labels {method, endpoint, status}:
200 HTTP methods × 1000 endpoints × 10 statuses = 2,000,000 combinations
```

**At scale, high cardinality is the primary cost driver:**
- Memory usage (index of all label combinations)
- Query latency (scan all matching series)
- Disk I/O
- Backup/restore complexity

### Mitigation Strategies

1. **Choose label dimensions wisely.** Avoid user ID, customer ID, request ID as labels; these are unbounded.

```
WRONG:
  payment_transaction{user_id=1234, amount=99.5, currency="USD"}  # unbounded user_id

CORRECT:
  payment_transaction_total{currency="USD"}  # aggregate across users
  # If user-specific metrics needed, store in logs or a separate trace system
```

2. **Drop or aggregate high-cardinality labels at scrape time.**

```yaml
global:
  external_labels:
    cluster: prod

scrape_configs:
  - job_name: app
    relabel_configs:
      # Drop pod ID if present
      - source_labels: [__meta_kubernetes_pod_name]
        action: drop
        regex: "prod-worker-.*"
      
      # Keep only service-level labels
      - source_labels: [__meta_kubernetes_namespace, __meta_kubernetes_pod_label_app]
        target_label: app
        regex: "(.+);(.+)"
        replacement: "${1}/${2}"
```

3. **Use recording rules to pre-aggregate to lower cardinality.**

```yaml
- record: http_requests:by_service:rate1m
  expr: sum by (service) (rate(http_requests_total[1m]))
```

Now consumers use `http_requests:by_service:rate1m` (low cardinality) instead of `http_requests_total` (high cardinality).

## Long-Term Storage: Thanos, M3, Cortex

Prometheus default storage retains ~15 days locally. Long-term systems extend retention and enable multi-Prometheus federation.

### Thanos (CNCF Incubating)

**Architecture:** Sidecar proxy to existing Prometheus. Uploads blocks to object storage (S3, GCS).

```
Prometheus 1 + Thanos Sidecar ──┐
Prometheus 2 + Thanos Sidecar ──┤
Prometheus 3 + Thanos Sidecar ──┤──→ Thanos Query (federation)
                                 │
                            Object Storage
                          (S3, GCS, etc)
```

**Advantages:**
- Non-invasive (sidecar pattern; Prometheus unchanged)
- Cheap storage (object storage cost-effective)
- Downsampling: 1h data aggregated to daily blocks
- Query federation across all Prometheus instances

**Disadvantage:** Added operational complexity; separate storage tier.

### M3 (Uber, Mature)

**All-in-one:** Replaces Prometheus. Write amplification through aggregation and rollup.

```
Write: metric → M3 Aggregator → Raw storage (1m resolution)
                            → Rollup to 10m resolution
                            → Rollup to 1h resolution
```

**Advantages:**
- Native multi-tenancy
- Tunable aggregation (balance cost vs precision)
- Strong cardinality controls

**Disadvantage:** Operational overhead; fork from Prometheus ecosystem.

### Cortex (CNCF, Graduated)

**Prometheus-compatible API**, multi-tenant, horizontally scalable.

```
Write path: Distributor → Ingester → DynamoDB/Cassandra
Read path: Querier ← Ingester cache
```

**Advantages:**
- Drop-in replacement for Prometheus (same API)
- Multi-tenant isolation
- CloudNative; designed for Kubernetes

**Disadvantage:** Operational complexity; separate infrastructure.

**Comparison:**
| System | Setup | Tenancy | Storage | Query Federation |
|--------|-------|---------|---------|---|
| Prometheus | 5min | Single | Local disk | No |
| Prometheus + Thanos | 30min | Single | Object storage | Yes |
| M3 | 1hr | Multi | Configurable | Native |
| Cortex | 1hr | Multi | Pluggable | Native |

## StatsD: Lightweight Metrics Collection

**Protocol:** UDP-based, fire-and-forget metrics protocol. Minimal overhead.

```ruby
# Client sends single UDP packet per metric
StatsD.increment('requests.total')
StatsD.gauge('memory.usage_bytes', 512000)
StatsD.histogram('request_duration_ms', 250)
StatsD.set('unique_users', user_id)
```

**Wire format:**
```
metric_name:value|type[|@sample_rate]

requests.total:1|c        # counter increment
memory.usage_bytes:512000|g  # gauge set
request_duration_ms:250|ms   # histogram (milliseconds)
unique_users:user123|s      # set (count unique values)
unique_users:user456|s
request.total:1|c|@0.1     # counter increment, sampled at 10%
```

**Server (e.g., StatsD, Graphite, InfluxDB) aggregates UDP packets and flushes to backend.**

**Advantages:**
- Minimal latency (UDP, no waiting for response)
- Minimal client overhead
- No backpressure (fire and forget)

**Disadvantages:**
- UDP packet loss possible (lossy delivery)
- Server-side aggregation required (clients can't query)
- Cardinality explosion if not careful with tags

**Use case:** High-frequency metrics from application code where precision matters less than speed. Event counters, timing distributions in hot paths.

## OpenTelemetry Metrics (OTLP)

**OTLP** (OpenTelemetry Protocol) provides standardized metrics collection and export.

```python
from opentelemetry import metrics
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.metrics.export import PeriodicExportingMetricReader
from opentelemetry.exporter.otlp.proto.http.metric_exporter import OTLPMetricExporter

exporter = OTLPMetricExporter(endpoint="http://otel-collector:4318/v1/metrics")
meter_provider = MeterProvider(metric_readers=[PeriodicExportingMetricReader(exporter)])
metrics.set_meter_provider(meter_provider)

meter = metrics.get_meter("app")
request_counter = meter.create_counter("http.requests")
request_counter.add(1)
```

**Format:** gRPC or HTTP Protocol Buffers; richer than Prometheus text format.

**Advantages:**
- Vendor-neutral; routes to any backend via collector
- Structured; supports semantic conventions

**Disadvantage:** Smaller ecosystem than Prometheus native instrumentation.

## Semantic Conventions

OpenTelemetry and observability community define standard metric names and labels:

```
HTTP:
  http.server.request.duration_seconds
  Labels: http.method, http.target, http.scheme, http.status_code

Database:
  db.query.duration_seconds
  Labels: db.system (postgres, mysql), db.name, db.operation

RPC:
  rpc.server.duration_seconds
  Labels: rpc.service, rpc.method
```

**Benefit:** Consistent naming across services and backends. Dashboards and alerts portable.

## See Also

- observability-distributed-tracing (trace-derived metrics)
- devops-prometheus-grafana (PromQL recipes, dashboard design)
- sre-slo-engineering (metric-based SLI/SLO construction)