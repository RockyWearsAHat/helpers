# Debugging Race Conditions — Data Races, Deadlocks, and Concurrency Pathologies

**Concurrency bugs** are among the hardest to diagnose because they're non-deterministic: the same code may execute correctly 99 times and fail on the 100th when thread scheduling happens to interleave differently. Unlike deterministic bugs, race conditions often disappear when you add instrumentation to investigate them (a phenomenon called **Heisenbug**).

## Categories of Concurrency Pathologies

### Data Races

A **data race** occurs when two threads access the same memory location concurrently, at least one access is a write, and there's no synchronization preventing the race. Data races are undefined behavior in languages like C++, Java, and Go. The CPU may reorder operations, the compiler may optimize away synchronization, or reads may return torn/inconsistent values.

Example (C):
```c
int x = 0;
// Thread 1: x = 1;
// Thread 2: int y = x;
// Race: Thread 2 might read 0 or 1 depending on scheduling.
```

Data races are insidious because they're invisible without special tools. Adding a `printf()` to debug often eliminates the race by changing timing.

### Deadlocks

A **deadlock** occurs when two threads hold locks in conflicting orders, each waiting forever for the other to release:

```python
# Thread 1         # Thread 2
lock_A.acquire()  lock_B.acquire()
lock_B.acquire()  lock_A.acquire()
# both wait forever; neither can proceed
```

Deadlocks freeze the system; affected threads stop responding. They're deterministic (given the same timing and input) but hard to debug because the actual locking order depends on thread scheduling.

### Livelocks

A **livelock** resembles a deadlock but the threads keep running:

```python
# Thread 1                # Thread 2
while True:              while True:
  if lock.is_held():       if lock.is_held():
    wait()                   wait()
  else:                    else:
    lock.acquire()         lock.acquire()
# Both check, both see it free, both try to acquire
# But only one win each time; the other retries
# System looks active but makes no progress
```

Livelocks are harder to detect than deadlocks because the process isn't frozen; CPU usage may be high but the program doesn't advance.

### Priority Inversion

Occurs in real-time systems when a low-priority thread holds a lock that a high-priority thread needs:

```
High-priority thread: waiting for lock held by Low-priority thread
Low-priority thread: preempted by Medium-priority thread
Result: High-priority work is blocked by medium-priority work
```

Classic example: Mars Pathfinder failure (1997). The VxWorks OS's priority inversion caused telemetry loss until a software patch enabled priority inheritance (high-priority thread briefly inherits low-priority thread's priority while holding the contested lock).

### ABA Problem

A thread reads a memory location (value A), gets preempted, resumes, and finds the location changed to B then back to A. The thread assumes nothing happened, but in reality, critical changes occurred:

```c
// Thread 1: reads x = A
// (preempted)
// Thread 2: changes x from A to B to A
// Thread 1: resumes, sees x = A, assumes it's unchanged (WRONG)
```

Common in lock-free data structures using compare-and-swap (CAS).

## Detection Tools

### ThreadSanitizer (TSan)

**ThreadSanitizer** is a dynamic race detector integrated into Clang/GCC and supported on Linux, macOS, Windows, and Android. It works by instrumenting memory accesses at compile time and tracking access patterns at runtime.

**Enabling (C/C++):**
```bash
clang -fsanitize=thread -g my_program.c -o my_program
./my_program
```

**Enabling (Go):**
```bash
go test -race ./...
go run -race main.go
```

ThreadSanitizer catches data races but not deadlocks or livelocks. It typically adds 2-20× runtime overhead and 5-10× memory overhead, making it unsuitable for production but ideal for tests and staged environments.

**Output example:**
```
WARNING: ThreadSanitizer: data race (pid=12345)
  Write of size 4 at 0x7b6a0000eff0 by thread T1:
    #0 increment_counter counter.c:10 (my_program+0x4a6c)
  Previous read of size 4 at 0x7b6a0000eff0 by thread T0:
    #0 check_counter counter.c:15 (my_program+0x4a7c)
```

### Go Race Detector (based on ThreadSanitizer)

Go's built-in race detector uses ThreadSanitizer under the hood. Unlike manual instrumentation tools, it's trivial to enable:

```bash
go test -race ./...
go run -race server.go
```

The Go race detector catches data races during execution. It won't find races in code paths that don't execute during your test, so comprehensive testing is essential.

**Limitations:** Detects only data races actually executed. Races in error paths or rare conditions won't be caught unless tests exercise them.

### Rust's Ownership System

Rust's **type system prevents data races at compile time**:

```rust
// This won't compile:
let x = 5;
let t1 = std::thread::spawn(|| x = 10);  // error: can't move x
let t2 = std::thread::spawn(|| println!("{}", x));

// Correct: use Arc (atomic reference count) for shared ownership
use std::sync::Arc;
let x = Arc::new(std::sync::Mutex::new(5));
let x1 = Arc::clone(&x);
let t1 = std::thread::spawn(move || *x1.lock().unwrap() = 10);
```

