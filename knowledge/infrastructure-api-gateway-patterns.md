# API Gateway Patterns — Kong, Envoy, Rate Limiting, Authentication & Request Transformation

## Overview

An API gateway is the single entry point for all client traffic to backend services. It decouples clients from service topology changes, centralizes cross-cutting concerns (authentication, rate limiting, logging), and simplifies load balancing and canary deployments. The gateway operates at the edge: it sits between clients and services, forwarding requests with optional transformations, retries, circuit breaking, and caching.

## Core Gateway Responsibilities

**Request Routing:** Map incoming requests to backend services based on path, hostname, method, headers, or custom rules. Enable path rewriting, virtual hosting, and prefix matching without changing downstream services.

**Cross-Cutting Concerns:** Apply authentication, authorization, rate limiting, logging, and request/response transformation uniformly across all services instead of replicating in each one.

**Traffic Control:** Implement circuit breakers, bulkheads, timeouts, and retry logic to protect services from cascading failures.

**Caching & Aggregation:** Cache responses, batch requests to backends, reduce downstream load.

**Protocol Translation:** Accept HTTP/REST from clients, forward to gRPC or other protocols; translate responses back.

**Load Balancing:** Distribute traffic across backend instances; integrate with service discovery.

**Observability:** Centralized logging, tracing, and metrics collection.

## Kong (Konghq)

Kong is a modern, open-source API gateway and service mesh built on Nginx, with a rich ecosystem of plugins.

### Architecture

Written in Lua (extending Nginx), Kong processes requests through a pipeline:
1. Client connects → load balancer selects Kong node
2. Request matches a **Route** (path, host, method conditions)
3. Route points to an **Upstream** (backend service)
4. Request passes through plugins in order (auth, rate-limit, logging, etc.)
5. Request forwarded to upstream service
6. Response passes reverse through plugins
7. Response returned to client

Kong separates the **Control Plane** (Admin API for configuration) from the **Data Plane** (router nodes that forward traffic). This allows scaling router nodes independently and enables declarative configuration management.

### Core Concepts

**Services:** Logical upstream services (your microservices).

**Routes:** Conditions that match incoming requests to services (e.g., `/users/*` → user-service).

**Upstreams:** Load balancer targets (sets of backend nodes); Kong supports active/passive health checks.

**Plugins:** Modular code that runs at specific request/response lifecycle points. Kong includes 100+ plugins (auth, rate-limit, jwt, cors, prometheus, etc.); custom Lua plugins are supported.

**Consumers:** Users/apps that authenticate with the gateway (used with auth plugins).

### Notable Plugins

**Rate Limiting:** `rate-limiting` plugin throttles requests by consumer or IP; configurable per-minute/hour/day. Redis backend for distributed rate limiting across multiple Kong nodes.

**Authentication:** `basic-auth`, `key-auth` (API keys), `jwt` (JSON Web Tokens), `oauth2` (full OAuth 2 flow), `ldap`.

**Transformation:** `request-transformer` (add/remove headers, query params, body modification), `response-transformer`, `body-size-limiting`.

**Circuit Breaker:** `upstream-healthchecks` (active and passive health checks); routes around unhealthy upstreams.

**Caching:** `proxy-cache` plugin caches responses by status code, content-type, and request properties; respects Cache-Control headers.

**Logging:** Plugins for request/response logging to files, syslog, HTTP endpoints, or analytics platforms.

**CORS:** `cors` plugin enforces cross-origin rules.

**Service Mesh Integration:** Kong mesh (separate product) extends gateway capabilities with mTLS, traffic shifting, and observability.

### Strengths

- Declarative REST API for configuration (`curl`-friendly)
- Rich plugin ecosystem; solves 90% of gateway use cases without custom code
- Horizontal scaling (stateless data plane, separate control plane)
- Active development, large community
- Works on-premises or in clouds
- Good fit for organizations moving from monolith to microservices

### Weaknesses

- Operational complexity; managing Kong cluster, database, Redis for distributed features
- Plugin ecosystem quality varies; some are less mature
- Not as deep as Envoy for advanced traffic shaping
- Kong 3.x requires external database (PostgreSQL) for HA; earlier versions used in-memory config (simpler)
- Troubleshooting complex plugin chains can be opaque

### Use Cases

Multi-service architectures requiring centralized auth/rate-limit, SaaS platforms with multi-tenant rate limiting, organizations adopting microservices incrementally.

## AWS API Gateway

AWS's managed service for hosting APIs, with tight integration to Lambda, AWS services, and IAM.

### Core Features

