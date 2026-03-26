# API Gateway Patterns

## Core Responsibilities

An API gateway is the single entry point for all client requests. It handles cross-cutting concerns so services don't have to.

```
Clients (Web, Mobile, IoT)
         │
         ▼
┌─────────────────────────────┐
│       API Gateway           │
│  ┌─────────────────────┐    │
│  │ Authentication      │    │
│  │ Rate Limiting       │    │
│  │ Request Routing     │    │
│  │ Load Balancing      │    │
│  │ Request Transform   │    │
│  │ Response Aggregation│    │
│  │ Caching             │    │
│  │ Logging & Metrics   │    │
│  └─────────────────────┘    │
└──────────┬──────────────────┘
           │
    ┌──────┼──────┐
    ▼      ▼      ▼
 Service  Service  Service
    A       B       C
```

## Routing

### Path-Based Routing

```
/api/orders/*    → Order Service
/api/products/*  → Catalog Service
/api/users/*     → User Service
```

### Header-Based Routing

```
X-API-Version: 2  → Service v2
X-Region: eu      → EU cluster
```

### Canary / Weighted Routing

```
/api/orders → 95% to v1, 5% to v2  (canary deployment)
```

## Authentication and Authorization

The gateway validates tokens so services don't each need to implement auth:

```
Client → Gateway → Validate JWT → Extract claims → Add X-User-Id header → Service
                     │
                     ├── 401 if token expired/invalid
                     └── 403 if insufficient scope
```

### Token Validation Patterns

| Pattern              | How                                            | When                                                 |
| -------------------- | ---------------------------------------------- | ---------------------------------------------------- |
| JWT local validation | Verify signature with public key, check expiry | Most common, no auth service call needed             |
| Token introspection  | Call auth server to validate opaque token      | When tokens can be revoked immediately               |
| OAuth2 passthrough   | Forward token to service, service validates    | When services need to make downstream auth decisions |

### mTLS Between Gateway and Services

```
Client ──(TLS)──→ Gateway ──(mTLS)──→ Services
                   │
            Terminates client TLS,
            initiates mTLS to backend
```

Services accept connections only from the gateway's certificate. Prevents bypassing the gateway.

## Rate Limiting at the Gateway

### Strategies

| Strategy  | Scope                     | Example                                   |
| --------- | ------------------------- | ----------------------------------------- |
| Per-user  | By API key or JWT subject | 100 req/min per user                      |
| Per-IP    | By source IP              | 1000 req/min per IP                       |
| Per-route | By endpoint               | /search: 30 req/min, /orders: 100 req/min |
| Per-plan  | By subscription tier      | Free: 100/day, Pro: 10000/day             |
| Global    | Across all clients        | 50000 req/min total                       |

### Response Headers

```http
HTTP/1.1 200 OK
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 42
X-RateLimit-Reset: 1706000060

# When exceeded:
HTTP/1.1 429 Too Many Requests
Retry-After: 30
```

### Distributed Rate Limiting

Single-instance counters don't work with multiple gateway instances. Solutions:

- **Redis-based**: Atomic counters in Redis (INCR + EXPIRE). Sub-millisecond overhead.
- **Sliding window in Redis**: Sorted set with timestamps. More precise, higher memory.
- **Local + sync**: Each gateway tracks locally, syncs periodically. Less precise, no Redis dependency.

## Request Transformation

### Request Rewriting

```
Client sends: POST /api/v2/orders
Gateway rewrites to: POST /internal/order-service/create
Adds headers: X-Request-Id, X-Forwarded-For, X-User-Id
Strips headers: Authorization (replaced with internal token)
```

### Request Validation

Validate request schema at the gateway to reject bad requests early:

```yaml
# Kong plugin example
plugins:
  - name: request-validator
    config:
      body_schema: |
        {
          "type": "object",
          "required": ["items", "shipping_address"],
          "properties": {
            "items": { "type": "array", "minItems": 1 }
          }
        }
```

## Response Aggregation

Combine responses from multiple services into one client response:

```
Client: GET /api/product-page/123

Gateway:
  → GET /catalog/products/123     → { name, description, price }
  → GET /reviews/products/123     → { reviews: [...], avg_rating }
  → GET /inventory/products/123   → { in_stock: true, quantity: 42 }

Response: { product: {...}, reviews: {...}, inventory: {...} }
```

### GraphQL Gateway

Gateway exposes a GraphQL API, resolves fields from different services:

```graphql
type Product {
  id: ID!
  name: String! # From Catalog Service
  reviews: [Review!]! # From Review Service
  inStock: Boolean! # From Inventory Service
}
```

Implementations: Apollo Federation, GraphQL Mesh. The gateway stitches subgraphs into a unified API.

## Backend for Frontend (BFF)

Separate gateways per client type, each optimized for its client's needs:

```
Web App  → Web BFF    → Services
Mobile   → Mobile BFF → Services
IoT      → IoT BFF    → Services
```

| BFF        | Optimizations                                          |
| ---------- | ------------------------------------------------------ |
| Web BFF    | Full payloads, pagination, complex queries             |
| Mobile BFF | Compressed responses, minimal fields, offline-friendly |
| IoT BFF    | Tiny payloads, binary protocols, batch reporting       |

