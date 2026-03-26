# Application Scaling — Stateless Design, Horizontal Scaling, Auto-Scaling & Session Management

## Overview

**Application scaling** distributes load across multiple application servers (horizontal scaling) and manages the demand fluctuations that trigger scaling decisions. Unlike database scaling, which targets storage or throughput bottlenecks, application scaling addresses **compute and concurrency limits**. The goal is to go from one application server handling 1,000 concurrent requests to ten servers each handling 1,000, without changing application code.

Horizontal application scaling is simpler conceptually than database scaling but requires discipline: applications must be **stateless** (no server-local session data) so any instance can handle any request.

## Stateless Design Principle

A stateless application server holds no information about user sessions or ongoing transactions. After handling a request, restarting that server doesn't affect future requests.

**Stateless architecture:**
```
client → load balancer → [server1, server2, server3, ...]
         (routes to any server)
```

**Non-stateless (coupled to specific server):**
```
client → load balancer → server1
         (sticky routing required;
          restarting server1 disconnects that client)
```

### Why Stateless Matters

1. **Horizontal scaling:** Add a new server; traffic immediately distributes to it. No migration or rebalancing.
2. **Resilience:** Server crash doesn't require reconnection or session recovery on that specific server.
3. **Rolling updates:** Restart server for deploy; traffic routes to others. Zero downtime possible.
4. **Autoscaling:** Spawn or destroy instances based on load without coordinating state migration.

### Implementation Pattern

- **Session storage:** Externalize to database (PostgreSQL, Redis). Application reads session from store per request.
- **HTTP requests:** Stateless by design (no connection state after response). Session ID passed in cookie.
- **WebSockets, SSH, gRPC:** Inherently stateful (maintain connection). Require sticky routing (route same client to same server) or session migration.

## Session Management

### Sticky Sessions (Server Affinity)

Route all requests from the same client to the same server. Session data can exist server-locally (in-memory).

**Implementation:**
- **IP hash:** Route based on client IP (fragile if client IPs change, e.g., mobile).
- **Cookie insertion:** LB inserts routing cookie; subsequent requests honor it.
- **Header routing:** Application returns header indicating preference.

**Tradeoffs:**
- **Pro:** Simple; no session store needed if data is small and doesn't need durability.
- **Con:** Server crash or drain loses session. Uneven load if some clients are busier. Autoscaling scales poorly (new servers not used until new clients arrive).

**When used:** Stateful services (WebSocket servers, game sessions, gRPC connections).

### Distributed Session Storage

Store session externally (Redis, database) so any server can retrieve it.

**Typical flow:**
1. Client sends request with session ID (cookie).
2. Server retrieves session from Redis/DB.
3. Server modifies session (add item to cart, update preference).
4. Server writes session back.
5. Response sent.

**Characteristics:**
- **True horizontal scaling:** New servers immediate productive.
- **Resilience:** Server crash doesn't affect session.
- **Latency:** Extra round-trip to session store (typically <5ms for Redis, <50ms for DB). Acceptable overhead for most workloads.
- **Consistency:** If multiple servers modify same session concurrently, last-write-wins or explicit locking needed.

**Session store options:**
- **Redis:** Fast (in-memory), clusterable, but not persistent. Ideal for short-lived sessions.
- **Database (PostgreSQL, DynamoDB):** Persistent, survives restarts, slower. For sessions that must survive application crash.
- **Memcached:** Similar to Redis but simpler, no persistence.

### JWT (JSON Web Token) Sessions

Encode session data into a signed token. Client stores and returns token with each request; server validates signature.

**Characteristics:**
- **No server session store:** Reduces storage needs.
- **Stateless validation:** Server only needs the signing key.
- **Scalability:** Infinitely scalable (no session service).
- **Tradeoff:** Cannot revoke tokens immediately (until expiration). Token size grows with data.

**Use:** Short-lived access tokens (30min expiry), longer-lived refresh tokens (7 days, stored in session) more common than full session in JWT.

## Vertical vs Horizontal Scaling

### Vertical Scaling (Bigger Servers)

Add CPU, GPU, memory, or network bandwidth to existing instances.

**Characteristics:**
- **Ceiling:** Limited by largest available hardware. Eventually marginal cost exceeds benefit.
- **Downtime risk:** Scaling usually requires restart.
- **Simplicity:** No load balancing, session replication, or multi-instance testing.

### Horizontal Scaling (More Servers)