**REST API:** Traditional request-response; maps methods/paths to resources and integration backends (Lambda, HTTP, AWS services).

**HTTP API:** Lighter-weight alternative to REST API; newer, cheaper, fewer features; suitable for simple routing.

**WebSocket API:** Real-time bidirectional communication; useful for chat, live dashboards, collaborative apps.

**Integrations:** Connect API paths to Lambda functions, HTTP endpoints, AWS Step Functions, SQS queues, kinesis, SNS topics, DynamoDB, etc.

**Authentication & Authorization:**
- **IAM:** AWS identity and access control; fine-grained permissions
- **Cognito:** User pools (authentication) and identity pools (authorization)
- **Custom Authorizers:** Lambda function that validates Bearer tokens and returns IAM policy

**Rate Limiting & Throttling:** Per-method throttling (requests/second, concurrent connections); burst capacity.

**Request/Response Transformation:** Mapping templates (Velocity Template Language) to transform input/output; add/remove headers, reformat JSON, etc.

**Caching:** Response caching per method/path; cache invalidation via cache key parameters; size limits.

**Logging & Monitoring:** CloudWatch integration; logs request/response, execution time, errors; X-Ray tracing.

**Deployment Stages:** Separate environments (dev, test, prod) with independent throttling, auth, logging config.

### Request/Response Flow

1. Request arrives at API Gateway
2. API Gateway checks throttling limits
3. Authorizer runs (if configured) to validate credentials
4. Request transformation mapping applied (URL encoding, header/body modification)
5. Request forwarded to integration backend (Lambda, HTTP, etc.)
6. Response mapping applied (format change, status code override)
7. Response returned with rate-limit headers

### Strengths

- **Zero Operational Overhead:** Fully managed; AWS handles scaling, HA, patching
- **Native AWS Integration:** Direct access to Lambda, Cognito, IAM, DynamoDB, S3
- **IAM Integration:** Leverage existing AWS identity practices
- **Pay-per-Use:** Billing per API call; cost-effective for variable traffic
- **Developer-Friendly:** Console, SAM, CloudFormation, Terraform support
- **Built-in Observability:** CloudWatch logs, X-Ray tracing without additional setup

### Weaknesses

- **Vendor Lock-In:** Difficult to move away from AWS; mappings are AWS-specific VTL
- **Cold Starts:** Lambda integration adds cold-start latency (mitigated by provisioned concurrency)
- **Limited Routing Logic:** Path-based routing is basic; complex content-based routing requires custom code
- **Cost at Scale:** Per-request billing can exceed self-hosted solutions at very high volumes
- **Regional Deployment:** API Gateway is regional; multi-region requires manual setup or Route 53 failover
- **Transformation Language:** VTL is proprietary; not portable

### Use Cases

Serverless applications (Lambda-native APIs), organizations deeply invested in AWS, startups prioritizing reduced ops overhead, ISVs with variable traffic patterns.

## Envoy (Lyft/Cloud Native Computing Foundation)

A modern proxy designed for APIs and service-to-service communication, built into the Istio service mesh.

### Architecture

Envoy is a **sidecar proxy** running alongside each service instance. Services communicate through their local Envoy proxy, which handles routing, load balancing, circuit breaking, and observability. Unlike Kong (reverse proxy acting as single entry point), Envoy is distributed.

**Data Plane:** Envoy proxy instances forward traffic based on configuration.

**Control Plane:** System (Istio, Consul, etc.) instructs proxies what routing rules, rate limits, and circuit breakers to apply.

Configuration is declarative and pushed to proxies; no restart required.

### Core Concepts

**Listeners:** Envoy listens on a port/address and accepts connections.

**Routes:** Match incoming requests by headers, paths, methods; forward to clusters.

**Clusters:** Named groups of endpoints (upstream services). Envoy health-checks cluster members and load-balances across healthy ones.

**Endpoints:** Individual backend service instances.

**Load Balancing:** Round-robin (default), least-request, ring-hash (consistent hashing), random, etc. Per-request load balancing (queries external service for decision).

**Outlier Detection:** Remove unhealthy instances from the pool based on error rate, latency, or health check failures.

### Traffic Control

**Circuit Breaking:** Limit connections/requests to upstreams; eject hosts that exceed failure thresholds.

**Retries:** Automatic retry with exponential backoff; disable on certain status codes.

**Timeouts:** Per-route timeouts; local and end-to-end.

**Rate Limiting:** Token bucket algorithm; local or redis-backed for distributed rate limiting.

**Traffic Shifting:** Shift percentage of traffic to canary versions (e.g., 95% stable, 5% canary).

