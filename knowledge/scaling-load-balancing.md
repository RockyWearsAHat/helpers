# Load Balancing for Scaling — Algorithms, Architectures, and Operational Patterns

## Overview

**Load balancing** distributes incoming traffic across multiple backend servers, preventing any single server from becoming a bottleneck. The load balancer sits between clients and backends, making routing decisions based on the chosen **algorithm** and available state (backend health, connection counts, request properties).

Load balancing is essential for horizontal scaling. Without it, traffic cannot distribute across multiple servers. Choosing the right algorithm and implementing health checks and graceful degradation separates functioning scaling from cascading failures.

## Layers: L4 vs L7 Load Balancing

The OSI model layer at which the load balancer operates determines what information it can see and decide on.

### L4 (Transport Layer) Load Balancing

L4 LBs operate on TCP/UDP headers (IP, port) only. No application payload inspection.

**Decision basis:**
- Source IP, destination IP, source port, destination port
- Protocol (TCP, UDP)
- Connection state (new, established)

**Algorithms available:**
- Round-robin (simple; no state needed)
- Least connections (track active connections per backend)
- IP hash (consistent single backend per source IP)

**Characteristics:**
- **Latency:** Minimal (just packet header rewriting)
- **Throughput:** Highest (can be hardware-implemented, no buffering)
- **Protocol agnostic:** Works with TCP, UDP, SSH, SMTP, anything TCP-based
- **Limitation:** Cannot make application-aware decisions. If one backend is slower, it's still next in rotation.

**Inner workings:**
1. Client connects to LB virtual IP (VIP) on port 80
2. LB rewrites packet destination (→ backend IP, port 80)
3. Backend receives packet, processes, responds
4. Response may go directly to client (DSR, Direct Server Return) or back through LB

**Examples:** Linux IPVS, HAProxy L4, F5 GTM, AWS Network Load Balancer (NLB)

### L7 (Application Layer) Load Balancing

L7 LBs parse application protocol (HTTP headers, URLs, cookies, JSON bodies). Full application context available.

**Decision basis:**
- HTTP method, path, query string, headers, cookies
- JWT claims or session data
- Request body (for APIs)
- Custom application headers

**Example routing rules:**
```
if path starts with /api/v1:           route to api-backend
if path starts with /images/:           route to image-backend
if header[X-Premium] == true:          route to premium-tier
if cookie[session_id] matches pattern: route to session-affinity backend
if body.user_tier == "enterprise":     route to enterprise-backend
```

**Characteristics:**
- **Latency:** Higher (must buffer and parse HTTP)
- **Throughput:** Lower (CPU-intensive per request)
- **Intelligence:** Can make fine-grained routing decisions
- **Capability:** Can rewrite requests/responses (inject headers, modify body)

**Disadvantages:**
- Must buffer entire request (problematic for large uploads)
- Must parse HTTP (CPU overhead)
- Complexity (more code, more bugs)

**Examples:** HAProxy L7 mode, NGINX, Envoy proxy, AWS Application Load Balancer (ALB), Traefik

### When to Use Each

**L4:** UDP-based services, non-HTTP protocols, extreme throughput requirements (millions of RPS), simple round-robin routing sufficient.

**L7:** HTTP APIs, content-based routing, URL rewriting, authentication/authorization at edge, rate limiting per path.

## Load Balancing Algorithms

### Round-Robin

Route requests in circular order: request 1 → backend A, request 2 → backend B, ..., request N+1 → backend A.

**Pros:**
- Trivial to implement (circular counter)
- Predictable
- No state needed

**Cons:**
- Assumes identical backends. If backend B is slower (GC pause, cache miss), it's still next in rotation.
- No awareness of backend load. Response time disparity causes queuing on slower backends.

**Best for:** Homogeneous backends with identical request handling time.

### Weighted Round-Robin

Assign weight to each backend. Backend A: weight 1, Backend B: weight 2. In each cycle, A gets 1 request, B gets 2.

**Pros:**
- Accounts for known capacity differences (if server A has 64 cores, server B has 32)
- Easy to tune dynamically

**Cons:**
- Weights must be configured and maintained
- Doesn't react in real-time to degradation (weight is static)

**Best for:** Known, stable backend capacity differences.

### Least Connections

Route to backend with fewest active connections.

**Mechanism:**
```
backends = [
  {ip: "10.0.1.1", connections: 5},
  {ip: "10.0.1.2", connections: 2},
  {ip: "10.0.1.3", connections: 8}
]
new_request → backend[2] (min connections)
```

**Pros:**
- Reacts to connection count imbalance
- Better for long-lived connections (WebSocket, SSH, persistent HTTP)

**Cons:**
- Assumes equal connection performance. If backend A handles requests in 100ms and B in 10ms, A will have higher connection count (requests pile up) even though it's slower.

**Best for:** Long-lived connections, variable request duration.

### IP Hash / Consistent Hashing

Hash source IP, mod number of backends: `backend_id = hash(client_ip) % N`.

**Pros:**
- Same client always routes to same backend
- Session affinity without storing state
- Survives brief backend additions/removals (hash ring with virtual nodes)

**Cons:**
- If backend goes down, some clients rehash to different backend entirely. Sessions lost.
- Uneven distribution if hash function is poor or client IPs skewed
- Consistent hashing mitigates but adds complexity

**Best for:** Stateful backends requiring session affinity (sticky sessions). Less common with distributed session stores (prefer L7 routing by session ID).

### Least Response Time / Power of Two Choices

Route to backend with lowest average response time, or sample two random backends and choose the faster.

**Pros:**
- Adaptive. Automatically load-balances toward faster backends.

**Cons:**
- Requires tracking response time per backend (additional state)
- Sampling two random (vs. tracking all) reduces state space and is nearly optimal with lower overhead

