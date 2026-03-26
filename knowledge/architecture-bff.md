# Backend for Frontend (BFF) — Client-Specific APIs & Response Shaping

## Overview

**Backend for Frontend (BFF)** is an architectural pattern where each client type (web, mobile, TV, etc.) gets a dedicated backend designed specifically for its needs, rather than having a single general-purpose API serve all consumers.

A BFF:
- **Aggregates** calls to internal microservices or domain-specific APIs
- **Shapes responses** for the client's bandwidth, format, and UI requirements
- **Handles client concerns** like authentication state, session management, error formatting
- **Sits close to the frontend** — owned, deployed, and versioned with the client team

Instead of a web client negotiating with Payments API + Users API + Inventory API directly, those calls happen in a BFF, which combines results and optimizes for web's latency profile.

**When it fits:** Microservices with multiple client types, teams with dedicated frontend ownership, clients with vastly different data models or constraints, systems needing per-client authentication logic.

---

## Per-Client Backends

### The Problem It Solves

Without a BFF, all frontend clients share a single API contract. This creates friction:

- **Mobile needs** less data per request (battery, bandwidth); web needs more (one round-trip acceptable)
- **TV** needs prefetching strategies; **web** needs real-time updates; **batch jobs** need bulk operations
- **Web** authentication lives in HttpOnly cookies; **mobile** lives in local storage; **embedded devices** need API keys
- The **general API** becomes bloated with optional fields and query parameters, a lowest-common-denominator contract

With separate BFFs:

```
// Mobile BFF: minimal fields, pagination optimized for scroll
GET /api/products
  → returns { id, name, thumbnail, price } for 50 items
  
// Web BFF: rich context for immediate rendering
GET /api/products
  → returns { id, name, images, price, rating, reviews, inventory }

// TV BFF: streaming/prefetch-friendly
GET /api/products/feed
  → returns { all fields } × 200 items, cursor for continuation
```

Each client's BFF is designed for its constraints without compromising others.

### Ownership & Deployment

A BFF is **owned by the frontend team**, not backend infrastructure. This creates clear ownership:

- The web team owns `bff-web/` repo, deploys to `bff.example.com`, versions alongside the web frontend
- The mobile team owns `bff-mobile/` repo, deploys to `bff.mobile.example.com`
- Frontend teams can ship fixes without coordinating with a central API team

This mirrors **two-pizza team** organization. Each team controls the request-response contract with its backend.

---

## API Aggregation & Response Shaping

### The Aggregation Layer

The BFF is fundamentally a **facade** that calls downstream services, combines results, and optimizes the response for the client.

```
// Browser makes ONE request to BFF
GET /api/order/123

// BFF calls three services in parallel
1. OrderService: GET /orders/123
2. ShippingService: GET /shipments?order_id=123
3. PaymentService: GET /payments?order_id=123

// BFF combines, filters, reshapes
Response: {
  order: { id, total, items },
  shipping: { address, carrier, estimatedDays },
  payment: { method, last4 },
  uiState: { canCancel: true, showTracking: true }
}
```

The BFF is not a dumb pass-through; it:
- **Parallelizes upstream calls** (fetch order, shipping, payment concurrently)
- **Handles failures gracefully** (if ShippingService is slow, return cached estimate)
- **Deduplicates fields** (both OrderService and PaymentService might return `order_id`; BFF includes it once)
- **Adds computed fields** (`canCancel` is business logic, not returned by OrderService)

### Response Shaping for Client Needs

Different clients have different bandwidth constraints and UI patterns. The BFF adapts.

| Client | Profile | BFF Behavior |
|--------|---------|--------------|
| **Web Fetch** | one large request OK | returns 1000 char object |
| **Mobile over 4G** | bandwidth-sensitive | returns 200 char object, thumbnail URLs only |
| **TV streaming** | battery irrelevant, prefetch-friendly | returns 5000 char bulk response with cursor |
| **GraphQL query** | client picks fields | BFF implements \_query resolver for flexible shaping |

---

## Authentication & Session Handling

The BFF typically handles authentication flows that differ by client.

### Web Client (HttpOnly Cookies)

```
POST /api/auth/login
Request: { username, password }
Response: (HttpOnly Set-Cookie header)

GET /api/profile
(Cookie sent automatically by browser)
Response: { user data }
```

### Mobile Client (Bearer Token)

```
POST /api/auth/login
Request: { username, password }
Response: { accessToken, refreshToken }

GET /api/profile
Header: Authorization: Bearer {accessToken}
Response: { user data }
```

### Derived State in BFF

The BFF may store session state or compute client-specific metadata:

