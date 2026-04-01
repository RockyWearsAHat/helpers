# HTTP Caching — Cache-Control, Validation, CDN Strategies & Invalidation

## Overview

HTTP caching is the foundational performance layer of the web. It reduces latency, saves bandwidth, and decreases server load by storing responses at the client (browser) and intermediate layers (CDNs, proxies). Caching decisions are controlled via HTTP headers: `Cache-Control`, `ETag`, `Last-Modified`, and `Vary`. Understanding when and where to cache is essential for both performance and correctness.

---

## Cache Hierarchy: Private vs. Shared Caches

Caches exist at multiple layers:

### Private (Browser) Cache
- Stores responses locally on the user's device
- Serves a single user; can contain personalized content
- Controlled by `Cache-Control: private` or default behavior
- Example: browser-cached CSS, images, API responses

### Shared Caches
- Exist between origin servers and clients (proxies, CDNs, edge caching)
- Serve many users; must not contain personalized content
- Controlled by `Cache-Control: public` (explicit) or implicit if shareable
- Examples: Cloudflare, Akamai, Varnish
- Must respect `Cache-Control: private` (forbidden in shared caches)

**Trade-off:** Shared caches boost CDN hit rates but require cache-safe content (no user-specific data).

---

## Cache-Control Directives

### Response Directives (Server → Cache)

**max-age=N**
- Response stays fresh for N seconds
- Most important directive; overrides `Expires` header
- After N seconds, entry becomes _stale_; revalidation required
- `Cache-Control: max-age=3600` → fresh for 1 hour

**public**
- Explicitly permit shared caches to store this response
- Not required (implicit for most responses); useful for clarity

**private**
- Forbidden in shared caches; browser cache only
- Use for personalized content (user profiles, account pages)
- `Cache-Control: max-age=3600, private`

**no-cache**
- Response can be cached but **must revalidate before reuse**
- Does NOT mean "don't cache"; means "don't use without validation"
- Client must check freshness via ETag or Last-Modified (conditional request)

**no-store**
- **Never cache this response**
- Used for sensitive data (passwords, credit cards, session tokens)
- `Cache-Control: no-store, no-cache` (belt-and-suspenders)

**must-revalidate**
- Once stale, **must revalidate with origin server**
- Shared caches cannot serve stale response even if offline
- Stricter than `max-age` alone

**proxy-revalidate**
- Like `must-revalidate` but for shared caches only; browser cache can ignore

**s-maxage=N**
- Override max-age for shared caches only (N seconds)
- `Cache-Control: max-age=60, s-maxage=3600` → browser: 60s, CDN: 1h
- Allows different freshness between layers

**stale-while-revalidate=N**
- After expiry, serve stale response **while revalidating in background**
- Client gets fast response; freshness happens asynchronously
- Excellent for reducing latency on "good enough" data
- `Cache-Control: max-age=3600, stale-while-revalidate=86400`

**stale-if-error=N**
- If origin is unreachable, serve stale response for up to N seconds
- Fault tolerance: users get stale data rather than 504 errors
- `Cache-Control: max-age=60, stale-if-error=86400`

**immutable**
- Response will never change; can be cached forever
- Used for versioned/hashed assets: `/js/app.abc123.js`
- Browser won't revalidate even if explicitly requested

### Request Directives (Client → Server, rarely used)

**max-age=N**
- Client will accept cached responses up to N seconds old

**min-fresh=N**
- Only accept responses fresh for at least N more seconds

**no-cache**
- Revalidate before using cached response
- Forces fresh copy from origin (used by `Ctrl+F5` refresh)

**only-if-cached**
- Accept cached response or error; don't contact origin

---

## Cache Validation: ETag and Last-Modified

When a cached response becomes stale, revalidation checks if the resource has changed WITHOUT downloading the full body again.

### Last-Modified Header
- Server sends: `Last-Modified: Tue, 25 Mar 2026 14:30:00 GMT`
- Client revalidates via: `If-Modified-Since: Tue, 25 Mar 2026 14:30:00 GMT`
- Server responds with 304 (Not Modified) if unchanged (small overhead; no body)
- Limitation: second-level granularity; unreliable for frequent updates

### ETag (Entity Tag)
- Server computes hash/version of response: `ETag: "abc123"` or `ETag: W/"abc123"` (weak)
- Client revalidates via: `If-None-Match: "abc123"`
- Server responds 304 if tag matches (resource unchanged)
- Weak ETags (`W/`) represent functionally equivalent representations; ignore minor format changes
- Strong ETags require byte-for-byte match
- More reliable than Last-Modified; supports conditional writes (PUT with `If-Match`)

**When to revalidate:**
- After `max-age` expires
- On user explicit refresh (`Ctrl+F5`)
- On conditional requests from client

Revalidation is cheap: server returns 304 (few bytes) instead of full response body.

---

## Vary Header: Multi-Dimensional Cache Keys

By default, cache key = URL. The `Vary` header expands the key to include request headers.

`Vary: Accept-Encoding` → separate cache entries for gzip, brotli, identity
- Browser with gzip capability gets cached gzip; non-gzip browser gets uncompressed
- Essential for CDNs serving multiple encodings

`Vary: Accept-Language` → separate cache entries for en, fr, es, etc.
- Content-negotiated by Accept-Language is cached separately

`Vary: Accept` → separate entries for application/json vs. application/xml

**Common combinations:**
- `Vary: Accept-Encoding, Authorization` (if response contains user-specific data)
- Beware: too many Vary dimensions fragment cache, reducing hit rate

---

## Cache Busting & Versioning

Static assets (JS, CSS, images) should be long-lived (max-age=31536000, 1 year) but must be updated when code changes. Strategies:

