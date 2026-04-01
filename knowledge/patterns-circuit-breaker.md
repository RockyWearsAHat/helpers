# Resilience Patterns: Circuit Breaker & Friends

Resilience patterns protect systems from cascading failures when dependencies degrade or fail. A single slow or dead service should not take down the entire system. Common patterns work together: circuit breakers stop calling failing dependencies, retries recover from transient faults, timeouts prevent hanging, bulkheads isolate failures, and rate limiting protects from overload.

## Circuit Breaker

Prevents cascading failures by stopping calls to a failing dependency, allowing it time to recover.

### States

```
CLOSED ──(failures > threshold)──→ OPEN ──(timeout)──→ HALF-OPEN
  ↑                                                          │
  └─────────────────(success)───────────────────────────────┘
  │                                                          │
  └──────────────────────(failure, back to OPEN)────────────┘
```

| State         | Behavior                                              |
| ------------- | ----------------------------------------------------- |
| **CLOSED**    | Normal state. Requests pass through. Failures counted. When threshold exceeded, trip to OPEN. |
| **OPEN**      | Failing mode. New requests fail immediately (fail-fast) without calling the service. A timer runs. When timer expires, advance to HALF-OPEN. |
| **HALF-OPEN** | Testing mode. Allow a limited number of trial requests. If they succeed, reset to CLOSED. If any fail, return to OPEN and restart timer. |

### Configuration

| Parameter                | Typical Value     | Purpose                                      |
| ------------------------ | ----------------- | -------------------------------------------- |
| **Failure threshold**     | 5 failures in 60s | Trip open when this many failures occur      |
| **Failure rate %**        | 50%               | Trip when failure rate exceeds this in window |
| **Open duration**         | 30-60s            | How long to stay open before trying HALF-OPEN |
| **Half-open permits**     | 1-3 requests      | How many requests to allow in HALF-OPEN      |
| **Slow call threshold**   | 5s                | Calls exceeding this time count as failures  |

### Java: Resilience4j

```java
CircuitBreakerConfig config = CircuitBreakerConfig.custom()
    .failureRateThreshold(50)           // 50% failure rate triggers
    .waitDurationInOpenState(Duration.ofSeconds(30))
    .slidingWindowSize(10)               // Last 10 calls
    .minimumNumberOfCalls(5)             // Need 5 calls to evaluate
    .slowCallRateThreshold(80)           // 80% of calls > 2s = fail
    .slowCallDurationThreshold(Duration.ofSeconds(2))
    .recordExceptions(IOException.class, TimeoutException.class)
    .ignoreExceptions(ValidationException.class)
    .build();

CircuitBreaker breaker = CircuitBreaker.of("paymentService", config);

Supplier<PaymentResult> decorated = CircuitBreaker
    .decorateSupplier(breaker, () -> paymentService.charge(order));

try {
    PaymentResult result = decorated.get();
} catch (CallNotPermittedException e) {
    log.warn("Payment service is open; skipping charge");
    // Fallback logic
}
```

### .NET: Polly

```csharp
var circuitBreakerPolicy = Policy
    .Handle<HttpRequestException>()
    .Or<TimeoutRejectedException>()
    .OrResult<HttpResponseMessage>(r => !r.IsSuccessStatusCode)
    .CircuitBreakerAsync(
        handledEventsAllowedBeforeBreaking: 3,
        durationOfBreak: TimeSpan.FromSeconds(30),
        onBreak: (outcome, duration) =>
        {
            Console.WriteLine($"Circuit open for {duration}");
        },
        onReset: () =>
        {
            Console.WriteLine("Circuit reset to closed");
        }
    );

var result = await circuitBreakerPolicy.ExecuteAsync(
    () => httpClient.GetAsync("https://api.external.com/data")
);
```

### Legacy: Hystrix

```java
// Hystrix is deprecated in favor of Resilience4j.
// It introduced circuit breakers to mainstream Java.
@HystrixCommand(
    fallbackMethod = "getDefaultValue",
    commandProperties = {
        @HystrixProperty(name = "execution.isolation.thread.timeoutInMilliseconds", value = "5000"),
        @HystrixProperty(name = "circuitBreaker.errorThresholdPercentage", value = "50")
    }
)
public String getValue() {
    return externalService.fetch();
}

public String getDefaultValue() {
    return "default";
}
```

