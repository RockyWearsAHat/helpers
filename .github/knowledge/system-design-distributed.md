# System Design & Distributed Systems — From Single Box to Planet Scale

## Why This Matters

Every non-trivial application eventually needs more than one machine. Understanding distributed systems separates engineers who build toys from engineers who build infrastructure. The principles here underpin every cloud service, database, message broker, and CDN you'll ever use.

---

## The Fundamental Constraints

### CAP Theorem (Brewer's Theorem)

A distributed system can provide at most **two of three** guarantees simultaneously:

| Guarantee               | Meaning                                                 |
| ----------------------- | ------------------------------------------------------- |
| **Consistency**         | Every read sees the most recent write (linearizability) |
| **Availability**        | Every request receives a non-error response             |
| **Partition Tolerance** | System continues operating despite network splits       |

**The real insight:** Network partitions _will_ happen (P is non-negotiable). Your actual choice is **CP vs AP**:

| Choice | Behavior During Partition                   | Examples                                  |
| ------ | ------------------------------------------- | ----------------------------------------- |
| **CP** | Reject some requests to stay consistent     | ZooKeeper, etcd, HBase, MongoDB (default) |
| **AP** | Serve possibly stale data to stay available | Cassandra, DynamoDB, CouchDB, DNS         |

**PACELC Extension:** Even when there's **no** partition (E = else), you still face a **Latency vs Consistency** tradeoff:

- Partition → choose A or C
- Else → choose L or C

```
Example: DynamoDB is PA/EL  (partition → available, else → low latency)
         ZooKeeper is PC/EC (partition → consistent, else → consistent)
```

### The Eight Fallacies of Distributed Computing (Peter Deutsch, 1994)

Every one of these is **false** — and your code must handle each failure case:

1. The network is reliable
2. Latency is zero
3. Bandwidth is infinite
4. The network is secure
5. Topology doesn't change
6. There is one administrator
7. Transport cost is zero
8. The network is homogeneous

**Practical implication:** Every network call can fail, be slow, be reordered, or be duplicated. Design for it.

---

## Consensus & Coordination

### Why Consensus Is Hard

Getting multiple machines to agree on a value seems simple — until machines crash, messages are delayed, and networks partition. The **FLP Impossibility Result** (1985) proves no deterministic consensus protocol can guarantee termination in an asynchronous system with even one faulty process.

### Raft (The Understandable Consensus Algorithm)

**Use when:** You need a replicated state machine (leader election, distributed config, metadata).

```
Term 1: Leader A    ─────────────────────────────>
         Follower B  ─────────────────────────────>
         Follower C  ─────────────────────────────>

Term 2: (A crashes)
         Leader B    ─────────────────────────────>
         Follower C  ─────────────────────────────>
         (A recovers as Follower)
```

**Key mechanics:**

- **Leader election:** Randomized timeouts prevent split-brain. Candidate requests votes; majority wins.
- **Log replication:** Leader appends entries → sends to followers → commits after majority acknowledgment.
- **Safety:** Committed entries are never lost (guaranteed by election restriction: candidates must have all committed entries).
- **Membership changes:** Joint consensus — old and new configurations overlap to prevent split-brain during transitions.

**Real implementations:** etcd (Kubernetes), CockroachDB, TiKV, Consul, RethinkDB.

### Paxos (The Original)

Same problem as Raft but historically earlier and notoriously hard to understand. **Use Raft in practice** unless you're building a database kernel.

**Key difference:** Paxos separates proposers, acceptors, and learners. Multi-Paxos optimizes for steady-state with a stable leader (which is essentially what Raft formalizes).

### ZooKeeper's ZAB (ZooKeeper Atomic Broadcast)

Designed specifically for primary-backup replication with ordered broadcast. Used by: Kafka (metadata), HBase, Hadoop.

---

## Consistency Models — The Spectrum

From strongest to weakest (stronger = more intuitive, weaker = more performant):

