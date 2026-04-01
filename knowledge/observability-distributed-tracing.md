# Observability: Distributed Tracing

Distributed tracing follows requests across service boundaries, revealing system behavior that metrics and logs alone cannot expose: latency hotspots, cascading failures, and interdependencies between services.

## Core Model: Traces and Spans

A **trace** is the entire journey of a request through the system. A **span** is a single operation within that trace.

```
Trace ID: abc123... (propagated across all services)
├── Span: frontend GET /checkout (300ms)
│   ├── Span: auth-service authenticate (20ms)
│   ├── Span: product-service getProduct (50ms)
│   ├── Span: cart-service getCart (40ms)
│   └── Span: payment-service charge (180ms)
│       └── Span: stripe API POST /charges (175ms)
```

**Span anatomy:**

| Field | Semantic Meaning |
|-------|---|
| `trace_id` | 128-bit ID (common to all spans in the trace); used to correlate logs and metrics |
| `span_id` | 64-bit unique ID for this span |
| `parent_span_id` | `span_id` of the caller; empty for root spans |
| `name` | Operation identifier (e.g., `http.client`, `db.query.select`) |
| `kind` | `SERVER`, `CLIENT`, `PRODUCER`, `CONSUMER`, `INTERNAL` to distinguish roles |
| `start_time`, `end_time` | Precise timestamps with microsecond resolution |
| `status` | `UNSET`, `OK`, or `ERROR`; combined with status_description |
| `attributes` | Key-value context: `http.method=GET`, `http.url=/api/orders`, `http.status_code=200` |
| `events` | Timestamped annotations within the span (e.g., "cache hit", "retry attempt") |
| `links` | Cross-references to spans in other traces (rare; mostly for event-driven workflows) |

**Duration calculation:** `latency = end_time - start_time`. This exposes network delays, queueing, processing time, and garbage collection pauses that aggregated metrics hide.

## Trace Propagation: Context Across Boundaries

For distributed tracing to work, the trace and span IDs must propagate through HTTP headers, message queue metadata, or RPC calls. Two standards dominate:

### W3C Trace Context (Modern Standard)

W3C Traceparent header format:

```
traceparent: 00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01
            │  │                                │                  │
         version│  trace_id (128-bit hex)      parent_span_id     flags
                │  (32 hex digits)              (16 hex digits)    (sampled?)
```

**Fields:**
- **version**: Currently `00`; allows protocol upgrades
- **trace_id**: 128-bit ID shared by all spans in the trace (all zeros invalid)
- **parent_span_id**: 64-bit ID of the caller's span (all zeros = root span)
- **flags**: Single byte; bit 0 = sampled (1=keep, 0=drop), bits 1-7 reserved

**Tracestate header** (optional, opaque to intermediaries):

```
tracestate: congo=t61rcZ94t1,rventures=true
```

Allows vendors to inject proprietary data without breaking standard parsing. Preserves vendor chains across hops.

**Advantage:** Standard format, immutable propagation, explicit sampling decision.

### B3 Propagation (Legacy, Backwards Compatibility)

Used by Zipkin and some distributed tracing deployments. Multiple formats supported:

**Single header:**
```
b3: 80f198ee56343ba8-e457b5a2e31da47e-1
   (trace_id)-(span_id)-(sampled)
```

**Multi-header:**
```
X-B3-TraceId: 80f198ee56343ba8
X-B3-SpanId: e457b5a2e31da47e
X-B3-ParentSpanId: 05e3ac9a4f6e3b90
X-B3-Sampled: 1  or  0
X-B3-Flags: 0  (debug flag, overrides sampling decision)
```

**Advantage:** Widely supported legacy systems. Simpler for teams already using Zipkin.

**Trade-off:** No version field. Flags field limited. Less explicit about sampling semantics.

**Migration path:** Start with W3C Traceparent for new services; extract B3 headers in older systems for interoperability.

## Sampling: Controlling Trace Volume

Collecting every trace from a high-traffic service is expensive. Sampling decisions balance observability depth with storage and CPU costs.

### Head Sampling

Decision made **before** trace collection — at span creation.

