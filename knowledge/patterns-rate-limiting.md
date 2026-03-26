# Rate Limiting Patterns

Rate limiting prevents resource exhaustion by restricting request volume. The challenge is balancing three concerns: accuracy (does the limit reflect true load?), efficiency (how much overhead to track?), and fairness (do all clients see consistent behavior?).

## Algorithms

### Token Bucket

The most widely used algorithm. Imagine a bucket that holds up to `capacity` tokens. Tokens refill at `rate` tokens per second. Each request costs one token; if tokens exist, deduct and allow; otherwise, reject.

**Dynamics:**
- Allows bursts: A client with an empty bucket can suddenly consume up to `capacity` tokens (burst allowance)
- Sustains steady load: After the burst, available tokens refill at `rate`

**Formula:**
```
tokens_available = min(capacity, tokens_available + rate * (now - last_refill))
if tokens_available >= 1:
  tokens_available -= 1
  allow request
else:
  reject
```

**Implementation overhead:** O(1) calculation per request.

**Real-world tuning:**
- CloudFlare, AWS, Google APIs use variants
- `capacity=100, rate=10/sec` means: burst 100 requests instantly, then 10/sec sustained
- Setting `capacity = rate` removes burst allowance (smooth throttling); useful for strict quotas

### Sliding Window Log

Track exact timestamps of N recent requests. For each new request, check how many occurred in the last `window` seconds; reject if ≥ `limit`.

**Accuracy:** Highest. No edge-case spike across window boundaries (unlike fixed windows).

**Trade-off:** Requires storing request timestamps (array or linked list per client). Memory scales with `limit` and number of unique clients.

**Implementation:**
```
if count(timestamps in last `window` seconds) >= `limit`:
  reject
else:
  record timestamp, allow
```

**Use cases:** When accuracy matters more than memory (small request limits, few clients). Legacy APIs where boundary-crossing behavior must be eliminated.

### Sliding Window Counter

A hybrid: combines two fixed-window counters to approximate sliding window without storing timestamps.

**Method:**
```
old_window_count = requests in [now - window - 1sec, now - window)
new_window_count = requests in [now - window, now)
weighted_count = old_window_count * (1 - overlap_fraction) + new_window_count
if weighted_count >= limit:
  reject
```

**Trade-off:** Less precise than sliding window log (can still overshoot at boundaries by up to 2x), but O(1) memory.

**Common implementation:** Two counters (old, current) that reset every `window` seconds.

### Leaky Bucket

Reverse of token bucket. Requests queue in a bucket; a leak drains requests at a fixed `rate`. When bucket is full, new requests are rejected (or dropped from the back).

**Dynamics:**
- Smooths bursty traffic into steady output
- Requests served in FIFO order — fair queuing
- Clients experience queueing delay, not rejection

**Trade-off:** Adds latency; suitable for non-time-sensitive workloads (batch processing, log ingestion).

**Real-world:** Used in some message queues and traffic shaping.

### Fixed Window (with Spike Risk)

Count requests in non-overlapping fixed intervals (e.g., `[0-60s, 60-120s, ...)`). Reject if count exceeds limit.

**Problem:** Distributed spike at boundaries. If limit is 1000/min, a client can send 1000 at t=59s and 1000 at t=60s, resulting in 2000 requests in 2 seconds.

**Rule:** Don't use fixed windows for public APIs. Token bucket or sliding window are safer.

## Distributed Rate Limiting

In a microservices architecture, rate limiting must coordinate across multiple servers. In-process counters become inconsistent.

### Redis + Lua Script

**Common pattern:** Store counter and expiry in Redis with a Lua script for atomic increment-and-check.

```lua
-- Check and increment in one atomic operation
local current = redis.call('GET', key)
if current == false then
  redis.call('SET', key, 1, 'EX', window)
  return 1
else
  local count = tonumber(current)
  if count < limit then
    redis.call('INCR', key)
    return count + 1
  else
    return limit + 1  -- over limit
  end
end
```

**Advantage:** Atomic; ensures no race condition between check and increment.

**Bottleneck:** Redis latency (typically 1-5ms). For high-throughput APIs, this adds measurable overhead.

### Token Bucket in Redis (Redisson, Bucket4j)

Libraries like Bucket4j (Java) or Redisson (multi-language) provide distributed token bucket via Redis, synchronizing refill logic across instances.