```
// A User Management service returns { id, email, plan }
// The BFF adds:
{
  id, email, plan,
  isTrialExpiring: false,
  billingPortalUrl: "...",  // derived
  featureTier: "premium",   // computed from plan
  showUpsellBanner: false   // UI-specific logic
}
```

---

## GraphQL as a BFF

GraphQL naturally serves the BFF role: a client specifies which fields it needs, and the server returns exactly that.

### GraphQL vs REST BFF

| Aspect | REST BFF | GraphQL BFF |
|--------|----------|------------|
| Field selection | predefined by BFF | client declares in query |
| Over-fetching | yes; BFF decides | no; client controls |
| Under-fetching | BFF chains calls | GraphQL resolver chains calls |
| N+1 problem | avoided by BFF design | DataLoader manages it |
| Deployment | REST BFF; web team owns | GraphQL BFF; can be schema-driven |

GraphQL shines when clients have **highly variable** data needs. A REST BFF makes sense when clients have **similar** needs but different constraints (mobile vs web device classes).

---

## BFF vs API Gateway

The concepts are often confused but serve different purposes.

| Aspect | API Gateway | BFF |
|--------|-------------|-----|
| **Purpose** | Centralized ingress, routing, auth enforcement, rate limiting | Client-specific backend aggregation |
| **Hosted** | infrastructure layer (ops team) | alongside client code (feature team) |
| **Routing** | path-based (`/api/*` → microservices) | client-specific aggregation logic |
| **Owned by** | platform/ops | frontend team |
| **Scaling** | one gateway scales for all clients | one BFF scales per client |
| **Example** | Kong, Envoy, HAProxy fronting microservices | web team's Node.js BFF, mobile team's Java BFF |

**Architecture in practice:** API Gateway sits first (centralized concerns), then client-specific BFFs aggregate from internal services.

```
Client (web) → BFF-Web (owned by web team) → [services behind API gateway]
Client (mobile) → BFF-Mobile (owned by mobile team) → [services behind API gateway]
```

---

## Scaling & Complexity

### BFF Bloat Risk

BFFs have a failure mode: they become a dumping ground for frontend-adjacent logic.

**Antipattern:**
```javascript
// Things that should NOT be in BFF
GET /api/reports/generate
  → Runs ML pipeline, takes 5 minutes, hogs memory
  
BFF is not a task queue or batch processor.
```

**Better:**
- Async job queues (workers) for expensive operations
- BFF coordinates with job service: `POST /jobs → { jobId }`, then poll `/jobs/{jobId}/status`

### Independent Scaling

Each BFF scales independently based on its client's load. A surge in mobile traffic doesn't impact web. Deploy strategies can differ per team.

### Team Friction Points

- **Shared business logic** across BFFs risks duplication. Mitigate with shared libraries or domain services
- **Service contract changes** may require changes to multiple BFFs, not one
- **Debugging** is harder; errors could be in client → BFF → service chain

---

## Deployment & Operations

### Versioning & Backwards Compatibility

BFFs are typically versioned with the frontend:

```
web/ (latest main branch)
  └── depends on bff-web@v2.1.0

bff-web/
  └── v2.1.0 (deployed when web ships)
```

Breaking changes to BFF require coordin with the frontend release. This is simpler than a shared API because only one client depends on each BFF version.

### Canary Deployments

Each BFF can canary independently. Mobile BFF canary doesn't affect web traffic.

### Monitoring

Monitor per-client:
- **Upstream latency** (time to fetch from services)
- **Downstream latency** (time for client to receive response)
- **Error rates by client**
- **Cache hit rates** (if BFF caches)

---

## Common Pitfalls

### BFF as a Service Layer

Avoid putting business logic in the BFF. It's an **aggregation and presentation layer**.

**Wrong:**
```javascript
// BFF decides which orders to show
if (user.isVIP) orders = orders.filter(o => o.discount > 0.1);
```

**Right:**
```javascript
// Service decides, BFF shapes the response
const orders = OrderService.getOrders(userId);
return orders.map(order => ({ ...order, isPriorityShip: order.discount > 0.1 }));
```

### Treating BFF as Stable Internal API

A BFF is a **frontend contract**. It changes when the frontend changes. Don't build other backend services depending on it.

### Over-Caching

BFFs can become so aggressively cached that they serve stale data. Balance cache TTLs with data freshness requirements.

---

## See Also

- **api-gateway** — centralized routing, orthogonal to BFF
- **api-design** — API principles applicable to BFF contracts
- **microservices-communication** — how BFF coordinates with services
- **patterns-event-driven** — alternative to aggregation via events