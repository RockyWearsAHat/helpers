# Server-Sent Events (SSE) — EventSource API, Streaming, Reconnection & Real-Time Patterns

## Overview

Server-Sent Events (SSE) is an HTTP-based protocol for unidirectional server-to-client streaming over persistent connections. Clients use the `EventSource` API to listen for named events. The server sends plain-text messages in a standardized format with automatic reconnection on disconnect. SSE is simpler than WebSocket (no bidirectional framing overhead), works through all proxies (plain HTTP), and suits server-push patterns like notifications, live feeds, and streaming logs. This note covers the EventSource API, event format, reconnection behavior, streaming patterns, comparison to WebSocket, and integration with HTTP/2 or NDJSON streaming.

## EventSource API & Connection Lifecycle

### Basic Connection

```javascript
const es = new EventSource('/api/events')

// Listen for all events (default type: "message")
es.addEventListener('message', (event) => {
  console.log('Data:', event.data)
  console.log('Last-Event-ID:', event.lastEventId)
})

// Listen for named event types
es.addEventListener('user-joined', (event) => {
  const user = JSON.parse(event.data)
  console.log(`${user.name} joined`)
})

// Error handling
es.addEventListener('error', (event) => {
  if (event.eventPhase === EventSource.CLOSED) {
    console.log('Connection closed permanently')
  } else if (event.eventPhase === EventSource.CONNECTING) {
    console.log('Reconnecting...')
  }
})

// Cleanup
es.close() // stops listening and reconnecting
```

### Connection State

EventSource has three states:

| State | Code | Meaning |
|-------|------|---------|
| `CONNECTING` | 0 | Initial state or reconnecting after disconnect |
| `OPEN` | 1 | Connected and receiving events |
| `CLOSED` | 2 | Connection closed; no automatic reconnection |

The connection moves to `CONNECTING` when the server closes or a read error occurs. The client automatically attempts reconnection (unless server sends explicit close or client calls `.close()`).

## Event Format & Message Structure

### Server Sends Plain Text (text/event-stream)

```
data: hello world

event: user-joined
data: {"id": 123, "name": "alice"}

: this is a comment (ignored by client)

retry: 5000
id: msg-456

event: custom-event
data: {"key": "value"}
```

**Parsing rules:**
1. **data:** — payload for the event (can span multiple lines)
2. **event:** — event type/name (default: "message")
3. **id:** — unique identifier for message (becomes `event.lastEventId`)
4. **retry:** — milliseconds before reconnect attempt (default: varies by browser, ~3000ms)
5. **comments** (`:`) — ignored by client; often used as keepalive ("`:` every 30s keeps the connection alive")

### Multi-Line Data

```
data: line 1
data: line 2
data: line 3

```

Becomes `event.data = "line 1\nline 2\nline 3"` (newlines inserted between the three data lines).

### JSON in Events

```javascript
// Server
res.write('event: notification\n')
res.write('id: notif-99\n')
res.write('data: ' + JSON.stringify({
  title: 'New message',
  body: 'Hello from Alice',
  timestamp: Date.now()
}) + '\n\n')

// Client
es.addEventListener('notification', (event) => {
  const notify = JSON.parse(event.data)
  console.log(notify.title)
})
```

## Reconnection & Last-Event-ID

### Automatic Reconnection

If the server closes the connection (or network drops), EventSource reconnects automatically:

```
Time:     0s          5s          10s
          ├─ connected
          │
          ├─ (server close)
          │
          ├─ [wait 3s]
          │
          ├─ reconnect attempt
          │
          ├─ connected again
```

Browser sends `Last-Event-ID` header in reconnect request:

```
GET /api/events HTTP/1.1
Last-Event-ID: msg-456
Connection: keep-alive
```

Server can use this to resume where the client left off (preventing duplicate/missed events).

**Example: using Last-Event-ID to resume**

```javascript
// Server (Express)
app.get('/api/events', (req, res) => {
  const clientLastId = req.header('Last-Event-ID') || '0'
  
  // Load events after clientLastId from database
  const events = db.query('SELECT * FROM events WHERE id > ? ORDER BY id', [clientLastId])
  
  // Send all missed events first
  res.setHeader('Content-Type', 'text/event-stream')
  res.setHeader('Cache-Control', 'no-cache')
  res.setHeader('Connection', 'keep-alive')
  
  for (const event of events) {
    res.write(`id: ${event.id}\n`)
    res.write(`data: ${JSON.stringify(event)}\n\n`)
  }
  
  // Then stream live events as they happen
  const listener = (newEvent) => {
    res.write(`id: ${newEvent.id}\n`)
    res.write(`data: ${JSON.stringify(newEvent)}\n\n`)
  }
  eventBus.on('new-event', listener)
  
  req.on('close', () => {
    eventBus.off('new-event', listener)
  })
})
```

### Retry Behavior

Server can control reconnection interval:

```
retry: 1000

```

Sets retry to 1 second. If server sends `retry: 60000`, browser waits 60 seconds before reconnecting (useful to back off during outages).

Default browser retry is typically 3000 ms. Send `retry: 0` to disable automatic reconnection (though clients can still listen for reconnection logic).

**Browser exponential backoff (not automatic):**
```javascript
let retryCount = 0
const es = new EventSource('/api/events')
es.addEventListener('error', () => {
  es.close()
  retryCount++
  const delay = Math.min(1000 * Math.pow(2, retryCount), 30000)
  setTimeout(() => {
    location.reload() // or create new EventSource
  }, delay)
})
```

## Streaming Patterns

### Streaming JSON Lines (NDJSON)

