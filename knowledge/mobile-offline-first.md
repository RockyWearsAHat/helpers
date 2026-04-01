# Mobile Offline-First — Local Databases, Sync Strategies, Conflict Resolution & Progressive Loading

## Overview

Offline-first architecture assumes the network is unreliable. App works fully offline using a local database, then syncs changes with the server when connected. Contrast to online-first (app requires network). Offline-first improves resilience, reduces latency (local reads/writes), and enables seamless transitions between network states (WiFi → cellular → offline).

## Local Databases

### SQLite

Embedded SQL database, de facto standard for native mobile.

**Android**: `androidx.room` provides type-safe wrapper.

```kotlin
@Entity
data class User(
  @PrimaryKey val id: String,
  val name: String,
  val email: String
)

@Dao
interface UserDao {
  @Insert suspend fun insert(user: User)
  @Query("SELECT * FROM user WHERE id = :id") suspend fun getUser(id: String): User?
}

@Database(entities = [User::class], version = 1)
abstract class AppDatabase : RoomDatabase() {
  abstract fun userDao(): UserDao
}

// In ViewModel or Repository
val user = userDao.getUser("123")
```

**Strengths**: ACID transactions, full-text search with FTS module, spatial queries with R-tree, JSON operations.

**Limitations**: Single-writer (locks other writers during transaction). On mobile, typically fine since usual pattern is 1 process.

**Size**: SQLite database files typical 10-100MB; can grow to GBs if not pruned.

### Room

Android-specific abstraction over SQLite. Provides:

- Type checking at compile time (detects column mismatches).
- Migration support (ALTER TABLE handling).
- Live data/Flow observation (auto-recompose on change).

```kotlin
// Migration from schema v1 to v2
val migration_1_2 = object : Migration(1, 2) {
  override fun migrate(db: SupportSQLiteDatabase) {
    db.execSQL("ALTER TABLE user ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0")
  }
}

Room.databaseBuilder(context, AppDatabase::class.java, "app.db")
  .addMigration(migration_1_2)
  .build()
```

### Core Data (iOS)

Apple's object-graph persistence framework. Stores objects; persists to SQLite or binary format.

```swift
@NSManaged var id: String
@NSManaged var name: String
@NSManaged var email: String

// Fetch
let request = NSFetchRequest<User>(entityName: "User")
request.predicate = NSPredicate(format: "id == %@", "123")
let users = try context.fetch(request)
```

**Strengths**: Object caching in memory (fetches return managed objects, changes auto-tracked).

**Weaknesses**: More complex migration than SQL migrations. Merging conflicts between multiple contexts is tricky.

### Realm

Cross-platform database (iOS, Android, web). Object-oriented, reactive.

```swift
// iOS
let user = realm.object(ofType: User.self, where: "id == '123'")
user.name = "Bob" // Auto-tracked change
```

**Advantages**: Reactive queries (auto-update collections when underlying data changes). Cross-platform code.

**Disadvantages**: Vendor lock-in. Schema changes simpler but less control than SQL migrations.

## Sync Strategies

### Queue-Based Sync

Client queues mutations (POST, PUT, DELETE) if offline. When online, flushes queue to server.

**Flow**:

```
[User Action] → [Check network]
  ├─ Online: Send to server immediately (background)
  └─ Offline: Queue locally, persist; return success to user
[When online again] → [Replay queue in order]
```

**Implementation**:

```kotlin
data class SyncQueue(
  val id: String,
  val operation: String, // "POST", "PUT", "DELETE"
  val resource: String, // "users", "posts"
  val resourceId: String?, // null for POST (no ID yet)
  val payload: String, // JSON
  val retries: Int = 0,
  val maxRetries: Int = 3
)

suspend fun updateUser(user: User) {
  if (isOnline()) {
    api.putUser(user) // direct
  } else {
    // Queue it
    db.syncQueueDao.insert(SyncQueue(
      operation = "PUT",
      resource = "users",
      resourceId = user.id,
      payload = Json.encodeToString(user)
    ))
  }
  userDao.insert(user) // local update
}

// When online, sync worker replays queue
class SyncWorker : CoroutineWorker() {
  override suspend fun doWork(): Result {
    val queued = db.syncQueueDao.getAll()
    for (item in queued) {
      try {
        when (item.operation) {
          "PUT" -> api.putUser(Json.decodeFromString(item.payload))
          "DELETE" -> api.deleteUser(item.resourceId!!)
          // Success: delete from queue
          db.syncQueueDao.delete(item)
        }
      } catch (e: HttpException) {
        if (item.retries < item.maxRetries) {
          db.syncQueueDao.updateRetries(item.id, item.retries + 1)
        } else {
          // Give up or alert user
          db.syncQueueDao.markFailed(item.id)
        }
      }
    }
    return Result.success()
  }
}
```

