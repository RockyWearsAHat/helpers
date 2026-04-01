# Prometheus Deep Dive — Pull Model, PromQL, Recording Rules, Federation & Long-Term Storage

## Pull-Based Architecture

Prometheus uses a **pull model**, meaning Prometheus discovers and scrapes targets rather than targets pushing metrics to Prometheus. This inverts traditional monitoring: targets expose a `/metrics` HTTP endpoint in Prometheus text exposition format; Prometheus periodically connects to each target, reads the response, parses it, and stores the data in its TSDB.

### Why Pull Over Push

**Push model disadvantages**: Push requires every client to know where the metrics sink lives (configuration complexity), handle network retries (distributed logic), and deal with backpressure if the sink can't keep up. Multiple independent producers complicate aggregation.

**Pull model advantages**: Prometheus is the single source of truth for what to monitor (declarative scrape configs). Easy to test targets locally (`curl localhost:8080/metrics`). Load balancing and service discovery are managed centrally. A single Prometheus instance can scrape thousands of targets. If Prometheus is down, targets aren't affected—metrics just don't get stored until Prometheus restarts.

**Cost**: Pull requires all targets to be consistently reachable. Targets on unreliable networks (edge devices, IoT) may be missed. For these, **Pushgateway** is a compromise: short-lived jobs push to the gateway, Prometheus scrapes the gateway. Pushgateway is an anti-pattern for continuous services.

## Scrape Mechanics

Each scrape target has:

- **scheme**: HTTP or HTTPS
- **address**: hostname:port or IP:port
- **path**: defaults to `/metrics`, configurable
- **interval**: how often to scrape (global default ~15s, per-target override possible)
- **timeout**: how long to wait for response (default ~10s)
- **sample limit**: max samples per scrape (prevents disk exhaustion from misbehaving exporter)

Prometheus performs service discovery (static, DNS, Consul, Kubernetes, cloud provider APIs) to populate the list of targets, then scrapes each in parallel. Failed scrapes are logged and marked with `up` metric = 0.

## PromQL Query Language

PromQL is Prometheus's query language. It operates on time series (metric name + label set → sequence of values over time).

### Data Types

- **Instant vector**: Single value per time series at a point in time. `http_requests_total` evaluates to current value. Used for alerts, debugging.
- **Range vector**: Sequence of values for a time range. `http_requests_total[5m]` returns 5 minutes of samples. Used in functions like `rate()`.
- **Scalar**: A single floating-point number. Result of `count()`, arithmetic, comparisons.
- **String**: Rarely directly returned by queries; used in label manipulation.

### Instant Queries

```promql
http_requests_total                          # All time series matching the metric
http_requests_total{job="api"}               # Filter by exact label match
http_requests_total{job=~"api.*"}            # Regex match
http_requests_total{status!="200"}           # Label != value (exclude)
http_requests_total{status!~"2.."}           # Regex != (exclude)
{__name__=~"http_.*"}                        # Regex on metric name
http_requests_total offset 5m                # Evaluate 5m ago
```

### Range Queries

```promql
rate(http_requests_total[5m])                # Per-second rate over last 5m
increase(http_requests_total[1h])            # Total increase over 1h
irate(http_requests_total[5m])               # Instant rate (ignores first sample)
delta(temperature[1h])                       # Change in gauge over 1h
deriv(cpu_usage[10m])                        # Per-second derivative
avg_over_time(cpu_usage[5m])                 # Average over time window
```

**rate() vs irate()**: `rate()` is resistant to scrape jitter by fitting a linear regression; `irate()` uses only the last two samples. `rate()` is smoother for alerting (fewer false positives), `irate()` is more responsive to rapid changes.

### Aggregation

```promql
sum(http_requests_total)                     # Sum across all time series
sum by (job) (http_requests_total)           # Group by job label, sum within groups
sum without (instance) (http_requests_total) # Sum across all labels except instance
topk(5, http_requests_total)                 # Top 5 by value
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
```

**histogram_quantile()** is crucial for percentile queries. Histograms emit `_bucket`, `_count`, `_sum` suffixes. Buckets are **cumulative**. `histogram_quantile()` uses linear interpolation between buckets:

```
rate(requests_duration_bucket{le="0.1"}[5m])  = 100  # 100 reqs/s < 100ms
rate(requests_duration_bucket{le="0.5"}[5m])  = 150  # 150 reqs/s < 500ms
rate(requests_duration_bucket{le="1.0"}[5m])  = 180  # 180 reqs/s < 1s

histogram_quantile(0.95, ...)                      # 950ms (interpolated)
```

### Joins & Filtering

```promql
node_memory_MemFree_bytes / 1024 / 1024           # Scalar arithmetic
count(http_requests_total)                        # Number of time series
count(http_requests_total) > 10                   # Boolean filter
vector(0)                                         # Create constant scalar
http_requests_total > bool 100                    # Boolean (1 or 0) instead of drop
sort_desc(http_requests_total)                    # Sort descending
```

## Recording Rules

Recording rules pre-compute expensive PromQL expressions and store results as new metrics. They're defined in YAML:

```yaml
groups:
- name: api_slos
  interval: 30s  # Evaluate every 30s
  rules:
  - record: job:http_requests:rate5m
    expr: sum by (job) (rate(http_requests_total[5m]))
  
  - record: job:http_requests:errors:rate5m
    expr: sum by (job) (rate(http_requests_total{status=~"5.."}[5m]))
  
  - record: job:http_error_ratio:rate5m
    expr: |
      (
        job:http_requests:errors:rate5m
        /
        job:http_requests:rate5m
      )
```