**NDJSON:** newline-delimited JSON for streaming (one JSON object per line).

Server sends `data:` once per line (no newlines in JSON):

```
data: {"event":"user-joined","id":1,"name":"alice"}
data: {"event":"message","id":2,"text":"hello","from":"alice"}
data: {"event":"user-left","id":3,"name":"bob"}
```

Client parses:
```javascript
es.addEventListener('message', (event) => {
  const obj = JSON.parse(event.data)
  console.log(obj)
})
```

### Streaming Logs

```javascript
app.get('/api/logs', (req, res) => {
  res.setHeader('Content-Type', 'text/event-stream')
  
  // Stream log lines as they appear
  const stream = fs.createReadStream('./app.log', { encoding: 'utf8' })
  
  stream.on('data', (chunk) => {
    const lines = chunk.split('\n').filter(Boolean)
    for (const line of lines) {
      res.write(`data: ${line}\n\n`)
    }
  })
  
  stream.on('end', () => {
    res.write(': [stream ended]\n\n')
  })
})
```

### Chat/Notification Feed

```javascript
app.get('/api/feed', (req, res) => {
  res.setHeader('Content-Type', 'text/event-stream')
  
  const feedListener = (item) => {
    res.write(`id: ${item.id}\n`)
    res.write(`event: ${item.type}\n`)
    res.write(`data: ${JSON.stringify(item.data)}\n\n`)
  }
  
  feedBus.on('new-item', feedListener)
  
  // Keepalive in case no activity (prevents proxy timeout)
  const keepalive = setInterval(() => {
    res.write(': alive\n\n')
  }, 30000)
  
  req.on('close', () => {
    clearInterval(keepalive)
    feedBus.off('new-item', feedListener)
  })
})
```

## HTTP/2 & Connection Multiplexing

SSE over HTTP/2 uses a single stream within the multiplexed connection, allowing the browser to:
1. Open SSE connection to `/api/events`
2. Simultaneously open other streams (fetch requests, images, etc.)
3. Share congestion control and flow-control windows

**Benefit:** no separate TCP connection needed for streaming; reduces overhead.

**Limitation:** HTTP/2 server push (server-initiated streams) is separate from EventSource (client-initiated streaming). EventSource doesn't automatically use server push; both use HTTP streams.

## Comparison: SSE vs WebSocket

| Feature | SSE | WebSocket |
|---------|-----|-----------|
| **Direction** | Server → Client only | Bidirectional |
| **Framing** | Text HTTP | Binary WebSocket frames |
| **Proxy Support** | Universal (plain HTTP) | Some proxies block |
| **Latency** | ~50-200ms (HTTP overhead) | ~10-50ms (lower frames/no HTTP) |
| **Reconnection** | Automatic (built-in) | Manual (app handles) |
| **Scaling** | Simpler (stateless HTTP) | Stateful; requires pub/sub |
| **Use Case** | Notifications, live feeds, logs | Chat, gaming, collaborative tools |

**Choose SSE if:**
- Server-to-client push only (no client-initiated messages)
- Proxy friendliness matters
- Automatic reconnection desired
- Simplicity prioritized

**Choose WebSocket if:**
- Bidirectional real-time is required
- Latency is critical (gaming, trading)
- Binary data needed

## CORS & Authentication

### CORS Headers

EventSource respects CORS; browser enforces cross-origin restrictions:

```javascript
app.get('/api/events', (req, res) => {
  res.setHeader('Access-Control-Allow-Origin', '*')
  res.setHeader('Access-Control-Allow-Credentials', 'true')
  res.setHeader('Content-Type', 'text/event-stream')
  res.write('data: hello\n\n')
})
```

### Authentication

**Cookies (simplest):**
```javascript
// Client (CORS with credentials)
const es = new EventSource('/api/events', { withCredentials: true })

// Server reads req.session or req.cookies from authenticated user
```

**Query parameter or Authorization header (not standard for EventSource):**
```javascript
const es = new EventSource(`/api/events?token=${jwtToken}`)

// Server parses token from URL
```

## Server Implementation Considerations

### Connection Lifecycle & Cleanup

```javascript
app.get('/api/events', (req, res) => {
  const clientId = generateId()
  
  res.setHeader('Content-Type', 'text/event-stream')
  res.setHeader('Cache-Control', 'no-cache')
  res.setHeader('Connection', 'keep-alive')
  
  // Send initial "connected" event
  res.write(`data: {"connected":true,"clientId":"${clientId}"}\n\n`)
  
  // Register listener
  const listener = (event) => {
    if (!res.writableEnded) {
      res.write(`data: ${JSON.stringify(event)}\n\n`)
    }
  }
  
  bus.on('event', listener)
  
  // Cleanup on disconnect
  req.on('close', () => {
    bus.off('event', listener)
    // log client left, update presence, etc.
  })
  
  // Timeout after 1 hour (browser reconnects)
  const timeout = setTimeout(() => {
    res.end()
  }, 3600000)
  
  req.on('close', () => clearTimeout(timeout))
})
```

### Resource Management

- **File descriptors:** Each open EventSource connection = one fd. Limit connections via load balancer or application logic.
- **Memory:** Each connection holds listeners in memory. Remove on disconnect.
- **CPU:** Avoid blocking event loop when sending events. Use async writes.

## See Also

- `networking-websocket-patterns.md` — heartbeat, reconnection patterns
- `web-real-time-patterns.md` — protocol comparison, scaling strategies
- `networking-http.md` — HTTP/2, persistent connections
- `networking-api-gateway-protocols.md` — SSE vs WebSocket vs gRPC comparison