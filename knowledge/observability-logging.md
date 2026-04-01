# Observability: Logging

Logs capture _what happened_: discrete events with context. Unlike metrics (aggregated state) and traces (request flow), logs preserve specific details needed for debugging.

## Structured Logging: The Foundation

**Structured logging** emits logs as machine-parseable records (JSON, key-value), not unstructured text.

**Unstructured:**
```
2025-03-15 10:30:00 ERROR Payment failed for user 42 with amount 99.5
```

**Structured (JSON):**
```json
{
  "timestamp": "2025-03-15T10:30:00Z",
  "level": "ERROR",
  "message": "Payment failed",
  "user_id": 42,
  "amount": 99.5,
  "currency": "USD",
  "error_code": "INSUFFICIENT_FUNDS",
  "service": "payment-api",
  "trace_id": "abc123...",
  "span_id": "def456..."
}
```

**Why structured matters:**
- **Searchable.** Query logs by `user_id=42` instead of grepping text.
- **Machine-readable.** Parsers extract values consistently.
- **Correlation.** Include `trace_id` to link with traces and `span_id` to link with specific operations.
- **Retention policies.** Drop sensitive fields based on data classification.

## Log Levels

**Purpose:** Categorize event severity and control volume. In production, typically INFO and above are kept; DEBUG is development-only.

### TRACE

Lowest level; rarely used in production due to volume.

```
trace "Creating HTTP client pool with 10 workers"
trace "Acquired database connection from pool"
```

**When to use:** Development debugging of detailed execution flow. Cost: noisy logs; only enable on demand.

### DEBUG

Development level; disabled by default in production.

```
debug "Request headers: Content-Type=application/json, Authorization=Bearer ***"
debug "Parsed user roles: [admin, editor]"
debug "Query took 45ms"
```

**When to use:** Detailed operational diagnostics. Enable in pre-production or when troubleshooting specific components. Cost: can significantly increase disk I/O and log volume.

### INFO

Normal operational level; always on in production.

```
info "Service started on port 8080"
info "Request GET /api/users completed in 120ms"
info "Batch job processed 5000 records"
info "Database migration from v1 to v2 completed"
```

**When to use:** Key operational events. Service lifecycle, normal transitions, business-critical operations. Acceptable to log for every request in moderate-traffic systems; high-traffic systems should sample.

### WARN

Unexpected but handled; does not prevent operation completion.

```
warn "Retry attempt 2 of 3 for payment API call"
warn "Deprecated API /v1/users used; migrate to /v2/users"
warn "Cache hit rate below 50%; consider cache tuning"
warn "Response time 2.5s exceeds SLO threshold of 1s"
```

**When to use:** Degraded conditions that resolve automatically, policy violations, unexpected API behavior. Actionable but not urgent. Cost: balance with noise; if WARN logs constantly fire, lower threshold or sample.

### ERROR

Operation failed or cannot complete; action required.

```
error "Payment declined: INSUFFICIENT_FUNDS"
error "Database connection failed; retrying..."
error "Request validation failed: missing required field 'amount'"
error "Third-party API returned 500"
```

**When to use:** Errors that prevent operation completion. Exceptions, API failures, validation errors. All ERROR logs should be actionable. Cost: in high-traffic systems, a single error rate spike can create log explosion.

### FATAL / CRITICAL

System-wide failure; application cannot continue.

```
fatal "Out of memory; cannot allocate 2GB"
fatal "Data corruption detected in database; refusing to start"
fatal "Critical dependency unavailable; shutting down"
```

**When to use:** Rare. Errors that leave the system in an unusable state. Typically followed by service shutdown or escalation. Cost: low volume; always alertable.

### Log Level Strategy

**Development:** TRACE or DEBUG enabled on components under development; INFO for system components.

**Pre-production staging:** INFO + sampling of DEBUG for infrequent issues; WARN and up always on.

**Production:**
- INFO + 10% sampling of successful requests
- WARN, ERROR, FATAL always on (no sampling)
- Typical log volume: 1M to 10M lines/day for moderate services

## Correlation IDs and Distributed Context

**Trace ID** is the primary correlation identifier. Propagated through all services in a request.

```
Frontend HTTP Request GET /checkout
  trace_id: abc123
  span_id: xyz789

┌─────────────────────────────────────┐
│  Auth Service                       │
│  Log: auth.attempt                  │
│  trace_id: abc123                   │
│  span_id: s1                        │
│  status: success                    │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│  Product Service                    │
│  Log: product.lookup                │
│  trace_id: abc123                   │
│  span_id: s2                        │
│  sku_id: 12345                      │
└─────────────────────────────────────┘

│  Payment Service                    │
│  Log: payment.charge                │
│  trace_id: abc123                   │
│  span_id: s3                        │
│  status: declined                   │
│  reason: INSUFFICIENT_FUNDS         │
└─────────────────────────────────────┘
```

**At query time:**
```
logs query: trace_id=abc123
# Returns all logs from all services in the request
# Sorted by timestamp; reveals exact sequence of events
```

