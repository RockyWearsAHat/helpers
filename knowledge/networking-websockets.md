# WebSockets — Protocol, Handshake, Framing, Extensions & Scaling

## Overview

WebSocket is a protocol that provides full-duplex communication over a single TCP connection, standardized as RFC 6455 (2011). It solves the limitation of HTTP request-response polling by allowing servers to push data to clients without waiting for requests. Widely used for real-time applications: chat, live notifications, collaborative editing, gaming, financial tickers.

## The HTTP Upgrade Path

WebSocket begins as an HTTP request but upgrades the connection protocol mid-stream:

```
Client → Server (HTTP request):
  GET /chat HTTP/1.1
  Host: server.example.com
  Upgrade: websocket
  Connection: Upgrade
  Sec-WebSocket-Key: x3JJHMbDL1EzLkh9GBhXDw==
  Sec-WebSocket-Version: 13
  Origin: http://example.com
  (optional: Sec-WebSocket-Protocol: subprotocol1, subprotocol2)
  (optional: Sec-WebSocket-Extensions: permessage-deflate; ...)
```

Server accepts:

```
Server → Client (HTTP response):
  HTTP/1.1 101 Switching Protocols
  Upgrade: websocket
  Connection: Upgrade
  Sec-WebSocket-Accept: HSmrc0sMlYUkAGmm5OPpG2HaGWk=
  (optional: Sec-WebSocket-Protocol: subprotocol1)
  (optional: Sec-WebSocket-Extensions: permessage-deflate; ...)
```

The `Sec-WebSocket-Accept` is computed as `Base64(SHA-1(Sec-WebSocket-Key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"))`. This handshake prevents accidental WebSocket requests from caches and naive HTTP proxies.

After 101 response, the TCP connection no longer speaks HTTP. Both sides use WebSocket framing protocol (binary, not text).

## Transport & Reliability

WebSocket is layered on TCP, inheriting TCP's reliability: ordered, no duplicates, retransmission on loss. Unlike HTTP, which is stateless, WebSocket is stateful. A single TCP connection drop terminates the WebSocket; client must reconnect.

## Framing Protocol (RFC 6455)

Each WebSocket message is composed of one or more **frames**. A frame structure:

```
  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
 +-+-+-+-+-------+-+-------------+-------------------------------+
 |F|R|R|R| opcode|M| Payload len |    Extended payload length     :
 |I|S|S|S|   (4) |A|    (7)      :         (0 or 2+8 bytes)        :
 |N|V|V|V|       |S|            :                                 :
 | |1|2|3|       |K|            :                                 :
 +-+-+-+-+-------+-+-------------+-------------------------------+
 :                     Masking-key (0 or 4 bytes)                 :
 :                                                                 :
 +--------------------------------------------------------------+
 :                    Payload Data (x bytes)                     :
 +--------------------------------------------------------------+
```

**Key fields:**
- **FIN** (bit 0): Final frame of message (1 = yes; 0 = continuation).
- **RSV1-3** (bits 1-3): Reserved; 0 unless extensions use them (e.g., permessage-deflate sets RSV1=1 if frame is compressed).
- **Opcode** (bits 4-7): Frame type:
  - `0x0` = Continuation
  - `0x1` = Text (UTF-8)
  - `0x2` = Binary
  - `0x8` = Close
  - `0x9` = Ping
  - `0xA` = Pong
- **MASK** (bit 8): 1 if payload is masked (client → server must be masked; server → client must not be masked).
- **Payload Length**: If ≤ 125, encoded in 7 bits. If 126, next 2 bytes are length (16-bit). If 127, next 8 bytes are length (64-bit).
- **Masking Key**: 4 bytes sent by client (XOR applied to each byte to prevent cache poisoning by proxies).
- **Payload Data**: Masked (for client frames) or unmasked (for server frames).

**Masking** is required for client-to-server frames to prevent intermediary proxies from misinterpreting frame data as HTTP. Every byte of payload is XORed with a rotating 4-byte key. Example:

```
Masking-Key: [0x37, 0xfa, 0x21, 0x3d]
Payload byte 0: original XOR 0x37
Payload byte 1: original XOR 0xfa
Payload byte 2: original XOR 0x21
Payload byte 3: original XOR 0x3d
Payload byte 4: original XOR 0x37 (repeats)
...
```

## Message vs Frame Fragmentation

A message can span multiple frames. First frame has opcode `0x1` (text) or `0x2` (binary) with FIN=0. Continuation frames have opcode `0x0` with FIN=0. Final frame has FIN=1.

```
Server sends fragmented text message "Hello, World!":
  Frame 1: FIN=0, opcode=0x1, "Hello, "
  Frame 2: FIN=0, opcode=0x0, ""
  Frame 3: FIN=1, opcode=0x0, "World!"
```

Fragmentation is useful for streaming large data (e.g., file uploads, database cursors) without buffering entire payload in memory.

## Control Frames

### Ping/Pong

Keepalive mechanism. Either endpoint can send a `Ping` frame (opcode `0x9`). Recipient automatically responds with `Pong` frame (opcode `0xA`) with identical payload. Useful for detecting stale connections or network failures (no application-level heartbeat needed).

```
Client: Ping (opcode 0x9, payload: epoch timestamp)
Server: Pong (opcode 0xA, payload: echo timestamp) → Client computes latency
```

### Close Handshake

Graceful shutdown via `Close` frames (opcode `0x8`):

```
Initiator → Recipient: Close (optional payload: status code + reason)
Recipient → Initiator: Close (echo status code + reason)
TCP connection closes
```

Close codes (2-byte status):
- `1000` — Normal closure.
- `1001` — Going away (server shutdown).
- `1002` — Protocol error.
- `1003` — Unsupported data type.
- `1006` — Abnormal closure (TCP failure; cannot be sent, only received).
- `1008` — Policy violation.
- `1009` — Message too big.
- `1010` — Mandatory extension.
- `1011` — Internal server error.

## Subprotocols

WebSocket can support multiple application-level protocols over the same transport. Client specifies accepted subprotocols during handshake:

```
Sec-WebSocket-Protocol: chat, superchat
```

Server chooses one:

```
Sec-WebSocket-Protocol: chat
```

Each subprotocol is a string (e.g., `chat`, `xmpp`, `graphql-ws`). Some libraries like Socket.IO, ProtocolBuffers-over-WebSocket, or GraphQL subscriptions use subprotocol negotiation to select serialization/interpretation.

## Extensions

Extensions negotiate compression, encryption, or other negotiated capabilities in the handshake:

```
Sec-WebSocket-Extensions: permessage-deflate; client_max_window_bits
```

### Permessage-Deflate (RFC 7692)

Compresses frames using DEFLATE (gzip-style) algorithm. Reduces bandwidth up to 50-80% for text messages.

**Mechanism**: Before sending a frame, compress the payload. Set RSV1 bit to indicate compression. On receipt, if RSV1=1, decompress the payload.

**Parameters** (negotiated in handshake):
- `server_no_context_takeover` — Server does not retain compression context between messages (reduces memory).
- `client_no_context_takeover` — Client does not retain context.
- `server_max_window_bits=15` — Server compression window size (default 15; 8-15 allowed).
- `client_max_window_bits` — Client window size.

**Trade-off**: Compression adds CPU overhead but reduces bandwidth. Most beneficial for text (JSON, XML), less so for already-compressed data (images, video).

### Custom Extensions

Extensions are strings with optional parameters. A hypothetical example:

```
Sec-WebSocket-Extensions: my-crypto; algorithm=AES128, integrity
```

Server implements the extension logic (e.g., encrypt/decrypt frames). RFC 6455 defines no security extension (developers should use TLS instead), but some implementations add per-message encryption for end-to-end security in environments where TLS termination occurs upstream.

## Security Considerations

### Same-Origin Policy

