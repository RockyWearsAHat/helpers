# Memory Models — Semantics, Hardware Ordering & Data Races

## Overview

A **memory model** specifies which values a read can observe from a shared memory. Without formal memory models, reasoning about concurrent programs is nearly impossible: different optimizations (compiler reordering, out-of-order CPU execution) can change behavior unpredictably.

Memory models exist at multiple levels:

1. **Language memory model** (C++, Java): specifies when programs are correctly synchronized, what can race
2. **Hardware memory model** (x86-TSO, ARM, POWER): specifies CPU-level ordering guarantees
3. **Compiler model**: when can the compiler reorder operations

## The Safety Boundary: Data Races

A **data race** occurs when two threads access the same memory location without synchronization, and at least one access is a write.

Programs **with data races have undefined behavior** (in C++, Java). The memory model doesn't specify what happens—the program is simply incorrect.

Programs **without data races** have well-defined behavior under their memory model.

Example (data race):

```cpp
int x = 0;
// Thread 1
x = 1;

// Thread 2
int y = x;   // Could read 0 or 1 (race condition)
```

Without synchronization, this is undefined. Correct version:

```cpp
std::atomic<int> x(0);
std::mutex m;

// Thread 1
x.store(1, std::memory_order_release);

// Thread 2
int y = x.load(std::memory_order_acquire);  // Synchronizes, sees x=1
```

## C++11/14/17 Memory Model

C++ formalizes concurrency via `std::atomic` and mutex operations. **Mutexes implicitly synchronize memory.**

### Synchronization Boundaries

- **Mutex lock**: all prior writes become visible to next lock
- **Atomic store**: releases new value
- **Atomic load**: acquires latest value
- **Condition variables**: pair with mutex lock

### Memory Ordering: Four Levels

#### 1. memory_order_relaxed

Atomic operation with no synchronization. Just ensures the operation is atomic (indivisible).

```cpp
std::atomic<int> counter(0);
counter.fetch_add(1, std::memory_order_relaxed);  // No sync!
```

Use: counters where exact count doesn't matter, only approximate increment.

#### 2. memory_order_release (store) / acquire (load)

- **Release store**: visible to next acquire load
- **Acquire load**: sees all writes before the release

```cpp
std::atomic<int> flag(0);
std::atomic<int> data(0);

// Thread 1
data.store(42, std::memory_order_relaxed);
flag.store(1, std::memory_order_release);   // "Release" this data

// Thread 2
if (flag.load(std::memory_order_acquire)) {  // "Acquire" flag
  int x = data.load(std::memory_order_relaxed);  // Guaranteed to see 42
}
```

**Key property**: release/acquire are paired synchronization points. If thread 1's release **happens-before** thread 2's acquire, all prior operations by thread 1 are visible to thread 2.

#### 3. memory_order_acq_rel (RMW)

Read-modify-write operations (compare-and-swap, fetch-add) with both acquire and release semantics. Acts as a full barrier on an object.

```cpp
std::atomic<int> x(0);
// This atomically loads, increments, and stores with acquire/release effects
x.fetch_add(1, std::memory_order_acq_rel);
```

#### 4. memory_order_seq_cst (Sequential Consistency)

Strongest guarantee: all threads see a **global total order** of all seq_cst operations.

```cpp
std::atomic<int> x(0), y(0);

// Thread 1
x.store(1, std::memory_order_seq_cst);

// Thread 2
y.store(1, std::memory_order_seq_cst);

// Thread 3
if (x.load(std::memory_order_seq_cst) == 0 &&
    y.load(std::memory_order_seq_cst) == 0) {
  // Impossible! One of the two stores must be visible to thread 3
}
```

All seq_cst operations form a **total order** that respects program order and causality.

**Trade-off**: seq_cst is easiest to reason about but slowest (requires global barriers, especially on weak architectures).

### Happens-Before Relation

Memory operations form a partial order via happens-before (HB):

- **Program order**: within a thread, operations in execution order HB each other
- **Synchronization**: release-store HB acquire-load on same var
- **Transitivity**: if A HB B and B HB C, then A HB C
- **Mutex**: unlock HB subsequent lock

**Invariant**: if operation A HB operation B, then A's effects are visible to B.

## Java Memory Model

Java's memory model is similar in spirit but uses different terminology (less explicit atomic operations, more reliance on mutexes).

### Key Elements

- **Atomicity**: volatile reads/writes, synchronized methods
- **Visibility**: volatile and synchronized establish visibility barriers
- **Ordering**: synchronized blocks form a total order

