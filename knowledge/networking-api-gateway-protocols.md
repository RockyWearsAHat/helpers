# API Gateway Protocols — REST, GraphQL, gRPC, tRPC, WebSocket, SSE, Long-Polling: When to Use Each

## Overview

Modern systems support multiple API protocols, each optimized for different use cases. REST is simple and cacheable; GraphQL is flexible; gRPC is fast for microservices; WebSocket and SSE push real-time data. Understanding the trade-offs—latency, throughput, cacheability, complexity, migration efforts—is essential for architecture decisions.

## REST: Simple, Cacheable, Stateless

REST uses HTTP verbs (GET, POST, PUT, DELETE) on stateless resources. It is the default for web APIs.

**Advantages:**
- Simple: mental model is clear (CRUD on resources)
- Cacheable: HTTP caches (browser, CDN) work out of the box
- Stateless: servers don't maintain session; horizontal scaling is easy
- Debuggable: curl, browser, Postman make testing trivial
- Standards: HTTP codes, status, headers are well-defined

**Disadvantages:**
- Over-fetching: A request for `GET /users/1` returns all fields, even if the client needs only `name` and `email`
- Under-fetching: A request for user + posts + comments requires three roundtrips (`/users/1`, `/users/1/posts`, `/posts/X/comments`)
- N+1 queries: Listing users then fetching each user's metadata causes N API calls
- Version sprawl: Changes to response schema force version management (`/v1/users`, `/v2/users`)

**Example:**
```
GET /users/1
→ { id: 1, name: "Alice", email: "a@example.com", age: 30, address: "...", ... }

POST /posts
{ title: "New post", userId: 1, content: "...", tags: [...] }
→ { id: 101, title: "...", createdAt: "...", views: 0, author: { id: 1, name: "Alice" } }
```

## GraphQL: Flexible, Single Endpoint, Complex to Deploy

GraphQL is a query language for APIs. Clients specify exactly what data they need; the server responds with only that data.

**Advantages:**
- No over-fetching: client requests `name` and `email`, receives only those fields
- No under-fetching: batch queries avoid roundtrips
- Single endpoint: all queries to `/graphql`
- Strongly typed schema: clients discover fields via introspection
- N+1 solution: with batching/dataloader, resolving user + posts is a single query

**Disadvantages:**
- Complexity: resolvers must be carefully engineered; naive implementations cause exponential queries
- No standard caching: HTTP caches don't work for POST requests; requires client-side cache
- Requires gateway: API gateways must parse and validate GraphQL, adding CPU cost
- Slow introspection: large schemas cause slow IDE autocomplete
- Not cacheable by default: queries are POST (not GET), so browser/CDN caches are bypassed

**Example:**
```graphql
query {
  user(id: 1) {
    name
    email
    posts {
      title
      comments(first: 5) {
        body
        author { name }
      }
    }
  }
}
```

Single roundtrip; only requested fields; no extraneous data.

## gRPC: High Performance for Microservices

gRPC uses protocol buffers (protobuf) for serialization and HTTP/2 multiplexing for transport.

**Advantages:**
- Fast: binary serialization is compact; parsing is CPU-efficient
- Multiplexing: multiple requests share one TCP connection via HTTP/2 streams
- Streaming: bidirectional streaming for real-time data, logs, etc.
- Type safe: protobuf definitions compile to strongly typed stubs
- Low latency: no JSON parsing overhead
- Integration: gRPC-to-JSON gateways bridge to REST clients

**Disadvantages:**
- Not human-readable: binary protobuf requires tools to inspect
- Limited browser support: grpc-web proxies are required; native gRPC in browser is not possible
- Operational complexity: requires protobuf tooling, versioning discipline, schema registry
- Not cacheable: gRPC is typically used for internal services (behind API gateway), not for caching (no HTTP caching layer)
- Learning curve: protobuf syntax, service definitions, code generation

**Example:**
```protobuf
service UserService {
  rpc GetUser(UserId) returns (User);
  rpc ListUsers(Empty) returns (stream User);
  rpc UpdateUser(User) returns (UpdateResult);
}

message User {
  int32 id = 1;
  string name = 2;
  string email = 3;
}
```

Compiled to Go, Python, Java, Rust, etc. stubs with full typing.

## tRPC: TypeScript Type Safety End-to-End

tRPC is a TypeScript-specific framework (client + server in same TS codebase) that eliminates the need for type duplication between frontend and backend.

**Advantages:**
- Type safe end-to-end: defining a backend procedure automatically types the client call
- No code generation: types flow directly from server to client via TypeScript
- Lightweight: small dependencies, fast startup
- Easy testing: mock server endpoints trivially
- Flexible transport: works over HTTP, WebSocket, or custom protocols

**Disadvantages:**
- TypeScript only: requires same or compatible language for both client and server
- Not language-agnostic: not suitable for polyglot microservices
- Less mature ecosystem: fewer tools, conventions, integrations than REST or gRPC
- Not suitable for mobile/web client in different teams: typing is end-to-end within a monorepo; external clients use JSON equivalently to REST

