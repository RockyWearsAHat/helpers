# NATS — Core Messaging (Pub/Sub, Request-Reply, Streams) & Distributed Patterns

NATS is a lightweight, open-source messaging infrastructure providing:

1. **Core messaging**: Publish/Subscribe, Request-Reply, Queue Groups (simple, fast)
2. **JetStream**: Persistence layer (stream + consumer model, exactly-once semantics)
3. **NATS KV**: Key-value store (built on JetStream)
4. **NATS Object Store**: Blob storage (built on JetStream)

Marketed as "cloud-native," designed for Kubernetes clusters with minimal operational overhead.

## Core Messaging: Subject-Based Addressing

NATS organizes messages by **subjects** (hierarchical strings with `.` as separator):

```
orders.paid      → Subjects for paid order events
orders.cancelled → Subjects for cancelled orders
users.created    → New user events
```

### Publish-Subscribe

Publisher sends to subject; all subscribers receive copy:

```
Publisher → NATS Server → [Subscriber 1, Subscriber 2, Subscriber 3]
```

No persistence by default (in-memory). If subscriber disconnects, message lost locally.

```go
nc.Publish("orders.paid", order_json)  // Fire and forget
```

### Request-Reply

Publisher waits for response:

```
Requester → NATS Server → [Responder 1, Responder 2]
              ↑              Responder 1 replies
              └─ Response back to requester
```

Internally: requester subscribes to auto-generated reply subject; server routes responder's response.

```go
reply, _ := nc.Request("invoice.generate", order_json, time.Second)  // Timeout for reply
```

### Queue Groups: Work Distribution

Multiple subscribers on same subject can form a **queue group**. Each message delivered to exactly one subscriber in group (not a broadcast):

```
Publisher → NATS Server [order.process subject]
              ↓
         Queue Group "order-workers"
           [Worker 1, Worker 2, Worker 3]
              ↓
         Deliver to Worker 2 only (load balanced)
```

Use case: Distribute work evenly among competing workers.

```go
// Worker 1
nc.QueueSubscribe("orders.process", "order-workers", handler1)

// Worker 2
nc.QueueSubscribe("orders.process", "order-workers", handler2)

// Publisher
nc.Publish("orders.process", order_msg)  // Goes to Worker 1 OR Worker 2, not both
```

## Clustering & Leaf Nodes: Scalability

### Core Network

Multiple NATS servers form cluster (mesh topology):

```
Client A → [NATS Server 1] ←→ [NATS Server 2] ← Client B
                   ↕                    ↕
              [NATS Server 3]
```

Full mesh: All servers share subscriptions and route messages. Publish on Server 1 → delivered to subscribers on Server 2/3.

### Leaf Nodes: Isolation & Scaling

Large deployments use **leaf nodes** (outlying servers that connect to core cluster):

```
[Region 1 - Core Cluster]
 Server 1 ←→ Server 2 ←→ Server 3

[Region 2 - Leaf Nodes]
 Leaf 1 ←→ Leaf 2
    ↑       ↑
    └───────┴──→ (connects to Region 1 core)
```

Leaf nodes:
- Subscribe locally (no mesh communication overhead)
- Forward unknown subjects to core cluster
- Reduce network traffic between regions

Clients in Region 2 talk to nearby Leaf; Leaf fans out to core as needed.

## JetStream: Persistence & Exactly-Once

Core NATS is ephemeral (fire-and-forget). **JetStream** adds:

1. **Streams**: Durable log of messages (append-only per subject)
2. **Consumers**: Subscribers with persisted position (offset)
3. **Exactly-once delivery**: Acknowledgment tracking prevents duplicates

### Streams

A stream captures all messages for set of subjects:

```
Stream "orders":
  Subject: orders.paid
  Subject: orders.cancelled
```

Messages appended to log; old messages retained per policy:

| Policy | Behavior |
|--------|----------|
| **Max messages** | Trim oldest if exceed count |
| **Max bytes** | Trim oldest if exceed size |
| **Max age** | Delete messages older than duration |

Example: Retain 1 million messages or 7 days, whichever first.

### Consumers

Subscriber creates consumer for stream:

```go
consumer, _ := stream.AddConsumer(&ConsumerConfig{
    Durable: "order-processor-1",  // Persisted position
    DeliverPolicy: DELIVER_ALL,    // Start from beginning
})

// Process messages
inbox := fmt.Sprintf("_INBOX.%s.%%s", nuid.Next())
consumer.Push(&Order{}, inbox)   // Deliver messages to inbox

// Acknowledge to advance position
msg.Ack()  // Server: advance consumer offset
```

Sequence of consumer offsets persisted. If consumer dies, new instance consumes from saved offset → no message loss.

### Exactly-Once Semantics

Without acknowledgment, consumer's position doesn't advance. If subscriber crashes:

1. Subscriber dies mid-processing of message M
2. Server doesn't receive ack for M
3. New consumer starts from old offset
4. Receives M again
5. Reprocesses (may be idempotent or use at-least-once semantics)