```
Sampler at frontend service:
  if (user_id == VIP || error_occurred) then sample=1
  else if (random() < 0.01) then sample=1  // 1% uniform
  else sample=0
```

**Propagate decision downstream.** Child services respect the decision to maintain trace coherence.

**Pros:** Compute-cheap (no buffering), enables complex logic (sampling based on user tier, error status).
**Cons:** Lose visibility into low-probability error paths if error sampling rate is too low. Cannot retroactively increase sampling for a failed request.

### Tail Sampling

Decision made **after** trace collection — based on span attributes and behavior.

```
Collector rule:
  if (any_span.status == ERROR) then keep=true
  else if (max_latency > 1s) then keep=true
  else if (trace_duration > 500ms) then keep=true
  else if (random() < 0.01) then keep=true
  else keep=false
  
Drop unsampled spans to reduce storage.
```

**Pros:** Keep all error traces, all slow traces, and a uniform sample of fast ones. Adapts to real system behavior.
**Cons:** Requires centralized collection. Extra storage during decision window. Higher latency in decision.

**Implementation trade-off:** Tail sampling requires buffering spans at the collector for a time window (e.g., 30s). If the window closes before a slow span arrives, the trace may be dropped.

### Adaptive/Dynamic Sampling

Sampling rate adjusts based on recent traceback volume and error rates.

```
Rule: if error_rate > 5% then head_sampling_rate = 0.01 (1%)
      if error_rate < 1% then head_sampling_rate = 0.001 (0.1%)
      
Increases sampling depth during incident windows; reduces overhead in steady state.
```

**Complexity:** Requires feedback loops and cluster coordination.

## Backends: Architecture and Trade-offs

### Jaeger (CNCF Incubating)

**Storage options:**
- In-memory (development)
- Cassandra (distributed, scalable; requires ops overhead)
- Elasticsearch (simpler ops, limited querying)
- Badger (single-node, embedded)

**Query model:** Trace ID + time range lookup; fast if trace is hot, can age-out. Attribute search and correlation querying less mature.

**Sampling:** Server-side adaptive sampling available.

**Use case:** On-prem microservices; operations teams comfortable with distributed databases.

### Zipkin (Apache, Mature)

**Storage:** Cassandra, Elasticsearch, MySQL/PostgreSQL, In-Memory.

**Query model:** Rich attribute search UI. Dependency graph visualization.

**Trait:** Simpler operational model than Jaeger; more limited distributed storage options.

**Use case:** Smaller services; teams preferring UI-first exploration.

### Grafana Tempo (CNCF Incubating)

**Storage:** Object storage (S3, GCS, Azure Blob), local filesystem.

**Architecture:** Separate ingesters, distributors, queriers; designed for multi-tenant shared infrastructure.

**Cost advantage:** No index required; object storage is cheap. Traces discoverable only through trace ID or Prometheus link.

**TraceQL:** Purpose-built query language for trace data (emerging standard).

**Use case:** Cloud-native deployments; teams leveraging Grafana stack.

### AWS X-Ray

**Proprietary:** AWS-specific; locked to Lambda, ECS, EC2, managed services.

**Strengths:** Tight integration with AWS services, automatic instrumentation of CloudTrail, VPC Flow Logs.

**Sampling:** Server-side rules; complex policies possible but opaque pricing.

**Drawback:** Vendor lock-in; traces expire after 30 days.

**Use case:** AWS-first organizations; compliance with single-vendor infrastructure.

## Span Context and Instrumentation

### OpenTelemetry SDK Integration

Automatic propagation requires SDK setup:

```python
from opentelemetry import trace
from opentelemetry.sdk.trace.export.in_memory_trace_exporter import InMemoryTraceExporter
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.exporter.jaeger.thrift import JaegerExporter
from opentelemetry.instrumentation.flask import FlaskInstrumentor
from opentelemetry.instrumentation.requests import RequestsInstrumentor

# Setup Flask auto-instrumentation
FlaskInstrumentor().instrument()
RequestsInstrumentor().instrument()

jaeger_exporter = JaegerExporter(agent_host_name="localhost", agent_port=6831)
trace.set_tracer_provider(TracerProvider())
trace.get_tracer_provider().add_span_processor(BatchSpanProcessor(jaeger_exporter))
```

