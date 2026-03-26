# Logging Best Practices — Levels, Structured Logging, Correlation IDs, PII, Sampling, and Cost Management

Logging is cheap in development and expensive in production. Best practices balance visibility against cost: you cannot log everything, so log strategically. This note covers the tactical implementation layer; see observability-logging.md for architecture.

## Log Levels: Semantics and When to Use Each

Log levels are a contract between producers and consumers. Choose a level based on the audience and use case, not your mood.

### TRACE (Deepest)

Used for very detailed execution flow, typically disabled in production.

```
TRACE: Entering function calculate_tax(amount=100, state='CA')
TRACE: Loaded tax table from cache
TRACE: Applied progressive rate tier 2 (22%)
TRACE: Round result: 22.00 -> 22
```

**When to use:** Development, debugging a specific subsystem, deep system instrumentation in labs.
**Cost:** Very high if left on; usually disabled by default.

**Pitfall:** TRACE logs that fire on every loop iteration in real-time systems. Disable TRACE in production or use dynamic configuration (see sampling, below).

### DEBUG

Diagnostic messages useful to developers debugging production issues, but not on the happy path.

```
DEBUG: Retrying request to API (attempt 2 of 3)
DEBUG: Cache miss for key user:4521; fetching from database
DEBUG: SQL query took 145ms
```

**When to use:** Development, staging, production debugging sessions (temporarily enabled).
**Cost:** Moderate; safe to leave on in staging, usually disabled in production unless investigating.

**Guideline:** DEBUG logs should help answer "why didn't the happy path work?" They're not narrative; they're diagnostic.

### INFO

Significant events in the application's lifecycle: startups, deployments, state changes, major milestones.

```
INFO: Server started on port 8080
INFO: Connected to database (5 connection pool active)
INFO: User 4521 logged in from 203.0.113.42
INFO: Payment processed: $49.99 (order #98765)
```

**When to use:** Production.
**Cost:** Low; designed to be always on.

**Guideline:** One INFO log per significant event. Too many INFO logs become noise (you don't need to log every HTTP request at INFO level; that's what metrics are for).

### WARN

Unexpected but recoverable conditions: use of deprecated APIs, unusual input, partial failures.

```
WARN: Deprecated API endpoint /api/v1/users used; migrate to /api/v2/users
WARN: Email address unusually long (256 chars); accepting but flagging for review
WARN: Database pool 80% full; may have connection leak
WARN: Retry budget exhausted for service X; falling back to cached response
```

**When to use:** Production.
**Cost:** Low; worth alerting on if correlated with user impact.

**Guideline:** WARN is not "INFO if things go wrong." It's "this is a condition that demands attention, but the system recovered."

**Common pitfall:** Overusing WARN for non-actionable conditions ("Timeout waiting for lock" in a lock-contention scenario). If it's truly recoverable and expected under load, use DEBUG. Save WARN for anomalies.

### ERROR

A failure in a specific operation that did not crash the system.

```
ERROR: Failed to send email to user@example.com: SMTP timeout after 30s
ERROR: Payment declined with error code INSUFFICIENT_FUNDS
ERROR: Cache flush failed; server proceeding with stale data
```

**When to use:** Production.
**Cost:** Low; should trigger alerting.

**Guideline:** One ERROR per discrete failure. If an operation has 10 steps and step 7 fails, log the error at step 7, not at steps 1-6.

**Distinction from WARN:** ERROR is "a specific request/operation failed." WARN is "an anomalous condition that might affect many requests."

### FATAL / CRITICAL

The application is about to crash or become unusable; this is the last log message before death.

```
FATAL: Database connection lost and all retries failed; shutting down
FATAL: Out of memory; cannot allocate buffer for critical path
```

**When to use:** Immediately before exit(1).
**Cost:** Very low; appears once.

**Guideline:** FATAL is not "something bad happened." It's "the system is terminating."

**Anti-pattern:** Logging FATAL then trying to recover. That's confusing. If the system recovers, it wasn't FATAL; it was ERROR.

### Production Level Recommendation

```
TRACE:   Off (disable or dynamically enable only via feature flags)
DEBUG:   Off (enable only for active investigation)
INFO:    On (significant events, deployments, state changes)
WARN:    On (anomalies that may need attention)
ERROR:   On (operation failures, alerting)
FATAL:   On (shutdown events)
```

## Structured Logging: JSON Over Text

Unstructured logs are human-readable but unmachine-querable. Structured logs are JSON (or key-value), enabling filtering, aggregation, and analysis.

### Unstructured

```
2025-03-15 10:30:00 ERROR Payment failed for user 42 with amount 99.5: Card declined
```

You can grep this, but extracting the user ID requires parsing.

