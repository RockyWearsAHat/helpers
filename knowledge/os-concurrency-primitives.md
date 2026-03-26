# Concurrency Primitives — Synchronization Mechanisms at the OS Level

## Overview

**Concurrency primitives** are kernel and hardware abstractions that coordinate multiple threads or processes accessing shared resources. Mutexes, semaphores, condition variables, spinlocks, and atomic operations form the foundation of multi-threaded programming. Understanding their semantics, performance characteristics, and limitations is essential for building reliable concurrent systems without race conditions, deadlocks, or data corruption.

## Mutual Exclusion Locks (Mutexes)

A **mutex** (mutual exclusion lock) ensures only one thread holds the lock at a time. A critical section guarded by a mutex is executed by at most one thread:

```
pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;

// Thread A
pthread_mutex_lock(&lock);     // Blocks if locked; atomically acquires
  // Critical section: modify shared_counter
  shared_counter++;
pthread_mutex_unlock(&lock);   // Release; wake waiting threads

// Thread B (blocked until A unlocks)
pthread_mutex_lock(&lock);     // Now acquires
  shared_counter++;
pthread_mutex_unlock(&lock);
```

**Properties:**
- **Ownership**: Locked by one thread, unlocked by the same thread (owner semantics)
- **Atomicity**: Acquire is atomic — only one thread can succeed
- **Fairness**: Kernel maintains a queue of waiters; order varies (usually FIFO, but not guaranteed)
- **Blocking**: Waiting threads sleep, not busy-wait (low CPU overhead)

**Lock types:**
- **Non-recursive (standard)**: Thread attempting to re-acquire its own lock deadlocks
- **Recursive (reentrant)**: Same thread can acquire the lock multiple times; unlock must be called equal times

## Semaphores

A **semaphore** is a synchronization object with an integer count and two atomic operations: `wait()` (decrement, block if ≤ 0) and `signal()` (increment, wake a waiter):

```
Semaphore sem = 3;  // Allows 3 concurrent accesses (trivial pool)

// Thread A
sem_wait(&sem);     // count=3 → count=2, proceed
  // Access critical resource
sem_post(&sem);     // count=2 → count=3, wake waiter if any

// Thread B (concurrent, count allows it)
sem_wait(&sem);     // count=2 → count=1, proceed
  // Access critical resource
sem_post(&sem);     // count=1 → count=2

// Thread C (if count=0)
sem_wait(&sem);     // count=0 → block until post() called
```

**Variants:**
- **Binary semaphore** (count ≤ 1): Acts like a mutex but without owner semantics (any thread can post, not just the acquirer)
- **Counting semaphore** (count ≥ 0): Represents resource availability (e.g., 5 database connections)

**Unlike mutexes:** No ownership. Thread A can `post()` a semaphore acquired by thread B. Used for signaling and resource pools.

## Condition Variables

A **condition variable** allows threads to wait for a specific condition and be woken when it changes. Always used with a mutex to protect the condition:

```
pthread_cond_t cond = PTHREAD_COND_INITIALIZER;
pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
int buffer_full = 0;

// Producer thread
pthread_mutex_lock(&lock);
  while (buffer_full) {
    pthread_cond_wait(&cond, &lock);  // Atomically: unlock, sleep, re-lock on wake
  }
  // Fill buffer
  buffer_full = 1;
  pthread_cond_signal(&cond);          // Wake one waiter
pthread_mutex_unlock(&lock);

// Consumer thread
pthread_mutex_lock(&lock);
  while (!buffer_full) {
    pthread_cond_wait(&cond, &lock);
  }
  // Consume buffer
  buffer_full = 0;
  pthread_cond_signal(&cond);
pthread_mutex_unlock(&lock);
```

**Semantics:**
- `wait()`: Atomically unlock the mutex, sleep, and re-acquire the mutex on wake (or timeout)
- `signal()`: Wake one waiting thread (no guarantee which)
- `broadcast()`: Wake all waiting threads

