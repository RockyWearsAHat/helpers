# Retry Patterns — Immediate vs. Exponential Backoff, Jitter, Budgets, and Amplification Risk

## Overview

**Retry patterns** structure how clients reattend failed requests to achieve availability in face of transient failures. A well-designed retry strategy must balance availability (recover from temporary outages) against amplification risk (converting a single failed request into a thundering herd). The core trade-offs involve timing (when to retry), scope (which failures), idempotency (safety), and feedback loops (circuit breakers, budgets).

## Core Strategies

### Immediate Retry (No Delay)

Retrying without delay immediately after failure is viable **only** in specific scenarios:
- **High-frequency, low-latency operations** (e.g., in-process cache lookups, DNS queries to a local resolver). The cost of waiting often exceeds the cost of retry.
- **Transient network blips** where the service recovers in microseconds. If a TCP packet is lost, immediate retry may succeed before timeout.
- **Lock contention** in databases; spinning or immediate re-attempting a transaction can sometimes succeed before backoff overhead.

**Risk**: Immediate retry without coordination—especially at scale—can amplify a momentary service overload into a total outage. If 10,000 clients each retry immediately and the service recovers at 100 req/sec, immediate retries will requeue 10,000 requests almost simultaneously, re-overloading the service. This is the retry storm danger.

### Exponential Backoff

Exponential backoff delays retry attempts, with each successive attempt waiting longer:

```
Attempt 1: immediate
Attempt 2: wait 2 seconds, then retry
Attempt 3: wait 4 seconds, then retry
Attempt 4: wait 8 seconds, then retry
Attempt 5: wait 16 seconds, then retry
```

**Formula**:
$$\text{delay} = \text{base} \times 2^{\text{attempt}}$$

**Parameters**:
- **base**: Initial delay (e.g., 1 second). Too small risks early storms; too large delays recovery unnecessarily.
- **max_delay**: Cap (e.g., 30 seconds). Prevents delays from growing indefinitely.
- **attempts**: Maximum retries (e.g., 5–10). Balance between persistence and eventual failure signaling.

**Advantage**: Spacing requests across time allows failing services time to recover and shed load. If a service is overloaded, backing off buys it breathing room.

**Problem**: Pure exponential backoff with uniformly timed retries across many clients causes **synchronized retry waves**. If 10,000 clients all back off for 2 seconds, then 4 seconds, the instant the 5-second window closes, all 10,000 retry simultaneously, creating a synchronized spike. This is still a retry storm.

### Jitter: Full and Decorrelated

**Jitter** is random variation added to backoff timing, desynchronizing retry attempts across clients. Two main approaches:

#### Full Jitter

Add random noise uniformly across the backoff window:

```
delay = random(0, base × 2^attempt)
```

Example:
```
Attempt 2: delay = random(0, 4) seconds = 2.3 seconds
Attempt 3: delay = random(0, 8) seconds = 5.7 seconds
Attempt 4: delay = random(0, 16) seconds = 11.2 seconds
```

**Advantage**: Retries spread evenly across the window, preventing thundering herds.

**Trade-off**: Average delay is high. With full jitter over range [0, 16], the expected delay is 8 seconds, compared to exactly 8 without jitter. This delays recovery slightly but ensures safety.

#### Decorrelated Jitter (AWS Strategy)

Decorrelated jitter improves on full jitter by correlating successive delays, keeping backoff growing but with noise:

```
delay = min(cap, random(base, previous_delay × 3))
```

Example (base=1, cap=32):
```
Attempt 1: delay = random(1, 1) = 1
Attempt 2: delay = random(1, 3) = 2.1
Attempt 3: delay = random(1, 6.3) = 4.5
Attempt 4: delay = random(1, 13.5) = 10.2
Attempt 5: delay = random(1, 30.6) = 25.1
```

**Advantage**: Grows exponentially on average (preserving backoff safety) but with jitter to desynchronize requests. Used by AWS SDK retries.

**Explanation**: The `previous_delay × 3` ensures growth, while `random()` prevents synchronization. This balances latency (doesn't go to full jitter's slowness) with safety (eliminates herd risk).

## Idempotency: The Absolutely Critical Requirement

**Retries are only safe if requests are idempotent**—repeating a request has the same effect as issuing it once.

### Idempotent Operations
- **GET** (reading data)
- **DELETE** followed by retry (if the resource is already deleted, retry succeeds with 404; idempotent)
- **PUT** with versioning/ETags (reruns to the same state)
- **Transactional commands** with deduplication (side-effect IDs)

### Non-Idempotent Operations
- **POST** (creates a new resource each time)
- **PATCH** (relative updates; "increment counter" ran twice increments twice)
- **Mutating operations without deduplication**

**Pattern**: Use **idempotent request IDs**. Include a unique identifier (UUID) in each request. The server stores processed IDs; duplicate requests are rejected or return the cached result. This allows retry without duplication.

```
POST /transfer
{
  "from": "account-A",
  "to": "account-B",
  "amount": 100,
  "idempotency_key": "txn-12345-retry"  // Unique per logical operation
}
```

