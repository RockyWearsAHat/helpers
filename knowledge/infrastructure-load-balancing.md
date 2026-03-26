# Load Balancing — Algorithms, Architectures, and Deployment Models

## L4 vs L7 Load Balancing

The OSI model layer determines what data the load balancer inspects.

### L4 (Transport Layer) Load Balancing

L4 load balancers operate on **IP + Port** (TCP/UDP headers). Forward traffic without inspecting application payload.

**Decision basis**:
- Source IP, destination IP
- Source port, destination port
- Protocol (TCP, UDP)

**Common algorithms**:
- Round-robin
- Least connections
- IP hash (consistent routing based on source IP)

**Implementation**:
1. Client connects to LB virtual IP (VIP)
2. LB rewrites packet headers (destination IP/port → backend)
3. Backend receives request, responds to LB (or directly to client via DSR)
4. LB rewrites response headers if needed

**Advantages**:
- Low latency (minimal packet inspection)
- High throughput (can be hardware-implemented)
- Works with any protocol (TCP, UDP, SSH, SMTP, etc.)
- Connectionless algorithms viable (can switch backends mid-stream)

**Disadvantages**:
- Can't make intelligent decisions based on application state
- Can't see HTTP headers, URLs, or request content
- May distribute unequally if clients vary in request size

**Examples**: Linux IPVS, HAProxy in L4 mode, F5 GTM, AWS Network Load Balancer (NLB)

### L7 (Application Layer) Load Balancing

L7 load balancers parse the **application protocol** (HTTP headers, URLs, cookies, JSON content).

**Decision basis**:
- HTTP method, URL path, query parameters
- Host header, user-agent header, custom headers
- Cookie values, JWT claims
- Request body (for JSON APIs)

**Example routing rules**:
```
if path starts with /api/v1:
    route to api-backend
if host is images.example.com:
    route to image-server
if cookie[session_id] matches pattern:
    route to session-affinity backend
if body.user_tier == "premium":
    route to premium backend
```

**Advantages**:
- Can make intelligent decisions based on request content
- Content-based routing (different backends for different URLs)
- Session affinity by cookie/JWT
- Can modify requests (rewrite URLs, inject headers)

**Disadvantages**:
- Higher latency (must parse HTTP, may buffer body)
- Lower throughput (more CPU per request)
- Complexity (requires HTTP knowledge)
- Must buffer entire request (potentially large uploads)

**Examples**: Linux LVS with L7 modules, HAProxy, NGINX, Envoy proxy, AWS Application Load Balancer (ALB)

## Load Balancing Algorithms

### Round-Robin

Distribute requests in circular order to all backends:

```
request 1 → backend 1
request 2 → backend 2
request 3 → backend 3
request 4 → backend 1 (cycle)
```

**Cost**: O(1) per request
**Best for**: Backends of uniform capacity, homogeneous workload
**Issues**: If one backend is slower (GC pause, cache miss), requests still routed to it

### Least Connections

Route to backend with **fewest active connections**:

```
backend 1: 5 connections
backend 2: 12 connections
backend 3: 3 connections
New request → backend 3
```

Favors backends that finish requests quickly.

**Cost**: O(n backends) to find minimum, or O(log n) with priority queue
**Best for**: Long-lived connections (WebSockets, SSH); variable processing time
**Issues**: New connections expensive; doesn't account for request size

### Weighted Round-Robin

Each backend has a **weight** (e.g., capacity in proportion to CPU cores):

```
backend 1: weight 3
backend 2: weight 1

Distribution (over 4 requests):
backend 1 gets 3 requests
backend 2 gets 1 request
```

**Use case**: Multi-tier infrastructure (newer servers weight 3, older servers weight 1).

### IP Hash (Source IP Affinity)

Hash source IP to consistently route to same backend:

```
hash(client_ip) % num_backends → backend index
```

All requests from same client always go to same backend.

**Benefit**: **Session affinity** without storing state on LB. If backend stores session in-process, client requests find their session.

**Cost**: O(1)

**Downside**:
- If backend fails, client's traffic hashes to different backend (cold cache, session lost)
- If backend added/removed, rebalancing affects all existing clients

