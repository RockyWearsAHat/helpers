# Mobile Performance — App Startup, Memory Management, UI Thread, Image Loading & Optimization

## Overview

Mobile performance is constrained by hardware (CPU, RAM, battery), network latency, and OS preemption (calls, notifications). Performance impacts user retention: 50% of users abandon apps after 3-second load. Key metrics: app startup time, memory footprint, UI responsiveness (frame rate), network efficiency, battery drain.

## App Startup Performance

App startup consists of sequential phases. Each has distinct bottlenecks and optimization strategies.

### Cold Start

Process is not in memory. OS launches the app.

**Timeline**:

1. **(~100ms)** `Application.onCreate()` or SwiftUI app initializer runs
2. **(~200ms)** First Activity/ViewController becomes visible
3. **(~500ms)** App content rendered and interactive

**Total**: 300-1000ms typical, user-perceptible as "freeze."

**Bottlenecks**:

- Static initializers running; large dependency graph
- Database migrations or schema validation on startup
- Expensive computations (JSON parsing, image decoding) on first Activity creation
- Disk I/O (reading preferences, caches)

**Optimizations**:

- **Lazy initialization**: Defer logging framework, analytics, third-party SDKs until after first screen renders.
- **Async initialization**: Move heavy work off the main thread (but thread priority should be low to not block user interaction).
- **Startup profiling**: Android Profiler, Xcode Instruments can measure each phase.
- **Reduce dependencies**: Each transitive dependency adds initialization cost.

```kotlin
// Bad: initializes on app startup
object LoggingSetup {
  init {
    Logger.initialize() // 50ms overhead
  }
}

// Good: initialize lazily
package com.app.logging

object LoggingSetup {
  private var initialized = false
  fun ensure() {
    if (!initialized) {
      Logger.initialize()
      initialized = true
    }
  }
}

// Call LoggingSetup.ensure() after first screen renders
```

**Framework Effects**:

- Android: Heavy Jetpack dependency resolution, warm-up JIT compilation (improved in Android 14 with cloud profiles).
- iOS: Swift compilation overhead; app thinning (code slicing) reduces binary size on device.

### Warm Start

Process exists in memory (backgrounded). OS brings it to foreground.

**Timeline**: 100-300ms, usually imperceptible.

**Bottleneck**: Restoring UI state. If ViewModel fetches data on every `onResume()`, creates lag.

**Optimization**: Cache on memory, skip re-fetch if data is fresh.

### Hot Start

App is in foreground. User taps and app responds.

**Timeline**: <16ms for 60fps (frame-bound).

**Key**: No janked frame. See "UI Thread Performance" below.

## Memory Management

Mobile devices have fixed RAM (2-8GB typical). Running out triggers OOM (out-of-memory) crashes and app termination by OS.

### Memory Model Comparison

#### Automatic Reference Counting (ARC) — iOS/Swift

iOS uses ARC: when all references to an object are released, object is deallocated. **Deterministic**: deallocation is immediate, not delayed.

```swift
class User {
  var name: String
}

func example() {
  let user = User(name: "Alice")
  // Reference count = 1
} // Reference count drops to 0, user deallocated immediately
```

**Pitfall**: Retain cycles. Strong reference A holds B, B holds A → neither is deallocated.

```swift
class ViewController: UIViewController {
  var viewModel: ViewModel?
  
  init() {
    super.init(nibName: nil, bundle: nil)
    // viewModel holds strong reference back to self → retain cycle
    viewModel = ViewModel(view: self)
  }
}
```

**Solution**: Mark back-reference weak.

```swift
class ViewModel {
  weak var view: ViewController?
}
```

#### Garbage Collection — Android

Android Dalvik and ART use GC: app doesn't deallocate. GC scans heap periodically, identifies unreachable objects, deallocates.

**Advantage**: No retain cycles. Manual cycles impossible.

**Drawback**: GC pauses freeze the app (typically 50-500ms). Unpredictable GC timing can cause frame drops.

**GC Types**:

- **Generational**: Most objects die young (temporary strings, collections). YGC (young) is fast; full GC is slow.
- **Incremental**: GC does work in small chunks to bound pause time.

**Android GC Tuning** (limited by developers):

- Reduce allocations per frame (reuse objects).
- Avoid large objects (trigger full GC).

```kotlin
// Bad: allocates new Paint on every draw
override fun onDraw(canvas: Canvas) {
  val paint = Paint()
  paint.color = Color.RED
  canvas.drawRect(0f, 0f, 100f, 100f, paint)
}

// Good: reuse Paint
val paint = Paint().also { it.color = Color.RED }
override fun onDraw(canvas: Canvas) {
  canvas.drawRect(0f, 0f, 100f, 100f, paint)
}
```

