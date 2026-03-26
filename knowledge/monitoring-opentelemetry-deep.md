# OpenTelemetry Deep Dive — SDK Architecture, Context Propagation, Exporters & Sampling

## SDK Architecture Overview

OpenTelemetry's SDKs (available in Java, Go, Python, .NET, JavaScript, Rust, etc.) implement the **three providers** that generate telemetry. Each provider is a factory for creating instruments that capture a specific signal.

### TracerProvider (Traces)

```
Application Code
    ↓
  Tracer (created by TracerProvider)
    ↓
  Spans (individual operations)
    ↓
  SpanProcessors (batching, sampling)
    ↓
  Exporters (send to backends)
```

**TracerProvider** is the entry point:

```java
// Configuration
TracerProvider tracerProvider = SdkTracerProvider.builder()
    .addSpanProcessor(new BatchSpanProcessor(jaegerExporter))
    .setResource(Resource.getDefault())
    .build();

// Usage
Tracer tracer = tracerProvider.get("my.app");
Span span = tracer.spanBuilder("processOrder").startSpan();
try (Scope scope = span.makeCurrent()) {
  // Span is active in this scope
  // Child spans created here automatically reference this parent
  doWork();
}
span.end();
```

**Key point**: Spans must be explicitly `start()`ed and `end()`ed. Without `end()`, the span hangs in memory indefinitely.

### MeterProvider (Metrics)

```
Application Code
    ↓
  Meter (created by MeterProvider)
    ↓
  Instruments (Counter, Gauge, Histogram)
    ↓
  MetricReaders (collect periodically)
    ↓
  Exporters (send to backends)
```

**MeterProvider** is the factory:

```java
MeterProvider meterProvider = SdkMeterProvider.builder()
    .registerMetricReader(
        new PeriodicMetricReader.builder(prometheusExporter)
            .setInterval(Duration.ofSeconds(10))
            .build()
    )
    .setResource(Resource.getDefault())
    .build();

Meter meter = meterProvider.get("my.app");
LongCounter requestCounter = meter.counterBuilder("http.requests").build();
requestCounter.add(1, Attributes.of("method", "GET", "status", "200"));
```

**Instruments are semantic**:

- **Counter**: Monotonically increasing (cumulative). `http.requests`, `disk.io.operations`.
- **UpDownCounter**: Can increase or decrease. `active.connections`, `memory.usage`.
- **Histogram**: Distribution. `http.request.duration_ms`, `message.size`.
- **ObservableGauge**: Snapshot callback. `cpu.usage` (queried on-demand).

### LoggerProvider (Logs)

```java
LoggerProvider loggerProvider = SdkLoggerProvider.builder()
    .addLogRecordProcessor(new BatchLogRecordProcessor(otlpExporter))
    .build();

Logger logger = loggerProvider.get("my.app");
logger.emit(
    LogRecordBuilder()
        .setMessage("User registered")
        .setSeverity(Severity.INFO)
        .build()
);
```

Logging integration is newer in OTel (traces and metrics more mature). Many projects continue using Log4j/SLF4J with **log bridges** that convert logs to OTel LogRecords.

## Context Propagation

### The Problem

In distributed systems, a request flows across process boundaries. A load balancer routes to Service A, which calls Service B, which queries a database. Each component has its own tracer. How do they know they're part of the same request?

**Answer**: Propagation. The client adds a header (`traceparent`) to outgoing requests. The server reads this header and links its spans to the parent trace.

### Propagation Format

**W3C Trace Context** (standardized, recommended):

```
traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01
            ├─ version (00)
            ├─ trace-id (4bf92f3577b34da6a3ce929d0e0e4736, 128-bit hex)
            ├─ parent-span-id (00f067aa0ba902b7, 64-bit hex)
            └─ trace-flags (01 = sampled, 00 = not sampled)
```

**Jaeger Trace Context** (older, still used):

```
uber-trace-id: trace-id:parent-span-id:flags:baggage
```

**Other formats**: Datadog, Xray, B3, gRPC trace-context.

### Propagators

A **propagator** reads/writes headers in a specific format:

```java
// Use W3C Trace Context
TextMapPropagator propagator = new W3CTraceContextPropagator();

// Extracting (server receiving request)
Context context = propagator.extract(Context.current(), httpHeaders, new HttpHeadersGetter());
TraceContext traceContext = context.get(Span.KEY);  // Parent info extracted

// Injecting (client sending request)
propagator.inject(Context.current(), httpHeaders, new HttpHeadersSetter());
// traceparent header is now set in the outgoing request
```

### Baggage

Baggage is **metadata that travels with a trace**. Unlike spans, baggage is lightweight and **always propagated** (not subject to sampling).

```java
// Baggage: user ID, request ID, feature flags
Baggage baggage = Baggage.builder()
    .put("user.id", "alice@example.com")
    .put("request.id", "req-12345")
    .put("feature.beta", "true")
    .build();

// Baggage is automatically injected into outgoing requests
// and extracted on the receiving side

// In a child service
Baggage received = Baggage.current();
String userId = received.getEntryValue("user.id");  // "alice@example.com"
```

Baggage is useful for **end-to-end metadata** without explicit parameters.

## Exporters

Exporters push telemetry to backends. They're pluggable.

### OTLP (OpenTelemetry Protocol)

OTLP is the native OTel wire format (gRPC or HTTP):

```java
OtlpGrpcSpanExporter spanExporter = OtlpGrpcSpanExporter.builder()
    .setEndpoint("https://otel-collector:4317")  // gRPC
    .build();

OtlpHttpMetricExporter metricExporter = OtlpHttpMetricExporter.builder()
    .setEndpoint("https://otel-collector:4318")  // HTTP
    .build();
```