**Tradeoff**: Simple, but assumes operations idempotent and commutative. If user edits field A offline, server changes field B, then queue replays → offline change overwrites server's.

### Last-Write-Wins (LWW)

Server and client track timestamps. Last write (highest timestamp) wins on conflict.

**Example**:

- **User 1** (offline): edits name to "Bob" at 2:00pm local. Stores locally with server_updated_at=1:30pm (from last sync).
- **User 2** (online): edits same user's email to bob@example.com at 2:05pm server time.
- When User 1 comes online, syncs: local name edit (timestamp 2:00pm) vs server email edit (2:05pm). Server email wins, User 1's name change is lost.

**Implementation**:

```kotlin
data class User(
  val id: String,
  val name: String,
  val updated_at: Long, // milliseconds since epoch
  val server_updated_at: Long // last sync
)

// On sync conflict:
fun resolveConflict(local: User, server: User): User {
  // Merge field-by-field (not whole record)
  return User(
    id = local.id,
    name = if (local.updated_at > server.updated_at) local.name else server.name,
    updated_at = maxOf(local.updated_at, server.updated_at),
    server_updated_at = System.currentTimeMillis()
  )
}
```

**Pros**: Deterministic, no user interaction needed.

**Cons**: Lossy (data silently overwritten). Timestamp skew (if clocks are out of sync across devices).

### Operational Transform (OT)

Mathematical approach: transform local operations to account for concurrent server operations, avoiding conflicts.

**Principle**: If operations A and B commute semantically, T(A, B) and T(B, A) produce the same result.

**Example** (textual):

- User 1 inserts "x" at position 0 in "hello" (A).
- User 2 inserts "y" at position 5 in "hello" (B).
- Results: both users should see "xhellow" or "helloy". If A and B don't commute, conflict.

**Transformation**:

- Node A: Insert(0, "x").
- Node B: Insert(5, "y").
- When A learns of B, it doesn't re-execute B; instead, A transforms its own position: the insert was before position 5, so position doesn't change. Result on A: "xhellow"

**Implementation**: Complex. Libraries: Google Docs (proprietary), Quill (web). Rarely used on mobile due to complexity.

### CRDTs (Conflict-Free Replicated Data Types)

Formal mathematical structures guaranteeing convergence without central authority. Multiple replicas apply same operations independently, always converge.

**Concept**: Instead of tracking field values, track **all changes**. Combine all changes deterministically.

**Example: Counter**

```kotlin
// Simple counter can conflict (last write wins loses updates):
// Replica A: increment to 1, Replica B: increment to 1 → both see 1, not 2.

// CRDT Counter:
// Each replica owns a slot. A increments A[0], B increments B[0].
// Counter = sum of all slots.
// Merge: take max per slot
class Counter {
  val slots = mutableMapOf<String, Long>() // node_id → count
  
  fun increment(nodeId: String) {
    slots[nodeId] = (slots[nodeId] ?: 0) + 1
  }
  
  fun merge(other: Counter) {
    for ((id, count) in other.slots) {
      slots[id] = maxOf(slots[id] ?: 0, count)
    }
  }
  
  fun value() = slots.values.sum()
}

val a = Counter()
a.increment("A") // A.value = 1
val b = a.copy()
b.increment("B") // B.value = 1
a.merge(b) // A now has A["A"]=1, A["B"]=1, value=2
```

**More Complex CRDTs**:

- **LWW Register**: Field with timestamp, merge takes max timestamp.
- **OR-Set** (Observed-Remove Set): Set allowing concurrent add/remove without conflicts.
- **RGA** (Replicated Growable Array): Array with insert/delete ops that converge.

**Libraries**: Yjs (web), Automerge (multiplatform). Limited mobile adoption due to performance.

**Advantage**: Eventual consistency without central server. Works offline fully.

**Disadvantage**: Requires rethinking data model (not traditional ACID). More metadata per record.

## Reachability Detection

App must know when network changes (online ↔️ offline, WiFi ↔️ cellular).

