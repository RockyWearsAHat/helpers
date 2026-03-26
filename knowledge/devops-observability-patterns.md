# Observability Patterns — Structured Logging, Tracing, Metrics, and SLOs

Observability is the ability to understand system behavior from external observations without adding instrumentation code. Modern high-cardinality distributed systems require intentional observability patterns beyond the basic three pillars (logs, metrics, traces). This note covers practical patterns: structured logging, correlation IDs, distributed tracing standards, metrics frameworks, SLI/SLO/SLA definitions, and observability-driven development.

## Structured Logging: JSON Over Free-Text

Traditional logging outputs unstructured text:

```
[2026-03-25 14:23:45] User login failed: user=alice ip=192.0.2.1 reason=invalid_password attempt=3
```

Parsing this requires regex; querying is fragile. Structured logging emits JSON:

```json
{
  "timestamp": "2026-03-25T14:23:45Z",
  "level": "WARN",
  "event": "user_login_failed",
  "user": "alice",
  "ip": "192.0.2.1",
  "reason": "invalid_password",
  "attempt": 3,
  "trace_id": "abc123..."
}
```

### Benefits of JSON Logging

- **Queryable:** Search by field (`reason=invalid_password`)
- **Aggregatable:** Group by field values, compute statistics
- **Pipeline-friendly:** Log aggregators (Datadog, Splunk, ELK) parse directly
- **Contextual:** Rich metadata at log time, no need to parse strings
- **Performance:** Structured fields faster than parsing free-text

### Implementation Pattern

Most languages have structured logging libraries:
- **Go:** `zap`, `logrus`
- **Python:** `structlog`, `pythonjsonlogger`
- **Java:** Jackson, SLF4J with JSON encoder
- **Node.js:** `pino`, `bunyan`

Standard fields across all logs:
- `timestamp`: ISO 8601 UTC
- `level`: DEBUG, INFO, WARN, ERROR
- `trace_id`: Unique request identifier (see Correlation IDs)
- `span_id`: Position in distributed trace (see Distributed Tracing)
- `service`: Which service emitted the log
- `version`: Application version for correlation with deployments

Application-specific fields capture domain context:
- User ID, request ID, feature flag state, feature toggle, business outcome

### Avoiding Log Chaos

**Anti-pattern:** Logging every variable in structured format creates unintelligible noise. Log when:
- Decisions made (feature disabled, fallback triggered)
- Errors occur
- Service boundaries crossed
- Anomalies detected

Not every conditional, loop, or function call.

## Correlation IDs: Linking Requests Across Services

In distributed systems, a single user request fans out across many services. Correlation IDs (also called request IDs or trace IDs) link all logs, metrics, and traces for that request.

### Pattern

1. **Ingress point generates ID:** Outermost service (API gateway, load balancer) generates a unique ID (UUID4 or timestamp+random)
2. **Propagate in request headers:** Include correlation ID in all downstream requests (HTTP header, gRPC metadata, message queue header)
3. **Log includes ID:** Every log in every service includes the correlation ID
4. **Search by ID:** Operator queries logs for `trace_id=abc123` and sees the entire request flow

### W3C Trace Context Standard

W3C Trace Context defines standard header propagation for distributed traces:

**HTTP headers:**
- `traceparent: 00-trace_id-span_id-sampled` (W3C standard)
- `tracestate: vendor-specific-state` (vendor extensions)

Older standards: `X-Trace-ID`, `X-Request-ID` (non-standard, still common).

**Benefits of W3C standard:**
- Vendor-agnostic
- Spans multiple protocol boundaries (HTTP, gRPC, messaging)
- Supports sampling decisions
- Becomes language/framework-agnostic

### Implementation

Most observability frameworks auto-propagate W3C headers:
- OpenTelemetry SDKs inject on outbound calls, extract on inbound
- Service mesh (Istio, Linkerd) propagates headers transparently

Teams need:
1. SDK setup in each service (OpenTelemetry SDK)
2. Shared context propagation (middleware/interceptors)
3. Log integration (include trace ID in every structured log)

**Common mistake:** Generating new trace ID at each service boundary instead of propagating. Result: impossible to correlate across services.

## Distributed Tracing: Request Paths Through Services

Distributed tracing records the path a request takes through services, with timing for each hop.

