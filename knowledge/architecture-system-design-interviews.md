# System Design Interviews — Load Estimation, Scaling Patterns, and Trade-Off Analysis

System design interviews evaluate whether an engineer can build large-scale distributed systems. They test intuition about capacity, trade-offs, and design patterns—not memorized solutions. The skill is rapid estimation and principled decision-making under constraints.

## Load Estimation Fundamentals

All system design begins with **scale**: How many users? How much data? How fast must it respond?

### Users and Requests Per Second

Start with **daily active users (DAU)** and estimate **requests per second (RPS)**.

```
Assumptions:
- DAU = 1 million
- Active hours = 8 hours (peak) + 16 hours (off-peak)
- Peak RPS = DAU / active_seconds = 1,000,000 / (8 * 3600) ≈ 35 RPS
- Average RPS ≈ Peak RPS * 0.3 ≈ 10 RPS
- But peaks within peaks: assume 3-5x spike factor
- Design capacity ≈ Peak RPS * 5 = 175 RPS

Rule of thumb: Design for peak RPS in peak hours * 3-5x buffer.
```

### Storage Estimation

Data grows continuously. Estimate **daily new data** and **retention policy**.

```
URL Shortener example:
- New short links per day = 100 million
- Each short link record ≈ 200 bytes
  {shortcode: "abc", long_url: "...", created_at, user_id, metadata}
- Daily storage = 100M * 200B = 20 GB
- Yearly storage = 20GB * 365 = 7.3 TB
- With 10 years retention = 73 TB

If accessed frequently, assume it stays in cache.
If archived after 1 year, storage is cheap.

Cache ≈ (DAU * avg_links_per_user_per_day * retention_days) * record_size
     = (1M * 5 * 30) * 200B = 30 GB
     (fits on a few Redis nodes)
```

### Bandwidth Estimation

Network is the bottleneck. Estimate **bytes per request**.

```
Social media feed:
- RPS = 10,000
- Response = 50 KB (JSON with feed data)
- Outbound bandwidth = 10K RPS * 50 KB = 500 MB/s = 4 Gbps

Typical data center: 10 Gbps uplink
This design uses 40% of link; needs failover.

Rule of thumb:
- 1 Mbps = ~100 KB/s
- 1 Gbps = ~100 MB/s
```

### Latency Budget

Latency compounds. A 100ms response = many sub-100ms operations.

```
Example: Loading a web page
- DNS: 50ms
- TCP handshake: 50ms
- HTTPS negotiation: 100ms
- HTTP GET: 100ms
- Backend query: 100ms
- Rendering: 50ms
---
Total: 450ms perceived latency

Budget allocation:
- Database query: 50ms
- Cache lookup: 5ms
- Network I/O: 50-100ms
- Application logic: 20ms
- Buffer/GC: 30ms
---
When total approaches budget, optimize the slowest path.
```

## Back-of-Envelope Math Patterns

Common calculations and shortcuts:

| Item | Estimate |
|------|----------|
| 1 million requests/day | 11-12 RPS average |
| 1 second at 10K RPS | 10K operations |
| 1 GB RAM | ~50 million simple objects or ~1 billion integers |
| 1 TB disk | ~5 billion documents or ~10 billion small rows |
| Latency: disk seek | ~10ms |
| Latency: sequential disk read (1 MB) | ~1ms |
| Latency: memory access | ~100ns |
| Latency: network RTT (same datacenter) | ~1ms |
| Latency: network RTT (cross-continent) | ~100ms |

**Useful multipliers:**
- 1 Mbps = 125 KB/s
- 1Gbps = 125 MB/s
- 1 human second = 1 billion processor cycles (at GHz clock)

## Common System Design Patterns

### 1. URL Shortener (Bit.ly, TinyURL)

**Scope:**
- 100M new short links per day
- Retrieve links 1000x more often than creating
- 10-year retention

**Key decisions:**

| Decision | Options | Trade-Off |
|----------|---------|-----------|
| **Shortcode generation** | Random (collision-check) vs. Sequential | Random = simple, Sequential = predictable IDs |
| **Storage** | SQL (indexed on shortcode) vs. NoSQL (fast reads) | SQL has ACID, NoSQL scales horizontally |
| **Redirect** | 301 (caching) vs. 302 (tracking) | 301 = client caches, 302 = always hits server |
| **Cache** | CDN + local cache vs. Redis | CDN = global footprint, Redis = real-time |

