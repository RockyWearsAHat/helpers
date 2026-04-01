# Tools: Observability Tooling

## The Three Pillars: Logs, Metrics, Traces

Observability requires three signal types working together.

- **Logs**: Discrete events with context (request started, database query took 50ms, error occurred)
- **Metrics**: Numerical time-series (request count, error rate %, P99 latency, CPU usage)
- **Traces**: Request journey (entry point → service A → service B → database → response)

**Correlation**: Bind logs/traces/metrics by request ID. Example:
```json
{
  "request_id": "abc-123",
  "service": "checkout",
  "timestamp": "2026-03-25T14:00:00Z",
  "level": "info",
  "message": "Payment processed",
  "duration_ms": 245,
  "payment_provider": "stripe"
}
```

Request ID flows through all log entries, trace spans, and metric labels.

## Open Source Stack

### Prometheus (Metrics)

**Architecture**: Time-series database + scraper + query language (PromQL).

```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'app'
    static_configs:
      - targets: ['localhost:9090']
  - job_name: 'postgres'
    static_configs:
      - targets: ['localhost:9187']  # postgres_exporter
```

Application exposes `/metrics` endpoint:
```
http_requests_total{path="/checkout", method="POST"} 1523
http_request_duration_seconds_bucket{path="/checkout", le="0.5"} 1000
http_request_duration_seconds_bucket{path="/checkout", le="1.0"} 1450
```

**Query language** (PromQL):
```promql
# Request rate per second
rate(http_requests_total[5m])

# P95 latency
histogram_quantile(0.95, http_request_duration_seconds_bucket)

# Error rate
rate(http_requests_total{status="5xx"}[5m]) / rate(http_requests_total[5m])
```

**Tradeoffs**:
- **Pros**: Lightweight, simple queries, strong community, integrates with Kubernetes
- **Cons**: Limited long-term retention (default 2 weeks); no correlation across signal types; cardinality management difficult

**Cardinality problem**: If you expose `http_requests_total{user_id="X", request_id="Y", path="/Z"}`, each unique combination is a new series. 1000 users × 100 requests × 50 paths = 5M series (expensive).

**Best for**: Infrastructure metrics, simple use cases, teams that value simplicity.

### Grafana (Visualization)

Dashboards for time-series data. Datasources: Prometheus, InfluxDB, Elasticsearch, Datadog, Splunk, etc.

```json
{
  "dashboard": {
    "title": "App Health",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [
          {
            "expr": "rate(http_requests_total[5m])"
          }
        ]
      },
      {
        "title": "Error Rate",
        "targets": [
          {
            "expr": "rate(http_requests_total{status=~\"5..\"}[5m])"
          }
        ]
      }
    ]
  }
}
```

**Alerting**: Define alert rules in Prometheus/Grafana; webhook to Slack, PagerDuty, etc.

```yaml
groups:
  - name: app-alerts
    rules:
      - alert: HighErrorRate
        expr: rate(http_requests_total{status="5xx"}[5m]) > 0.05
        for: 5m
        annotations:
          summary: "Error rate > 5%"
```

**Tradeoffs**:
- **Pros**: Flexible, works with many datasources, strong visualization, free
- **Cons**: Requires separate datasource; Grafana itself is stateless (configs in code/database)

### Loki (Logs)

Log aggregation without full-text indexing (reduces cost vs. Elasticsearch).

**Architecture**: Write logs in JSON with labels; Loki indexes only labels.

```json
{
  "level": "error",
  "service": "checkout",
  "request_id": "abc-123",
  "message": "Payment gateway timeout"
}
```

**Query**:
```
{service="checkout"} | json | level="error"
```

Finds logs where service=checkout, then filters by level=error in JSON payload.

