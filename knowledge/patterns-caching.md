# Caching Patterns

Caching stores frequently accessed data in a faster-to-access layer (memory) to avoid repeated expensive operations (database queries, API calls, computations). Different patterns describe where and how caches fit in the request path, each with different trade-offs around consistency, complexity, and performance.

## Cache-Aside (Lazy Loading)

The application is responsible for checking the cache, loading from the source on miss, and populating the cache.

```
Application logic:
  1. Check if key exists in cache
  2. If HIT: return cached value
  3. If MISS:
     a. Load from source (database, API)
     b. Store in cache
     c. Return value
```

### Implementation

```python
def get_user(user_id):
    # Check cache
    cached = cache.get(f"user:{user_id}")
    if cached:
        return cached
    
    # Cache miss: load from DB
    user = db.query(f"SELECT * FROM users WHERE id = {user_id}")
    
    # Populate cache with TTL
    cache.set(f"user:{user_id}", user, ttl=3600)
    
    return user
```

### Pros

- Simple to implement; no special infrastructure
- Only requested data is cached (no waste on unpopular items)
- Stale cache doesn't break correctness (miss forces reload)

### Cons

- **Cache-aside miss penalty:** First request for an item waits for database query
- **Stampede risk:** Multiple concurrent misses for the same key all query the database (see "Cache Stampede" below)
- Application must handle cache failure gracefully (cache is optional, not critical path)

### When to Use

- Distributed systems with high cardinality (millions of keys)
- Read-heavy workloads with long tail (few hot items, many cold)
- When consistency isn't critical

## Read-Through

The cache library itself handles the fetch-on-miss. Application doesn't know about the source database.

```
Application → Cache
  Cache: Is key present? 
    YES → Return value
    NO → Fetch from data loader function → Store in cache → Return to app
```

### Implementation

```java
// Java: Guava LoadingCache
LoadingCache<Integer, User> userCache = CacheBuilder.newBuilder()
    .expireAfterWrite(1, TimeUnit.HOURS)
    .build(new CacheLoader<Integer, User>() {
        @Override
        public User load(Integer userId) {
            return db.getUserById(userId);  // Automatic on miss
        }
    });

User user = userCache.get(123);  // Automatic load if missing
```

### Pros

- Clean separation: application is abstracted from cache logic
- Automatic consistency: cache expiry and reload are transparent
- Library can optimize miss handling (deduplication, synchronization)

### Cons

- Requires a cache library that supports loaders
- Less flexible than cache-aside (loader logic is fixed)
- Loader failure propagates to application

### When to Use

- Systems with a well-defined data access layer
- When you want to hide caching complexity from business logic

## Write-Through

Writes go to the cache first, then to the database synchronously. Cache and database stay consistent.

```
Application: PUT user
  |
  v
Cache: Store value
  |
  v (synchronous)
Database: Store value
  |
  v (success)
Return to application
```

### Implementation

```python
def update_user(user_id, user_data):
    # Write to cache first
    cache.set(f"user:{user_id}", user_data)
    
    # Then to database (synchronous)
    db.update(f"UPDATE users SET ... WHERE id = {user_id}", user_data)
    
    return user_data
```

### Pros

- Cache and database always consistent
- Safe: no data loss if cache fails (database has it)
- Predictable: writes are synchronous

### Cons

- **Write latency:** Every write waits for database. No advantage if database is the bottleneck.
- **Write penalty:** All writes incur cache overhead, even if the data is never read
- Cache failure blocks writes (unless you add fallback logic)

### When to Use

- Critical data where consistency is paramount (financial transactions, inventory)
- When write latency is not a bottleneck (database is already fast)

## Write-Behind (Write-Back)

Writes go to the cache only. A separate process asynchronously flushes the cache to the database. Fast writes at the cost of eventual consistency.

```
Application: PUT user
  |
  v (fast)
Cache: Store value + mark dirty
  |
  v (background job, async)
Write-behind queue
  |
  v (batched, delayed)
Database: Flush dirty entries
```

### Implementation