On retry with the same `idempotency_key`, the server returns the same result without re-executing the transfer.

## Retry Budgets: Limiting Retry Amplification

A **retry budget** caps the total retry traffic as a ratio or absolute quota, preventing cascading retries from overwhelming the system.

### Quota-Based Retry Budget

Reserve a budget (e.g., 20,000 retries/second on a 100,000 req/sec system = 20% overhead):
- Track retry count globally (or per service, per client).
- When the quota is exhausted, stop retrying; fail the request immediately.
- Replenish at a steady rate (e.g., 20,000 retries/sec replenish continuously).

**Effect**: Prevents a cascading retry storm from consuming all capacity. If a single service fails and triggers 50,000 retries, the budget caps it at 20,000, leaving 80,000 capacity for new requests and other retries.

### Ratio-Based Retry Budget

Allow retries only up to a ratio of the original request rate:
- For every 100 original requests, allow up to 10 retries.
- Track per-client or per-service.

**Trade-off**: Limits amplification but sacrifices some availability under load spikes.

## Circuit Breaker Integration

Retries should be wrapped with a **circuit breaker** to fail fast when a service is down, not persisting through all retry attempts.

### Pattern

```
try {
  call_service()  // Attempt request
} catch exception {
  if circuit_breaker.is_open() {
    fail immediately  // Service is definitely down; don't retry
  } else {
    retry_with_exponential_backoff()
  }
}
```

### States

1. **CLOSED**: Service is healthy. Retries proceed normally.
2. **OPEN**: Service is failing. Skip retries; fail immediately after circuit opens (e.g., after 5 consecutive failures).
3. **HALF_OPEN**: Testing recovery. Allow one request through; if it succeeds, close the circuit; if it fails, reopen.

**Benefit**: Prevents wasted retry attempts on a service that's known to be failing. Saves latency and resources.

## Retry Storms and Amplification

### What Causes Retry Storms?

1. **Synchronized retry timing**: All clients back off for 2 seconds, then retry together. Impact: a single service failure triggers a 10x spike.
2. **Cascading retries**: Service A fails, retries to Service B. Service B is overloaded by retries, fails, retries to Service C. Impact: amplifies outage across dependencies.
3. **No circuit breaker**: Retries persist even though the service is clearly down, wasting resources.
4. **No retry budget**: Infinite retry loops exhaust all system capacity.

### Mitigation

- **Jitter**: Desynchronize retry timing.
- **Exponential backoff**: Space attempts; reduce load on recovering services.
- **Circuit breaker**: Fail fast when a service is down.
- **Retry budgets**: Cap total retry traffic.
- **Max attempt limits**: Fail after N retries, even if still CLOSED.
- **Timeout diversity**: Don't let all clients use identical timeouts; vary them to avoid synchronized waves.

## Idempotent Consumer Pattern

In event-driven systems, a **retry-safe consumer** must handle duplicate messages (from broker replay on failure):

```
try {
  process_message(event_id)
  mark_as_processed(event_id)  // Transactional with processing
} catch exception {
  if already_processed(event_id) {
    skip  // Idempotent; safe to retry
  } else {
    rethrow
  }
}
```

**Example**: Kafka consumer reads an event, processes it, commits offset. If the consumer crashes before committing, the next consumer replays the same message. If processing is idempotent (e.g., "increment counter"), replay is safe.

## Practical Implementations

### Resilience4j (Java)

```java
RetryConfig config = RetryConfig.custom()
    .maxAttempts(3)
    .waitDuration(Duration.ofMillis(1000))
    .intervalFunction(IntervalFunction.ofExponentialBackoff(1000, 2))
    .build();

Retry retry = Retry.of("name", config);
Supplier<String> supplier = Retry.decorateSupplier(retry, () -> callService());
```

### AWS SDK (Automatic)

AWS SDKs embed retry logic with decorrelated jitter:
- Default: 3 retries on throttling / transient failures.
- Uses `previous_delay × random(0, 3)` for spacing.

### HTTP Client Libraries

Most HTTP clients expose retry configuration:
- **Python `requests`**: Use `retry` adapter with backoff multiplier.
- **Go `http`**: Manual loop with `time.Sleep()` and backoff.

## Trade-offs Summary

| Strategy | Latency | Amplification Risk | Implementation |
|----------|---------|-------------------|-----------------|
| Immediate retry | Lowest | Very high | Avoid unless >1000 req/sec, <10ms ops |
| Exponential backoff | Moderate | High (synchronized waves) | Use with jitter |
| Full jitter | High | Very low | Simple but slow |
| Decorrelated jitter | Moderate-low | Low | AWS standard; recommended |
| + circuit breaker | Lower (fail fast) | Low | Essential for distributed systems |
| + retry budget | Moderate | Very low | Critical for multi-service cascades |

## See Also

- [Resilience Patterns: Circuit Breaker & Friends](patterns-circuit-breaker.md)
- [Architecture Patterns: Resilience](architecture-resilience.md)
- [SRE Load Management & Backpressure](sre-load-management.md)
- [Patterns: Rate Limiting](patterns-rate-limiting.md)