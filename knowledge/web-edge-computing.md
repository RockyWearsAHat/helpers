# Edge Computing for Web — Functions, Databases, KV Stores & Geographic Routing

## Overview

Edge computing relocates application logic and data closer to end users, distributed across edge nodes in content delivery networks (CDNs). Instead of processing requests at a centralized data center, code executes at the geographic edge. This reduces latency, improves resilience to origin outages, and enables request-time customization (geo-blocking, A/B testing, login flows). Tradeoff: constrained runtime environments, cold starts, and increased operational complexity.

## Edge Functions

### Execution Model

Edge functions (Cloudflare Workers, Vercel Edge, Deno Deploy) run JavaScript/TypeScript/Rust at edge nodes. A request arrives at an edge server close to the user; your function intercepts it, modifies the request/response, or generates content directly.

**Invocation:**

```javascript
// Cloudflare Worker
export default {
  async fetch(request, env, ctx) {
    const userCountry = request.headers.get('cf-ipcountry')
    
    if (userCountry === 'US') {
      return new Response('Welcome to US site')
    }
    
    // Proxy to origin
    return fetch('https://api.example.com', request)
  }
}

// Deno Deploy
import { serve } from "https://deno.land/std@0.208.0/http/server.ts"
serve(req => new Response("Hello from edge"), { port: 8000 })
```

### Deployment Model

- **Instant propagation:** Deploy once, runs on all edge nodes (no rolling deploy)
- **Stateless:** Assume no persistent local storage (Durable Objects, KV, or external storage required for state)
- **Serverless metering:** Pay per request or time used; no upfront capacity

### Cold Start

When an edge node receives a request for the first time, the runtime must:

1. Fetch the function code from CDN
2. Initialize JavaScript VM or WebAssembly module
3. Execute the function

This **cold start** typically takes 10–100ms on modern platforms. Strategies to reduce impact:

- **Keep functions small:** Faster to fetch and initialize
- **Minimize dependencies:** Each import adds initialization time
- **Warmth:** Frequently-used regions have warm instances; rarely-used regions experience cold starts on the first request
- **Scheduled pings:** Prevent eviction by periodically invoking the function (but this costs)

## Edge Use Cases

### Request Routing & Authentication

```javascript
// Route based on hostname/path before hitting origin
if (request.url.includes('/api/')) {
  return fetch('https://api.example.com' + url.pathname)
}

// Check auth token and redirect if needed
const token = request.headers.get('authorization')
if (!token) {
  return new Response('Unauthorized', { status: 401 })
}
```

**Benefit:** One entry point, dynamic routing, low latency.

### Response Modification

```javascript
const response = await fetch(origin)
const text = await response.text()
const modified = text.replace('</head>', '<link rel="preconnect" href="https://cdn.example.com"></head>')
return new Response(modified)
```

Use cases: injecting scripts, modifying headers, streaming transformations.

### Geo-Blocking & Personalization

```javascript
const country = request.headers.get('cf-ipcountry')
if (['CN', 'RU'].includes(country)) {
  return new Response('Not available in your region', { status: 451 })
}

// Serve locale-specific content
const acceptLanguage = request.headers.get('accept-language')
```

## Edge Databases

Traditional databases (PostgreSQL, MySQL) expect persistent server infrastructure and aren't designed for thousands of geographic locations. Edge databases are replicated or sharded globally to be local.

### LibSQL & Turso

Turso is a SQLite fork optimized for edge and embedded deployment:

- **Embedded:** SQLite runs in the same process as your app (browser, edge function, server)
- **Async design:** Uses modern async primitives (io_uring on Linux) instead of blocking I/O
- **Replication:** Data syncs from Turso Cloud to edge locations on demand
- **Vector search:** Native similarity search for LLM applications

```javascript
// In-process SQLite (Wasm in browser)
import Database from "sql.js"
const db = new Database()
db.run("CREATE TABLE users (id INTEGER, name TEXT)")

// Turso Cloud: managed SQLite with replication
import { createClient } from "@libsql/client"
const db = createClient({
  url: "libsql://dbname-user.turso.io",
  authToken: "token"
})
const result = await db.execute("SELECT * FROM users")
```

**Tradeoff:** SQLite is lightweight and single-writer-friendly, but not intended for high-concurrency write workloads. Turso is adding concurrent writes; until then, consider for read-heavy or low-contention writes.

### Cloudflare D1

D1 is Cloudflare's serverless SQL database using SQLite backend with CloudflareD1 replicates data to edge, but writes serialize to a primary location. Enables local reads with eventual consistency.

**Connection & Query:**

```javascript
// Cloudflare Worker
export default {
  async fetch(request, env) {
    const db = env.DB  // bound via wrangler.toml
    const users = await db.prepare("SELECT * FROM users WHERE id = ?")
      .bind(req.userId)
      .all()
    return new Response(JSON.stringify(users))
  }
}
```

