# Concurrency Models — Shared-Memory, Message-Passing, and Beyond

## Overview

Concurrent systems need to coordinate multiple tasks. The choice of **concurrency model** shapes how you think about coordination, what bugs can occur, and how systems scale. There is no universally best model; each trades correctness, expressivity, and performance differently.

The main models are:

1. **Shared-memory (threads + locks)** — traditional, powerful, error-prone
2. **Message-passing (actors, CSP)** — safer by default, more overhead
3. **Software transactional memory (STM)** — shared state without locks
4. **Async/await** — cooperative multitasking on a single thread
5. **Green threads / fibers** — lightweight abstractions over OS threads
6. **Structured concurrency** — strong fairness and cleanup guarantees

## Shared-Memory Concurrency: Threads and Locks

### The Model

Multiple threads share memory directly. Coordination uses **locks** (mutexes) to serialize access to shared data. This is the default for C, C++, Java (classic threading), Python (despite the GIL), and most OS-level concurrency.

```
Thread A      Shared Memory      Thread B
   |               |                 |
   +--lock(M)------+                 |
   |             [Critical Section]   |
   +--unlock(M)---+                 |
                    |--lock(M)-------+
                    | [Critical Section]
                    +--unlock(M)---
```

### Advantages

- **Performance**: minimal overhead on single core; native OS support
- **Familiar**: imperative model matches single-threaded thinking
- **Powerful**: shared memory enables fine-grained coordination

### Problems

1. **Data races**: unsynchronized access to shared memory → undefined behavior
2. **Deadlocks**: circular wait for locks (Thread A waits for B's lock, B waits for A's lock)
3. **Complexity**: reasoning about interleaving of threads is combinatorially explosive
4. **Priority inversion**: low-priority thread holds lock, high-priority thread waits
5. **False sharing**: threads on different cores contend for the same cache line, thrashing performance

### Lock Strategies

- **Coarse-grained**: one lock for large data structures (fast synchronization, more contention)
- **Fine-grained**: many locks for small pieces (less contention, harder to reason about, deadlock risk)
- **Lock-free**: CAS (compare-and-swap) loops, atomic operations (fast, very hard to get right)
- **Read-write locks**: multiple readers XOR one writer (better for read-heavy workloads)

### Tools

- **Condition variables**: allow threads to wait for a condition and be signaled
- **Barriers**: synchronize N threads at a rendezvous point
- **Semaphores**: counter-based synchronization

**Empirically**, shared-memory bugs are the #1 source of concurrency failures. Studies of production bugs show data races and deadlocks are common and hard to debug.

## Message-Passing Concurrency: Actors

### The Model

Actors are isolated entities that communicate only via asynchronous messages. No shared state between actors; each actor processes its mailbox sequentially.

```
Actor A (mailbox)        Actor B (mailbox)      Actor C (mailbox)
  [msg1, msg2]             [msg3]                 []
     |                       |                     |
  Process msg1           Process msg3          (idle)
  ...send to B/C...                              |
     |                       |                 Receive from A
```

Key properties:

- **Location transparency**: send to same actor or remote; API is identical
- **Supervision**: actors form hierarchies with parent-child relationships; parent handles child failures
- **Resilience**: if one actor crashes, others continue
- **Implicit batching**: each actor processes one message at a time, eliminating many race conditions

### Advantages

- **Simpler reasoning**: each actor is single-threaded; no internal synchronization needed
- **Fault tolerance**: supervision enables recovery from failures
- **Scalability**: millions of lightweight actors are feasible
- **Distribution-ready**: message semantics work locally and remotely

### Disadvantages

- **Latency**: message passing has overhead vs. shared memory
- **Debugging**: asynchronous execution makes stack traces harder to interpret
- **Ordering guarantees**: messages can be reordered by the network (requires application-level sequencing)
- **Back-pressure**: if actor mailbox fills, senders still get exceptions (or block); requires careful handling

### Implementations

- **Erlang/Elixir**: BEAM VM: lightweight, hot code reload, "let it crash" philosophy
- **Akka (JVM)**: distributed, location-transparent, typed (Akka Typed)
- **Tokio (Rust)**: async task framework with mpsc channels
- **Go**: goroutines + channels are actor-like (though Go philosophy differs)