**Critical pattern**: Always use a `while` loop, not `if`. Between wakeup and re-acquiring the lock, another thread may consume the condition:

```
// WRONG:
if (!item_available) cond_wait();
consume_item();  // May not exist; another thread consumed it

// CORRECT:
while (!item_available) cond_wait();  // Re-check after wakeup
consume_item();
```

## Read-Write Locks (Shared/Exclusive Locks)

**Read-write locks** allow multiple readers simultaneously, but only one writer (and no readers during write):

```
Reader thread A: acquires read lock   → proceeds
Reader thread B: acquires read lock   → proceeds (concurrent)
Writer thread:   waits for all readers to release

Once writer acquires lock:
Reader thread C: waits until writer releases

Writer releases: all readers (including C) may proceed
```

```
pthread_rwlock_t lock = PTHREAD_RWLOCK_INITIALIZER;

// Reader
pthread_rwlock_rdlock(&lock);  // Multiple readers allowed
  // Read shared_state
pthread_rwlock_unlock(&lock);

// Writer
pthread_rwlock_wrlock(&lock);  // Exclusive; blocks readers
  // Modify shared_state
pthread_rwlock_unlock(&lock);
```

**Trade-off**: Reader-friendly when reads heavily outnumber writes. Writer-hungry workloads can starve writers (reader priority) or readers (writer priority depending on implementation).

## Spinlocks

**Spinlocks** are busy-waiting locks: an acquiring thread loops repeatedly, polling the lock state. No sleep:

```
volatile int lock = 0;

// Acquire spinlock
while (atomic_test_and_set(&lock)) {
  // Busy-wait: consume CPU, no context switch
}

// Critical section
...

// Release
atomic_clear(&lock);
```

**Why use spinlocks?**
- **Very short critical section**: Less overhead than mutex (context switch + wake-up latency)
- **High-priority real-time**: Predictable latency
- **Kernel code**: Can't sleep (e.g., interrupt handlers)

**Why avoid spinlocks?**
- **Wastes CPU**: Polling consumes cycles while spinning
- **Priority inversion**: Low-priority thread holds lock while high-priority thread spins
- **Unfair**: No queueing; last thread to check might acquire (if not using atomic primitives carefully)

Modern kernels use **adaptive spinlocks**: spin briefly, then sleep if still contested.

## Futexes (Fast Userspace Mutexes, Linux)

**Futex** combines fast userspace spinning with kernel-level blocking. Most operations avoid kernel calls:

```
Futex implementation (Linux-specific):
  struct futex {
    volatile int value;  // User-space; no syscall needed for uncontended case
  };

  futex_wait(futex, expected_value):
    if (futex->value != expected_value) return immediately
    else syscall into kernel, sleep on queue

  futex_wake(futex, num_to_wake):
    If no contention, wake `num_to_wake` waiters without syscall
    Else syscall to wake from kernel queue
```

Example (pseudo-code):
```
// Acquire
while (!atomic_compare_and_set(&futex, 0, 1)) {
  futex_wait(&futex, 1);  // Syscall only if truly contested
}

// Release
atomic_set(&futex, 0);
futex_wake(&futex, 1);    // Syscall to wake waiter
```

**Advantage**: Uncontended case is pure userspace (~10 cycles); contended case uses kernel scheduler. Most performance-critical mutexes (pthreads on modern Linux) use futexes internally.

## Barriers

A **barrier** synchronization primitive prevents threads from proceeding past a point until all have arrived:

```
pthread_barrier_t barrier;
pthread_barrier_init(&barrier, NULL, 3);  // Wait for 3 threads

// Thread A
... do work ...
pthread_barrier_wait(&barrier);  // Block until all 3 arrive
... continue ...

// Thread B
... do work ...
pthread_barrier_wait(&barrier);  // Block until all 3 arrive
... continue ...

// Thread C
... do work ...
pthread_barrier_wait(&barrier);  // Block until all 3 arrive
... continue ...

// All proceed in lockstep past barrier
```