**Limits:** ~1 GB per database, ~100 MB query result size, 30s timeout.

### PlanetScale Serverless

MySQL-compatible database with serverless driver (HTTP instead of TCP). Works better than traditional MySQL for edge functions:

```javascript
const connection = await serverless(db)
const users = await connection.query("SELECT * FROM users LIMIT 10")
```

**Benefit:** Connection pooling is automatic; no persistent TCP connection needed.

## Key-Value Stores at Edge

### Cloudflare KV

Global key-value store accessible from Workers:

```javascript
export default {
  async fetch(request, env) {
    const cached = await env.KV.get('page:homepage')
    if (cached) return new Response(cached)
    
    const resp = await fetch('https://api.example.com')
    const data = await resp.text()
    
    // Cache globally, 1 hour TTL
    await env.KV.put('page:homepage', data, { expirationTtl: 3600 })
    return new Response(data)
  }
}
```

**Consistency model:** Eventually consistent across regions (writes at one edge replicate to others with ~60s latency). Suitable for cache, not ACID transactions.

### Durable Objects (Stateful Storage)

For workloads requiring strong consistency and isolation:

```javascript
export class Counter {
  constructor(state, env) {
    this.state = state
  }
  
  async fetch(request) {
    let count = await this.state.storage.get('count') || 0
    count++
    await this.state.storage.put('count', count)
    return new Response(count)
  }
}

export default {
  async fetch(request, env) {
    const id = env.COUNTER.idFromString('global')
    const obj = env.COUNTER.get(id)
    return obj.fetch(request)
  }
}
```

**Model:** Each Durable Object is a unique, persistent namespace anchored to one geographic location. Requests to the same object serialize; good for global counters, rate limiting, user presence.

## Edge Middleware Patterns

### Authorization & Session Validation

```javascript
export default async (request) => {
  const session = parseCookie(request.headers.get('cookie'))
  
  // Verify session token (cached in KV)
  const valid = await checkSessionToken(session.id)
  if (!valid) {
    return new Response('Session expired', { status: 401 })
  }
  
  // Decorate request for downstream handlers
  request.user = session.user
  return fetch(originUrl, request)
}
```

### Rate Limiting & Sliding Window

```javascript
export default async (request, env) => {
  const ip = request.headers.get('cf-connecting-ip')
  const key = `rate:${ip}`
  const count = (await env.KV.get(key)) || 0
  
  if (count > 100) {
    return new Response('Too many requests', { status: 429 })
  }
  
  await env.KV.put(key, count + 1, { expirationTtl: 60 })
  return fetch(originUrl, request)
}
```

### Request Transformation

```javascript
// Decompress body, re-sign request
export default async (request) => {
  let body = await request.text()
  body = decompress(body)
  
  const newRequest = new Request(originUrl, {
    ...request,
    body,
    headers: new Headers(request.headers),
  })
  
  newRequest.headers.set('x-signature', sign(body))
  return fetch(newRequest)
}
```

## Geographic Routing

Edge functions can inspect request metadata to route based on geography:

```javascript
const country = request.headers.get('cf-ipcountry')
const lat = request.headers.get('cf-iplatitude')
const lon = request.headers.get('cf-iplongitude')

// Route to nearest origin
const origins = {
  'us': 'https://us-api.example.com',
  'eu': 'https://eu-api.example.com',
  'ap': 'https://ap-api.example.com',
}

const endpoint = origins[georegion(lat, lon)]
return fetch(endpoint, request)
```

## Constraints & Tradeoffs

### CPU & Memory Limits

- **CPU time:** 50ms to 30s depending on platform, plan tier
- **Memory:** 128 MB to 512 MB
- **Request body size:** 100 MB (varies)

Large computations, ML inference, or heavy data processing don't fit; they belong on origin servers or specialized inference platforms.

### Cold Start Sensitivity

For latency-critical path (user-facing request), cold starts are noticeable. Warm regions have cached instances; rarely-used edge locations start cold. Mitigate by deploying popular features to all nodes, keeping code small, and caching results.

### Debugging & Observability

Stack traces don't show source maps by default. Logging to stdout is available but limited. Use structured logging (JSON) and ship to a logging service. Performance profiling is harder than on servers; rely on request metadata (CPU time, memory) and real-time metrics from the platform.

## Comparison: Edge vs. Origin

| Aspect | Edge | Origin |
|--------|------|--------|
| Latency | Low (near user) | Centralized (variable) |
| Cold start | Yes | Usually warm |
| Statefulness | Limited (KV, Durable Objects) | Full (databases, caches) |
| Consistency | Eventual | Strong (if configured) |
| Cost | Per-request | Per-server or serverless pods |
| Complexity | Higher (distributed state) | Simpler (single location) |

Edge is best for request filtering, personalization, and response augmentation. Origin servers handle stateful operations, complex transactions, and long-running tasks.