## Retry with Backoff

Transient faults (network glitch, temporary overload) often recover quickly. Retry the operation, but don't retry immediately — exponential backoff spreads out retries and avoids overwhelming the service.

### Exponential Backoff with Jitter

```python
import random, time

def retry_with_backoff(fn, max_retries=5, base_delay=1.0):
    for attempt in range(max_retries):
        try:
            return fn()
        except TransientError:
            if attempt == max_retries - 1:
                raise  # Last attempt, propagate error
            
            # Exponential: 1s, 2s, 4s, 8s, 16s
            delay = base_delay * (2 ** attempt)
            
            # Jitter: 0-50% of delay, prevents thundering herd
            jitter = random.uniform(0, delay * 0.5)
            
            time.sleep(delay + jitter)
```

**Why jitter:** Without it, all clients retry at the same time, creating a "thundering herd" that overwhelms a recovering service. Jitter spreads retries uniformly.

### What to Retry vs. Don't Retry

| Retry                              | Don't Retry                   |
| ---------------------------------- | ----------------------------- |
| 503 Service Unavailable            | 400 Bad Request               |
| 429 Too Many Requests (rate limit) | 401/403 Authorization errors  |
| Connection timeout / refused       | 404 Not Found                 |
| DNS resolution failure             | 409 Conflict (usually)        |
| Network partition                  | Business logic errors         |
| 500 Internal Server Error (maybe)  | **User errors**               |

**Idempotency is crucial.** Only retry if the operation is idempotent or you risk duplicate side effects (e.g., charging a credit card twice).

### Cascade Timeout Budgets

If Service A calls B calls C, timeouts must cascade:

```
Service A: 10s total timeout
  ├─ Service B: 8s total timeout (room for A's processing + retries)
  │   ├─ Service C: 5s total timeout (room for B's processing + retries)
  │   └─ C returns at 3s
  │   ├─ B processes at 2s
  │   └─ B returns at 5s
  └─ A processes at 2s
  └─ A returns at 7s (within budget)
```

Each service reserves timeout for its own processing and retries.

## Bulkhead

Isolate failures so one struggling dependency doesn't consume all resources.

### Thread Pool Isolation

Each dependency gets its own thread pool:

```
Application
├─ Payment Service Pool (10 threads)
│  └─ 8 threads in use, 2 free
├─ Inventory Service Pool (20 threads)
│  └─ 5 threads in use, 15 free
└─ Shipping Service Pool (5 threads)
   └─ 5 threads in use (maxed out)
```

If Shipping is slow, its thread pool fills and local requests queue, but Payment and Inventory pools remain responsive.

```java
// Resilience4j Bulkhead
BulkheadConfig config = BulkheadConfig.custom()
    .maxConcurrentCalls(10)
    .maxWaitDuration(Duration.ofMillis(100))
    .build();

Bulkhead bulkhead = Bulkhead.of("paymentService", config);

Supplier<Result> decorated = Bulkhead
    .decorateSupplier(bulkhead, () -> paymentService.process());
```

### Semaphore Isolation

Lighter-weight than thread pools. Limit concurrent calls via a semaphore:

```java
BulkheadConfig config = BulkheadConfig.custom()
    .maxConcurrentCalls(20)
    .build();

// No thread pool overhead, but no queuing either.
// If 20 concurrent calls are active, the 21st gets rejected immediately.
```

**Trade-off:** Semaphores use less memory but don't queue backpressure. Requests are rejected rather than waiting.

## Timeout

Every external call must have a timeout. Waiting forever is a form of failure.

```
C library call
├─ Connection timeout (1-5s) — time to establish TCP connection
├─ Read/socket timeout (5-30s) — time to receive response data
├─ Request timeout (30-60s) — total time including retries
└─ Idle timeout (30-300s) — connection pool idle duration
```

```python
# Python requests: always set both
response = requests.get(url, timeout=(3.05, 27))
#                                 conn  read
# Connection: 3.05s, Read: 27s

# Dangerous: no timeout
response = requests.get(url)  # Can hang forever
```

## Fallback

When a call fails, provide a degraded but functional response.