**When to use BFF**: When client needs diverge significantly. Not needed if web and mobile need the same data in the same format.

## API Versioning Through Gateway

| Strategy        | Implementation                            | Pros/Cons                         |
| --------------- | ----------------------------------------- | --------------------------------- |
| URL path        | `/v1/orders`, `/v2/orders`                | Simple, explicit. Clutters URLs.  |
| Header          | `Accept: application/vnd.api.v2+json`     | Clean URLs. Harder to test/share. |
| Query param     | `/orders?version=2`                       | Easy to test. Non-standard.       |
| Gateway routing | Gateway routes to correct service version | Services unaware of versioning.   |

### Gateway Version Routing

```
/api/v1/orders → order-service-v1.internal
/api/v2/orders → order-service-v2.internal

# Or with header:
X-API-Version: 1 → order-service-v1.internal
X-API-Version: 2 → order-service-v2.internal (default)
```

## Protocol Translation

Gateway handles protocol differences between clients and services:

```
Client (REST/HTTP) → Gateway → Service (gRPC)
Client (WebSocket) → Gateway → Service (HTTP/SSE)
Client (REST)      → Gateway → Service (GraphQL)
```

gRPC-Web pattern: Browser clients can't use gRPC directly. Gateway translates REST/gRPC-Web to native gRPC.

## Caching at the Gateway

| Pattern               | When to Use                              | Implementation                    |
| --------------------- | ---------------------------------------- | --------------------------------- |
| Response caching      | Identical requests within TTL            | Redis, Varnish, CDN               |
| Request deduplication | Multiple simultaneous identical requests | Collapse into one backend call    |
| Conditional GET       | Client sends ETag/Last-Modified          | Gateway returns 304 if unchanged  |
| Cache invalidation    | Backend data changes                     | Purge by tag/pattern or short TTL |

## Gateway Implementations Comparison

| Gateway             | Type            | Strengths                                           | Weaknesses                                      |
| ------------------- | --------------- | --------------------------------------------------- | ----------------------------------------------- |
| **Kong**            | API Gateway     | Plugin ecosystem, declarative config, DB-backed     | Memory footprint, learning curve                |
| **AWS API Gateway** | Managed         | Zero ops, Lambda integration, WebSocket support     | Cold starts, vendor lock-in, limited transforms |
| **Nginx**           | Reverse proxy + | High performance, battle-tested, flexible config    | Limited API management features OOB             |
| **Envoy**           | Service proxy   | Dynamic config (xDS), observability, gRPC-native    | Complex config, primarily infrastructure-level  |
| **Traefik**         | Cloud-native    | Auto-discovery (K8s, Docker), Let's Encrypt, simple | Less mature plugin ecosystem                    |
| **Apisix**          | API Gateway     | High performance (nginx+Lua), dynamic, dashboard    | Smaller community than Kong                     |

### Decision Framework

- **Managed cloud**: AWS API Gateway + Lambda, GCP API Gateway, Azure API Management
- **Kubernetes-native**: Envoy (via Istio/Ambassador), Traefik, Nginx Ingress + Kong
- **Plugin-heavy**: Kong (richest plugin ecosystem), Apisix
- **High-performance proxy**: Envoy, Nginx

## Gateway vs Service Mesh

| Concern       | API Gateway                          | Service Mesh                 |
| ------------- | ------------------------------------ | ---------------------------- |
| Position      | Edge (north-south traffic)           | Between services (east-west) |
| Auth          | External client auth (JWT, API keys) | mTLS between services        |
| Rate limiting | Per-client/per-route                 | Per-service-pair             |
| Routing       | Client→service                       | Service→service              |
| Protocol      | REST, GraphQL, WebSocket             | gRPC, HTTP, TCP              |
| Managed by    | API team                             | Platform team                |

**Use both**: Gateway at the edge for external clients; service mesh internally for service-to-service comms. They solve different problems.

## Observability

### Access Logs

Every request through the gateway logged with:

```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "method": "POST",
  "path": "/api/orders",
  "status": 201,
  "latency_ms": 145,
  "client_ip": "203.0.113.42",
  "user_id": "user-789",
  "request_id": "req-abc-123",
  "upstream": "order-service",
  "upstream_latency_ms": 130
}
```

### Distributed Tracing Propagation

Gateway generates or propagates trace context:

```
Client → Gateway (generates trace-id: abc, span: gateway)
  → Service A (propagates trace-id: abc, parent-span: gateway)
    → Service B (propagates trace-id: abc, parent-span: service-a)
```

Headers: W3C Trace Context (`traceparent`, `tracestate`) is the standard. Legacy: Zipkin B3, Jaeger uber-trace-id.

### Security: CORS

```yaml
# Kong CORS plugin
plugins:
  - name: cors
    config:
      origins: ["https://app.example.com"]
      methods: ["GET", "POST", "PUT", "DELETE"]
      headers: ["Authorization", "Content-Type"]
      credentials: true
      max_age: 3600
```

**Setting `origins: ["*"]` with `credentials: true`** is a security vulnerability. Specific origins should be whitelisted.