Add more instances behind a load balancer.

**Characteristics:**
- **No ceiling:** Theoretically unlimited (but diminishing returns from coordination overhead, networking, database contention).
- **Resilience:** Single instance failure doesn't take down service.
- **Complexity:** Load balancing, session management, deployment to N servers.
- **Cost:** Linear scaling (N servers cost N×, no bulk discount curve like vertical).

**Practical hybrid:** Use medium-sized instances (not tiny, not massive) horizontally scaled. Better resilience than single giant instance, better cost efficiency than tiny instances.

## Auto-Scaling Triggers and Orchestration

**Auto-scaling** automatically adjusts capacity based on demand. Triggers and scale policies vary.

### Trigger Metrics

**CPU utilization:** Average CPU across all instances. Target 60-80% (leave headroom for spikes).
- **Pro:** Universal, available everywhere.
- **Con:** Doesn't correlate with application performance (some work is I/O-bound, CPU sits idle).

**Memory utilization:** Less common trigger; memory leaks cause creeping increases. Prefer explicit (allocate memory per instance) over reactive.

**Request latency:** If p99 latency > threshold, scale out. Sophisticated but indicates actual user impact.

**Queue depth:** For async workloads, if task queue grows, spin up workers. Proactive scaling (predict queue growth) is better than reactive.

**Custom metrics:** Application-emitted metrics (active WebSocket connections, in-flight requests, cache hit ratio).

### Scale Policy

**Scale-out (add instances):**
- Triggered when metric > threshold for duration (e.g., CPU > 80% for 2 min)
- Add N instances (or scale to desired count)
- Ramp: Add 1 at a time with cooldown to avoid thrashing, or all at once for urgency

**Scale-in (remove instances):**
- Triggered when metric < threshold for longer duration (e.g., CPU < 30% for 10 min)
- More conservative cooldown (want to avoid thrashing from natural fluctuations)
- Connection draining: Gradually close connections on instance before removing

### Orchestration Platforms

- **Kubernetes:** Horizontal Pod Autoscaler (HPA) based on metrics; declarative scaling policies.
- **Cloud providers:** AWS ASG, GCP MIG, Azure VMSS. Simpler than Kubernetes for traditional VMs.
- **Manual:** Monitor metrics, spawn/destroy instances via API.

## Load Leveling with Async Processing

Synchronous request processing can't scale beyond database/external service capacity. **Async processing** decouples request acceptance from work completion.

**Pattern:**
```
client → API endpoint → [task queued, response 202 Accepted] (instant)
       ↓
   [background worker] → [process task] → [database update] (delayed)
```

**Characteristics:**
- **Client latency:** Decoupled from work duration. Response is instant message "task accepted".
- **Throughput:** Limited by queue depth and worker capacity, not synchronous processing.
- **Scalability:** Add workers independently; queue is central (but easily replicated/clustered).
- **Tradeoff:** Task result not immediately available to client. Requires polling or webhooks.

**Queue options:**
- **In-memory queues** (RabbitMQ, Redis, NATS): Fast but single point of failure if not replicated.
- **Distributed (Kafka, Pulsar):** Fault-tolerant, replicated, but higher latency.
- **Cloud queues** (AWS SQS, GCP Pub/Sub): Managed; scaling handled by provider.

## Graceful Degradation

Under extreme load, systems can't keep up. **Graceful degradation** reduces functionality rather than failing completely.

**Techniques:**
- **Circuit breaker:** If downstream service (database, external API) is slow, fail open and use cached/stale data instead of queuing.
- **Drop non-essential features:** Disable logging, caching, personalization. Serve only core functionality.
- **Request shedding:** Reject low-priority requests (e.g., analytics, telemetry) before rejecting user-facing requests.
- **Timeout and fast-fail:** Don't wait for timeouts to trigger; if latency is degrading, proactively reject.

## Regional and Multi-Region Scaling

Scaling isn't just about adding servers in one data center.

**Geo-distribution:**
- **Regional:** Deploy in multiple AWS regions, each with independent auto-scaling.
- **Global load balancing (DNS, Anycast):** Route users to nearest region.
- **Consistency challenges:** Distributed across regions increases latency for real-time sync. Updates must be asynchronous/eventual. Session storage must be replicated.

See also: **infrastructure-capacity-planning.md**, **infrastructure-load-balancing.md**, **distributed-data-consistency.md**, **web-api-patterns.md**.