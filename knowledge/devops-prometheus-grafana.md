# Prometheus and Grafana

## Prometheus Architecture

```
┌──────────────┐     scrape      ┌──────────────┐
│  Targets     │ ◄────────────── │  Prometheus   │
│  (apps,      │                 │  Server       │
│   exporters) │                 │  ┌──────────┐ │
└──────────────┘                 │  │  TSDB    │ │
                                 │  │  (local) │ │
┌──────────────┐  push           │  └──────────┘ │
│  Pushgateway │ ◄────────────── │               │
│  (short jobs)│                 └───────┬───────┘
└──────────────┘                         │
                                         │ evaluate
                                    ┌────▼────┐       ┌──────────────┐
                                    │  Rules  │──────►│ Alertmanager │
                                    └─────────┘       └──────┬───────┘
                                                             │ route
                                                    ┌────────▼────────┐
                                                    │ Slack, PagerDuty│
                                                    │ Email, Webhook  │
                                                    └─────────────────┘
```

**Pull model**: Prometheus scrapes HTTP endpoints at configured intervals. Targets expose `/metrics` in text exposition format. Benefits: no client-side push config, easy to run locally, targets discoverable.

## Metric Types

| Type          | Description                                  | Example                             | PromQL Function         |
| ------------- | -------------------------------------------- | ----------------------------------- | ----------------------- |
| **Counter**   | Monotonically increasing. Resets on restart. | `http_requests_total`               | `rate()`, `increase()`  |
| **Gauge**     | Goes up and down.                            | `temperature_celsius`, `queue_size` | Direct value, `delta()` |
| **Histogram** | Counts observations in configurable buckets. | `http_request_duration_seconds`     | `histogram_quantile()`  |
| **Summary**   | Client-side calculated quantiles.            | `rpc_duration_seconds`              | Direct quantile labels  |

### Instrumentation

```python
# Python (prometheus_client)
from prometheus_client import Counter, Gauge, Histogram, start_http_server

REQUEST_COUNT = Counter(
    'http_requests_total',
    'Total HTTP requests',
    ['method', 'endpoint', 'status']
)

REQUEST_DURATION = Histogram(
    'http_request_duration_seconds',
    'HTTP request duration',
    ['method', 'endpoint'],
    buckets=[0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
)

QUEUE_SIZE = Gauge('job_queue_size', 'Current queue depth')

# Usage
REQUEST_COUNT.labels(method='GET', endpoint='/api/users', status='200').inc()
QUEUE_SIZE.set(42)

# Histogram context manager
with REQUEST_DURATION.labels(method='GET', endpoint='/api/users').time():
    handle_request()
```

### Histogram vs Summary

|                    | Histogram                          | Summary                        |
| ------------------ | ---------------------------------- | ------------------------------ |
| **Quantile calc**  | Server-side (PromQL)               | Client-side                    |
| **Aggregatable**   | Yes (can combine across instances) | No (pre-calculated)            |
| **Bucket config**  | Must define buckets upfront        | Quantiles defined upfront      |
| **Cost**           | Cheap per observation              | Expensive (streaming quantile) |
| **Recommendation** | Prefer for most cases              | Rarely needed                  |

## PromQL

### Core Functions

```promql
# Rate — per-second rate of counter increase (handles resets)
rate(http_requests_total[5m])

# Increase — total increase over time range
increase(http_requests_total[1h])

# irate — instant rate (last two data points, more volatile)
irate(http_requests_total[5m])

# Histogram quantile (p95 latency)
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Average request duration
rate(http_request_duration_seconds_sum[5m])
  / rate(http_request_duration_seconds_count[5m])
```

### Aggregation Operators

```promql
# Sum across all instances
sum(rate(http_requests_total[5m]))

# Sum by label (per-service request rate)
sum by (service) (rate(http_requests_total[5m]))

# Without (exclude labels from aggregation)
sum without (instance, pod) (rate(http_requests_total[5m]))

# Top 5 endpoints by request rate
topk(5, sum by (endpoint) (rate(http_requests_total[5m])))

# Count of time series matching
count(up == 1)

# Average across instances
avg by (service) (process_resident_memory_bytes)

# Quantile across instances (different from histogram_quantile)
quantile(0.95, rate(http_requests_total[5m]))
```

