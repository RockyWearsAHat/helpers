# Java Virtual Threads — Project Loom, Structured Concurrency, and Lightweight Async

## Overview

Virtual threads (Project Loom) represent a paradigm shift in Java concurrency. They are lightweight, user-mode threads managed by the JVM rather than the kernel. A single JVM process can host millions of virtual threads with negligible memory overhead. Unlike platform threads (OS threads, one-to-one with OS kernel threads), virtual threads use a small pool of platform threads as "carrier threads" and rapidly switch context between them, enabling thread-per-request concurrency at scale.

**Key shift**: Traditional thread pools scale to hundreds; virtual threads scale to millions per process.

## Virtual Threads vs Platform Threads

### Memory & Creation Cost

**Platform threads**: ~1-2 MB each (including stack). Max ~thousands per JVM, limited by OS resources.

```java
// Platform thread: expensive
Thread t = new Thread(() -> {
    // OS kernel manages this thread
});
t.start();
```

**Virtual threads**: ~100 bytes, no OS kernel involvement. Millions possible.

```java
// Virtual thread: cheap
Thread vt = Thread.ofVirtual()
    .start(() -> {
        // JVM scheduler manages this thread
    });
```

Virtual threads are created on-demand, used once, then discarded (no pooling overhead). Blocking a virtual thread does not block the carrier thread; the JVM unmounts the virtual thread, allowing the carrier to service other virtual threads.

### Blocking Behavior

Blocking a virtual thread (I/O wait, locks) causes **unmounting**: the JVM suspends the virtual thread and schedules it for resumption later, freeing the carrier thread. Most Java I/O operations (SocketChannel.read(), URLConnection) support automatic unmounting. Synchronized blocks and ReentrantLock also unmount.

**Pinning** occurs when a virtual thread cannot unmount because the carrier thread is pinned by:
- Native code execution
- `synchronized` blocks
- `Object.wait()` in critical paths

Pinning prevents the carrier from serving other virtual threads, reducing concurrency.

## Structured Concurrency (Java 21+)

Structured concurrency introduces deterministic lifecycle management for concurrent tasks—no task escapes, and cancellation is guaranteed.

### StructuredTaskScope

`StructuredTaskScope` ensures all forked tasks complete before the scope exits, either successfully or via exception.

```java
// Virtual threads + structured concurrency
public static String fetchData(String url1, String url2) throws Exception {
    try (var scope = new StructuredTaskScope.ShutdownOnFailure()) {
        Callable<String> task1 = scope.fork(() -> fetch(url1));
        Callable<String> task2 = scope.fork(() -> fetch(url2));
        
        scope.joinUntilComplete();  // Wait for all tasks or first failure
        scope.throwIfFailed();       // If any failed, throw
        
        return task1.resultNow() + task2.resultNow();
    }
    // Scope guarantees all threads are joined before exit
}
```

### Cancellation Semantics

If one thread fails, `ShutdownOnFailure` cancels remaining tasks (interrupts virtual threads). `ShutdownOnSuccess` stops after first success.

```java
try (var scope = new StructuredTaskScope.ShutdownOnSuccess<Integer>()) {
    scope.fork(() -> downloadFromMirror1());
    scope.fork(() -> downloadFromMirror2());
    scope.fork(() -> downloadFromMirror3());
    
    scope.joinUntilComplete();
    return scope.result();  // First successful result
}
// On exit, any remaining tasks are cancelled
```

## Scoped Values

Scoped values are immutable, thread-local values bound to a specific task scope and inherited by child threads. Unlike ThreadLocal (global, mutable), scoped values have explicit lifecycle.

```java
static final ScopedValue<User> CURRENT_USER = ScopedValue.newInstance();

// Bind scoped value
ScopedValue.where(CURRENT_USER, user)
    .run(() -> {
        var u = CURRENT_USER.get();  // user is accessible
        // Inherited by virtual threads forked within scope
    });

// In a forked virtual thread:
try (var scope = new StructuredTaskScope.ShutdownOnFailure()) {
    scope.fork(() -> {
        var u = CURRENT_USER.get();  // Inherited from parent
        return u.name();
    });
    scope.joinUntilComplete();
}
```

Scoped values inherit down the call stack but are immutable—no cross-thread mutations.

## Thread-Per-Request at Scale

Virtual threads enable the thread-per-request model (one thread per HTTP request) to scale to millions of concurrent requests without thread pools.