Exactly-once requires:
- **Deterministic handling** (e.g., upsert with timestamp) or
- **Idempotent operations** (safe to reprocess)

JetStream provides "at-least-once"; application ensures exactly-once via idempotency.

## NATS KV: Distributed Configuration

Built on JetStream; behaves like distributed hashtable:

```go
kv, _ := js.KeyValue("config")  // Underlying stream: "$KV_config"

kv.Put("feature.flags.rollout", "0.5")      // 50% rollout
status := kv.Get("feature.flags.rollout")   // Returns value + revision

kv.Watch("feature.flags.>")  // Watch all keys under prefix
  // Receives updates in real-time
```

Internally:
- Stream per bucket stores all key versions
- Deletes implemented as tombstones (keys with empty value)
- Consumers used for watches

No strict quorum; eventually consistent within cluster.

### Use Case: Feature Flags

```
ConfigService → NATS KV (central truth)
                   ↑
  App 1 watches "flags.>", reconfigures on change
  App 2 watches "flags.>", reconfigures on change
  App 3 watches "flags.>", reconfigures on change
```

No polling; all apps notified within milliseconds.

## NATS Object Store: Blob Storage

Similar to KV but for large files. Divides objects into chunks:

```go
objStore, _ := js.ObjectStore("backups")

// Upload file
file, _ := os.Open("backup.tar.gz")
objectInfo, _ := objStore.PutFile("backup-20250325.tar.gz", file)

// Download
result, _ := objStore.GetFile("backup-20250325.tar.gz", "./backup.tar.gz")
```

Internally: Stream stores 128KB chunks; consumer reads all chunks, reconstructs.

## Subject-Based Routing: Flexibility

NATS subjects support **wildcards**:

```
orders.*      → Matches orders.paid, orders.cancelled (single level)
orders.>      → Matches orders.paid, orders.processing.pending (multi-level)
```

Subscribers can use wildcards:

```go
nc.Subscribe("orders.>", handler)  // Receive all order events
```

Publishers are explicit (no wildcards in publish).

This provides **topic-like abstraction** without pre-defining topics.

## Security: TLS, Auth, Tokens

### Transport Security

```go
nc, _ := nats.Connect(
    "nats://server:4222",
    nats.ClientCert("/path/cert.pem", "/path/key.pem"),
    nats.RootCAs("/path/ca.pem"),
)
```

HTTPS-like: TLS enforces encryption + server/client authentication.

### Authorization

NATS supports ACLs (per-user subject permissions):

```yaml
users:
  - username: app-worker
    password: pwd
    permissions:
      publish: ["orders.process"]
      subscribe: ["orders.>", "$JS.API.CONSUMER.>"]
```

User can publish to `orders.process` only; subscribe to order events and JetStream consumer endpoints.

### NKEYS: Public Key Authentication

Alternative to passwords; uses Ed25519 keys:

```go
nkey, _ := nkeys.FromSeed(seed)
nc, _ := nats.Connect("nats://server:4222", nats.Nkey(nkey))
```

Better for service-to-service authentication (no shared secrets).

## Comparison to Related Systems

| Feature | NATS | Kafka | RabbitMQ |
|---------|------|-------|----------|
| **Deployment** | Single binary | JVM, Zookeeper | Erlang cluster |
| **Core model** | Pub/Sub, Request-Reply | Topics/Partitions | Queues, Exchanges |
| **Persistence** | JetStream (optional) | Core feature | Durable queues |
| **Exactly-once** | Idempotent, at-least-once | Depends on config | Ack-based |
| **Setup complexity** | Minimal | Moderate | Moderate |
| **Ops overhead** | Very low | Medium | Medium |

## Limitations

1. **No strong ordering across partitions** — JetStream has per-subject ordering; multi-subject ordering requires client coordination
2. **No built-in transactions** — Publish to multiple subjects can partially fail
3. **Consumer lag tracking basic** — Not as rich as Kafka consumer group monitoring
4. **Subject hierarchy ad-hoc** — Wildcards are flexible but no schema enforcement

## Use Cases

**Microservices event bus**: Services publish domain events (orders.paid, users.created); subscribers react without tight coupling.

**Real-time coordination**: Kubernetes-like systems: control plane publishes desired state changes; workers watch and adapt.

**Distributed tracing**: Tracing backend publishes events; multiple consumers aggregate (metrics, alerting, analysis).

**Messaging at edge**: Leaf nodes in IoT networks reduce latency; edge devices publish to nearby leaf.

**Feature flag distribution**: Flags stored in NATS KV; apps watch, reconfigure instantly on change.

## See Also

- [Distributed Messaging Systems](distributed-messaging.md) — Kafka, RabbitMQ, patterns
- [MQTT](networking-mqtt.md) — IoT-focused pub/sub alternative
- [Distributed Coordination](distributed-coordination.md) — Using NATS for elections, locks