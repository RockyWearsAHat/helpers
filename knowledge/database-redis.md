# Redis

## Data Structures

### Strings

The most basic type. Can store text, numbers, or binary data up to 512MB.

```redis
SET key "value" EX 3600           -- set with 1-hour TTL
SET key "value" NX                -- set only if not exists (atomic)
SET key "value" XX                -- set only if exists
MSET k1 "v1" k2 "v2"             -- multi-set
MGET k1 k2                       -- multi-get
INCR counter                     -- atomic increment
INCRBY counter 5                 -- atomic add
INCRBYFLOAT price 1.50           -- float increment
APPEND key " more"               -- append to string
GETRANGE key 0 4                 -- substring
SETNX key "value"                -- set if not exists (prefer SET NX)
GETSET key "new"                 -- set and return old value
GETDEL key                       -- get and delete (6.2+)
```

### Lists

Doubly-linked lists. O(1) head/tail operations.

```redis
LPUSH queue "job1" "job2"         -- push to head
RPUSH queue "job3"                -- push to tail
LPOP queue                        -- pop from head
RPOP queue                        -- pop from tail
BRPOP queue 30                    -- blocking pop (30s timeout) — for work queues
BLPOP queue1 queue2 0             -- block on multiple lists
LRANGE queue 0 -1                 -- get all elements
LLEN queue                        -- length
LPOS queue "job2"                 -- find position (6.0.6+)
LMOVE src dst LEFT RIGHT          -- atomic move between lists (6.2+)
LTRIM queue 0 999                 -- keep only first 1000 (capped list)
```

### Sets

Unordered collections of unique strings.

```redis
SADD tags "go" "backend" "api"
SMEMBERS tags                     -- all members
SISMEMBER tags "go"               -- membership check O(1)
SCARD tags                        -- count
SRANDMEMBER tags 3                -- random sample
SPOP tags                         -- random remove
SINTER tags1 tags2                -- intersection
SUNION tags1 tags2                -- union
SDIFF tags1 tags2                 -- difference
SINTERCARD 2 tags1 tags2 LIMIT 5 -- count intersection (7.0+)
```

### Sorted Sets

Sets ordered by score. The core data structure for leaderboards, priority queues, rate limiters.

```redis
ZADD leaderboard 1500 "alice" 1200 "bob" 1800 "charlie"
ZRANGE leaderboard 0 -1 WITHSCORES         -- lowest first
ZREVRANGE leaderboard 0 2 WITHSCORES       -- top 3
ZRANK leaderboard "alice"                   -- rank (0-indexed)
ZSCORE leaderboard "alice"                  -- get score
ZINCRBY leaderboard 50 "alice"              -- increment score
ZRANGEBYSCORE leaderboard 1000 2000         -- score range
ZRANGEBYLEX leaderboard "[a" "[d"           -- lex range (same scores)
ZCOUNT leaderboard 1000 2000               -- count in score range
ZRANGESTORE dst src 0 9 BYSCORE REV        -- top 10 → new key (6.2+)
ZPOPMIN leaderboard                         -- remove lowest
BZPOPMAX queue 30                          -- blocking pop max (priority queue)
```

### Hashes

Field-value maps. Natural fit for objects.

```redis
HSET user:42 name "Alice" email "a@b.com" age 30
HGET user:42 name
HMGET user:42 name email
HGETALL user:42                   -- all fields and values
HINCRBY user:42 age 1             -- increment numeric field
HDEL user:42 temp_field
HEXISTS user:42 email
HLEN user:42
HSCAN user:42 0 MATCH "pref_*"   -- iterate matching fields
HRANDFIELD user:42 2 WITHVALUES  -- random fields (6.2+)
```

### Streams

Append-only log with consumer groups. Redis's answer to Kafka/message queues.

```redis
-- Produce
XADD events * type "click" url "/products" user_id "42"

-- Read (simple)
XRANGE events - + COUNT 10
XREAD COUNT 5 BLOCK 2000 STREAMS events $   -- tail -f equivalent

-- Consumer groups
XGROUP CREATE events mygroup $ MKSTREAM
XREADGROUP GROUP mygroup consumer1 COUNT 10 BLOCK 2000 STREAMS events >
XACK events mygroup 1234567890-0             -- acknowledge processing
XPENDING events mygroup - + 10               -- check unacknowledged

-- Claim stuck messages
XAUTOCLAIM events mygroup consumer2 60000 0-0  -- claim messages idle >60s
```