**Trade-off:** Slightly more complex than simple counter; full burst allowance across all instances (distributed burst).

### Cell-level Sharding

Partition clients into shards. Each shard has independent rate limit storage. A hashing function (e.g., `hash(client_id) % num_shards`) routes requests to a specific shard.

**Advantage:** Reduces single-point contention; some requests can be checked locally.

**Limitation:** Shard capacity becomes the bottleneck. If `num_shards = 10` and each can handle 1000 checks/sec, total throughput caps at 10k/sec.

## Per-User vs. Global Limits

Most APIs enforce both:

**Per-user limit:** "User can make 1000 requests/hour." Isolates usage per client, prevents one client from starving others.

**Global limit:** "API can handle 100k requests/hour total." Protects backend infrastructure from cascading failure.

**Interaction:** A request is allowed only if both budgets have capacity.

```
if (user_count < user_limit) AND (global_count < global_limit):
  allow
  user_count += 1
  global_count += 1
else:
  reject (429)
```

**Fairness issue:** If global limit is hit, which user gets rejected? Typically: oldest requests first (FIFO), or fair queuing allocates proportional budget.

## HTTP Semantics

### 429 Too Many Requests

Standard HTTP status code indicating rate limit exceeded. Return it to the client.

```
HTTP/1.1 429 Too Many Requests
Retry-After: 60
```

### Retry-After Header

Indicates how long the client should wait before retrying.

**Format:**
- Seconds (integer): `Retry-After: 60` — wait 60 seconds
- HTTP date: `Retry-After: Wed, 21 Oct 2026 07:28:00 GMT` — retry after this time

**Client behavior:** Exponential backoff + Retry-After max:
```
wait_time = min(exponential_backoff, Retry-After)
```

### Rate Limit Info Headers

Standardization effort (draft RFC): convey budget state to clients.

- `RateLimit-Limit: 1000` — requests allowed in the window
- `RateLimit-Remaining: 42` — requests left in current window
- `RateLimit-Reset: 1234567890` — Unix timestamp when counter resets

**Note:** Not all APIs implement these; check documentation.

## API Quota Management

Rate limiting assumes time-window-based budgets. Quota management adds:

**Monthly quota:** "100k requests per month." Typically starts on a calendar day (or billing cycle).

**Standing reservations:** "Team pays for 10M requests/month upfront." Enables predictable costs and priority allocation.

**Burst allowance:** "Quota includes 20% burst capacity." Smooths spiky workloads within budget.

**Quota enforcement:**
```
if (monthly_requests_used + 1 > monthly_quota):
  reject (429, Quota exceeded)
else:
  if (burst_used + 1 > burst_capacity):
    rate_limit = base_rate (lower)  # Revert to per-second limit
  allow
  monthly_requests_used += 1
```

**Billing integration:** Track quota usage per customer; bill monthly/yearly based on overage tier.

## Client Strategies

### Exponential Backoff + Jitter

When rate limited:

```
attempt = 0
while attempts < max_attempts:
  response = request()
  if response.status == 429:
    wait = min(MAX_WAIT, 2^attempt + random(0, jitter))
    sleep(wait)
    attempt += 1
  else:
    return response
```

Jitter prevents thundering herd: if all clients backoff the same duration, they retry simultaneously, causing another spike.

### Token Budget Estimation

Clients can estimate their own budget without every request hitting the rate-limit check:

```
budget = RateLimit-Remaining (from previous response)
rate = RateLimit-Limit / window_seconds
estimated_budget = limit * (now - last_response_time) / window_seconds
budget = max(budget, estimated_budget)  # Conservative estimate
```

### Predictive Throttling

For bursty clients (e.g., batch jobs), estimate upcoming load and throttle proactively rather than reacting to 429s.

```
if estimated_next_batch_size + current_usage > limit:
  self_throttle(delay) // Request slower
```

## When to Apply

- **Public APIs:** Always rate limit to prevent abuse and cascade failures
- **Internal services:** Often unnecessary (trust boundary); use only at service boundary
- **Database connections:** Pool limits + query timeouts (not time-window rate limiting)
- **Message queues:** Backpressure (see `patterns-backpressure`) is more appropriate than rate limiting

## See Also

- `security-api.md` — API security broadly
- `patterns-backpressure.md` — Flow control mechanisms
- `patterns-circuit-breaker.md` — Failover when dependencies degrade