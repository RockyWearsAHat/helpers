# Web API Patterns — GraphQL, gRPC, WebSockets, Webhooks & Versioning

## Overview

Beyond REST, modern APIs use diverse patterns optimized for different use cases: GraphQL for flexible data fetching, gRPC for performance and streaming, WebSockets for real-time bidirectional communication, Server-Sent Events for one-way server updates, and webhooks for event delivery. Each pattern has trade-offs in simplicity, performance, caching, and compatibility.

---

## GraphQL: Flexible Query Language

### Core Concept
GraphQL lets clients request exactly the data they need, reducing over-fetching (extra fields) and under-fetching (multiple roundtrips). Server defines a schema; clients write queries.

### Schema & Resolvers
- **Schema:** Type definitions (User, Post, Comment); queries and mutations
- **Resolvers:** Functions that fetch/compute field values

```graphql
# Schema
type User {
  id: ID!
  name: String!
  email: String!
  posts: [Post!]!
}

type Post {
  id: ID!
  title: String!
  author: User!
}

type Query {
  user(id: ID!): User
  posts(limit: Int = 10): [Post!]!
}
```

```graphql
# Query (client requests)
query {
  user(id: "123") {
    name
    email
    posts {
      title
    }
  }
}
```

```javascript
// Resolver (server implementation)
const resolvers = {
  Query: {
    user(parent, { id }, context, info) {
      return fetchUser(id); // database query
    }
  },
  User: {
    posts(parent, args, context, info) {
      return fetchPostsByUserId(parent.id);
    }
  }
};
```

### Advantages
- **No over-fetching:** Client gets only requested fields (saves bandwidth)
- **No under-fetching:** Nested queries avoid multiple roundtrips
- **Self-documenting schema:** Introspection reveals types, fields, documentation
- **Strongly typed:** Type safety at query time; validation before execution
- **Single endpoint:** `/graphql` instead of dozens of REST routes

### The N+1 Problem

Naive resolver implementation causes exponential queries:

```javascript
// Resolver for posts field on User type
User: {
  posts(parent, args, context, info) {
    return db.query(`SELECT * FROM posts WHERE author_id = ${parent.id}`);
  }
}

// Query:
query {
  users(limit: 10) {
    name
    posts {
      title
    }
  }
}
```

- Fetch 10 users → 1 query
- For each user, fetch posts → 10 queries
- Total: 11 queries (N+1 problem) instead of 2

### Solutions

**DataLoader (Batching)**
```javascript
const userLoader = new DataLoader(async (userIds) => {
  const users = await db.query(
    `SELECT * FROM posts WHERE user_id IN (${userIds.join(',')})`
  );
  return userIds.map(id => users.filter(u => u.user_id === id));
});

User: {
  posts(parent, args, context) {
    return userLoader.load(parent.id);
  }
}
```

- Collect all post requests for batch
- Execute single query: `WHERE user_id IN (1, 2, 3, ...)`
- Return batched results in order
- Reduces 11 queries to 3 queries

**Query Analysis / Depth Limiting**
- Parse query at server; reject overly deep queries (can force exponential computation)
- Example: limit query depth to 5 levels

**Caching at Resolver Level**
- Cache field resolution results; reuse across requests
- E.g., cache User.posts results for 1 minute

### Subscriptions (Real-Time)
GraphQL Subscriptions enable server push (similar to WebSockets):

```graphql
subscription {
  messageAdded {
    id
    text
    author { name }
  }
}
```

Server pushes each new message to all subscribers. Implementation requires persistent connection (WebSocket).

### Disadvantages
- **Complexity:** More complex than REST; requires careful resolver implementation
- **Query cost:** Complex/nested queries can be expensive; requires rate limiting per-query
- **Caching:** HTTP caching less effective (single endpoint, query in body); cache by query + variables
- **Error handling:** Partial success possible (some fields fail, others succeed); client must handle
- **File uploads:** Not built-in; requires multipart form data workaround