### Other Types

| Type         | Module          | Purpose                                            |
| ------------ | --------------- | -------------------------------------------------- |
| HyperLogLog  | Core            | Probabilistic cardinality (0.81% error, 12KB max)  |
| Bitmap       | Core            | Bit arrays for flags/counting                      |
| Bitfield     | Core            | Packed integers in strings                         |
| Geospatial   | Core            | Lat/lng with radius queries (backed by sorted set) |
| JSON         | RedisJSON       | Full JSON document support with JSONPath           |
| Bloom filter | RedisBloom      | Probabilistic membership testing                   |
| Time series  | RedisTimeSeries | Downsampling, aggregation rules                    |

```redis
-- HyperLogLog (unique counting)
PFADD daily:visitors:2024-01-15 "user1" "user2" "user3"
PFCOUNT daily:visitors:2024-01-15           -- approximate unique count
PFMERGE weekly:visitors daily:visitors:*    -- merge counters

-- Geospatial
GEOADD stores -73.935242 40.730610 "nyc" -118.243685 34.052234 "la"
GEOSEARCH stores FROMMEMBER "nyc" BYRADIUS 100 mi ASC COUNT 5
GEODIST stores "nyc" "la" mi
```

## Persistence

### RDB (Snapshotting)

Point-in-time snapshots written to disk. Fast restart, but data loss between snapshots.

```conf
save 900 1        # snapshot if ≥1 key changed in 900s
save 300 10       # snapshot if ≥10 keys changed in 300s
save 60 10000     # snapshot if ≥10000 keys changed in 60s
dbfilename dump.rdb
rdbcompression yes
```

### AOF (Append Only File)

Logs every write operation. Better durability, larger files.

```conf
appendonly yes
appendfilename "appendonly.aof"
appendfsync everysec    # good balance (default)
# appendfsync always    # safest, slowest
# appendfsync no        # OS decides when to flush

# AOF rewrite (compaction)
auto-aof-rewrite-percentage 100
auto-aof-rewrite-min-size 64mb
```

### Recommended: RDB + AOF

Enable both. Redis uses AOF for recovery (more complete), RDB for backups and faster restarts.

Since Redis 7.0, AOF uses multi-part files (`appendonly.aof.1.base.rdb` + incremental files), making rewrite safer.

## Replication

```conf
# Replica config
replicaof 192.168.1.100 6379
replica-read-only yes
```

- Asynchronous by default — replicas may lag
- `WAIT numreplicas timeout` — synchronous wait for replication
- Replication is non-blocking on the primary for both full resync and partial resync
- Diskless replication: primary streams RDB directly to replica socket

## Sentinel (High Availability)

Monitors primary, detects failure, promotes a replica, notifies clients.

```conf
# sentinel.conf
sentinel monitor mymaster 192.168.1.100 6379 2  # quorum of 2
sentinel down-after-milliseconds mymaster 5000
sentinel failover-timeout mymaster 60000
sentinel parallel-syncs mymaster 1
```

Deploy at least 3 Sentinel instances. Clients connect to Sentinel to discover the current primary.

## Cluster (Data Sharding)

Automatically partitions data across multiple Redis nodes using **16384 hash slots**.

```
Key → CRC16(key) % 16384 → slot → node
```

- Each node handles a subset of slots
- Minimum: 3 primaries (6 nodes with replicas)
- Automatic failover within the cluster
- Multi-key operations require all keys on the same node → use **hash tags**: `{user:42}:profile` and `{user:42}:sessions` go to the same slot

```redis
CLUSTER INFO
CLUSTER NODES
CLUSTER KEYSLOT mykey
-- Resharding: redis-cli --cluster reshard host:port
```

## Pub/Sub

Fire-and-forget messaging. No persistence — if nobody is listening, the message is lost.

```redis
-- Subscriber
SUBSCRIBE channel1 channel2
PSUBSCRIBE news.*              -- pattern subscribe

-- Publisher
PUBLISH channel1 "hello"
```

