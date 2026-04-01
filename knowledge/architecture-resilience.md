# Resilience Patterns

## Circuit Breaker

Prevents cascading failures by stopping calls to a failing dependency. Three states:

```
CLOSED ──(failures > threshold)──→ OPEN ──(timeout expires)──→ HALF-OPEN
  ↑                                                              │
  └─────────────────(success)──────────────────────────────────←─┘
  │                                                              │
  └─────────────────────────────────←─(failure)──────────────────┘
                                    (back to OPEN)
```

| State         | Behavior                                                                |
| ------------- | ----------------------------------------------------------------------- |
| **Closed**    | Requests pass through. Failures counted. Trips open when threshold hit. |
| **Open**      | Requests fail immediately (no call to dependency). Timer starts.        |
| **Half-Open** | Limited requests pass through. Success → Closed. Failure → Open.        |

### Configuration Parameters

| Parameter              | Typical Value     | Purpose                                |
| ---------------------- | ----------------- | -------------------------------------- |
| Failure threshold      | 5 failures in 60s | When to trip open                      |
| Open duration          | 30-60s            | How long to stay open before testing   |
| Half-open permits      | 1-3 requests      | How many test requests in half-open    |
| Failure rate threshold | 50%               | Percentage-based (over sliding window) |
| Slow call threshold    | 5s                | Calls exceeding this count as failures |

### Implementations

| Library      | Language | Notes                               |
| ------------ | -------- | ----------------------------------- |
| Resilience4j | Java     | Modern, lightweight, functional API |
| Polly        | .NET     | Policy-based, composable            |
| Hystrix      | Java     | Netflix, deprecated but influential |
| opossum      | Node.js  | Simple circuit breaker for Node     |
| pybreaker    | Python   | Basic circuit breaker               |

```java
// Resilience4j
CircuitBreakerConfig config = CircuitBreakerConfig.custom()
    .failureRateThreshold(50)
    .waitDurationInOpenState(Duration.ofSeconds(30))
    .slidingWindowSize(10)
    .minimumNumberOfCalls(5)
    .build();

CircuitBreaker breaker = CircuitBreaker.of("paymentService", config);
Supplier<PaymentResult> decorated = CircuitBreaker
    .decorateSupplier(breaker, () -> paymentService.charge(amount));
```

## Retry

Automatically retry failed operations. Critical: the operation must be **idempotent** or you risk duplicate side effects.

### Exponential Backoff with Jitter

```python
import random, time

def retry_with_backoff(fn, max_retries=5, base_delay=1.0):
    for attempt in range(max_retries):
        try:
            return fn()
        except TransientError:
            if attempt == max_retries - 1:
                raise
            delay = base_delay * (2 ** attempt)
            jitter = random.uniform(0, delay * 0.5)
            time.sleep(delay + jitter)
```

**Why jitter**: Without jitter, retries from multiple clients synchronize (thundering herd). Jitter spreads them out.

### Retry Strategies

| Strategy             | Formula                              | Best For                                       |
| -------------------- | ------------------------------------ | ---------------------------------------------- |
| Fixed delay          | `delay`                              | Simple cases, known recovery time              |
| Exponential backoff  | `base * 2^attempt`                   | Unknown recovery time, avoid overwhelming      |
| Exponential + jitter | `base * 2^attempt + random(0, half)` | Distributed systems (prevents thundering herd) |
| Linear backoff       | `base * attempt`                     | Gradual increase                               |

### What to Retry

| Retry                   | Don't Retry            |
| ----------------------- | ---------------------- |
| 503 Service Unavailable | 400 Bad Request        |
| 429 Too Many Requests   | 401 Unauthorized       |
| Connection timeout      | 404 Not Found          |
| DNS resolution failure  | 409 Conflict (usually) |
| Network partition       | Business logic errors  |

## Timeout

Every external call needs a timeout. No exceptions.

| Type                | What It Limits                   | Typical Value |
| ------------------- | -------------------------------- | ------------- |
| Connection timeout  | Time to establish TCP connection | 1-5s          |
| Read/socket timeout | Time to receive response data    | 5-30s         |
| Request timeout     | Total time including retries     | 30-60s        |
| Idle timeout        | Connection pool idle time        | 30-300s       |

```python
# Python requests — always set both timeouts
response = requests.get(url, timeout=(3.05, 27))
#                                    ^^^^  ^^
#                              connect  read

# Risky — no timeout:
response = requests.get(url)  # Default timeout = None (waits forever)
```

**Cascading timeout budget**: If Service A → B → C, and A has a 10s timeout, B should have <10s total (e.g., 8s), and C should have <8s (e.g., 5s). Each hop leaves room for processing and retries.

## Bulkhead

Isolate failures so one struggling dependency doesn't consume all resources.

### Thread Pool Isolation

Each dependency gets its own thread pool. If the payment service is slow, it exhausts its pool but leaves inventory service unaffected.

```
┌─────────────────────────────────────┐
│ Application                         │
│  ┌──────────────┐ ┌──────────────┐  │
│  │ Payment Pool │ │ Inventory    │  │
│  │ (10 threads) │ │ Pool (20)    │  │
│  │ ████████░░   │ │ ██░░░░░░░░░  │  │
│  └──────────────┘ └──────────────┘  │
└─────────────────────────────────────┘
```

### Semaphore Isolation

Lighter weight: limit concurrent calls via semaphore count. No thread pool overhead but no queuing.

## Rate Limiting

