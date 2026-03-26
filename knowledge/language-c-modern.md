# Modern C — C11 to C23 Features, Undefined Behavior, and Safe Practices

## Overview

C11 (2011) introduced threads, atomics, and type-generic programming. C17 brought minor alignments and minor features. C23 (2024) added static typing features, new keywords (`typeof`), and practical improvements for real-world code. Yet C's fundamental challenge remains: powerful and fast, but unsafe. Modern C practices acknowledge this tension and devise discipline to mitigate footguns.

## Core Language Features Since C11

### _Generic: Type-Safe Polymorphism

C has no overloading or templates. `_Generic` enables selection of code at compile time based on argument type:

```c
#define max(a, b) _Generic((a) + (b), \
    int: max_int, \
    double: max_double, \
    float: max_float \
)(a, b)
```

This achieves type-safe polymorphism without macros or casting. C23 simplified syntax: `_Generic` is less verbose.

### _Noreturn and Function Attributes

`_Noreturn` signals that a function never returns:

```c
_Noreturn void panic(const char *msg) {
    fprintf(stderr, "PANIC: %s\n", msg);
    exit(1);
}
```

Compilers use this for optimization (unreachable code elimination) and static analysis (detecting control flow issues). C23 adds `[[noreturn]]` attribute syntax, more readable but equivalent.

### _Static_assert: Compile-Time Validation

```c
_Static_assert(sizeof(int) == 4, "int must be 32-bit");
```

Validates invariants at compile time. C23 shortens to `static_assert`.

### _Thread_local: Thread-Local Storage

Each thread gets its own copy:

```c
_Thread_local int errno_copy;  // Each thread has its own copy
```

Used for thread-local error codes, global state that must be per-thread, and avoiding locks for read-heavy thread-local data. Performance: zero overhead vs. locks, but incurs TLS register overhead on some architectures.

### Anonymous Structs and Unions (C11)

```c
typedef struct {
    int x, y;
    union {
        struct { char r, g, b, a; };  // Anonymous struct
        uint32_t color;
    };
} Pixel;
```

This enables `p.r` (via anonymous union) and `p.color` (direct union field) without naming intermediate types. Convenient but reduces readability for unfamiliar code.

## C11 Atomics: Synchronization Without Locks

### Atomic Operations and Memory Ordering

```c
#include <stdatomic.h>

_Atomic(int) x = 0;
atomic_store_explicit(&x, 1, memory_order_release);
int val = atomic_load_explicit(&x, memory_order_acquire);
```

Atomics guarantee all-or-nothing updates to shared variables across threads. **Memory ordering** controls synchronization guarantees:

- `memory_order_relaxed`: No synchronization, just atomicity (rarely useful)
- `memory_order_acquire` / `memory_order_release`: One-way synchronization (threads can't reorder operations around the atomic)
- `memory_order_acq_rel`: Full bidirectional synchronization
- `memory_order_seq_cst`: Sequentially consistent (default, most intuitive, slight performance cost)

### When Atomics Replace Locks

Atomics are faster than locks for simple operations (counters, flags):

```c
typedef struct {
    _Atomic(int) count;
} Counter;

void increment(Counter *c) {
    atomic_fetch_add_explicit(&c->count, 1, memory_order_relaxed);
}
```

vs. locked equivalent:

```c
void increment(Counter *c) {
    pthread_mutex_lock(&c->lock);
    c->count++;
    pthread_mutex_unlock(&c->lock);
}
```

Atomic version: no context switch, no lock contention. But atomics don't protect compound operations (increment-then-check). For complex critical sections, locks remain necessary.

### The Memory Model and Witness Traces

Atomics rely on C11's memory model: operations on non-atomic variables are reordered by the compiler unless constrained by atomic operations or volatile. Two threads, no synchronization:

```c
int x = 0, y = 0;

// Thread 1:
x = 1;
y = 1;

// Thread 2:
if (y == 1) assert(x == 1);  // FAILS! assert can trigger
```

Thread 2 might observe `y = 1` but still see `x = 0` if the compiler reordered writes. Synchronize with atomics or volatile accesses.

## C23 Additions

### typeof and decltype: Type Aliasing Without Typedef

C23 adds `typeof`:

```c
int x = 5;
typeof(x) y = 10;  // y is int
typeof(1.0 + 2) z = 3.5;  // z is double (type of 1.0 + 2)
```

Compilers support `typeof` as an extension since GCC 3.x, but C23 standardizes it. Useful for generic macros without explicitly naming types.

### _BitInt: Arbitrary-Precision Integers

```c
_BitInt(128) large = 1_000_000_000_000ULL;
```

Enables 128-bit, 256-bit, or arbitrary-bit integers as first-class types. Slower than fixed-width types but necessary for cryptography and big integer arithmetic. Implementation varies; not all compilers support this fully.

### #embed: Compile-Time Resource Embedding

```c
const unsigned char favicon[] = {
    #embed "favicon.ico"
};
```

Instead of writing binary data as hex escape sequences, `#embed` reads a file at compile time and generates the byte array. Cleaner than source generation through external scripts.

### constexpr and Compile-Time Computation

C23's `_Bool constexpr` (full syntax `_Bool const volatile constexpr`) enables functions evaluated at compile time when all arguments are compile-time constants:

```c
constexpr int fib(int n) { return n <= 1 ? n : fib(n-1) + fib(n-2); }
int arr[fib(10)];  // Array size computed at compile time
```

Limited compared to C++'s `constexpr`—only arithmetic operations, no heap allocation—but eliminates runtime overhead for compile-time derivations (e.g., Fibonacci, bit manipulation).

## Undefined Behavior: The Roadblock to Correctness

### Classes of UB

**Buffer overflows**: `int arr[10]; arr[15] = x;` writes past array bounds. Behavior is undefined; the program may crash, silently corrupt memory, or appear to work.

**Integer overflow in signed types**: `int x = INT_MAX; x++;` is undefined (not defined to wrap like unsigned). Compilers may optimize assuming it doesn't happen.

**Use-after-free**: Dereferencing freed pointers. The memory may be reused; the program may crash; or the access may succeed and corrupt something else.

**Uninitialized variables**: Reading uninitialized stack variables is undefined. Optimizers assume they're never read and eliminate initialization.

**Data races**: Concurrent memory access without synchronization. The memory may observe torn reads, reordered writes, or any intermediate state.

### Speculative Fixes

1. **Static analysis**: Clang Static Analyzer, `clang -fsanitize=undefined` catches many UB issues at runtime
2. **Valgrind and Sanitizers**: Instrumentation detects memory leaks, use-after-free, buffer overflows
3. **Memory-safe subsets**: Using safe string functions (`strncpy` instead of `strcpy`), bounds checking, or language extensions

### The Reality: UB as a Compiler Optimization Lever

Compilers treat UB as "the program will never do this," so they optimize aggressively. A seemingly innocent optimization can trigger UB, causing bizarre behavior:

```c
// Compiler optimizes:
if (x != 0) y = 1000 / x;  // x can't be 0, so no check needed
// But if x is used-after-free and is 0, divide by zero is undefined
```

## Safe C Practices

### Prefer Fixed-Width Types

```c
#include <stdint.h>

uint32_t x;  // Always 32-bit, not "unsigned int" (size varies)
int16_t y;   // Always 16-bit
```

Portable code avoids assumptions about `int` size.

### Bounds Checking and Safe String Functions

C11 optional Annex K provides safer functions:

```c
#include <string.h>

errno_t strncpy_s(char *dest, rsize_t dstsize, const char *src, rsize_t n);
```

Not widely adopted (Microsoft VC++, some others), but the practice is sound: always specify buffer sizes.

### Avoid Macros for Extensibility

Classic mistake:

```c
#define MAX(a, b) ((a) > (b) ? (a) : (b))
MAX(++i, 10)  // i is incremented TWICE!
```

Use `_Generic` or inline functions instead. Inline functions are simpler, safer, and don't have macro pitfalls.

### Use const and restrict Qualifiers

```c
void copy(const char *restrict src, char *restrict dest, size_t n);
```

`const` documents immutability; `restrict` tells the optimizer that src and dest don't overlap. Improves compiler analysis and optimization.

### Minimize Pointer Casting

Pointer casts silence compiler warnings but don't fix the underlying type mismatch. If you need a cast, question whether the design is wrong:

```c
void *data;
int *int_ptr = (int *)data;  // Is data really int array?
```

Better: use correct types or opaque typedef'd pointers.

## Comparison to C++

### Size and Complexity

C remains simpler: fewer keywords, smaller standard library, no generics or templates (until C23's limited `_Generic`). C++ is a superset offering classes, templates, and higher abstraction. C is suitable for embedded and systems programming where code size matters; C++ for larger applications where abstraction justifies complexity.

### Undefined Behavior in Both

Both C and C++ have UB. C++'s type system catches more errors at compile time (templates, templates specialization), but the fundamental unsafe operations (pointer dereference, unchecked cast) remain.

### When to Use C

- Embedded systems with memory constraints
- Performance-critical kernels
- Interfaces requiring C ABI compatibility (most system libraries)
- Projects where simplicity outweighs abstraction benefits

## See Also

- [Memory Management and Allocation Strategies](memory-management.md)
- [Concurrency Patterns and Lock-Free Programming](algorithms-concurrency.md)
- [C++ Conventions and Idioms (Modern C++20/23)](language-cpp.md)