**Android ConnectivityManager**:

```kotlin
val connectivityManager = context.getSystemService<ConnectivityManager>()
val activeNetwork = connectivityManager?.activeNetwork ?: return
val caps = connectivityManager?.getNetworkCapabilities(activeNetwork) ?: return
val isOnline = caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)

// Observe changes
connectivityManager.registerNetworkCallback(
  NetworkRequest.Builder()
    .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
    .build(),
  object : ConnectivityManager.NetworkCallback() {
    override fun onAvailable(network: Network) {
      Log.d(TAG, "Network available")
      triggerSync()
    }
    override fun onLost(network: Network) {
      Log.d(TAG, "Network lost")
    }
  }
)
```

**iOS Reachability** (NWPathMonitor):

```swift
import Network

let monitor = NWPathMonitor()
monitor.pathUpdateHandler = { path in
  if path.status == .satisfied {
    print("Network available")
    self.triggerSync()
  } else {
    print("Network unavailable")
  }
}
monitor.start(queue: DispatchQueue.main)
```

**Caveats**: Reachability ≠ connectivity. Device may have internet flag but network unreachable (firewall, auth portal). Apps should always handle network failures gracefully (timeouts, retries).

## Progressive Data Loading

Offline-first apps load content in stages:

1. **Display cached/local**: Instant (1-50ms). User sees data immediately.
2. **Fetch fresh from server**: Async. If available and newer, update UI.
3. **Handle stale/missing**: If local is old or missing, show placeholder until fetch completes.

**Pattern**:

```kotlin
val user = userRepository.getUser(id)

// Repository:
suspend fun getUser(id: String): User {
  // Step 1: Try cache
  val cached = db.userDao.getUser(id)
  if (cached != null && isRecent(cached)) {
    return cached
  }
  
  // Step 2: Fetch fresh (if online)
  try {
    val fresh = api.getUser(id) // May fail if offline
    db.userDao.insert(fresh)
    return fresh
  } catch (e: IOException) {
    // Offline; return cached even if stale
    return cached ?: throw UserNotFound()
  }
}
```

**UI Level**:

```kotlin
@Composable
fun UserDetail(id: String) {
  val viewModel: UserViewModel = viewModel()
  val user by viewModel.getUser(id).collectAsState(initial = null)
  
  when (user) {
    null -> {
      // Loading/error state
      CircularProgressIndicator()
    }
    else -> {
      // Display user (may be stale cached data initially, updates when fresh fetches)
      UserProfile(user!!)
    }
  }
}
```

## Pruning & Expiration

Local databases grow unbounded. Implement data expiration to manage size.

**Strategy**:

- **TTL (Time-To-Live)**: Delete records older than T days.
- **LRU (Least Recently Used)**: Keep only most recently accessed N records.
- **Quota-based**: Keep database ≤ 50MB, evict old records until size met.

```kotlin
suspend fun pruneOldData() {
  val cutoff = System.currentTimeMillis() - 30 * 24 * 60 * 60 * 1000 // 30 days
  db.userDao.deleteOlderThan(cutoff)
  db.postDao.deleteOlderThan(cutoff)
  
  // Compact database file
  db.compactDatabase()
}
```

**Scheduling**: Run pruning in low-impact time (background sync, on charging).

## Sync Conflict UI

When conflicts arise (LWW or explicit), present choice to user:

```kotlin
// Detect conflict during sync
if (local.updated_at > remote.updated_at) {
  // User likely wants their change, but inform them
  showConflictDialog(local, remote)
}

// User chooses "Keep mine" or "Accept server"
fun userChoseKeepLocal() {
  // Queue as update
  db.syncQueueDao.insert(SyncQueue(operation = "PUT", payload = Json.encode(local)))
}

fun userChoseAcceptServer() {
  db.userDao.update(remote)
  // Remove from queue if queued
}
```

## See Also

- [Progressive Web Applications](web-pwa.md) — offline-first on web with Service Workers
- [Service Workers](web-service-workers.md) — web offline caching strategy
- [Database Replication Patterns](database-replication-patterns.md) — server-side multi-replica sync
- [Mobile Development Patterns](mobile-development-patterns.md) — app lifecycle during network change
- [Performance: Caching Strategies](performance-caching-strategies.md) — cache invalidation patterns
- [Distributed: Consensus & Ordering](distributed-consensus-ordering.md) — why eventual consistency is hard