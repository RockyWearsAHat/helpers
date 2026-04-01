# Load Balancing Algorithms — Theory, Trade-offs & Selection

## Overview

Load balancing distributes incoming requests across multiple backend servers. The **algorithm** determines which backend receives each request. Selection affects fairness, latency, cache efficiency, and operational simplicity.

Algorithms exist on a spectrum: **simple deterministic** (round-robin, least connections) to **stateful complex** (power-of-two-choices, consistent hashing) to **aware** (response-time aware, locality-aware). No single algorithm is best; choice depends on traffic patterns, backend heterogeneity, and required properties (e.g., session affinity, cache locality).

## Simple Deterministic Algorithms

### Round-Robin

Cycle through backends in order: request 1 → backend A, request 2 → backend B, ..., request N+1 → backend A.

**Pros:**
- Trivial to implement (circular counter)
- Predictable
- No state tracking required

**Cons:**
- **Assumes identical backends**: if one backend is slower, it gets as many requests as faster ones
- **No awareness of backend load**: if backend A processes long requests, it's still next in rotation
- **Cache locality isn't optimized**: same user ID might always go to different backends (no session affinity unless explicitly added)

**Use case:** homogeneous backends receiving similar-load requests.

### Weighted Round-Robin

Assign integer weight to each backend. Request 1 → A, request 2 → B, request 3 → B, request 4 → A (if A,B have weights 1:2). Distributes proportionally.

**Pros:**
- Accounts for backend capacity (slow backend gets fewer requests)
- Easy to tune in operation

**Cons:**
- Weights must be configured and maintained
- Still doesn't react to real-time backend state (if a backend becomes slow mid-request-stream, weights are stale)
- Requests with different processing times still lead to queue buildup on heavier backends

**Use case:** known backend capacity differences (e.g., server A has 64 cores, server B has 32 cores).

## Connection-Aware Algorithms

### Least Connections

Track active connections per backend. New request goes to backend with fewest open connections.

**Implementation:**
```
backends = [
  {ip: "10.0.1.1", connections: 3},
  {ip: "10.0.1.2", connections: 1},
  {ip: "10.0.1.3", connections: 5}
]
new_request → backend[1] (min connections)
```

**Pros:**
- Adapts to real-time backend load (slow backend accumulates connections, gets fewer new ones)
- Works well for request/response protocols (HTTP) where connection count correlates with load
- Simple state tracking (per-backend counter)

**Cons:**
- **Ignores request processing time**: if backend has 100 pending long-running requests, a new short request goes elsewhere — good — but the counter seeing 100 connections doesn't tell you how backlogged clients are
- **Not effective for long-polling or WebSocket**: connections stay open even if idle
- **L4 vs L7 limitation**: L4 load balancers see only open TCP connections, not HTTP request count (affects accuracy)

**Use case:** Short-lived request/response protocols (HTTP request/response), unknown backend heterogeneity.

### Least Response Time

Track ongoing requests per backend AND recent response time. Route to backend with lowest `(active_requests + expected_wait)`.

**Concept:**
```
backend A: 2 active requests, avg response time 100ms
           expected_wait ≈ 2 * 100ms = 200ms

backend B: 10 active requests, avg response time 10ms
           expected_wait ≈ 10 * 10ms = 100ms
           
new_request → backend B (lower expected wait)
```

**Pros:**
- Accounts for request processing time, not just count
- Better latency distribution for heterogeneous backends
- Naturally load-balances fast vs. slow backends

**Cons:**
- Requires response-time tracking (sampling or instrumentation)
- Assumes response time is predictable (varies if backend is CPU-bound vs. I/O-bound)
- Harder to implement (more state than least connections)

**Use case:** heterogeneous backends or requests with widely varying processing times.

## Hash-Based Algorithms

Hash-based algorithms use a function to map requests to backends, enabling **session affinity** (same user always goes to same backend) and cache locality.

### IP Hash (Source IP-Based)

Route based on client IP: `backend_index = hash(client_ip) % num_backends`.

All requests from a single client go to the same backend.

**Pros:**
- Session affinity (user stays on same backend)
- No state tracking (hash is deterministic)
- Works for L4 load balancers (only need source IP from packet header)

**Cons:**
- **Uneven distribution if client IPs cluster** (e.g., mobile users behind same NAT share few IPs)
- **Backend failure or addition rebalances all clients** (hash(ip) % 3 differs from hash(ip) % 4 for most IPs)
- Doesn't account for backend capacity

**Use case:** Simple session affinity where stickiness matters but perfect distribution is less critical (e.g., user session stored on backend).

### Consistent Hashing

Hash-based but tolerates backend addition/removal WITHOUT rebalancing most clients.

**Concept:** Arrange backends and items on a ring. Hash(item) lands on the ring; walk clockwise to find the next backend. When a backend is added, only items in the "new segment" are rebalanced; others stay put.

**Classic scenario:**
```
Ring positions: 0, 1, 2, ..., 360

Backend A at position 90
Backend B at position 200
Backend C at position 300

Request with hash=150 → walk clockwise from 150 → find Backend B at 200

Add Backend D at position 400:
  Requests hashing to 300-400 now go to D (instead of wrapping to A)
  Requests with hash 1-90 still go to A
  Rebalancing limited to ~25% of traffic (segment for D), not 75% (like simple hash)
```

**Pros:**
- Minimal rebalancing on backend changes (only ~1/N of load shifts when adding/removing one backend)
- Session affinity preserved for existing clients
- Scales well for dynamic systems (containers spinning up/down)

