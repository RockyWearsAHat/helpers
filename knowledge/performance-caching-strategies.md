# Performance: Caching Strategies — HTTP, CDN, Application, and Database Layers

Caching is the primary technique for reducing latency and database load. Different cache layers (HTTP, CDN, application, database, ORM) each serve a purpose and require different invalidation strategies.

## HTTP Caching: Browser and Intermediary Cache Control

HTTP caching is the foundational layer. When properly configured, it eliminates round trips entirely, reducing latency from hundreds of milliseconds to near-zero.

### Cache-Control Directives

**Cache-Control** header tells clients and intermediaries how long to store and reuse a response:

```
Cache-Control: public, max-age=3600, s-maxage=86400, stale-while-revalidate=604800
```

| Directive | Effect |
| --- | --- |
| `public` | Cacheable by any cache (browser, CDN, proxy) |
| `private` | Browser-only; intermediaries must not cache |
| `no-cache` | Don't serve from cache without revalidation (always check with server) |
| `no-store` | Don't cache at all (sensitive data) |
| `max-age=N` | Stick around for N seconds |
| `s-maxage=N` | Shared cache (CDN) override for max-age |
| `stale-while-revalidate=N` | After expiry, serve stale copy for N seconds while revalidating in background |
| `stale-if-error=N` | If origin is unreachable, serve stale for N seconds |
| `immutable` | Response will never change; safe to cache indefinitely (used for fingerprinted assets like app.abc123.js) |

**Common patterns:**
- **Static assets**: `Cache-Control: public, max-age=31536000, immutable` (1 year, paired with fingerprinting)
- **HTML**: `Cache-Control: no-cache` (always revalidate, but can serve from cache during outage)
- **API**: `Cache-Control: private, max-age=60` (user-specific, short duration)
- **User data**: `Cache-Control: no-store` (never cache)

### Validation Headers

When cache expires, revalidation checks if the origin has a newer version:

**ETag** (Entity Tag): Opaque hash of the response body. If unchanged, server responds with `304 Not Modified` (no body transmitted).

```
Response: ETag: "33a64df551425fcc55e4d42a148795d9f25f89d4"
Request:  If-None-Match: "33a64df551425fcc55e4d42a148795d9f25f89d4"
Server:   304 Not Modified (cache remains valid)
```

**Last-Modified**: Timestamp. If no changes since this date, server responds with `304`.

```
Response: Last-Modified: Wed, 21 Oct 2025 07:28:00 GMT
Request:  If-Modified-Since: Wed, 21 Oct 2025 07:28:00 GMT
Server:   304 Not Modified
```

Weak ETags (`W/"..."`) indicate that the response is equivalent but not byte-for-byte identical (useful for gzip variance). Strong ETags require exact matches.

### Vary Header

**Vary** signals that the cache key includes more than just the URL. Common uses:

```
Vary: Accept-Encoding, User-Agent, Authorization
```

This tells intermediaries: "Store different versions of this response depending on the Accept-Encoding (`gzip` vs plain), User-Agent (mobile vs desktop), and Authorization (authenticated users get different content than anonymous)." Without Vary, an intermediary might serve a gzipped response to a client that doesn't support gzip, causing corruption.

## CDN Caching: Edge and Origin Shield

Content Delivery Networks cache content at geographically distributed edge servers, reducing round-trip time from the user to the server.

### Edge Caching

When a user requests a URL:
1. Edge server checks its local cache
2. If hit, serve immediately (reduces latency from ~100-500ms to ~10-50ms)
3. If miss, fetch from origin (or next layer)
4. Cache the response for future requests

**Cache key**: By default, the full URL. But CDNs allow customization:
- **Query string handling**: `?utm=123` vs `?utm=456` — should they be the same cache entry? (usually yes; strip UTM params from cache key)
- **Header-based variation**: Serve different cached versions based on request headers (via `Vary` or CDN-specific rules)
- **Geographic keys**: Cache different content per region

### Origin Shield

An **origin shield** is an extra cache layer between edge and origin. Instead of 100 edge servers each missing cache and hammering the origin, they share a single shield cache. Reduces origin load dramatically during cache misses.

**Tradeoff**: adds one extra hop (edge → shield → origin) on shields miss, but shields have high hit rates (90%+), so it's net positive.

### Cache Purge Strategies

**TTL-based expiry**: Cache expires automatically after max-age. Simple but inactive.

**Explicit purge**: Invalidate specific URLs when content changes. Most CDNs support `PURGE` HTTP method or admin APIs. Challenges:
- Dependency chains: purging a user profile might need to purge related comment pages
- Scale: billions of URLs can't all be purged instantly
- Backend latency: purge requests can overwhelm the origin API

**Surrogate Keys**: Tag responses with logical identifiers (e.g., tag all URLs related to user 123 with `Surrogate-Key: user-123-profile`). Purge by tag instead of URL: `Purge-Cache-By-Surrogate-Key: user-123-profile` invalidates all tagged URLs instantly. Recommended for dynamic content.