### Structured (JSON)

```json
{
  "timestamp": "2025-03-15T10:30:00Z",
  "level": "ERROR",
  "message": "Payment failed",
  "user_id": 42,
  "amount": 99.5,
  "currency": "USD",
  "error_code": "CARD_DECLINED",
  "service": "payment-api",
  "host": "prod-api-3",
  "trace_id": "abc123def456"
}
```

A log aggregation system (ELK, Loki) can query: "How many payments failed with CARD_DECLINED in the last hour?" This is impossible with unstructured logs.

### Structured Logging Recommendations

1. **Core fields:** Always include timestamp (ISO 8601), level, message, service name, host.
2. **Context fields:** Include IDs (user_id, request_id, transaction_id) and domain-specific fields (amount, currency, status_code).
3. **Never log redundantly:** Don't log the same information at multiple levels (ERROR should not duplicate WARN context).
4. **Use consistent field names:** Don't use "user_id" in one log and "userId" in another. Standardize.
5. **Avoid structured-logging bloat:** Not every variable is worth logging. Log what's needed to understand the failure and trace user impact.

## Correlation IDs: Tracing Requests Across Services

In a distributed system, a user's request touches multiple services. Without correlation, understanding the full path is impossible.

### Pattern: Correlation ID (Trace ID)

Generate or receive a unique ID representing the entire request's journey:

```
Client → API Gateway [trace_id: abc123]
         ↓
       Service A [trace_id: abc123]
         ↓
       Service B [trace_id: abc123]
         ↓
       Service C [trace_id: abc123]
```

Every log from every service includes the trace_id. A monitoring system queries all logs with trace_id=abc123 to reconstruct the request flow.

### Implementation

```python
# Pseudocode
def handle_request(request):
    trace_id = request.headers.get("X-Trace-ID") or generate_uuid()
    
    # Pass trace_id to all downstream calls
    response = call_service_b(data, trace_id=trace_id)
    
    # Include trace_id in every log
    logger.info("Processing request", trace_id=trace_id, user_id=user_id)
    
    # Return trace_id to client for debugging
    response.headers["X-Trace-ID"] = trace_id
    return response
```

### Span IDs: Additional Granularity

In distributed tracing (OpenTelemetry), each service's work on a request gets a span_id. Multiple spans share one trace_id:

```json
{
  "trace_id": "abc123",         # Global request ID
  "span_id": "span-service-a",  # This service's work
  "parent_span_id": null,        # No parent (this is the root)
  "service": "api-gateway",
  "message": "Received request"
}
```

**See also:** observability-logging.md and observability-distributed-tracing.md for deeper patterns.

## PII Redaction: Protecting Sensitive Data

PII (Personally Identifiable Information) — names, emails, phone numbers, credit cards, social security numbers — must not appear in logs retained long-term.

### Redaction Strategies

1. **Never log it:** Best strategy. If you don't log the credit card, you can't leak it. Use hashes or tokens instead:
   ```
   ❌ ERROR: Payment failed for card 4111111111111111
   ✓  ERROR: Payment failed for card token tok_abc123
   ```

2. **Immediate redaction:** Log the value, but replace it before sending to persistent storage:
   ```
   logger.info("User email", email=redact_pii(user_email))
   ```

3. **Configurable retention:** Short retention for logs with potential PII (5 days), longer retention for sanitized logs (90 days).

4. **Encryption:** Log the PII encrypted; decrypt only in authorized systems.

### Common PII to Redact

- Email addresses, phone numbers
- Credit card numbers (PAN), expiration, CVV
- Social security numbers
- API keys, tokens, passwords
- IP addresses (sometimes; depends on jurisdiction)
- User names/identifiers (depends on sensitivity)

### Implementation

Use your log aggregation system's redaction features:
- **ELK:** Use Logstash filters to redact patterns.
- **Loki:** Label values can be redacted at index time.
- **Application-level:** Redact in the logger before emission.

### Pitfall

Never assume logs are private. Developers read logs; they're often searched, shared, or stored in recoverable places. Treat all PII with skepticism.

## Log Aggregation: ELK, Loki, and Alternatives

A single service's logs are manageable. A fleet of 100 services produces 100s of GB of logs per day. A log aggregation system centralizes, indexes, and searches them.

### ELK Stack (Elasticsearch, Logstash, Kibana)

**Elasticsearch:** Full-text search engine; stores and indexes logs.
**Logstash:** Pipeline that ingests logs, transforms, and forwards to Elasticsearch.
**Kibana:** Web UI for querying and visualizing logs.

**Strengths:** Feature-rich, mature, industry standard, excellent for ad-hoc search and dashboards.
**Weaknesses:** Expensive at scale (Elasticsearch consumes significant storage/CPU), requires operational overhead.