```java
Supplier<OrderStatus> decorated = CircuitBreaker
    .decorateSupplier(breaker, () -> orderService.getStatus());

try {
    return decorated.get();
} catch (CallNotPermittedException e) {
    // Circuit open; fallback to cached value or default
    return fallback.getLastKnownStatus();
}
```

**Fallback strategies:**
- Return cached data (stale but available)
- Return a sensible default (generic rather than specific)
- Queue request for later processing (async)
- Return error immediately (fail fast)

## Hedged Requests

When response time matters, send the same request to multiple replicas and return the first to complete. The slowest requests are simply canceled.

```
Client sends request to Backend-A
  (waits for hedgingDelay)
  If no response after hedgingDelay:
    Client also sends request to Backend-B
  (waits for first to complete)
  Client gets result from whichever replies first
  Cancels the slower request
```

### Configuration

```csharp
// Polly hedging
var hedgingPolicy = Policy
    .Bulkhead(10)  // Max 10 concurrent calls
    .Hedging(
        maxParallelAttempts: 2,  // Send to 2 replicas
        delay: TimeSpan.FromMilliseconds(100)
    );
```

**Trade-off:** Reduces tail latency (p95, p99) at the cost of extra load on backends.

## Rate Limiting

Protect services from being overwhelmed. Applied at API gateways, load balancers, or service level.

### Token Bucket Algorithm

Tokens are added to a bucket at a fixed rate. Each request costs a token. If no tokens available, reject.

```python
class TokenBucket:
    def __init__(self, rate: float, capacity: int):
        self.rate = rate               # Tokens/second
        self.capacity = capacity       # Max burst
        self.tokens = capacity
        self.last_refill = time.monotonic()
    
    def allow(self) -> bool:
        now = time.monotonic()
        elapsed = now - self.last_refill
        self.tokens = min(self.capacity, self.tokens + elapsed * self.rate)
        self.last_refill = now
        
        if self.tokens >= 1:
            self.tokens -= 1
            return True
        return False
```

Allows bursts up to `capacity`, then smooths to `rate` requests/second.

### HTTP Rate Limit Headers

```
HTTP/1.1 429 Too Many Requests
Retry-After: 30
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1706000000  // Unix timestamp
```

## Load Shedding

When overloaded, reject requests early rather than queueing them. Serves fewer requests successfully instead of making all requests fail slowly.

**Strategies:**
- **Random early drop:** As load increases, drop a percentage of incoming requests. Better to reject 10% early than to serve 100% slowly.
- **Priority-based:** Shed low-priority requests first (monitoring, batch jobs). Keep critical requests.
- **Queue depth:** If queue depth exceeds threshold, reject new requests.

```python
if request_queue.size() > MAX_QUEUE_DEPTH:
    return HttpResponse(503, "Service Overloaded")
```

## Combining Patterns

These patterns compose:

```
CircuitBreaker(
  Retry(
    Timeout(
      Bulkhead(
        call_to_dependency()
      )
    ),
    max_retries=3
  ),
  failure_threshold=5
)
```

**Order matters:**
1. **Bulkhead** — outermost, limits concurrency
2. **Timeout** — ensures no hanging
3. **Retry** — handles transients
4. **Circuit Breaker** — outer safety catch

Example: Bulkhead limits to 10 concurrent calls. For each call, timeout is 5s. If it fails transiently, retry 3 times. If permanent failure detected, circuit breaker opens.

## Library Comparison

| Library | Language | Notes |
| --- | --- | --- |
| **Resilience4j** | Java | Modern, functional API, low overhead. Recommended for new projects. |
| **Polly** | .NET | Policy-based, composable, first-class .NET support. |
| **Hystrix** | Java | Netflix, deprecated but influential. Replaced by Resilience4j. Still used in legacy systems. |
| **opossum** | Node.js | Lightweight circuit breaker for Node. |
| **pybreaker** | Python | Basic circuit breaker. Lightweight. |
| **service-mesh (Istrix, Envoy)** | Polyglot | Operationally manage resilience patterns at infrastructure level. Decouples from application code. |

## See Also

- **architecture-resilience** — Broader resilience patterns including health checks, chaos engineering, graceful degradation
- **devops-service-mesh** — Implementing resilience at the network layer  
- **patterns-event-driven** — Handling failures in event-driven systems