**Implementation:**
- **HTTP headers:** Propagate via `traceparent` or `X-B3-TraceId`
- **Middleware:** Extract trace ID from request; inject into logger context
- **Async tasks:** Propagate trace ID via message attributes or thread-local storage
- **Logging library:** Include `trace_id` in every log record

```python
from opentelemetry import trace

def log_with_trace(message):
    span = trace.get_current_span()
    trace_id = format(span.get_span_context().trace_id, '032x')
    logger.info(message, extra={"trace_id": trace_id})
```

## ELK Stack: Elasticsearch, Logstash, Kibana

**ELK** is the classic centralized logging architecture.

### Elasticsearch

Full-text search engine; stores indexed JSON documents.

| Component | Purpose |
|-----------|---------|
| **Indices** | Analogue to database tables; one per day (e.g., `logs-2025.03.15`) |
| **Mappings** | Schema; defines field types (`keyword`, `text`, `integer`, `date`) |
| **Shards** | Horizontal partitions; enable parallel search |
| **Replicas** | Redundancy; survives node failure |

**Cost trade-offs:**
- More replicas = higher availability + higher storage/cost
- Larger shards = fewer shards, faster merge-time, higher memory
- More indices = finer management but more overhead

**Query language:** Lucene query syntax + JSON DSL. Rich but steep learning curve.

```
Query: user_id:42 AND level:ERROR AND timestamp:[now-1h TO now]
# Returns all ERROR logs for user 42 in the last hour
```

### Logstash

ETL (extract, transform, load) pipeline. Ingests logs, parses, enriches, exports.

```
Input (file, syslog, HTTP) 
  ↓ Parse (grok regex, JSON)
  ↓ Filter (add fields, drop PII, route)
  ↓ Output (Elasticsearch, S3, Kafka)
```

**Example:** Parse Apache access logs into structured records:

```ruby
input {
  file { path => "/var/log/apache2/access.log" }
}

filter {
  grok {
    match => { "message" => "%{IP:client_ip} %{WORD:method} %{URIPATH:path} HTTP/%{NUMBER:http_version} %{NUMBER:status_code}" }
  }
  
  if [status_code] >= 400 {
    mutate { add_field => { "error" => true } }
  }
  
  # Drop logs containing secrets
  mutate { gsub => [ "message", "password=[^&]*", "password=***" ] }
}

output {
  elasticsearch { hosts => ["esdb.example.com:9200"] index => "logs-%{+YYYY.MM.dd}" }
}
```

**Drawback:** High CPU usage; stateful processing; often a bottleneck.

### Kibana

Web UI for Elasticsearch. Visualize, search, dashboards, alerts.

```
Kibana 7+:
  ├─ Discover tab: full-text search + field explorer
  ├─ Visualize tab: charts, heatmaps, geo maps
  ├─ Dashboard tab: multi-panel composition
  ├─ Alerts tab: query-based alerting
  └─ Canvas tab: custom visualizations
```

**Typical workflow:**
1. Search logs: `trace_id=abc123`
2. Click on a log record to expand
3. View all fields and their values
4. Save query as dashboard panel
5. Correlate with other dashboards

### ELK Trade-offs

**Advantages:**
- Rich query language and UI
- Stateful; supports complex transformations
- Widely adopted; large community and tooling

**Disadvantages:**
- High operational overhead (deployment, tuning, scaling)
- Elasticsearch stores everything; no built-in retention
- Expensive at scale (CPU, memory, disk for Logstash + Elasticsearch)
- Query performance degrades with large result sets

## PLG Stack: Promtail, Loki, Grafana

**PLG** is a cloud-native alternative to ELK: lower CPU overhead, kubernetes-native, simpler operations.

### Promtail

Log shipper. Lightweight; reads local logs and forwards to Loki.

```yaml
# /etc/promtail/config.yml
scrape_configs:
  - job_name: kernel
    static_configs:
      - targets:
          - localhost
        labels:
          job: kernel
          __path__: /var/log/kern.log

  - job_name: docker
    docker_sd_configs:
      - host: unix:///var/run/docker.sock
    relabel_configs:
      - source_labels: [__meta_docker_container_name]
        target_label: container
```

**Philosophy:** Minimal; just ship and label. No parsing (parsing happens at query time in Loki).

### Loki

Log aggregation engine. Inspired by Prometheus; indexes labels only (not full text).

**Architecture:**
```
Promtail ──→ Loki Distributor ──→ Ingester ──→ Cassandra / BoltDB
                                ↓
                        Querier (federation)
```

**Label-based indexing:**
- Indexes: `{job="api",hostname="node1"}`
- Full log line stored separately (not indexed)
- Query: `{job="api"} | json | status=500` (label match + regex + JSON filtering)

**Query language:** LogQL (Prometheus-inspired label selection + regex).

