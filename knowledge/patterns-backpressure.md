# Backpressure Patterns

Backpressure is a mechanism for producers to signal demand constraints to consumers, preventing queue overflow, memory exhaustion, and cascading failures. The fundamental principle: faster components should wait for slower ones, not accumulate data indefinitely.

## Core Concept

In a streaming system, data flows from producer → queue/buffer → consumer. Without backpressure:

```
Producer: emits 1M events/sec
Buffer: capacity 10k items
Consumer: processes 100k events/sec

Buffer fills in 10ms, then overflows
```

With backpressure:

```
Consumer signals: "I can only take 10k events/sec"
Producer throttles to match
System reaches equilibrium
```

Backpressure answers the question: "How does the consumer tell the producer to slow down?"

## Reactive Streams Specification

The Reactive Streams spec (Java, JavaScript, Python, Go) formalizes backpressure as a pull-based demand signal.

### Contract

```
Producer emits events → Subscriber.onNext(event)
Subscriber can handle N items → Subscription.request(N)  // Demand signal
Producer respects demand → never emits more than requested
```

**Subscription object:**

```javascript
interface Subscription {
  request(n)  // "I can handle n more items"
  cancel()    // "Stop sending"
}

interface Subscriber {
  onNext(value)     // Event arrives
  onError(error)    // Terminal signal: error
  onComplete()      // Terminal signal: finished
}
```

**Flow:**

```
Subscriber subscribes to Publisher
  Publisher → Subscription.request(10)  // "Send me 10 items"
  Publisher emits 10 items
  Subscriber processes, then → Subscription.request(10)  // "Give me 10 more"
  Cycle repeats
```

### Example (RxJava)

```java
observable
  .subscribe(new Subscriber<Item>() {
    private Subscription sub;
    private int buffer = 0;
    private final int BATCH_SIZE = 100;
    
    @Override
    public void onSubscribe(Subscription s) {
      this.sub = s;
      s.request(BATCH_SIZE);  // Initial demand
    }
    
    @Override
    public void onNext(Item item) {
      processItem(item);
      buffer--;
      if (buffer < BATCH_SIZE / 2) {
        sub.request(BATCH_SIZE / 2);  // Refill demand
      }
    }
    
    @Override
    public void onError(Throwable e) { ... }
    
    @Override
    public void onComplete() { ... }
  });
```

**Key insight:** Demand is signaled explicitly; producer never emits unsolicited events. The consumer controls the rate.

## Bounded Queues

In imperative (non-reactive) systems, backpressure often manifests as a bounded queue: a buffer with fixed capacity. When full, attempts to enqueue block (or raise an exception).

```java
BlockingQueue<Item> queue = new LinkedBlockingQueue<>(1000);  // Capacity 1000

// Producer
for (Item item : items) {
  queue.put(item);  // Blocks if queue is full (backpressure)
}

// Consumer
while (true) {
  Item item = queue.take();  // Waits if queue is empty
  process(item);
}
```

**Semantics:**
- Producer blocks when queue is full → natural backpressure
- Consumer drains queue at its own pace
- System reaches equilibrium: producer rate ≈ consumer rate

**Tradeoff:** Synchronous blocking threads == thread pool exhaustion if too many producers block.

```
100 producers each with 10k events
800 threads block on queue.put()
Thread pool saturated → no threads available to process events
Deadlock
```

**Solution:** Use timeouts or async/await instead of blocking threads.

```java
if (queue.offer(item, 100, TimeUnit.MILLISECONDS)) {
  // Enqueued
} else {
  // Queue full, timeout — handle backpressure (drop, retry, alert)
  handleBackpressure(item);
}
```

## TCP Flow Control

TCP implements backpressure at the protocol level via the **receive window**.

```
Sender sends data
  ↓
Receiver ACKs with "receive window = X bytes"
  ↓
Sender adjusts send rate: can't send more than X bytes before next ACK
```

**Window-based flow control:**

```
Receiver: "I can buffer 64KB"
  TCP Header: Window Size = 65536

Sender: sends 32KB
Receiver: ACK, Window Size = 32KB (only 32KB left)

Sender: cannot send more until window increases
```

**If receiver is slow:**

```
Receiver accumulates data faster than app processes
Receive buffer fills
TCP advertises window = 0 bytes
Sender must wait (stalls on send())
```

**This is automatic TCP backpressure.** No application code needed; the OS kernel enforces it.

**Implication for application developers:**
- Non-blocking socket APIs (epoll, kqueue, IOCP) + application-level backpressure are needed to handle backpressure from multiple slow clients
- A single slow client can stall the send buffer; **non-blocking I/O is essential**

## Pull-Based Consumption

Contrast with push-based (sender decides when to send). Pull-based systems let the consumer request data.

**Push (producer-driven):**

```
Server collects events, sends to client whenever ready
Client may be overwhelmed
```

**Pull (consumer-driven):**

```
Client: "Give me the next 100 events"
Server: sends 100 events
Client processes, then: "Give me the next 100 events"
```

**Examples:**
- **Kafka:** Consumer controls offset; pulls messages at its pace (consumer.poll())
- **GCS/S3 object listing:** Client specifies max result count; paging cursor-based
- **GraphQL:** Query specifies what to fetch; server doesn't send unsolicited data