### Consistent Hashing

Mitigate IP hash's rebalancing problem with a **ring structure**:

```
Conceptual ring (0 to 2^64):

    backend_1 (hash 100)
        ↑
ring:  |
    backend_3 (hash 50)
        ↑
    backend_2 (hash 20)

Client IP hashes to ring position; walk clockwise to find backend.
```

If backend_1 added/removed, only ~1/n of clients are affected (not all).

**Example**: Memcached clients, Cassandra peer selection, sharded databases.

### Maglev: Fast Consistent Hashing (Google)

Consistent hashing is deterministic but not cache-optimal (same position might round-robin through backends). Maglev computes a **lookup table** that minimizes backend switching:

```
lookup_table[256] = {1, 1, 2, 1, 3, 2, 1, 2, 1, ...}
hash(client_ip) % 256 → lookup_table[result] → backend
```

Maglev ensures that moving one backend affects only ~1/n clients and minimizes "churn" (unneeded rebalancing).

Used by Google Cloud Load Balancer, Kubernetes service endpoints.

### Least Loaded / Latency-Based

Query backends for current **load** (CPU, memory, latency):

```
backend 1: CPU 80%, latency p99=50ms
backend 2: CPU 30%, latency p99=10ms
New request → backend 2
```

**Requires**: Backend monitoring (metrics API or out-of-band checks).
**Benefit**: Adapts to runtime conditions; no static assumptions.
**Cost**: O(n backends), plus monitoring latency (stale data possible).

## Session Affinity (Sticky Sessions)

Many applications store state in-process (HTTP sessions, WebSocket state). Requests from **same client must return to same backend**.

### Cookie-Based Affinity

```
response includes: Set-Cookie: SERVERID=backend_2

LB rule: if cookie SERVERID=backend_2, route to backend_2
```

LB inspects cookie in each request, routes accordingly. If backend_2 fails, LB can failover (cookie becomes stale; next request has no valid cookie).

**Trade-off**: Client stickiness vs redistribution on failure.

### IP-Based Affinity

Hash source IP to determine affinity (no cookie needed). All requests from same IP go to same backend.

**Downside**: No graceful failover (hash changes if backend removed).

### Sticky Timeout

Associate client with backend for a **time window** (e.g., 5 minutes). If client doesn't send request within timeout, affinity expires.

**Use**: Gradual backend drains (stop accepting new connections to backend, drain existing clients over timeout).

## Health Checks

Load balancers periodically check backend health to remove failed servers.

### TCP Health Check

```
LB tries: connect to backend:port
If connected within timeout: HEALTHY
Else: UNHEALTHY
```

Simple, fast. Doesn't verify application is working (port open ≠ app healthy).

### HTTP Health Check

```
GET /health HTTP/1.1
Host: backend:port

If response 2xx: HEALTHY
Else: UNHEALTHY
```

Application can return 200 OK only if all critical dependencies (DB, cache) are healthy. More accurate than TCP.

**Graceful shutdown**:
```
1. Handler returns 503 Service Unavailable
2. LB detects 503, marks unhealthy
3. LB stops routing new requests to backend
4. LB waits for in-flight requests to finish (connection draining)
5. Backend process exits
```

### Health Check Frequency

Too frequent: Overhead on LB and backends, false positives (transient failures).
Too infrequent: Slow to detect failures (gap = failover latency).

Typical: Every 5-10 seconds, 2-3 failures before marking unhealthy.

## SSL/TLS Termination

HTTPS traffic can be terminated at the **LB** or **backend**.

### TLS Termination at LB

```
Client ←HTTPS→ LB ←HTTP→ Backend
```

LB decrypts, inspects request, reencrypts to backend (or forwards in plaintext if backend is trusted).

**Advantages**:
- CPU cost of TLS on LB (not backends); backends can be simpler
- LB can decrypt and route based on SNI (Server Name Indication), certificate
- Easier certificate management (single cert on LB, not all backends)

**Disadvantage**: Backends can't see client IP directly (unless LB injects X-Forwarded-For header).

### TLS Pass-Through