### Hash-Based Versioning (Preferred)
- Filename includes content hash: `/js/app.a1b2c3.js`
- Change code → new hash → new filename → cache miss
- Old hash remains cached separately (no collision)
- Set `Cache-Control: max-age=31536000, immutable`

### Query String (Legacy, Less Reliable)
- `/js/app.js?v=1.2.3` → cache key includes query string
- Problem: some caches/proxies ignore query params; inconsistent behavior
- Avoid for production

### URL Path with Version
- `/v1/app.js` or `/app-1.2.3.js`
- Same problem as query strings; less reliable than hash

**Best practice:** Use build tools (webpack, Vite, esbuild) to insert content hashes into filenames automatically.

---

## Service Worker Caches

Service Workers provide client-side cache control independent of HTTP headers:

- `Cache API`: explicit, offline-capable cache controlled by JavaScript
- Not affected by HTTP `Cache-Control` or browser cache expiry
- Can cache failed requests, pre-cache assets, implement custom revalidation logic
- Useful for progressive web apps (PWAs), offline support
- Use with HTTP cache for layered strategy: HTTP headers for simple cases, Service Worker for complex offline/retry scenarios

---

## CDN Caching Strategies

### Edge Caching (CDN Layer)
- CDNs like Cloudflare, Akamai, AWS CloudFront cache at edge locations near users
- Obey `Cache-Control: s-maxage`, `public`, `max-age`
- Private/personalized responses bypass CDN cache
- `Cache-Control: public, max-age=300, s-maxage=3600` → browser: 5min, CDN: 1h

### Cache Purging
- Explicitly clear CDN cache before max-age expires
- Triggered by: explicit API call, webhook from deployment, tag-based purging
- Example: deploy new code → purge CDN → serve fresh version
- Cost: small latency hit on next request (CDN re-fetches from origin)

### Origin Shield
- Extra cache layer between CDN and origin
- Reduces thundering herd: multiple CDN nodes request origin simultaneously
- One node fetches, others wait and reuse result

---

## Cache Invalidation Strategies

### Time-Based Expiry (Passive)
- Response expires after max-age; stale revalidation required
- Simplest; eventual consistency
- Used for non-critical data

### Event-Based Purging (Active)
- Manually trigger cache purge via API (Cloudflare, Akamai, custom)
- On deployment: clear all cache immediately
- On edit: clear only affected resources (by tag or URL pattern)
- Cost: increases origin load if purged too aggressively

### Surrogate Keys / Tags
- Assign cache entries to logical groups: `Surrogate-Key: product-123 inventory`
- Purge all entries with key `product-123` on product update
- Reduces false positives (don't purge unrelated entries)

### Purge-On-Publish (Webhooks)
- Deployment system calls CDN purge webhook after release
- Cache updates within seconds of deployment
- Risk: if webhook fails, stale content served until max-age expires

---

## HTTP/2 & HTTP/3 Caching Implications

- **Don't disable caching for "security"** — multiplexing doesn't change security model
- HTTP/2 connection reuse means validation overhead is smaller (header-only roundtrip)
- QUIC (HTTP/3) further reduces validation latency
- Cache strategies remain the same; latency improvements benefit stale revalidation

---

## Common Patterns & Pitfalls

### Immutable + Long TTL (Static Assets)
```
Cache-Control: public, max-age=31536000, immutable
ETag: "hash123"
```
- Use hash-based versioning in filename
- Cache forever; updates via new filename

### Revalidatable (HTML, API)
```
Cache-Control: public, max-age=300, must-revalidate
ETag: "hash456"
Last-Modified: Tue, 25 Mar 2026 15:00:00 GMT
```
- Fresh for 5 minutes; revalidate after expiry
- Clients get fast response; eventual freshness

### Sensitive (Passwords, Tokens)
```
Cache-Control: no-store, no-cache, private
Pragma: no-cache
Expires: 0
```
- Never cache; never store on disk
- Historical Pragma/Expires for compatibility

### Stale-While-Revalidate (User-Facing Data)
```
Cache-Control: public, max-age=60, stale-while-revalidate=604800
```
- Serve stale for 7 days if origin unreachable
- Revalidate in background
- Improves availability and user experience

### No Cache (Generate on Each Request)
```
Cache-Control: no-cache
ETag: strong validator
```
- Example: personalized dashboards, time-sensitive data
- Revalidate on every request; serve 304 if unchanged

---

## Mental Model: The Cache Hierarchy

```
┌─────────────────────────────────────────┐
│ Browser Private Cache (max-age, private) │  ← Fastest; user-specific
└────────────────────┬────────────────────┘
                     │
         ┌───────────┴───────────┐
         │  Revalidation Needed? │
         │  (stale)              │
         └───────────┬───────────┘
                     │
          ┌──────────▼──────────┐
          │ CDN/Proxy Cache     │  ← Regional; public data
          │ (s-maxage, public)  │
          └──────────┬──────────┘
                     │
         ┌───────────┴───────────┐
         │  Revalidation Needed? │
         │  (stale)              │
         └───────────┬───────────┘
                     │
          ┌──────────▼──────────┐
          │ Origin Server       │  ← Slowest; ground truth
          │ (compute response)  │
          └─────────────────────┘
```

- Request hits browser cache → instant (no network)
- Browser cache stale → check CDN → instant (regional)
- CDN cache stale → check origin → ~200ms (depends on geography)
- Cache miss or no-cache → full compute → ~500ms (depends on app)

---

## See Also
- [networking-http.md](networking-http.md) — HTTP protocol semantics
- [web-performance.md](web-performance.md) — broader performance optimization
- [architecture-api-gateway.md](architecture-api-gateway.md) — CDN and edge caching patterns
- [security-web-application.md](security-web-application.md) — cache security (no-store for sensitive data)