# API Rate Limiting — Algorithms, Headers & Quota Management

## Overview

Rate limiting protects API backends from overload by restricting request volume per client. From the API design perspective, the challenge extends beyond the algorithm: you must choose which requests to limit (per-user, per-IP, per-API-key, per-endpoint?), communicate limits clearly to clients, and handle graceful degradation when limits are reached. The algorithm is implementation detail; the API contract is what matters for client integration.

## Rate Limiting Scope & Identity

Effective rate limiting requires choosing what constitutes a "client":

**API Key**: Most common for authenticated APIs. Each key gets its own quota. Problem: one key shared across multiple services over-counts one downstream consumer while starving others.

**User ID**: For user-centric APIs. Rate limit per user across all their API keys. Better fairness, but harder to track for multi-tenant systems.

**IP Address**: For public APIs without authentication. Problem: corporate networks behind a single exit IP get treated as one client. Causes customer complaints when one overzealous user starves the entire company's access.

**Endpoint-level Quota**: Different endpoints have different costs. Upload endpoints should have stricter limiting than read endpoints. Trade-off: added complexity for clients who must now track multiple limits.

**Tiered Quotas**: Freemium models assign higher limits to paid tiers. Enforcement happens at authentication/provisioning time.

## Algorithm Selection for API Design

See `patterns-rate-limiting.md` for algorithm details. From an API design perspective:

### Token Bucket (Industry Standard)

**Why dominant:** Allows bursts (clients can consume up to `capacity` tokens instantly) while maintaining steady-state rate. This matches human usage patterns: occasional spike of requests followed by idle periods.

**API tradeoff:** Simple to communicate to clients. "100 requests per hour" is broadly understood. Internals (tokens, refill rate, capacity) are hidden; clients just experience rejection or throttling.

### Fixed Window Counter

**Simplicity:** Easy for small-scale systems. "100 requests per minute" resets at `:00`, `:01`, etc.

**API problem:** Boundary effects create visible unfairness. Client near window boundary gets only 50 requests; client aligned with window start gets 100. Causes support complaints: "Why did I get rate-limited mid-hour?"

### Sliding Window Log

**Why rarely exposed:** Requires expensive tracking of individual request timestamps. Clients don't benefit from this accuracy — they just see "rejected" or "allowed."

**API use case:** Internal APIs where precision trumps client experience. Compliance scenarios where audit logs need exact timing.

### Sliding Window Counter

**Trade-off algorithm:** Less precise than sliding window log (can exceed limit by 2x at boundaries in worst case), but O(1) memory. Rarely mentioned to clients; usually hidden behind generic "rate limit exceeded."

## API Contracts & Headers

### Standard Rate Limit Headers

The API communicates quotas to the client via HTTP response headers:

```
RateLimit-Limit: 100
RateLimit-Remaining: 42
RateLimit-Reset: 1630703445  (Unix timestamp when limit resets)
```

**Semantics:**
- `RateLimit-Limit`: Total requests allowed in the window
- `RateLimit-Remaining`: How many requests left (decrements with each call)
- `RateLimit-Reset`: Unix timestamp when the window resets

**Client responsibility:** Apps track these headers and proactively back off before hitting zero. Good clients implement exponential backoff + jitter; bad clients hammer until `429 Too Many Requests`.

### On Rejection: 429 Too Many Requests

When a client exceeds quota, respond with `429 Too Many Requests`.

Optional helpful additions:
```
HTTP/1.1 429 Too Many Requests
RateLimit-Reset: 1630703445
Retry-After: 60  (seconds to wait, or HTTP-date)

{
  "error": "rate_limit_exceeded",
  "message": "100 requests per minute exceeded. Retry after 60 seconds."
}
```

**Retry-After semantics:** Either seconds-to-wait (integer) or HTTP-date. Clients use this to schedule retry. If missing, clients guess (exponential backoff).

### No Single Standard

Different services use different header names:
- GitHub: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`
- Stripe: Same as GitHub
- AWS: No standard headers; rate limit info in error response body
- Twitter/X: `x-rate-limit-limit`, (lowercase), same pattern

**API design lesson:** Pick one convention and stick with it. Document clearly. Consistency matters more than perfection.

## Distributed Rate Limiting

Single-server rate limiting breaks down in distributed systems:

**Problem:** Each server maintains its own counter. Client makes 100 requests, distributed across 10 servers. Each server sees 10 requests (within limit), total is 100x over limit.

**Solution:** Centralized shared state (Redis, Memcached):
```
Client → Request → [Server A/B/C] → Query Redis: "increment:user-123"
  Redis increments counter, returns new value
  If value > limit, reject
  Otherwise, allow
