# Go Concurrency: Goroutines, Channels, and Patterns

## Introduction

Go's concurrency model is built on three primitives: **goroutines** (lightweight threads), **channels** (typed message passing), and the **`select` statement** (multiplexing). Go treats concurrency as a first-class language feature, not a library. This produces code that scales well and reads naturally compared to callback-based or futures-based systems.

## Goroutines: The M:N Scheduler

A **goroutine** is a lightweight thread managed by the Go runtime. The runtime multiplexes goroutines onto OS threads (M:N scheduling):

```go
go fmt.Println("concurrent task")

go func() {
    result := expensiveComputation()
    fmt.Println(result)
}()
```

Starting a goroutine is as cheap as allocating a few KB of stack (compared to OS threads at ~MB). A single Go program routinely runs millions of goroutines.

**Scheduling Model:**
- Go runtime uses a work-stealing scheduler
- Each P (processor) runs M (OS thread) workers
- Goroutines (G) are queued on P runqueues
- When a goroutine blocks (e.g., on I/O), the runtime parks it and wakes another from the queue
- This is **preemptive at function call boundaries** in Go 1.14+; no true preemption mid-function

**Implication:** A goroutine that never yields (tight loop) can starve others on the same P, though this is rare in real code.

## Channels: Typed Message Passing

A **channel** is a typed pipe for sending and receiving values between goroutines:

```go
// Create an unbuffered channel
ch := make(chan string)

// Send
go func() {
    ch <- "hello"
}()

// Receive (blocks until sent)
msg := <-ch
fmt.Println(msg)  // "hello"
```

**Unbuffered channels** synchronize sender and receiver: send blocks until a receiver is ready, and vice versa. Both goroutines rendezvous.

**Buffered channels** decouple sender and receiver:

```go
ch := make(chan int, 10)  // Buffer 10 ints

ch <- 1
ch <- 2
// Sender does not block until buffer fills
```

**Closing channels:**

```go
close(ch)  // Signal no more sends

// Range over channel until close
for msg := range ch {
    fmt.Println(msg)
}

// Check if closed
val, ok := <-ch
if !ok {
    fmt.Println("channel closed")
}
```

Sending on a closed channel panics. Receiving on a closed channel returns the zero value.

## The Select Statement

`select` multiplexes multiple channel operations, executing whichever is ready first:

```go
select {
case msg := <-ch1:
    fmt.Println("from ch1:", msg)
case msg := <-ch2:
    fmt.Println("from ch2:", msg)
case ch3 <- result:
    fmt.Println("sent to ch3")
default:
    fmt.Println("no operation ready")
}
```

If multiple cases are ready, one is chosen randomly. This non-determinism prevents subtle ordering bugs and encourages robust code.

## The Context Package

`context.Context` is an interface for propagating cancellation, deadlines, and values across goroutines:

```go
import "context"

ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
defer cancel()

select {
case result := <-longOperation():
    fmt.Println(result)
case <-ctx.Done():
    fmt.Println("timeout or cancelled:", ctx.Err())
}
```

**Common context functions:**
- `WithCancel(parent)` — manual cancellation
- `WithTimeout(parent, duration)` — automatic cancellation after duration
- `WithDeadline(parent, time)` — cancellation at a specific time
- `WithValue(parent, key, value)` — attach request-scoped data

Context is the **idiomatic way to implement cancellation and structured concurrency in Go**. Every blocking operation should accept a context.

## Sync Primitives

### Mutex

Protects shared state with mutual exclusion:

```go
type SafeCounter struct {
    mu    sync.Mutex
    count int
}

func (c *SafeCounter) Increment() {
    c.mu.Lock()
    defer c.mu.Unlock()
    c.count++
}
```

Go discourages Mutex-based concurrency in favor of channels: "Share memory by communicating; don't communicate by sharing memory." But Mutexes are necessary for some patterns (caches, atomic state updates).

### WaitGroup

Waits for a group of goroutines to complete:

```go
var wg sync.WaitGroup
for i := 0; i < 10; i++ {
    wg.Add(1)
    go func(id int) {
        defer wg.Done()
        doWork(id)
    }(i)
}
wg.Wait()  // Block until all Done() called
```

### Once

Ensures a function is called exactly once:

```go
var once sync.Once
var result string

once.Do(func() {
    result = expensiveInitialization()
})
// Subsequent calls to Do() are no-ops
```

### Pool

Caches objects to reduce GC pressure:

```go
var bufferPool = sync.Pool{
    New: func() interface{} { return make([]byte, 0, 64) },
}

buf := bufferPool.Get().([]byte)
// use buf
bufferPool.Put(buf)
```

## Concurrency Patterns

### Fan-Out/Fan-In

Distribute work across goroutines, then collect results:

```go
// Fan-out
workers := make([]<-chan Result, numWorkers)
for i := 0; i < numWorkers; i++ {
    workers[i] = startWorker(jobs)
}

// Fan-in: merge results
for result := range merge(workers...) {
    process(result)
}

func merge(results ...<-chan Result) <-chan Result {
    var wg sync.WaitGroup
    out := make(chan Result)
    
    send := func(c <-chan Result) {
        defer wg.Done()
        for r := range c {
            out <- r
        }
    }
    
    for _, c := range results {
        wg.Add(1)
        go send(c)
    }
    
    go func() { wg.Wait(); close(out) }()
    return out
}
```

### Pipeline

Chain stages where each stage sends output to the next:

```go
// Stage 1: generate numbers
numbers := make(chan int)
go func() {
    for i := 0; i < 10; i++ {
        numbers <- i
    }
    close(numbers)
}()

// Stage 2: square
squares := make(chan int)
go func() {
    for n := range numbers {
        squares <- n * n
    }
    close(squares)
}()

// Stage 3: print
for s := range squares {
    fmt.Println(s)
}
```

Pipelines naturally express data flow and backpressure (downstream blocking upstream receives).

### Errgroup

Simplifies error handling across goroutine groups:

```go
import "golang.org/x/sync/errgroup"

g, ctx := errgroup.WithContext(context.Background())

for item := range items {
    item := item  // Local copy for closure
    g.Go(func() error {
        return processItem(ctx, item)
    })
}

if err := g.Wait(); err != nil {
    fmt.Println("error:", err)  // First non-nil error
}
```

## Race Detector

The `go run -race` command detects concurrent access to shared memory:

```bash
go run -race ./main.go
```

It instruments the binary to detect data races (unsynchronized access to shared variables). Use in tests and CI to catch race conditions early. The overhead is ~5-10x slower, so not for production.

## Scheduler Behavior and Fairness

The Go scheduler is fair but not perfectly preemptive. Goroutines are preempted at function calls and on channel operations, but tight CPU-bound loops won't preempt:

```go
// This goroutine can starve others
go func() {
    for {
        // tight loop, no function calls or channels
    }
}()
```

In practice, real code has I/O and function calls, so starvation is rare. But be aware that the scheduler isn't OS-level preemptive.

## Common Pitfalls

- **Goroutine leaks:** Starting goroutines that never exit (waiting on a channel that's never sent to)
- **Channel deadlock:** All goroutines waiting on channels; no one to send
- **Not checking context:** Ignoring `ctx.Done()` in loops; resource waste
- **Closing shared channels:** Only the sender should close; multiple senders cause panics
- **Forgetting value semantics:** `wg.Add(1)` then `go func() { defer wg.Done() ... }()` — don't modify WaitGroup outside the goroutine

## See Also

- **concurrency-patterns** — general async/concurrent design patterns
- **paradigm-concurrent-models** — comparison with actor model, shared memory
- **language-go** — Go idioms and conventions