**Use cases**: Reduce query latency (pre-compute), reduce cardinality (aggregate labels), simplify alert expressions, use in other rules.

**Naming convention** (underscore-separated, hierarchical): `level:metric:aggregation_period`. Example: `job:requests:rate1m`.

## Alerting Rules

Alert rules define conditions that trigger notifications:

```yaml
groups:
- name: api_alerts
  interval: 1m
  rules:
  - alert: HighErrorRate
    expr: |
      (
        sum by (job) (rate(http_requests_total{status=~"5.."}[5m]))
        /
        sum by (job) (rate(http_requests_total[5m]))
      ) > 0.05
    for: 5m  # Must be true for 5m before firing
    labels:
      severity: critical
      team: platform
    annotations:
      summary: "Error rate > 5% for {{ $labels.job }}"
      description: "Current rate: {{ $value | humanizePercentage }}"
```

**for clause**: Prevents alert flapping by requiring the condition to be true for a minimum duration. Alert enters `PENDING` state during the duration, then `FIRING` when the duration elapses.

**Labels and annotations**: Labels are used for routing, silencing, and grouping in Alertmanager. Annotations provide human-readable context.

## Federation

Federation allows a Prometheus instance ("federation server" or "hierarchical" Prometheus) to scrape another Prometheus to aggregate metrics across regions/clusters:

```yaml
# Global Prometheus in us-west
scrape_configs:
- job_name: federation
  static_configs:
  - targets: ['us-east:9090', 'eu-central:9090']
  metric_path: /federate  # Special endpoint
  params:
    match[]:  # Which metrics to federate
      - '{job=~"api|database"}'
```

The federated Prometheus at `us-east:9090` exposes `/federate?match[]=...` which streams all metrics matching the selector. The federation server scrapes this endpoint and ingests the results.

**Limitations**: Federation scales to ~10s of Prometheus instances; beyond that use Thanos or Cortex. Cross-region queries are complex (each query must fan out to multiple Prometheus instances). Doesn't solve long-term retention.

## Remote Write & Thanos

### Remote Write

Prometheus can write metrics to long-term storage backends via the `remote_write` config:

```yaml
global:
  remote_write:
  - url: http://thanos-receiver:19291/api/v1/write
    write_relabel_configs:
    - source_labels: [__name__]
      regex: 'expensive_.*'
      action: drop  # Don't ship expensive metrics to remote
```

Prometheus buffers writes locally, retries on failure, and can queue up to thousands of samples. Remote write is **asynchronous**—a failed remote write doesn't block scraping.

Remote write enables **separation of concerns**: short-term local TSDB for rapid queries, remote store for long-term retention. It also enables **multi-cluster aggregation**: multiple Prometheus instances write to the same remote store.

### Thanos

Thanos is a distributed query engine and long-term storage layer for Prometheus:

- **Sidecar**: Runs alongside Prometheus, exposes same query API, uploads TSDB blocks to object storage (S3, GCS, Azure Blob) every ~2 hours.
- **Store**: Queries archived blocks from object storage.
- **Query**: Frontend that merges results from Sidecars and Store nodes. Deduplicates metrics from replicated Prometheus instances.
- **Compactor**: Compacts and downsamples old blocks (reduce from 15s → 5m → 1h resolution as data ages).

The sidecar doesn't replace Prometheus—it runs alongside it. Prometheus serves queries locally (recent data), and Thanos routes old queries to the store.

**Deduplication**: If you run two Prometheus instances with the same scrape targets (for HA), Thanos can detect duplicates via a `__replica` label and keep only one.

### Cortex / Mimir

Cortex (now Grafana Mimir after acquisition) is a horizontally scalable multi-tenant TSDB:

- **Ingester**: Receives metrics from Prometheus remote_write, holds in memory, writes to object storage.
- **Querier**: Queries both in-memory (recent) and object storage (historical).
- **Any-to-any replication**: Each ingester replicates to N peers; losing one ingester doesn't lose data.

Mimir is purpose-built for cloud-native multi-tenant scenarios (Grafana Cloud uses it). Simpler to operate than Thanos for single-tenant cases (no sidecars, no separate compaction).

## Time Series Cardinality

Cardinality is the number of unique label combinations. `http_requests_total{method, endpoint, status, instance}` with millions of endpoints and instances can explode to billions of metric combinations.

High cardinality **burns disk** (TSDB size), **slows queries** (more series to scan), and **increases memory** (index grows). Prometheus has a `-tsdb.max-exemplars` limit and optional `relabel_configs` to drop unwanted labels early:

```yaml
metric_relabel_configs:
- source_labels: [handler]
  regex: '.*'  # Match all
  action: drop  # Drop the handler label entirely
```

Best practice: Label on what you query and aggregate on, not per-request identifiers.

## Backups and High Availability

Prometheus is stateful but designed for single-node operation. For HA, run two Prometheus instances scraping the same targets in parallel, each with remote write to shared storage. Use Thanos Query on top to deduplicate.

Snapshots (`/api/v1/admin/tsdb/snapshot`) create read-only copies of TSDB blocks for backup, but aren't incremental—production backups typically use block uploads to object storage via Thanos/Mimir.

## Related Topics

See also: [observability-metrics.md](observability-metrics.md), [observability-alerting.md](observability-alerting.md), [devops-prometheus-grafana.md](devops-prometheus-grafana.md), [architecture-data-pipeline.md](architecture-data-pipeline.md).