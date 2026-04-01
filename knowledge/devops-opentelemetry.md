# OpenTelemetry — Observability Framework

## Overview

OpenTelemetry (OTel) is a vendor-neutral observability framework for generating, collecting, and exporting telemetry data. It provides APIs, SDKs, and tools for the three pillars of observability: traces, metrics, and logs.

**Key principle:** Instrumentation is decoupled from backends. Instrument once with OTel, export to any observability platform (Jaeger, Prometheus, Datadog, Grafana, New Relic, etc.).

## Three Signals

### Traces

Distributed traces track request flow across service boundaries. A trace is a tree of **spans**.

```
Trace: abc123
├── Span: HTTP GET /api/orders (frontend, 120ms)
│   ├── Span: authenticate (auth-service, 15ms)
│   ├── Span: SELECT * FROM orders (db, 45ms)
│   └── Span: GET /api/inventory (inventory-service, 50ms)
│       └── Span: Redis HGET inventory:* (cache, 3ms)
```

**Span anatomy:**
| Field | Description |
|-------|-------------|
| TraceID | 128-bit ID shared by all spans in a trace |
| SpanID | 64-bit unique ID for this span |
| ParentSpanID | SpanID of the parent (empty for root span) |
| Name | Operation name (e.g., `HTTP GET /api/orders`) |
| Kind | CLIENT, SERVER, PRODUCER, CONSUMER, INTERNAL |
| StartTime / EndTime | Timestamps |
| Status | UNSET, OK, ERROR |
| Attributes | Key-value pairs (e.g., `http.method=GET`) |
| Events | Timestamped annotations within a span |
| Links | References to spans in other traces |

### Metrics

Numerical measurements aggregated over time. Three semantic types:

| Instrument                  | Type                              | Example                            |
| --------------------------- | --------------------------------- | ---------------------------------- |
| Counter                     | Monotonically increasing sum      | `http.server.request.count`        |
| UpDownCounter               | Sum that can increase or decrease | `queue.depth`                      |
| Histogram                   | Distribution of values (buckets)  | `http.server.request.duration`     |
| Gauge                       | Point-in-time value               | `system.cpu.utilization`           |
| Observable (async) variants | Callback-based measurement        | `process.runtime.jvm.memory.usage` |

**Counter vs Gauge:** Counters always go up (resets allowed). Gauges can go up or down and represent current state. Use counters for "how many" and gauges for "how much right now."

**Histogram:** Records a distribution — the SDK tracks count, sum, and configurable bucket boundaries. Query for percentiles (p50, p95, p99), averages, and rates.

### Logs

Structured log records correlated with traces and metrics via context:

```json
{
  "timestamp": "2025-03-15T10:30:00Z",
  "severity": "ERROR",
  "body": "Failed to process order",
  "attributes": {
    "order.id": "12345",
    "error.type": "PaymentDeclined"
  },
  "trace_id": "abc123...",
  "span_id": "def456..."
}
```

OTel logs bridge existing logging libraries (Log4j, SLF4J, Python logging, slog) — you don't replace your logger, you connect it.

## SDK Architecture

```
┌─────────────────────────────────┐
│         Application Code        │
│   tracer.start_span("work")    │
│   meter.create_counter("req")  │
└──────────┬──────────────────────┘
           │ API calls
┌──────────▼──────────────────────┐
│           OTel SDK              │
│  ┌──────────────────────────┐   │
│  │ TracerProvider            │   │
│  │  └─ SpanProcessor(s)     │   │
│  │      └─ SpanExporter     │   │
│  ├──────────────────────────┤   │
│  │ MeterProvider             │   │
│  │  └─ MetricReader(s)      │   │
│  │      └─ MetricExporter   │   │
│  ├──────────────────────────┤   │
│  │ LoggerProvider            │   │
│  │  └─ LogRecordProcessor(s)│   │
│  │      └─ LogRecordExporter│   │
│  └──────────────────────────┘   │
└─────────────────────────────────┘
```