---

## gRPC: High-Performance RPC

### Overview
gRPC uses **Protocol Buffers** (protobuf) for serialization and HTTP/2 for transport. Designed for service-to-service communication, not browser clients.

### Protocol Buffers (IDL)

```protobuf
syntax = "proto3";

service UserService {
  rpc GetUser (GetUserRequest) returns (User);
  rpc ListUsers (Empty) returns (stream User);
  rpc UpdateUser (stream UpdateRequest) returns (UpdateResponse);
  rpc BiDirectionalStream (stream Request) returns (stream Response);
}

message User {
  int32 id = 1;
  string name = 2;
  string email = 3;
}

message GetUserRequest {
  int32 id = 1;
}
```

- **Code generation:** Compiler generates client & server stubs in multiple languages
- **Strongly typed:** All messages validated at compile time
- **Binary serialization:** Compact encoding (smaller than JSON)

### Streaming Modes
- **Unary:** Request/response (like HTTP request/response)
- **Server streaming:** Client request → server sends multiple messages
  - Example: `ListUsers()` streams all users as they arrive
  - Reduces memory overhead; server doesn't buffer entire list
- **Client streaming:** Client sends multiple messages → server response
  - Example: bulk upload (many records → summary)
- **Bidirectional streaming:** Client sends while server sends
  - Example: real-time chat, live updates

### HTTP/2 Transport
- Single TCP connection; multiplexed streams
- Headers compressed (HPACK); no "chunked" encoding overhead
- Server push (experimental)
- Flow control; back-pressure

### Advantages
- **Performance:** Binary encoding (3-10x smaller than JSON), HTTP/2 multiplexing
- **Code generation:** Type-safe stubs; less boilerplate
- **Streaming:** Native support for large datasets, real-time
- **Service-to-service:** Efficient inter-datacenter communication
- **Deadlines:** Built-in timeout propagation across services

### Limitations
- **Browser incompatibility:** gRPC requires HTTP/2 and protobuf; browsers don't support directly
  - Workaround: gRPC-Web gateway (translates HTTP/1.1 + JSON to gRPC)
- **Not human-readable:** Binary format harder to debug; tools needed
- **Streaming complexity:** More complex than request/response
- **Caching:** HTTP caching not applicable; custom cache logic needed

### Use Cases
- Microservices internal APIs (low latency, high throughput)
- Real-time streaming (log aggregation, metrics)
- NOT for public browser APIs (use REST or GraphQL)

---

## WebSockets: Persistent Bidirectional Connection

### Overview
HTTP is half-duplex (request → response). WebSockets establish persistent, full-duplex connection over TCP.

### Handshake
```http
GET /chat HTTP/1.1
Host: example.com
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13

HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
```

- Client initiates HTTP upgrade
- Server responds 101 (Switching Protocols)
- Connection becomes bidirectional; HTTP semantics no longer apply

### Messaging
Once handshake complete, both client and server can send messages:

```javascript
// Client
ws.send(JSON.stringify({ type: 'message', text: 'hello' }));

// Server
ws.on('message', (data) => {
  const msg = JSON.parse(data);
  // broadcast to other clients
  broadcast(msg);
});
```

Messages are framed; no request/response pairing (entirely async).

### Advantages
- **Low latency:** No HTTP roundtrip overhead; messages delivered immediately
- **Bidirectional:** Server can push to client at any time
- **Persistent connection:** Reduce connection establishment overhead
- **Efficiency:** Less header overhead than repeated HTTP requests

### Limitations
- **Connection management:** Must handle reconnection, heartbeats, keep-alive
- **Scaling:** Stateful per connection; load balancing requires session affinity or pub/sub backend
- **Caching/proxying:** Standard HTTP caches don't understand WebSocket
- **Mobile:** Long-lived connections drain battery; TCP connection drops on network switch
- **Debugging:** Harder to inspect with browser DevTools than HTTP