**Cons:**
- Uneven distribution possible (backends land at poor ring positions)
- Mitigated by **virtual nodes**: each backend claims multiple positions on ring (e.g., Backend A at rings 90, 91, 92, ... — improves distribution variance)
- Requires implementation (not as simple as round-robin)

**Use case:** Dynamic systems with service discovery (Kubernetes, service mesh), caching layers (memcached consistent-hash avoids cache invalidation on backend changes).

## Probabilistic Algorithms

### Random

Select a backend uniformly at random: `backend_index = random(0, num_backends)`.

**Pros:**
- Simple (no state)
- Guaranteed even distribution over many requests (by law of large numbers)
- No hashing function needed

**Cons:**
- **High variance for small request counts**: 10 requests to 5 backends is not guaranteed 2 each
- **No session affinity**
- **Doesn't account for backend state**

**Use case:** Stateless requests where minor imbalance is acceptable and simplicity matters.

### Power of Two Choices

Instead of random single backend, **sample two random backends and choose the one with fewer active connections**.

**Compared to pure random:**
- If random picked backend with 100 connections and another random pick has 1 connection, choosing the second is better
- Still randomized (no deterministic hash) so state-less, but much better load distribution

**Performance:** Reduces max queue depth from O(log N) (pure random) to O(log log N) where N is the number of backends. Proven in probability theory.

**Pros:**
- Simple (two random samples per request)
- Better distribution than pure random
- No per-backend state tracking (just active count)
- Stateless

**Cons:**
- Only marginal improvement if all backends are already balanced
- Still doesn't account for response time or backend capacity differences
- Requires real-time connection counting

**Use case:** Simple systems where least-connections is too much state, but random isn't good enough; cloud functions, microservices with many identical backends.

## Locality & Awareness Algorithms

### Locality-Aware (Geographic, Data-Center)

Route requests to backends in the **same rack, zone, or geographic region** first. Reduces latency, preserves data-center locality.

**Pattern:**
```
Client in region US-West → prefer backend in US-West
             → fallback to US-Central (longer latency)
             → fallback to US-East (last resort)
```

**Implementation:** Global load balancer (DNS-based or BGP-advertised) routes to regional LB. Regional LB routes to local backends.

**Use case:** Global services with data-center replication (CDNs, SaaS platforms with multi-region failover).

### Request-Aware (Content-Based)

Route based on request properties: URL path, header, cookie.

- "All requests to `/api/*` go to `api-tier` backends"
- "All requests with `User: premium` header go to `premium-tier` backends"
- "API reads go to `read-replica`, writes to `write-primary`"

This is L7 load balancing (HTTP-aware).

**Use case:** Multi-tier architectures (API tier, web tier, cache tier), prioritization (premium users get lower-latency backends).

## Health Checking & Connection Draining

### Health Checks

Backends are periodically tested (TCP port check, HTTP /health endpoint, gRPC healthcheck). If failing, backend is marked `down` and removed from pool.

**Frequency:** typically every 5-10 seconds for L7, faster for custom checks. Trade-off: frequent checks add overhead; slow checks delay failover.

**Graceful degradation:** If N backends and 1 is down, remaining N-1 share the load. No request loss (assuming load balancer has connection pooling).

### Connection Draining (Graceful Shutdown)

When removing a backend (upgrade, scale-down):

1. Mark backend as `draining` (stop accepting new requests)
2. Wait for existing connections to complete
3. Remove backend after timeout (e.g., 30 seconds)

This avoids dropping in-flight requests.

**Without draining:** Abrupt backend termination → client gets TCP RST → client must retry (if idempotent) or lose transaction.

**With draining:** Backend finishes and closes gracefully → client sees FIN → client knows request completed.

## L4 vs L7: Algorithm Implications

### L4 Load Balancing

Operates on TCP/UDP headers (IP, port, protocol). Supported algorithms:

- Round-robin, weighted
- IP hash (source IP)
- Least connections (TCP state table)
- Random, power-of-two

**Can't do:**
- Response-time aware (no request visibility)
- URL-based routing (need to inspect HTTP headers)
- Connection draining (L4 sees TCP handshake, not HTTP semantics)

**Advantage:** Very fast, high throughput. Hardwired in hardware load balancers.

### L7 Load Balancing

Operates on application protocol (HTTP, gRPC). Additional algorithms available:

- Least response time
- Request-aware routing
- Connection draining with HTTP 1.1 compatibility
- Rate limiting per backend

**Trade-off:** More overhead (must parse HTTP, maintain per-request state), but more intelligent routing.

## Algorithm Selection Matrix

| Scenario | Algorithm | Reason |
|----------|-----------|--------|
| **Identical backends, simple stateless** | Round-robin | Trivial, sufficient |
| **Known capacity differences** | Weighted round-robin | Accounts for size differences |
| **Unknown/dynamic backend latency** | Least connections | Adapts in real-time |
| **Very heterogeneous backends** | Least response time | Accounts for processing time |
| **Session affinity needed** | IP hash or consistent hash | Keeps user on same backend |
| **Dynamic backend membership** | Consistent hash | Minimizes rebalancing |
| **Simple + good distribution** | Power of two | Best effort, stateless |
| **Microservices, multi-region** | Locality-aware + health checks | Reduces latency, failover |
| **Multi-tier routing** | L7 (request-aware) | Route by content-type, user role |

## See Also

- [infrastructure-load-balancing.md](./infrastructure-load-balancing.md) — Architectures and deployment models
- [architecture-patterns.md](./architecture-patterns.md) — Consistency hashing in distributed systems
- [infrastructure-service-discovery.md](./infrastructure-service-discovery.md) — Integration with service registry
- [algorithms-hash-tables.md](./algorithms-hash-tables.md) — Hash functions and collision strategies