### Memory Footprint

**Android**: Typical app use 50-200MB resident. OS can kill background apps if memory exceeds threshold.

**iOS**: OS more permissive but still can kill backgrounded apps. Typical footprint: 30-150MB.

**Best Practices**:

- **Image caching**: Images are large (2-10MB for 4K photos). Use LRU caches with size limits.
- **WebView memory**: Each WebView is ~10MB overhead; reuse instances if possible.
- **Leak detection**: Android: LeakCanary (runtime detection). iOS: Xcode Memory Graph Debugger.

## UI Thread Performance

Mobile UIs must maintain 60 fps (frame every 16.67ms) for smoothness. Any frame >16ms causes "jank" (observable stutter).

### Frame Lifecycle

```
[1] Input events queued
[2] Choreographer wakes (typically 60Hz, 16.67ms interval)
[3] Measure → Layout → Draw → Render (GPU)
[4] Display shows frame (if ready)
```

Any step >16ms delays the frame.

**Culprits**:

1. **Expensive Draw**: Complex shapes, non-integer coordinates, large clip regions force GPU to work harder.
2. **Layout Thrashing**: Calling `getMeasuredWidth()` triggers sync layout pass, then calling it again = 2 passes.
3. **Main Thread I/O**: Network call, database query on main thread = ANR (app not responding) if >5 seconds.
4. **GC Pause** (Android): Full GC can pause 100-500ms.

### Targeting 60 fps

**Tools**:

- **Android**: Profiler (frame timeline), GPU profiler.
- **iOS**: Instruments (Core Animation tool, fps counter).

**Optimization Strategy**:

```kotlin
// Bad: database call on main thread
override fun onCreate(savedInstanceState: Bundle?) {
  super.onCreate(savedInstanceState)
  val user = database.getUser(1) // ANR if >5s
  setContentView(layoutIdfor(user))
}

// Good: async on background thread
override fun onCreate(savedInstanceState: Bundle?) {
  super.onCreate(savedInstanceState)
  setContentView(R.layout.loading_screen)
  viewModel.loadUser(1) // async
}
```

**Compose/SwiftUI**:

Recomposition (re-running composable) happens on main thread. If recomposition includes expensive computation, jank results.

```kotlin
@Composable
fun UserList(users: List<User>) {
  // If users list changes, recomposes ENTIRE UserList
  // If compute is expensive, jank happens
  val sorted = users.sortedBy { it.name } // expensive if 10k+ users
  LazyColumn(users = sorted) { ... }
}
```

**Fix**: Compute outside recomposition (in ViewModel).

## Lazy Loading & Pagination

Loading all data upfront wastes memory and network. Instead, load in windows.

**Pagination Pattern**:

```kotlin
// Fetch 20 users per page
fun loadMoreUsers(page: Int) {
  val users = api.getUsers(limit=20, offset=page*20)
  // append to list
}
```

**Libraries**:

- **Android Paging 3**: Handles offset/cursor pagination, retry logic, prefetch.
- **iOS**: Custom pagination with NSFetchedResultsController or SwiftUI conditional loading.

```kotlin
val pager = Pager(PagingConfig(pageSize = 20)) { source ->
  UserPagingSource(api, db)
}.flow
  .cachedIn(viewModelScope)

val lazyUsers = pager.collectAsLazyPagingItems()
LazyColumn {
  items(lazyUsers.itemCount) { index ->
    UserRow(lazyUsers[index])
  }
}
```

## Image Loading & Caching

Images are largest app assets. Mismanagement causes memory spikes, jank, and power drain.

### Loading Libraries

**Android**: Coil, Glide, Picasso. Coil is modern (coroutine-based).

```kotlin
// Coil (recommended for Compose/Kotlin)
AsyncImage(
  model = "https://example.com/user.jpg",
  contentDescription = "User avatar",
  modifier = Modifier.size(64.dp),
  contentScale = ContentScale.Crop
)
```

**iOS**: SDWebImage (established), Kingfisher (Swift), Nuke.

```swift
// Kingfisher
ImageView.kf.setImage(with: URL(string: "https://example.com/user.jpg"))
```

### Cache Strategy

**3-Level Cache**:

1. **Memory**: LRU cache (fast access, bounded size ~50MB).
2. **Disk**: SQLite or file cache (survives app restart).
3. **Network**: HTTP cache headers (server respects If-Modified-Since).