Rust guarantees that if code compiles, there are no data races. (Unsafe blocks can create races, but they must be explicitly marked and reviewed.)

## Debugging Strategies

### Deterministic Replay

Reproduce the buggy interleaving reliably. Techniques:

1. **Fixed random seed:** If concurrency relies on work distribution, seed the random number generator identically across runs
2. **Record and replay:** Log thread interactions during a failure, then replay the exact sequence in test
3. **Timing barriers:** Insert synchronization to force specific thread orderings (careful: this may hide the race!)

Tools like **rr** (record and replay) for Linux can record a multi-threaded program's execution and replay it deterministically in GDB:

```bash
rr record ./my_program
rr replay
# In GDB: step through the exact execution that triggered the bug
```

### Causality-Aware Logging

Add timestamped logs that capture ordering without adding much synchronization overhead:

```python
import logging
import threading
import time

logging.basicConfig(
    format='%(asctime)s [%(threadName)s] %(message)s',
    level=logging.DEBUG
)

def worker():
    logging.debug("Acquiring lock")
    with lock:
        logging.debug("Lock acquired")
        time.sleep(0.1)
        logging.debug("Releasing lock")
```

The key: use a centralized, thread-safe logger. Avoid inline `print()` statements, which can race.

For distributed systems, use **correlation IDs**: inject a unique ID into each request and propagate it across service boundaries so logs from related operations can be correlated.

### Stress Testing

Data races often require specific thread interleavings to manifest. Stress testing increases the likelihood by running the buggy code repeatedly under load:

```bash
# Run a test 10,000 times with -race enabled
for i in {1..10000}; do go test -race ./... || break; done
```

Combine with **thread pool injection** (vary the number of threads, wake up times, etc.) to explore different scheduling scenarios.

### Testing with Conditional Breakpoints

In an IDE debugger with thread support:

1. Set a conditional breakpoint in the critical section: `thread1_counter != thread2_counter`
2. Run under ThreadSanitizer or with explicit lock instrumentation
3. When the condition triggers, inspect thread state

Most debuggers (GDB, LLDB, Visual Studio) support per-thread breakpoints and thread step commands to manually control scheduling.

## Prevention and Safe Patterns

### Immutability

Immutable data never races:

```rust
// Rust enforces this at compile time, but any language benefits:
let config = ImmutableConfig::load("config.json");
// spawn threads that read config; no locks needed
```

### Message Passing

Instead of sharing mutable state, pass data between threads:

```go
// Instead of: shared_map with locks
// Use: channels
done := make(chan struct{})
go func() {
    results := calculate()
    done <- results  // send once; main receives
}()
result := <-done
```

Message passing eliminates shared mutable state entirely, making synchronization bugs impossible.

### Actor Model

Erlang and similar actor-based systems run independent actors that communicate only via message-passing. No shared state → no data races.

### Principle: Minimize Critical Sections

Lock the smallest scope of code for the shortest time:

```python
# BAD: huge critical section
with lock:
  data = expensive_io_call()  # 1 second
  result = process(data)      # 2 seconds
  shared_state.update(result) # 10ms that actually need the lock

# GOOD: critical section is tiny
data = expensive_io_call()
result = process(data)
with lock:
  shared_state.update(result)  # only this 10ms is locked
```

Smaller critical sections reduce lock contention and lower the chance of races triggered by specific timing.

### Monitor for Lock Ordering

Document and enforce a total lock order. If code must acquire both Lock A and Lock B, always do so in that order:

```python
# Define: LOCK_ORDER = [LOCK_A, LOCK_B]
# Always acquire in this order across the entire codebase
def deadlock_free_operation():
  with LOCK_A:
    with LOCK_B:
      # safe; everyone acquires A before B
      critical_section()
```

Use static analysis to check for violations:

```bash
clang-tidy -checks="misc-static-assert" my_program.cpp
```

## Language-Specific Pathologies

**Python:** The Global Interpreter Lock (GIL) prevents true data races in CPython but enables deadlocks if threads acquire multiple locks in conflicting orders.

**Java:** Memory visibility guaranteed by the Java Memory Model (within-thread and across-thread ordering) but races still possible if two threads read/write without synchronization. Tools: FindBugs, Checker Framework.

**Go:** Goroutines are multiplexed (many per OS thread), making scheduling non-deterministic. The race detector catches actual races but misses races in unexecuted code paths.

## Debugging Production Deadlocks

A frozen application often points to deadlock. Techniques:

1. **Thread dump:** Send a signal (SIGABRT in C, thread dump in Java) to capture all thread stacks
2. **Lock graph:** Inspect which threads hold locks and which are waiting; look for cycles
3. **Timeout-based recovery:** Auto-detect stuck threads and terminate gracefully (aggressive but prevents cascading failure)

A single well-designed code review checklist (lock order validation, critical section minimization, exhaustive test coverage with -race or TSan) prevents most concurrency bugs from reaching production.