```java
// Before: limited by thread pool size
ExecutorService pool = Executors.newFixedThreadPool(200);
for (HttpRequest req : requests) {
    pool.submit(() -> handleRequest(req));  // 200 threads max
}

// After: virtual threads, no pooling
try (ExecutorService executor = Executors.newVirtualThreadPerTaskExecutor()) {
    for (HttpRequest req : requests) {
        executor.submit(() -> handleRequest(req));  // Millions possible
    }
} // wait for all
```

Spring Boot 21+ defaults to virtual threads for request handling. Each HTTP request gets its own virtual thread; blocking on I/O is efficient (unmounts to carrier).

## Carrier Thread Pinning

Pinning reduces concurrency because the carrier thread cannot serve other virtual threads while a pinned virtual thread is running.

### Common Pinning Sources

1. **Synchronized blocks**: Use ReentrantLock instead for better unpinning behavior
   
   ```java
   // Pinned
   synchronized (monitor) {
       Thread.sleep(1000);  // Carrier is blocked, other VTs wait
   }
   
   // Unpins (may repin if lock contention, but preferred)
   lock.lock();
   try {
       Thread.sleep(1000);
   } finally {
       lock.unlock();
   }
   ```

2. **Native code**: JNI calls pin the carrier thread.
3. **Object.wait()**: Legacy, pins the carrier (use Lock.await() instead).

VM flag `-XX:+UnlockDiagnosticVMOptions -XX:+PrintJfrStatistics` logs pinning events.

## Migration from Thread Pools

### ExecutorService Patterns

```java
// Old: explicit pool management
ExecutorService pool = Executors.newFixedThreadPool(100);
pool.submit(() -> handleTask());
pool.shutdown();
pool.awaitTermination(60, TimeUnit.SECONDS);

// New: virtual thread per task
ExecutorService executor = Executors.newVirtualThreadPerTaskExecutor();
executor.submit(() -> handleTask());
executor.close();  // Auto-waits
```

### Reactive → Virtual Threads

Reactive libraries (RxJava, Project Reactor) used callbacks to avoid blocking thread pools. Virtual threads eliminate this need:

```java
// Before: reactive chains, complex error handling
mono
    .flatMap(user -> fetchDetails(user))
    .map(details -> process(details))
    .onErrorResume(err -> handleError(err))
    .subscribe();

// After: straight-line imperative code
try {
    User user = fetchUser();
    Details details = fetchDetails(user);
    process(details);
} catch (Exception e) {
    handleError(e);
}
// No thread pool bottleneck; virtual concurrency is free
```

## Performance Characteristics

### Throughput

Virtual threads excel at **high concurrency, I/O-heavy workloads**:
- 10,000+ concurrent requests: virtual threads faster
- Each request waits on network/DB: virtual threads unmount, carrier services others
- CPU-bound loops: no benefit (thread count doesn't help); use parallel streams or ForkJoinPool

### Memory

Virtual threads use ~100-200 bytes; heap-allocated stack frames (~1KB per method frame) only if not inlined. Platform threads use ~1-2 MB stack.

### Latency

Context switching between virtual threads is JVM-managed and fast (~microseconds). No syscall overhead. However, GC pauses affect all virtual threads equally.

## Debugging & Monitoring

Virtual threads appear in thread dumps (JDK 21+):

```
#71 "virtual-1" virtual thread
    java.lang.VirtualThread/"virtual-1"@0x...
    java.lang.Thread.run
```

JFR (Java Flight Recorder) events for virtual thread lifecycle:

```bash
java -XX:StartFlightRecording=filename=vt.jfr,dumponexit=true App
jfr dump vt.jfr
```

Relevant JFR events: `jdk.VirtualThreadStart`, `jdk.VirtualThreadEnd`, `jdk.VirtualThreadPin`.

## Limitations & Edge Cases

1. **Pinning is silent**: No compiler warning; profiling is required. Use `-XX:+UnlockDiagnosticVMOptions -XX:+PrintJfrStatistics` to detect.
2. **ThreadLocal still works** (inherited by virtual threads) but scoped values are preferred for new code.
3. **SecurityManager** restrictions apply to virtual threads as normal.
4. **Cluster-aware async patterns**: Virtual threads simplify single-JVM async; for distributed async (e.g., workflow engines), use explicit messaging.
5. **GC pause affect all VTs equally**: If GC pauses 200 ms, all virtual threads pause.

## See Also

- Structured concurrency (JEP 428, 453)
- Scoped values (JEP 429, 446)
- Virtual threads (JEP 444)
- Concurrency patterns
- Thread pools (legacy comparison)