### Loki (Prometheus-like for Logs)

Designed by Grafana, purpose-built for log aggregation in Kubernetes environments.

**Model:**
- Labels (service=payment-api, env=prod) are indexed.
- Log lines themselves are not indexed; search is filtered by label first.

**Strengths:** Cheap (log lines compressed, storage efficient), integrates well with Prometheus metrics and Grafana, designed for DevOps.
**Weaknesses:** Not full-text searchable; label-based filtering requires upfront design.

### CloudWatch (AWS), Stackdriver (GCP), Azure Monitor

Cloud vendors' native logging.

**Strengths:** Managed; automatic scaling; integrated with cloud infrastructure.
**Weaknesses:** Vendor lock-in; per-gigabyte ingestion pricing can exceed on-prem solutions at scale.

### Choosing

- **High-volume, complex queries:** ELK.
- **Kubernetes environments, cost-conscious:** Loki.
- **AWS-native workloads:** CloudWatch.
- **Multi-cloud or strong open-source preference:** Loki or ELK.

## Sampling: Reducing Log Volume at Scale

Even with efficient storage, logging every request at every service creates volume problems. Sampling reduces cost without sacrificing visibility for common cases.

### Rate-Based Sampling

Log 10% of requests randomly:

```python
import random
if random.random() < 0.1:
    logger.info("Request handled", trace_id=trace_id)
```

**Drawback:** When investigating a specific user's issue, you have only 10% chance of logs. The bug might not appear in sampled logs.

**Solution:** Always log errors and warnings; sample infoative/debug logs.

```python
if request.status_code >= 400 or random.random() < 0.01:
    logger.info("Request handled", trace_id=trace_id, status=request.status_code)
```

### Adaptive Sampling

Increase sampling rate when error rates are high:

```python
error_rate = recent_errors / total_requests
if error_rate > 0.05:
    sample_rate = 0.5  # Log 50% of requests when errors spike
else:
    sample_rate = 0.01  # Log 1% normally
```

### Dynamic Sampling / Tail-Based Sampling

Log all requests with errors; log a small percentage of successful requests. This captures failures without logging every success:

```python
if trace_errors.contains(trace_id):
    logger.info("Request", trace_id=trace_id)  # Full logging for this trace
else if random.random() < 0.01:
    logger.info("Request", trace_id=trace_id)  # Sample of successes
```

### OpenTelemetry Sampling

OpenTelemetry offers head-based (sampler decides at request start) and tail-based (sampler decides at request end) sampling. Tail-based is more powerful but complex.

## Cost Management

Logging at scale becomes expensive. Strategies to manage costs:

1. **Set retention policies:** Logs older than 90 days are often not needed. Archive to cheaper storage.
2. **Differentiate retention by level:** FATAL/ERROR retained 365 days; WARN retained 90 days; DEBUG retained 7 days.
3. **Compress logs:** JSON is repetitive; compression reduces storage.
4. **Sampling:** As above; reduce volume.
5. **Structured logging:** Smaller JSON than verbose text; more efficient indexing.
6. **Exclude low-value logs:** Stop logging health checks, status requests, and other high-volume, low-signal events.
7. **Use log-level filtering at source:** Don't send DEBUG logs to expensive aggregation; filter at the service level.

### Example Cost Breakdown

For a service processing 1M requests/hour:

- Log every request (1M logs/hour): ~50 GB/day → ~1.5 TB/month @ $0.50/GB stored = **$750/month**.
- Log 1% of requests + all errors (~50K logs/hour): ~2.5 GB/day → ~75 GB/month @ $0.50/GB = **$37.50/month**.
- Same with 90-day retention (vs. 365 days): **$11.25/month**.

Sampling and retention policies are force multipliers on cost.

## Logging Anti-Patterns

**Log spam:** Every function entry/exit, logged warnings that aren't actionable. Results in noise that obscures real events.

**Async log loss:** Logging asynchronously without guarantees. If the process crashes, recent logs are lost. Use at-least-once delivery semantics for critical logs.

**Global mutable state in logs:** Logging request-scoped context in a global variable. In concurrent systems, logs bleed context between requests. Use correlation IDs and structured logging instead.

**Logs as a feature:** "Let's log sensitive data temporarily for debugging." Once logged, it's captured in backups and archives forever.

**Ignoring log configuration:** Production instances using debug logging levels because developers forgot to change the default. Use external configuration, not code defaults.

---

**See also:** observability-logging.md (logging architecture and structured logging), devops-observability-patterns.md (logs + metrics + traces), error-handling-patterns.md (logging errors properly)