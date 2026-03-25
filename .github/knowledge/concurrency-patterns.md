# Concurrency & Parallelism Patterns

## Core Concepts

### Concurrency vs. Parallelism

- **Concurrency**: Structuring code to handle multiple tasks that overlap in time (interleaved execution). One CPU can handle concurrent tasks via context switching.
- **Parallelism**: Actually executing multiple tasks simultaneously on multiple CPUs/cores.
- Concurrency is about _structure_. Parallelism is about _execution_.

### Threads vs. Processes

- **Process**: Own memory space, isolation, heavyweight. Use for: CPU-heavy work, isolation requirements.
- **Thread**: Shared memory within a process, lighter weight. Use for: I/O-heavy work, shared state.
- **Green threads / coroutines**: Managed by the runtime (not OS). Extremely lightweight. Go goroutines, Erlang processes, Kotlin coroutines, Python asyncio tasks.

## Synchronization Primitives

### Mutex (Mutual Exclusion)

Protects a critical section — only one thread can hold the lock at a time. Always lock and unlock in the same scope. Avoid holding locks while doing I/O.

### Read-Write Lock (RWLock)

Multiple readers can hold the lock simultaneously, but writers need exclusive access. Use when reads vastly outnumber writes.

### Semaphore

Controls access to a finite pool of resources. Allows N concurrent accessors (mutex is a semaphore with N=1). Use for: connection pools, rate limiting.

### Condition Variable

Allows threads to wait for a specific condition to become true. Always use with a mutex. Avoids busy-waiting.

### Atomic Operations

Lock-free operations on primitive values (compare-and-swap, fetch-and-add). Fastest synchronization but limited to simple operations. Every language has atomic types: `AtomicInteger` (Java), `std::atomic` (C++), `sync/atomic` (Go), `Atomics` (JS SharedArrayBuffer).

## Common Problems

### Race Condition

Two or more threads access shared data simultaneously and at least one modifies it. Result depends on execution timing. Fix: synchronize access (locks, atomics, message passing).

### Deadlock

Two or more threads each hold a lock and wait for the other's lock. Neither can proceed. Prevention:

1. Always acquire locks in the same order.
2. Use timeouts on lock acquisition.
3. Prefer lock-free designs.
4. Use `try_lock` instead of blocking.

### Livelock

Threads keep changing state in response to each other but make no progress. Like two people trying to pass each other in a hallway.

### Starvation

A thread never gets CPU time because higher-priority threads always run first. Fix: fair scheduling, priority inversion protocols.

## Concurrency Patterns

### Producer-Consumer (Bounded Buffer)

Producers add items to a shared queue. Consumers take items from it. The queue handles synchronization. Use for: task processing, data pipelines, work distribution.

### Actor Model (Erlang, Akka)

Each actor is an independent entity with its own state. Actors communicate only through asynchronous message passing. No shared state → no locks. Use for: highly concurrent, fault-tolerant systems. Languages: Erlang/Elixir (native), Akka (JVM), Actix (Rust).

### CSP — Communicating Sequential Processes (Go)

Goroutines communicate through channels. "Don't communicate by sharing memory; share memory by communicating." Channels can be:

- **Unbuffered**: Sender blocks until receiver is ready (synchronization point).
- **Buffered**: Sender blocks only when buffer is full.
- `select` statement handles multiple channels.

### Fork-Join

Split work into parallel subtasks (fork), execute them, then combine results (join). Java `ForkJoinPool`, parallel streams. Use for: divide-and-conquer, map-reduce.

### async/await (Cooperative Concurrency)

Single-threaded concurrency for I/O-bound workloads. The runtime suspends a function at `await` points and runs other tasks while waiting. Languages: JavaScript, Python, Rust, C#, Swift, Kotlin.

- **Not parallelism** — still one thread (except Rust/Kotlin where the runtime may use a thread pool).
- Perfect for: HTTP servers, database queries, file I/O.
- Bad for: CPU-intensive computation (blocks the event loop).

### Thread Pool

Pre-create a fixed number of threads. Submit tasks to a queue; idle threads pick up work. Avoids thread creation overhead. Java `ExecutorService`, Python `concurrent.futures.ThreadPoolExecutor`, .NET `ThreadPool`.

### Event Loop

Single thread processes events from a queue. When I/O completes, its callback is queued. Node.js, browser JavaScript, Python asyncio, Tokio (Rust). Extremely efficient for I/O-bound workloads. Must never block the loop — offload CPU work to worker threads.

## Best Practices

1. **Prefer immutable data.** If nothing is mutable, there are no race conditions.
2. **Prefer message passing over shared state.** Channels > locks.
3. **Keep critical sections small.** Hold locks for the minimum time.
4. **Use higher-level abstractions** (async/await, channels, actors) over raw threads/locks when possible.
5. **Test concurrent code** with stress tests, race detectors (Go `-race`, ThreadSanitizer), and property-based testing.
6. **Document thread-safety contracts** explicitly in APIs.

---

_Sources: Go Blog ("Share Memory By Communicating"), Rust Book (Fearless Concurrency), Java Concurrency in Practice (Goetz), Erlang documentation (Actor Model), Joe Armstrong (Programming Erlang)_
