# Logging & Observability

## The Three Pillars of Observability

### 1. Logs — What happened

Discrete events with context. The most familiar signal.

**Structured logging** (JSON, key-value) is strongly preferred for production systems:

```json
{
  "timestamp": "2024-03-15T10:30:00Z",
  "level": "error",
  "service": "payment-api",
  "trace_id": "abc123",
  "user_id": 42,
  "message": "Payment declined",
  "error_code": "INSUFFICIENT_FUNDS",
  "amount": 99.5
}
```

**NOT this:**

```
ERROR: Payment failed for user 42
```

**Log levels:**

- **TRACE/DEBUG**: Typically development-only. Rarely enabled in production due to volume.
- **INFO**: Normal operations. Service started, request completed, job finished.
- **WARN**: Unexpected but handled. Retry succeeded, deprecated API used, slow query.
- **ERROR**: Something failed. Request failed, unhandled exception, external service down.
- **FATAL/CRITICAL**: System is unusable. Out of memory, data corruption, can't connect to required services.

### 2. Metrics — How is the system performing

Numeric measurements aggregated over time. Cheap to collect, easy to alert on.

**The four golden signals (Google SRE):**

- **Latency**: How long requests take (p50, p95, p99).
- **Traffic**: Requests per second.
- **Errors**: Error rate (5xx responses, failed operations).
- **Saturation**: How full the system is (CPU, memory, disk, connection pool).

**RED method (for request-driven services):** Rate, Errors, Duration.
**USE method (for resources):** Utilization, Saturation, Errors.

**Metric types:**

- **Counter**: Monotonically increasing (total requests, total errors).
- **Gauge**: Current value that goes up and down (memory usage, queue depth).
- **Histogram**: Distribution of values (request latency buckets).

### 3. Traces — The request journey

Follow a single request across services and components.

- **Trace**: The entire journey of a request.
- **Span**: A single operation within a trace (HTTP call, DB query, function execution).
- **Trace ID**: Unique identifier propagated across all services.
- **Span ID**: Identifies each operation within the trace.

**Distributed tracing is non-optional for microservices.** Without it, debugging cross-service issues is nearly impossible.

## OpenTelemetry (OTel)

The industry standard for observability instrumentation. Vendor-neutral, supported by every major observability platform.

**Components:**

- **API**: Interfaces for creating traces, metrics, logs.
- **SDK**: Reference implementation of the API.
- **Collector**: Receives, processes, and exports telemetry data.
- **Exporters**: Send data to backends (Jaeger, Prometheus, Datadog, Grafana, etc.).

**Auto-instrumentation** available for most frameworks — add a library and get HTTP, DB, and framework traces automatically.

## Logging Patterns

1. **Use structured logging from day one.** Switching later is painful.
2. **Include correlation IDs.** Trace ID in every log line. Link logs to traces.
3. **Standardize field names.** `user_id` everywhere, not `userId` somewhere and `user` elsewhere.
4. **Log at appropriate levels.** If everything is ERROR, nothing is.
5. **Don't log sensitive data.** Redact passwords, tokens, PII, credit card numbers.
6. **Include context.** Log the relevant identifiers (request ID, user ID, order ID). "Error occurred" is useless.
7. **Make logs searchable.** Use structured fields, not embedded-in-message values.
8. **Set up log rotation and retention.** Logs grow fast. Archive or delete based on policy.
9. **Use sampling for high-volume services.** Log 10% of successful requests, 100% of errors.

## Alerting Patterns

- **Alert on symptoms, not causes.** Alert on "error rate > 5%" not "database CPU > 80%."
- **Every alert must be actionable.** If nobody needs to do anything, it's not an alert — it's a metric.
- **Set appropriate thresholds.** Too sensitive = alert fatigue. Too lenient = missed incidents.
- **Use severity levels.** Page for P1 (customer impact now). Ticket for P2 (degraded but functional). Log for P3 (investigate later).
- **Include runbook links in alerts.** When the alert fires at 3am, the on-call engineer needs steps, not just a metric name.

## Observability Stack (Common Choices)

| Component       | Options                                                             |
| --------------- | ------------------------------------------------------------------- |
| Metrics         | Prometheus, Datadog, CloudWatch, InfluxDB                           |
| Logs            | ELK (Elasticsearch/Logstash/Kibana), Loki, Datadog, CloudWatch Logs |
| Traces          | Jaeger, Tempo (Grafana), Zipkin, Datadog APM                        |
| Dashboards      | Grafana, Datadog, Kibana                                            |
| Alerting        | PagerDuty, Opsgenie, Grafana Alerting                               |
| Instrumentation | OpenTelemetry (universal)                                           |

---

_Sources: Google SRE Book (Monitoring Distributed Systems), OpenTelemetry documentation, Honeycomb (Observability Engineering), Brendan Gregg (Systems Performance)_