### Selectors and Matchers

```promql
# Label matchers
http_requests_total{method="GET", status=~"2.."}     # regex match
http_requests_total{endpoint!="/healthz"}             # not equal
http_requests_total{status!~"5.."}                    # negative regex

# Offset — query historical data
rate(http_requests_total[5m] offset 1h)

# @ modifier — query at specific timestamp
http_requests_total @ 1609459200

# Subquery
max_over_time(rate(http_requests_total[5m])[1h:1m])
# rate over 5m, sampled every 1m, max over 1h
```

### Common Query Patterns

```promql
# Error rate percentage
sum(rate(http_requests_total{status=~"5.."}[5m]))
  / sum(rate(http_requests_total[5m])) * 100

# SLO: percentage of requests under 200ms
sum(rate(http_request_duration_seconds_bucket{le="0.2"}[5m]))
  / sum(rate(http_request_duration_seconds_count[5m])) * 100

# Saturation: CPU usage approaching limit
sum by (pod) (rate(container_cpu_usage_seconds_total[5m]))
  / sum by (pod) (container_spec_cpu_quota / container_spec_cpu_period) * 100

# Disk will be full in 4 hours (linear prediction)
predict_linear(node_filesystem_avail_bytes[1h], 4 * 3600) < 0

# Absent (fire alert when metric disappears)
absent(up{job="myapp"})
```

## Recording Rules

Pre-compute expensive queries and store as new time series.

```yaml
# prometheus/rules/recording.yml
groups:
  - name: http_rules
    interval: 30s
    rules:
      - record: job:http_requests:rate5m
        expr: sum by (job) (rate(http_requests_total[5m]))

      - record: job:http_request_duration:p95
        expr: |
          histogram_quantile(0.95,
            sum by (job, le) (rate(http_request_duration_seconds_bucket[5m]))
          )

      - record: job:http_error_rate:ratio
        expr: |
          sum by (job) (rate(http_requests_total{status=~"5.."}[5m]))
            / sum by (job) (rate(http_requests_total[5m]))
```

**Naming convention**: `level:metric:operations` — e.g., `job:http_requests:rate5m`.

## Alerting Rules

```yaml
# prometheus/rules/alerts.yml
groups:
  - name: app_alerts
    rules:
      - alert: HighErrorRate
        expr: job:http_error_rate:ratio > 0.05
        for: 5m # must be true for 5 minutes
        labels:
          severity: critical
          team: backend
        annotations:
          summary: "High error rate on {{ $labels.job }}"
          description: "Error rate is {{ $value | humanizePercentage }} (>5%)"
          runbook_url: "https://wiki.internal/runbooks/high-error-rate"

      - alert: InstanceDown
        expr: up == 0
        for: 3m
        labels:
          severity: warning
        annotations:
          summary: "Instance {{ $labels.instance }} down"

      - alert: DiskSpaceLow
        expr: |
          (node_filesystem_avail_bytes{fstype!~"tmpfs|overlay"}
            / node_filesystem_size_bytes) * 100 < 10
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Disk space below 10% on {{ $labels.instance }}:{{ $labels.mountpoint }}"
```

## Alertmanager

