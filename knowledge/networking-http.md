# HTTP — Protocol Evolution, Methods, Headers, Caching & Content Negotiation

## Overview

HTTP is the foundation of web communication. The protocol has evolved from a simple one-line request model to a sophisticated system supporting streaming, multiplexing, server push, and 0-RTT connection establishment. Understanding the semantics, performance characteristics, and semantic guarantees of each version informs API design, caching strategy, and deployment decisions.

## HTTP/0.9 Through HTTP/1.0

**HTTP/0.9** (1991) was minimal: a client sent a single line (`GET /resource`), the server replied with HTML, and the connection closed.

**HTTP/1.0** (1996) added:
- Request methods: `GET`, `POST`, `HEAD`
- Status line with code and reason phrase
- Headers (e.g., `Content-Type`, `Content-Length`)
- Connection close after each request (stateless, but expensive)

Each request required a new TCP handshake and connection teardown, creating significant latency overhead for document-heavy pages.

## HTTP/1.1 — Persistent Connections & Pipelining

**HTTP/1.1** (1997, RFC 2616) moved the web from transactional to streaming architecture:

- **Keep-Alive / Connection Reuse**: Connections stay open by default. `Connection: keep-alive` (now implied) reduced handshake overhead for multiple requests.
- **Pipelining**: Clients could send multiple requests without waiting for responses. Servers still processed and responded sequentially.
- **Chunked Transfer Encoding**: `Transfer-Encoding: chunked` allowed servers to send data without knowing content length (e.g., streaming logs, database cursors).
- **Host Header**: Required to support virtual hosts on shared IP addresses.
- **Caching Semantics**: Full Cache-Control model with directives like `max-age`, `s-maxage`, `must-revalidate`, `Expires`, `ETag`, and `Last-Modified`.
- **Range Requests**: `Range: bytes=100-200` allowed partial resource retrieval and resumable downloads.
- **Content Negotiation**: `Accept*` headers (Content-Type, Language, Encoding) allowed clients to express preferences.

### HTTP/1.1 Limitations

Pipelining was rarely used due to **head-of-line blocking**: if response N was delayed, responses N+1, N+2 queued behind it, even if uncorrelated. Browsers defaulted to opening 6-8 parallel TCP connections per domain—wasteful and latency-amplifying.

## HTTP/2 — Binary Framing & Multiplexing

**HTTP/2** (2015, RFC 7540) reimagined transport efficiency:

- **Binary Framing**: Replaced text protocol with length-prefixed binary frames. Parsers became simpler and faster; no more counting newlines.
- **Multiplexing Over Single Connection**: All requests and responses interleave on one TCP connection. No head-of-line blocking at the HTTP layer (though TCP retransmission still blocks).
- **Server Push**: Server could proactively send resources (`PUSH_PROMISE` frame) before client requested them, reducing round-trip latencies.
- **Header Compression**: HPACK algorithm compressed repetitive headers (which requests repeat 90% of the time) to ~20% of original size.
- **Stream Prioritization**: Clients could assign weights and dependencies to streams, guiding server resource allocation.

HTTP/2 adopted a **predefined pseudo-header** scheme (`:method`, `:path`, `:scheme`, `:authority`) in place of the request line, maintained backward compatibility with HTTP/1.1 via the HTTP Upgrade mechanism.

### HTTP/2 Limitations

Still layered on TCP. A single lost packet causes TCP retransmission and blocks all multiplexed streams (head-of-line blocking at the transport layer). Server push proved less effective than expected (clients already cached; browsers had to implement complex heuristics to avoid bloat). HPACK complexity introduced security risks (Huffman codes + compression ratio side channels).

## HTTP/3 — QUIC Transport & 0-RTT

**HTTP/3** (2022, RFC 9114) replaces TCP with **QUIC** (RFC 9000), a UDP-based protocol:

- **0-RTT Connection Establishment**: Connection state cached on client; first packet contains encrypted data (no separate handshake round trip).
- **Native Multiplexing Without Head-of-Line Blocking**: QUIC streams are independent at the transport layer. Loss of packet for stream A does not delay stream B.
- **Connection Migration**: Connection persists across network changes (WiFi → cellular). Identified by connection ID, not IP:port tuple.
- **Packet Loss Recovery**: QUIC's congestion control and loss detection run at the protocol level, not delegated to OS TCP stack (more responsive).
- **Header Compression**: Still uses HPACK; reduces redundancy further.

HTTP/3 framing and semantics are nearly identical to HTTP/2; the main improvement is QUIC reliability and latency characteristics.

### Adoption Challenges

