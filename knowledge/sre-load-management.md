# SRE — Load Management: Graceful Degradation, Load Shedding & Failure Prevention

## Core Problem

Systems have finite capacity: compute, network bandwidth, disk I/O, database connections. When load exceeds capacity, failure is inevitable. Load management is the practice of making intelligent tradeoffs—accepting degraded service rather than total failure—when resources are scarce.

## Load Shedding & Graceful Degradation

### Load Shedding

**Load shedding** means deliberately rejecting some requests before they consume resources, rather than accepting all requests and failing catastrophically.

Strategies:

1. **Reject lowest-priority traffic** (non-paying users, non-critical operations, batch jobs)
2. **Rate limiting** (hard per-client quota; drop excess)
3. **Queue overflow rejection** (if queue is full, reject new requests immediately)
4. **Admission control** (predict if request will complete before timeout; reject if unlikely)

The key: rejecting a request is *better* than accepting it and timing out. A rejected request has low blast radius; a slow timeout ties up resources and cascades to other systems.

### Graceful Degradation

**Graceful degradation** means serving reduced functionality rather than failing entirely.

Examples:

- **Personalization disabled**: During load surge, recommendation algorithm is skipped (users see unranked results); page loads faster
- **Cache bypass**: Look-aside cache is degraded (TTL reduced or skipped); slight increase in database latency, but system stays up
- **Asynchronous processing**: Real-time requests complete synchronously; async jobs (analytics, notifications) are queued for later
- **Reduced freshness**: Stale data is acceptable if current data is unavailable (cache hit, not database query)

Tradeoff: Users experience degraded UX but system remains operational.

## Priority-Based Request Handling

Rank requests by business value and handle them in priority order during saturation.

### Priority Tiers

**Tier 1 (Critical)**: Paying customer transactions, security-related, payment processing → Always attempt
**Tier 2 (Important)**: Premium features, authenticated sessions → Attempt, shed if critical
**Tier 3 (Best effort)**: Analytics, notifications, batch processing → Shed under load

During saturation:

1. Accept critical requests (may still timeout if queue grows)
2. Accept important requests if capacity available
3. Shed best-effort requests
4. If still overloaded, gracefully reject some critical requests with "try again later" rather than timeout

### Implementation

- **Request classification**: Router examines request (user ID, endpoint, headers) and assigns tier
- **Quota enforcement**: Per-tier budget of connections/CPU/queries
- **Shedding policy**: When quota reached, reject new requests in that tier
- **Exponential backoff**: Clients retry with increasing delay (avoid thundering herd when system recovers)

Example: SaaS platform during traffic spike:
- Tier 1: API calls from paying enterprise customers → always accept
- Tier 2: API calls from trial users → accept if slots available
- Tier 3: Batch analytics jobs → shed entirely until load drops

## Circuit Breakers at Scale

**Circuit breakers** prevent cascading failures by stopping requests to degraded dependencies before they timeout.

### States & Transitions

- **Closed (normal)**: Requests flow normally; success counters reset
- **Open (failure detected)**: Error rate or latency exceeds threshold; new requests immediately fail (fast, without waiting)
- **Half-open (recovery)**: After cooldown, allow limited test requests; if they succeed, close; if fail, reopen

### Scaling Pattern: Multiple Stages

In large systems, cascading failures chain across services. Multi-stage circuit breaker patterns mitigate:

1. **Application-level breaker**: Service A detects Service B is slow → opens circuit → drops requests to B → preserves capacity for Service A's clients
2. **Client-side pool enforcement**: Load balancer/client detects backend instance is degraded → removes from pool → routes around it
3. **Database connection pool**: Exhausted connections trigger immediate "too many connections" error; prevents connection pool bloat

### Configuring Breakers

- **Threshold**: Normally 50% error rate or p99 latency > target; topic-specific
- **Cooldown**: After opening, wait 30-60s before half-open (give time for recovery)
- **Fast-fail timeout**: Requests timeout immediately at t=100ms rather than waiting for full timeout

## Adaptive Concurrency Control

Static concurrency limits (fixed connection pool, fixed thread pool) don't adapt to degraded conditions. **Adaptive concurrency** adjusts limits based on latency feedback.

### Mechanism

1. **Measure**: Track p99 latency of requests
2. **Adjust**: If latency increases, reduce concurrency limit (fewer simultaneous requests)
3. **Recover**: If latency returns to target, increase concurrency slowly
4. **Limit enforcement**: Reject requests that exceed dynamic limit

