# Distributed Caching — Architecture, Consistency, and Load Mitigation

## Overview

A distributed cache spreads data across multiple cache servers to reduce latency and database load. Caching trades storage and coordination complexity for performance: a cache hit is 1-10 microseconds, a database query is 1-100 milliseconds. Distributing the cache introduces challenges: maintaining consistency across replicas, handling cache failures, and preventing thundering herds when popular items expire.

## Cache Topologies

### Client-Side Caching

The application process caches data in its own memory. No separate cache cluster.

**Advantages:**
- Fastest lookups (local memory, no network roundtrip).
- No coordination between clients.

**Disadvantages:**
- Limited capacity (one process's memory).
- No cache sharing between processes or machines.
- Difficult to invalidate across clients when data changes.

### Sidecar (Co-located) Caching

A cache process (e.g., Redis) runs on the same machine as the application. The application communicates with the cache via Unix socket (faster than TCP).

**Advantages:**
- Low latency (local network).
- Simple failover (cache and app fail together).

**Disadvantages:**
- No cache sharing between machines.
- Resource contention (cache competes with application for CPU and memory).

### Dedicated Cache Cluster

The cache runs as a separate cluster (e.g., Memcached or Redis Cluster) shared by multiple application servers.

**Advantages:**
- Cache is shared and aggregated across all clients.
- Can scale cache independently from application.
- Centralized cache invalidation.

**Disadvantages:**
- Network roundtrip adds latency (5-10ms in a data center).
- Cache cluster is a potential single point of failure.
- Requires coordination between cache nodes.

## Memcached Architecture

Memcached is a simple, distributed in-memory cache designed for horizontal scaling.

**Design:**
- Clients use consistent hashing to map keys to cache nodes.
- Each node is stateless and does not replicate data (no replication).
- Clients directly contact the node holding the key.
- If a node fails, its data is lost; application code retries the database query.

**Advantages:**
- Simple, predictable, and lean (minimal resource overhead).
- Horizontal scaling by adding more nodes and rehashing keys.

**Disadvantages:**
- No replication; node failures cause cache misses.
- No consistency guarantees: writes to the database don't automatically invalidate cached copies.
- Cache misses in a thundering herd can spike database load.

## Redis Cluster

Redis Cluster adds replication, failover, and richer data structures.

**Design:**
- Partitions data into slots (16,384 by default).
- Each slot is assigned to a primary node and one or more replicas.
- Primary nodes accept both reads and writes; replicas accept reads only.
- If a primary fails, a replica is elected as the new primary (via consensus).

**Advantages:**
- Replication provides fault tolerance and read scalability.
- Atomic transactions and Lua scripting for complex operations.
- Automatic failover.

**Disadvantages:**
- More complex than Memcached; higher operational burden.
- Resharding (adding/removing nodes) requires moving keys and involves temporary cluster downtime or partial unavailability.
- Strong consistency guarantees limit write throughput on primary nodes.

## Eviction Policies

When cache memory is full, eviction policies decide which items to remove.

### LRU (Least Recently Used)

Evict items that haven't been accessed in the longest time.

**Advantages:**
- Intuitive and widely used.
- Performs well for workloads with temporal locality (popular items accessed recently stay cached).

**Disadvantages:**
- Scans or updates to an item "touch" it, even if the data isn't read (can keep stale items in cache).
- No awareness of item value (expensive computations and cheap computations evict equally).

### LFU (Least Frequently Used)

Evict items accessed least often.

**Advantages:**
- Better for uneven access patterns (popular items are rarely evicted).

**Disadvantages:**
- Requires counters per item (memory overhead).
- "Temporal decay" must be added to avoid old popular items dominating forever.

### Adaptive LRU (Redis)

Combines LRU with a sampling approach. Redis samples a random subset of keys and removes the least recently used among them. Adjusts sample size based on eviction rate.

**Advantages:**
- Low memory overhead (no per-key metadata).
- Close to true LRU with tunable accuracy.

**Disadvantages:**
- Approximate; not a true LRU.

### TTL (Time-To-Live)

Items have an explicit expiration time. Expired items are removed passively (on access) or actively (background garbage collection).

**Advantages:**
- Application-driven; explicit semantics.
- Works well with database consistency (set TTL to the staleness tolerance).

**Disadvantages:**
- Requires active expiration to reclaim memory (consumes CPU).
- Large number of TTL misses if expiration time is short.

## Cache Consistency Challenges

### Thundering Herd (Cache Stampede)

When a popular cached item expires, many concurrent requests hit the cache simultaneously, get a miss, and all query the database. Database load spikes sharply.

**Scenario:**
1. Cache key `user_profile_123` has TTL 1 hour.
2. At hour 1, 1000 requests arrive simultaneously.
3. All miss the cache, all query the database.
4. Database throughput spikes from 100 req/s to 10,000 req/s.
5. Database connection pool exhausts, responses timeout, errors cascade.

**Mitigations:**
- **Lease-based expiration:** Extend the TTL for a client that requested it (only one client rebuilds the cache, others wait).
- **Probabilistic early expiration:** Expire items with probability $p$ before the hard deadline, triggering early refresh and spreading load.
- **Cache warming:** Proactively refresh popular items before they expire (batch refresh in background).
- **Request coalescing:** Multiple requests for the same expired key block until one completes the database query, then all use the result.

### Cache Invalidation

When underlying data changes, cached copies must be invalidated or updated.

**Write-Through:** Update the cache and database together in a transaction. Slower but ensures consistency.

**Write-Behind (Write-Back):** Update the cache and later queue a database write (batch updates). Fast but risks data loss if cache fails before the write completes.

**TTL-Based:** Let items expire naturally; no explicit invalidation. Simple but risks stale reads.

**Event-Driven:** Publish data change events; subscribers invalidate cached copies. More complex but allows fine-grained invalidation.

## Cache Placement

### Cache-Aside (Lazy Loading)

Application checks the cache first. On miss, fetch from database and populate the cache.

```
if cache.has(key):
  return cache.get(key)
else:
  value = database.fetch(key)
  cache.put(key, value)
  return value
```

**Advantages:** Only cacheable data ends up in cache; application controls what to cache.

**Disadvantages:** Misses are expensive (block the user's request).

### Read-Through

Cache handles the check and database fetch transparently. Application always queries the cache.

**Advantages:** Simpler application code; cache owns consistency.

**Disadvantages:** All data (even uncacheable) flows through cache; cache must know database schema.

### Write-Through

Application writes to cache and database together, in order.

**Advantages:** Cache and database stay consistent.

**Disadvantages:** Every write involves the cache (slower).

## CDN Caching Layer

Content Delivery Networks (CDNs) cache static content (HTML, images, videos) at edge locations worldwide.

**HTTP Cache-Control Headers:**
- `max-age`: Item is fresh for N seconds.
- `s-maxage`: Shared cache (CDN) freshness; overrides `max-age`.
- `must-revalidate`: Always check with origin server if expired.

CDNs use these headers to cache aggressively at edges while allowing fast origin updates.

## Monitoring and Observability

Critical metrics:
- **Hit ratio:** % of requests served from cache.
- **Eviction rate:** Items removed per second (indicator of cache size inadequacy).
- **Latency (p50, p99):** Cache hit vs. miss latency.
- **Error rate:** Cache node failures and timeouts.

A hit ratio below 50-70% often indicates insufficient cache capacity or poor key design.

## See Also

- [web-http-caching.md](web-http-caching.md) — browser and HTTP proxy caching
- [distributed-replication.md](distributed-replication.md) — replication for cache replicas
- [system-design-distributed.md](system-design-distributed.md) — integrating cache into system design