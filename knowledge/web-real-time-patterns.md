# Web Real-Time Patterns — WebSockets, SSE, CRDTs & Collaborative Editing

## Overview

Real-time web patterns enable bidirectional communication between client and server, supporting live updates, presence detection, and collaborative data structures. This note covers protocol choices (WebSocket vs. Server-Sent Events vs polling), scaling strategies (heartbeats, connection pooling), conflict resolution via CRDTs (Yjs, Automerge), and applications like collaborative editing (Google Docs, Figma).

## Protocols & Transport Choices

### WebSocket: Full-Duplex, Stateful

WebSocket is a protocol upgrade from HTTP that establishes a persistent TCP connection, allowing the server and client to send messages to each other at any time, without polling.

**Handshake:**

```
Client:
GET /chat HTTP/1.1
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: x3JJHMbDL1EzLkh9GBhXDw==
Sec-WebSocket-Version: 13

Server:
HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Accept: HSmrc0sMlYUkAGmm5OPpG2HaGWk=
```

After 101 response, the connection speaks WebSocket framing protocol (binary), not HTTP.

**Message Framing:**

Each message is broken into frames with a header (FIN bit, opcode, payload length) and mask (clients mask, servers don't). This allows middleboxes to distinguish WebSocket traffic and prevents cache poisoning.

**Characteristics:**

- Low latency (no HTTP overhead per message)
- Stateful (server must track open connections)
- Browser support is widespread (>97%)
- Some proxies/firewalls block WebSocket connections

### Server-Sent Events (SSE)

Server sends text messages over HTTP to the client using `text/event-stream` content type. Client listens via `EventSource`:

```javascript
const es = new EventSource('/updates')
es.addEventListener('message', (e) => {
  console.log('Message:', e.data)
})
es.addEventListener('user-joined', (e) => {
  console.log('User joined:', JSON.parse(e.data))
})
```

Server sends:

```
data: {"action":"user-joined","user":"alice"}

event: user-joined
data: {"id":123,"name":"alice"}

: comment (keeps connection alive)

retry: 5000
```

**Characteristics:**

- Unidirectional (server → client only; client sends via separate HTTP requests or XHR)
- HTTP-based (works through proxies)
- Automatic reconnection on disconnect (configurable retry)
- Text-only (binary requires encoding)
- Lower latency than polling, higher than WebSocket

### HTTP Long-Polling

Client makes an HTTP request; server holds the connection open until it has data to send, then responds. Client immediately re-requests.

```javascript
async function poll() {
  const resp = await fetch('/updates')
  const data = await resp.json()
  console.log('Data:', data)
  poll()  // re-request immediately
}
```

**Characteristics:**

- Works through all proxies (plain HTTP)
- Higher latency (connection close/reopen overhead)
- Higher server resource usage (many open connections)
- Simplest to implement and debug

### When to Use Each

- **WebSocket:** Real-time chat, gaming, collaborative editing, live dashboards. Optimize for latency.
- **SSE:** Server push notifications, live feeds, real-time logs. Unidirectional suffices; advantages of HTTP resilience.
- **Polling:** Degraded fallback, low-frequency updates. Simplest but inefficient.

## Scaling WebSocket Connections

### Single Server Problem

A single server can handle ~10k concurrent connections (memory & OS file descriptor limits). Most real-time apps need horizontal scaling.

### Pub-Sub Across Servers

```
Client A → Server 1 → [message] → Redis Pub/Sub → Server 2 → Client B
```

When Client A sends a message to Server 1, Server 1 publishes to Redis. Server 2 (which has Client B) subscribes to the channel and delivers the message.

```javascript
// Server 1
io.on('connection', (socket) => {
  socket.on('chat-message', (msg) => {
    redis.publish('chat:room1', msg)
  })
  
  redis.subscribe('chat:room1', (msg) => {
    socket.emit('chat-message', msg)
  })
})
```

**Tradeoff:** Redis adds latency; pub-sub is not a queue (messages in transit when no subscribers are lost).

### Sticky Sessions

Keep a user's requests routed to the same server (via load balancer, or client-side reconnection logic):

```
Client → Load Balancer → Server A (holds WebSocket connection)
```

If Server A goes down, Client reconnects via load balancer, typically gets routed to Server B. State is lost unless persisted (session store, Redis).

### Connection Pooling & Heartbeat

Detect stale connections and clean up:

```javascript
// Server
const HEARTBEAT_INTERVAL = 30000  // 30 seconds
setInterval(() => {
  io.emit('ping', {})
}, HEARTBEAT_INTERVAL)

// Client
socket.on('ping', () => {
  socket.emit('pong', {})
})

// Server: track pong responses
socket.on('pong', () => {
  socket.alive = Date.now()
})

// Periodically remove dead sockets
setInterval(() => {
  Object.values(sockets).forEach(s => {
    if (Date.now() - s.alive > HEARTBEAT_INTERVAL * 1.5) {
      s.close()
    }
  })
}, HEARTBEAT_INTERVAL)
```

**Why needed:** Network interruptions might not close the connection immediately. Dead connections waste server memory and block new connections.

## Socket.IO: WebSocket + Fallbacks

Socket.IO abstracts over WebSocket, SSE, long-polling, and WebTransport. It automatically selects the best transport based on browser capability and network conditions.

**Features:**

- **Automatic fallback:** Tries WebSocket, falls back to long-polling if blocked by proxy
- **Acknowledgements:** Client waits for server to confirm receipt
- **Binary support:** Serializes and compresses binary data
- **Rooms & broadcasting:** Built-in multiplexing (instead of managing channels manually)

```javascript
// Server
io.on('connection', (socket) => {
  socket.join('room:123')
  
  socket.on('message', (msg, callback) => {
    io.to('room:123').emit('message', msg)
    callback({ status: 'ok' })  // acknowledgement
  })
  
  socket.on('disconnect', () => {
    socket.leave('room:123')
  })
})

// Client
const socket = io()
socket.emit('message', { text: 'hello' }, (ack) => {
  console.log('Server said:', ack)
})

socket.on('message', (msg) => {
  console.log('Received:', msg)
})
```

**Overhead:** Socket.IO adds a framing layer (~42 bytes per message). For high-frequency updates (gaming), raw WebSocket is faster.

## Conflict-Free Replicated Data Types (CRDTs)

Real-time collaboration requires resolving concurrent edits. CRDTs enable conflict-free merging without a central authority.

### Operational Transform (OT)

Edits are operations (insert, delete) with positions. When two users edit simultaneously, transform operations to account for concurrent changes:

```
User A: insert "h" at position 0 → "hello"
User B: insert "w" at position 0 → "world"

Transform B's operation given A's edit:
B's insert now at position 1 (shifted by A's insert)
Merged: "whello" (exact order depends on tie-breaking)
```

**Complexity:** Transform functions are complex to implement; different OT variants (Google Wave, ShareDB).

### CRDT: State-Based

Each replica maintains a data structure with metadata (timestamps, unique IDs). Merging is deterministic; no transformation needed.

**Example: Last-Write-Wins (LWW) Register**

```
User A at time T1: { value: "hello", timestamp: T1, nodeId: "A" }
User B at time T2: { value: "world", timestamp: T2, nodeId: "B" }

If T2 > T1, use User B's value. If T1 == T2, use lexicographic order (tie-break).
```

**Limitations:** Simple conflicts (single value) are easy; complex structures need sophisticated merge logic.

### Yjs (Array CRDT)

Yjs provides shared types that sync automatically:

```javascript
import * as Y from 'yjs'

const ydoc = new Y.Doc()
const ytext = ydoc.getText('shared-text')
const yarray = ydoc.getArray('users')

// Edit like normal data
ytext.insert(0, 'hello')
yarray.push(['alice'])

// Automatic sync to other clients
ytext.observe(event => {
  console.log('Text changed:', ytext.toString())
})
```

**Transport:** Yjs uses a binary protocol to sync changes. Transports are pluggable (y-websocket, y-webrtc, y-redis).

**Awareness:** Track user presence (cursor position, selection) without merging conflicts:

```javascript
const awareness = ydoc.awareness
awareness.setLocalState({
  user: { name: 'alice', color: '#ff0000' },
  cursor: { line: 5, column: 10 }
})

awareness.on('change', changes => {
  changes.forEach(change => {
    const state = awareness.getStates().get(change.client)
    console.log('User cursor:', state.cursor)
  })
})
```

### Automerge

Similar to Yjs: JSON-like data structures, automatic sync, binary encoding.

```javascript
let doc = new Automerge.init()
doc = Automerge.change(doc, d => {
  d.text = new Automerge.Text()
  d.text.insertAt(0, ...'hello'.split(''))
})

// Save as bytes, send over network
const bytes = Automerge.save(doc)
const other = Automerge.load(bytes)

// Merge changes from two branches
const merged = Automerge.merge(branch1, branch2)
```

## Collaborative Editing Example

A shared document with Yjs and WebSocket:

```javascript
// Server
const wss = new WebSocket.Server({ port: 8080 })
const docMap = new Map()

wss.on('connection', (ws, req) => {
  const docId = req.url.split('/')[1]
  const ydoc = docMap.get(docId) || new Y.Doc()
  docMap.set(docId, ydoc)
  
  // Sync protocol: share initial state, then delta updates
  let state = Y.encodeStateAsUpdate(ydoc)
  ws.send(JSON.stringify({ type: 'sync-state', state }))
  
  // Listen for client updates
  ws.on('message', (msg) => {
    const event = JSON.parse(msg)
    if (event.type === 'update') {
      Y.applyUpdate(ydoc, new Uint8Array(event.update))
      
      // Broadcast to other clients
      docMap.get(docId).clients.forEach(client => {
        if (client !== ws) {
          client.send(JSON.stringify({ type: 'update', update: event.update }))
        }
      })
    }
  })
})

// Client
const ydoc = new Y.Doc()
const ytext = ydoc.getText('shared-doc')

const ws = new WebSocket(`ws://localhost:8080/${docId}`)
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data)
  if (msg.type === 'sync-state') {
    Y.applyUpdate(ydoc, new Uint8Array(msg.state))
  } else if (msg.type === 'update') {
    Y.applyUpdate(ydoc, new Uint8Array(msg.update))
  }
}

ytext.observe(event => {
  const update = Y.encodeStateAsUpdate(ydoc)
  ws.send(JSON.stringify({ type: 'update', update: Array.from(update) }))
})
```

## Presence Detection

Track who's online and their cursor position:

```javascript
const presence = {
  [userId]: {
    name: 'alice',
    color: '#ff0000',
    cursor: { line: 5, column: 10 }
  }
}

// Broadcast presence changes
socket.on('presence-update', (data) => {
  presence[socket.id] = data
  io.emit('presence-changed', presence)
})

socket.on('disconnect', () => {
  delete presence[socket.id]
  io.emit('presence-changed', presence)
})
```

## Conclusion

Real-time web patterns range from simple polling (works everywhere, inefficient) to sophisticated CRDTs (efficient collaboration, complex). Protocol choice depends on network constraints, latency requirements, and browser support. Socket.IO abstracts these tradeoffs. CRDTs are powerful for collaborative editing but add complexity in serialization, state management, and debugging. Most production systems combine patterns: WebSocket for speed, fallbacks for resilience, and CRDTs for conflict resolution.