### TracerProvider Setup (Python)

```python
from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.resources import Resource

resource = Resource.create({
    "service.name": "order-service",
    "service.version": "1.2.3",
    "deployment.environment": "production",
})

provider = TracerProvider(resource=resource)
provider.add_span_processor(
    BatchSpanProcessor(OTLPSpanExporter(endpoint="http://otel-collector:4317"))
)
trace.set_tracer_provider(provider)

tracer = trace.get_tracer("order-service", "1.2.3")
```

### MeterProvider Setup (Python)

```python
from opentelemetry import metrics
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.metrics.export import PeriodicExportingMetricReader
from opentelemetry.exporter.otlp.proto.grpc.metric_exporter import OTLPMetricExporter

reader = PeriodicExportingMetricReader(
    OTLPMetricExporter(endpoint="http://otel-collector:4317"),
    export_interval_millis=30000,
)
provider = MeterProvider(resource=resource, metric_readers=[reader])
metrics.set_meter_provider(provider)

meter = metrics.get_meter("order-service", "1.2.3")
request_counter = meter.create_counter("http.server.requests", unit="1", description="Total HTTP requests")
request_duration = meter.create_histogram("http.server.duration", unit="ms", description="Request duration")
```

### Creating Spans

```python
# Basic span
with tracer.start_as_current_span("process-order") as span:
    span.set_attribute("order.id", order_id)
    span.set_attribute("order.total", 99.99)
    result = process(order_id)
    if error:
        span.set_status(trace.StatusCode.ERROR, "Payment failed")
        span.record_exception(error)

# Nested spans (automatic parent-child via context)
with tracer.start_as_current_span("handle-request"):
    with tracer.start_as_current_span("validate-input"):
        validate()
    with tracer.start_as_current_span("query-database"):
        query()

# Add events (structured logs within a span)
span.add_event("cache-miss", {"cache.key": "user:123"})
```

### Node.js SDK

```typescript
import { NodeSDK } from "@opentelemetry/sdk-node";
import { OTLPTraceExporter } from "@opentelemetry/exporter-trace-otlp-grpc";
import { OTLPMetricExporter } from "@opentelemetry/exporter-metrics-otlp-grpc";
import { PeriodicExportingMetricReader } from "@opentelemetry/sdk-metrics";
import { getNodeAutoInstrumentations } from "@opentelemetry/auto-instrumentations-node";

const sdk = new NodeSDK({
  serviceName: "order-service",
  traceExporter: new OTLPTraceExporter({ url: "http://otel-collector:4317" }),
  metricReader: new PeriodicExportingMetricReader({
    exporter: new OTLPMetricExporter({ url: "http://otel-collector:4317" }),
  }),
  instrumentations: [getNodeAutoInstrumentations()],
});

sdk.start();
```

## Exporters

| Protocol        | Port | Format           | Use Case                   |
| --------------- | ---- | ---------------- | -------------------------- |
| OTLP/gRPC       | 4317 | Protobuf         | Default, best performance  |
| OTLP/HTTP       | 4318 | Protobuf or JSON | Firewall-friendly, browser |
| Prometheus      | 8889 | Prometheus text  | Prometheus scraping        |
| Zipkin          | 9411 | JSON             | Legacy Zipkin backends     |
| Jaeger (native) | —    | —                | Deprecated, use OTLP       |

**OTLP is the standard.** Most backends accept OTLP natively. Use backend-specific exporters only when OTLP isn't supported.

## OTel Collector

Vendor-agnostic proxy that receives, processes, and exports telemetry data. Decouples applications from backends.

```
┌──────────┐   ┌──────────────────────────────────────┐   ┌──────────┐
│  App 1   │──►│           OTel Collector              │──►│  Jaeger  │
│  App 2   │──►│  Receivers → Processors → Exporters   │──►│Prometheus│
│  App 3   │──►│                                       │──►│  Loki    │
└──────────┘   └──────────────────────────────────────┘   └──────────┘
```

