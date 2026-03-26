# Webhooks: Event Delivery, Resilience & Security

## Overview

Webhooks enable servers to push events to client-defined URLs instead of clients polling. This note covers delivery semantics (at-least-once, at-most-once, exactly-once challenges), retry strategies with backoff, idempotency patterns, signature verification (HMAC), event ordering, fan-out patterns, and comparison to polling and Server-Sent Events.

## Fundamentals

### What is a Webhook

A webhook is a user-provided URL that the server calls (HTTP POST) when an event occurs:

```
Event occurs (e.g., order paid) → Server calls POST https://client.example.com/webhook
```

vs. polling:

```
Client repeatedly asks: "Any new orders?" → Server responds
```

Webhooks are **push-based** (server to client); polling is **pull-based** (client asks repeatedly).

## Delivery Semantics

### At-Least-Once (Most Common)

The server guarantees that each event is delivered at least once, but may deliver duplicates:

```
Event: OrderCreated #123
Attempt 1: POST https://webhook.example.com/notify
  → 500 error (server down temporarily)
Attempt 2: POST https://webhook.example.com/notify (retry)
  → 200 OK
Attempt 3: POST https://webhook.example.com/notify (retry scheduled before Attempt 2 succeeded)
  → 200 OK (duplicate delivery)
```

**Implication**: Clients must handle idempotency (duplicate events should not cause side effects).

### At-Most-Once

Each event is delivered at most once, but some deliveries may be lost:

```
Attempt 1: POST https://webhook.example.com/notify
  → 500 error
  → Abandoned (not retried)
```

- Fast, simple
- **Dangerous**: Clients miss critical events
- Rarely justified

### Exactly-Once (Impossible in Distributed Systems)

True exactly-once delivery is theoretically impossible without global consensus. The paper "You Cannot Have Exactly-Once Delivery" (Tyler Treat, 2015) explains:

1. If the sender retries forever, the receiver may see duplicates
2. If the sender gives up, the receiver may see loss
3. Both sides must agree on success, which requires round-trips
4. Round-trip failures are indistinguishable from network partitions

**Workaround**: Combine at-least-once delivery with **idempotency keys**.

## Retry Strategies

### Exponential Backoff with Jitter

A robust retry policy prevents thundering herd and resource exhaustion:

```
Attempt 1: immediate
Attempt 2: 2 seconds + random jitter (0–1s)
Attempt 3: 4 seconds + jitter
Attempt 4: 8 seconds + jitter
Attempt 5: 16 seconds + jitter
...
Attempt 10: 512 seconds (8+ minutes) + jitter
```

After N failed attempts (often 24 hours or 10 retries), move to Dead Letter Queue.

```javascript
const maxAttempts = 10;
const baseDelay = 1000;  // 1 second
const maxDelay = 3600000; // 1 hour

function getRetryDelay(attempt) {
  const exponential = Math.min(
    baseDelay * Math.pow(2, attempt),
    maxDelay
  );
  const jitter = Math.random() * exponential * 0.1;  // 10% jitter
  return exponential + jitter;
}
```

### Idempotency Keys

Clients provide a unique key for each webhook delivery attempt, allowing servers to deduplicate:

```json
{
  "event": "order.created",
  "idempotencyKey": "evt_abc123xyz", 
  "orderId": 42,
  "total": 99.99,
  "createdAt": "2026-01-15T10:30:00Z"
}
```

- Idempotency key is **unique per event instance**, not per attempt
- Client always sends the same key for retries
- Server: "If I've seen this key before, return cached result"

Implementation:
```javascript
function handleWebhook(req) {
  const key = req.body.idempotencyKey;
  const cached = cache.get(key);
  
  if (cached) {
    return cached.result;  // Return without re-processing
  }
  
  const result = processEvent(req.body);
  cache.set(key, result);  // Cache for future retries
  return result;
}
```

## HMAC Signature Verification

Most webhook providers authenticate deliveries via HMAC (Hash-Based Message Authentication Code):

### Example: GitHub Webhooks

```javascript
// GitHub sends header: X-Hub-Signature-256: sha256=abcdef...
// Server verifies using webhook secret

const crypto = require('crypto');

function verifyGitHubSignature(secret, body, signature) {
  const hmac = crypto
    .createHmac('sha256', secret)
    .update(body)
    .digest('hex');
  
  return signature === `sha256=${hmac}`;
}

// In webhook handler:
const signature = req.headers['x-hub-signature-256'].replace('sha256=', '');
if (!verifyGitHubSignature(secret, req.rawBody, signature)) {
  return res.status(401).send('Unauthorized');
}
```

### Verification Best Practices

1. **Use timing-safe comparison**: Prevent timing attacks on secret comparison
   ```javascript
   crypto.timingSafeEqual(
     Buffer.from(signature),
     Buffer.from(computed)
   );
   ```

2. **Include timestamp**: Prevent replay attacks
   ```javascript
   // Header: X-Webhook-Timestamp: 1642252800
   const age = Date.now() - parseInt(header['x-webhook-timestamp']) * 1000;
   if (age > 5 * 60 * 1000) {  // 5 minute window
     return res.status(401).send('Timestamp too old');
   }
   ```

3. **Sign the entire request**: Body + timestamp + request path
   ```javascript
   const payload = `${timestamp}.${path}.${body}`;
   const signature = hmac('sha256', payload, secret);
   ```

## Event Ordering

### Challenges

In distributed systems, events may arrive out of order:

```
Event A: order.created (t=1000)
Event B: order.paid (t=1001)
Delivery to client:
  Order.paid arrives first (network delay)
  Order.created arrives second
```

