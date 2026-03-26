# Kotlin Coroutines: Structured Concurrency, Hands-on Guide

## Core Concepts: Suspension, Structured Concurrency, and Scope

Kotlin coroutines are lightweight threads that can be suspended and resumed. Unlike OS threads, thousands of coroutines can run without blocking hardware resources. The key innovation is **structured concurrency** — child coroutines are scoped to their parent, ensuring orderly cleanup and cancellation.

```kotlin
// Basic coroutine launch
import kotlinx.coroutines.*

fun main() = runBlocking {
    println("Main start")
    
    launch {  // Fire-and-forget child coroutine
        delay(1000)
        println("Child: after 1 second")
    }
    
    println("Main end")
    // Blocks until all children complete
}
// Output:
// Main start
// Main end
// Child: after 1 second
```

**Key insight:** `runBlocking` creates a CoroutineScope that blocks the calling thread until all child coroutines complete. It's typically used only in `main()` or for bridging coroutines to synchronous code.

## CoroutineScope and Job Hierarchy

Every coroutine runs in a scope. The scope provides a **Job** (a handle) and manages cancellation and exception propagation.

```kotlin
val scope = CoroutineScope(Dispatchers.Main)

// Launch a child coroutine
val job = scope.launch {
    delay(1000)
    println("Done")
}

// Job provides control
job.join()          // Wait for completion
job.cancel()        // Request cancellation
job.isActive        // Check if still running
job.isCancelled      // Check if cancelled
job.isCompleted      // Check if done or cancelled

// Cancel all children in scope
scope.cancel()

// Job hierarchy
val parent = Job()
val child = Job(parent = parent)  // child's fate tied to parent

parent.cancel()  // Also cancels child
```

**Parent-child relationship:** When a parent is cancelled, all children are cancelled transitively. If a child throws an uncaught exception, the parent (and siblings) are also cancelled by default.

## Async and Await

`async` launches a coroutine and returns a **Deferred** — a future-like object that holds the result.

```kotlin
fun main() = runBlocking {
    // Sequential: second waits for first
    val result1 = async {
        delay(1000)
        10
    }.await()  // Waits for result
    
    val result2 = async {
        delay(1000)
        20
    }.await()
    
    println("Total: ${result1 + result2}")
    // Takes ~2 seconds
}

// Parallel: launch both, then await
fun main() = runBlocking {
    val deferred1 = async {
        delay(1000)
        10
    }
    
    val deferred2 = async {
        delay(1000)
        20
    }
    
    val total = deferred1.await() + deferred2.await()
    println("Total: $total")
    // Takes ~1 second (concurrent)
}

// awaitAll for multiple async jobs
fun main() = runBlocking {
    val deferreds = (1..5).map { n ->
        async {
            delay(100)
            n * n
        }
    }
    
    val results = deferreds.awaitAll()
    println(results)  // [1, 4, 9, 16, 25]
}
```

## Dispatchers: Thread Scheduling

Dispatchers specify which thread(s) execute the coroutine. Common dispatchers:

```kotlin
// Main — UI thread (Android)
launch(Dispatchers.Main) {
    updateUI()  // Safe on main thread
}

// Default — thread pool (CPU-bound work)
launch(Dispatchers.Default) {
    val result = complexCalculation()
}

// IO — thread pool optimized for I/O
launch(Dispatchers.IO) {
    val data = readFile()  // Many concurrent I/O ops
    val response = httpClient.get(url)
}

// Unconfined — resume on whatever thread signaled completion (rare)
launch(Dispatchers.Unconfined) {
    // Can switch threads arbitrarily
}

// Custom: single-threaded
val singleThreadDispatcher = Dispatchers.Default.limitedParallelism(1)
launch(singleThreadDispatcher) {
    // Runs serially
}

// withContext — switch dispatchers within a coroutine
suspend fun fetchUserData(id: String): User = withContext(Dispatchers.IO) {
    api.getUser(id)  // Runs on IO dispatcher
}  // Resume on caller's dispatcher
```

**Rule of thumb:**
- `Main` — UI updates, light async coordination
- `Default` — CPU work (sorting, parsing, algorithms)
- `IO` — Network, database, file operations
- Custom — specific thread requirements

## Suspend Functions and Suspension Points

A `suspend` function can pause execution and resume later. Suspension doesn't block a thread — the coroutine is removed from the dispatcher, freeing the thread.

```kotlin
suspend fun fetchUser(id: String): User {
    // Runs on the caller's dispatcher
    return withContext(Dispatchers.IO) {
        api.getUser(id)  // Suspension point
        // Thread is released while waiting for response
    }
}

// Calling suspend function requires a suspending context
launch {  // Inside coroutine
    val user = fetchUser("123")  // Can call suspend
}

// Regular function cannot call suspend
fun regular() {
    val user = fetchUser("123")  // ❌ Compile error
}

// Composed suspend functions
suspend fun getUserWithPosts(id: String): UserWithPosts {
    val user = fetchUser(id)           // Suspend point
    val posts = fetchPosts(user.id)    // Suspend point
    return UserWithPosts(user, posts)
}
```

## Flow: Cold, Lazy Streams

