# WebSocket at Scale — Connection Management, Pub/Sub, Sticky Sessions & Presence

## Overview

WebSocket enables persistent bidirectional communication, but a single server typically handles ~10k concurrent connections before hitting memory or OS file descriptor limits. Production systems require architectural patterns: pub/sub broadcast across multiple servers, sticky session routing, load balancer support, connection pooling strategies, presence tracking, room-based messaging, and graceful shutdown. This note covers operational deployment patterns for scaling WebSocket to thousands of concurrent users.

## Single Server Limits

### Memory & File Descriptor Constraints

Each WebSocket connection consumes:
- **Memory:** ~10-100 KB per idle connection (connection state, buffers, metadata)
- **File descriptors:** 1 per connection (Linux default: 1024 per process; systemd typically 65536 for services)

At 10k concurrent connections, a single Node.js server consumes 100 MB–1 GB memory plus buffers. Larger numbers require:
1. Vertical scaling (larger machine) — limited; doesn't solve single-point-of-failure
2. Horizontal scaling (multiple servers) — requires routing, deduplication, state synchronization

### Per-Process Bottlenecks

Beyond file descriptors:
- **CPU bound:** Message broadcasting, large payload serialization
- **Network bound:** Egress bandwidth for broadcast messages
- **Memory bound:** Buffering unacknowledged messages during backpressure

Scaling solution: distribute connections across multiple servers; separate message publishing from connection serving.

## Pub/Sub Architecture for Horizontal Scaling

### Pattern: Multi-Server Broadcast

```
Server A (Connections: ClientA, ClientB)
  ↓ publishes
Redis / NATS / Kafka (central message broker)
  ↓ subscribes
Server B (Connections: ClientC, ClientD)
Server C (Connections: ClientE, ClientF)
```

**Message Flow:**
1. ClientA sends message to Server A
2. Server A publishes to broker channel (e.g., "room-123")
3. All servers subscribed to "room-123" receive the message
4. Each server delivers to its connected clients (ClientB, ClientC, ClientE)
5. ClientD (also in room-123, connected to Server B) receives the message

**Example (Node.js with Redis):**
```javascript
// Server A
io.on('connection', (socket) => {
  socket.on('message', (msg) => {
    redisClient.publish('room-123', JSON.stringify(msg))
  })
})

// All servers (A, B, C)
redisSubscriber.subscribe('room-123', (msg) => {
  io.to('room-123').emit('message', JSON.parse(msg))
})
```

### Broker Choice: Redis vs NATS vs Kafka

| Broker | Use Case | Notes |
|--------|----------|-------|
| **Redis** | < 100k clients, moderate message volume | In-memory, fast, persistence optional; PUBLISH/SUBSCRIBE is simple; no message history |
| **NATS** | High throughput, < 50k subjects | Fast, low-latency, subject-based routing; no disk persistence; good for request-reply |
| **Kafka** | High volume, need message history/replay | Persistent, ordered messages, consumer groups; higher latency; good for analytics pipelines |

**Firebase / other hosted options:** Easier operations, vendor lock-in, higher cost at scale.

## Sticky Sessions & Load Balancer Configuration

### The Sticky Session Problem

When ClientA connects to Server A and establishes WebSocket, subsequent messages from ClientA must route to Server A (where the connection exists). If a load balancer routes ClientA's next message to Server B, the connection is lost.

**Solution:** Sticky sessions (connection affinity): route all requests from a source IP (or a session token) to the same backend server.

### Load Balancer Patterns

**Simple IP Hash (Layer 4):**
```
Load Balancer
  ↓ (hash of ClientIP % num_servers)
  ├─ Server A (ClientA, ClientC)
  ├─ Server B (ClientB, ClientD)
  └─ Server C (ClientE, ClientF)
```
Pro: Simple; works for WebSocket upgrade. Con: uneven distribution after server failures.

**Session Token (Layer 7):**
```
Client: GET /ws?sessionId=abc123
Load Balancer: extracts sessionId, hashes to Server B
Server B: looks up sessionId, retrieves user context
```
Pro: Distributes load evenly; survives IP changes (mobile); easier rebalancing.

**Example HAProxy config:**
```
backend websocket_servers
  balance leastconn
  cookie SERVERID insert indirect nocache
  server server1 127.0.0.1:8001 cookie server1
  server server2 127.0.0.1:8002 cookie server2
```

### Handling Server Failures

