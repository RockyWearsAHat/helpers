# Error Handling in Distributed Systems — Partial Failures, Timeouts & Resilience

## Overview

Distributed systems introduce unique error modes absent in single-machine code: **partial failures** (some nodes succeed, others fail), **unreliable networks**, **asynchronous uncertainty**, and **cascading failures**. Error handling strategies differ fundamentally: detecting failures is hard, recovering from them is complex, and preventing cascade is paramount. Core patterns include timeout distinction, idempotency, compensating transactions, circuit breakers, bulkheads, and dead letter queues.

---

## Partial Failures and Failure Detection

### The Fundamental Problem

In a single process, errors are binary: operation succeeds or fails entirely. In distributed systems, anything can partially fail:

- **Client sends request:** Network timeout (did server receive it?)
- **Server processes:** Server crashes mid-operation (partial state update?)
- **Server responds:** Response is lost in network (client thinks failure, server thinks success?)

**Example:**

```
Transfer $100 from Account A to Account B
├─ Debit from A: SUCCESS
├─ Debit sent to B: NETWORK TIMEOUT
└─ Did B receive the funds? Unknown.

Options:
1. Assume B didn't receive it; retry → potential double-charge
2. Assume B received it; do nothing → potential loss
3. Query B's account → still uncertain (query could be stale)
```

### Distinguishing Timeout from Failure

**Timeout ≠ Failure.** A timeout means "I didn't hear back within T seconds." This could mean:

- Server never received request (real failure)
- Server is processing (slow, not failed)
- Server responded but response was lost (success on server side)
- Network congestion (transient)

**Best practice:** Assume timeout is transient and **retry only if the operation is idempotent** (see below).

```javascript
async function callWithTimeout(fn, timeoutMs = 5000) {
  try {
    return await Promise.race([
      fn(),
      new Promise((_, reject) =>
        setTimeout(() => reject(new Error('Timeout')), timeoutMs)
      )
    ]);
  } catch (e) {
    if (e.message === 'Timeout') {
      // Don't know if it succeeded; only retry if idempotent
      throw new UncertainError('Request may have succeeded or failed');
    }
    throw e;
  }
}
```

### Failure Detection

How do you know a dependency is down?

**Health checks:** Periodic ping to a dependency

```javascript
async function isHealthy(service) {
  try {
    const response = await fetch(`${service}/health`, { timeout: 2000 });
    return response.ok;
  } catch {
    return false;
  }
}

setInterval(async () => {
  const healthy = await isHealthy('https://payment-service');
  if (!healthy) {
    console.warn('Payment service appears down');
    // Trigger circuit breaker, notify ops
  }
}, 10000);
```

**Heartbeat:** Service actively reports "still alive"

```javascript
// Payment service sends heartbeat every 5 seconds
setInterval(() => {
  redis.setex('service:payment:heartbeat', 10, JSON.stringify({
    timestamp: Date.now(),
    version: '1.2.3'
  }));
}, 5000);

// Monitor checks for stale heartbeat
setInterval(() => {
  const lastBeat = redis.get('service:payment:heartbeat');
  if (!lastBeat || Date.now() - JSON.parse(lastBeat).timestamp > 15000) {
    console.warn('Payment service missed heartbeat');
  }
}, 3000);
```

**Liveness vs. Readiness:**

