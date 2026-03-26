# WebSocket Patterns: Lifecycle, Heartbeat, Reconnection, Scaling & Room Patterns

## Overview

WebSocket (RFC 6455) enables full-duplex real-time communication over a single TCP connection. Beyond protocol mechanics, production systems require patterns for managing connection lifecycle, detecting stale connections, recovering from disconnections gracefully, and scaling to thousands of concurrent clients. This note covers operational patterns: heartbeat/keepalive, exponential backoff reconnection, binary vs text frame choice, subprotocol negotiation, scaling architectures (sticky sessions, pub/sub broadcast), Socket.IO abstractions, presence tracking, and backpressure management.

## Connection Lifecycle & State Management

### Establishment Phase

```
1. Client establishes TCP connection
2. Client sends HTTP Upgrade request
3. Server responds with 101 Switching Protocols
4. Both sides begin frame exchange (WebSocket protocol)
5. Application-level handshake (optional: exchange version, capabilities)
```

**Application handshake (after WebSocket upgrade):**
```json
// Client → Server (first message)
{
  "type": "connect",
  "version": "1.0",
  "clientId": "uuid-12345"
}

// Server → Client
{
  "type": "connected",
  "serverId": "server-A",
  "features": ["presence", "rooms"]
}
```

Useful for: client version verification, feature negotiation, initial state sync.

### Active Phase

Both client and server send frames asynchronously. No requirement for request-response order; either side can initiate.

**Message structure (typical):**
```json
{
  "type": "message" | "presence" | "status",
  "channel": "room-123",
  "data": { ... },
  "id": "msg-456" // for deduplication
}
```

### Graceful Close (Clean Shutdown)

Initiated by either side:

```
Client: sends CLOSE frame (opcode 0x8)
        optional status code: 1000 (normal), 1001 (going away), 1002 (protocol error)
Server: receives CLOSE, flushes pending messages, responds with CLOSE
```

Both sides clean up: close file handles, flush buffers, notify subscribers ("user X left room Y").

**Abrupt Termination:**
```
Network failure, process crash, timeout
Neither side sends CLOSE frame
TCP connection drops
Opposite side detects connection broken on next activity
```

Applications must handle both graceful and abrupt closes.

## Heartbeat & Keepalive Patterns

### Ping/Pong (Protocol-Level)

WebSocket defines control frames for keepalive:

```
Client → Server: PING frame (opcode 0x9, optional payload)
Server → Client: PONG frame (opcode 0xA, echoes payload)
```

Server/endpoint automatically responds to PING with PONG (no application code required). Useful for:
- Detecting dead TCP connections (if PONG not received within timeout, connection is stale)
- Measuring roundtrip latency (timestamp in PING payload, compare to PONG)

**Firewall implications:** Some proxies expect periodic activity; PING/PONG prevents proxy timeout.

### Application-Level Heartbeat

Protocol-level ping may not be sufficient. Applications often define heartbeat messages:

```json
// Server → Client (every 30 seconds)
{
  "type": "ping",
  "timestamp": 1640000000
}

// Client → Server (responds)
{
  "type": "pong",
  "timestamp": 1640000000,
  "clientTime": 1640000003
}
```

Advantages:
- Application can embed metadata ("I'm alive and processing")
- Can trigger client-side action (refresh UI state, reconnect if stale)
- Decoupled from WebSocket framing

**Timeout:** If heartbeat not received for N heartbeat intervals (e.g., 5 × 30s = 150s), client/server declares connection dead and closes.

## Reconnection with Exponential Backoff

### Exponential Backoff Strategy

Connection drops due to network failure, server restart, etc. Client needs to reconnect without hammering server:

```
Attempt 1: Try immediately (backoff = 1s)
Attempt 2: Wait 1s, retry (backoff = 2s)
Attempt 3: Wait 2s, retry (backoff = 4s)
Attempt 4: Wait 4s, retry (backoff = 8s)
Attempt 5: Wait 8s, retry (backoff = 16s)
...
Max backoff: 60s (don't grow forever)
Jitter: ±10% randomization to prevent thundering herd
```

**Pseudocode:**
```python
import random

backoff = 1  # seconds
max_backoff = 60
jitter_factor = 0.1

while not connected:
  try:
    connect()
    backoff = 1  # reset on successful connection
  except ConnectionFailed:
    actual_backoff = backoff * (1 + random.uniform(-jitter_factor, jitter_factor))
    sleep(actual_backoff)
    backoff = min(backoff * 2, max_backoff)
```

**Jitter rationale:** If 1000 clients all disconnect at same time and retry with same backoff intervals, retry wave hits server at exact same time (thundering herd). Adding randomness spreads retries.

### Message Deduplication During Reconnection

Client sends message M1. Connection drops before server ACKs. Client reconnects, resends M1. Server processes M1 twice (state corruption risk).

