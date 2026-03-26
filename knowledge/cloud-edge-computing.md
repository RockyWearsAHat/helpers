# Edge Computing — CDN Evolution, Edge Functions & Data Sovereignty

## Overview

**Edge computing** executes application logic near users instead of centralized data centers. A request reaches an edge node (in a city or ISP), logic runs locally (within milliseconds), and only necessary data travels to origin. This reduces latency, improves resilience, and aligns with growing data residency requirements. The model evolved from content delivery networks (CDNs) caching static assets to programmable compute platforms executing arbitrary code at the edge.

## CDN Evolution: From Caching to Computation

### Traditional CDNs (Static Caching)
Early CDNs (Akamai, CloudFlare, CloudFront 2010s) focused on content delivery. A publisher uploads assets (images, CSS, JavaScript) to edge nodes; users download from the nearest node instead of origin.

**Model**: Request hits edge → check cache → if miss, fetch from origin → cache 15-60 minutes → serve user.

**Limitations**:
- **Static content only**: HTML with dynamic data (user profiles, real-time pricing, personalized content) couldn't be fully cached.
- **Origin latency**: Cache misses and repeated origin requests still incur 50-200 ms round trips.
- **Compute impossible**: No ability to run code; edges were passive stores.

### Edge Functions (Programmable Edge)
**(~2017-present)** Platforms like Cloudflare Workers, AWS Lambda@Edge, AWS CloudFront Functions, Vercel Edge Functions, and Deno Deploy enable running arbitrary code at the edge.

* **Cloudflare Workers**: Deploy JavaScript functions to 300+ global PoPs (data centers). Request hits edge → run Worker code → optionally fetch from origin or third-party APIs → respond to user.
* **AWS Lambda@Edge**: Deploy Node.js or Python code to CloudFront edge locations (~500 globally). Powers real-time request/response inspection, personalization, A/B testing, bot mitigation.
* **AWS CloudFront Functions**: Lightweight (10 KB limit) functions for simple logic (rewrites, redirects, header manipulation). Faster cold starts (~25 KB/millisecond) than Lambda@Edge but fewer capabilities.
* **Vercel Edge Functions**: Nexus of Vercel's Next.js framework and serverless hosting. Run on Vercel's infra or Cloudflare Workers as runtime. Tight integration with front-end deployments.
* **Deno Deploy**: Runs arbitrary JavaScript/TypeScript on Deno's edge network. Lower latency than Node.js (V8 engine vs. Node); focus on simplicity and performance.

**Tradeoffs**:

| Platform | Cold Start | Compute Limit | Latency | Cost Model | Best For |
| --- | --- | --- | --- | --- | --- |
| Cloudflare Workers | <1 ms | 50 ms / request | Lowest | $0.50/M requests | High-volume APIs, request transform |
| Lambda@Edge | 5-50 ms | 30 s / request | Low-medium | $0.60/M requests + $0.01/GB data | Personalization, origin failover |
| CloudFront Functions | <1 ms | 1 ms / request | Lowest | Billed per 1M requests (cheaper) | URL rewrites, simple routing |
| Vercel Edge | 5-50 ms | 25 s / request | Low | Free tier + $0.50-1/M requests | Next.js optimization, A/B tests |
| Deno Deploy | <1 ms | 50 ms / request | Lowest | Free tier + compute time billing | Rapid development, V8 performance |

## Use Cases & Latency Requirements

### Real-Time Request Transformation
**Example**: Route requests based on geography, user agent, or custom logic.