### Collector Configuration

```yaml
# otel-collector-config.yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318
  prometheus:
    config:
      scrape_configs:
        - job_name: "node-exporter"
          static_configs:
            - targets: ["localhost:9100"]
  hostmetrics:
    collection_interval: 30s
    scrapers:
      cpu:
      memory:
      disk:

processors:
  batch:
    timeout: 5s
    send_batch_size: 1024
    send_batch_max_size: 2048
  memory_limiter:
    check_interval: 1s
    limit_mib: 512
    spike_limit_mib: 128
  attributes:
    actions:
      - key: environment
        value: production
        action: upsert
  filter:
    error_mode: ignore
    traces:
      span:
        - 'attributes["http.target"] == "/health"' # drop health checks
  tail_sampling:
    decision_wait: 10s
    policies:
      - name: errors
        type: status_code
        status_code: { status_codes: [ERROR] }
      - name: slow-requests
        type: latency
        latency: { threshold_ms: 1000 }
      - name: probabilistic
        type: probabilistic
        probabilistic: { sampling_percentage: 10 }

exporters:
  otlp/jaeger:
    endpoint: jaeger:4317
    tls:
      insecure: true
  prometheus:
    endpoint: 0.0.0.0:8889
  loki:
    endpoint: http://loki:3100/loki/api/v1/push

connectors:
  spanmetrics: # derive metrics from trace spans
    dimensions:
      - name: http.method
      - name: http.status_code

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [memory_limiter, batch]
      exporters: [otlp/jaeger, spanmetrics]
    metrics:
      receivers: [otlp, prometheus, spanmetrics]
      processors: [memory_limiter, batch]
      exporters: [prometheus]
    logs:
      receivers: [otlp]
      processors: [memory_limiter, attributes, batch]
      exporters: [loki]
  telemetry:
    logs:
      level: info
    metrics:
      address: 0.0.0.0:8888 # collector's own metrics
```

### Deployment Patterns

| Pattern             | Description                                                          |
| ------------------- | -------------------------------------------------------------------- |
| **Agent**           | Sidecar/DaemonSet on each node, low processing, forwards to gateway  |
| **Gateway**         | Central collector cluster, heavy processing (sampling, enrichment)   |
| **Agent + Gateway** | Agents collect locally, gateway aggregates and exports (recommended) |

## Auto-Instrumentation

Zero-code instrumentation for common libraries. Injects tracing/metrics into HTTP clients, database drivers, messaging systems automatically.

```bash
# Python — wrap your app
opentelemetry-instrument \
  --service_name order-service \
  --exporter_otlp_endpoint http://collector:4317 \
  python app.py

# Java — agent JAR
java -javaagent:opentelemetry-javaagent.jar \
  -Dotel.service.name=order-service \
  -Dotel.exporter.otlp.endpoint=http://collector:4317 \
  -jar app.jar

# Node.js — require before app
node --require @opentelemetry/auto-instrumentations-node/register app.js
```

### Kubernetes Auto-Instrumentation (Operator)

```yaml
apiVersion: opentelemetry.io/v1alpha1
kind: Instrumentation
metadata:
  name: auto-instrumentation
spec:
  exporter:
    endpoint: http://otel-collector:4317
  python:
    image: ghcr.io/open-telemetry/opentelemetry-operator/autoinstrumentation-python:latest
  java:
    image: ghcr.io/open-telemetry/opentelemetry-operator/autoinstrumentation-java:latest
---
# Annotate pods for injection
apiVersion: apps/v1
kind: Deployment
metadata:
  name: order-service
spec:
  template:
    metadata:
      annotations:
        instrumentation.opentelemetry.io/inject-python: "true"
```

## Context Propagation

Carries trace context across process boundaries (HTTP headers, message queue headers).

### W3C TraceContext (Standard)