**Solution 1: Client-side message ID & server deduplication:**
```
Message format:
{
  "id": "msg-uuid-123",
  "type": "chat",
  "text": "hello"
}

Server maintains "seen message IDs" for 5 minutes:
- If ID in cache → drop duplicate
- If ID not seen → process and add to cache
```

**Solution 2: Server ACKs messages:**
```
Client → Server:
{
  "id": "msg-123",
  "type": "chat",
  "text": "hello"
}

Server → Client:
{
  "type": "ack",
  "id": "msg-123"
}

Client: Once ACK received, forget msg-123; don't resend if reconnect
```

**Solution 3: Idempotence (preferred):**
```
Message is designed to be idempotent (safe to execute multiple times).
Example: "Set my status to 'online'" is idempotent; "Increment counter" is not.
Choose idempotent patterns where possible.
```

## Binary vs Text Frames

### Text Frames (UTF-8 JSON)

```
opcode: 0x1 (text)
payload: valid UTF-8
```

**Advantages:**
- Human-readable (debugging in browser DevTools)
- Standard: JSON marshaling in any language
- Debuggable: easy to inspect in logs

**Disadvantages:**
- Larger payload (JSON overhead, especially for arrays)
- Parsing overhead (JSON tokenizer)

**Typical use:** Chat, presence updates, configuration changes (low frequency, human-readable).

### Binary Frames (Protobuf, MessagePack, etc.)

```
opcode: 0x2 (binary)
payload: arbitrary bytes (not necessarily UTF-8)
```

**Typical format: length-prefixed, with message type field:**
```
[1 byte: message type] [4 bytes: length] [payload]
Example: 0x03 0x00 0x00 0x00 0x20 [32 bytes of protobuf data]
```

**Advantages:**
- Compact (binary serialization denser than JSON)
- Faster parsing (no tokenization, direct field access via offset/type info)
- Supports nested structures, arrays efficiently

**Disadvantages:**
- Not human-readable; harder to debug
- Requires message type registry (decoder must know message format)

**Typical use:** High-frequency data (metrics, game state, real-time transactions), large payloads (video frames, sensor arrays).

**Hybrid approach:** Send text (JSON) for low-frequency commands, binary for high-frequency data streams.

## Subprotocols & Capability Negotiation

Clients and servers can agree on an additional protocol layer via `Sec-WebSocket-Protocol` header:

```
Client request:
Sec-WebSocket-Protocol: chat, superchat

Server response (picks one):
Sec-WebSocket-Protocol: superchat
```

Application logic: "ah, client speaks superchat; use advanced features".

**Typical use:**
```
Subprotocol "v1": Basic chat (text frames only)
Subprotocol "v2": Advanced chat (binary frames, compression, typing indicators)
Subprotocol "game": Gaming protocol (state synchronization format)
```

Allows client/server version negotiation without changing WebSocket upgrade handshake.

## Scaling Patterns

### Single Server (Development)

```
Client 1 ↓
Client 2 → WebSocket Server ← Connections in memory
Client 3 ↑
```

All clients connected to single server. Broadcasting: O(N) iteration (send to each client in memory).

**Limit:** ~10K-50K concurrent connections per server (OS file descriptor limits, memory).

### Multiple Servers: Sticky Sessions

```
Load balancer (affinity = client IP / session ID)
  ↓
Server A (Clients 1-1000 in memory)
Server B (Clients 1001-2000 in memory)
Server C (Clients 2001-3000 in memory)
```

Each client "stuck" to one server. Broadcasting: still O(N) per server sending.

**Challenge:** When server goes down, its clients lose connections (not automatically migrated to another server).

**Implementation:** Load balancer routes based on cookie/IP hash:
```go
// Load balancer (NGINX, HAProxy, Kubernetes Service)
http {
  upstream websocket_servers {
    hash $remote_addr;  // or $cookie_sessionid
    server 10.0.0.1:8000;
    server 10.0.0.2:8000;
    server 10.0.0.3:8000;
  }
  
  server {
    location /ws {
      proxy_pass http://websocket_servers;
      proxy_http_version 1.1;
      proxy_set_header Upgrade websocket;
      proxy_set_header Connection "upgrade";
    }
  }
}
```

### Multiple Servers: Pub/Sub Broadcast (Redis, NATS, Kafka)

```
Server A                 Redis Pub/Sub           Server B
Clients 1-1000          (or NATS, Kafka)         Clients 1001-2000
  ↓ (publish)          ↓ (broadcast)              ↓ (publish)
[Message from Client 1] → [Redis channel X] ← [Message from Client 1500]

Server A receives from Redis:
  "Message from Client 1500" → Broadcast to all clients on Server A
```

**Workflow:**
1. Client connected to Server A sends message
2. Server A publishes message to Redis channel (e.g., "room:123")
3. Server B subscribed to "room:123" receives message
4. Server B broadcasts to all its clients in room:123

