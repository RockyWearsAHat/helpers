# Memory Management — Stack, Heap, and Everything Between

## The Two Memory Regions

### Stack
```
Fast (pointer bump allocation), LIFO, fixed-size frames
Each function call pushes a frame; return pops it.
Size: typically 1-8 MB per thread (configurable).

What lives here: local variables, function arguments, return addresses.
```

```c
void foo() {
    int x = 42;        // Stack-allocated: fast, automatic cleanup
    char buf[256];      // Stack array: same
    // When foo() returns, x and buf are gone instantly
}
```

**Stack overflow:** Deep recursion or large stack allocations exhaust the fixed-size stack:
```c
void infinite() { char buf[1024]; infinite(); }  // Stack overflow!
```

### Heap
```
Flexible (arbitrary size, arbitrary lifetime), manual or GC-managed
Allocated via malloc/new/Box, freed explicitly or by garbage collector.
Size: limited by available RAM + virtual memory.

What lives here: objects that outlive their creating function, 
                 dynamically-sized data, shared data.
```

```c
void foo() {
    int *p = malloc(sizeof(int) * 1000);  // Heap: survives after foo returns
    // Must free(p) somewhere, or it leaks
}
```

### Stack vs Heap Performance
| Aspect | Stack | Heap |
|--------|-------|------|
| Allocation | ~1 ns (pointer bump) | ~100 ns (find free block) |
| Deallocation | Free (frame pop) | Free list / GC |
| Cache locality | Excellent (contiguous) | Poor (scattered) |
| Thread safety | Thread-local (no sync needed) | Shared (needs sync) |
| Size flexibility | Fixed at compile time | Dynamic |

## Manual Memory Management (C/C++)

### The Four Horsemen of Memory Bugs

**1. Use After Free**
```c
int *p = malloc(sizeof(int));
*p = 42;
free(p);
printf("%d\n", *p);  // Undefined behavior! Memory may be reused
```

**2. Double Free**
```c
free(p);
free(p);  // Undefined behavior! Corrupts allocator metadata
```

**3. Memory Leak**
```c
void leak() {
    int *p = malloc(1024);
    return;  // p is lost, memory never freed
}
```

**4. Buffer Overflow**
```c
char buf[10];
strcpy(buf, "this string is way too long");  // Writes past buf, corrupts stack
```

### C++ RAII (Resource Acquisition Is Initialization)
```cpp
// Bad: manual management
void bad() {
    File* f = open("data.txt");
    process(f);      // What if this throws?
    close(f);        // Skipped on exception → leak!
}

// Good: RAII wrapper
void good() {
    std::ifstream f("data.txt");  // Opens in constructor
    process(f);                    // If this throws...
    // f's destructor runs automatically, closing the file
}

// Smart pointers: RAII for heap memory
auto p = std::make_unique<Widget>();   // unique_ptr: sole ownership
auto q = std::make_shared<Widget>();   // shared_ptr: reference counted
std::weak_ptr<Widget> w = q;           // weak_ptr: non-owning observer
```

## Rust Ownership — Compile-Time Memory Safety

### The Three Rules
1. Each value has exactly one **owner**
2. When the owner goes out of scope, the value is **dropped** (freed)
3. You can have **either** one mutable reference OR any number of immutable references — never both

```rust
fn main() {
    let s1 = String::from("hello");  // s1 owns the String
    let s2 = s1;                      // Ownership MOVES to s2. s1 is invalid.
    // println!("{}", s1);            // Compile error! s1 was moved.
    
    let s3 = s2.clone();             // Explicit deep copy. Both valid.
    
    takes_ownership(s2);              // s2 moves into the function
    // s2 is invalid here
    
    let s4 = String::from("world");
    let len = calculate_length(&s4);  // Borrow: function gets a reference, not ownership
    println!("{} is {} bytes", s4, len);  // s4 still valid!
}

fn takes_ownership(s: String) {
    println!("{}", s);
}   // s is dropped (freed) here

fn calculate_length(s: &String) -> usize {
    s.len()
}   // s goes out of scope but doesn't own the String, so nothing happens
```

### Lifetimes
```rust
// Compiler needs to know: does the returned reference live as long as x or y?
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}
// Both x and y must live at least as long as 'a
```

## Garbage Collection Strategies

### Reference Counting
```
Each object has a counter. Increment on new reference, decrement on lost reference.
Free when counter reaches zero.

Pros: Deterministic destruction, simple
Cons: Can't handle cycles (A→B→A), counter overhead on every ref change

Used by: Swift (ARC), Python (+ cycle collector), Rust Rc<T>/Arc<T>, Objective-C (ARC)
```

**The Cycle Problem:**
```python
# Python: reference counting + cycle detector
a = []
b = []
a.append(b)  # a → b
b.append(a)  # b → a  (cycle! refcount never reaches 0)
# Python's gc module periodically detects and collects cycles
```