```yaml
# alertmanager.yml
global:
  resolve_timeout: 5m
  slack_api_url: "https://hooks.slack.com/services/xxx"

route:
  receiver: default
  group_by: [alertname, cluster, service]
  group_wait: 30s # wait to batch related alerts
  group_interval: 5m # wait between batches for same group
  repeat_interval: 4h # re-send if still firing

  routes:
    - match:
        severity: critical
      receiver: pagerduty-critical
      continue: true # also send to next matching route
    - match:
        severity: critical
      receiver: slack-critical

    - match_re:
        service: (frontend|api)
      receiver: slack-backend
      routes:
        - match:
            severity: warning
          receiver: slack-backend-warnings

receivers:
  - name: default
    slack_configs:
      - channel: "#alerts"
        title: "{{ .GroupLabels.alertname }}"
        text: '{{ range .Alerts }}{{ .Annotations.summary }}\n{{ end }}'

  - name: pagerduty-critical
    pagerduty_configs:
      - routing_key: "<key>"
        severity: '{{ if eq .GroupLabels.severity "critical" }}critical{{ else }}warning{{ end }}'

  - name: slack-critical
    slack_configs:
      - channel: "#alerts-critical"

inhibit_rules:
  - source_match:
      severity: critical
    target_match:
      severity: warning
    equal: [alertname, cluster, service]
    # Suppress warnings when critical is already firing for same alert
```

## Service Discovery

```yaml
# prometheus.yml
scrape_configs:
  # Static targets
  - job_name: "app"
    static_configs:
      - targets: ["app1:9090", "app2:9090"]

  # Kubernetes service discovery
  - job_name: "kubernetes-pods"
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
        action: keep
        regex: true
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_path]
        action: replace
        target_label: __metrics_path__
        regex: (.+)

  # EC2 discovery
  - job_name: "ec2"
    ec2_sd_configs:
      - region: us-east-1
        port: 9100
    relabel_configs:
      - source_labels: [__meta_ec2_tag_Environment]
        target_label: environment

  # Consul
  - job_name: "consul"
    consul_sd_configs:
      - server: "consul:8500"
        services: ["api", "frontend"]

  # File-based (for custom integrations)
  - job_name: "file"
    file_sd_configs:
      - files: ["/etc/prometheus/targets/*.json"]
        refresh_interval: 30s
```

## Long-Term Storage: Thanos / Mimir

### Thanos

```
Prometheus ──► Thanos Sidecar ──► Object Storage (S3/GCS)
                                         │
                                  Thanos Store Gateway
                                         │
                         Thanos Querier (global PromQL view)
                                         │
                              ┌──────────┼──────────┐
                              │          │          │
                         Sidecar 1  Sidecar 2  Store GW
```

Components: **Sidecar** (ships blocks to object storage), **Store Gateway** (serves historical data from object storage), **Querier** (global view across all Prometheus + stores), **Compactor** (downsampling, deduplication), **Ruler** (distributed rule evaluation).

### Grafana Mimir

All-in-one horizontally scalable TSDB. Drop-in replacement for Prometheus remote write. Components: distributor, ingester, compactor, store-gateway, querier, query-frontend. Simpler operations than Thanos for large-scale deployments.

---

## Grafana

### Dashboards

```json
// Dashboard JSON model (simplified)
{
  "title": "API Overview",
  "tags": ["api", "production"],
  "time": { "from": "now-6h", "to": "now" },
  "refresh": "30s",
  "panels": [
    {
      "type": "timeseries",
      "title": "Request Rate",
      "targets": [
        {
          "expr": "sum by (endpoint) (rate(http_requests_total[5m]))",
          "legendFormat": "{{ endpoint }}"
        }
      ]
    }
  ]
}
```

### Template Variables

```
# Query variable from label values
label_values(http_requests_total, service)

# Query variable with regex filter
label_values(http_requests_total{environment="$environment"}, service)

# Interval variable for rate windows
$__rate_interval    # auto-calculated safe interval for rate()

# Usage in panel query
sum by (service) (rate(http_requests_total{service=~"$service"}[$__rate_interval]))
```

Variable types: Query (from data source), Custom (static list), Constant, Interval, Text box, Ad hoc filters.

### Provisioning (Infrastructure as Code)

```yaml
# /etc/grafana/provisioning/datasources/prometheus.yml
apiVersion: 1
datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    jsonData:
      timeInterval: 15s

  - name: Loki
    type: loki
    access: proxy
    url: http://loki:3100
```