QUIC requires NAT traversal (UDP middleboxes often rate-limit). Deployment on shared hosting or behind aggressive firewalls is complex. eBPF and kernel-space QUIC stacks (like Cloudflare's) bypass userspace overhead but add operational complexity.

## Request and Response Model

```
Client                                    Server
  │
  ├─ TCP handshake (1 RTT)
  │
  ├─ TLS handshake (1-2 RTT, if applicable)
  │
  ├─ HTTP request (method, URI, headers, body)
  │
  ├───────────────────────────>
  │
  │                              Parse request
  │                              Lookup resource
  │                              Apply auth, ACL
  │                              Compute response
  │
  │                              HTTP response (status, headers, body)
  │
  │<───────────────────────────
  │
  └─ Connection Close or Keep-Alive → Ready for next request
```

## HTTP Methods

| Method | Semantics | Idempotent | Cacheable | Body |
|--------|-----------|-----------|-----------|------|
| **GET** | Retrieve resource; no state change | ✓ | ✓ | No (convention) |
| **HEAD** | Like GET, but no response body (for preconditions) | ✓ | ✓ | No |
| **POST** | Submit data; may create or trigger action; not idempotent | ✗ | Conditional | Yes |
| **PUT** | Replace resource at URI; idempotent creation/update | ✓ | ✗ | Yes |
| **DELETE** | Remove resource; idempotent | ✓ | ✗ | No |
| **PATCH** | Partial modification; not inherently idempotent | ✗ | ✗ | Yes |
| **OPTIONS** | Describe communication capabilities (CORS preflight) | ✓ | ✗ | No |
| **TRACE** | Echo request (security risk; often disabled) | ✓ | ✗ | No |
| **CONNECT** | Establish tunnel (e.g., HTTPS proxy) | ✗ | ✗ | No |

**Idempotency** means repeating the request N times has the same effect as once. POST is *not* idempotent by default (N submittal triggers N distinct actions).

## Status Codes

HTTP status codes signal the outcome of a request.

### 1xx (Informational)

- `100 Continue`: Client should send body. Usually generated by reverse proxies to reduce latency on large PUTs.
- `101 Switching Protocols`: HTTP Upgrade successful (e.g., to WebSocket).

### 2xx (Success)

- `200 OK`: Resource retrieved or action succeeded.
- `201 Created`: Resource created; `Location` header provides URI.
- `202 Accepted`: Request accepted for async processing; no guarantee of completion.
- `204 No Content`: Success, but no content to return (e.g., DELETE).
- `206 Partial Content`: Range request satisfied; `Content-Range` header indicates byte range.

### 3xx (Redirection)

- `301 Moved Permanently`: Resource moved; cache new URI.
- `302 Found`: Temporary redirect; client should resend to original URI next time.
- `304 Not Modified`: Cached copy is still valid (response to conditional request).
- `307 Temporary Redirect`: Like 302, but client must preserve method (POST stays POST).
- `308 Permanent Redirect`: Like 301, but preserve method.

### 4xx (Client Error)

- `400 Bad Request`: Malformed request.
- `401 Unauthorized`: Authentication required.
- `403 Forbidden`: Authenticated, but not authorized to access resource.
- `404 Not Found`: Resource does not exist.
- `405 Method Not Allowed`: Method not permitted for this resource.
- `409 Conflict`: Incompatible request state (e.g., version mismatch in PUT).
- `410 Gone`: Resource permanently deleted; don't retry.
- `429 Too Many Requests`: Rate limit exceeded.

### 5xx (Server Error)

- `500 Internal Server Error`: Unexpected server error.
- `502 Bad Gateway`: Reverse proxy received invalid response from upstream.
- `503 Service Unavailable`: Server temporarily overloaded; client should retry.
- `504 Gateway Timeout`: Upstream did not respond in time.

## Headers: Request & Response

### Common Request Headers

- `Host`: Required in HTTP/1.1; domain name and optional port.
- `User-Agent`: Client identifier (browser, tool, bot).
- `Accept`: Preferred media types (`application/json, text/html`).
- `Accept-Language`: Preferred languages (`en-US, fr`).
- `Accept-Encoding`: Supported encodings (usually `gzip, deflate`).
- `Authorization`: Credentials (`Bearer <token>`, `Basic <base64>`).
- `Cookie`: Sent state from client.
- `If-Modified-Since`: Conditional; return 304 if not modified since date.
- `If-None-Match`: Conditional; return 304 if ETag matches.
- `Referer`: Referring page (privacy concerns; restricted by header policy).

### Common Response Headers

- `Content-Type`: MIME type (`application/json`, `text/html; charset=utf-8`).
- `Content-Length`: Size of body in bytes.
- `Content-Encoding`: Compression applied (`gzip`).
- `Cache-Control`: Caching directives (see Caching section).
- `Set-Cookie`: Store state on client.
- `Location`: Redirect target or created resource URI.
- `ETag`: Opaque resource version identifier.
- `Last-Modified`: Last modification date.
- `Server`: Server software identifier (information disclosure risk).
- `CORS Headers` (Access-Control-*): Browser same-origin policy exceptions.

## Content Negotiation

### By Media Type

Client sends `Accept: application/json, text/html;q=0.9` (quality factors weight preferences). Server chooses representation and responds with `Content-Type: application/json`.

### By Language

Client sends `Accept-Language: en-US, fr;q=0.8`. Server selects language variant and may include `Content-Language: en-US`.

### By Encoding

Client sends `Accept-Encoding: gzip, deflate`. Server compresses and includes `Content-Encoding: gzip`.

### Vary Header

`Vary: Accept, Accept-Language, Accept-Encoding` signals to caches that this response varies by these request headers, so caches must store separate copies for each combination.

## Persistent Connections

**HTTP/1.1** keeps connections alive by default. Connection closes if:
- Client or server sends `Connection: close`.
- Request times out (server-dependent, often 30-60 seconds).
- Idle time exceeds keepalive timeout.

Persistent connections reduce TCP handshake overhead. HTTP/2 and HTTP/3 eliminate the concept—multiplexing on a single connection is the default.

## Multiplexing (HTTP/2+)

HTTP/2 and HTTP/3 allow multiple logical requests and responses on the same connection simultaneously. Eliminates the need for domain sharding (splitting assets across subdomains to bypass parallel connection limits). Improves latency for document-heavy pages; reduces connection overhead.

## Server Push

HTTP/2 servers can proactively send resources before clients request them:

```
Server sends: PUSH_PROMISE (stream_id=1) for /style.css
Server sends: Pushed response (stream_id=2) headers, body
Client: If already cached, sends RST_STREAM to cancel push
```

Real-world effectiveness was lower than expected. Clients lack context (are they in a browser cache? have they been idle?), so pushing reduces efficiency (bloat). Modern practice favors hint-based approaches (Link: rel=preconnect).

## Caching Semantics

### Cache-Control Directives

**Response directives:**
- `public`: Cache stores this response publicly; shared caches (CDN) and browser caches.
- `private`: Only browser caches; not shared caches.
- `max-age=3600`: Cache valid for 3600 seconds (200 seconds, 5 minutes, 1 hour, etc.).
- `s-maxage=86400`: Shared cache max-age (overrides `max-age` for CDN).
- `no-cache`: Revalidate with origin before using (conditional GET with ETag/Last-Modified).
- `no-store`: Don't cache; fetch from origin every time.
- `must-revalidate`: After expiry, revalidate with origin; don't serve stale.
- `proxy-revalidate`: Like must-revalidate, but only for shared caches.
- `immutable`: Resource will not change; cache forever (used for versioned assets like `/v1.0.123-abc.js`).

**Request directives:**
- `max-age=0`: Force fresh copy (older than 0 seconds = expired).
- `no-cache`: Revalidate before using.
- `no-store`: Don't cache response.

### Conditional Requests

**ETag**: Opaque resource version string. Client includes `If-None-Match: "abcd1234"`. If ETag matches, server responds `304 Not Modified` (no body transmission).

**Last-Modified**: Resource modification date. Client includes `If-Modified-Since: Wed, 21 Oct 2025 07:28:00 GMT`. If not modified, server responds `304 Not Modified`.

Conditional requests reduce bandwidth when resource is unchanged.

### Freshness & Staleness

- **Fresh**: Cache age < max-age. Use directly without revalidation.
- **Stale**: Cache age ≥ max-age. Must revalidate before use (conditional GET).
- **Heuristic Freshness**: If no Cache-Control, use `(Last-Modified - Date) / 10` heuristic (rarely recommended).

## CORS (Cross-Origin Resource Sharing)

Browser enforces same-origin policy: scripts from `https://example.com` cannot fetch `https://api.other.com`. CORS allows controlled cross-origin requests:

1. Client sends preflight OPTIONS request.
2. Server responds with `Access-Control-Allow-Origin`, `Access-Control-Allow-Methods`, `Access-Control-Allow-Headers`.
3. Browser allows cross-origin request if credentials match.

Credentials (`Authorization` header, cookies) are not sent in cross-origin requests unless `credentials=include` and `Access-Control-Allow-Credentials: true`.

## Cookies

`Set-Cookie` instructs browser to store state:

```
Set-Cookie: session=abc123; Path=/; Domain=.example.com; Secure; HttpOnly; SameSite=Strict
```

- **Secure**: Only send over HTTPS.
- **HttpOnly**: JavaScript cannot access (reduces XSS impact).
- **SameSite**: Restrict sending in cross-site requests (Strict/Lax/None; None requires Secure).

Cookies are sent with every request to the domain, increasing payload size. Modern APIs prefer Bearer tokens in `Authorization` headers.

## See Also

- **networking-websockets** — Upgrade from HTTP 1.1; persistent bidirectional communication.
- **networking-tcp-ip** — TCP connection lifecycle; congestion control.
- **security-web-application** — CORS, cookie security, CSP headers.