**Best for:** Heterogeneous backend performance (some servers faster than others, load-dependent).

## Health Checks and Failover

Load balancer must detect unhealthy backends to avoid routing traffic to them.

### Check Models

**Active (proactive) checks:** LB sends regular probes (HTTP GET, TCP SYN, gRPC health check).

```
Every 5 seconds:
  if GET /health → 200 OK: backend is HEALTHY
  if timeout or 5xx: backend is UNHEALTHY
```

**Characteristics:**
- Proactive (detect issues before traffic hits unhealthy backend)
- Extra traffic (each LB sends probes, adds ~1% overhead)
- Configurable interval (tradeoff: frequent = fast detection but more traffic; infrequent = less overhead but slower detection)

**Passive (reactive) checks:** LB observes client traffic. If requests fail/timeout, mark backend unhealthy.

```
If 3 consecutive requests fail: mark UNHEALTHY
If 2 successful requests: mark HEALTHY
```

**Characteristics:**
- No extra traffic
- Slower detection (depends on client traffic pattern)
- Cascading failures (unhealthy backend receives less traffic, fewer chances to recover)

**Hybrid:** Most LBs combine both (active probes for detection, passive observation to confirm).

### Failover Behavior

When backend fails:
1. LB stops routing new requests to it
2. Existing connections to that backend: either close abruptly or drain gracefully
3. Upon recovery: reintroduce backend gradually (warmup requests) to avoid thundering herd

**Graceful degradation:**
- If all backends fail, LB can serve cached error page (502, 503) rather than connection refused
- Or failover to standby pool
- Or circuit break and return error quickly instead of waiting for timeout

## Connection Draining and Graceful Shutdown

During deployment or maintenance, a backend should drain existing connections before shutting down, rather than abruptly terminating them.

**Process:**
1. Mark instance as "draining" (stop accepting new connections)
2. Existing connections complete (wait timeout, typically 30-300 seconds)
3. Kill process; new traffic routes to other backends

**Orchestration:**
- Kubernetes: Set `terminationGracePeriodSeconds` (e.g., 60)
- Cloud providers: Auto-scaling group termination hooks
- Manual: Drain traffic via admin interface before restart

**Without graceful drain:** User requests timeout, session data lost, shopping carts disappear. With drain: users complete transactions, sessions preserved.

## TLS/SSL Termination

Decrypting TLS is CPU-intensive. LBs can terminate TLS (decrypt) at edge, communicate with backends over plain HTTP (trusted internal network).

**Topology:**
```
client --[TLS]--> LB --[HTTP]--> backend
         (encrypted)    (plain, internal network)
```

**Pros:**
- Backends don't need TLS computation (CPU freed for application logic)
- Single certificate management (all certificates on LB)
- LB can inspect application layer (TLS termination enables L7 routing)

**Cons:**
- LB CPU cost (TLS is expensive)
- Internal network not encrypted (acceptable on trusted internal networks; unacceptable on untrusted)

## Global Load Balancing (Geo-Distribution)

Scaling beyond a single data center.

### DNS Failover

DNS records point to multiple data center IPs. If one DC fails, remove its IP from A record.

**Tradeoff:** DNS TTL (time to live) determines failover speed. TTL=300s means up to 5 minutes before clients use new IP.

### Anycast Routing

Same IP advertised from multiple geographic locations. Internet routing (BGP) sends traffic to nearest location.

**Tradeoff:** Requires ISP cooperation, complex routing changes.

### GeoDNS

DNS response varies by client location. Query from EU gets EU data center IP; query from AP gets AP data center IP.

**Pros:**
- Low latency (users reach nearest DC)
- Fast failover (DNS change immediate to new clients)

**Cons:**
- Cross-region replication needed (data consistency across regions)

## Consistent Hashing and Scaling Load Balancers

**Problem with simple hash:** If backend count changes (scale from 3 to 4 backends), `hash(key) % N` completely reshuffles assignments. Most cached data misses.

**Consistent hashing solution:** Use ring topology where backends occupy ranges. Adding/removing backend only affects nearby ranges.

```
Ring: [0 ---- backend1 ---- backend2 ---- backend3 ---- 360]
hash(key) finds position on ring; traverses clockwise to next backend.
Adding backend4 between 1 and 2 only remaps keys between them.
```

**Cost:** More complex but scales much better during backend churn.

## Load Balancer Topologies

### Non-HA: Single LB

Single load balancer is a single point of failure (LB fails, all traffic stops).

### HA: Active-Standby

Two load balancers, primary and backup. Standby watches primary; if primary fails, standby takes over (via floating IP or DNS failover).

**Cost:** 2× LB infrastructure. Standby sits idle (wasted capacity).

### HA: Active-Active

Multiple load balancers all actively accepting traffic (via Anycast, DNS round-robin, or client-side discovery).

**Benefit:** Full capacity utilization.

**Challenge:** LB synchronization (if session state is stored on LB, must be replicated).

## Common LB Implementations

**HAProxy (open source):** L4 and L7, stateful, wide ecosystem. CPU-efficient.

**NGINX (open source):** L7 primarily, high-performance, simple configuration.

**Envoy (open source/CNCF):** L4/L7, service mesh integration, fine-grained observability.

**Cloud-managed:**
- **AWS NLB:** L4, extreme throughput (millions RPS)
- **AWS ALB:** L7, HTTP/HTTPS, content-based routing
- **GCP Load Balancer:** L4 (NLB) and L7 (ALB) equivalents
- **Azure Load Balancer:** L4 and L7 variants

See also: **infrastructure-load-balancing.md**, **networking-load-balancing-algorithms.md**, **distributed-consistent-hashing.md**, **infrastructure-api-gateway-patterns.md**.