### Mark and Sweep
```
Phase 1 (Mark):   Starting from roots (stack, globals), traverse all reachable objects
Phase 2 (Sweep):  Free everything not marked

Pros: Handles cycles naturally
Cons: Stop-the-world pauses, fragmentation

Used by: JavaScript (V8), early Java
```

### Generational GC
**Hypothesis:** Most objects die young (weak generational hypothesis).

```
Young Generation (nursery): Small, collected frequently, fast
  Most objects are allocated here and die here.
  Survivors get promoted to →

Old Generation (tenured): Large, collected rarely, slower
  Long-lived objects accumulate here.
  Full GC is expensive (stop-the-world).
```

**Used by:** Java (G1, ZGC), .NET, V8 (JavaScript), Python (generational + ref counting)

### Modern GC Algorithms
| Algorithm | Language | Key Feature |
|-----------|----------|-------------|
| G1 GC | Java (default) | Region-based, predictable pauses |
| ZGC | Java | Sub-millisecond pauses, concurrent, up to 16TB heap |
| Shenandoah | Java (Red Hat) | Concurrent compaction, low pause |
| Orinoco | V8 (JavaScript) | Concurrent, incremental, parallel |
| .NET GC | C#/.NET | Generational, concurrent background, server/workstation modes |

## Arena / Region Allocation

```
Allocate objects from a contiguous memory region.
Free everything at once when the arena is destroyed.
No individual frees. No fragmentation. Extremely fast.
```

```rust
// Rust: bumpalo arena
use bumpalo::Bump;

let arena = Bump::new();
let x = arena.alloc(42);               // Fast: just bumps a pointer
let s = arena.alloc_str("hello");      // No individual free needed
// Everything freed when `arena` is dropped
```

**Use when:** Phase-based allocation (compilers: allocate per-function, free after), request-scoped (web: allocate per-request, free after response), game frames, parsers.

## Memory Layout & Alignment

### Struct Padding
```c
// Naive layout (with padding for alignment):
struct Bad {
    char a;     // 1 byte + 7 bytes padding
    double b;   // 8 bytes (needs 8-byte alignment)
    char c;     // 1 byte + 7 bytes padding
};  // Total: 24 bytes

// Reordered (no wasted padding):
struct Good {
    double b;   // 8 bytes
    char a;     // 1 byte
    char c;     // 1 byte + 6 bytes padding
};  // Total: 16 bytes (saved 33%!)
```

**Rule:** Order struct fields from largest to smallest alignment.

### False Sharing (Cache Line Contention)
```
CPU caches work in cache lines (typically 64 bytes).
If two threads modify variables that share a cache line,
the cache line bounces between cores → massive slowdown.
```
```c
// Bad: counter[0] and counter[1] likely share a cache line
int counter[2];  // Thread 0 writes counter[0], Thread 1 writes counter[1]

// Good: pad to separate cache lines
struct alignas(64) PaddedCounter { int value; };
PaddedCounter counter[2];
```

## Memory Debugging Tools

| Tool | What it catches | Language |
|------|----------------|---------|
| Valgrind (memcheck) | Leaks, use-after-free, uninit reads | C/C++ |
| AddressSanitizer (ASan) | Buffer overflow, use-after-free, leaks | C/C++/Rust |
| MemorySanitizer (MSan) | Uninitialized memory reads | C/C++ |
| LeakSanitizer (LSan) | Memory leaks (standalone or with ASan) | C/C++ |
| ThreadSanitizer (TSan) | Data races, deadlocks | C/C++/Go |
| Heaptrack | Heap profiling (who allocated what) | C/C++ |
| `tracemalloc` | Memory allocation tracking | Python |
| `gc` module | Garbage collection debugging | Python |

```bash
# Compile with ASan
gcc -fsanitize=address -g program.c -o program
./program  # Crashes with detailed report on memory errors
```

## Quick Reference: Memory Model by Language

| Language | Model | Key Mechanism |
|----------|-------|---------------|
| C | Manual | malloc/free |
| C++ | Manual + RAII | new/delete + smart pointers + destructors |
| Rust | Ownership | Compile-time borrow checker, no GC |
| Go | GC | Concurrent, tri-color mark-and-sweep |
| Java | GC | Generational (G1/ZGC/Shenandoah) |
| C# | GC | Generational + `struct` for stack allocation |
| Python | RC + GC | Reference counting + generational cycle collector |
| JavaScript | GC | Generational (V8 Orinoco) |
| Swift | ARC | Automatic Reference Counting (compile-time inserted retain/release) |
| Zig | Manual | Allocator-aware standard library, no hidden allocations |

---

*"Memory is like a closet. You can organize it carefully (Rust), hire someone to clean it periodically (GC), or just pile everything in and hope for the best (C with no discipline)."*
