# Idempotency Patterns

Idempotency means an operation produces the same result regardless of how many times it's executed. In distributed systems, idempotence is the primary defense against duplicate requests caused by retries, network failures, and exactly-once delivery promises.

## Core Concept

**Non-idempotent:** `POST /accounts/42/balance/add?amount=100` — each request increments the balance. Retries cause overcharges.

**Idempotent:** `POST /transfers` with `Idempotency-Key: uuid-123` — calling five times with the same key produces one transfer and four duplicates rejected as already-processed.

The difference is captured in request metadata (the idempotency key), not HTTP method alone. PUT and DELETE are inherently idempotent; POST is not—unless guarded by a key.

## Idempotency Keys

A client-generated unique identifier (typically a UUID) that identifies a logical operation. The server stores (key → result) and returns the cached result on duplicate requests.

**Protocol:**
```
Request 1:
  POST /transfers
  Idempotency-Key: 123e4567-e89b-12d3-a456-426614174000
  { "amount": 100, "to": "user-456" }
  → 200 OK, Transfer created

Request 2 (duplicate, same key):
  POST /transfers
  Idempotency-Key: 123e4567-e89b-12d3-a456-426614174000
  { "amount": 100, "to": "user-456" }
  → 200 OK, Same transfer returned (cached)
```

**Key generation:** Client responsibility. UUID v4 (random) is standard; UUID v1 (timestamp-based) adds sequencing info.

**Storage:** Server-side. Typically a table `(key, result, expiry)`.

```sql
CREATE TABLE idempotency_cache (
  key VARCHAR(36) PRIMARY KEY,
  operation_id VARCHAR(36),
  result_json TEXT,
  status_code INT,
  created_at TIMESTAMP DEFAULT NOW(),
  expires_at TIMESTAMP
);

-- On duplicate request, query cache
SELECT result_json, status_code FROM idempotency_cache WHERE key = ?;
```

**Expiry:** Keys must expire. Common: 24 hours for API transactions, shorter for in-process operations. Expiry prevents unbounded cache growth.

**Gotchas:**
- If key expires but the original request is still in-flight, a second attempt might execute the operation twice
- If the cache is lost (server restart, database corruption), duplicates bypass the check
- Different clients using the same key is a bug; educate clients or use namespace prefixes (e.g., `<client-id>-<random>`)

## Stripe-Style Idempotency

Stripe's implementation is the industry standard for payment processors.

**Pattern:**
1. Compute a **request signature** from request body (hash all parameters)
2. Store `(key, signature, result)` 
3. On duplicate key:
   - If signature matches: return cached result
   - If signature differs: reject with 400 Bad Request (client error)

**Rationale:** Prevents accidental replay with different parameters. If a client generates a new UUID but the network retried the old request, you catch it.

```sql
-- Table schema (Stripe pattern)
CREATE TABLE idempotency_results (
  key VARCHAR(36),
  request_signature VARCHAR(64),
  result_json TEXT,
  status_code INT,
  created_at TIMESTAMP DEFAULT NOW(),
  PRIMARY KEY (key)
);

-- On request
signature = hash(request.body)
cached = SELECT result_json, signature FROM idempotency_results WHERE key = ?;

if cached:
  if cached.signature != signature:
    return 400 Bad Request ("Idempotency key re-used with different request")
  else:
    return cached.result_json
else:
  result = process_request(request)
  INSERT INTO idempotency_results (key, signature, result_json, status_code) VALUES (...)
  return result
```

## At-Least-Once Delivery + Deduplication

In event-driven systems, producers guarantee **at-least-once** delivery (events may arrive multiple times). Consumers must deduplicate.

**Pattern: Deduplication by Event ID**

Events carry an immutable `event_id`. Consumers track processed event IDs and skip duplicates.

```sql
-- Consumer-side dedup table
CREATE TABLE processed_events (
  event_id VARCHAR(36) PRIMARY KEY,
  event_type VARCHAR(100),
  processed_at TIMESTAMP DEFAULT NOW()
);

-- Consumer logic
event = receive_event()
if EXISTS (SELECT 1 FROM processed_events WHERE event_id = event.id):
  skip (duplicate)
else:
  INSERT INTO processed_events (event_id) VALUES (event.id)
  process_event(event)
```

**Trade-off:** Requires persistence. If the consumer crashes after processing but before recording event_id, it may reprocess on restart. Use transactions to atomically record ID and side effect.

```sql
BEGIN TRANSACTION
  INSERT INTO processed_events (event_id) VALUES (...)
  UPDATE accounts SET balance = balance + event.amount WHERE account_id = event.account_id
COMMIT
```

## Database Constraints

**Unique constraint idempotency:** Let the database enforce uniqueness.

**Pattern:**

```sql
CREATE TABLE pending_transfers (
  transfer_id UUID PRIMARY KEY,
  user_id INT,
  amount DECIMAL,
  destination VARCHAR(255),
  CONSTRAINT unique_pending UNIQUE (user_id, destination, amount) -- not realistic, just example
);

-- Client generates transfer_id (idempotency key)
-- Client always inserts with the same transfer_id
-- First insert succeeds; duplicates violate UNIQUE constraint → application layer catches error
INSERT INTO pending_transfers (transfer_id, user_id, amount, destination) 
VALUES ('uuid-123', 42, 100, 'user-456')
  ON CONFLICT DO NOTHING
  RETURNING *;
```

