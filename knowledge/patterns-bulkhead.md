# Bulkhead Pattern — Resource Isolation and Failure Containment

## Overview

The **bulkhead pattern** isolates resources (threads, connections, memory) into separate pools so that a resource leak or cascading failure in one part of the system doesn't exhaust all resources and starve other parts.

Named after watertight compartments in ships (bulkheads), which contain a leak to one compartment instead of sinking the whole vessel, the pattern applies the same principle to applications: partition critical resources and let one partition fail without affecting others.

---

## The Core Problem

### Cascading Resource Exhaustion

Without isolation, a single misbehaving component can starve the entire system:

```
Single Thread Pool (no bulkheads):
  ├─ Request 1 → Slow downstream service (hangs 30s) → Thread blocked
  ├─ Request 2 → Slow downstream service (hangs 30s) → Thread blocked
  ├─ Request 3 → Slow downstream service (hangs 30s) → Thread blocked
  └─ ... Threads exhaust → All requests timeout, even fast ones
  
Result: Entire system becomes unresponsive
```

With bulkheads:

```
Bulkhead 1 (Fast endpoints): Thread Pool A (10 threads)
Bulkhead 2 (Slow service): Thread Pool B (5 threads)

Request to fast endpoint → Uses Pool A (unaffected by Pool B)
Request to slow service → Uses Pool B (limited to 5 concurrent, doesn't block fast endpoints)
```

---

## Thread Pool Isolation

### Dedicated Pools Per Service/Endpoint

Assign a thread pool to each critical resource or external service:

```java
ExecutorService fastEndpointPool = Executors.newFixedThreadPool(50);
ExecutorService paymentServicePool = Executors.newFixedThreadPool(10);
ExecutorService analyticsPool = Executors.newFixedThreadPool(5);

// Fast endpoint uses isolated pool
fastEndpointPool.execute(() -> handleFastRequest());

// Payment service uses separate pool, can't starve fast endpoint
paymentServicePool.execute(() -> callPaymentService());

// Analytics uses minimal resources, failures don't matter
analyticsPool.execute(() -> logAnalytics());
```

### Pool Sizing

**Per-resource sizing based on workload characteristics:**

```
Fast endpoint (median 50ms, SLA 200ms):
  Pool size = optimal_threads_per_core * cores * (1 + wait_time / compute_time)
            = 2 * 8 * (1 + 0.05/0.050)
            = 32 threads

Slow service (median 500ms, SLA 5s, target concurrency 20):
  Pool size = target_concurrency * (median_latency / target_SLA)
            = 20 * (0.5 / 5)
            = 2 threads (let it queue instead of creating many)

External cache lookups (median 2ms, high concurrency):
  Pool size = cpu_cores * 2 = 16 threads
```

### Queue Sizing

Pair thread pools with bounded queues:

```java
BlockingQueue<Runnable> queue = new LinkedBlockingQueue<>(100);
ExecutorService pool = new ThreadPoolExecutor(
    10,                    // core threads
    10,                    // max threads
    60, TimeUnit.SECONDS,  // idle timeout
    queue,
    new ThreadPoolExecutor.AbortPolicy()  // Fail fast if queue is full
);
```

**Rejection policy:** When queue is full, reject new tasks instead of queuing indefinitely:

```java
try {
    pool.execute(task);
} catch (RejectedExecutionException e) {
    logger.error("Bulkhead exhausted; rejecting task");
    return Response.status(503).entity("Service unavailable").build();
}
```

---

## Connection Pool Isolation

### Database Connection Pools

Assign connection pools per critical database or query type:

```java
// High-priority queries (analytics dashboard)
HikariDataSource criticalPool = new HikariDataSource();
criticalPool.setMaximumPoolSize(20);
criticalPool.setConnectionTimeout(2000);

// Standard queries (user profile, orders)
HikariDataSource standardPool = new HikariDataSource();
standardPool.setMaximumPoolSize(50);
standardPool.setConnectionTimeout(5000);

// Low-priority queries (background jobs)
HikariDataSource batchPool = new HikariDataSource();
batchPool.setMaximumPoolSize(10);
batchPool.setConnectionTimeout(30000);

// Use based on priority
if (query.isPriority()) {
    try (Connection conn = criticalPool.getConnection()) {
        // Execute query
    }
}
```

### HTTP Client Connection Pools

Isolate HTTP connections per downstream service:

```java
// Payment service: strict timeouts, small pool (critical)
CloseableHttpClient paymentClient = HttpClientBuilder.create()
    .setMaxConnTotal(5)
    .setMaxConnPerRoute(5)
    .setConnectionTimeToLive(30, TimeUnit.SECONDS)
    .build();

// Analytics service: lenient, small pool (non-critical)
CloseableHttpClient analyticsClient = HttpClientBuilder.create()
    .setMaxConnTotal(10)
    .setMaxConnPerRoute(2)
    .setConnectionTimeToLive(60, TimeUnit.SECONDS)
    .build();
```

---

## Semaphore-Based Isolation

### Limiting Concurrent Operations

Use semaphores to limit how many requests can concurrently access a resource:

```java
Semaphore paymentSemaphore = new Semaphore(5);  // Max 5 concurrent payments

public Response processPayment(PaymentRequest req) {
    if (!paymentSemaphore.tryAcquire()) {
        return Response.status(503).entity("Payment service at capacity").build();
    }
    
    try {
        // Call payment service (max 5 concurrent calls)
        return callPaymentService(req);
    } finally {
        paymentSemaphore.release();
    }
}
```

**Advantages over dedicated thread pools:**

- Works across request-handling threads (not confined to a pool)
- Can be applied to any operation, not just thread-based work
- Lightweight (no thread creation overhead)

**Disadvantages:**

- Doesn't isolate threads; a slow request still blocks the handling thread
- Requires explicit try/finally or try-with-resources to release

---

## Hystrix-Style Bulkheads

### NetFlix Hystrix Pattern

Hystrix popularized a combined bulkhead + circuit breaker pattern:

```java
HystrixCommand<Response> paymentCommand = new HystrixCommand<Response>(
    HystrixCommandGroupKey.Factory.asKey("PaymentService"),
    HystrixThreadPoolKey.Factory.asKey("PaymentPool")) {
    
    @Override
    protected Response run() throws Exception {
        // Isolated thread
        return callPaymentService();
    }
    
    @Override
    protected Response getFallback() {
        // Fallback if payment service fails or thread pool exhausted
        return Response.status(503).entity("Payment service unavailable").build();
    }
};

// Executes in isolated thread pool; falls back if queue full or timeout
Response response = paymentCommand.execute();
```

**Hystrix features:**

- **Thread pool isolation:** Separate thread pool per command
- **Fallback:** Execute fallback logic if the command fails or times out
- **Metrics:** Track successes, failures, rejections, latencies
- **Circuit breaker:** Open circuit (fail-fast) after threshold failures

**Metrics example:**

```
PaymentService command:
  Success: 9,850
  Failure: 12
  Rejection (queue full): 5
  Timeout: 3
  Success rate: 99.78%
  Mean latency: 45ms
  p99 latency: 120ms
```

---

## Resilience4j Bulkheads

### Modern Alternative

Resilience4j provides lightweight bulkhead implementations:

```java
import io.github.resilience4j.bulkhead.Bulkhead;
import io.github.resilience4j.bulkhead.BulkheadConfig;
import io.github.resilience4j.bulkhead.BulkheadRegistry;

// Configure bulkhead
BulkheadConfig config = BulkheadConfig.custom()
    .maxConcurrentCalls(10)
    .maxWaitDuration(Duration.ofSeconds(2))
    .build();

BulkheadRegistry registry = BulkheadRegistry.of(config);
Bulkhead bulkhead = registry.bulkhead("paymentService", config);

// Use bulkhead
Supplier<Response> supplier = () -> callPaymentService();
Supplier<Response> bulkheaded = Bulkhead.decorateSupplier(bulkhead, supplier);

Response response = bulkheaded.get();  // Rejected if > 10 concurrent calls
```

**Configuration options:**

```properties
# application.properties
resilience4j.bulkhead.instances.paymentService.maxConcurrentCalls=10
resilience4j.bulkhead.instances.paymentService.maxWaitDuration=2s

resilience4j.bulkhead.instances.analyticsService.maxConcurrentCalls=5
resilience4j.bulkhead.instances.analyticsService.maxWaitDuration=5s
```

**Events and metrics:**

```java
bulkhead.getEventPublisher()
    .onCallRejected(event -> logger.warn("Request rejected: " + event))
    .onCallFinished(event -> logger.info("Request finished: " + event));

BulkheadMetrics metrics = BulkheadMetrics.ofBulkheadRegistry(registry);
MeterRegistry meterRegistry = new SimpleMeterRegistry();
metrics.bindTo(meterRegistry);
```

---

## Sidecar-Based Isolation

### Service Mesh Bulkheads

In service mesh architectures (Istio, Linkerd), bulkheads are configured outside the application:

```yaml
# Istio DestinationRule
apiVersion: networking.istio.io/v1beta1
kind: DestinationRule
metadata:
  name: payment-service
spec:
  host: payment-service
  trafficPolicy:
    connectionPool:
      tcp:
        maxConnections: 20
      http:
        http1MaxPendingRequests: 100
        http2MaxRequests: 1000
        maxRequestsPerConnection: 2
    outlierDetection:
      consecutive5xxErrors: 5
      interval: 30s
      baseEjectionTime: 30s
```

**Benefits:**

- No application code changes
- Centralized policy management
- Language-agnostic (works for any service)
- Integrated with observability (metrics, tracing)

**Drawback:** Requires service mesh infrastructure (added complexity, operational overhead).

---

## Prioritized Request Handling

### Priority Queues

Process high-priority requests before low-priority ones:

```java
class PriorityQueuedPool {
    PriorityBlockingQueue<Task> queue = new PriorityBlockingQueue<>();
    
    public void submit(Task task) {
        queue.offer(task);  // Inserted in priority order
    }
}

class Task implements Comparable<Task> {
    public enum Priority { HIGH, STANDARD, LOW }
    Priority priority;
    
    @Override
    public int compareTo(Task other) {
        // HIGH priority (lower int) processes first
        return Integer.compare(this.priority.ordinal(), other.priority.ordinal());
    }
}

// Submit tasks with different priorities
pool.submit(new Task(Priority.HIGH, () -> handleCriticalPayment()));
pool.submit(new Task(Priority.STANDARD, () -> handleUserRequest()));
pool.submit(new Task(Priority.LOW, () -> runBackgroundJob()));
```

**Result:** During high load, critical tasks (HIGH priority) are processed before background jobs (LOW priority), even if background job was queued first.

---

## Bulkhead Sizing Guidelines

### Determining Optimal Size

**Formula:**

```
pool_size = (target_concurrency) * (p99_latency / target_SLA) + buffer
```

**Example: Payment Service**

```
Target concurrency: 30 concurrent requests (SLA allows)
p99 latency: 200ms
Target SLA: 2000ms (2 seconds)

pool_size = 30 * (0.2 / 2) + 5 buffer
          = 3 + 5
          = 8 threads

Interpretation:
  To handle 30 concurrent callers (average 200ms each),
  we need ~8 threads plus 5 extra for outlier requests.
```

**Validation by load test:**

```
1. Set pool size to calculated value
2. Generate synthetic load (100, 200, 300 RPS)
3. Measure:
   - Success rate (should be > 99.9%)
   - Latency p99 (should be < SLA)
   - Queue depth (should stay bounded)
4. Adjust pool size if metrics bad
```

### Monitoring and Alerts

```
Metrics to monitor:

1. Active threads: Should see variation (not flat)
2. Queue depth: Should rarely exceed pool size
3. Rejections: Should be 0 in normal operation (traffic spike = expected)
4. Wait time: Should stay under 100ms percentile

Alerts:

- IF rejections > 0.1% for > 5 minutes → Page oncall
- IF queue depth > pool_size * 2 for > 1 minute → Page oncall
- IF active threads == max threads for > 10 minutes → Page oncall
```

---

## Anti-Patterns

### Too Many Bulkheads

Creating a bulkhead per request handler leads to micro-sizing and operational complexity:

```java
// Anti-pattern: Too granular
Bulkhead getUser = new Bulkhead(5);
Bulkhead getOrders = new Bulkhead(5);
Bulkhead getProfile = new Bulkhead(5);
Bulkhead updateProfile = new Bulkhead(5);
// → 4 separate pools, hard to reason about, wasted resources
```

**Better:** Group related operations into 1-2 bulkheads.

```java
// Better: Group by service
Bulkhead userServiceBulkhead = new Bulkhead(20);  // reads + writes
```

### Ignoring Queue Size

Setting max threads without bounded queue leads to memory leaks:

```java
// Anti-pattern: Unbounded queue
ExecutorService pool = Executors.newFixedThreadPool(10);
// Default queue size: Integer.MAX_VALUE
// Result: If requests queue, memory bloats

// Better: Bounded queue
BlockingQueue<Runnable> queue = new LinkedBlockingQueue<>(100);
ExecutorService pool = new ThreadPoolExecutor(
    10, 10, 60, TimeUnit.SECONDS, queue,
    new ThreadPoolExecutor.AbortPolicy()
);
```

---

## See Also

- [patterns-circuit-breaker.md](patterns-circuit-breaker.md) — Circuit breaker pattern (often paired with bulkheads)
- [concurrency-patterns.md](concurrency-patterns.md) — Thread pool and concurrent queue patterns
- [sre-load-management.md](sre-load-management.md) — Load shedding and overload protection