| Model                      | Guarantee                                                                           | Example                  |
| -------------------------- | ----------------------------------------------------------------------------------- | ------------------------ |
| **Linearizability**        | Operations appear instantaneous at some point between invocation and response       | Single-node DB, etcd     |
| **Sequential Consistency** | All operations appear in some total order consistent with each process's order      | Zookeeper (per-client)   |
| **Causal Consistency**     | Causally related operations appear in order; concurrent ops may differ per observer | CRDT-based systems       |
| **Eventual Consistency**   | All replicas converge eventually if writes stop                                     | DynamoDB, Cassandra, DNS |

### CRDTs (Conflict-Free Replicated Data Types)

**The dream:** Replicas that can be modified independently and always merge without conflicts.

**How they work:** Mathematical structures where all concurrent operations commute (order doesn't matter).

| Type             | Example                                 | Use Case               |
| ---------------- | --------------------------------------- | ---------------------- |
| **G-Counter**    | Grow-only counter per node; sum to read | Distributed page views |
| **PN-Counter**   | Two G-Counters (positive + negative)    | Like/unlike counts     |
| **G-Set**        | Grow-only set (add only)                | Seen-message IDs       |
| **OR-Set**       | Observed-Remove set (add + remove)      | Shopping cart items    |
| **LWW-Register** | Last-Writer-Wins by timestamp           | User profile fields    |

**Real use:** Redis CRDT, Riak, Automerge (collaborative editing), Yjs (real-time collaboration).

### Vector Clocks & Logical Time

**Problem:** Physical clocks drift. You can't reliably order events across machines by wall-clock time.

**Lamport Clocks:** Single integer. Captures happens-before ordering. Cannot detect concurrent events.

**Vector Clocks:** One counter per node. Can detect both ordering AND concurrency.

```
Node A: [A:1, B:0, C:0]  →  sends msg  →  [A:2, B:0, C:0]
Node B: [A:0, B:1, C:0]  →  receives    →  [A:2, B:2, C:0]
                                              (merge: max per component + increment own)

Concurrent detection: [A:2, B:1] vs [A:1, B:2]  →  neither dominates  →  CONCURRENT
```

**Hybrid Logical Clocks (HLC):** Combine physical timestamps with logical counters. Used in CockroachDB, YugabyteDB. Best of both worlds: human-readable time + causal ordering.

---

## Replication Strategies

### Single-Leader (Primary-Backup)

```
Client → Leader (writes) → Follower 1 (reads)
                         → Follower 2 (reads)
                         → Follower 3 (reads)
```

**Sync replication:** Leader waits for follower ACK before confirming write. Durability guaranteed, higher latency.
**Async replication:** Leader confirms immediately. Lower latency, risk of data loss on leader failure.
**Semi-sync:** Wait for at least one follower. The practical sweet spot (MySQL, PostgreSQL).

### Multi-Leader (Active-Active)

Each datacenter has its own leader. Writes go to local leader, replicate asynchronously.

**The problem:** Write conflicts. Resolution strategies:

- **Last-writer-wins (LWW):** Simple, loses data silently
- **Application-level resolution:** Application receives conflicts, decides (CouchDB)
- **CRDTs:** Automatic merge, limited data types

**Use when:** Multi-datacenter deployment where you need local write latency.

### Leaderless (Dynamo-Style)

Client sends writes to multiple replicas. Reads from multiple replicas. Quorum math decides consistency:

```
W + R > N  →  guaranteed to read the latest write

N = total replicas       (e.g., 3)
W = write acknowledgments required  (e.g., 2)
R = read acknowledgments required   (e.g., 2)

W=2, R=2, N=3  →  2+2=4 > 3  →  CONSISTENT
W=1, R=1, N=3  →  1+1=2 < 3  →  EVENTUALLY CONSISTENT (faster)
```

**Anti-entropy:** Background process compares replicas and fixes divergence (Merkle trees for efficiency).
**Read repair:** On read, if replicas disagree, update stale ones.

---

## Partitioning (Sharding)

### Why Partition?

One machine can't hold all the data or handle all queries. Partition data across nodes.

### Strategies

| Strategy               | How                                 | Pros                              | Cons                              |
| ---------------------- | ----------------------------------- | --------------------------------- | --------------------------------- |
| **Hash partitioning**  | hash(key) mod N                     | Even distribution                 | No range queries                  |
| **Range partitioning** | Key ranges per partition            | Range queries work                | Hot spots if keys cluster         |
| **Consistent hashing** | Hash ring with virtual nodes        | Minimal rebalancing on add/remove | Slightly uneven load              |
| **Directory-based**    | Lookup table maps keys → partitions | Flexible                          | Lookup table is a bottleneck/SPOF |

### Consistent Hashing (Deep Dive)

```
Hash Ring (0 to 2^32):

        Node A (position 1000)
       /
  ──●──────────●──────────●──────────●──
    0       Node B       Node C      2^32
            (pos 5000)   (pos 8000)

Key with hash 3000 → goes to next node clockwise → Node B
Key with hash 9000 → wraps around → Node A

Adding Node D at position 6000:
Only keys between 5000-6000 move (from C to D). Everything else stays.
```

**Virtual nodes:** Each physical node owns multiple positions on the ring. Fixes load imbalance. Standard: 100-200 virtual nodes per physical node.

### Rebalancing

- **Fixed partitions:** Create many more partitions than nodes (e.g., 1000 partitions, 10 nodes). Move whole partitions between nodes.
- **Dynamic partitioning:** Split partitions that grow too large, merge small ones. Like a B-tree at the cluster level.
- **Never rebalance by hash mod N.** Adding a node reshuffles everything.

---

## Patterns for Distributed Systems

### Saga Pattern (Distributed Transactions Without 2PC)

**Problem:** You need atomicity across services (order → payment → inventory → shipping), but 2PC blocks and doesn't scale.

**Solution:** A sequence of local transactions, each with a compensating action if a later step fails.

```
Step 1: Create Order          ←  Compensate: Cancel Order
Step 2: Reserve Payment       ←  Compensate: Refund Payment
Step 3: Reserve Inventory     ←  Compensate: Release Inventory
Step 4: Arrange Shipping      ←  Compensate: Cancel Shipping

If Step 3 fails:
  → Run Compensate Step 2 (refund)
  → Run Compensate Step 1 (cancel order)
```

**Orchestration:** Central coordinator tells each service what to do.
**Choreography:** Each service publishes events; next service reacts. No central coordinator. Harder to debug but more decoupled.

### Event Sourcing

**Instead of storing current state, store the sequence of events that produced it.**

```
Traditional:   Account { balance: 150 }
Event Sourced: [Opened(0), Deposited(200), Withdrew(50)]

Current state = replay all events
Any historical state = replay events up to that point
```

**Benefits:** Complete audit trail, temporal queries, easy debugging ("what happened?"), can rebuild read models.
**Costs:** Event schema evolution is hard, eventual consistency for read models, storage grows forever (use snapshots).

### CQRS (Command Query Responsibility Segregation)

Separate the write model (commands) from the read model (queries).

```
Commands → Write Store (optimized for writes, normalized)
                ↓ (events/projections)
Queries  → Read Store (optimized for reads, denormalized, possibly different DB)
```

**Use when:** Read and write patterns differ dramatically (e.g., writes are complex domain logic, reads are simple lookups across denormalized views).

**Often combined with Event Sourcing:** Events from the write side project into materialized read views.

### Circuit Breaker

**Problem:** A downstream service is failing. Your retries amplify the problem (thundering herd).

```
States:
  CLOSED (normal)  →  failures exceed threshold  →  OPEN (fast-fail)
  OPEN             →  timeout expires           →  HALF-OPEN (test)
  HALF-OPEN        →  test succeeds             →  CLOSED
  HALF-OPEN        →  test fails                →  OPEN
```

**Configuration levers:**

- Failure threshold (e.g., 5 failures in 60 seconds)
- Open duration (e.g., 30 seconds before trying again)
- Half-open max requests (e.g., 3 test requests)

**Libraries:** Hystrix (Java, deprecated but influential), resilience4j (Java), Polly (.NET), cockatiel (TypeScript).

### Bulkhead

Isolate components so one failing component doesn't take down everything.

```
Thread Pool Bulkhead:
  Service A: [pool of 20 threads]  ← if A is slow, only these 20 block
  Service B: [pool of 20 threads]  ← B continues normally
  Service C: [pool of 10 threads]
```

### Backpressure

When a producer is faster than a consumer, the system must signal the producer to slow down rather than buffer endlessly (which leads to OOM).

**Strategies:**

- Drop: Discard excess items (acceptable for metrics/telemetry)
- Buffer with bounded queue: Reject when full (HTTP 503)
- Rate limiting: Token bucket or leaky bucket at the producer
- Reactive Streams: Protocol-level backpressure (consumer requests N items)

---

## Load Balancing

### Algorithms

| Algorithm                | How It Works                                  | Best For                         |
| ------------------------ | --------------------------------------------- | -------------------------------- |
| **Round Robin**          | Rotate through servers                        | Equal-capacity servers           |
| **Weighted Round Robin** | Rotate with weights                           | Mixed-capacity servers           |
| **Least Connections**    | Send to server with fewest active connections | Variable request duration        |
| **Least Response Time**  | Combine connections + response time           | Latency-sensitive                |
| **IP Hash**              | Hash client IP → consistent server            | Session affinity without cookies |
| **Consistent Hash**      | Hash ring (see above)                         | Distributed caches               |
| **Random Two Choices**   | Pick 2 random servers, send to less loaded    | Simple, surprisingly effective   |

### Layer 4 vs Layer 7

```
Layer 4 (Transport): Routes based on IP + port. Fast. No content inspection.
  Examples: HAProxy (TCP mode), AWS NLB, IPVS

Layer 7 (Application): Routes based on HTTP headers, URL path, cookies. Slower. Smart.
  Examples: Nginx, HAProxy (HTTP mode), AWS ALB, Envoy, Traefik
```

### Service Mesh

**Problem:** In microservices, every service needs: load balancing, retries, circuit breaking, mTLS, observability. Implementing in each service is madness.

**Solution:** Sidecar proxy next to each service handles networking concerns.

```
[Service A] ↔ [Envoy Proxy] ←→ [Envoy Proxy] ↔ [Service B]
                    ↕                  ↕
              [Control Plane (Istio/Linkerd)]
```

**Key service meshes:**

- **Istio** (Envoy-based): Feature-rich, complex
- **Linkerd** (Rust proxy): Lighter, simpler, fast
- **Consul Connect** (HashiCorp): Integrated with Consul service discovery

---

## Caching Strategies

### Cache Patterns

| Pattern                       | Flow                                                    | Trade-offs                                              |
| ----------------------------- | ------------------------------------------------------- | ------------------------------------------------------- |
| **Cache-Aside (Lazy)**        | App checks cache → miss → load from DB → store in cache | Simple, stale data possible                             |
| **Read-Through**              | Cache itself loads from DB on miss                      | Cleaner code, cache must know DB schema                 |
| **Write-Through**             | Write to cache + DB synchronously                       | Consistent, higher write latency                        |
| **Write-Behind (Write-Back)** | Write to cache → async flush to DB                      | Fast writes, risk of data loss                          |
| **Refresh-Ahead**             | Proactively refresh entries near expiry                 | Reduces latency spikes, wastes resources on unread keys |

### Cache Invalidation (The Hard Part)

> "There are only two hard things in Computer Science: cache invalidation and naming things." — Phil Karlton

**Strategies:**

- **TTL (Time-to-Live):** Simple. Accept staleness window. Set based on data change frequency.
- **Event-driven invalidation:** Publish cache-invalidation events on writes. More complex, lower staleness.
- **Version keys:** Include version in cache key. Increment on change. Old keys expire naturally.

**Cache stampede prevention:**

- **Lock/mutex:** First thread locks, fetches, populates. Others wait.
- **Probabilistic early expiration:** Each reader has a small chance of refreshing before TTL expires.
- **Stale-while-revalidate:** Serve stale data immediately, refresh in background.

### Cache Sizing

**Working set:** The subset of data actively accessed. Your cache should hold the working set.
**Hit ratio target:** 90%+ for most workloads. Below 80% means your cache is too small or your access pattern is too random.

---

## Message Queues & Event Streaming

### Message Queue (Point-to-Point)

```
Producer → [Queue] → Consumer
                   → Consumer  (competing consumers — each message processed once)
```

**Use when:** Task distribution, work queues, request buffering.
**Tools:** RabbitMQ, Amazon SQS, Redis (with Streams or Lists).

### Event Streaming (Pub/Sub with Persistence)

```
Producer → [Topic/Partition] → Consumer Group A (all messages)
                              → Consumer Group B (all messages)

Each consumer group gets every message. Within a group, partitions are divided among consumers.
```

**Use when:** Event sourcing, stream processing, data pipelines, decoupled microservices.
**Tools:** Apache Kafka, Amazon Kinesis, Redpanda, Apache Pulsar.

### Kafka Deep Dive

```
Topic: "orders"
  Partition 0: [msg1, msg4, msg7, msg10, ...]   → Consumer A
  Partition 1: [msg2, msg5, msg8, msg11, ...]   → Consumer B
  Partition 2: [msg3, msg6, msg9, msg12, ...]   → Consumer C

Ordering: Guaranteed WITHIN a partition. NOT across partitions.
Key-based routing: Orders for same customer → same partition → ordered per customer.
```

**Critical concepts:**

- **Consumer offsets:** Each consumer group tracks its position per partition. Can replay by resetting offsets.
- **Retention:** Time-based (7 days default) or size-based. Can be infinite (compacted topics).
- **Compaction:** Keep only the latest value per key. Ideal for changelogs and state stores.
- **Exactly-once semantics:** Possible with idempotent producers + transactional consumers (Kafka 0.11+).

### Delivery Guarantees

| Guarantee         | Meaning                             | How                                                                 |
| ----------------- | ----------------------------------- | ------------------------------------------------------------------- |
| **At-most-once**  | May lose messages. Never duplicate. | Fire and forget.                                                    |
| **At-least-once** | Never lose. May duplicate.          | ACK after processing. Retry on failure.                             |
| **Exactly-once**  | Never lose. Never duplicate.        | Idempotent processing + transactional commit. Complex but possible. |

**Practical rule:** Design for **at-least-once** + **idempotent consumers**. It's simpler and achieves effectively exactly-once semantics.

---

## Distributed Storage

### Object Storage (S3 Pattern)

```
Key-value store for blobs. Flat namespace (no real directories).
PUT /bucket/path/to/file.jpg  →  stored with redundancy (erasure coding)
GET /bucket/path/to/file.jpg  →  retrieved from nearest replica
```

**Durability:** 99.999999999% (11 nines). You'll lose a file once every 10 million years.
**Use for:** Static assets, backups, data lake, ML training data, logs.

### Distributed File Systems

| System             | Use Case                                | Key Property                         |
| ------------------ | --------------------------------------- | ------------------------------------ |
| **HDFS**           | Batch analytics (Hadoop)                | Optimized for large sequential reads |
| **GFS / Colossus** | Google's internal storage               | Inspired HDFS                        |
| **Ceph**           | Unified storage (block + object + file) | CRUSH algorithm for placement        |
| **MinIO**          | S3-compatible on-premise                | Single binary, fast                  |

### Time-Series Databases

| DB                  | Backed By            | Key Feature                             |
| ------------------- | -------------------- | --------------------------------------- |
| **InfluxDB**        | InfluxData           | Purpose-built, flux query language      |
| **TimescaleDB**     | PostgreSQL extension | Full SQL, hypertables                   |
| **Prometheus**      | CNCF                 | Pull-based metrics, PromQL              |
| **VictoriaMetrics** | Open-source          | High performance, Prometheus-compatible |
| **ClickHouse**      | Yandex               | Columnar, fast aggregation              |

---

## System Design Process (Interview & Real World)

### Step-by-Step Framework

1. **Clarify requirements** (5 min in interview)
   - Functional: What does the system DO?
   - Non-functional: Scale (users, data), latency, availability, consistency
   - Constraints: Budget, team size, existing infra

2. **Back-of-envelope estimation**

   ```
   Users: 100M monthly, 10M daily active
   Writes: 10M * 5 writes/day = 50M writes/day ≈ 600 writes/sec
   Reads: 10:1 read:write → 6000 reads/sec
   Storage: 50M writes * 1KB = 50GB/day = 18TB/year
   Bandwidth: 6000 reads/sec * 1KB = 6MB/sec
   ```

3. **High-level design** (API + data flow + major components)
4. **Detailed design** (data model, specific algorithms, specific technologies)
5. **Identify bottlenecks** (single points of failure, hot spots, scaling limits)
6. **Scale & optimize** (caching, CDN, sharding, async processing)

### Latency Numbers Every Programmer Should Know

```
L1 cache reference                    0.5 ns
Branch mispredict                     5   ns
L2 cache reference                    7   ns
Mutex lock/unlock                    25   ns
Main memory reference               100   ns
Compress 1KB with Snappy          3,000   ns  (3 μs)
Send 1KB over 1 Gbps network    10,000   ns  (10 μs)
Read 4KB randomly from SSD      150,000   ns  (150 μs)
Read 1MB sequentially from memory 250,000 ns  (250 μs)
Round trip within same datacenter  500,000 ns  (500 μs = 0.5 ms)
Read 1MB sequentially from SSD  1,000,000 ns  (1 ms)
HDD seek                       10,000,000 ns  (10 ms)
Read 1MB sequentially from HDD 20,000,000 ns  (20 ms)
Send packet CA→Netherlands→CA  150,000,000 ns  (150 ms)
```

**Key takeaways:**

- Memory is 100x faster than SSD, 1000x faster than HDD
- Network within datacenter = 0.5ms, across continent = 150ms
- Compress before sending over network (almost always worth it)
- Sequential reads are 100-1000x faster than random reads

### Common System Design Problems

| System              | Key Challenges                                    | Core Components                                |
| ------------------- | ------------------------------------------------- | ---------------------------------------------- |
| **URL Shortener**   | Hash collisions, read-heavy (301 vs 302 redirect) | Hash function + KV store + cache               |
| **Rate Limiter**    | Distributed counting, sliding window precision    | Token bucket / sliding window + Redis          |
| **Chat System**     | Real-time delivery, presence, offline messages    | WebSocket + message queue + fanout             |
| **News Feed**       | Fanout on write vs read, ranking algorithm        | Timeline cache + social graph + ranking        |
| **Notification**    | Multi-channel, reliability, deduplication         | Priority queue + templates + delivery tracking |
| **Search Engine**   | Inverted index, ranking (TF-IDF/BM25), crawling   | Crawler + indexer + query parser + ranker      |
| **Video Streaming** | Transcoding, adaptive bitrate, CDN, storage       | Upload pipeline + CDN + ABR (HLS/DASH)         |
| **Ride Sharing**    | Geospatial matching, real-time tracking, pricing  | Geohash/quadtree + matching engine + pricing   |

---

## Observability in Distributed Systems

### Distributed Tracing

**Problem:** A request touches 10 services. Where's the bottleneck? Where did it fail?

**Solution:** Propagate a trace ID through every service call. Each service reports spans.

```
Trace: abc-123
├── [API Gateway]          0ms ─────────────── 200ms
│   ├── [Auth Service]     5ms ──── 15ms
│   ├── [Order Service]    20ms ────────────── 180ms
│   │   ├── [DB Query]     25ms ──── 45ms
│   │   ├── [Inventory]    50ms ───────── 120ms  ← SLOW
│   │   └── [Payment]      125ms ─── 170ms
│   └── [Notification]     185ms ── 195ms
```

**Standards:** OpenTelemetry (OTEL) — the converged standard. Combines Jaeger + Zipkin + OpenTracing + OpenCensus.

**Tools:** Jaeger, Zipkin, Tempo (Grafana), Datadog APM, Honeycomb.

### Health Checks & Readiness

```
/health  → Am I alive? (liveness probe)
/ready   → Can I serve traffic? (readiness probe — DB connected, cache warmed, etc.)

Kubernetes uses these:
  livenessProbe:  restart container if unhealthy
  readinessProbe: remove from load balancer if not ready
  startupProbe:   prevent premature liveness checks during slow starts
```

---

## Anti-Patterns to Avoid

| Anti-Pattern                         | Problem                                         | Solution                                           |
| ------------------------------------ | ----------------------------------------------- | -------------------------------------------------- |
| **Distributed Monolith**             | Microservices that must deploy together         | Proper bounded contexts, async communication       |
| **Two-Phase Commit everywhere**      | Blocks all participants, doesn't scale          | Saga pattern with compensating transactions        |
| **Shared database between services** | Tight coupling, schema changes break everything | Database-per-service, events for integration       |
| **Chatty services**                  | N+1 queries across network                      | Batch APIs, BFF (Backend-for-Frontend), GraphQL    |
| **No idempotency**                   | Retries cause duplicate actions                 | Idempotency keys on all mutating operations        |
| **Ignoring partial failure**         | Assuming all-or-nothing in distributed calls    | Graceful degradation, fallback responses, timeouts |
| **Unbounded retries**                | Retry storms amplify failures                   | Exponential backoff + jitter + circuit breaker     |
| **Synchronous chains**               | Service A → B → C → D, all blocking             | Async messaging where possible, timeout budgets    |

---

## Quick Reference: Technology Decision Matrix

### Choosing a Database

| Need                        | Technology                    | Why                                     |
| --------------------------- | ----------------------------- | --------------------------------------- |
| Relational + ACID           | PostgreSQL                    | Best all-around RDBMS                   |
| Document store              | MongoDB                       | Flexible schema, horizontal scaling     |
| Wide-column (massive scale) | Cassandra, ScyllaDB           | Linear scalability, tunable consistency |
| Key-value (simple, fast)    | Redis, DragonflyDB            | Sub-millisecond, in-memory              |
| Graph relationships         | Neo4j, Amazon Neptune         | Traverse relationships efficiently      |
| Time-series                 | TimescaleDB, InfluxDB         | Optimized for time-ordered data         |
| Search                      | Elasticsearch, Meilisearch    | Full-text search, fuzzy matching        |
| Analytical (OLAP)           | ClickHouse, DuckDB, BigQuery  | Columnar, fast aggregation              |
| NewSQL (distributed SQL)    | CockroachDB, YugabyteDB, TiDB | SQL + horizontal scaling + ACID         |

### Choosing a Message Broker

| Need                              | Technology          | Why                                   |
| --------------------------------- | ------------------- | ------------------------------------- |
| Simple task queue                 | Redis Streams, SQS  | Low overhead                          |
| Complex routing                   | RabbitMQ            | Exchanges, bindings, DLQ              |
| Event streaming (high throughput) | Kafka, Redpanda     | Partitioned log, replay, exactly-once |
| Real-time pub/sub                 | Redis Pub/Sub, NATS | Low latency, lightweight              |
| Multi-protocol                    | Apache Pulsar       | Streaming + queuing, tiered storage   |
