# Networking Protocols — From Packets to Applications

## The Layered Model (OSI vs Reality)

```
OSI Model                  TCP/IP (What We Actually Use)
─────────                  ─────────────────────────────
7. Application             Application (HTTP, DNS, SMTP, SSH)
6. Presentation               ↑ (TLS lives here conceptually)
5. Session                     ↑
4. Transport               Transport (TCP, UDP, QUIC)
3. Network                 Internet (IP, ICMP)
2. Data Link               Link (Ethernet, Wi-Fi, ARP)
1. Physical                    ↑ (cables, radio)
```

## TCP — The Reliable Workhorse

### Three-Way Handshake
```
Client          Server
  |--- SYN ------->|     "I want to connect"
  |<-- SYN-ACK ----|     "OK, I acknowledge"
  |--- ACK ------->|     "Connection established"
```

### Key Mechanisms
- **Sequence numbers**: Every byte is numbered. Receiver knows what it got and what's missing.
- **Acknowledgments**: Receiver tells sender "I got everything up to byte N."
- **Retransmission**: If no ACK within timeout, resend.
- **Flow control**: Receiver advertises window size ("I can buffer N more bytes").
- **Congestion control**: Slow start → congestion avoidance → fast retransmit/recovery.

### TCP Gotchas
- **Head-of-line blocking**: One lost packet blocks all subsequent data until retransmitted.
- **Nagle's algorithm**: Buffers small writes into larger packets. Adds latency. Disable with `TCP_NODELAY` for interactive protocols.
- **TIME_WAIT**: After closing, socket stays in TIME_WAIT for 2×MSL (~60s). Can exhaust ports under high connection churn.
- **Keep-alive**: TCP connections can silently die (half-open). Use TCP keep-alive or application-level heartbeats.

## UDP — Fire and Forget

```
No handshake, no acknowledgments, no ordering, no flow control.
Just: "here's a packet, good luck."
```

**Use when:** Low latency > reliability. Gaming, VoIP, video streaming, DNS queries.
**Build on top:** QUIC, DTLS, RTP all use UDP as their transport.

## HTTP Evolution

### HTTP/1.1 (1997)
- Text-based protocol
- One request per TCP connection at a time (pipelining failed in practice)
- Workaround: browsers open 6 parallel TCP connections per host
- Persistent connections (`Connection: keep-alive`)

### HTTP/2 (2015)
- Binary framing layer
- **Multiplexing**: Multiple requests/responses over a single TCP connection
- **Server push**: Server can proactively send resources
- **Header compression** (HPACK): Eliminates redundant headers
- **Stream prioritization**: Client hints which resources matter most
- **Still TCP underneath**: Head-of-line blocking at the TCP layer remains

### HTTP/3 (2022)
- **Built on QUIC** (UDP-based, not TCP)
- **No head-of-line blocking**: Lost packets only affect their stream
- **0-RTT connection establishment**: Resuming connections with zero round trips
- **Built-in TLS 1.3**: Encryption mandatory, integrated into the protocol
- **Connection migration**: Survives IP address changes (mobile switching networks)

### HTTP Methods
```
GET     Retrieve a resource (idempotent, cacheable)
POST    Create / submit data (not idempotent)
PUT     Replace entire resource (idempotent)
PATCH   Partial update (not necessarily idempotent)
DELETE  Remove resource (idempotent)
HEAD    GET without body (check headers/existence)
OPTIONS Pre-flight for CORS, discover methods
```

### Status Codes That Matter
```
200 OK                    Success
201 Created               Resource created (POST/PUT)
204 No Content            Success, no body (DELETE)
301 Moved Permanently     URL changed, update bookmarks
302 Found                 Temporary redirect
304 Not Modified          Cached version is fine
400 Bad Request           Client sent invalid data
401 Unauthorized          Authentication required (misleading name)
403 Forbidden             Authenticated but not authorized
404 Not Found             Resource doesn't exist
405 Method Not Allowed    Wrong HTTP method
409 Conflict              State conflict (e.g., edit collision)
422 Unprocessable Entity  Validation failed (WebDAV but widely used)
429 Too Many Requests     Rate limited
500 Internal Server Error Server bug
502 Bad Gateway           Upstream server error
503 Service Unavailable   Overloaded or maintenance
504 Gateway Timeout       Upstream server timeout
```

## WebSocket

```
1. HTTP Upgrade handshake (single HTTP request)
2. Bidirectional, full-duplex communication over single TCP connection
3. Low overhead: 2-byte frame header vs HTTP headers on every message
```

```javascript
// Client
const ws = new WebSocket('wss://example.com/ws');
ws.onopen = () => ws.send('Hello');
ws.onmessage = (event) => console.log(event.data);
ws.onclose = () => console.log('Disconnected');
```