```yaml
# /etc/grafana/provisioning/dashboards/dashboards.yml
apiVersion: 1
providers:
  - name: Default
    folder: ""
    type: file
    disableDeletion: false
    updateIntervalSeconds: 30
    options:
      path: /var/lib/grafana/dashboards
      foldersFromFilesStructure: true
```

### Grafana Alerting

```yaml
# Alert rule (Grafana 9+ unified alerting)
# Condition: error rate > 5% for 5 minutes
# Data query A: sum(rate(http_requests_total{status=~"5.."}[5m])) / sum(rate(http_requests_total[5m]))
# Condition: A > 0.05
# Evaluation: every 1m, for 5m
# Notification: contact point "slack-critical"
```

Grafana alerting supports multi-data-source evaluation, correlated alerts, silences, and contact points (Slack, PagerDuty, email, webhook, Opsgenie, etc.).

## Loki (Logs)

Log aggregation system designed to work with Prometheus labels. Does NOT index log content — indexes labels only.

### LogQL

```logql
# Stream selector (like Prometheus label selectors)
{job="api", namespace="production"}

# Filter by content
{job="api"} |= "error"               # contains
{job="api"} != "healthcheck"          # doesn't contain
{job="api"} |~ "status=[45].."        # regex match
{job="api"} !~ "status=2.."           # negative regex

# Parsers
{job="api"} | json                    # parse JSON logs
{job="api"} | json | status >= 500    # filter parsed fields
{job="api"} | logfmt                  # parse logfmt (key=value)
{job="api"} | regexp `(?P<ip>\S+) .* "(?P<method>\S+) (?P<path>\S+)"`
{job="api"} | pattern `<ip> - - [<_>] "<method> <path> <_>" <status>`

# Metric queries (generate metrics from logs)
rate({job="api"} |= "error" [5m])                    # errors per second
sum by (status) (count_over_time({job="api"} | json [1h]))
bytes_rate({job="api"}[5m])                          # log throughput

# Unwrap (extract numeric value for aggregation)
avg_over_time({job="api"} | json | unwrap duration [5m])

# Top 10 IPs by request count
topk(10, sum by (ip) (count_over_time({job="nginx"} | pattern `<ip> - -` [1h])))
```

### Loki Deployment

```yaml
# Loki config (simplified)
schema_config:
  configs:
    - from: 2024-01-01
      store: tsdb
      object_store: s3
      schema: v13
      index:
        prefix: loki_index_
        period: 24h

storage_config:
  tsdb_shipper:
    active_index_directory: /loki/index
    cache_location: /loki/cache
  aws:
    s3: s3://region/bucket-name
```

**Promtail** (agent): tails log files, adds labels, pushes to Loki. Alternative agents: Grafana Alloy, Fluentd/Fluent Bit with Loki output plugin, Vector.

## Tempo (Traces)

Distributed tracing backend. Stores traces in object storage. Query by trace ID or via TraceQL.

```
# TraceQL
{ resource.service.name = "api" && span.http.status_code >= 500 }
{ span.duration > 2s && resource.service.name = "checkout" }
{ name = "HTTP POST" && span.http.url =~ "/api/order.*" }
```

**Trace-to-logs/metrics correlation**: Grafana links traces ↔ logs ↔ metrics via shared labels (service, pod, trace ID). Exemplars in Prometheus link metrics to specific traces.

## Exporter Ecosystem

| Exporter                | Metrics                                |
| ----------------------- | -------------------------------------- |
| **node_exporter**       | Linux host: CPU, memory, disk, network |
| **blackbox_exporter**   | Probe endpoints: HTTP, TCP, ICMP, DNS  |
| **mysqld_exporter**     | MySQL server metrics                   |
| **postgres_exporter**   | PostgreSQL metrics                     |
| **redis_exporter**      | Redis server metrics                   |
| **cadvisor**            | Container resource usage               |
| **kube-state-metrics**  | Kubernetes object state                |
| **cloudwatch_exporter** | AWS CloudWatch metrics                 |