WebSocket is subject to browser same-origin policy. Script from `https://example.com` can open a WebSocket to `https://example.com:8080` but not `https://other.com` (unless server sends CORS-equivalent headers, which WebSocket doesn't have). The `Origin` header during handshake is sent but not a formal CORS check; security depends on proper origin validation in server code.

### TLS/HTTPS

Use `wss://` (WebSocket Secure) over TLS, not `ws://` in production. `wss://` provides:
- Encryption (eavesdropping resistant).
- Integrity guarantees.
- Server authentication.
- Masking is required by spec (client frames) but HTTPS Eavesdropping not a concern.

### Denial of Service

WebSocket connections are stateful and consume server resources (memory, file descriptors). Attackers can open many connections and keep them idle, exhausting resources. Mitigate with:
- Connection limits per IP.
- Idle timeout (close connections after N seconds without frames).
- Message rate limiting.
- Backpressure: stop reading from a client if unable to keep up.

### Message Injection

Unlike HTTP headers, WebSocket frames can contain arbitrary binary data. Validate and sanitize frame payloads in application code. Example: frame arrives as text, JSON-decoded, then used in SQL query without parameterization → SQL injection.

## Scaling Patterns

### Stateless Architecture

WebSocket is stateful; a single connection is tied to one server instance. Horizontal scaling requires:

1. **Sticky Sessions**: Route client reconnections to the same server (load balancer session affinity). Failure of that server disconnects all its clients.
2. **Shared State**: Store session state in Redis, database. On reconnection, client provides a session token; server looks up state.
3. **Pub/Sub Broker**: Multiple servers connect to a message broker (Redis Pub/Sub, RabbitMQ). Client A connects to Server 1, Client B to Server 2. Server 1 publishes message → Broker → Server 2 → Client B. Works for fan-out scenarios (e.g., chat room, notifications).
4. **Partitioning**: Assign users to server partitions. Client always connects to partition server. Partition fail = those clients disconnect. Coordinate via coordination service (etcd, Zookeeper).

### Connection Pooling

Use a connection pool per server to limit resource usage. Each client connection allocates a handle (file descriptor, TCP socket, memory buffer). Cap total connections (e.g., 65k per server, OS file descriptor limit).

### Message Queues

For high-volume asymmetric loads (e.g., many sends to few receives), decouple publishers from subscribers via a queue:

```
Publisher → Queue → Workers (consume, forward to WebSocket clients)
```

Prevents queue overflow from blocking publishers.

## Comparison: WebSocket vs Server-Sent Events (SSE)

| Aspect | WebSocket | SSE |
|--------|-----------|-----|
| Direction | Bidirectional | Server → Client |
| Protocol | Binary frames, stateful | HTTP; streaming text (text/event-stream) |
| Connection Reuse | Single connection for all messages | HTTP connection; limited reuse |
| Message Format | Binary or text; application-defined | Text; newline-delimited events |
| Reconnection | Manual (client code) | Browser built-in; `EventSource` API with auto-reconnect |
| Resource Overhead | Per-connection state (memory, FD) | Similar (one TCP stream per client) |
| Proxy Compatibility | Better (binary masking prevents proxy tampering) | Susceptible to caching proxies |
| Browser Support | All modern browsers | All modern browsers |

**Use WebSocket if**: Real-time bidirectional communication needed (chat, collaborative editing, gaming).
**Use SSE if**: Server-to-client push only (notifications, live scores, stock tickers).

## Libraries & Frameworks

### Socket.IO

Node.js library that abstracts WebSocket and falls back to polling if WebSocket unavailable. Adds rooms, namespaces, and reconnection logic. Example:

```javascript
const io = require('socket.io')(3000);
io.on('connection', (socket) => {
  socket.on('message', (msg) => {
    io.emit('message', msg);
  });
});
```

### GraphQL Subscriptions

GraphQL protocols (e.g., graphql-ws) run over WebSocket subprotocols. Subscriptions are push-based queries.

### Raw WebSocket

JavaScript `WebSocket` API in browsers:

```javascript
ws = new WebSocket('wss://example.com/chat');
ws.addEventListener('message', (event) => {
  console.log(event.data);
});
ws.send(JSON.stringify({type: 'chat', msg: 'hello'}));
```

## See Also

- **networking-http** — HTTP upgrade mechanism; difference from long polling.
- **security-web-application** — HTTPS/TLS, origin validation.
- **concurrency-patterns** — Handling concurrent WebSocket connections.