Protect services from being overwhelmed. Applied at API gateways, load balancers, or service level.

### Algorithms

| Algorithm                  | How It Works                                      | Pros                            | Cons                                      |
| -------------------------- | ------------------------------------------------- | ------------------------------- | ----------------------------------------- |
| **Token bucket**           | Tokens added at fixed rate; request costs a token | Allows bursts up to bucket size | Burst can temporarily exceed rate         |
| **Leaky bucket**           | Requests queued, processed at fixed rate          | Smooth output rate              | No bursting, queue overflow               |
| **Fixed window**           | Count requests per time window (e.g., per minute) | Simple                          | Boundary problem: 2x burst at window edge |
| **Sliding window log**     | Track timestamp of each request                   | Precise                         | Memory-intensive (stores every timestamp) |
| **Sliding window counter** | Weighted average of current and previous window   | Good balance                    | Slight approximation                      |

### Token Bucket Implementation

```python
class TokenBucket:
    def __init__(self, rate: float, capacity: int):
        self.rate = rate          # Tokens per second
        self.capacity = capacity  # Max burst size
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

### HTTP Rate Limit Headers

```
HTTP/1.1 429 Too Many Requests
Retry-After: 30
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1706000000
```

## Load Shedding

When overloaded, reject requests early rather than accepting them and failing slowly. Strategies:

- **Random early drop**: Start dropping a percentage of requests as load increases
- **Priority-based**: Shed low-priority requests first (monitoring, batch, non-critical)
- **LIFO queue**: Newest requests get served first; oldest dropped (they've likely already timed out on the client)

## Backpressure

Signal to producers to slow down when consumers can't keep up:

| Mechanism               | How                                            |
| ----------------------- | ---------------------------------------------- |
| Reactive streams        | Publisher respects subscriber's demand signals |
| TCP flow control        | Receiver window advertises buffer space        |
| Queue depth limits      | Reject/block new messages when queue is full   |
| HTTP 429                | Server tells client to back off                |
| Consumer lag monitoring | Alert when Kafka consumer falls behind         |

## Health Checks

### Kubernetes Probes

| Probe         | Purpose                            | Failure Action                     |
| ------------- | ---------------------------------- | ---------------------------------- |
| **Liveness**  | Is the process alive?              | Kill and restart pod               |
| **Readiness** | Can the process serve traffic?     | Remove from load balancer          |
| **Startup**   | Has the process finished starting? | Waits before liveness checks begin |

```yaml
livenessProbe:
  httpGet:
    path: /health/live
    port: 8080
  periodSeconds: 10
  failureThreshold: 3

readinessProbe:
  httpGet:
    path: /health/ready
    port: 8080
  periodSeconds: 5
  failureThreshold: 2

startupProbe:
  httpGet:
    path: /health/startup
    port: 8080
  periodSeconds: 5
  failureThreshold: 30 # 30 * 5s = 150s to start
```

### Health Check Design

**Liveness**: Minimal — is the process responsive? Don't check dependencies (a database outage shouldn't restart your pods).

**Readiness**: Check critical dependencies — database connection pool, required caches, configuration loaded.

**Deep health**: Separate endpoint for monitoring. Check everything: DB, cache, message broker, downstream services. Never use for Kubernetes probes.

## Graceful Degradation

When a component fails, provide reduced but functional service:

| Component Down        | Degradation Strategy                    |
| --------------------- | --------------------------------------- |
| Recommendation engine | Show popular/recent items instead       |
| Search service        | Fall back to category browsing          |
| Payment processor     | Queue orders for later processing       |
| CDN                   | Serve from origin (slower)              |
| Analytics             | Drop telemetry silently                 |
| Cache                 | Read from database (slower but correct) |

## Chaos Engineering

### Principles

1. Define steady state (normal behavior metrics)
2. Hypothesize that steady state continues during a failure
3. Introduce real-world failures
4. Observe the difference
5. Fix what broke

### Tools

| Tool         | Scope       | Key Feature                              |
| ------------ | ----------- | ---------------------------------------- |
| Chaos Monkey | VM/Instance | Random instance termination              |
| Litmus       | Kubernetes  | CRD-based chaos experiments              |
| Gremlin      | Platform    | Enterprise chaos-as-a-service            |
| Toxiproxy    | Network     | TCP-level latency, blackhole, bandwidth  |
| Chaos Mesh   | Kubernetes  | Pod, network, I/O, time, JVM chaos       |
| AWS FIS      | AWS         | Managed fault injection on AWS resources |

### Experiment Example

```yaml
# Litmus ChaosEngine: Kill random pods in payment namespace
apiVersion: litmuschaos.io/v1alpha1
kind: ChaosEngine
metadata:
  name: payment-pod-kill
spec:
  appinfo:
    appns: payment
    applabel: "app=payment-service"
  chaosServiceAccount: litmus-admin
  experiments:
    - name: pod-delete
      spec:
        components:
          env:
            - name: TOTAL_CHAOS_DURATION
              value: "30"
            - name: CHAOS_INTERVAL
              value: "10"
            - name: FORCE
              value: "false"
```

### What to Test

- Instance/pod failure (is recovery automatic?)
- Network partition between services (does the circuit breaker work?)
- Dependency latency injection (do timeouts fire?)
- DNS failure (does service discovery fall back?)
- Disk full / high CPU (does load shedding engage?)
- Clock skew (do distributed timestamps break?)

**Start in staging. Graduate to production only with safeguards and kill switches.**