```

**Trade-off:** Redis adds latency + new failure mode (Redis down = rate limiter down). Most APIs accept this; the alternative (per-server limits) is worse.

**Failure mode:** If Redis becomes unavailable, do you:
1. Reject all requests? (availability sacrifice for consistency)
2. Allow all requests? (consistency sacrifice for availability)
3. Fall back to per-server limits? (limit accuracy suffers, but requests flow)

Industry varies. Payment processors choose #1 (safety). Social media APIs choose #3 (user experience).

## Quota Enforcement Patterns

### Hard Limit (Blocking)

Reject requests that exceed quota. Client sees `429` and must retry later.

**Pros:** Strict, predictable, protects backend.
**Cons:** Sudden breakage for clients. "Why did my script die at request 101?"

### Soft Limit (Throttling)

Allow requests beyond quota but queue them or delay responses. Client sees higher latency but no errors.

**Pros:** Graceful degradation. Bursty usage doesn't break integrations.
**Cons:** Backend still overflows if throttled requests back up. Works only for latency-tolerant workloads (batch jobs, not interactive UI).

### Soft + Automatic Backoff

Client receives `429` and implements exponential backoff (2s, 4s, 8s, ...). Backend counts backoff delays toward recovery time.

**Pros:** Allows temporary overages (e.g., spike during lunch hour) without breaking clients globally.
**Cons:** Requires sophisticated client implementation.

## Tiered & Usage-Based Quotas

### Simple Tiers

```
Free: 100 requests/hour
Pro: 10,000 requests/hour
Enterprise: Unlimited
```

**API design:** Require explicit tier specification at authentication time. Client code doesn't need to be aware of tiers; tier enforcement is transparent.

**Problem:** What if a customer upgrades? Do they immediately get new limit, or do they see their old tier until the rate window resets? Generally: upgrade takes effect immediately; downgrade may have a grace period (e.g., reset at next calendar day).

### Usage-Based Metering

Beyond request count, meter actual usage:
- Requests: raw count
- Data volume: GB transferred
- Compute: inference tokens (LLM APIs), endpoint hours (ML)

Each dimension has its own quota.

**API complexity:** Response must include consumption metadata:
```
{
  "data": [...],
  "metrics": {
    "requests_used": 1,
    "tokens_used": 423,  // LLM-specific
    "cost_cents": 2
  }
}
```

**Billing integration:** Usage is forwarded to billing system for invoicing. Quota enforcement is often delayed (check usage after invoice generation) to avoid over-billing during sync delays.

## Consumer Communication & Transparency

### Quota Visibility Endpoints

Provide an endpoint that returns current usage without consuming quota:

```
GET /v1/account/rate-limit-status
→ 200 OK
{
  "limit": 10000,
  "used": 3421,
  "remaining": 6579,
  "reset_at": "2024-03-26T15:30:00Z",
  "tier": "professional"
}
```

**Value:** Clients proactively decide when to batch vs. request. Removes guesswork.

### Rate Limit Exceeded Feedback

When rate-limited, include actionable context:

```json
{
  "error": {
    "code": "rate_limit_exceeded",
    "message": "1000 requests per minute exceeded",
    "retry_after_seconds": 60,
    "usage": {
      "current_window_requests": 1001,
      "limit": 1000,
      "window_start": "2024-03-26T15:29:00Z",
      "window_end": "2024-03-26T15:30:00Z"
    }
  }
}
```

**Value:** Clients understand *why* they're limited and *when* to retry. Reduces support tickets.

### Deprecation & Limit Changes

If you change rate limits, communicate in advance:

1. Document the change in API changelog
2. Email affected customers (especially those near current limit)
3. Provide a grace period (e.g., 30 days of reduced enforcement)
4. On enforcement date, monitor logs for clients that start hitting new limit

**Why this matters:** Rate limit changes break automated systems. A batch job that ran fine at 500 req/min breaks if you drop limit to 100 req/min without warning.

## See Also

- `patterns-rate-limiting.md` — Algorithm details (token bucket, sliding window, etc.)
- `security-rate-limiting-defense.md` — Rate limiting for DDoS/brute force defense
- `api-design.md` — REST principles and error handling