A client processing .paid before .created may fail (no order record exists).

### Solutions

#### 1. Timestamp Ordering

Include creation timestamp; client orders by it:

```json
{
  "event": "order.created",
  "occurredAt": "2026-01-15T10:30:00.000Z",
  "orderId": 42
}
```

Client buffers out-of-order events and processes by timestamp.

#### 2. Causality Tokens (Causal Consistency)

Server includes a token representing causal ordering:

```json
{
  "event": "order.paid",
  "causality": "evt_abc123xyz",  // Must process evt_abc123xyz first
  "orderId": 42
}
```

#### 3. Event Versioning

Include logical version numbers per event stream:

```json
{
  "event": "order.created",
  "sequenceNumber": 5,  // This is the 5th event for this order
  "orderId": 42
}
```

Most systems don't enforce strict ordering; client code handles modest out-of-order arrivals.

## Webhook Management

### Registration & Discovery

APIs typically expose endpoints for clients to register webhooks:

```
POST /webhooks
{
  "url": "https://client.example.com/notify",
  "events": ["order.created", "order.paid"],
  "active": true
}

200 OK
{
  "id": "wh_abc123",
  "url": "https://client.example.com/notify",
  "events": ["order.created", "order.paid"],
  "createdAt": "2026-01-15T10:30:00Z"
}
```

### Event Filtering

Clients subscribe to specific event types to reduce noise:

```json
{
  "url": "https://webhook.example.com/notify",
  "events": ["order.created", "order.paid"]  // Only these
}
```

vs. receiving all events.

### Webhook Testing

- **Echo endpoints** for validation (test before adding to production)
- **Replay capability**: Admin UI to resend past events
- **Event history**: View delivery status (success, failed, pending)
- **Delivery logs**: Read request/response for debugging

Example: Stripe Dashboard shows webhook delivery history per event.

## Fan-Out Patterns

### Single Webhook Per Customer

Simplest: One URL per customer. Payload contains a batch of events:

```json
{
  "events": [
    { "event": "order.created", "orderId": 1 },
    { "event": "order.paid", "orderId": 2 },
    { "event": "order.shipped", "orderId": 3 }
  ]
}
```

Client processes batch. If processing fails, retry entire batch.

### Multiple Specialized Webhooks

Clients register different URLs for different event types:

```
POST /webhooks (orders service)
  → https://client.example.com/orders

POST /webhooks (payments service)
  → https://client.example.com/payments
```

Each service independently retries; allows service-level resilience.

### Fan-Out via Message Queue

For internal systems, publish events to a queue; independent consumers retry:

```
Event occurs → Publish to Kafka → Multiple consumers → Retry independently
```

More scalable for high volume; adds operational complexity.

## Dead Letter Queues (DLQ)

After N retries over 24+ hours fail, move webhook to DLQ:

```
Webhook delivery attempts:
  T=0: Fail (retry in 2s)
  T=2: Fail (retry in 4s)
  T=6: Fail (retry in 8s)
  ...
  T=3600: Fail (max attempts reached)
    → Move to DLQ, alert admin
```

DLQ strategies:
- **Manual retry**: Admin reviews, clicks "retry" in dashboard
- **Automatic replay**: DLQ worker retries periodically (e.g., hourly for 7 days)
- **Alerting**: PagerDuty alert for high-priority events in DLQ

## Webhooks vs. Alternatives

### Webhooks vs. Polling

| Aspect | Webhooks | Polling |
|--------|----------|---------|
| Latency | Low (server-initiated) | High (depends on poll interval) |
| Infrastructure | Requires client HTTP server | Client only needs HTTP client |
| Reliability | At-least-once (requires idempotency) | Can miss events or double-count |
| Complexity | Signature verification, retries | Simpler client-side |
| Scalability | O(1) per client | O(N) queries to server |

**Webhooks are better for real-time, low-latency use cases.**

### Webhooks vs. Server-Sent Events (SSE)

| Aspect | Webhooks | SSE |
|--------|----------|-----|
| Direction | Server pushes to client (outbound) | Server pushes to client (inbound) |
| Connection model | Separate HTTP POST per event | Long-lived TCP connection |
| Firewall-friendly | May require inbound firewall rules | Client initiates (easier) |
| Reliability | Explicit retries | Connection retries (browser built-in) |

**SSE is better for real-time streams from a server to a browser or single client.**

## Webhook Platforms (SaaS)

Services like Svix and Hookdeck abstract webhook infrastructure:

- **Event queuing**: Reliable storage before delivery
- **Retry management**: Exponential backoff, DLQ
- **Monitoring**: Dashboards, alerting
- **Testing**: Replay, echo endpoints
- **SDKs**: Signature verification libraries

Useful for reducing webhook operations burden; trade-off is vendor dependency.

## Best Practices

1. **Always use HMAC signatures** to authenticate
2. **Include timestamps** to prevent replay
3. **Support idempotency keys** (allow clients to deduplicate)
4. **Provide retry visibility** (dashboard showing delivery status)
5. **Document event schema** (OpenAPI/Async API)
6. **Test webhook playback** (provide event samples, replay tools)
7. **Use exponential backoff with jitter** for retries
8. **Implement health checks** (periodic test events to verify webhook is live)
9. **Version events** (add `version` field for schema evolution)
10. **Monitor DLQ** (alert when events accumulate)

## Cross-References

See also: [api-message-queues.md](api-message-queues.md), [web-api-patterns.md](web-api-patterns.md), [architecture-event-driven.md](architecture-event-driven.md)