**Weighted Load Balancing:** Send different weights of traffic to different clusters; useful for gradual deployments.

### Observability

Envoy emits rich metrics (HTTP status codes, latencies, upstream errors) and access logs. Integration with Prometheus, Jaeger (tracing), and log aggregation systems is straightforward. Envoy is designed to be observable; extensive debugging endpoints.

### Strengths

- **Protocol Support:** HTTP/1.1, HTTP/2, HTTP/3, gRPC, WebSockets, TCP/UDP
- **Performance:** Low-latency, minimal cpu/memory overhead
- **Traffic Control Sophistication:** Advanced load balancing, canary deployments, traffic splitting
- **Distributed Model:** No single point of failure; scales horizontally
- **Observability:** Built for monitoring and debugging; generates detailed metrics and access logs
- **Service Mesh Integration:** Native component of Istio, Consul Connect; can be used standalone
- **Maturity:** Battle-tested at large scale (Lyft, Uber, AWS, Google)

### Weaknesses

- **Operational Complexity:** Requires external control plane (Istio, Consul) to be useful
- **Resource Overhead:** Sidecar model means one proxy per service instance; higher resource footprint than single centralized gateway
- **Learning Curve:** Configuration is verbose and complex; requires deep understanding of routing concepts
- **Istio Complexity:** Service mesh adds operational overhead (CRDs, control plane, debugging)
- **Not a Drop-In Gateway:** Designed for service-to-service; requires service mesh architecture

### Use Cases

Large microservices with strict SLOs, organizations adopting service mesh (Istio, Linkerd), advanced traffic shifting requirements (canaries, A/B tests), teams comfortable with distributed systems complexity.

## KrakenD

A high-performance, open-source API gateway focused on **request aggregation** and **response caching**.

### Core Capability

KrakenD's primary strength is aggregating multiple backend calls into a single response. Rather than clients making N calls to N services, the client makes one call to the gateway, which orchestrates backend calls, merges responses, and returns unified data.

### Features

**Request Aggregation:** Single gateway endpoint can call multiple backends in parallel, combine responses, filter fields before returning to client.

**Response Caching:** Cache responses with TTL; perfect for read-heavy APIs.

**Rate Limiting:** Per-endpoint or per-consumer; integrates with Redis for distributed limits.

**Authentication:** JWT validation, OAuth 2 client credentials.

**Request/Response Transformation:** Header/body modification; field filtering.

**Circuit Breaking & Retries:** Automatic resilience handling.

**Middleware System:** Custom logic via JavaScript or Go plugins.

**Declarative Config:** YAML/JSON configuration; no database needed.

### Example: Request Aggregation

Client requests `/api/user-profile/:id` from the gateway. The gateway internally:
1. Calls `/api/users/:id` from user-service
2. Calls `/api/preferences/:id` from preferences-service (in parallel)
3. Calls `/api/recommendations/:id` from recommendations-service (in parallel)
4. Merges the three responses
5. Returns combined profile data to client

Without the gateway, the client would make 3 separate calls.

### Strengths

- **Simple Deployment:** Single binary, no external dependencies
- **Request Aggregation:** Unique strength; reduces client chattiness
- **Fast:** Minimal overhead, high throughput
- **Stateless:** Easy to scale horizontally
- **Good for Resource-Constrained Environments:** Mobile clients, IoT with limited bandwidth

### Weaknesses

- **Limited Traffic Control:** No advanced canary routing or weighted load balancing
- **Smaller Ecosystem:** Fewer integrations than Kong, less mature than Envoy
- **Field Filtering:** Limited compared to GraphQL for complex data transformations
- **Not a Full Service Mesh:** Lacks mTLS, advanced observability

### Use Cases

Mobile APIs requiring aggregated responses, gateway caching layer for read-heavy systems, organizations looking for simplicity over feature richness.

## Authentication & Authorization at the Gateway

### Pattern 1: OAuth 2 / OpenID Connect

Client obtains a token from an authorization server, includes token in API requests (`Authorization: Bearer <token>`). Gateway validates token signature (or checks with auth server), verifies scopes, and forwards request if authorized.

**Strengths:** Industry standard, widely supported, separates auth logic from services, token refresh without reauth.

**Weaknesses:** Adds OAuth provider as a dependency; token validation latency (mitigated by caching/validating locally).

### Pattern 2: API Keys

Client includes API key in header or query parameter; gateway looks up key, verifies permissions, forwards request.

**Strengths:** Simple, no external auth provider needed, suitable for service-to-service or machine clients.