## Communicating Sequential Processes (CSP)

### The Model

Processes communicate via **channels** — typed, synchronous (or buffered) conduits. A channel couples sender and receiver tightly; both block until data is exchanged.

```go
// Go syntax
ch := make(chan int, 0)  // unbuffered channel (synchronous)

go func() {
    ch <- 42  // send; blocks until receiver is ready
}()

x := <-ch   // receive; blocks until sender is ready
```

CSP is formally defined by Hoare's 1978 paper and rooted in process algebra.

### Advantages

- **Clear semantics**: synchronous channels have deterministic ordering
- **Built-in flow control**: blocking ensures back-pressure
- **Composable**: processes combine via channels naturally
- **Deadlock-detectable**: static analysis can find potential deadlocks

### Disadvantages

- **Synchronous blocking**: can cause performance issues or artificial waits
- **Buffering trade-offs**: unbuffered → tightly coupled, buffered → decoupling but queue management
- **Distributed complexity**: synchronous channels don't work well across networks; real systems need async + timeouts

### Implementations

- **Go**: channels are central to the language; `select` for multiplexing
- **Rust**: crossbeam, tokio mpsc (though Rust also supports shared-memory concurrency)
- **Occam (academic)**: purely CSP-based language

## Message-Passing vs. Shared-Memory

### Key Differences

| Aspect | Shared-Memory | Message-Passing (Actor) | CSP |
|--------|--------------|------------------------|-----|
| **Coupling** | Tight (shared data) | Loose (messages only) | Tight (channels couple endpoints) |
| **Reasoning** | Global state, interleaving | Isolated state, async comms | Process algebra, synchronization |
| **Debugging** | Stack traces, shared state inspection | Distributed logs, message traces | Temporal logic, CSP specifications |
| **Latency** | Low (direct memory access) | Higher (message overhead) | Medium (channel ops + blocking) |
| **Scalability** | Poor (contention + GC) | Good (lightweight + isolation) | Moderate (channel overhead) |
| **Distribution** | Not straightforward | Natural (locality-transparent) | Awkward (channels are local) |

## Software Transactional Memory (STM)

### The Model

**STM** allows shared-memory access without explicit locks. Transactions execute speculatively; if concurrent modifications are detected, the transaction **aborts and retries**.

```haskell
-- Haskell STM example
transfer from to amount = atomically $ do
    fromBalance <- readTVar from
    if fromBalance >= amount
        then do
            writeTVar from (fromBalance - amount)
            toBalance <- readTVar to
            writeTVar to (toBalance + amount)
        else error "Insufficient funds"
```

### Advantages

- **No deadlocks**: conflicts cause retries, not circular waits
- **Composability**: transactions compose naturally without deadlock worry
- **No priority inversion**: retries don't cause ordering issues
- **Simpler code**: looks like shared-memory (no lock management)

### Disadvantages

- **Overhead**: tracking reads/writes for conflict detection is expensive
- **Aborts under contention**: high conflict rates cause thrashing
- **I/O in transactions is problematic**: output can't be undone; side effects are risky
- **Limited adoption**: Haskell has it, but few mainstream languages do (some languages offer weaker ABIs)

**Empirically**: STM shines in scenarios with moderate contention and low-conflict workloads. High contention makes it slower than locks due to retry overhead.

## Futures / Promises

### The Model

A **future** (or **promise**) is a handle to a value that will be available in the future. Computation proceeds asynchronously; code can await the result.

```javascript
// JavaScript Promises
const p = fetch('/api/data')
    .then(resp => resp.json())
    .then(data => process(data))
    .catch(err => console.error(err))
```

Futures are like one-way channels: producer computes a value; consumers wait for it. No back-and-forth messaging, just a single result (or error).

### Characteristics

- **Composable**: `then`, `map`, `flatMap` chain futures
- **Non-blocking**: callbacks execute in a event loop or threadpool
- **Eager execution**: computation starts immediately (differs from lazy evaluation)
- **Error propagation**: exceptions chain through the future