### Use Cases
- Real-time collaboration (Google Docs, Figma)
- Live chat, notifications
- Gaming, interactive dashboards
- Stock tickers, live updates

---

## Server-Sent Events (SSE): One-Way Server Push

Lighter-weight alternative to WebSockets when server-to-client push is sufficient.

### How It Works
```javascript
// Client
const eventSource = new EventSource('/stream');
eventSource.onmessage = (event) => {
  console.log(event.data);
};

// Server (HTTP chunked response)
res.writeHead(200, {
  'Content-Type': 'text/event-stream',
  'Cache-Control': 'no-cache',
  'Connection': 'keep-alive',
});
res.write(`data: {"message": "hello"}\n\n`);
res.write(`data: {"message": "world"}\n\n`);
```

Format:
```
id: 1
event: chat
data: {"text": "hi"}

id: 2
event: notification
data: {"type": "alert"}
```

### Advantages
- **Simple:** Uses HTTP (not protocol upgrade); simpler than WebSockets
- **Browser built-in:** No library needed; just `new EventSource(url)`
- **Automatic reconnection:** Browser auto-reconnects with exponential backoff
- **Event types:** Can distinguish message types (chat vs. notification)

### Limitations
- **One-way:** Only server → client (no client push within SSE; requires separate HTTP)
- **HTTP/1.1 limit:** Browsers limit ~6 concurrent SSE connections per domain
  - HTTP/2 solves this; but not all servers support HTTP/2
- **Scaling:** Same challenges as WebSocket (stateful, load balancing)

### Use Cases
- Live updates (status, feeds)
- Server-sent notifications
- Simple real-time (when bidirectional not needed)

---

## Webhooks: Event-Driven API

### Concept
Instead of client polling for updates, server pushes events to client-provided URL.

### Flow
1. Client registers webhook: `POST /webhooks { url: 'https://client.com/callback', event: 'user.created' }`
2. Server stores webhook registration
3. Event occurs (user created); server calls: `POST https://client.com/callback { data: {...} }`
4. Client receives and processes event

### Delivery Guarantees
- **Best-effort:** Server attempts delivery once; no retry
- **Improved:** Retry with exponential backoff (e.g., 1s, 2s, 4s, ...) up to N times
- **Guaranteed:** Persist event; retry indefinitely until acknowledged
- **Idempotent:** Include `Idempotency-Key` header; client must handle duplicate attempts

### Signature Verification
Server signs webhook payload (HMAC-SHA256) so client can verify authenticity:

```javascript
// Server
const signature = hmac(secret, JSON.stringify(payload));
res.post(url, payload, {
  'X-Signature': signature
});

// Client
const received = req.body;
const signature = req.header('X-Signature');
const expected = hmac(secret, JSON.stringify(received));
if (signature !== expected) throw new Error('Invalid signature');
```

### Advantages
- **Real-time:** No polling; event delivered immediately
- **Lightweight:** Server doesn't maintain connection state
- **Decoupled:** Client can be offline; event queued until delivery
- **Scalable:** Server broadcasts to many clients without stateful connections

### Limitations
- **Latency:** Network-dependent; retry may delay delivery
- **Complexity:** Client must expose HTTP endpoint, handle async delivery, ensure idempotency
- **Debugging:** Harder to test; often require tools like ngrok or webhook inspection services

---

## API Versioning Strategies

### URL Path Versioning
```
GET /v1/users → old version
GET /v2/users → new version
```
- **Pros:** Clear; explicitly breaks compatibility
- **Cons:** Multiple code paths; duplication

### Header Versioning
```http
GET /users
Accept: application/vnd.example.v2+json
```
- **Pros:** Single URL; version in metadata
- **Cons:** Clients often forget; harder to test

### Query String
```
GET /users?apiVersion=2
```
- Least common; similar issues as URL versioning