## Application Caching: In-Memory Layers

Application caches (Redis, Memcached) sit in front of the database and reduce query load.

### Pattern: Cache-Aside

Application checks cache; on miss, loads from DB and populates cache. Simple but requires cache logic in business code.

```python
def get_user(user_id):
    key = f"user:{user_id}"
    cached = cache.get(key)
    if cached:
        return json.loads(cached)
    
    user = db.query(f"SELECT * FROM users WHERE id = {user_id}")
    cache.set(key, json.dumps(user), ttl=3600)
    return user
```

**Pros:** Only cache what's requested. **Cons:** first request is slow; multiple concurrent misses can cascade to DB ("thundering herd").

### Pattern: Read-Through

Cache library handles the fetch-on-miss transparently. Application only knows about the cache.

```java
cache.get(userId, (id) -> db.getUserById(id));  // Sync or async loading
```

**Pros:** clean separation; library can optimize misses (batching, deduplication). **Cons:** less flexible.

### Pattern: Write-Through / Write-Behind

**Write-through**: Write to cache AND database synchronously. Ensures cache is always consistent but adds write latency.

```python
def update_user(user_id, data):
    cache.set(f"user:{user_id}", data)
    db.update(f"UPDATE users SET ... WHERE id = {user_id}")
```

**Write-behind** (write-through async): Write to cache immediately (fast), queue DB write for later. Risk: if cache fails before DB is written, data is lost. Used when consistency is secondary to speed (analytics dashboards, metrics).

### Cache Warming

**Proactive cache population** before requests arrive. Strategies:
- On startup: bulk-load hot items into cache
- On data change: when DB updates, proactively refresh related cache entries
- Off-peak: load forecasted hot data during low-traffic periods

Warming prevents "cold cache" performance cliffs after restarts or failures.

### Memcached vs Redis

**Memcached**: Simple, ultra-fast. Stores only strings. No persistence. Extreme horizontal scaling (add more nodes, transparent sharding). Use when you need a dumb, super-fast drop-in cache.

**Redis**: Richer data structures (sets, hashes, streams, sorted sets). Built-in persistence (RDB, AOF). Single-threaded but single-threaded simplifies consistency. Supports Lua scripting for transactions. Use when you need consistency guarantees or complex operations (e.g., rate limiting with sorted sets, leaderboards).

## Database Query Caching and ORM Caching

### Query Result Caching

Some databases cache query results, but most rely on the application layer. Why? Because:
- Query plans can change (optimizer behavior is data-dependent)
- Cache invalidation is hard (any related data change must invalidate the cached result)
- Most queries are rarely repeated identically

**Exception:** Read-heavy OLAP warehouses (BigQuery, Redshift) cache results aggressively.

### ORM Caching

ORMs (SQLAlchemy, Hibernate, Sequelize) can cache objects to avoid repeated queries for the same ID:

```python
# SQLAlchemy
user = session.query(User).get(123)  # Query
user = session.query(User).get(123)  # Cache hit, no query
session.expunge_all()                # Clear session cache
```

Session caches are per-connection and expire when the session ends. Distributed caches require explicit integration (e.g., Hibernate's distributed caching with Ehcache + Infinispan).

**Pitfall:** ORM caches can hide N+1 query problems. A query that loads 1000 users + their profiles might look instant due to session cache, but creates 1001 queries. Always profile.

## Cache Hierarchies and Tiering

Modern systems use **multi-level caching**:

```
Browser Cache (HTTP Cache-Control)
       ↓
CDN Edge Cache
       ↓
Origin Shield Cache
       ↓
Application Cache (Redis)
       ↓
Database Cache (buffer pool)
       ↓
Disk
```

Each layer has different TTL, granularity, and invalidation strategy:
- **Browser**: Long TTL; managed via HTTP headers
- **CDN**: Managed via max-age + purge rules
- **Application**: Shorter TTL; invalidated on data change
- **Database**: Least controllable; managed by query optimizer

Misaligning TTLs causes confusion: change a user's profile, but it remains stale in CDN for 1 hour while application cache expires after 5 minutes.

## Cache Invalidation Patterns

**Time-based (TTL)**: Simplest; cache expires after N seconds. Downside: stale data visible for up to N seconds.

**Event-based**: On data change, invalidate related cache entries. Requires discipline; expensive if dependency chains are complex.

**Hybrid**: Short TTL + event-based purge. If event-based purge fails, TTL ensures eventual consistency.

**Cache-busting via fingerprinting**: For static assets, include a hash in the URL (app.abc123.js). New version has different URL, so no invalidation needed; old and new coexist in cache.

## See Also

- patterns-caching.md — cache-aside, read-through, write-through patterns
- distributed-caching.md — consistency models, cache coherence
- web-http-caching.md — HTTP caching deep dive
- infrastructure-cdn.md — CDN architecture and edge computing