```
# Find all ERROR logs from auth service
{job="auth"} | json | level="ERROR" | duration > 1s

# Count errors per service per minute
count_over_time({level="ERROR"}[1m]) by (job)
```

**Advantages:**
- Low memory overhead (labels-only index)
- Scales better than ELK (cheaper per-GB storage)
- Kubernetes-native (auto-label from pod annotations)
- Simple operations

**Disadvantage:**
- No full-text search; query by labels + regex only
- Less flexible parsing; schema-less but hard to refactor

### Grafana

Same UI as kibana, but integrated with Prometheus + Loki data sources.

```
Grafana detects:
  ├─ Prometheus data source: metrics queries
  ├─ Loki data source: log queries
  └─ Alertmanager data source: alert rules
```

Single dashboard can mix metrics + logs + traces. Jump between them seamlessly.

## Log Aggregation Patterns

### Central Logging Hub

All services ship logs to a central system (Elasticsearch, Loki, Datadog, Splunk).

```
Service A ──┐
Service B ──┼─→ Log Aggregator ──→ Backend Storage
Service C ──┤
            └─→ Local retention (1 day)
```

**Benefit:** Single query interface; historical records; compliance. Cost: network overhead, storage.

### Sampling

Not every log line is valuable. Reduce volume via sampling.

```python
# Log all errors and WARNs; sample INFOs at 10%
if log_level in [ERROR, WARN]:
    send_to_aggregator(log_record)
elif random.random() < 0.1:
    send_to_aggregator(log_record)
```

**Tradeoff:** Reduces storage and processing; loses visibility into low-probability events within the sample. Use tail-based sampling (keep all errors, sample successes) to maintain error visibility.

### Sequential ID Tracking

For transactional workflows, track progress through logs.

```
Request 1: step_id=1 step_name=validate status=success
Request 1: step_id=2 step_name=fetch_inventory status=success
Request 1: step_id=3 step_name=reserve_stock status=failed error=STOCK_DEPLETED
Request 1: step_id=4 step_name=charge_payment status=skipped reason=previous_failure
```

Query: `request_id=123 | sort by step_id` reveals exact workflow progression.

## Retention Policies

Logs are expensive to store long-term. Policies balance compliance and cost.

### Common Strategies

| Data Class | Retention | Destination | Reason |
|------------|-----------|-------------|--------|
| Production errors | 90 days | Hot storage | Incident diagnosis |
| Production info/warn | 30 days | Warm storage | Operational troubleshooting |
| Development | 3 days | Local | Cost control |
| Compliance (finance, health) | 7 years | Cold storage (archive) | Regulatory |

### Implementation

```yaml
Elasticsearch index lifecycle policy:
  hot:    1 day (all writes go here)
  warm:   7 days (moved to cheaper hardware)
  cold:   30 days (archived to S3)
  delete: 90 days (removed)

# Loki retention
retention_period: 30d

# Datadog
- Production logs: 30-day retention
- Compliance logs: 1-year retention (separate index)
```

## PII Masking and Data Protection

**PII** (Personally Identifiable Information) must be redacted before storage: credit card numbers, SSNs, passwords, email addresses, user IDs (context-dependent).

### At Application Level (Preferred)

Mask PII before logging:

```python
def mask_credit_card(cc):
    return f"****{cc[-4:]}"

def mask_email(email):
    local, domain = email.split("@")
    return f"{local[0]}***@{domain}"

logger.info("Payment processed", extra={
    "card": mask_credit_card("4532123456789010"),
    "email": mask_email("alice@example.com"),
    "amount": 99.5
})
```

**Benefit:** Simple; prevents secrets before they reach log system. Downside: requires discipline across all log sites.

### At Log Aggregator Level

Use regex to redact patterns in Logstash or Loki:

```ruby
# Logstash
mutate {
  gsub => [
    "message", "credit_card=[0-9]{16}", "credit_card=****",
    "message", "password=[^&]*", "password=***",
    "message", "ssn=\d{3}-\d{2}-\d{4}", "ssn=***-**-****"
  ]
}
```

```
# Loki filter
{job="api"} | logfmt | auth_token != "" | replace(auth_token, "Bearer .*", "Bearer ***")
```

**Downside:** Centralized processing; secret may leak before regex fires. Use with application-level masking.

### Compliance Frameworks

- **GDPR:** Right to erasure; logs containing user data must be deleted on request (hard problem for distributed systems)
- **HIPAA:** PHI (Protected Health Information) must be encrypted; access logged
- **PCI DSS:** Never log full credit card numbers; masked exceptions allowed
- **SOC 2:** Audit trail of who accessed logs; retention policies audited

**Recommendation:** Separate logs by sensitivity level:
- `logs-prod-app`: application logs (non-sensitive) → 30-day retention
- `logs-prod-compliance`: financial/health data → 7-year retention (encrypted)
- `logs-prod-debug`: development logs → 3-day retention

## See Also

- observability-distributed-tracing (trace ID correlation)
- devops-observability-patterns (three pillars)
- security-incident-response (forensic log analysis)