# Infrastructure — Edge Computing Patterns: Locations, Compute, Data & Offline

## Overview

Edge computing distributes computation and data storage closer to users, reducing latency from the 50-200ms of centralized datacenters to 10-50ms at regional edges. This note covers the operational and architectural patterns for deploying at edge: where edges exist (CDN PoPs, telco nodes, on-premise), what workloads run there (compute-at-edge functions, edge caching), data consistency at edge, and offline capability. The fundamental tradeoff: milliseconds of latency gain against operational complexity (distribution, consistency, rollback).

## Edge Locations: Where to Run Code

Edge infrastructure exists at three physical scopes:

### CDN Points of Presence (PoPs)

Most accessible edge: 300-500 globally distributed nodes managed by CDN providers (Cloudflare, AWS CloudFront, Fastly). Organized into geographic regions (NorthAmerica, Europe, Asia-Pacific) with multiple nodes per continent.

**Deployment model**: Push code to CDN provider; platform handles replication to all PoPs. No infrastructure provisioning required.

**Latency**: ~20-50ms for major cities; 100+ ms for remote regions depending on PoP density.

**Compute limits**: 
- **Cloudflare Workers**: JavaScript/WebAssembly, 30 sec CPU time per request, 128 MB memory
- **AWS Lambda@Edge**: Node.js/Python, 30 sec timeout, 3008 MB memory  
- **CloudFront Functions**: 10 KB code, 1 ms execution (lightweight rewrites only)

**Use cases**: Request/response manipulation (auth, redirects, header injection), personalization, A/B testing, first-layer bot protection, geographic routing.

### Telco/Mobile Edge (Multi-Access Edge Computing, MEC)

Telecom operators colocate compute at cell tower sites and ISP backbone nodes. Lower latency (5-20ms) than CDN PoPs for dense mobile networks. Emerging but not standardized; carrier partnerships required.

**Characteristics**:
- Operator-controlled infrastructure; SDN/NFV-based deployment
- Closer to mobile users than regional PoPs
- Limited regional coverage outside major cities/carriers

**Use cases**: Real-time mobile applications (AR/VR, live gaming), autonomous vehicles, industrial IoT.

### On-Premise/Private Edge

Customer-operated edge nodes in offices, factories, or retail locations. Kubernetes clusters, serverless runtimes (Knative), or custom containerized workloads run locally.

**Model**: Data and compute stay on-premise for compliance; sync state to cloud asynchronously.

**Connectivity**: Assumes reliable internet for sync; handles intermittent connectivity gracefully.

**Use cases**: Privacy-sensitive workloads (healthcare, finance), low-latency local processing (manufacturing quality control), offline-first applications.

## Compute Patterns at Edge

### Stateless Edge Functions

Request reaches edge → execute function → return response. No session data persists.

**Constraints**: Lightweight processing only (milliseconds). Heavy computation offloads to origin.

**Pattern**: Request transformation (add headers, rewrite URLs), authentication (verify JWT), geolocation-based routing.

**Implementation example (Cloudflare Workers pseudocode)**:
```javascript
export default {
  async fetch(request) {
    const country = request.headers.get('cf-country');
    if (country === 'US') {
      return fetch('https://us-api.example.com' + new URL(request.url).pathname);
    }
    return fetch('https://global-api.example.com' + new URL(request.url).pathname);
  }
}
```

### Edge Rendering

Server-side render (SSR) HTML at edge instead of sending JavaScript bundle to browser.

**Model**: Request → edge renders template → returns HTML → browser displays immediately.

**Benefit**: Faster first contentful paint (FCP); SEO-friendly HTML; reduced JavaScript bloat.

**Tradeoff**: Higher memory/CPU per edge node (rendering overhead); limited scaling per PoP.

**Frameworks**: Next.js Edge Runtime, Remix on Remix Server, SvelteKit on edge runtimes.

### Origin Shield Pattern