With sticky sessions, if Server A dies, its connected clients are orphaned. Mitigation:
1. **Health checks:** Load balancer probes servers; marks unhealthy ones down. New clients route to healthy servers.
2. **Connection re-establishment:** Clients detect disconnect (no heartbeat), reconnect. Load balancer may route to different server.
3. **Session state replication:** User state stored in Redis (not Server A process memory) so any server can resume context.

## Room-Based Messaging & Presence Systems

### Room Concept

A **room** is a logical grouping of connections. Multiple clients subscribe to a room; messages broadcast to all members.

**Example: Chat room "dev-team"**
```
Room: dev-team
  ├─ ClientA (user: alice)
  ├─ ClientB (user: bob)
  └─ ClientC (user: charlie)

Message sent by alice → all three clients receive
```

**Implementation:**
```javascript
// Client joins
socket.emit('join-room', { roomId: 'dev-team' })

// Server
socket.on('join-room', ({ roomId }) => {
  socket.join(roomId)
  // also publish to Redis for other servers
  redisClient.publish(`room-${roomId}`, JSON.stringify({
    event: 'user-joined',
    user: socket.userId
  }))
})

// Broadcast in-room
socket.on('message', (msg) => {
  io.to(roomId).emit('message', msg)
  redisClient.publish(`room-${roomId}`, JSON.stringify(msg))
})
```

### Presence Tracking

**Presence:** knowing which users are online in a room (avoiding "ghost" entries when connections drop).

**Naive approach (problematic):**
```
✗ Set presence in Redis
✗ TTL expires after 5 minutes
✗ If user disconnects without clean goodbye, presence remains for 5 minutes
```

**Better: heartbeat-driven presence**
```
┌─ Client sends heartbeat (pong) every 30s
│
├─ Server receives pong, updates presence timestamp: presence:dev-team:alice = {lastSeen: now}
│
└─ Background worker queries: users where lastSeen < now - 60s are offline; remove from presence
```

**Hybrid (Redis Streams + TTL):**
```javascript
// Client action
socket.on('user-action', (action) => {
  // Stream: audit log (persists for later replay)
  redisClient.xadd(`presence:stream:${roomId}`, '*', 'userId', userId, 'action', action)
  
  // Set: active presence (TTL 90s, extends on each action)
  redisClient.set(`presence:${roomId}:${userId}`, JSON.stringify({
    userId, action, timestamp: Date.now()
  }), 'EX', 90)
})

// Query active presence
redisClient.keys('presence:dev-team:*') // gives all active users
```

## Connection Management & Resource Cleanup

### Graceful Shutdown

When a server restarts or is drained, connected clients must reconnect:

```javascript
// Node.js with Socket.IO
server.close() // stops accepting new connections

// Set timeout to force-disconnect stragglers
setTimeout(() => {
  io.disconnectSockets()
}, 30000) // give clients 30s to gracefully close
```

Clients should handle reconnection with exponential backoff (see `networking-websocket-patterns.md`).

### Backpressure Management

If clients send messages faster than the server can process, buffers fill. Solutions:
1. **Rate limiting:** Drop or queue messages over threshold
2. **Adaptive flow control:** Client backs off (waits before sending next message) when server signals congestion
3. **Message prioritization:** Critical messages (presence) prioritized over optional updates (system log)

```javascript
// Server example: drop low-priority messages if buffer high
io.on('connection', (socket) => {
  socket.on('non-critical-update', (msg) => {
    if (io.engine.clientsCount > 9000) {
      // nearing capacity; ignore non-critical
      return
    }
    // process message
  })
})
```

## Considerations for Different Deployment Models

### Self-Hosted / On-Premises

- Use Redis Cluster for broker redundancy
- Deploy load balancer (HAProxy, nginx) in front of servers
- Monitor connection count, memory, CPU
- Plan for failover (standby server)

### Managed Services (Firebase Realtime DB, AWS AppSync, Pusher, etc.)

- Automatically handles scaling, pub/sub, presence
- Trade-off: less control, vendor lock-in, higher cost at extreme scale
- Good for 10k–100k concurrent users with modest engineering effort

## See Also

- `networking-websocket-patterns.md` — heartbeat, reconnection, close lifecycle
- `web-real-time-patterns.md` — protocol comparison, CRDT-based collaborative editing
- `networking-http.md` — proxies, load balancer behavior
- `database-redis.md` — Redis PUBLISH/SUBSCRIBE, data structures