### Semantic Versioning (No Explicit Version)
```
GET /users
```
- Server returns resources; clients understand multiple versions
- Use **field deprecation** signals (headers, responses)
- Example: `Deprecation: true`, `Sunset: 2026-12-31`

**Best practice:** Avoid versioning by designing backward-compatible APIs. Add fields, deprecate gradually, remove only after long notice period. Use URL versioning only if incompatible change is unavoidable.

---

## HATEOAS: Hypermedia As The Engine Of State

Principle: API responses include links to next actions (state transitions).

```json
{
  "id": 42,
  "name": "Alice",
  "_links": {
    "self": { "href": "/users/42" },
    "posts": { "href": "/users/42/posts" },
    "edit": { "href": "/users/42", "method": "PUT" },
    "delete": { "href": "/users/42", "method": "DELETE" }
  }
}
```

- Client discovers available actions from response, not hardcoded URL knowledge
- Enables API evolution: endpoints can move without breaking clients
- Used in REST best practices; rarely adopted (complexity vs. benefit)

---

## API Gateway Patterns

Central entry point for all client requests:

```
Client → API Gateway → Service1, Service2, Service3
```

### Responsibilities
- **Routing:** Direct request to correct backend service
- **Authentication:** Validate token/session; reject unauthorized
- **Rate limiting:** Per-user, per-IP quota enforcement
- **Logging/monitoring:** Centralized request/response logging
- **Request/response transformation:** Format conversion, field mapping
- **Caching:** Cache responses to reduce backend load
- **Circuit breaker:** Fail gracefully if backend unavailable

### Trade-offs
- **Centralization:** Single point of failure; careful deployment needed
- **Latency:** Gateway adds ~10-50ms overhead
- **Complexity:** Must handle all edge cases (timeout, retry, errors)

---

## Rate Limiting & Pagination

### Rate Limiting
Control request rate to prevent abuse:

```http
HTTP/1.1 429 Too Many Requests
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1645000000 (Unix timestamp)
```

**Strategies:**
- Per-user (authenticated)
- Per-IP (anonymous)
- Global (total across all clients)
- Per-endpoint (some endpoints cost more)

**Algorithms:**
- Token bucket: refill N tokens per second; each request costs 1
- Leaky bucket: smooth rate; refill gradually
- Sliding window: count requests in last N seconds

### Pagination
Avoid returning massive result sets:

```http
GET /posts?page=2&limit=20

{
  "data": [...],
  "page": 2,
  "total": 453,
  "nextPage": 3,
  "prevPage": 1,
  "pageSize": 20
}
```

**Cursor-based (preferred for large datasets):**
```http
GET /posts?cursor=abc123&limit=20

{
  "data": [...],
  "nextCursor": "def456",
  "prevCursor": "xyz789"
}
```
- Stable across insertions (cursor points to position, not offset)
- Efficient for large datasets; no offset counting required

---

## Mental Model: When to Use Which Pattern

| Pattern | Use Case | Latency | Complexity | Caching |
|---------|----------|---------|-----------|---------|
| **REST** | CRUD operations | ~200ms | Low | HTTP-friendly |
| **GraphQL** | Flexible queries, reducing over-fetch | ~150ms | Medium | Query-based cache |
| **gRPC** | Internal services, streaming | ~10ms | Medium | Manual |
| **WebSocket** | Real-time bidirectional | ~1ms | Medium | Not applicable |
| **SSE** | Server push, simple | ~100ms | Low | Not applicable |
| **Webhook** | Event notification, async | ~500ms+ | High | Not applicable |

---

## See Also
- [api-design.md](api-design.md) — REST design principles, resource modeling
- [architecture-api-gateway.md](architecture-api-gateway.md) — Gateway patterns, routing, middleware
- [networking-http.md](networking-http.md) — HTTP semantics, caching, headers
- [networking-websockets.md](networking-websockets.md) — WebSocket protocol details
- [system-design-distributed.md](system-design-distributed.md) — Scaling, consistency, service communication