Example:

- Normal: p99 latency is 50ms; allow 100 concurrent requests
- Load spike: p99 latency climbs to 500ms; reduce limit to 50 concurrent requests
- Dependency slow: p99 latency hits 2s; reduce limit to 10 concurrent requests
- Recovery: p99 latency returns to 60ms; increase limit back to 100

Benefit: System automatically backs off without operator intervention.

## Cascading Failure Prevention

Cascading failures occur when one service's degradation causes upstream services to degrade, which causes their upstream services to degrade. The collapse spreads. Prevent by:

### Timeout Hierarchies

Each service layer has a timeout, each slightly less than the next:

- Client timeout: 30s (wait for response or fail)
- Frontend timeout: 25s (wait for backend or fail, respond to client with error)
- Backend timeout: 20s (wait for database or fail, respond to frontend)
- Database timeout: 15s (abort query or fail)

If database is slow (10s query + processing = 15s), backend times out cleanly → frontend times out → client timeout is respected. No unbounded queuing.

### Bulkheads & Resource Isolation

Partition resources by workload type:

- **Thread pool per endpoint**: Slow reports don't steal threads from fast API calls
- **Database connections per tenant**: One tenant's runaway query doesn't exhaust connection pool for others
- **Queue isolation**: Real-time requests and batch jobs use separate queues; batch overload doesn't starve real-time

### Retry Storms & Exponential Backoff with Jitter

When service recovers from outage, all queued requests retry simultaneously, causing new spike (thundering herd). Prevent:

1. **Exponential backoff**: Client retries after delay = base_delay * (2 ^ attempt), capped (e.g., 100ms, 200ms, 400ms, 1s, 1s, 1s)
2. **Jitter**: Add random ± 10% variation (`delay = (base_delay * 2^attempt) * (0.9 + 0.2 * rand())`), so retries spread over time
3. **Circuit breaker**: If all retries fail, circuit opens; clients stop retrying until cooldown expires
4. **Max retries**: Limit to 3-5 attempts; beyond that, accept failure and avoid retry storms

Formula example: `delay_ms = min(30000, previous_delay_ms * 2 * (0.9 + 0.2 * random()))`

## Queue-Based Load Leveling

Decouple timing of request arrivals from processing capacity.

### Pattern

1. **Client** submits request to queue (fast acknowledgment)
2. **Server** processes requests from queue at sustainable rate
3. **Speed mismatch absorbed**: Long queue during spike; processes gradually as capacity allows

Benefits:

- Client doesn't wait for server to be available; they get immediate ack
- Server processes at sustainable rate; no bursty overload
- Failures are localized (queue persists; requests retry; decoupled from frontend)

Tradeoff: Requests are not processed immediately; suitable for async workflows (notifications, analytics, batch).

### Failure Modes

- **Queue overflow**: If queue grows unbounded, memory exhaustion → server crash. Mitigate: Set max queue size; reject new requests when full
- **Poison messages**: Bad request queued permanently, blocks others. Mitigate: Dead-letter queue (forward bad messages after N retries); separate error queue
- **Delayed failure visibility**: Client gets "accepted" but doesn't know if processing failed hours later. Mitigate: Callback or status polling after completion

## Putting It Together: Load Spike Scenario

High-traffic event (launch, sale, viral moment) causes 10x traffic spike:

1. **Immediate**: Load balancer routes to servers
2. **Servers saturate**: CPU/memory/connections exceed capacity
3. **Circuit breaker opens**: Database is slow (queries queue); backend circuit opens to database
4. **Load shedding engages**: New requests to non-critical endpoints are rejected with 503 "try again later" + exponential backoff
5. **Graceful degradation**: Personalization disabled; low-priority background jobs offloaded to queue
6. **Priority handling**: Critical transactions (payments) still execute; batch processing deferred
7. **Timeout hierarchy**: Frontend drops requests that can't complete in 5s rather than queuing indefinitely
8. **Queue levels**: Async work spreads over time from queue
9. **Recovery**: Spike subsides; backlogged requests process from queue; circuit breaker enters half-open, then closes

Result: Some users see degraded UX; no cascading failure; no data loss; system recovers naturally.

## See Also

- [SRE — SLO Engineering](sre-slo-engineering.md) — Managing error budgets during load events
- [Patterns — Rate Limiting](patterns-rate-limiting.md) — Implementation details for rate limiting
- [Architecture — Resilience](architecture-resilience.md) — Resilience patterns including bulkheads and circuit breakers