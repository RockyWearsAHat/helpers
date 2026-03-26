# Rust Async: Futures, Executors, and Structured Concurrency

## Introduction

Async in Rust models asynchronous I/O through **Futures** — lazy computations that complete when readied. Unlike threads, async tasks are lightweight cooperatively-scheduled, enabling thousands to run concurrently on few OS threads. Understanding Futures, `Pin`, and executor models is essential to working with Rust's async ecosystem.

## The Future Trait

A `Future` is the core abstraction:

```rust
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}

pub enum Poll<T> {
    Ready(T),
    Pending,
}
```

When polled, a Future returns either `Poll::Pending` (not ready yet, task will be woken later) or `Poll::Ready(value)`. Unlike threads that sleep until unblocked by the OS, Futures must be **explicitly woken** via a `Waker`, which is passed in `Context`. This push-based notification model reduces CPU overhead compared to busy-wait or exhaustive polling.

## Pin and Unpin

`Pin` wraps a pointer and prevents moving the referent without unsafe code. It exists to enforce self-referential structures in async code.

**Why?** Futures may contain references to their own fields:

```rust
data: Vec<u8>,
ptr: *const u8,  // points into data field
```

If `Vec` is moved in memory, the pointer becomes dangling. Async code that suspends in the middle of a Future's method must ensure the Future location is stable during suspension.

**Pin Guarantees:**
- `Pin<&mut T>` means T cannot be moved (safe to rely on addresses)
- `Pin<&T>` provides immutable access; interior mutability can bypass Pin

**Unpin Marker:**
- Types implementing `Unpin` are safe to move even through Pin (most types: primitives, most standard structs)
- Types *not* implementing `Unpin` (self-referential, or containing non-Unpin fields) prevent casual moves, signaling care is required

In practice, `Pin` enforcement is mostly compile-time fencing that prevents accidental violations in safe code. Most futures are implicitly `!Unpin` when they chain `.await`, and tooling handles it automatically.

## Async/Await Desugaring

`async fn` is sugar for a function returning a Future:

```rust
async fn foo(x: i32) -> String {
    bar().await;
    format!("result: {}", x)
}

// Roughly desugars to:
fn foo(x: i32) -> impl Future<Output = String> {
    async move {
        bar().await;
        format!("result: {}", x)
    }
}
```

Each `.await` point becomes a poll call in the state machine. The async runtime polls the Future repeatedly until it yields `Ready`. Temporary values across `.await` boundaries must be stored in the Future struct's state — hence large stack-like state growth for complex flows.

## Executors and Runtimes

An **executor** repeatedly polls Futures until completion. It:
1. Maintains a queue of ready tasks
2. Polls each task
3. Stashes tasks that return `Pending` with their Waker
4. When Waker fires (e.g., I/O ready), re-queues the task

**Tokio** is the dominant executor:
- Multi-threaded runtime (configurable work-stealing scheduler)
- Built-in async I/O (socket, file, timer bindings)
- Task spawning: `tokio::spawn()` returns a `JoinHandle`
- **Note:** Tokio uses `!Unpin` futures by default; code treating futures as movable will fail

**Alternatives:**
- **async-std** — lower-level, closer to stdlib
- **smol** — minimal, embeddable executor (~200 lines)
- **embassy** — for embedded/no-std async

Choosing an executor is runtime dependency; once picked, you're locked in (Tokio spawned tasks won't work on smol).

## Task Spawning and Lifetimes

```rust
tokio::spawn(async {
    println!("background task");
});

// Captured references must be 'static
let data = String::from("hello");
tokio::spawn(async {
    println!("{}", data);  // Error: data not 'static, may be dropped
});

// Solution: move into the task
tokio::spawn(async move {
    println!("{}", data);  // OK: data moved into task
});
```

Spawned tasks must own all captured state (no borrowed references), because they run in the background and may outlive the spawning scope. This is stricter than threads but ensures safety.

## Select and Alternative Operations

`tokio::select!` race multiple Futures, returning the result of whichever completes first:

```rust
tokio::select! {
    val = some_future() => println!("first: {}", val),
    _ = slow_operation() => println!("slow finished"),
}
```

This is crucial for implementing timeouts, cancellation, and fan-in patterns. Futures not selected are **dropped**, so resources (e.g., spawned tasks) must be explicitly cleaned up.

For waiting on all Futures, use `futures::join!` or collect into a vector and poll with `select_all()`.

## Structured Concurrency and Cancellation

Rust async lacks a native TaskGroup like Java or Python. Cancel patterns are manual:

```rust
let cancel_token = tokio_util::sync::CancellationToken::new();
let token = cancel_token.clone();

tokio::spawn(async move {
    tokio::select! {
        _ = long_running_work() => { /* ... */ },
        _ = token.cancelled() => { /* ... */ },
    }
});

// Later: trigger cancellation
cancel_token.cancel();
```

Structured concurrency (ensuring all child tasks complete before parent) requires explicit coordination; it's not enforced by the type system like Nurseries in Trio.

## Async Drop Challenges

Futures may own resources that need async cleanup, but `impl Drop` only supports synchronous code:

```rust
struct AsyncResource {
    file: std::fs::File,
    // Wants to flush asynchronously before close
}

// impl Drop can't be async ❌
```

Workaround: Use explicit cleanup or `AsyncDrop` (unstable RFC): manually call cleanup methods in a scope guard or at the end of an async block.

```rust
{
    let resource = acquire_async_resource().await;
    // use resource
    resource.cleanup().await;
}
```

## Waker Mechanics

When an async operation (e.g., socket read) completes, the I/O driver creates a `Waker` and calls `wake()`. This moves the task back to the executor's ready queue. Understanding Waker is essential for implementing custom Futures or low-level I/O:

```rust
struct MyFuture {
    data: i32,
}

impl Future for MyFuture {
    type Output = i32;
    
    fn poll(self: Pin<&mut self>, cx: &mut Context<'_>) -> Poll<i32> {
        let waker = cx.waker().clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(1));
            waker.wake();  // Re-queue task
        });
        Poll::Pending
    }
}
```

The `Waker` is cheaply cloneable and may be stored across suspensions.

## Common Pitfalls

- **Large futures:** Complex async flows build large state machines. Use explicit Fan-Out/Fan-In or split into smaller tasks.
- **Holding locks across await:** Deadlock risk; the lock isn't released during suspension.
- **Sync code blocking:** Expensive computations in async code starve other tasks. Use `tokio::task::spawn_blocking()`.
- **Forgetting to await:** Calling an async fn without `.await` creates a Future that does nothing; typos silently succeed.

## See Also

- **concurrency-patterns** — general async patterns (timeouts, retries, fan-out/fan-in)
- **language-rust** — ownership and borrowing model
- **os-io-models** — blocking vs. non-blocking I/O, reactor pattern