```python
def update_user(user_id, user_data):
    # Write to cache only (fast)
    cache.set(f"user:{user_id}", user_data, mark_dirty=True)
    return user_data

# Background job
def flush_cache():
    dirty_entries = cache.get_dirty_entries()
    db.batch_update(dirty_entries)
    cache.mark_clean(dirty_entries)
```

### Pros

- Very fast writes (memory speed, no database wait)
- Batches database writes (more efficient than individual inserts)
- Decouples cache from database latency

### Cons

- **Data loss risk:** If cache crashes before flushing, recent writes are lost
- **Eventual consistency:** Database momentarily behind the cache
- Complexity: need a reliable write-behind process, queue management, conflict resolution

### Durability Pattern: Write-Through + Cache

Mitigate data loss with the outbox pattern (see patterns-event-driven):

```sql
BEGIN TRANSACTION
  UPDATE cache SET ... ;  -- OR just in-memory
  INSERT INTO outbox (operation) VALUES (...);
COMMIT;

-- Background process
SELECT * FROM outbox WHERE flushed = false;
  -- Execute operations on database
  UPDATE outbox SET flushed = true;
```

Ensures cache and outbox are atomically updated. Database catches up eventually.

### When to Use

- High-volume writes where latency is critical (IoT sensors, click tracking)
- Non-critical data (analytics, denormalized reads)
- When you can tolerate temporary data loss

## Cache Invalidation Strategies

The hardest problem in caching is knowing when cached data is stale.

### TTL (Time-to-Live)

Cache entries expire after a fixed duration.

```python
cache.set(key, value, ttl=3600)  # Expires after 1 hour
```

**Trade-off:**
- Short TTL (1min): frequent cache misses, but data is fresher
- Long TTL (1hr): fewer misses, but stale data longer
- **No perfect TTL.** Choose based on business tolerance for staleness.

### Event-Driven Invalidation

When data changes, explicitly remove or update the cache entry.

```python
def update_user(user_id, new_data):
    db.update(new_data, user_id)
    
    # Invalidate cache
    cache.delete(f"user:{user_id}")
    
    # Also invalidate related caches
    cache.delete(f"user_posts:{user_id}")
    cache.delete(f"user_friends:{user_id}")
```

**Challenge:** Cascading invalidation. Updating a user invalidates user posts, posts by friends, friend counts, etc. Easy to miss dependencies.

### Pattern: Versioned Keys

Instead of deleting keys, increment a version and point to the new one:

```python
def update_user(user_id, new_data):
    db.update(new_data, user_id)
    
    # Increment version
    current_version = cache.get(f"user_version:{user_id}") or 0
    new_version = current_version + 1
    
    cache.set(f"user:{user_id}:v{new_version}", new_data)
    cache.set(f"user_version:{user_id}", new_version)
    
    # Old cached references still work; they just get old data
```

Avoids thunder-herd on deletion (all clients switch to new version gradually).

### Lazy Invalidation

Don't delete the cache. Instead, mark it stale and re-fetch if accessed:

```python
def update_user(user_id, new_data):
    db.update(new_data, user_id)
    cache.invalidate_version(f"user:{user_id}")

def get_user(user_id):
    entry = cache.get(f"user:{user_id}")
    if entry and entry.is_valid():
        return entry.value
    else:
        return fetch_fresh(user_id)
```

Saves work if nobody accesses the stale data.

## Cache Stampede Prevention

When a popular cache entry expires, many concurrent requests might simultaneously fetch from the database, overwhelming it.

```
t=0: Cache miss on frequently-accessed key
t=1: Client A queries database for value
t=1: Client B queries database for same key (same miss)
t=1: Client C queries database for same key (no cache yet)
     ...10 clients all hammer the database for one value
```

### Solution: Probabilistic Early Expiry

Expire the cache entry early (before true TTL) with a low probability. The first client to see "expired" refetches; others still get cached value while it's being refreshed.

```python
import random

def get_with_early_expiry(key, fetch_fn, ttl=3600):
    entry = cache.get(key)
    
    if entry is None:
        # True miss, fetch
        value = fetch_fn()
        cache.set(key, value, ttl=ttl)
        return value
    
    # Probabilistic expiry: 1% chance to refresh early if > 90% of TTL elapsed
    age = time.time() - entry.created_at
    if age > 0.9 * ttl and random.random() < 0.01:
        # Background refresh (don't wait for it)
        pool.submit(lambda: cache.set(key, fetch_fn(), ttl=ttl))
    
    return entry.value
```