**Advantages:**
- Clients on any server receive all broadcasts (cross-server visibility)
- Server failure: clients on failed server reconnect to another server, resume
- Linear scaling: add more servers, add more Redis subscribers

**Trade-off:** Redis (or message broker) becomes bottleneck; must handle all broadcasts.

### Room/Channel Patterns

Each client joins zero or more rooms/channels:

```
Client joins room "engineering":
  Server: rooms["engineering"].add(client_id)

Event "new-issue":
  Broadcast to room "engineering":
    for all_clients in rooms["engineering"]:
      send_to(client)
```

**Pub/Sub with rooms:**
```
Client publishes to room "engineering"
  → Server publishes to Redis: PUBLISH "room:engineering" "{message}"
  → All servers subscribed to "room:engineering" receive message
  → Each server broadcasts to its local clients in that room
```

**Implementation (typical Socket.IO abstraction):**
```javascript
// Client
socket.emit("message", { text: "hello" });
socket.on("message", (data) => { console.log(data.text); });

socket.join("room-123");
socket.emit("message-in-room", { text: "room message" });

// Server
io.on("connect", (socket) => {
  socket.on("message", (data) => {
    io.emit("message", data); // broadcast to all clients
  });
  
  socket.on("join", (room) => {
    socket.join(room);
  });
  
  socket.on("message-in-room", (data) => {
    io.to(room).emit("message-in-room", data); // broadcast to room
  });
});
```

## Socket.IO & High-Level Abstractions

**Socket.IO** layers over WebSocket (with HTTP long-polling fallback for incompatible environments):

**Built-in features:**
- Automatic reconnection with exponential backoff
- Message acknowledgment (request-response pattern)
- Namespaces (logical separation; example `/chat`, `/notifications`)
- Rooms (groups within namespace)
- Broadcasting APIs (to all, to room, to specific client)
- Event aliasing (register handlers by event name, not by protocol frames)

**Typical usage:**
```javascript
const io = require('socket.io')(server);

io.on('connection', (socket) => {
  console.log('Client connected:', socket.id);
  
  socket.on('chat-message', (msg) => {
    socket.broadcast.emit('chat-message', msg); // send to others
  });
  
  socket.on('join-room', (room) => {
    socket.join(room);
  });
  
  socket.on('room-message', (room, msg) => {
    io.to(room).emit('room-message', msg);
  });
});
```

**Trade-off:** Socket.IO convenience vs. overhead (not raw WebSocket; additional framing, event encoding).

## Presence & Online Status

### Simple Presence (Per-Connection)

```
Client A connects → Notify all: "User A online"
Client A disconnects → Notify all: "User A offline"
```

**Challenge:** False offline due to network hiccup. Client A loses connection, rejoins in 2 seconds. If broadcast sent instantly, others think User A is offline momentarily.

**Solution:** Debounce offline notifications.
```
Client disconnects → Wait 5 seconds
(Client reconnects within 5s) → Cancel offline notification
(No reconnect within 5s) → Broadcast "User A offline"
```

### Presence with Activity Timestamps

Track last-seen timestamp:

```json
// User presence state
{
  "userId": "alice",
  "status": "online",
  "lastSeen": 1640000000,
  "activity": "editing document"
}

// Periodically broadcast (every 30s)
{
  "type": "presence",
  "users": [
    { "userId": "alice", "status": "online", "lastSeen": 1640000000 },
    { "userId": "bob", "status": "away", "lastSeen": 1639999000 }
  ]
}
```

Client-side logic: Mark user "away" if lastSeen > 5 minutes.

## Backpressure Management

### Problem: Slow Consumer

Server sending data faster than client can process:

```
Server: 1000 messages/sec → Client buffer fills
Client buffer: 100MB limit → buffer overflow → dropped messages or memory exhaustion
```

### Solution 1: Flow Control (Application-Level)

Server checks client's read buffer before sending:

```javascript
// Node.js WebSocket
if (!client_socket.write(data)) {
  // Buffer full; pause sending
  client_socket.once('drain', () => {
    // Buffer drained; resume sending
    continue_sending();
  });
}
```

### Solution 2: Server-Side Queuing

Queue messages; send at client's consumption rate:

```
Server generates 1000 msg/sec but client processes 100 msg/sec
Server queues messages in memory (up to limit)
If queue exceeds limit, drop oldest or newest messages
Client catches up at 100 msg/sec; queue drains

Risk: Memory growth if consumer falls far behind
```

### Solution 3: Adaptive Rate Limiting

Server measures client latency, adapts sending rate:

```
Client ACKs messages (implicit: how much has been processed)
Server calculates consumption rate: 100 msg/sec
Server throttles generation to match: send 100 msg/sec
```

See also: networking-websockets.md, web-real-time-patterns.md, networking-protocols.md