### volatile Keyword

A volatile field has release/acquire semantics (similar to C++ `std::atomic`).

```java
class SharedData {
  volatile int data = 0;
  
  // Thread 1
  void update() {
    data = 42;  // Release
  }
  
  // Thread 2
  int read() {
    return data;  // Acquire
  }
}
```

### Synchronized Blocks

Mutex unlock has release semantics; next lock has acquire semantics.

```java
class Counter {
  int count;
  
  synchronized void increment() {
    count++;  // Protected by mutex
  }
}
```

## Hardware Memory Ordering

CPUs provide **weaker** guarantees than languages like C++ or Java. Compiler must insert **memory barriers** to enforce language-level semantics.

### x86-64: Total Store Order (TSO)

x86 is relatively strong: **stores are always in order** (though loads may reorder around stores).

Allowed reorderings:

```c
// CPU may reorder these (load-after-store):
x = 1;
y = load(a);  // Can happen before store to x is visible globally
```

Disallowed:

```c
// CPU never reorders stores:
x = 1;
y = 2;        // y = 2 never visible before x = 1
```

Barrier: `mfence` (memory fence) forces all prior stores visible before subsequent operations.

### ARM: Relaxed Ordering

ARM allows much more reordering than x86. Both loads and stores can be reordered:

```c
x = 1;
y = 2;
a = load(b);
c = load(d);
// Any of these may be visible in any order
```

Barriers: `DMB` (data memory barrier), `ISB` (instruction barrier) enforce strict ordering.

### POWER: Weakest Ordering

POWER (IBM) is even weaker than ARM. Stores to different locations may be visible in different order on different CPUs.

## From Language Model to Hardware

The compiler's job:

1. Analyze which synchronization orders are required (language model)
2. Insert barriers where needed to enforce those orders on target hardware

Example: `std::memory_order_acquire` on ARM requires a `DMB` instruction; on x86, it's often free (x86 is strong enough).

## Happens-Before in Practice

Given two operations A and B:

- **A HB B** → A's effects always visible to B (can safely communicate data)
- **A !HB B** → no guarantee; result depends on scheduling

Example (classic visibility bug):

```cpp
int done = 0;

// Thread 1   
work();
done = 1;      // Regular (non-atomic) write

// Thread 2
while (!done) { }   // Spin-wait for done=1
std::cout << "Work complete";
```

**Race**: thread 2 may see `done=0` indefinitely (compiler may hoist `done` into a register, reading the same stale value every iteration).

**Fix**: use `std::atomic<int>` for `done` (establishes sync-point).

## Data Race as Undefined Behavior

C++ and Java memory models place **all responsibility on the programmer**: if your program has a data race, the model says nothing about behavior.

This is different from, say, a checked exception model (which catches errors at runtime). Memory model violations may crash, hang, or silently corrupt data.

Rationale: the language wants compiler freedom to optimize. If races were defined, the compiler couldn't reorder across them (performance hit).

## Compiler Reordering Constraints

Consider this thread:

```cpp
x = 1;
y = 2;
```

**Without synchronization**, compiler may emit:

```asm
; Could reorder to:
store y, 2
store x, 1
```

**With release semantics** on the second operation, the compiler must ensure ordering:

```cpp
x = 1;
y.store(2, std::memory_order_release);
```

Compiler must emit barriers to prevent reordering:

```asm
store x, 1
mfence (or nothing on x86)
store y, 2
```

## Common Mistakes

### Missing Synchronization

```cpp
// WRONG
int shared = 0;
// Thread 1: shared = 1;
// Thread 2: while (!shared) { }  // Spins forever
```

Thread 2's loop may be optimized to `while (true)`.

### Assuming Sequential Consistency

```cpp
// WRONG: Assumes all threads see stores in order
x = 1;
y = 2;  // Doesn't guarantee anyone sees x before y
```

Use `synchronize-with` edges to establish order.

### Not Understanding acquire/release

```cpp
// WRONG: confusion about pairing
flag.store(1, std::memory_order_release);
// ...later, different thread...
int val = flag.load(std::memory_order_relaxed);  // No sync!
```

Should use acquire on load to pair with release on store.

## See Also

- **Concurrency patterns**: concurrency-patterns.md (higher-level sync primitives)
- **Hardware architecture**: hardware-cpu-architecture.md (OOO execution, caching)
- **Debugging multithreaded code**: tools-debugging.md