**Example:**
```typescript
// Backend
const router = t.router({
  getUser: t.procedure
    .input(z.object({ id: z.number() }))
    .query(async ({ input }) => { return db.user.findUnique({ where: { id: input.id } }); })
});

// Frontend (in same TypeScript project)
const user = await trpc.getUser.query({ id: 1 });
// ← user is typed as { id: number; name: string; ... }
```

No request/response types to define; types inferred from both ends.

## WebSocket: Bidirectional Real-Time

WebSocket establishes a persistent TCP connection over which both client and server can send messages at any time.

**Advantages:**
- Bidirectional: server sends data to client without client request
- Low latency: no HTTP overhead per message (after handshake)
- Persistent connection: efficient for many updates

**Disadvantages:**
- Requires explicit framing: unlike HTTP with its headers/body structure, WebSocket frames are application-defined
- No automatic request-response correlation: must implement message ID tracking for request-response patterns
- Hard to debug: browser DevTools don't inspect WebSocket frames as easily as HTTP
- Horizontal scaling complexity: load balancers must maintain sticky sessions (routing all messages from a client to the same backend instance)
- Firewall complexity: some network policies block WebSocket

**Use when:**
- Chat, notifications, real-time dashboards require server-push
- Latency below 100ms is critical

## Server-Sent Events (SSE): One-Way Server Push

SSE is simpler than WebSocket: the server sends events to the client over an HTTP connection that stays open.

**Advantages:**
- Single direction: server → client only (no client sending on same connection)
- HTTP-friendly: uses standard HTTP GET; works through firewalls/proxies
- Reconnection: built-in for automatic reconnect on network interruption
- Simple API: `new EventSource(url)` in JavaScript

**Disadvantages:**
- One-way only: client must open a separate HTTP connection to send data to server
- Browser limit: browsers limit concurrent EventSource connections per domain (6); multiple SSE streams are limited
- Not suitable for high-frequency updates: per-message HTTP overhead remains

**Use when:**
- Server broadcasts to many clients (notifications, logs, metrics)
- Client rarely sends; updates are mostly unidirectional

## Long-Polling: Fallback When WebSocket/SSE Unavailable

Client polls the server repeatedly for new data. When the server has data, it responds; otherwise, it waits before responding.

**Advantages:**
- Works anywhere: HTTP only; no WebSocket protocol requirement

**Disadvantages:**
- High latency: bounded by poll interval (typically 1-30 seconds)
- High bandwidth: each poll is an HTTP request/response
- Server resource waste: server threads block waiting for data

**Use only as a fallback** when WebSocket and SSE are unavailable.

## Protocol Comparison Table

| Protocol         | Latency | Throughput | Cacheability | Overhead | Use Case |
|------------------|---------|-----------|--------------|----------|----------|
| REST + HTTP      | High    | Low       | High         | Medium   | CRUD APIs, public APIs |
| GraphQL          | High    | Med       | Low          | High     | Complex data fetching |
| gRPC             | Very Low | High      | None         | Very Low | Microservices, streaming |
| tRPC             | High    | Low       | Low          | Low      | TS monorepo clients |
| WebSocket        | Very Low | High      | None         | Very Low | Chat, real-time games |
| SSE              | Low     | Med       | None         | Low      | Notifications, streams |
| Long-Poll        | High    | Very Low  | Medium       | High     | Fallback only |

## Migration Between Protocols

**REST → GraphQL:**
Implement a GraphQL endpoint alongside REST. Gradually migrate clients. Schema stitching can compose multiple REST services into a single GraphQL graph.

**REST → gRPC:**
Use grpc-gateway or Envoy to translate HTTP/REST requests to gRPC. Allows REST clients to work with gRPC backends transparently.

**HTTP → WebSocket:**
Establish WebSocket after authentication over HTTP. Some systems maintain both for connection stability (retry WebSocket; fallback to polling).

## Protocol Bridging

**GraphQL over gRPC:** Translate GraphQL queries to gRPC procedures via a gateway. Allows GraphQL clients to query gRPC services.

**gRPC-to-REST:** Use grpc-gateway (Go) or similar to auto-generate REST endpoints from protobuf definitions. Request maps to gRPC call; response marshalled to JSON.

**WebSocket over REST:** Some architectures use Server-Sent Events as a fallback when WebSocket is blocked. Browser attempts WebSocket first; on failure, switches to SSE or long-polling.

## Choosing a Protocol

1. **Public, cacheable API?** REST.
2. **Complex, flexible queries?** GraphQL. (Trade-off: complexity, requires gateway, needs caching strategy.)
3. **Internal microservices, speed critical?** gRPC.
4. **Full-stack TypeScript, monorepo?** tRPC.
5. **Real-time, low-latency bidirectional?** WebSocket.
6. **Server broadcasts, client rarely sends?** SSE.
7. **No WebSocket support?** Long-poll (worst choice, only fallback).

## See Also

- `web-api-patterns.md` — GraphQL patterns, resolver design, federation
- `api-rest-maturity.md` — REST API design maturity model
- `architecture-api-gateway.md` — API gateway patterns, request transformation
- `networking-websockets.md` — WebSocket protocol, framing, scaling
- `web-real-time-patterns.md` — Real-time architectures