### Vendor-Specific Exporters

- **Jaeger**: Direct export to Jaeger collector
- **Prometheus**: MetricReader that exposes metrics on `/metrics` endpoint
- **Datadog**: Send traces and metrics to Datadog API
- **New Relic**: Direct feed to New Relic ingest
- **AWS X-Ray**: Convert OTel traces to X-Ray format

## The Collector

The OpenTelemetry Collector is a **standalone service** that receives telemetry from applications and forwards to backends. It's the bridge between OTel SDKs and any observability platform.

```
App 1  ──────>
App 2  ──────> [ OTEL Collector ] ──> [ Prometheus ]
App 3  ──────> [  Receivers     ] ──> [ Jaeger      ]
        OTLP   [  Processors    ] --> [ Datadog     ]
               [  Exporters     ]     [ ...         ]
```

### Receiver Types

- **OTLP** (gRPC, HTTP): Native OTel protocol
- **Prometheus**: Scrape Prometheus targets
- **Jaeger**: Accept Jaeger traces
- **Zipkin**: Accept Zipkin traces
- **Syslog**: Parse syslog events
- **Kafka**: Consume from topic
- **Hostmetrics**: Collect OS metrics (CPU, memory, disk)

### Processors

Processors transform or filter data in-flight:

- **Batch**: Buffer and send in batches (reduces network round-trips)
- **Memory Limiter**: Drop data if memory exceeds threshold
- **Sampler**: Sample spans/traces based on attributes
- **Attributes**: Add, drop, or modify attributes (e.g., add env label)
- **Resource Detection**: Detect cloud provider, container, Kubernetes metadata

Example config:

```yaml
receivers:
  otlp:
    protocols:
      grpc: {}
      http: {}

processors:
  batch:
    send_batch_size: 1000
    timeout: 10s
  
  memory_limiter:
    check_interval: 1s
    limit_mib: 512  # Drop data if memory > 512 MB
  
  attributes:
    actions:
      - key: environment
        value: production
        action: insert

exporters:
  prometheus:
    endpoint: "0.0.0.0:8888"
  jaeger:
    endpoint: "jaeger:14250"

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [memory_limiter, batch, attributes]
      exporters: [jaeger]
    metrics:
      receivers: [otlp, prometheus]
      processors: [batch, attributes]
      exporters: [prometheus]
```

## Sampling Strategies

### The Problem

Tracing every request is expensive (network, storage, computation). A high-throughput service (1M requests/sec) can't afford to trace all. But tracing **none** means you miss errors.

### Sampling Decision

Sampling happens at **collection time**:

1. **Head-based**: Collector decides on first span whether to sample (sample the whole trace or discard)
2. **Tail-based**: Collector waits for full trace, then decides based on outcome (sample error traces, discard success)

### Common Strategies

**always_on**: Trace everything (development).

**always_off**: Trace nothing (disable tracing).

**trace_id_ratio**: Sample based on trace ID % (stateless, deterministic):

```yaml
processors:
  tail_sampling:
    policies:
      - name: sample_all_traces_with_errors
        traces_per_second: 100
        # Sample if any span has an error attribute
        status_code:
          status_codes: [ERROR]
```

**adaptive_sampling**: Adjust sample rate based on traffic (high traffic = lower %, low traffic = higher %).

**Error-based**: Always sample traces with errors or high latency:

```yaml
policies:
  - name: error_traces
    traces_per_second: 1000
    status_code:
      status_codes: [ERROR, UNSET]
```

### Sampling Trade-offs

- **Head-based (early decision)**: Lightweight, doesn't require buffering full traces, but may sample out errors before detecting them.
- **Tail-based**: Sees full trace, can make smart decisions, but requires buffering (higher memory, latency).

Recommendation: Use **head-based with high ratio (10% or higher)** for errors, **low ratio (0.1%)** for success paths.

## Auto-Instrumentation

Auto-instrumentation wraps frameworks without code changes:

```bash
# Java agent
java -javaagent:opentelemetry-javaagent.jar \
     -Dotel.service.name=my-app \
     -Dotel.exporter.otlp.endpoint=http://collector:4317 \
     -jar myapp.jar
```

The agent:
- **Injects bytecode** at runtime to intercept framework calls
- **Creates spans** for HTTP handlers, database queries, cache operations
- **Propagates context** automatically across service boundaries
- **Exports** to configured backend

Supported auto-instrumentation plugins:

| Language | System | Example |
|----------|--------|---------|
| Java | Spring, Tomcat, Jetty, Hibernate | Auto-spans for @RequestMapping, JDBC |
| Python | Django, Flask, SQLAlchemy | Auto-spans for routes, ORM queries |
| Go | net/http, gRPC, database/sql | Transport-level instrumentation |
| .NET | ASP.NET Core, EF Core, HTTP client | ILogger capture, dependency resolution |

### Manual vs Auto

**Auto-instrumentation** is easiest (no code change, deploy agent), but **coarse-grained** (spans per HTTP request, not per business operation).

**Manual instrumentation** requires code changes, but **fine-grained** (create spans for specific logic: shopping cart checkout, payment processing).

Typical workflow: Auto-instrument first (framework level), then add manual spans for **domain concepts** (user registration, order fulfillment).

## Related Topics

See also: [devops-opentelemetry.md](devops-opentelemetry.md), [observability-distributed-tracing.md](observability-distributed-tracing.md), [devops-observability-patterns.md](devops-observability-patterns.md), [monitoring-prometheus-deep.md](monitoring-prometheus-deep.md).