**Typical Library Defaults**:

- Memory: 20-25% of heap.
- Disk: 50MB-1GB depending on device.

**Manual Tuning**:

```kotlin
// Coil ImageLoader with custom cache
ImageLoader.Builder(context)
  .memoryCache { MemoryCache.Builder(context).maxSizePercent(0.25).build() }
  .diskCache { DiskCache.Builder().directory(cacheDir).maxSizeBytes(50L * 1024 * 1024).build() }
  .build()
```

### Image Optimization

- **Appropriate Resolution**: Load thumbnail for list, full resolution only on detail screen.
- **Compression**: JPG (photos), WebP (mixed), PNG (graphics). WebP saves ~25% vs JPG.
- **Requests**: Batch requests (download 5 thumbnails parallel, not sequentially).

## Network Efficiency

Network latency dominates mobile apps (typical 50-500ms per request). Minimize round-trips.

### Strategies

- **Request Batching**: Fetch multiple users in one API call, not N calls.
- **Compression**: gzip (HTTP Accept-Encoding) reduces payload 60-80%.
- **Connection Pooling**: Reuse TCP connections (HTTP Keep-Alive). Libraries handle automatically.
- **DNS Caching**: Cache DNS lookups to avoid 50ms lookup per domain.
- **Offline Queuing**: Queue mutations (POST, PUT) if offline, sync when online.

### Monitoring

- **Metric**: Request latency, payload size, retry rate.
- **Tools**: Charles (proxy), Fiddler for inspection.

## Battery Awareness

Mobile devices run on battery. Power drain from:

1. **CPU**: Sustained computation; polls/busy loops waste power.
2. **Radio**: Cellular/WiFi radio is expensive; batch requests to minimize radio on-time.
3. **GPS**: Always-on location tracking drains 50% battery per hour.
4. **Display**: Brightness and refresh rate (120Hz uses more power than 60Hz).

### Power Optimization

- **Batch requests**: Instead of fetching every minute, fetch every 5 minutes.
- **WorkManager/BackgroundTasks**: Schedule low-priority tasks when device is plugged in.
- **Location**: Use coarse location if precision not needed. Disable when not in use.

```kotlin
// Request low-precision location, updates every 5min
val request = LocationRequest.Builder(Priority.PRIORITY_LOW_POWER, 5 * 60 * 1000).build()
```

## ProGuard / R8 Optimization

Android bytecode optimization and obfuscation.

### Shrinking

Remove unused classes, methods, fields detected via static analysis.

```
-dontshrink // disable if analysis is too conservative
```

### Obfuscation

Rename classes/methods to short names (a, b, c) to reduce binary size and hinder reverse engineering.

- Typical binary reduction: 10-20%.

### Optimization

ProGuard can optimize bytecode (inlining, dead code removal).

**Configuration**:

```proguard
# Keep third-party APIs from being obfuscated
-keep class com.google.** { *; }

# Keep annotations
-keepattributes *Annotation*

# Keep model classes (JSON serialization needs names)
-keep class com.example.model.** { *; }
```

## Frame Rate & 120Hz Displays

Modern phones support 120Hz or 144Hz (1 frame every 8.33ms). Apps can opt-in.

- **120Hz**: Requires 8.33ms per frame budget. Existing optimizations multiply in difficulty.
- **Adaptive Refresh**: Device scales refresh rate based on app content (static content → 60Hz, scrolling → 120Hz).

**Decision**: Use 120Hz if motion is frequent (scrolling lists, games). Otherwise 60Hz suffices.

```xml
<!-- Android: enable adaptive refresh -->
<adaptive-icon>
  <foreground android:drawable="@drawable/ic_launcher" />
  <monochrome android:drawable="@drawable/ic_launcher_mono" />
</adaptive-icon>
```

## Performance Testing

**Synthetic (lab)**: Use trace-based tools to measure specific scenarios.

**Real User Monitoring (RUM)**: Instrument production app to collect performance metrics.

- **Android**: Firebase Performance Monitoring.
- **iOS**: MetricKit.

## See Also

- [Performance Profiling](performance-profiling.md) — tools and methodology
- [Performance Optimization](performance-optimization.md) — general strategies
- [Web Performance](web-performance.md) — how to think about performance tradeoffs
- [Mobile Development Patterns](mobile-development-patterns.md) — platform lifecycle, interruption handling
- [Android Development Patterns](mobile-android-patterns.md) — View system vs Compose rendering