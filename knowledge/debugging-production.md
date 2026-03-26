# Debugging Production — Observability, Tracing, and Safe Investigation

Production systems are hostile environments for debugging. You cannot attach interactive debuggers without freezing the system. You cannot recompile and redeploy on a whim. You cannot stress-test to reproduce rare issues. Instead, production debugging relies on **passive observation**: reading what the system tells you through logs, metrics, and traces.

## The Observability Foundation

**Observability** is the ability to understand a system's internal state from external observations alone (logs, metrics, traces). It's not a tool but a design philosophy: every important event must be measurable without stopping the system.

The three pillars map to different granularity levels:

- **Metrics**: aggregated counts and time series (e.g., "95th percentile latency is 200ms")
- **Logs**: discrete events with context ("request processed by service X")
- **Traces**: connected spans showing a single request's journey through the system

## Distributed Tracing and Correlation IDs

A single user request in a microservice system touches many services:

```
User Request
  ↓ (trace ID: abc123)
API Gateway (span: gateway-in)
  ↓ (trace ID: abc123)
User Service (span: user-lookup)
  ↓ (trace ID: abc123)
Order Service (span: order-fetch)
  ↓ (trace ID: abc123)
Payment Service (span: payment-validate)
```

**Distributed tracing** follows a single request through all these services, recording timing and state at each step. The **correlation ID** (or trace ID) is the glue: it's propagated in HTTP headers, message metadata, or context objects.

### Implementing Correlation IDs

**At entry point**, generate and propagate:

```python
from uuid import uuid4
from flask import request, g

@app.before_request
def before():
    trace_id = request.headers.get('X-Trace-ID') or str(uuid4())
    g.trace_id = trace_id
    # Pass to downstream services
```

**In logs**, always include:

```python
import logging
logging.basicConfig(
    format='%(asctime)s trace_id=%(trace_id)s %(message)s'
)
logger = logging.getLogger()
logger.info('Processing', extra={'trace_id': g.trace_id})
```

**In outbound requests**, inject the header:

```python
import requests
headers = {'X-Trace-ID': g.trace_id}
response = requests.get('http://order-service/api/orders', headers=headers)
```

**Benefits:** Grep logs by trace ID to reconstruct the exact sequence of events for a single request. Much more valuable than unordered logs from multiple services.

### Trace Sampling

Tracing every request is expensive. Instead, **sample** a percentage (e.g., 1 in 100 requests at baseline, more for errors):

```python
def should_sample(trace_id, error_occurred=False):
    if error_occurred:
        return True  # always sample errors
    return hash(trace_id) % 100 < 1  # 1% baseline
```

Sampling reduces storage and processing costs while retaining visibility into both normal and exceptional paths.

## Observability-Driven Debugging

**The workflow:**

1. **Alert triggers** (latency spike, error rate increase)
2. **Query metrics** to narrow scope (which service? which operation?)
3. **Inspect traces** for that time window and service
4. **Read logs** for the specific trace IDs, focusing on state transitions
5. **Correlate with deployments/code changes** to find root cause

Example: "Orders are taking 5 seconds instead of 500ms"

1. **Metrics**: tail latency percentile at 5s; normal is 500ms at peak
2. **Traces**: sample a slow order request; see that `payment-service` is slow
3. **Logs**: payment-service logs show "database connection timeout"; all requests during that window timed out
4. **RCA**: database connection pool exhaustion; `payment-service` didn't close connections after failure

This discipline makes production debugging methodical rather than chaotic.

## Canary Debugging

**Canary debugging** is a deployment strategy where a new version handles a small percentage of production traffic (e.g., 1-5% initially). Issues are detected in production traffic without impacting most users.

**Workflow:**

1. Deploy new version to 1% of servers
2. Monitor metrics (error rate, latency, resource usage) for the canary fleet
3. If metrics degrade, rollback immediately
4. If healthy, gradually increase traffic (1% → 5% → 25% → 100%)

**Advantages:** Catches environment-specific bugs, load-dependent issues, and configuration errors that escape staging tests.

**Safety:** The blast radius is limited. If the canary version is buggy, only 1% of users are affected. Rollback happens automatically if metrics cross thresholds.

## Traffic Replay and Shadow Defenses

### Traffic Replay

**Traffic replay** records production traffic and replays it against a new version in a staging environment. Advantages:

- Tests against real-world traffic patterns, not synthetic data
- Detects edge cases that staging didn't exercise
- Characterizes performance under production load

**Tools:** GoReplay, Gor, or custom log parsing:

```bash
# Record production traffic
gor --input-raw :8080 --output-file=requests.log

# Replay against staging
gor --input-file=requests.log --output-http=http://staging:8080
```

### Shadow Traffic

**Shadow traffic** (or "dark traffic") asynchronously copies a percentage of production traffic to a shadow version without letting that version respond to real users:

```
User Request → Live Service [responds to user]
           ↘ Shadow Service [processes silently]
```

Shadow traffic tests new code against real production data without impacting user experience. Differences in behavior (unexpected latency, errors) are logged for investigation.

**Example (Kubernetes Istio):**
```yaml
apiVersion: networking.istio.io/v1beta1
kind: VirtualService
metadata:
  name: my-service
spec:
  hosts:
  - my-service
  http:
  - match:
    - uri:
        prefix: /api
    route:
    - destination:
        host: my-service-v1
    mirror:
      host: my-service-v2  # shadow version
    mirrorPercent: 100     # mirror 100% of traffic
```

## Log Analysis and Pattern Recognition

Logs are unstructured by nature but contain crucial information. Effective log analysis requires:

### Structured Logging

Emit logs as structured data (JSON), not free-form text:

```python
import json
import logging

class JSONFormatter(logging.Formatter):
    def format(self, record):
        log_obj = {
            'timestamp': self.formatTime(record),
            'level': record.levelname,
            'message': record.getMessage(),
            'trace_id': getattr(record, 'trace_id', 'N/A'),
            'component': record.name
        }
        if record.exc_info:
            log_obj['exception'] = self.formatException(record.exc_info)
        return json.dumps(log_obj)

handler = logging.StreamHandler()
handler.setFormatter(JSONFormatter())
logger.addHandler(handler)
```

### Cardinality Awareness

Avoid high-cardinality fields (fields with many unique values) in log aggregation. Example anti-pattern:

```python
# BAD: user_id has millions of unique values
logger.info('User login', extra={'user_id': user_id})
```

Better: commit cardinality-heavy values only to trace storage:

```python
# logs: high-level summary
logger.info('Login event', extra={'region': region, 'auth_method': method})

# traces: full detail including user_id
tracer.record_span('user_login', {'user_id': user_id, 'region': region})
```

## State Capture at Failure

When a critical error occurs, capture state before recovery:

```python
def try_critical_operation():
    try:
        return perform_operation()
    except CriticalException as e:
        # Capture state for diagnosis
        state = {
            'connection_pool_size': pool.size(),
            'active_requests': len(active),
            'last_error': str(e),
            'heap_usage_mb': psutil.virtual_memory().used / 1e6
        }
        logger.error('Operation failed', extra=state)
        raise
```

## Debugging Difficult Issues: Lock-Free Investigation

Production debugging must be **read-only**. Modifications can:
- Introduce latency side effects
- Alter timing (Heisenbug)
- Risk cascading failures
- Violate compliance (HIPAA, GDPR)

**Safe techniques:**

1. **Sampling requests**: Log every Nth request's full state without performance cost
2. **Metrics snapshots**: Capture internal counters at high frequency (every second)
3. **Heap dumps**: Snapshot memory state without freezing the JVM (most runtimes support this)
4. **Traffic inspection**: tcpdump, wireshark, or protocol-aware logging captures what went over the network
5. **Kernel tracing**: Linux eBPF traces system calls and network events without touching application code

**What not to do:**
- Don't insert breakpoints (freezes the system)
- Don't change log levels at runtime (may hide failures)
- Don't modify state to "fix" transient issues (masks root cause)
- Don't add critical instrumentation inside locks (may deadlock)

## Tools and Platforms

### Jaeger (Distributed Tracing)

Jaeger stores and visualizes traces. A trace shows all spans for a single request, with timing and error status:

```
GET /api/orders [100ms]
  ├─ authenticate [5ms]
  ├─ fetch-user [10ms]
  ├─ fetch-orders [50ms]
  │   ├─ db-query [45ms]
  │   └─ cache-check [2ms]
  └─ serialize [3ms]
```

### Datadog, New Relic, Honeycomb

These are commercial observability platforms. They aggregate logs, metrics, and traces, then provide dashboards for analyzing relationships between them.

### OpenTelemetry

An open standard for emitting observability data (traces, metrics, logs) to any backend. Reduces vendor lock-in and allows switching platforms without changing application code.

## Discipline and Culture

Production debugging is less about exotic tools and more about discipline:

1. **Plan for observability from day one.** Add correlation IDs, structured logging, and metrics to the architecture before the system goes live.
2. **Monitor what matters.** Focus on SLOs (Service Level Objectives): error rate, latency percentiles, availability.
3. **Practice incident response.** Run game days where the team debugs simulated failures using only production-like observability.
4. **Document post-mortems.** After an incident, record what was observed, how root cause was found, and what instrumentation was missing.
5. **Rotate on-call.** The developers closest to failures are the best positioned to fix them. Rotating on-call duty keeps the team sharp and improves observability investments.

A well-tuned production system talks; you just have to listen carefully.