**Automatic instrumentation covers:**
- HTTP server (inbound requests)
- HTTP client (outbound requests)
- Database queries
- Message queue operations
- Framework-specific hooks (Django views, FastAPI routes, Flask handlers)

### Manual Span Creation

Supplement auto-instrumentation for business logic:

```python
tracer = trace.get_tracer(__name__)

def process_payment(order_id, amount):
    with tracer.start_as_current_span("process_payment") as span:
        span.set_attribute("order.id", order_id)
        span.set_attribute("amount", amount)
        
        try:
            result = payment_api.charge(amount)
            span.set_attribute("payment.status", "success")
            return result
        except PaymentDeclinedError as e:
            span.record_exception(e)
            span.set_status(trace.Status(trace.StatusCode.ERROR))
            raise
```

## Trace Analysis and Debugging

### Latency Breakdown

```
Total trace: 500ms
├── auth-service: 20ms (4%)
├── product-service: 50ms (10%)
├── cart-service: 40ms (8%)
└── payment-service: 380ms (76%)  ← BOTTLENECK
    └── stripe API: 375ms
        └── Network: 200ms
        └── Stripe processing: 175ms
```

**Insight:** Stripe API accounts for 75% of latency. Mitigation options: async payment processing, caching of auth results, batching charges.

### Critical Path Analysis

In complex traces, not all spans contribute equally to total latency. Sequential spans block; parallel spans do not.

```
Frontend (300ms total)
├─ Auth (20ms) ──→ Product (50ms) ──→ Cart (40ms) ──→ Payment (180ms)
│                    └─ Parallel Query (15ms)
└─ Dependency Graph: Auth → Product → Cart → Payment (critical path)
```

The critical path is Auth + Product + Cart + Payment = 290ms. The parallel query (15ms) doesn't extend the trace because it runs inside Product.

### Error and Exception Detection

Span status fields propagate errors:

```
Span with status=ERROR:
  error.type: "SQLException"
  error.message: "Duplicate key '12345'"
  exception.stacktrace: "...at db.insert(line42)..."
  db.error.code: "23505"  // PostgreSQL unique violation
```

**Intelligent rollup:** Traces with ANY span status=ERROR bubble up in dashboards. Enables rapid incident detection.

## Correlation with Logs and Metrics

### Trace ID in Logs

Every log line in a traced service should include `trace_id`:

```json
{
  "timestamp": "2025-03-15T10:30:00Z",
  "level": "ERROR",
  "trace_id": "abc123...",
  "span_id": "def456...",
  "message": "Payment failed",
  "user_id": 42
}
```

**Benefit:** Click from trace to logs; see raw events that span represents.

### Metrics as Span Outcomes

Prometheus metrics derived from trace outcomes:

```
# Metric 1: Count spans by status
histogram_quantile(0.95, rate(trace_span_duration_seconds_bucket[5m]))

# Metric 2: Error rate by service
sum by (service) (rate(trace_spans_total{status="error"}[5m]))
         / sum by (service) (rate(trace_spans_total[5m]))

# Metric 3: Slow endpoint detection
topk(5, rate(http_server_duration_seconds_bucket{le="+Inf"}[5m]))
```

**Insight:** Metrics aggregate; traces explain. Use metrics for alerting; use traces for diagnosis.

## Common Pitfalls

1. **Missing propagation in async code.** If child operations (callbacks, goroutines, async tasks) don't propagate trace ID, they become orphaned.

2. **Sampling rate too low.** 1% sampling may miss rare errors. Use tail sampling for errors.

3. **High cardinality span attributes.** `span.set_attribute("response_body", json_dump())` creates explosion in backend index. Restrict attributes to low-cardinality values.

4. **Forgetting context.** Middleware must extract and inject propagation headers; libraries don't do this automatically in all frameworks.

5. **No sampling decision in headers.** If propagation doesn't include sampled flag, child services re-sample independently, breaking trace cohesion.

## See Also

- structured logging (correlation IDs)
- observability-metrics (service health metrics)
- observability-alerting (SLO-driven alerts for latency)