**Design sketch:**
```
Client → CDN (global, TTL 1 hour)
      ↓ miss
      → Load Balancer
      → API Server (lookup cache first)
      → Redis Cache (1M top URLs)
      → Database (PostgreSQL replicated)
```

**Capacity:**
- New writes = 100M/day ≈ 1,200 writes/second
- Reads = 1,200 * 1000 = 1.2M reads/second (hits cache mostly)
- Database stores 3.5B URLs (10 years * 365 * 100M), ~700 GB indexed

### 2. Chat System (Slack, Facebook Messenger)

**Scope:**
- 10M active users
- Average 50 messages per user per day
- Low latency (< 1 second), high reliability

**Keyblocking problems:**
1. **Message ordering** — Users must see messages in order (causality matters)
2. **Presence** — Who's online?
3. **Notifications** — Offline users must know new messages exist
4. **Group chat** — Fan-out to many recipients

**Design decisions:**

| Problem | Solution | Why |
|---------|----------|-----|
| **Ordering** | Sequence numbers per conversation + monotonic IDs | Ensures global order, idempotent re-sends |
| **Scalability** | Partition conversations by user_id or conversation_id | Sharding ensures writes don't hotspot |
| **Real-time delivery** | WebSocket (stateful) + Message queue (fallback) | WebSocket = instant, queue = reliable |
| **Presence** | User heartbeat every 30s to presence service | Detects disconnects; easy to query |
| **Offline delivery** | Durable queue + retry with exponential backoff | Ensures delivery even if server down |

**Architecture:**
```
Client ──── WebSocket -→ Connection Handler
                          │
                          ├→ Message Queue (Kafka)
                          ├→ Message Storage (MongoDB)
                          ├→ Presence Service (Redis)
                          └→ Notification Service
```

Trade-offs: WebSockets are stateful (hard to scale) but instant. Message queues are resilient but add latency (100-1000ms). Most chat uses hybrid: WebSocket for online, queue+push for offline.

### 3. News Feed (Twitter, Facebook)

**Scope:**
- 100M users
- Each user has ~500 followers (avg)
- 500 posts/day in the system
- Real-time updates

**The fanout problem:**
- Celebrities have millions of followers
- Pushing updates to each follower is expensive
- But pulling (each user fetches from all follows) is slow

**Solutions:**

| Approach | Write Cost | Read Cost | Best For |
|----------|-----------|----------|----------|
| **Push (Fanout)** | High (distribute to all followers) | Low (read cache) | Anyone except celebrities |
| **Pull** | Low (post goes to DB) | High (query all follows) | Write-heavy users |
| **Hybrid** | Medium | Medium | Most systems |

**Hybrid design:**
- Normal users: push to follower caches when they post
- Power users (celebrities): pull only; followers query directly
- Threshold: switch to pull at ~1M followers

```
User Posts →
  if followers < 1M:
    Push to Redis cache for each follower
  else:
    Write to DB; followers pull when needed
```

**Feed generation:**
```
Client requests feed →
  Query cache (personalized feed, 1-hour TTL)
  Miss → aggregate:
    - Use fanout: posts in follower caches
    - Use pull: recent posts from followed users
    - Merge, rank, paginate (20 items per page)
    - Cache result
```

### 4. Search System (Google, Elasticsearch)

**Scope:**
- Index 1 billion documents
- Query response < 200ms
- Handle typos and synonyms

**Key challenges:**
1. **Index size** — 1 billion docs with ~500 bytes = 500 GB+ (compress, shard)
2. **Query latency** — Full table scan unacceptable
3. **Relevance** — Ranking matters

**Architecture:**
```
Query → Load Balancer
     → Search Node (query index, apply ranking)
     → {Shard 1, Shard 2, ...} (indexed documents)
     ↓
     Top-100 results from each shard
     → Rank, merge, return top-10
```

**Optimizations:**
- Inverted index: word → document list (fast lookup)
- Bloom filters: reject non-matching docs quickly
- Caching: common queries cached
- Sharding: 10 shards = 10x parallelism
- Replication: 3x per shard for fault tolerance

Typical queries hit 20% of shards (via bloom filters), rank in parallel, return results in 100-200ms.

## Scaling Patterns

### Horizontal Scaling

Add more servers. Works for stateless services; problematic for stateful ones.