**Use for:** Real-time apps (chat, live updates, gaming). NOT for: request-response patterns (use HTTP).
**Alternative:** Server-Sent Events (SSE) — simpler, one-way (server→client), auto-reconnects, works over HTTP/2.

## gRPC

- **Built on HTTP/2** — binary protocol, multiplexed
- **Protocol Buffers** (protobuf) for serialization — smaller, faster than JSON
- **Strongly typed** — schema defined in `.proto` files, code generated for any language
- **Streaming**: Unary, server-streaming, client-streaming, bidirectional streaming

```protobuf
// user.proto
syntax = "proto3";

service UserService {
    rpc GetUser (GetUserRequest) returns (User);
    rpc ListUsers (ListUsersRequest) returns (stream User);  // Server streaming
}

message GetUserRequest { int64 id = 1; }
message User {
    int64 id = 1;
    string name = 2;
    string email = 3;
}
```

**Use for:** Service-to-service communication (microservices). NOT for: browser clients (no native browser gRPC support without grpc-web proxy).

## DNS — The Phone Book of the Internet

```
Browser → Recursive Resolver → Root (.com) → TLD (example.com) → Authoritative → IP
                ↓
         Local cache (TTL-based)
```

### Record Types
```
A       IPv4 address          example.com → 93.184.216.34
AAAA    IPv6 address          example.com → 2606:2800:220:1:248:...
CNAME   Alias to another name www.example.com → example.com
MX      Mail server           example.com → mail.example.com (priority 10)
TXT     Arbitrary text        SPF, DKIM, domain verification
NS      Name server           example.com → ns1.example.com
SOA     Zone authority         Serial, refresh, retry, expire, minimum TTL
SRV     Service location       _http._tcp.example.com → port 80, target
```

### DNS Debugging
```bash
dig example.com A              # Query A record
dig +short example.com         # Just the IP
dig @8.8.8.8 example.com      # Use specific resolver
dig +trace example.com         # Full delegation chain
nslookup example.com           # Simpler alternative
host example.com               # Simplest
```

## TLS — Transport Layer Security

### TLS 1.3 Handshake (1 round trip)
```
Client                          Server
  |--- ClientHello + KeyShare -->|   Supported ciphers + key material
  |<-- ServerHello + KeyShare ---|   Chosen cipher + key material
  |<-- {Certificate} -----------|   Server cert (encrypted!)
  |<-- {Finished} --------------|
  |--- {Finished} ------------->|
  |===== Encrypted data =======>|   Application data flows
```

**TLS 1.3 improvements over 1.2:**
- 1-RTT handshake (was 2-RTT)
- 0-RTT resumption (with replay risk)
- Removed insecure algorithms (RC4, 3DES, SHA-1, static RSA)
- Forward secrecy mandatory (ephemeral key exchange)
- Encrypted certificates (observer can't see which site)

### Certificate Chain
```
Root CA (in browser/OS trust store)
  └── Intermediate CA
        └── Your certificate (for example.com)
```
Let's Encrypt provides free, automated certificates via ACME protocol.

## REST vs GraphQL vs gRPC — When to Use What

| Aspect | REST | GraphQL | gRPC |
|--------|------|---------|------|
| Data format | JSON (text) | JSON (text) | Protobuf (binary) |
| Schema | OpenAPI (optional) | SDL (required) | .proto (required) |
| Over/under-fetching | Common problem | Solved (query what you need) | N/A (defined messages) |
| Caching | HTTP caching (easy) | Hard (POST everything) | Custom |
| Browser support | Native | Native | Needs proxy |
| Streaming | SSE/WebSocket | Subscriptions | Native |
| Best for | Public APIs, CRUD | Flexible frontend queries | Microservices, high perf |

## Common Networking Mistakes

1. **Not handling partial reads**: TCP is a byte stream. A `read()` call may return partial data. Always loop until you have a complete message.
2. **DNS caching forever**: JVM caches DNS forever by default. In cloud environments where IPs change, this causes outages.
3. **No timeouts**: Always set connect timeout, read timeout, and overall timeout. Default of "wait forever" = production hang.
4. **Ignoring backpressure**: If producer is faster than consumer, buffer grows until OOM. Use flow control.
5. **HTTP/1.1 without keep-alive awareness**: Opening a TCP+TLS connection per request adds ~100ms+ latency.
6. **Not considering MTU**: Packet fragmentation causes performance issues. TCP handles this; UDP users must care.
7. **Trusting client headers blindly**: `X-Forwarded-For`, `Host` — can be spoofed. Validate at the edge.

---

*Networking is layers of abstraction. When something breaks, start at the bottom (can you ping it?) and work up.*