Only 1% of requests trigger a refresh, spread across time. Smoother load than a thundering herd.

### Solution: Locking

First client to miss acquires a lock, fetches, stores. Other clients wait for the lock to release, then use the refreshed value.

```python
lock_key = f"lock:{key}"

if not cache.get(key):
    if cache.acquire_lock(lock_key, timeout=5):
        try:
            value = fetch_fn()
            cache.set(key, value, ttl=ttl)
        finally:
            cache.release_lock(lock_key)
    else:
        # Couldn't get lock, wait for someone else to refresh
        time.sleep(0.1)
        return cache.get(key) or fetch_fn()
```

Prevents duplicate work but can become a bottleneck if the lock is slow.

## Distributed Caching

Single-machine caches (Memcached on localhost) don't scale. Distributed caches partition data across multiple nodes.

### Redis

In-memory key-value store. O(1) reads/writes, durable via AOF (append-only file) or RDB snapshots.

```python
import redis

cache = redis.Redis(host='localhost', port=6379)
cache.set('user:123', json.dumps(user), ex=3600)  # ex = expiration
user = json.loads(cache.get('user:123'))
```

- **Pros:** Fast, supports data structures (sets, lists, hashes), Lua scripting, pub/sub
- **Cons:** Single-threaded (throughput bounded by CPU), all data must fit in memory

### Memcached

Simple key-value store. No persistence, no advanced data types.

```python
from pymemcache import PooledClient

cache = PooledClient('localhost:11211')
cache.set(b'user:123', b'alice', expire=3600)
value = cache.get(b'user:123')
```

- **Pros:** Very fast, simple, scales well
- **Cons:** No data structures, no durability, cluster management is manual

### Consistency in Distributed Caches

**Multi-write problem:** If you write to cache node A and read from node B (before replication), you get a stale value.

Solutions:
- **Replica-aware clients:** Client writes to primary, reads from replicas after ensuring replication.
- **Write-through:** Bypass distributed cache for writes; write to DB first, then invalidate cache.
- **Read-your-own-writes:** Client remembers what it wrote; applies locally if reading from cache too soon.

## Multi-Layer Caching

CPU cache → L1/L2/L3 → RAM → SSD → Disk → Network. Each layer is faster but smaller.

```
Request
  |
  ├─ Local in-process cache (HashMap) — instant, no serialization
  |    MISS
  |
  ├─ Distributed cache (Redis) — network round-trip, serialized
  |    MISS
  |
  └─ Database — slow, SQL query
```

### In-Process Cache

```java
LoadingCache<Integer, User> local = CacheBuilder.newBuilder()
    .expireAfterWrite(5, TimeUnit.MINUTES)
    .maximumSize(1000)
    .build(key -> redis.get(key));  // Loader fetches from Redis
```

Faster than Redis (no network), but memory-constrained and doesn't scale across processes.

### Layered Strategy

Store hot data in-process (user profile), warm data in Redis (user posts), cold data only in database.

**Invalidation challenge:** When user is updated, invalidate in-process cache on all servers. Use pub/sub or explicit cache clearing.

## Metrics and Visibility

Track cache behavior to understand if it's helping:

- **Hit rate:** (hits) / (hits + misses). Target: 70-90%+ for well-tuned caches
- **Eviction rate:** Policies like LRU (Least Recently Used) evict old entries when cache is full
- **TTL distribution:** Are entries expiring naturally or being invalidated?
- **Latency:** p50, p95, p99 of cache lookups

Low hit rate indicates:
- TTL too short (entries expiring quickly)
- Cache too small (entries being evicted)
- Access pattern has high cardinality (many unique keys)

## See Also

- **web-http-caching** — Browser and CDN caching (different scope)
- **database-redis** — Redis internals and operations
- **patterns-event-driven** — Using event streams to drive cache invalidation
- **performance-system-design** — Caching strategy in large-scale systems