**Stateless services (easy):**
- API servers, workers, batch processors
- Add instance → immediate capacity increase
- Typical headroom: 70% utilization

**Stateful services (hard):**
- Databases, caches, message queues
- Adding instance requires rebalancing state
- Often use consistent hashing to minimize disruption

### Data Partitioning (Sharding)

Distribute data across servers using a shard key.

```
Shard function: hash(user_id) % num_shards = shard_id

user_id=123 → shard 123 % 10 = 3 → Server 3
user_id=456 → shard 456 % 10 = 6 → Server 6
```

**Trade-offs:**
- **Range sharding** (by date, geography): uneven distribution, hot shards
- **Hash sharding** (hash of key): even distribution, harder to query ranges
- **Directory-based** (lookup table): flexible, adds query overhead

Resharding when adding shards is painful; plan for growth upfront.

### Caching Layers

Cache frequently accessed data closer to users.

| Layer | Latency | Hit Rate | Size |
|-------|---------|----------|------|
| L1: Browser cache | — | — | KB-MB |
| L2: CDN | 10-50ms | 70-90% | GB-TB |
| L3: Redis (in-datacenter) | 1-5ms | 90-99% | GB-100GB |
| L4: Database | 10-100ms | 5% | TB-PB |

Caches reduce load on DB; missing a cache layer often doubles latency.

**Cache invalidation strategies:**
- **TTL** — Simple, stale data acceptable
- **Event-driven** — Invalidate on writes, complex
- **Write-through** — Update cache + DB together, ensures consistency
- **Write-behind** — Update cache first, DB async (risky if cache fails)

### Database Replication

Copy data across servers for availability and read scaling.

```
Primary (writes) → Replica 1 (read-only)
                → Replica 2 (read-only)
                → Replica 3 (read-only)
```

**Sync vs. Async:**
- **Sync replication**: wait for replica ACK before committing (safe, slow)
- **Async replication**: commit locally, propagate to replicas (fast, might lose data if primary crashes)

Most systems use *semi-sync*: primary waits for one replica, async for others.

### Load Balancing

Distribute requests across servers. Methods:

| Method | Overhead | Affinity | Use Case |
|--------|----------|----------|----------|
| **Round-robin** | Low | No | Stateless services |
| **Least connections** | Medium | No | Long-lived connections |
| **Session hash** | Low | Yes | Sticky sessions |
| **Geo-based** | Low | Yes | CDN-style distribution |

## Trade-Off Analysis Framework

Every design choice trades something:

### Consistency vs. Availability

**Strong consistency** — All replicas agree before returning (expensive, slow, safe)
**Eventual consistency** — Return immediately, replicas sync later (fast, risky)

```
Flight booking: strong consistency (double-book is costly)
Social media likes: eventual consistency (exact count unimportant)
Bank account: strong consistency (money is precious)
Ad impressions: eventual consistency (off-by-one is OK)
```

### Latency vs. Throughput

**Batch processing** — Wait for 100 items, process together (high throughput, high latency)
**Stream processing** — Process immediately (low latency, lower throughput)

```
Financial reports: batch (low latency unimportant)
Chat messages: stream (latency critical)
```

### Storage vs. Compute

**Denormalization** — Store precomputed results (fat storage, fast reads)
**Normalization** — Compute on-read (lite storage, slow reads)

```
Social feed: denormalized (pre-compute for each user)
Invoice system: normalized (compute on-demand)
```

## Interview Execution Checklist

1. **Clarify scope** — "100K users? 100M? Worldwide?" Don't assume.
2. **Estimate load** — DAU → RPS, storage, bandwidth. Speak aloud.
3. **Enumerate concerns** — Scalability, availability, consistency, latency.
4. **Pick an area** — Database design? Caching? Don't try to solve everything.
5. **Design sketch** — Draw boxes and arrows. Be specific about tech.
6. **Estimate again** — "At 100K RPS, do we need this cache layer?"
7. **Discuss trade-offs** — "If we use eventual consistency, reads are faster but..."
8. **Mention disasters** — "If the database crashes, here's the recovery..."
9. **Ask for feedback** — "What would you change?"

The best answers aren't perfect systems—they're clear reasoning and willingness to revise based on constraints.

See also: [system-design-distributed.md](system-design-distributed.md), [database-scaling.md](database-scaling.md), [distributed-consensus.md](distributed-consensus.md)