```
traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01
             ^^-^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^-^^^^^^^^^^^^^^^^-^^
             ver       trace-id (32 hex)         parent-id (16)  flags
                                                                 01=sampled

tracestate: vendor1=value1,vendor2=value2
```

**Propagation happens automatically** when using OTel SDK with HTTP client instrumentation. For manual propagation:

```python
from opentelemetry.propagate import inject, extract

# Inject context into outgoing request headers
headers = {}
inject(headers)
requests.get("http://other-service/api", headers=headers)

# Extract context from incoming request
context = extract(request.headers)
with tracer.start_as_current_span("handle", context=context):
    ...
```

Other propagation formats: B3 (Zipkin), Jaeger, AWS X-Ray. Configure via `OTEL_PROPAGATORS=tracecontext,baggage,b3multi`.

## Sampling

Control volume of collected trace data. Critical for cost management at scale.

### Head Sampling (SDK-level)

Decision made at trace start — all-or-nothing for entire trace:

```python
from opentelemetry.sdk.trace.sampling import TraceIdRatioBased, ParentBased

# Sample 10% of traces
sampler = TraceIdRatioBased(0.1)

# Respect parent's sampling decision, sample 10% of root spans
sampler = ParentBased(root=TraceIdRatioBased(0.1))

provider = TracerProvider(sampler=sampler, resource=resource)
```

| Sampler                   | Behavior                                   |
| ------------------------- | ------------------------------------------ |
| `ALWAYS_ON`               | Sample everything                          |
| `ALWAYS_OFF`              | Sample nothing                             |
| `TraceIdRatioBased(rate)` | Probabilistic by trace ID                  |
| `ParentBased(root=X)`     | Use parent's decision, or `X` if no parent |

### Tail Sampling (Collector-level)

Decision made after trace is complete — can sample based on outcome (errors, latency, attributes). Requires the collector to buffer complete traces:

```yaml
# In OTel Collector config
processors:
  tail_sampling:
    decision_wait: 10s # wait for all spans to arrive
    num_traces: 100000 # max traces in memory
    policies:
      - name: keep-errors
        type: status_code
        status_code: { status_codes: [ERROR] }
      - name: keep-slow
        type: latency
        latency: { threshold_ms: 2000 }
      - name: sample-rest
        type: probabilistic
        probabilistic: { sampling_percentage: 5 }
```

**Head vs Tail:** Head sampling is simpler and cheaper (less data generated). Tail sampling is more intelligent (keeps interesting traces) but requires collector buffering and more resources.

## Semantic Conventions

Standardized attribute names for consistent telemetry across services and languages. Defined in OpenTelemetry spec.

| Category    | Key Attributes                                                                   |
| ----------- | -------------------------------------------------------------------------------- |
| HTTP Server | `http.request.method`, `url.path`, `http.response.status_code`, `server.address` |
| HTTP Client | `http.request.method`, `url.full`, `server.address`, `server.port`               |
| Database    | `db.system`, `db.statement`, `db.operation`, `db.name`                           |
| Messaging   | `messaging.system`, `messaging.operation`, `messaging.destination.name`          |
| RPC         | `rpc.system`, `rpc.service`, `rpc.method`                                        |
| Resource    | `service.name`, `service.version`, `deployment.environment`, `host.name`         |
| Exception   | `exception.type`, `exception.message`, `exception.stacktrace`                    |

### Environment Variables

```bash
OTEL_SERVICE_NAME=order-service
OTEL_EXPORTER_OTLP_ENDPOINT=http://collector:4317
OTEL_EXPORTER_OTLP_PROTOCOL=grpc           # or http/protobuf
OTEL_TRACES_SAMPLER=parentbased_traceidratio
OTEL_TRACES_SAMPLER_ARG=0.1
OTEL_PROPAGATORS=tracecontext,baggage
OTEL_LOGS_EXPORTER=otlp
OTEL_METRICS_EXPORTER=otlp
OTEL_RESOURCE_ATTRIBUTES=deployment.environment=production,service.version=1.2.3
```