**Weaknesses:** Keys are static; rotation is manual; no built-in scoping; compromise of one key compromises all requests made with that key.

### Pattern 3: mTLS (Mutual TLS)

Both client and gateway present certificates; gateway validates client certificate and extracts identity from certificate fields (CN, SAN, etc.).

**Strengths:** Strong mutual authentication; integrates with service mesh; works for both user and service identities.

**Weaknesses:** Requires PKI infrastructure; certificate management is operational overhead; client libraries must support client certificates.

### Pattern 4: Custom Authorizer (AWS API Gateway)

Lambda function receives request details (headers, context) and returns IAM policy (allow/deny specific resources). Enables fine-grained, dynamic authorization based on custom logic.

**Strengths:** Fully customizable; integrates with AWS services; enables feature flags, rate limiting based on user properties.

**Weaknesses:** Lambda invocation adds latency; cost per authorization check.

## Rate Limiting Strategies

**Per-Consumer:** Limit by API key, OAuth subject, or IP address. Different consumers have different limits (free tier: 100 req/min, premium: 10k req/min).

**Per-Endpoint:** Different limits for different paths (strict on expensive operations, lenient on fast queries).

**Distributed Rate Limiting:** When gateway is scaled horizontally, rate limit state must be shared (Redis, Memcached). Each gateway increments a counter in shared state; enforcing global limit across all gateway instances.

**Token Bucket:** Grant N tokens per interval; each request consumes 1 token. When tokens run out, requests are rejected or queued. Supports burst traffic (accumulated tokens).

**Sliding Window:** Track request times in a time window; reject if > N requests in the last minute. More precise than token bucket but higher memory overhead.

**Circuit Breaker-aware Rate Limiting:** When backends are degraded/circuit open, reduce incoming traffic (shed load) rather than queue requests.

## Request Transformation Examples

**Header Injection:** Gateway adds `X-Request-ID`, `X-Forwarded-For`, `X-Real-IP` headers for tracing, access logs, and identity propagation.

**Path Rewriting:** Client requests `/api/v1/users`; gateway rewrites to `/v2/users` before forwarding (enabling gradual API version migration).

**Body Serialization:** Client sends JSON; gateway converts to protobuf before forwarding to a service expecting protobuf.

**Authentication Header Translation:** Client sends OAuth Bearer token; gateway exchanges for service-internal token before forwarding.

**Compression:** Client indicates support for gzip; gateway compresses response downstream, decompresses for client.

## Response Caching

**Objectives:** Reduce downstream load, decrease client latency, improve availability when backends are slow.

**Strategies:**
- **HTTP Caching Headers:** Cache-Control: max-age=3600 indicates cache duration; gateway respects this.
- **Explicit Bypass:** Cache: no-store in response tells gateway not to cache.
- **Per-Status Caching:** Cache 200 responses, don't cache 4xx/5xx.
- **Cache Invalidation:** Set TTL, or add endpoints to manually invalidate caches (POST /cache/invalidate).

**Distributed Caching:** Use Redis/Memcached for shared cache across multiple gateway instances.

## Canary Deployments & Traffic Shifting

### Blue-Green Deployment via Gateway

1. Deploy new version (green) alongside current version (blue)
2. Gateway routes 0% to green, 100% to blue
3. Health checks verify green is healthy
4. Gateway shifts traffic: 5% to green, 95% to blue
5. Monitor metrics (error rate, latency); if regression detected, shift back to 0%
6. Gradually increase: 25%, 50%, 100%
7. Once at 100%, decommission blue

The gateway is the control point; no client changes required.

## Trade-Offs & Selection Guide

| **Gateway** | **Deployment** | **Setup** | **Scalability** | **Routing Logic** | **Best For** |
|---|---|---|---|---|---|
| **Kong** | Centralized | Medium | High | Good | Traditional microservices, multi-tenant SaaS |
| **AWS API Gateway** | Managed | Low | Very High | Basic | Serverless, AWS-native, low ops |
| **Envoy** | Distributed (sidecar) | High | Very High | Excellent | Large-scale distributed systems, service mesh |
| **KrakenD** | Centralized | Low | High | Good | Request aggregation, caching, simple routing |

## See Also

- [API Security](security-api.md) — Authentication, authorization, and protection mechanisms
- [Software-Defined Networking (SDN)](infrastructure-networking-sdn.md) — Gateway as a control plane concept
- [Microservices Architecture](architecture-microservices.md) — Gateway role in distributed systems
- [Service Discovery](infrastructure-service-discovery.md) — Gateway integration with service registries
- [Request/Response Caching Patterns](architecture-patterns.md) — Caching strategies