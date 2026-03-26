# CPython Runtime Internals

## Overview

CPython, the reference Python implementation, executes bytecode through an interpreter-only design enhanced with runtime profiling and adaptive optimization. Unlike JVMs or .NET runtimes that JIT-compile early, CPython historically relied on bytecode interpretation, though modern versions (3.11+) are adding JIT experimentation. The Global Interpreter Lock (GIL) serializes Python bytecode execution across threads, a fundamental architectural choice that maximizes single-threaded performance but sacrifices parallelism.

## The Global Interpreter Lock

### Purpose & Design

The **GIL** is a mutex protecting the entire Python runtime. Only one OS thread can execute Python bytecode at a time:

```c
// Pseudocode
while (true) {
  Acquire(GIL);
  Execute one bytecode instruction;
  Release(GIL);
  Check for thread switches;
}
```

Why? CPython uses **reference counting** (not tracing GC) for memory management. Reference counting requires atomic increments/decrements on every object reference. Protecting the entire heap with a single lock is simpler than fine-grained locking on each object.

### Implications

| Scenario | Behavior |
|----------|----------|
| Single thread running | Full CPU utilization, high throughput |
| N threads, CPU-bound | Only 1 thread runs; others blocked. No speedup. |
| N threads, I/O-bound | All threads run; GIL released during I/O syscalls |
| C extensions | Native code can release GIL if thread-safe |

Threaded Python is suitable for I/O-bound workloads (web servers, database queries) but not CPU-bound parallelism. Developers typically use multiprocessing (separate processes, separate GILs) for parallelism.

### PEP 703 & Free-Threading

**PEP 703** (Sam Gross, 2023) proposes removing the GIL by replacing reference counting with **biased reference counting** (tracking thread ownership) and **deferred reference counting** (batching decrements). A Python 3.13 build is being prepared with the GIL optional. This may fragment the ecosystem—extensions must be recompiled for GIL-free.

## Memory Management

### Reference Counting

Every Python object carries a `ob_refcount` field:

```c
struct PyObject {
  Py_ssize_t ob_refcount;  // Reference count
  PyTypeObject *ob_type;    // Type descriptor
};
```

When a reference is created (`x = obj`), `ob_refcount++`. When deleted (`del x` or scope exit), `ob_refcount--`. When `ob_refcount == 0`, the object is freed immediately.

Advantages:
- Immediate reclamation of unused memory (no latency from GC pauses)
- Deterministic cleanup (finalizers run immediately)
- Low memory overhead (one counter per object)

Disadvantages:
- No handling of reference cycles (`a.ref = b; b.ref = a` both have refcount ≥1 forever)
- Atomic refcount updates are expensive in threaded contexts

### Cycle Collector

To handle cycles, CPython runs a **cycle collector** periodically (when total allocation exceeds a threshold):

```python
import gc
gc.collect()  # Force collection
```

The collector is a **mark-and-sweep** algorithm:

1. Walk the object graph marking reachable objects
2. Sweep unmarked (cyclic garbage) objects
3. Deallocate swept objects

By default, the collector runs automatically when `len(tracked_objects) > threshold` (default ~700 for Gen0). Collections pause the interpreter but typically take milliseconds.

## Bytecode Compilation & Interpretation

### Source to Bytecode

```
Python source (.py)
         │
         ▼
      Parser (build AST)
         │
         ▼
      Compiler (walk AST, emit bytecode)
         │
         ▼
   Code object (bytecode + metadata)
         │
      Cache in .pyc
```

The bytecode is **stack-based** (unlike V8's register-based, JVM's hybrid):

```pyc
  1           0 LOAD_CONST               1 (10)
              2 LOAD_CONST               2 (20)
              4 BINARY_ADD
              6 RETURN_VALUE
```

Each instruction pops operands from the stack, pushes results.

### The Main Evaluation Loop

The **eval loop** (in `Python/ceval.c`) is CPython's core — a giant switch statement dispatching bytecode:

```c
while (true) {
  opcode = *pc++;
  switch (opcode) {
    case LOAD_CONST: Push(constants[arg]); break;
    case BINARY_ADD: b=Pop(); a=Pop(); Push(a+b); break;
    ...
  }
}
```

This tight loop is slow compared to compiled code but acceptable for interpreted bytecode. Performance is dominated by:

- Bytecode dispatch overhead (50-70% of runtime for tight loops)
- Type checking at every operation (Python is dynamically typed)
- Memory allocation pressure (small objects allocated frequently)