Flow is a reactive stream type: cold (doesn't execute until collected), lazy, and backpressure-aware.

```kotlin
fun countNumbers(): Flow<Int> = flow {
    for (i in 1..5) {
        delay(100)
        emit(i)  // Send to collector
    }
}

fun main() = runBlocking {
    countNumbers()
        .map { it * 2 }
        .filter { it > 4 }
        .collect { println(it) }  // Triggers execution
    
    // Output: 4, 6, 8, 10 (with delays)
}

// Flow operators (similar to Sequence, but suspending)
fun main() = runBlocking {
    countNumbers()
        .take(3)              // First 3 items
        .transform { emit(it * 2) }  // Custom transform
        .onEach { delay(10) }  // Side effect
        .collect { println(it) }
}

// flatMapLatest — cancel previous operation, start new
fun main() = runBlocking {
    userIds.asFlow()
        .flatMapLatest { id ->
            fetchUserFlow(id)  // If new ID arrives, cancel previous fetch
        }
        .collect { user -> println(user) }
}
```

**Key distinction:** Flow is **cold** (executes on each collection), unlike `Channel` which is **hot** (produces eagerly).

## StateFlow and SharedFlow

SharedState across coroutines without manual synchronization.

```kotlin
// StateFlow — property-like value holder with current state
class ViewModel {
    private val _count = MutableStateFlow(0)
    val count: StateFlow<Int> = _count.asStateFlow()
    
    fun increment() {
        _count.value += 1  // Emit new state
    }
}

class UI {
    fun bind(viewModel: ViewModel) = runBlocking {
        viewModel.count.collect { count ->
            printf("Count: $count")
        }
    }
}

// SharedFlow — broadcast to multiple collectors
class EventBus {
    private val _events = MutableSharedFlow<Event>()
    val events: SharedFlow<Event> = _events.asSharedFlow()
    
    suspend fun publish(event: Event) {
        _events.emit(event)  // All collectors receive
    }
}
```

## Channels: Hot, Bounded Queues

Channels pass values between coroutines with optional buffering. Hot (produce regardless of collectors) and bounded (limited queue size).

```kotlin
fun main() = runBlocking {
    val channel = Channel<Int>(capacity = 3)
    
    launch {
        for (i in 1..10) {
            channel.send(i)  // Blocks if buffer full
            println("Sent $i")
        }
        channel.close()
    }
    
    for (value in channel) {  // Collect until close
        println("Received $value")
        delay(500)  // Slow consumer
    }
}

// Channel variants
val unlimited = Channel<Int>(Channel.UNLIMITED)  // Unbuffered
val rendezvous = Channel<Int>()                  // Sender blocks till receiver gets it
val buffered = Channel<Int>(capacity = 5)       // Limit queue

// produce — convenience for sending to channel
fun produceNumbers() = coroutineScope {
    produce<Int> {
        for (i in 1..10) {
            send(i)
            delay(100)
        }
    }
}

fun main() = runBlocking {
    for (value in produceNumbers()) {
        println(value)
    }
}
```

## Exception Handling

Exceptions in coroutines propagate to the parent. Uncaught exceptions cancel siblings.

```kotlin
launch {
    try {
        val result = async {
            throw RuntimeException("Oops")
        }.await()
    } catch (e: Exception) {
        println("Caught: ${e.message}")
    }
}

// CoroutineExceptionHandler — global fallback
val handler = CoroutineExceptionHandler { _, exception ->
    println("Exception: ${exception.message}")
}

launch(handler) {
    throw RuntimeException("Uncaught")
}

// supervisorScope — child exceptions don't cancel siblings
supervisorScope {
    val job1 = async {
        delay(100)
        throw RuntimeException("Job 1 failed")
    }
    
    val job2 = async {
        delay(200)
        "Job 2 succeeded"
    }
    
    try {
        job1.await()
    } catch (e: Exception) {
        println("Job 1: ${e.message}")
    }
    
    println(job2.await())  // Still runs despite job1 exception
}
```

## Cancellation and Cleanup

Cooperative cancellation: a coroutine must check for cancellation or use cancellation-aware suspend functions.

```kotlin
// Cancellation-aware code
launch {
    try {
        repeat(1000) {
            delay(10)  // Throws CancellationException if cancelled
            println("Iteration $it")
        }
    } finally {
        println("Cleaning up")
    }
}

// Manual cancellation check
launch {
    repeat(1000) {
        if (!isActive) break  // Check active flag
        println("Iteration $it")
    }
}

// withTimeoutOrNull — timeout-based cancellation
val result = withTimeoutOrNull(1000) {
    delay(2000)
    "Done"
}
println(result)  // null (timeout)

// ensureActive — throw if cancelled
launch {
    try {
        ensureActive()
    } catch (e: CancellationException) {
        println("Already cancelled")
    }
}
```

## Testing Coroutines

`runTest` provides a test dispatcher that advances time automatically, eliminating delays.

```kotlin
class UserRepositoryTest {
    @Test
    fun testFetchUser() = runTest {
        val repo = UserRepository(FakeApi())
        
        val user = repo.getUser("123")  // Instant, no delay
        assertEquals("Alice", user.name)
    }
}

// FakeApi for testing
class FakeApi : Api {
    override suspend fun getUser(id: String): User {
        delay(1000)  // Ignored in runTest
        return User(id, "Alice")
    }
}

// Test dispatcher combos
@Test
fun testWithCustomDispatcher() = runTest(Dispatchers.IO) {
    // IO operations execute instantly
}
```

## Android Integration

Android's scope management prevents common memory leaks.

```kotlin
class MainActivity : AppCompatActivity() {
    private val viewModel = UserViewModel()
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.main)
        
        // lifecycleScope — cancels when lifecycle is destroyed
        lifecycleScope.launch {
            viewModel.user.collect { user ->
                updateUI(user)  // Safe: collector cancels on destroy
            }
        }
        
        // repeatOnLifecycle — restarts when lifecycle resumes
        lifecycleScope.repeatOnLifecycle(Lifecycle.State.STARTED) {
            viewModel.user.collect { user ->
                updateUI(user)
            }
        }
    }
}
```

---

## See Also

- [Language: Kotlin Conventions](language-kotlin.md)
- [Paradigm: Concurrent Models](paradigm-concurrent-models.md)
- [Python Async: Asyncio](language-python-async.md)