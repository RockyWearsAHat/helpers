# CDN and Caching for Scaling — Edge Architecture, Cache Strategies, and Compute at Scale

## Overview

**Caching** and **Content Delivery Networks (CDNs)** are the primary mechanisms for scaling content serving beyond what a single origin server can handle. The principle is simple: store copies of data geographically closer to users, reducing latency and freeing the origin from redundant requests. Modern CDNs extend beyond static files to dynamic content, computation, and real-time data streams.

Caching exists at multiple layers—HTTP caches in browsers, CDN edge nodes, application-level caches (Redis), database query caches—each with different invalidation semantics and operational complexity.

## CDN Architecture: Origin, Edge, and POPs

### Core Topology

**Origin server:** The authoritative source of content (your application backend, S3 bucket, or database). Doesn't serve end users directly; serves the CDN.

**Points of Presence (POPs):** Regional clusters of edge-caching servers, typically deployed near Internet Exchange Points (IXPs) and ISP peering locations. When a user requests content, DNS geo-routing sends them to the nearest POP.

**Edge node:** Individual server in a POP. Caches content and serves hits directly; misses request origin or parent cache tier.

**Origin Shield:** Optional tier between origin and edge servers. On cache miss, edge servers query the shield instead of stampeding the origin. Shield caches result and serves subsequent edge misses. Reduces origin load and improves cache hit ratio at cost of one extra hop.

### Request Flow

```
User → (DNS) → Nearest POP
              → Edge cache HIT: serve directly (1-5ms latency)
              → Edge cache MISS:
                 → Origin Shield cache HIT: fetch, serve from edge (50-100ms)
                 → Origin Shield cache MISS:
                    → Origin: fetch, shield stores, serve through edge (200-500ms)
```

**Hit ratio** directly reduces origin load. Hit ratios above 95% are common; above 99% is excellent (remaining 1% of requests still reach origin, but vastly reduced load).

## Cache Control and HTTP Headers

Caching behavior is governed by HTTP `Cache-Control` headers from origin.

### Directives

| Directive | Purpose |
|-----------|---------|
| `public` | Any cache (browser, CDN, proxy) may store |
| `private` | Browser-only; CDN must not cache |
| `max-age=N` | Fresh for N seconds |
| `s-maxage=N` | Shared cache (CDN) overrides max-age |
| `stale-while-revalidate=N` | After expiry, serve stale for N seconds while revalidating in background |
| `stale-if-error=N` | Serve stale if origin unreachable |
| `no-cache` | Don't serve from cache without validating with origin (via ETag/Last-Modified) |
| `no-store` | Don't cache (sensitive data) |
| `immutable` | Will never change; safe to cache indefinitely. Used for versioned assets (app.abc123.js) |

### Common Patterns

**Static assets** (images, CSS, JS):
```
Cache-Control: public, max-age=31536000, immutable
```
Pair with content hashing (app.hash.js) so updates change the URL, triggering fresh fetches.

**HTML:**
```
Cache-Control: no-cache
```
Always revalidate (ETag/Last-Modified checks) but use cached copy if validation passes or origin is down.

**API responses:**
```
Cache-Control: private, max-age=60
```
User-specific data; browser-only, short TTL.