**Limitation:** Only works if the operation itself is naturally unique (e.g., "transfer $100 to user X" is unique by (user_id, destination, amount)). Most operations don't have a natural uniqueness.

## Conditional Requests (ETags, If-Match)

For **read-modify-write** patterns, prevent lost updates using conditional headers.

**ETag (Entity Tag):** Server sends a hash of the current resource state. Client sends it back in `If-Match` header.

```
GET /orders/123
  → 200 OK
  { "id": 123, "status": "pending", "amount": 100 }
  ETag: "abc123" (hash of resource state)

PUT /orders/123
  If-Match: "abc123"
  { "status": "confirmed" }
  → If server's current ETag != "abc123", return 412 Precondition Failed
  → Otherwise, update and return new ETag
```

**Semantics:** "Only update if the resource hasn't changed since I last read it."

**Pattern: Distributed counter**

```
GET /counters/page-views
  → ETag: "v5" (version 5)

PUT /counters/page-views
  If-Match: "v5"
  { "value": 1001 }
  → If ETag matches, increment and return new ETag ("v6")
  → If ETag doesn't match, return 412 (someone else updated first)
```

**Limitation:** Doesn't guarantee exactly-once (just prevents overwrites). Must retry on 412.

## Message Deduplication in Queues

Message brokers (Kafka, RabbitMQ, AWS SQS) provide idempotency semantics.

**Kafka:**
- Producer idempotence: `enable.idempotence = true` — Kafka tracks (producer_id, sequence_number) and rejects duplicates
- Consumer idempotence: Consumer tracks offsets; on restart, resume from last committed offset

**RabbitMQ:**
- publisher-confirms: Producer waits for broker ACK before considering message sent
- Consumer manual acknowledgments: Consumer ACKs only after processing

**Pattern: Dedup in consumer**

```python
async def consume():
  processed_ids = set()  # or Redis set
  async for message in queue:
    if message.id in processed_ids:
      continue
    processed_ids.add(message.id)
    await process(message)
    await queue.ack(message)
```

## UPSERT Patterns

**UPSERT (UPDATE or INSERT)** naturally idempotent: calling multiple times with the same data always results in the same state.

```sql
INSERT INTO users (id, name, email) VALUES (123, 'Alice', 'alice@example.com')
  ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, email = EXCLUDED.email;
```

**Semantics:** "Ensure user 123 has name `Alice` and email `alice@example.com`." Calling five times produces the same result.

**Limitation:** UPSERT doesn't solve distributed retry idempotency for **operations** (transfers, payments, orders). It works for **state** (user profile, configuration).

## Timeout and Dangling Operations

A critical edge case: A request starts executing but times out before returning a result. The client retries with a new request. Now you have two in-flight operations.

**Solution:** Store idempotency key **before** starting operation.

```python
def transfer(key, source, dest, amount):
  if idempotency_cache.get(key):
    return idempotency_cache.get(key)
  
  # Mark as in-progress
  idempotency_cache.set(key, status="pending")
  
  try:
    result = execute_transfer(source, dest, amount)
    idempotency_cache.set(key, result)
    return result
  except Exception as e:
    idempotency_cache.set(key, status="error", error=str(e))
    raise
```

**Retry logic then checks for `status="in-progress"` and waits:**

```
if cache[key].status == "pending":
  wait_with_backoff()  # Wait for first attempt to complete
  return cache[key].result
```

## Design Trade-offs

| Approach | Storage | Accuracy | Overhead |
| --- | --- | --- | --- |
| Idempotency Key | Persistent table | High (unless key deleted) | 2x DB writes |
| Unique Constraint | Database constraint | Very high (database enforces) | Works only for natural uniqueness |
| ETag + Condition | Server-side version | Medium (doesn't prevent retries, just lost updates) | One extra header |
| Event Dedup (ID log) | Event tracker table | High (if atomically updated with effect) | Scales with event volume |
| UPSERT | None (database native) | Perfect for state | Not applicable to operations |

## When and Where

**APIs:**
- **Payment processing:** Always use idempotency keys (required for PCI compliance level)
- **Order creation:** Idempotency keys prevent duplicate charges
- **Account updates:** Unique constraints or ETags sufficient

**Async systems:**
- **Event consumers:** Track event IDs; dedup at consumption
- **Message processors:** Use broker-native idempotence (Kafka producer idempotence, RabbitMQ confirms)
- **Distributed transactions:** Coordinate via saga pattern + idempotent steps

**HTTP clients:**
- **GET/DELETE:** Inherently idempotent (repeating is safe)
- **POST:** Use idempotency keys if the operation is not naturally idempotent
- **PUT:** Use ETags if updating existing resources to prevent lost updates

## See Also

- `api-design.md` — API design principles
- `api-webhooks.md` — Event delivery idempotency
- `patterns-circuit-breaker.md` — Retry strategies
- `architecture-event-driven.md` — Event-driven patterns