```javascript
// Cloudflare Worker: Route /api to geographically closest origin
addEventListener('fetch', event => {
  const country = event.request.headers.get('cf-ipcountry');
  const origin = country === 'AU' ? 'au.api.example.com' : 'us.api.example.com';
  event.respondWith(fetch(`https://${origin}${new URL(event.request.url).pathname}`, event.request));
});
```

**Latency requirement**: <10 ms added overhead. Edge logic must complete in <1 ms to be transparent.

### Personalization & A/B Testing
Modify responses based on user context: show darker UI theme for night mode (set via cookie), redirect cohort A to /v1/pricing and cohort B to /v2/pricing, inject experiment IDs into analytics calls.

```javascript
// Inject experiment ID header
response.headers.set('x-experiment-id', cohort);
```

**Latency requirement**: Similar request time with or without personalization. If personalization adds >50 ms, users perceive slowdown.

### Bot Detection & DDoS Mitigation
Validate requests at the edge before reaching origin; block malicious traffic early, reducing bandwidth waste and origin load.

```javascript
// Reject requests with suspicious patterns
if (suspiciousRequest(request)) {
  return new Response('Access Denied', { status: 403 });
}
```

**Latency requirement**: <5 ms to avoid slowing legitimate traffic.

### Authentication & Authorization
Validate JWT tokens, check permissions, or redirect unauthenticated users before origin.

**Latency requirement**: <20 ms (network round-trip to auth service is acceptable if cached locally).

### Cache Purging & Invalidation  
Execute cache purges without waiting for background jobs. Request hits edge → run purge logic → invalidate related cache keys → respond.

**Latency requirement**: Response time unchanged; purge propagates asynchronously.

## Data Sovereignty & Regulatory Constraints

### Geofencing
Edge functions can enforce data residency: refuse to process or cache data from users in certain regions if regulations prohibit it.

```javascript
const country = request.headers.get('cf-ipcountry');
if (['CN', 'RU'].includes(country)) {
  return new Response('Not available in your region', { status: 403 });
}
```

**Implication**: GDPR, CCPA, and similar laws may require data processing to occur in specific geographies. Edge functions running in data centers outside those zones violate regulations. Solution: geo-fence traffic to origin servers in compliant regions, or run edge functions only in approved data center regions (fewer PoPs, higher latency).

### Data Movement Constraints
HIPAA and financial regulations restrict where PHI or PII can be transmitted. Encrypting data remains within-edge-encrypted-blobs, but transmitting to US-based origin can violate compliance. Solutions:

1. **Run origin in regulated region**: EU data never leaves EU; route EU traffic to EU origin.
2. **Use edge functions to encrypt before transit**: Worker encrypts sensitive data, sends encrypted payload to third-party origin.
3. **Restrict edge function deployment**: Some providers (Cloudflare, Vercel) offer data residency features limiting function execution to specific regions.

## Edge Databases & Persistent State

Traditional edge functions are stateless — each request is independent. Introducing data requires either:

1. **Origin databases**: Queries travel 50-200 ms round-trip, defeating edge latency benefits.
2. **Edge caching**: TTL-based caching in KV stores (Cloudflare Workers KV, Deno's KV) reduces origin trips but risks stale reads.
3. **Global distributed databases**: Databases replicated to multiple regions (Neon, Planetscale, Fauna) provide local replicas with replication lag (5-30 seconds). Good for read-heavy workloads; writes conflict with other regions.
4. **Edge Durable Objects** (Cloudflare): Stateful, strongly-consistent compute at the edge. One Durable Object per logical entity (session, connection, counter), automatically replicated within a region. Latency: <5 ms for same-region access, 20-100 ms for multi-region failover.

**Tradeoff**: Stateful edge compute increases complexity (failover, replication, consistency) but enables real-time collaboration features (multiplayer editing, live counters) with sub-100 ms latency.

## IoT & Fog Computing

### IoT Edge Inference
IoT devices (sensors, cameras, drones) collect data continuously. Sending all raw data to cloud is expensive (bandwidth, storage, processing). **Edge inference** runs ML models locally (on device or nearby gateway) to pre-process, filter, or classify data.

Example: Camera detects 1000 frames/second; run ML model locally to identify "person" frames (100 per second); send only those to cloud. Reduces bandwidth 10x, latency from 2 seconds to 200 ms.

### Fog Computing
Fog describes the continuum from device → edge → cloud. A hierarchical model:
- **Device layer**: Temperature sensors, actuators (millisecond latency, minimal compute)
- **Fog layer**: Local gateway, edge cache (in factory or warehouse; 10-100 ms latency, modest compute)
- **Cloud layer**: ML training, analytics, archival (second+ latency, unlimited compute)

Data flows up; policies flow down. Example: Temperature reading triggers alert if > 65°C locally (1 ms latency). Alert propagates to cloud after 5 minutes (batched). Cloud retrains model overnight; push updated model back to fog layer.

### Multi-Access Edge Computing (MEC)
MEC deploys compute at the cellular network's edge (co-located with 5G/LTE radio towers). Telecom operators (Verizon, Deutsche Telekom) offer MEC as a service.

**Latency**: 1-5 ms to MEC (vs. 20-100 ms to cloud data center) for 5G users.
**Use case**: Autonomous vehicles need <10 ms latency to process sensor data; MEC handles local decision-making; cloud handles fleet analytics.
**Limitation**: Limited to MEC provider's infrastructure; not globally available.

## Cold Starts & Performance Trade-offs

Edge functions suffer **cold starts**: first invocation after deployment takes 5-50 ms (spin up container, JIT compile); subsequent requests <1 ms.

**Impact**: Site metrics (Largest Contentful Paint, First Input Delay) are sensitive to cold starts. If 5% of requests hit cold starts, P95 latency jumps visibly.

**Mitigations**:
- Pre-warm functions (send dummy requests before traffic ramps).
- Use languages with fast startup (JavaScript, Go). Avoid Python, Java if cold start is critical.
- Limit function complexity (functions >10 MB have slower cold starts).
- Gradual rollout: 1% traffic first, 10%, 100% over hours to distribute cold starts.

## Cost & Trade-offs

**Billing models**:
- Cloudflare Workers: $0.50/million requests (requests, not compute time). $0.50/GB egress from Workers to origin.
- Lambda@Edge: $0.60/million requests + $0.01/GB data processed.
- Vercel Edge: Included in Vercel hosting; overage billed per execution.
- Deno Deploy: Free tier (1B requests/month); paid tier $5/month per project.

**When NOT to use edge**:
- Heavy compute (ML inference >500 ms): run on origin GPU.
- Stateful workflows (sessions, carts): use origin or edge durable objects, not unreliable edge caching.
- Data dependencies (multiple database queries): origin latency dominates; edge adds minimal gain.
- Low traffic (<1 million requests/month): free tier or single-region origin is simpler and cheaper.

## See Also

- **web-http-caching.md** — CDN caching strategies, Cache-Control headers
- **architecture-serverless.md** — Serverless compute patterns; edge is extreme serverless
- **networking-dns.md** — DNS routing to edge points of presence
- **cloud-disaster-recovery.md** — Edge as resilience pattern for origin failover