```
Client ←HTTPS→ LB ←HTTPS→ Backend
```

LB never decrypts; routes on TLS record metadata (SNI, IP). Backend handles TLS.

**Advantages**:
- Lower LB CPU (no crypto)
- Backends see client IP naturally
- End-to-end encryption

**Disadvantage**: Routing logic limited (can't inspect HTTP layer).

## Global Server Load Balancing (GSLB)

GSLB distributes traffic across **data centers in different geographic regions** using DNS + health checks.

**Architecture**:

```
Primary DC (US)
├─ LB
├─ Backends
└─ Health checks → GSLB controller

Secondary DC (EU)
├─ LB
├─ Backends
└─ Health checks → GSLB controller

GSLB Controller
├─ Monitors DC health
├─ Makes DNS decisions
├─ Returns different IPs based on geography/health
```

**Failover example**:
1. Client resolves api.example.com
2. GSLB controller checks health: US DC healthy, EU DC healthy
3. Returns US IP (lower latency for US clients) or EU IP (for EU clients)
4. US DC fails; health check detects failure
5. GSLB updates DNS: stops returning US IP
6. New client queries resolve to EU IP only
7. Existing client connections: DNS cached (TTL), but TCP to US fails
8. Client retries (app-dependent), succeeds connecting to EU

**TTL impact**: Low TTL (60s) → faster failover but higher DNS load. High TTL (3600s) → persistent stale data.

## Hardware vs Software Load Balancers

| Aspect | Hardware | Software |
|---|---|---|
| **Throughput** | 100 Gbps+, line-rate | 10-100 Gbps (bottlenecked by CPU) |
| **Latency** | Sub-microsecond | Milliseconds (context switching) |
| **Cost** | $50K-500K+ | Free to $10K |
| **Flexibility** | Configured via GUI/CLI; hard to change | Full programmability; can modify at runtime |
| **Scaling** | Vertical (bigger box) or cluster | Horizontal (more instances) |
| **Examples** | F5 BIG-IP, Citrix NetScaler, Radcom | HAProxy, NGINX, Envoy, IPVS |

### HAProxy

Mature software L4/L7 load balancer. Can run on commodity hardware.

```
# L4 mode (TCP)
listen web_socket
    bind *:80
    mode tcp
    balance roundrobin
    server web1 10.0.1.1:80
    server web2 10.0.1.2:80

# L7 mode (HTTP)
frontend web_traffic
    bind *:80
    default_backend web_backends

backend web_backends
    balance leastconn
    server web1 10.0.1.1:80
    server web2 10.0.1.2:80 backup
```

### NGINX

Web server with reverse proxy (L7 load balancer) capabilities.

```
upstream backend {
    server 10.0.1.1:80 weight=3;
    server 10.0.1.2:80 weight=1;
    server 10.0.1.3:80 backup;
}

server {
    listen 80;
    location / {
        proxy_pass http://backend;
    }
}
```

### Envoy

Modern, cloud-native proxy. Complex configuration (YAML/protobuf), integrates with service mesh (Istio).

**Capabilities**:
- L4 + L7 load balancing
- Outlier detection (dynamically remove slow backends)
- Rate limiting, circuit breaking
- Distributed tracing integration

## Key Deployment Patterns

**Single LB (single point of failure)**:
```
Client → LB → {backend1, backend2, ...}

Failure: If LB fails, all traffic lost.
```

**Active-active LB pair (via anycast or DNS round-robin)**:
```
Client → {LB1, LB2} [via anycast or DNS RR] → {backend1, backend2, ...}

Failure: If LB1 fails, anycast/DNS routes to LB2. Low failover latency.
```

**LB cluster (Kubernetes, Consul)**:
```
LB1, LB2, LB3 all run on separate nodes
Service mesh assigns requests via Consistent Hashing
If LB1 fails, remaining LBs absorb traffic
```

## See Also

- infrastructure-dns-architecture (DNS-based routing, GeoDNS, GSLB interaction)
- networking-tcp-ip (TCP/IP stacks, packet flow)
- system-design-distributed (distributed system resilience, failover patterns)