### Bytecode Caching (.pyc files)

Python caches compiled bytecode in `__pycache__/*.pyc` to avoid recompilation:

```
source_file.py (modified time)
      ↓ (if modified)
 recompile → bytecode → cache in __pycache__/source_file.cpython-311.pyc
      ↓ (if not modified)
 load from __pycache__
```

Timestamps are checked; if source is newer than .pyc, recompilation occurs. In Python 3.7+, this is checked via the source's **FROZEN_EXCLUDES** list to handle edge cases.

## Frame Objects & Execution State

### Frame Structure

When code executes, CPython creates a **frame** (execution context):

```c
struct PyFrameObject {
  PyObject *f_back;        // Previous frame (for traceback)
  PyCodeObject *f_code;    // Bytecode object
  PyObject **f_localsplus; // Local variables + cell variables
  PyObject **f_valuestack; // Evaluation stack
  int f_lineno;            // Current line number
};
```

Frames are heap-allocated (`PyMem_Malloc`), causing allocation pressure. Since Python 3.11, frames are "lazily" allocated — the interpreter uses lightweight stack frames and materializes full PyFrameObject only when needed (e.g., for traceback).

### Local Variables & Scope

Local variable access uses bytecode instructions optimized for common cases:

| Instruction | Use | Performance |
|-------------|-----|-------------|
| LOAD_FAST | Local variable | O(1) array index |
| LOAD_GLOBAL | Module-level variable | Hash table lookup |
| LOAD_DEREF | Nonlocal (closure) | Cell object dereference |
| LOAD_ATTR | Attribute access | Type descriptor + cache |

**LOAD_FAST** is fastest because locals are stored in a contiguous array indexed by position.

## Memory Allocators

### CPython's Memory Hierarchy

CPython uses layered allocators:

```
Python object allocation (PyMem_Malloc)
         ↓
    pymalloc (small object arena allocator, <512 bytes)
         ↓
    libc malloc (large allocations)
         ↓
    OS (mmap, brk)
```

**pymalloc** segregates allocations by size into arenas (64KB regions) and pools (4KB blocks), reducing fragmentation for typical Python workloads (lots of 50-200 byte objects).

### Modern Allocators: mimalloc

Python 3.13 introduces **mimalloc** (Microsoft's allocator) as an option:

```bash
./configure --with-mimalloc
```

Mimalloc provides better multi-threaded scaling and lower fragmentation compared to libc malloc, particularly under GIL-free scenarios.

## Performance Evolution (3.11+)

### Adaptive Specialization

Python 3.11 introduced **adaptive specialization** — the interpreter tracks type patterns and specializes bytecode:

```python
def add(x, y):
    return x + y

add(1, 2)      # First call: generic BINARY_ADD
add(3, 4)      # Interpreter logs: + always sees int + int
               # Specializes bytecode to int-optimized operation
```

While not JIT, specialization improves hot paths by narrowing type checks. The runtime can descope specialization if types change (e.g., add called with floats).

### Inlined Function Calls

Python 3.11 started inlining small built-in functions within the eval loop, reducing call overhead.

### PEP 659 & Quickening

**Quickening** (PEP 659) rewrites bytecode on first execution, replacing generic operations with specialized opcodes. This is a form of "rapid JIT" — not native code compilation, but aggressive bytecode optimization without full recompilation.

## Design Philosophy

CPython prioritizes:

1. **Simplicity** — Single-threaded, reference counted, interpreted. Each choice reduces complexity.
2. **Familiarity** — Direct bytecode mapping to Python semantics; easy to trace and debug.
3. **C extension compatibility** — Native extensions' tight coupling to internal structures (refcounts, type objects).
4. **Single-threaded performance** — GIL removed forces trade-offs; optimization in single-threaded context is simpler.

Compared to JVMs or .NET:

- **No JIT by default** — saves compilation overhead and binary size but means slower peak performance
- **Immediate GC via refcounts** — predictable latency but poor multi-threaded scaling
- **Dynamic typing throughout** — forces type checks on every operation

CPython's strength is consistency and simplicity; its weakness is raw performance and parallelism. PyPy (alternative Python VM) adds JIT; Cython (C+Python hybrid) compiles to C for speed. Each trades away some of CPython's simplicity.

## See Also

- Reference counting vs. tracing garbage collection, cycle detection
- Dynamic typing and dispatch mechanisms, attribute lookup caching
- Bytecode interpretation trade-offs, JIT compilation barriers
- Parallelism models: threading vs. multiprocessing