**Tradeoffs**:
- **Pros**: Cheap (log volume doesn't impact query speed much), integrates with Prometheus/Grafana, high cardinality labels safe
- **Cons**: No full-text search; label design is critical; JSON parsing on query (slower than pre-indexed)

**Best for**: High-volume logging, cost-sensitive workloads, teams using Prometheus already.

### Jaeger (Traces)

Distributed tracing. Traces are broken into spans; spans have parent-child relationships.

```
Request starts at API gateway
├─ Span: gateway → checkout (10ms)
├─ Span: checkout service (500ms)
│  ├─ Span: validate_payment (50ms)
│  ├─ Span: call_stripe (400ms)
│  └─ Span: update_database (50ms)
└─ Span: return_response (5ms)
```

**Instrumentation** (OpenTelemetry):
```python
from opentelemetry import trace

tracer = trace.get_tracer(__name__)
with tracer.start_as_current_span("process_payment") as span:
    span.set_attribute("user_id", user_id)
    span.set_attribute("amount", amount)
    result = stripe_client.charge()
    span.set_attribute("stripe_charge_id", result.id)
```

Jaeger backend collects spans and stores them; UI queries by trace ID, service, or latency.

**Tradeoffs**:
- **Pros**: Understand request flow; debug cross-service latency; identify bottlenecks
- **Cons**: Sampling/cardinality explosion risk; requires instrumentation in every service; storage can be large

### Tempo (Traces with cost optimization)

Modern alternative to Jaeger. Doesn't retain all traces; uses probabilistic sampling + strong indexing.

**Sampling strategy**:
- Sample 1% of traces (trace every 100th request)
- Adaptive sampling: 100% of slow traces (> 1s), lower % of fast traces
- Combine with metric-based alerts: Don't need to trace every request

**Storage**: Batches traces; compresses well.

**Tradeoffs**:
- **Pros**: Better scalability than Jaeger; integrates with Grafana/Prometheus
- **Cons**: Traces are non-exhaustive (sampling); requires understanding sample rates

**Best for**: Teams running Kubernetes with Prometheus/Grafana already; cost-conscious.

## Managed Observability SaaS

### Datadog

**Components**:
- **Agent**: Runs on host/container; collects metrics, logs, traces, profiles
- **APM** (Application Performance Monitoring): Auto-instrument services; collect traces without code changes
- **Log Management**: Full-text searchable logs; integrates with metrics/traces via tags
- **Infrastructure Monitoring**: Host CPU, disk, network, container metrics
- **RUM** (Real User Monitoring): Browser-side telemetry (page load time, JS errors, user interactions)
- **Synthetic Monitoring**: Scheduled tests (API health checks, browser tests)

**Pricing**: Per-host + per-GB-ingested (metrics, logs); expensive at scale.

**Strengths**:
- Tight integration across signals (click error in log → see correlated metrics → drill to trace)
- AI-powered anomaly detection
- Out-of-box dashboards for popular services (PostgreSQL, Redis, MySQL)
- Strong APM (auto-instrumentation, profiling)
- Browser RUM is most mature in market

**Limitations**:
- Expensive (Prometheus free alternative can be 10x cheaper)
- Vendor lock-in (Datadog ecosystem proprietary)
- Sampling built-in; may miss rare issues

**Best for**: Enterprises needing all-in-one platform; teams with budget; high-volume applications.

### New Relic

**Components**:
- **APM**: End-to-end transaction tracing (similar to Datadog)
- **Browser (RUM)**: User experience monitoring; strong replay feature
- **Infrastructure**: Host metrics, container monitoring
- **Logs**: Full-text searchable; NRQL query language
- **AI**: Incident detection, anomaly detection

**Pricing**: Per GB ingested; cheaper than Datadog at extreme scale.

**Strengths**:
- Strong browser monitoring (user replay, session recording)
- NRQL is powerful query language
- Less expensive than Datadog at high volume
- Good documentation

**Limitations**:
- Not as deep/automated as Datadog APM
- Weaker container/Kubernetes support vs Datadog
- Sampling means rare errors missed

**Best for**: Companies with significant RUM needs; high-volume cost optimization.

### Splunk

**Components**:
- **Core**: Full-text searchable logs; SPL (Splunk Processing Language) for queries
- **SPL**: Complex queries (aggregation, correlation across fields)
- **SOAR** (Security Orchestration): Automated incident response; integrates with paging systems
- **ITSI** (IT Service Intelligence): Application dependency mapping, anomaly detection
- **Observability Cloud**: APM + metrics + traces (separate product, competing with Datadog)

**Pricing**: Per-GB-indexed + licensing model; enterprise sales-driven.

**Strengths**:
- SPL is Turing-complete; can express complex data transformations
- Strong for security/compliance (retention policies, audit trails)
- SOAR automation reduces mean-time-to-remediation (MTTR)
- Long history; mature platform

**Limitations**:
- Expensive (often 2-3x Datadog)
- Steep learning curve (SPL is complex)
- Overkill if you don't need SOAR/compliance features
- Slow UI for large result sets

**Best for**: Enterprises with compliance requirements; security teams; complex data analysis needs.

## Managed vs Self-Hosted Tradeoffs

| Aspect | Managed (Datadog, New Relic) | Self-Hosted (Prometheus + Grafana + Loki + Tempo) |
|--------|------------------------------|--------------------------------------------------|
| **Cost** | Predictable but high | Low for compute; monitoring overhead |
| **Scalability** | Unlimited growth | Limited by infrastructure; need planning |
| **Retention** | Months-years (configurable) | Weeks-months (storage dependent) |
| **Setup** | Hours (install agent) | Days-weeks (infrastructure + tuning) |
| **Maintenance** | Vendor maintains | Team responsibility; updates, upgrades |
| **Integration** | Tight (proprietary APIs) | Loosely coupled (Prometheus standard) |
| **Customization** | Moderate (limited to product features) | Unlimited (source code available) |
| **Support** | Enterprise SLA | Community-driven or purchased support |

**Decision factors**:
- **Small team, limited ops**: Managed is simpler
- **Cost-sensitive, mature ops**: Self-hosted
- **Compliance/data residency**: Self-hosted (keep data on-premise)
- **High volume (> 1TB/day logs)**: Self-hosted (cost explodes with managed)

## Observability Best Practices

### Cardinality Management

High-cardinality labels explode storage/query costs.

**Bad**: `requests_total{user_id, request_id, session_id, path}`

User ID alone: 1M users × 1B requests = 1T unique series.

**Good**: `requests_total{service, endpoint, method, status}` + correlation via trace ID

Trace ID in logs; queries correlate via logs, not metrics.

### Sampling Strategy

Sample high-volume traces to keep costs reasonable.

- **Head sampling** (decide at origin): Sample 1% of all traces. Risk: Rare errors fall through.
- **Tail sampling** (decide after collection): Collect all traces; only save if duration > 1s or error occurred. Requires temporary storage.
- **Adaptive sampling**: Sample % varies by service (critical paths 100%, background jobs 1%)

### SLOs & Golden Signals

Define what "good" means; alert when breached.

**Golden Signals**:
- **Latency**: P99 < 500ms
- **Traffic**: Requests/sec maintained
- **Errors**: < 0.1% error rate
- **Saturation**: CPU < 80%, memory < 85%

Alert on SLI breach (Latency P99 > 500ms), not on metrics (CPU > 90%). SLIs matter to users; metrics are proxies.

### Structured Logging

Log JSON, not prose. Enables correlation + queries.

```json
{"request_id": "abc123", "service": "checkout", "user_id": "user456", "status": "completed", "duration_ms": 245}
```

vs.

```
2026-03-25 14:00:00 Checkout request from user 456 took 245ms
```

JSON version: Query `{service="checkout"} | duration_ms > 1000` (queries now include structured data).

## See Also

devops-prometheus-grafana, logging-observability, observability-metrics, sre-observability-strategy, devops-observability-patterns