### Trace Structure

**Trace** = complete journey of a request from ingress to response

**Span** = one node in the journey (one service, one function)

Example:
```
Trace: request-123
├─ Span: API.GetUser (10ms)
│  ├─ Span: AuthService.Validate (2ms)
│  ├─ Span: UserDB.Query (5ms)
│  └─ Span: CacheService.Get (1ms)
└─ Span: ResponseEncode (1ms)
```

Each span records:
- Start time, duration
- Service name
- Operation name (function, handler)
- Span ID, parent span ID (trace linkage)
- Tags (key-value context): user_id, http_method, status_code
- Events: "cache miss", "retry attempt 2"
- Errors: exception message, stack trace

### Trace Context Propagation

When Service A calls Service B, the trace ID and parent span ID propagate in request headers. Service B creates a new child span as part of the same trace.

**Sampling consideration:** Capturing every request as a full trace in high-volume systems costs storage and compute. Sampling strategies:
- **Head sampling:** Sample at ingress (first service). Decision propagates downstream
- **Tail sampling:** Collect all traces, decide which to keep based on content (high error rate, high latency). More complex, more informative
- **Adaptive sampling:** Increase sample rate for errors or slow requests

### OpenTelemetry: The Observability Standard

OpenTelemetry is the CNCF standard for emitting traces, metrics, and logs:

**Components:**
- **Instrumentation:** Auto-instrumentation (framework plugins) or manual SDK calls
- **SDK:** Bundles telemetry, configures samplers, processors
- **Exporters:** Sends telemetry to backend (Jaeger, Zipkin, Datadog, Grafana Loki)
- **Collector:** Central service that receives, processes, and exports telemetry

**Advantages:**
- Vendor-neutral (no lock-in to Datadog, New Relic, etc.)
- Growing library of auto-instrumentation (databases, HTTP clients, frameworks)
- W3C Trace Context compliance
- Supports distributed tracing, metrics, logs in one framework

**Current state:** Stable for tracing, maturing for metrics and logs.

## Metrics: RED vs USE vs Four Golden Signals

Different metric frameworks serve different questions:

### RED Method (Request-Driven Systems)

For systems where users send requests (web services, APIs):

| Metric | Definition | Example |
|--------|----------|---------|
| **Rate** | Requests per second | 1000 req/s |
| **Errors** | Failed requests (5xx, timeouts) | 5% error rate |
| **Duration** | Latency (p50, p95, p99) | p99 latency: 500ms |

RED captures application-level performance. Works for microservices, APIs. Not for background jobs or event-driven systems.

### USE Method (Resource-Driven Systems)

For infrastructure and resource health:

| Metric | Definition | Example |
|--------|----------|---------|
| **Utilization** | % of capacity in use | CPU 70%, memory 85%, disk 60% |
| **Saturation** | Queue depth, contention | 10 requests queued, lock wait time 5ms |
| **Errors** | Hardware/OS errors | Disk I/O errors, page faults, drop packets |

USE is for nodes, databases, load balancers—anything with finite capacity. RED + USE together give full picture.

### Four Golden Signals (Google SRE)

Similar to RED but with slight differences:

| Signal | Definition |
|--------|----------|
| **Latency** | Time to serve request |
| **Traffic** | Demand (requests/s, bytes/s, connections) |
| **Errors** | Rate of failed requests |
| **Saturation** | How "full" the service is (CPU, memory, queue depth) |

Differs from RED by separating traffic from rate. Better for capacity planning.

### Metric Types

**Counter:** Monotonically increasing value (requests served, bytes sent). Only increases or resets.

**Gauge:** Arbitrary numeric value (CPU %, connections, queue size). Can go up or down.

**Histogram:** Distribution of values (latencies, request sizes). Tracks counts in buckets, allows p50/p95/p99 calculation.

**Summary:** Similar to histogram but pre-aggregates quantiles (more compute at collection time, simpler at query time).

## SLI, SLO, SLA: Reliability Contracts

After collecting metrics, the next step is defining acceptable reliability:

### SLI (Service Level Indicator)

An SLI is a **measurable metric** representing system health. Examples:

- Availability: % of requests receiving a response
- Latency: % of requests with latency < 200ms
- Error rate: % of requests without a 5xx error
- Throughput: Requests successfully processed per second