For durable messaging, use **Streams** instead.

## Lua Scripting

Atomic execution — no other command runs during a script. Scripts run on a single thread.

```redis
-- Atomic compare-and-set
EVAL "if redis.call('get', KEYS[1]) == ARGV[1] then return redis.call('set', KEYS[1], ARGV[2]) else return 0 end" 1 mykey oldval newval

-- Load script for repeated use
SCRIPT LOAD "return redis.call('get', KEYS[1])"
-- Returns SHA1 hash
EVALSHA <sha1> 1 mykey
```

Redis 7.0+ supports **Functions** as an improved replacement:

```redis
FUNCTION LOAD "#!lua name=mylib\nredis.register_function('myfunc', function(keys, args) return redis.call('get', keys[1]) end)"
FCALL myfunc 1 mykey
```

## Transactions

```redis
MULTI
SET account:1:balance 950
SET account:2:balance 1050
EXEC         -- atomic execution
-- or DISCARD to abort
```

`WATCH` for optimistic locking:

```redis
WATCH account:1:balance
val = GET account:1:balance
MULTI
SET account:1:balance (val - 50)
EXEC  -- fails if account:1:balance changed since WATCH
```

Transactions don't roll back on individual command failures — they execute all or none based on `WATCH`.

## Eviction Policies

| Policy            | Behavior                                          |
| ----------------- | ------------------------------------------------- |
| `noeviction`      | Return error on write when memory full (default)  |
| `allkeys-lru`     | Evict least recently used (recommended for cache) |
| `allkeys-lfu`     | Evict least frequently used (Redis 4.0+)          |
| `volatile-lru`    | LRU among keys with TTL set                       |
| `volatile-lfu`    | LFU among keys with TTL set                       |
| `allkeys-random`  | Random eviction                                   |
| `volatile-random` | Random among keys with TTL                        |
| `volatile-ttl`    | Evict keys with shortest TTL                      |

```conf
maxmemory 4gb
maxmemory-policy allkeys-lfu
maxmemory-samples 10            # LRU/LFU approximation sample size
```

## Pipelining

Batch commands to reduce round trips. Not atomic (use MULTI/EXEC or Lua for atomicity).

```python
# Python example (redis-py)
pipe = r.pipeline(transaction=False)
for i in range(1000):
    pipe.set(f"key:{i}", f"value:{i}")
pipe.execute()  # one round trip for 1000 commands
```

Pipelining can improve throughput by 5-10x for bulk operations.

## Common Patterns

### Rate Limiting (Sliding Window)

```redis
-- Sorted set sliding window
MULTI
ZREMRANGEBYSCORE ratelimit:user:42 0 (now - window_ms)
ZADD ratelimit:user:42 now now
ZCARD ratelimit:user:42
EXPIRE ratelimit:user:42 window_seconds
EXEC
-- Check ZCARD result against limit
```

### Distributed Lock (Redlock)

```redis
-- Acquire (SET NX with TTL)
SET lock:resource <unique_id> NX PX 30000

-- Release (only if we hold it — use Lua)
EVAL "if redis.call('get',KEYS[1]) == ARGV[1] then return redis.call('del',KEYS[1]) else return 0 end" 1 lock:resource <unique_id>
```

For production: use the Redlock algorithm across N independent Redis instances (majority quorum).

### Leaderboard

```redis
ZADD game:leaderboard 1500 "player:42"
ZINCRBY game:leaderboard 25 "player:42"       -- add points
ZREVRANK game:leaderboard "player:42"          -- global rank
ZREVRANGE game:leaderboard 0 9 WITHSCORES     -- top 10
ZCOUNT game:leaderboard 1000 +inf             -- players above threshold
```

### Cache-Aside Pattern

```
read(key):
  value = redis.get(key)
  if value is null:
    value = db.query(...)
    redis.setex(key, TTL, value)
  return value

write(key, data):
  db.update(data)
  redis.del(key)    -- invalidate (don't update cache — avoids race conditions)
```

### Session Store

```redis
HSET session:<token> user_id 42 role "admin" created_at 1704067200
EXPIRE session:<token> 86400  -- 24 hours
```

Use hashes for sessions — update individual fields without read-modify-write.