- **Liveness:** Is the service still running? (restart if false)
- **Readiness:** Can the service handle requests? (remove from LB if false, but don't restart)

```javascript
app.get('/healthz', (req, res) => {
  // Liveness: is the process alive?
  if (process.memoryUsage().heapUsed > MAX_MEMORY) {
    return res.status(500).json({ status: 'dying' });
  }
  res.json({ status: 'alive' });
});

app.get('/ready', (req, res) => {
  // Readiness: can we handle requests?
  if (!database.isConnected) {
    return res.status(503).json({ status: 'not_ready' });
  }
  res.json({ status: 'ready' });
});
```

---

## Idempotency for Safe Retries

### The Core Idea

An operation is **idempotent** if calling it multiple times has the same effect as calling it once.

**Idempotent:**
- `SET user:1 => { name: 'Alice' }` (same result regardless of how many times)
- `DELETE user:1` (idempotent in terms of outcome)
- `GET /users/1` (read operations are always idempotent)

**Not idempotent:**
- `POST /transfers { from: 1, to: 2, amount: 100 }` (each call transfers $100)
- `UPDATE balance += 100` (each call adds $100)

### Implementation: Idempotency Keys

To make non-idempotent operations retryable, use an **idempotency key**:

```javascript
// Client generates a unique key per logical operation
const idempotencyKey = uuid.v4();

async function transfer(from, to, amount) {
  // Retry this multiple times with same key
  const result = await api.post('/transfers', 
    { from, to, amount },
    { headers: { 'Idempotency-Key': idempotencyKey } }
  );
  return result;
}
```

**Server-side:**

```javascript
app.post('/transfers', async (req, res) => {
  const { idempotencyKey } = req.headers['idempotency-key'];
  
  // Check if we've already processed this key
  const cached = await cache.get(`idempotent:${idempotencyKey}`);
  if (cached) {
    // Return cached result—don't process again
    return res.json(cached);
  }
  
  // Process the transfer
  const result = await processTransfer(req.body);
  
  // Cache the result for future requests with same key
  await cache.setex(`idempotent:${idempotencyKey}`, 24 * 3600, result);
  
  res.json(result);
});
```

**Key lifetime:** Store for at most 24 hours (idempotency keys must still be in cache if client retries).

---

## Compensating Transactions (Saga Pattern)

### The Problem

Multi-step distributed operations can fail partway through, leaving partial state:

```
Reserve hotel → Book flight → Process payment
   ✓              ✓             ✗ (card declined)

Now: hotel is reserved, flight is booked, but payment failed.
Need to undo both.
```

### Choreography vs. Orchestration

**Choreography:** Services listen to events and trigger compensations

```javascript
// Event: Payment failed
eventBus.on('payment_failed', async (reservation) => {
  await hotelService.cancel(reservation.hotelId);
  await flightService.cancel(reservation.flightId);
});
```

**Cons:** Complex event chains, hard to debug, cyclical compensation loops.

**Orchestration:** Central coordinator manages the saga

```javascript
class ReservationSaga {
  async execute(booking) {
    try {
      const hotel = await hotelService.reserve(booking);
      this.compensations.push(() => hotelService.cancel(hotel.id));
      
      const flight = await flightService.book(booking);
      this.compensations.push(() => flightService.cancel(flight.id));
      
      await paymentService.charge(booking.payment);
      
      return { hotel, flight };
    } catch (error) {
      // Compensate in reverse order
      for (const compensate of this.compensations.reverse()) {
        await compensate();
      }
      throw error;
    }
  }
}
```

### Partial Compensation Failure

What if compensation itself fails?

```javascript
// Payment fails, so we compensate:
try {
  await hotelService.cancel(hotel.id);  // ✓
  await flightService.cancel(flight.id);  // ✗ (network error)
} catch (e) {
  // Now: hotel canceled, flight still booked
  // We're in an inconsistent state
}
```

**Solutions:**

1. **Retry compensation:** Use exponential backoff + dead letter queue
2. **Manual intervention:** Log failure, alert ops
3. **Idempotent compensation:** Ensure canceling twice is safe

```javascript
async function compensate(booking) {
  try {
    await hotelService.cancel(booking.hotelId);
  } catch (error) {
    // Queue for retry
    deadLetterQueue.push({
      action: 'compensate_hotel',
      booking,
      error
    });
    throw error;
  }
}
```

---

## Circuit Breaker Integration

### Preventing Cascades

A slow or failing dependency should not take down the entire system. **Circuit breaker** stops calling a failing service, failing fast instead:

```
CLOSED (normal)
  └─ failures > threshold → OPEN
OPEN (fail-fast)
  └─ timeout expires → HALF-OPEN
HALF-OPEN (testing)
  └─ success → CLOSED
  └─ failure → OPEN
```

### Implementation

```javascript
class CircuitBreaker {
  constructor(fn, { failureThreshold = 5, timeout = 60000 } = {}) {
    this.fn = fn;
    this.failureThreshold = failureThreshold;
    this.timeout = timeout;
    this.failureCount = 0;
    this.state = 'CLOSED';
    this.lastFailureTime = null;
  }

  async call(...args) {
    if (this.state === 'OPEN') {
      if (Date.now() - this.lastFailureTime > this.timeout) {
        this.state = 'HALF-OPEN';
      } else {
        throw new Error('Circuit breaker is OPEN');
      }
    }

    try {
      const result = await this.fn(...args);
      
      if (this.state === 'HALF-OPEN') {
        this.state = 'CLOSED';
        this.failureCount = 0;
      }
      
      return result;
    } catch (error) {
      this.failureCount++;
      this.lastFailureTime = Date.now();
      
      if (this.failureCount >= this.failureThreshold) {
        this.state = 'OPEN';
      }
      
      throw error;
    }
  }
}

const paymentBreaker = new CircuitBreaker(paymentService.charge);

// Auto-fails fast if breaker is open
try {
  await paymentBreaker.call(order);
} catch (e) {
  if (e.message === 'Circuit breaker is OPEN') {
    // Handle gracefully: offer offline mode, queued payment, etc.
    return { status: 'retry_later' };
  }
  throw e;
}
```

**Tuning:**

- **Failure threshold:** Too low → flaps open/closed. Too high → slow to detect failures.
- **Timeout:** Too short → rapid retries. Too long → slow recovery.
- **Half-open requests:** Allow 2-3 test requests, not just 1.

---

## Bulkhead Isolation

### Preventing Resource Exhaustion

One failing dependency should not exhaust shared resources (threads, connections) and take down other services.

**Thread pool per service:**

```javascript
class ServiceClient {
  constructor(name, options = {}) {
    this.pool = new pLimit(options.concurrency || 10);  // isolate
    this.name = name;
  }

  async call(fn) {
    // Request queues up to 10 concurrent, rest wait
    // If this service hangs, only its pool is exhausted
    return this.pool(() => fn());
  }
}

const paymentClient = new ServiceClient('payment', { concurrency: 5 });
const shippingClient = new ServiceClient('shipping', { concurrency: 10 });

// Payment hangs → only blocks payment requests, not shipping
Promise.all([
  paymentClient.call(() => chargeCard()),
  shippingClient.call(() => arrangeShipping())
]);
```

---

## Dead Letter Queues (DLQ)

### Handling Poison Messages

Message brokers (Kafka, RabbitMQ, SQS) need to handle messages that can't be processed:

```
Message Consumer
  │
  ├─ Success: ack message (remove from queue)
  ├─ Transient error: nack, retry later (message goes back to queue)
  └─ Permanent error (poison message): stuck in retry loop forever
```

**Dead letter queue** routes poison messages to a separate queue for manual inspection and replay:

```javascript
async function processMessage(message) {
  try {
    validate(message);  // throws if invalid format
    await handleBusinessLogic(message);
    consumer.ack(message);  // success
  } catch (error) {
    if (isPermanentError(error)) {
      // Move to DLQ
      deadLetterQueue.push({
        originalMessage: message,
        error: error.message,
        timestamp: Date.now()
      });
      consumer.ack(message);  // remove from main queue
    } else {
      // Transient error: retry
      consumer.nack(message);
      setTimeout(() => consumer.requeue(message), 5000);
    }
  }
}

function isPermanentError(error) {
  // Schema validation, type errors = permanent
  // Network timeouts, service down = transient
  return error instanceof ValidationError;
}
```

### Replaying DLQ

Once root cause is fixed, replay messages:

```javascript
async function replayDLQ() {
  const messages = await deadLetterQueue.getAll();
  
  for (const item of messages) {
    try {
      await processMessage(item.originalMessage);
      console.log(`DLQ replay successful: ${item.originalMessage.id}`);
    } catch (error) {
      console.error(`DLQ replay failed: ${error.message}`);
      // Leave in DLQ or move to failed replay queue
    }
  }
}
```

### Preventing Poison in First Place

- **Validation:** Strict schema validation before processing
- **Versioning:** Handle old message formats gracefully
- **Graceful degradation:** Warn instead of fail for unexpected fields

---

## Graceful Degradation

### Feature Flags for Cascading Failures

Disable non-critical features when dependencies fail:

```javascript
async function getProductPage(productId) {
  const product = await productService.get(productId);  // critical
  
  // Optional features—degrade if unavailable
  let reviews = [];
  if (featureFlags.isEnabled('show_reviews')) {
    try {
      reviews = await reviewService.get(productId);
    } catch (e) {
      console.warn('Review service down, hiding reviews');
    }
  }
  
  let recommendations = [];
  if (featureFlags.isEnabled('show_recommendations')) {
    try {
      recommendations = await recommendationService.get(productId);
    } catch (e) {
      console.warn('Recommendation service down, hiding section');
    }
  }
  
  return { product, reviews, recommendations };
}
```

### Cache Fallback

Serve stale data rather than fail:

```javascript
async function getUser(userId, options = {}) {
  try {
    const fresh = await userService.get(userId);
    await cache.set(`user:${userId}`, fresh, { ttl: 3600 });
    return fresh;
  } catch (error) {
    if (!options.allowStale) throw error;
    
    // Try cache
    const cached = await cache.get(`user:${userId}`);
    if (cached) {
      console.warn('Serving stale user data from cache');
      return cached;
    }
    
    throw error;
  }
}

// Usage: fallback to stale data on error
const user = await getUser(123, { allowStale: true });
```

---

## Timeouts at Every Layer

Cascading timeouts amplify failure. Set timeouts at each layer:

```javascript
// Client timeout
const response = await fetch('/api/data', { timeout: 5000 });

// In handler: validate upstream still has time
app.get('/api/data', async (req, res, next) => {
  if (req.deadline < Date.now()) {
    return res.status(408).json({ error: 'Request timeout' });
  }
  
  // Db call with remaining time
  const remaining = req.deadline - Date.now();
  const data = await database.query(sql, { timeout: remaining - 500 });
  
  res.json(data);
});
```

---

## Best Practices

1. **Assume partial failure:** Every RPC can fail, succeed with partial data, or timeout.
2. **Distinguish timeout from failure:** Don't assume timeout = permanent failure.
3. **Make it idempotent:** Use idempotency keys for retryable non-idempotent operations.
4. **Circuit break early:** Fail fast when a dependency is down.
5. **Isolate with bulkheads:** Limit concurrency per dependency.
6. **Use DLQs:** Poison messages shouldn't block healthy traffic.
7. **Degrade gracefully:** Disable non-critical features, serve stale data.
8. **Timeout consistently:** Set timeouts at every layer, accounting for propagation.
9. **Compensate on failure:** Multi-step sagas need reversal logic.
10. **Monitor failures:** Distinguish transient from permanent errors when alerting.

---

## See Also

- [Error Handling Language Patterns](error-handling-language-patterns.md)
- [Resilience Patterns: Circuit Breaker](patterns-circuit-breaker.md)
- [Distributed Messaging Systems](distributed-messaging.md)
- [Distributed Systems Theory](math-distributed-theory.md)
- [System Design & Distributed Systems](system-design-distributed.md)