Used in parallel algorithms (e.g., matrix multiply where each phase depends on all threads completing). One thread returns a special status (e.g., `PTHREAD_BARRIER_SERIAL_THREAD`) allowing it to perform aggregate work.

## Atomic Operations & Compare-And-Swap (CAS)

**Atomic operations** execute indivisibly; no other thread can observe an intermediate state. Provide the foundation for lock-free data structures.

```
Atomic increment:
  atomic_increment(&counter);  // Equivalent to counter++, but indivisible
  
Compare-and-swap (CAS):
  bool result = atomic_cmp_xchg(&mem, expected, new_value);
  // If *mem == expected: *mem = new_value, return true
  // Else: return false, *mem unchanged
  
  // Usage: optimistic lock-free update
  do {
    old_val = atomic_read(&shared);
    new_val = old_val + 1;
  } while (!atomic_cmp_xchg(&shared, old_val, new_val));
```

**Hardware support**: Modern CPUs provide `CMPXCHG` (x86), `CAS` (ARM), `LLD/SCD` (MIPS) instructions executing atomically with memory barriers.

## Memory Ordering & Memory Barriers

Atomic operations alone don't ensure all threads see updates in the expected order. **Memory barriers** enforce ordering:

```
// Without barrier (may reorder):
shared_state = 42;
flag = 1;  // Another thread might see flag=1 before state=42

// With barrier:
shared_state = 42;
atomic_store_release(&flag, 1);  // Release barrier: all prior stores visible
// Another thread:
if (atomic_load_acquire(&flag)) {  // Acquire barrier
  // Guaranteed to see state=42
}
```

**Barrier types:**
- **Release**: Prior stores/loads must not move past this point
- **Acquire**: Subsequent loads/stores must not move before this point
- **Full barrier (Sequential Consistency)**: Strict ordering; most expensive

Linux provides `smp_*mb()` macros; C11 `<stdatomic.h>` offers `memory_order_*` enum.

## Lock-Free Data Structures & The ABA Problem

**Lock-free** algorithms use only atomic operations and memory barriers, no locks. Threads never block:

```
Lock-free queue (simplified):
  CAS-based push/pop; no mutex held

  push(x):
    do {
      old_head = read(&head);
      node = new_node(x, old_head);
    } while (!CAS(&head, old_head, node));

  pop():
    do {
      old_head = read(&head);
      if (!old_head) return NULL;
    } while (!CAS(&head, old_head, old_head->next));
    return old_head;
```

### The ABA Problem

A subtle race condition in lock-free code:

```
Thread A:
  1. Read value = X (pointing to node A)
  2. Compute new value = node B
  3. CAS(&ptr, A, B)

Thread B (concurrent):
  1. Remove node A from relevant structures
  2. Delete node A (memory freed)
  3. Allocate new node at the same old address (now called C, but same address)

Thread A resumes:
  The CAS at step 3 compares old address == old address (coincidentally!)
  CAS succeeds, but now ptr → C (a different node), not B as intended
```

**Solution**: Use versioned pointers (tag CAS with counter incrementing on every CAS) or hazard pointers (safe memory reclamation).

## Deadlock & Priority Inversion

**Deadlock**: Mutual waiting forever. Example: Thread A holds M1, waits for M2; Thread B holds M2, waits for M1.

**Priority inversion**: Low-priority thread holds lock; high-priority thread waits. CPU runs low-priority thread, delaying high-priority. Real-time systems can suffer: high-priority task blocked by low-priority background thread. **Solution**: Priority inheritance — lock holder temporarily inherits waiter's priority.

## See Also

- [OS Process Management](os-process-management.md) — Process scheduling, context switching overhead
- [Concurrency Patterns](concurrency-patterns.md) — High-level patterns (Actor model, async/await)
- [Systems Reasoning](systems-reasoning.md) — Why synchronization matters