SLIs must correspond to **user-facing** concerns, not internal details. Example: "CPU utilization" is not an SLI (users don't care); "requests < 200ms" is.

### SLO (Service Level Objective)

An SLO is a **target** for an SLI over a time window:

*"99% of requests will complete within 200ms, measured over a 30-day window"*

SLOs are internal commitments (targets) but inform public messaging. SLOs guide operational decisions:
- SLO met: Safe to deploy, prioritize features
- SLO missed: Focus on stability, maybe halt deployments

### SLA (Service Level Agreement)

An SLA is a **contractual commitment** to customers. Failure to meet SLA incurs penalties (SLA credits, discounts, refunds).

Example: *"We guarantee 99.9% uptime. If availability < 99.9%, customer receives 10% service credit."*

Note: SLO is almost always stricter than SLA. If SLA is 99.9%, SLO might be 99.95%. Buffer allows internal recovery before customer impact.

### Error Budgets

SLO creates an error budget: allowable failures per time window.

*Example SLO: 99.5% availability per month*

```
Error budget = (100 - 99.5)% = 0.5%
Monthly budget = 0.5% × 43,200 minutes = 216 minutes (~3.6 hours)
```

Team can "spend" this budget on deployments, experiments, chaos testing. Once spent, no risky changes until month resets. Aligns incentives: want to hit SLO, can use budget wisely instead of being overly conservative.

**Error budget consumption policy:**
- Incidents that violate SLO burn budget (team must mitigate)
- Planned deployments/experiments burn budget (team chooses when)
- Unused budget carries forward (or resets monthly, depending on policy)

## Observability-Driven Development

Observability isn't just operational; it guides development:

### 1. Design for Observability from Day One

- Use structured logging, not debug prints
- Emit metrics at key decision points (fallback triggered, feature disabled)
- Instrument dependencies (database latency, cache hit rate)
- Design for traceability (propagate trace IDs through all paths)

Cost of adding observability retrofitted is high. Cost of building it in is low.

### 2. Alerting as Hypotheses

Alert thresholds represent hypotheses about system health:

*"If p99 latency > 500ms, we have a problem."*

Test hypotheses: does alert firing actually correlate with user impact? If not, adjust threshold or metric.

**Anti-pattern:** Alert fatigue. Noisy alerts (cry-wolf) desensitize teams. Better to under-alert (miss some issues) than over-alert (noise).

### 3. Dashboarding Anti-Patterns

**Too many metrics:** Dashboards with 50+ metrics are decorative, not actionable. Focus on SLO metrics and top-level health.

**Per-service dashboards:** Operators context-switch between dashboards. Central dashboard linking to service-specific detail better.

**No historical context:** Metrics without trends (today vs. yesterday vs. last month) obscure patterns.

**Hardcoded thresholds:** Baselines change with load/seasonality. Anomaly detection or dynamic thresholds more useful than fixed numbers.

### 4. Measurement-Driven Deployment Decisions

- Deploy only after alerting/metrics infrastructure is in place
- Monitor SLI impact of each deployment
- Automated rollback if SLI degrades
- Canary deployments require metric integration

Without observability, deployment is blind. Observability + progressive delivery = safe deployments.

## Tracing Pitfall: The Observability Tax

Full-fidelity tracing (every request as a trace) at massive scale consumes significant storage and compute. Companies spend millions on observability infrastructure.

Solutions:

**Sampling:** Trace only N% of requests. Misses rare issues in unsampled requests.

**Tail sampling:** Sample based on error rate, latency, or anomalies. Keeps traces you care about.

**Structured logging fallback:** For high-volume systems, structured logs + correlation IDs + metrics may replace full traces. Cheaper, sufficient for most debugging.

**Filtering:** Don't send health check requests, static assets to observability system.

## See Also

- Logging and observability infrastructure: [logging-observability.md](logging-observability.md)
- OpenTelemetry framework: [devops-opentelemetry.md](devops-opentelemetry.md)
- Prometheus and metrics collection: [devops-prometheus-grafana.md](devops-prometheus-grafana.md)
- SRE practices: [sre-incident-management.md](sre-incident-management.md)
- Resilience patterns: [architecture-resilience.md](architecture-resilience.md)