**Dynamic/personalized content:**
```
Cache-Control: no-store
```
Or use cookies/Authorization headers (CDN can't cache authenticated responses without careful configuration).

## Cache Key Design

CDN cache key is typically `URL + relevant query parameters + headers`.

**Poor cache key:** `https://example.com/user/profile?utm_source=google&utm_medium=cpc&utm_campaign=feb`
- Every ad click changes query string, creating new cache entries.
- Same user profile cached multiple times (cache waste).

**Good cache key:** `https://example.com/user/profile`
- Normalize and strip tracking parameters before caching.
- Sort query parameters for consistency.

**Vary header:** Tells CDN to key by additional headers.
```
Vary: Accept-Encoding  # Different cache entries for gzip vs uncompressed
Vary: Authorization   # Different entries per authenticated user (but carefully)
```

Overusing Vary creates cache fragmentation; use judiciously.

## Cache Purge and Invalidation

### Immediate Purge

Remove content from all CDN edge servers synchronously. Used when content becomes stale unexpectedly (critical bug fixed, wrong image uploaded).

**Cost:** Expensive operation (some CDNs charge per purge or cap purges/minute). Avoid excessive purge; design for infrequent invalidation.

### Soft Purge (Stale-While-Revalidate)

Mark content as stale instantly but continue serving it while revalidating in background.

**Benefit:** Clients see fresh content after revalidation completes; no cache miss latency.

### Version-Based Invalidation

Instead of invalidating, change the URL (via content hashing or version parameter).

```
app.v1.js  →  (update)  →  app.v2.js
```

Only new requests fetch v2; old requests continue serving v1 from cache. No explicit invalidation needed. Works best with immutable cache directives.

## Dynamic Content Acceleration (DSA)

CDNs optimize dynamic, personalized content (shopping carts, profiles) with:

### Request Collapsing

Multiple simultaneous requests for the same uncached resource wait for the first request to origin, then all receive the result. Prevents thundering herd / cache stampede.

### Connection Pooling

CDN edge servers maintain persistent connections to origin, reusing TCP/TLS handshakes across origins requests. Reduces connection latency.

### Compression and Optimization

Automatic compression (gzip, brotli), image optimization (convert to WebP, resize), protocol upgrades (HTTP/2, HTTP/3).

### Smart Origin Selection

Real-time latency measurement to origin. If primary origin is slow, use secondary. Improves tail latency.

## Origin Shield and Multi-Tier Caching

Origin shield is a shared cache between origin and edge servers.

**Benefit:** Reduces origin QPS. Instead of 100 edge caches independently missing and hitting origin, they all miss to shield first. Shield's hit ratio (often 70-90%) means origin only sees 10-30% of misses.

**Cost:** One extra hop (~50ms added latency on miss).

**Configuration:** Usually configurable (e.g., AWS CloudFront origin shield in us-east-1, shared among all edge POPs).

## Edge Computing and Compute

Modern CDNs embed compute at edge nodes:

### Cloudflare Workers

JavaScript (V8) runtime at every Cloudflare edge node. Write logic (authentication, routing, transformation) that runs at edge latency (1-5ms from user).

```
// Rewrite URLs at edge
if (request.path.startsWith('/old/')) {
  return fetch(request.path.replace('/old/', '/new/'))
}
```

**Use case:** Request redirection, authentication, Bot management, custom routing.

### AWS Lambda@Edge

Run Lambda functions at CloudFront edge locations (C@E for viewer-facing, origin-facing).

```
// Add security headers at edge
response.headers['X-Frame-Options'] = 'DENY'
return response
```

**Use case:** Custom business logic at edge; geo-blocking; rate limiting.

## Multi-CDN Strategy

Relying on single CDN creates availability risk (CDN outage = global outage).

**Multi-CDN architecture:**
```
Users → (DNS load balancing or Anycast)
     → CDN A (Cloudflare, 30% of traffic)
     → CDN B (CloudFront, 30% of traffic)
     → CDN C (Fastly, 40% of traffic)
```

**Characteristics:**
- **Resilience:** One CDN down, others continue serving.
- **Cost:** Multiples each CDN cost. Premium strategy.
- **Complexity:** Must verify cache hit ratios, latency, uptime across suppliers.
- **DNS failover:** Point clients to healthy CDN vias failover records.

## HTTP Caching: Browser and Intermediary

CDN caching optimizes origin load; browser caching optimizes client latency further.

**Browser cache directives:**
- `max-age` → stay fresh for N seconds
- `ETag` / `Last-Modified` → revalidate (conditional request); 304 Not Modified confirms cache validity

**HTTP/2 and HTTP/3 improvements:**
- Header compression reduces redundant cookie/header bytes
- Multiplexing reduces latency (no head-of-line blocking)
- These features improve throughout even without CDN

See also: **web-http-caching.md**, **infrastructure-cdn.md**, **performance-caching-strategies.md**, **cloud-edge-computing.md**.

## Cache Monitoring and Debugging

**Metrics to track:**
- **Hit ratio:** Percentage of requests served from cache. Target 95%+.
- **Origin bandwidth:** Bytes served from origin per second. Lower = better (cache doing its job).
- **Latency percentiles (p50, p95, p99):** Is edge serving fast enough?
- **Cache fraud:** Requests that bypass cache (e.g., `Cache-Control: no-cache` forcing revalidation). Often unexpectedly high.

**Debugging cache misses:**
- Check `Cache-Control` headers (is origin telling CDN to cache?)
- Check `Vary` headers (is fragmentation occurring?)
- Check cookies/auth (authenticated responses not cached unless configured)
- Check `User-Agent` (mobile vs desktop can be separate cache entries)

## When NOT to Use CDN

**Cases where CDN adds no benefit:**
- **Static internal tools:** Latency isn't the bottleneck; CDN overhead adds complexity.
- **Ultra-low-value content:** Content so niche that cache hits are rare (<50%). More cost than benefit.
- **Custom latency-sensitive protocol:** Binary protocol, WebSocket, gRPC. CDN adds latency without benefit. Use application-level scaling instead.

**Cases where edge compute is wrong:**
- Complex business logic (edge functions have latency/memory limits)
- Statefulness (edge has no persistent storage; state must live in external DB)
- Billing-critical logic (difficult to debug and test at edge scale)