Intermediate cache tier between edge and origin. Multiple edge PoPs query shield instead of origin directly.

**Benefit**: Reduces origin load on cache miss; improves hit ratio (shield caches popular items).

**Tradeoff**: One additional hop; adds 10-50ms latency but reduces origin load 10-100x.

**Implementation**: AWS CloudFront Origin Shield, Cloudflare Argo Smart Routing.

## Data Management at Edge

### Cache-Aside Pattern

Edge caches frequently accessed data (user profile, product catalog) on first miss. Subsequent requests hit cache.

**TTL management**: Short TTL (seconds) for mutable data; long TTL (hours/days) for stable data.

**Stale-while-revalidate**: Serve stale cache immediately; revalidate in background; update cache for next request.

### Event-Driven Sync

Changes to master data (product price, user profile) broadcast to edge nodes via pub/sub (Kafka, Redis Streams, SQS).

**Pattern**: Origin commits change → publishes event → edge nodes subscribe → update local cache.

**Implementation**: AWS EventBridge to Lambda@Edge, Cloudflare Workers via Durable Objects + Pub/Sub APIs.

### Edge KV Stores

Some platforms offer edge-accessible key-value storage (Cloudflare Durable Objects, AWS Lambda@Edge DynamoDB query). Tiny memory footprint; sub-millisecond access.

**Use case**: Session state (user authentication token, A/B test assignment), rate-limit counters.

**Tradeoff**: Storage limited (Durable Objects: 128 MB max, billed per write); eventual consistency if distributed.

## Offline Capability & Resilience

### Service Worker Caching (Browser-side Edge)

Browser-managed cache strategy (offline-first, network-first, stale-while-revalidate). When network fails, browser serves cached responses.

**Model**: Client App → Service Worker → check cache → if miss, fetch network → cache response → serve.

**Benefit**: Works offline; faster loads even on slow networks.

**Libraries**: Workbox, SWR (stale-while-revalidate).

### Sync Queue Pattern

Edge collects user mutations (form submissions, edits) in local queue when offline. On reconnect, edge syncs queue to origin server.

**Implementation**: IndexedDB or localStorage + scheduled sync job.

**Conflict resolution**: Last-write-wins, operational transformation, or application-specific logic.

### Graceful Degradation

Edge function detects origin timeout/failure → falls back to stale cache or reduced functionality.

**Pattern**:
```javascript
try {
  return await fetchFromOrigin(10); // 10s timeout
} catch {
  let stale = await cache.get(request);
  if (stale) return stale; // Serve outdated but available
  return new Response('Service temporarily unavailable', {status: 503});
}
```

## Operational Patterns

### Blue-Green Deployment at Edge

Two versions of edge function deployed simultaneously; traffic split between them. Rollback: flip traffic to blue if green has errors.

**Implementation**: Cloudflare Workers Routes, Lambda@Edge traffic split.

### Monitoring Latency by Region

Instrument edge functions to report execution time, origin fetch time, and cache hit/miss rate by geographic region.

**Anti-pattern**: Assuming edge reduces latency everywhere; some regions may have poor origin connectivity.

### Cost of Edge Compute

Pricing model differs from serverless:
- **Cloudflare**: Pay per 10Ms requests; flat rate regardless of execution time
- **AWS Lambda@Edge**: Per-request charge + memory-time (similar to Lambda)
- **Vercel Edge**: Per-invocation charge; bundled with runtime

**Consideration**: Heavy computation at edge (ML inference, image resizing) may be cheaper on origin than paying per-edge-request fees.

## See Also

- [Cloud Edge Computing](cloud-edge-computing.md) — CDN evolution, edge functions platforms, data sovereignty
- [Infrastructure CDN](infrastructure-cdn.md) — Cache architecture, PoP strategy, multi-CDN
- [Performance Caching Strategies](performance-caching-strategies.md) — Cache tiers and invalidation
- [Web PWA](web-pwa.md) — Progressive web apps and offline capability