**Advantages:**
- Consumer never overloaded (requests exactly what it can handle)
- Consumer drives complexity: sequential vs. parallel requests
- Graceful degradation: slow client just pulls slower

## Load Shedding

When backpressure fails (queue overflows, buffer exhausted), drop requests rather than crash.

**Strategies:**

**Random drop:** Drop incoming requests when queue exceeds threshold.

```java
if (queue.size() > MAX_QUEUE_SIZE) {
  if (random() < DROP_PROBABILITY) {
    return 503 Service Unavailable;  // Drop
  }
}
queue.add(request);
```

**Tail drop:** Always reject new requests when queue is full.

```java
if (queue.size() >= MAX_QUEUE_SIZE) {
  return 503 Service Unavailable;
}
queue.add(request);
```

**Priority drop:** Drop lowest-priority requests; keep high-priority.

```java
if (queue.size() >= MAX_QUEUE_SIZE) {
  Item lowest = findLowestPriority(queue);
  queue.remove(lowest);
}
queue.add(request);
```

**When to apply:** When the system cannot process all incoming requests, dropping some is better than queuing indefinitely or crashing.

## Adaptive Concurrency Limits

Instead of fixed pool size, adjust concurrency dynamically based on latency and error rate.

### Vegas Algorithm (Congestion Control)

From TCP Vegas. Estimate available capacity by comparing expected throughput to observed.

```
throttle_limit = max(1, limit * (1 - alpha * (base_latency - current_latency) / base_latency))
```

- **base_latency:** Latency when no congestion
- **current_latency:** Current observed latency
- **alpha:** Sensitivity tuning parameter

**Intuition:** If latency is rising, `current_latency > base_latency` → reduce `throttle_limit`.

### Gradient Algorithm (Adaptive Concurrency)

Used by Copper by Shopify. Monitors queue depth and latency gradient.

```
if latency_increasing:
  limit -= 1  // Reduce concurrency
elif latency_stable:
  limit += 1  // Increase concurrency
```

### Java Implementation (Coherence library)

```java
ConcurrencyLimit limit = ConcurrencyLimits.aimdConcurrencyLimit()
  .maxConcurrency(100)
  .build();

Semaphore sem = new Semaphore(limit.getLimit());
sem.acquire();  // Acquire permit
try {
  result = executeRequest();
  limit.onSuccess();
} catch (TimeoutException e) {
  limit.onIgnore();
} finally {
  sem.release();
}
```

## Circuit Breaker as Backpressure

A circuit breaker stops calling a failing downstream service, allowing it to recover. This is a form of backpressure: the downstream is signaling "I'm overloaded/broken; stop sending requests."

```
CLOSED: normal requests
  ↓ (too many failures)
OPEN: reject requests immediately (backpressure)
  ↓ (timeout passes)
HALF-OPEN: trial requests to test recovery
  ↓ (success)
CLOSED
```

**Implementation:**

```java
CircuitBreaker breaker = new CircuitBreaker()
  .failureThreshold(5)
  .successThreshold(2)
  .delay(10, TimeUnit.SECONDS);

if (breaker.isOpen()) {
  throw new ServiceUnavailableException();  // Backpressure
}

try {
  response = downstream.call();
  breaker.recordSuccess();
} catch (Exception e) {
  breaker.recordFailure();
  throw e;
}
```

## Akka Streams (Actor Model Backpressure)

Akka Streams combines reactive streams + actor model for backpressure.

```scala
Source(1 to 100)
  .throttle(1, 100.millis)  // 1 element per 100ms
  .to(Sink.foreach(println))
  .run()
```

**Backpressure propagation:**

```
Sink (slow) → Stage → Source (fast)
Sink.request(10) ← request(10) ← Source slows to emit 10
```

Demand flows backward; elements flow forward. Stages autom
atically enforce backpressure.

## Combining Patterns

A production system typically layers multiple backpressure mechanisms:

```
1. Upstream rate limiting (too many requests) → 429
2. Bounded queue in load balancer
3. Adaptive concurrency limit in backend service
4. Circuit breaker to downstream dependency
5. Timeout + retry (with backoff) on failure
```

**Stack up, not silo.** Each layer catches a different failure mode.

## When to Apply

**Reactive Streams (push + pull demand):**
- Real-time data processing (Kafka consumers, WebSockets)
- Streaming APIs where order and responsiveness matter
- Frameworks: RxJS, RxJava, Project Reactor, Akka Streams

**Bounded Queues (implicit backpressure):**
- Job queues (task processing, batch jobs)
- Rate limiting + queueing (message brokers, API gateways)
- Thread pools + blocking calls

**Circuit Breaker (failure-based backpressure):**
- Cross-service communication (microservices)
- External APIs where timeouts occur

**Load Shedding (explicit backpressure rejection):**
- High-load scenarios where no buffering capacity remains
- Graceful degradation: drop requests rather than cascade failures

**Adaptive Limits:**
- Variable workload (spiky traffic, unpredictable latency)
- When fixed concurrency limits are too conservative or too optimistic

## See Also

- `paradigm-reactive-programming.md` — Reactive streams and observables
- `patterns-rate-limiting.md` — Rate limiting and throttling
- `patterns-circuit-breaker.md` — Resilience via circuit breakers
- `architecture-resilience.md` — Resilience patterns broadly