### Variants

- **Promises** (JavaScript): mutable state (pending/resolved/rejected)
- **Futures** (Java, Rust, Scala): immutable, resolved once
- **Tasks** (C#): similar to futures
- **Continuations**: lowest-level; call-stack suspended, resumed later

## Async/Await

### The Model

**Async/await** is syntactic sugar for continuations. An `async` function is compiled into a state machine; `await` suspends execution and registers a callback.

```python
# Python asyncio
async def fetch():
    resp = await httpx.get('/api')  # suspend here; resume when ready
    return resp.json()

async def main():
    data1 = await fetch()
    data2 = await fetch()  # await both
    return data1, data2
```

Async/await makes asynchronous code look sequential, hiding the underlying callback machinery.

### Advantages

- **Readability**: imperative style, not callback Hell (`then().then()...`)
- **Composability**: `await` at any level flattens nesting
- **Error handling**: try-catch works with async code
- **Efficient**: single thread with cooperative multitasking; low context-switch overhead

### Disadvantages

- **Contagion**: asyncness spreads; calling code must also be async
- **Implicit thread pool**: scheduler is hidden; debugging thread behavior is opaque
- **CPU-bound work blocks the loop**: long computations freeze other tasks
- **Backpressure**: if producer is too fast, unbuffered queues overflow (need bounded queues + retry logic)

### Implementations

- **JavaScript**: ES6 async/await (unified with Promises)
- **Python**: asyncio, trio, anyio frameworks
- **Rust**: tokio, async-std (borrowed from futures; matures in 1.70+)
- **C#**: async/await (Task-based)

**Key difference from threads**: async code runs on a single thread (or small thread pool) via a scheduler. Threads are preempted by the OS; async tasks yield control explicitly (or via await). This makes async lighter-weight and more predictable, but requires async-aware libraries everywhere in the call stack.

## Green Threads / Lightweight Processes

### The Model

**Green threads** are user-space lightweight threads managed by the runtime, not the OS. Multiple green threads can run on a single OS thread or be distributed across a thread pool.

The **M:N threading model**: M green threads on N OS threads.

### Erlang BEAM

The **Erlang Machine** (BEAM) executes Erlang processes as green threads. The runtime:
- Schedules millions of lightweight processes
- Preempts every N reductions (fixed quanta, unlike time-slicing)
- Monitors memory use tightly (each process has a heap)
- Supports hot reloading: code changes without stopping processes

```erlang
spawn(my_module, my_func, [Args]).  % spawn a process
```

Erlang processes are isolated; no shared state. This isolation enables crash recovery without cascade failures.

### Java Virtual Threads (Project Loom)

Virtual threads (Java 21+) are lightweight tasks on a thread pool. Similar to Erlang processes but:
- Can be pinned to an OS thread (e.g., if holding a lock that blocks the scheduler)
- Inherit context (ThreadLocal, requestcontext)
- Integrate with existing Java APIs (no async rewrite needed)

### Go Goroutines

Goroutines are green threads on a work-stealing scheduler. Key features:

- **Minimal overhead**: creating millions is feasible
- **Work-stealing**: idle threads steal work from busy threads for load balancing
- **Blocking** is cooperative: a goroutine can block on I/O or channels; the scheduler yields to others
- **M:N on top of M:1 initially**: Go 1.5+ uses work-stealing across multiple OS threads

Go's philosophy: goroutines + channels. Simpler mental model than Erlang supervision, cleaner than callbacks + promises.

### Comparison

| System | Thread Model | Scheduling | State | Isolation |
|--------|--------------|-----------|-------|-----------|
| **Erlang** | Green (BEAM) | Preemptive (reductions) | Actor model | Strong (process = isolated heap) |
| **Java Virtual Threads** | Green (ForkJoinPool) | Preemptive (time-sliced) | Can share memory | Weak (threads share JVM heap) |
| **Go Goroutines** | Green (work-stealing) | Cooperative (blocking) + preemptive | CSP (channels) | Medium (goroutines isolated by convention) |

## Structured Concurrency

### The Model

**Structured concurrency** organizes concurrent tasks into hierarchies with strong cancellation and cleanup semantics. Inspired by structured programming (where goto was replaced by block structure).

Principles:
- **Parent-child relationships**: spawned tasks are children; parent awaits all children
- **Cancellation**: cancelling a parent cancels all children
- **Exception handling**: exceptions from children propagate to parent
- **Deterministic cleanup**: when a scope exits, all child tasks are awaited and cleaned up

```kotlin
// Kotlin coroutines (scope-based)
runBlocking {
    val scope = CoroutineScope(Dispatchers.Default)
    scope.launch {
        // child task
    }
    // parent awaits all children on scope exit
}
```

### Advantages

- **Fairness**: no task starves children (parent waits for all)
- **Cancellation is clear**: hierarchical propagation, no surprise orphans
- **Deterministic cleanup**: finally / try-catch-finally semantics extend to concurrency
- **No resource leaks**: scope exit guarantees all tasks are done

### Implementations

- **Kotlin Coroutines**: full integration, scopes are first-class
- **Python Trio**: async library; nursey (scope) for child tasks
- **Java**: using structured concurrency (JEP 428, Java 21 preview, full in Java 23)
- **Erlang**: supervisor trees (similar concept for process hierarchies)

### Controversy

Structured concurrency is relatively new. Critics note:
- **Overly restrictive**: some use cases (background workers, long-lived services) don't fit hierarchies
- **Interop**: retrofitting into existing systems (thread pools, callbacks) is awkward
- **Performance**: strict cleanup may have overhead

## Comparing Models in Practice

### Shared-Memory Threads

Best for: **Compute-bound workloads, tight loops, fine-grained data sharing**.

Worst for: **Large numbers of I/O tasks, distributed systems, high concurrency at scale**.

### Actors

Best for: **Distributed systems, fault tolerance, supervisor hierarchies, millions of concurrent tasks**.

Worst for: **Tight loops, fine-grained shared state, low-latency message ordering** (network reorders).

### CSP (Channels)

Best for: **Pipeline stages, clear data flow, formal verification**.

Worst for: **Distributed systems (channels are local), many-to-many communication patterns**.

### Async/Await

Best for: **I/O-bound applications (web servers, databases), modern languages with ecosystem support**.

Worst for: **CPU-bound tasks, deep call stacks, legacy systems without async APIs**.

### Green Threads

Best for: **High concurrency without callback rewriting, platforms that can afford per-thread overhead** (GC, memory).

Worst for: **Extremely high concurrency (Erlang uses more memory per process), tight CPU-bound loops** (preemption overhead).

### Structured Concurrency

Best for: **Fairness guarantees, exception handling, cleanup semantics, scope-bounded parallelism**.

Worst for: **Background tasks outside scopes, complex task dependencies**, long-lived detached work.

## Mental Model: What Bugs Can Occur?

| Model | Primary Bug | Debugging | Risk |
|-------|------------|-----------|------|
| **Shared-memory** | Data races, deadlock | Stack traces, shared state | Very high; subtle interleaving bugs |
| **Actors** | Mailbox overflow, logic errors | Message traces, timing | Medium; isolated by design |
| **CSP** | Deadlock (channel timeout helps) | Formal analysis, traces | Medium; but more analysis tools |
| **Async/Await** | Scheduler starvation, resource exhaustion | Event loop tracing | Medium; callbacks hide issues |
| **Green threads** | Similar to async (fewer low-level controls) | Preemption is managed by runtime | Lower (isolation helps) |
| **Structured concurrency** | Task cancellation timeouts, logic errors | Scope exit traces | Low; hierarchical guarantees |

## See Also

- [The Actor Model — Message-Passing Concurrency](paradigm-actor-model.md)
- [Concurrency & Parallelism Patterns](concurrency-patterns.md) — implementation techniques
- [OS I/O Models — Blocking, Non-Blocking, Async, and Multiplexing](os-io-models.md)
- [OS Concurrency Primitives — Synchronization Mechanisms at the OS Level](os-concurrency-primitives.md)