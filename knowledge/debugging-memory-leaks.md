# Debugging Memory Leaks — Detection, Analysis, and Prevention

A **memory leak** occurs when a program allocates memory but fails to release it, causing memory usage to grow unbounded over time. Unlike compile-time errors, leaks are insidious: the program may run correctly for hours before exhausting system memory and crashing. Effective leak debugging requires both the right tools and understanding of how different languages and environments manage memory.

## The Leak Landscape: What Counts as a Leak?

Not all retained memory is a leak. Understanding the distinction is critical:

- **True leak**: allocated memory is unreachable from the root set (impossible to free)
- **Phantom reference**: memory held by circular references despite the program losing all pointers to it (common in GC'd languages)
- **Generational bloat**: legitimate long-lived objects accumulating in memory (not a leak per se, but can mimic leak symptoms)

Different tools target different leak types. A tool reporting "leaked memory" in a garbage-collected language might be flagging phantom references held by cycles. A true leak in C requires allocated pointers to be completely lost.

## Heap Snapshots: Point-in-Time Memory State

A **heap snapshot** captures the memory state at a single moment: all allocated objects, their types, sizes, and reference relationships. Snapshots are most useful for comparing two states and identifying what grew between them.

### Chrome DevTools Heap Snapshots

**Workflow:**
1. Open DevTools → Memory tab → Heap snapshot section
2. Click "Take snapshot" to capture the current heap
3. Perform a user action (load data, trigger a memory-consuming operation)
4. Take a second snapshot
5. Use the "Comparison" view to filter only new/grown allocations

**Key columns:**
- **Constructor**: object type (e.g., HTMLDivElement, String, Object)
- **Distance**: hops from the root set; lower is more directly referenced
- **Shallow size**: bytes this object alone consumes
- **Retained size**: bytes this object plus its descendants would free if deleted

**Reading the diff:** A large retained size for a mundane object (e.g., a String or Array) often points upstream: something is holding unnecessary references to it.

**Detached DOM nodes:** A classic leak signature. The JavaScript heap retains references to offscreen DOM nodes that have been removed from the document. Chrome DevTools has a dedicated detector: Memory → Detached DOM nodes. Each detached node holds references to its entire subtree and ancestors, making this a high-impact leak type.

### Allocation Timeline (Chrome)

Instead of snapshots, the **Allocation Timeline** tracks allocations continuously. Each blue bar represents a period of time; height is proportional to allocation rate. This is more effective than snapshots for finding __which operations__ cause memory to grow:

1. Open Memory tab → Allocation timeline
2. Start the profiler
3. Perform a sequence of actions
4. Stop and examine the timeline
5. Click on bars to see which constructor calls grew during that period

**Use case:** Discovering that a "harmless" button click somehow allocates thousands of event listeners.

## Valgrind and AddressSanitizer: C/C++ Leak Detection

These tools work via **runtime instrumentation**, intercepting every malloc/free call to track allocations and report what wasn't freed.

### Valgrind (Memcheck)

Valgrind's Memcheck tool runs the program under a virtual machine layer, tracking every heap operation:

```bash
valgrind --leak-check=full --show-leak-kinds=all ./my_program
```

Output categories:
- **Definitely lost**: unreachable allocations (true leaks)
- **Indirectly lost**: only reachable through lost blocks
- **Possibly lost**: reachable via lost pointers (conservative; often false positives)
- **Still reachable**: allocated at exit, possibly intentional

Valgrind is thorough but slow (10-100× overhead). Use it during development, not production profiling.

### AddressSanitizer (ASAN)

ASAN is a compiler facility (supported in Clang, GCC ≥4.8):

```bash
clang -fsanitize=address -g my_program.c -o my_program
./my_program
```

ASAN is **much faster** than Valgrind (2-5× overhead) because it's instrumented at compile time. It detects:
- Heap buffer overflows
- Use-after-free
- Memory leaks (with `-fsanitize=leak`)
- Uninitialized reads

ASAN output is more readable than Valgrind's, with stack traces pinpointing the problematic allocation.

## Language-Specific Leak Patterns

### JavaScript: Closures and Event Listeners

JavaScript's garbage collector is reachability-based. If an event listener captures a large object in its closure, that object remains alive as long as the listener is attached:

```javascript
function setupListener() {
  const hugeData = new Array(1e6);
  button.addEventListener('click', () => {
    console.log(hugeData[0]); // closure holds hugeData alive
  });
}
// Even if setupListener() returns, the listener holds hugeData
```

**Fix:** Explicitly remove listeners or use weak references:

```javascript
button.removeEventListener('click', handler);
// or
const controller = new AbortController();
button.addEventListener('click', handler, { signal: controller.signal });
controller.abort(); // removes all listeners with this signal
```

### Python: Circular References

Python uses reference counting, but **circular references** defeat it:

```python
class Node:
  def __init__(self):
    self.next = None

a = Node()
b = Node()
a.next = b
b.next = a  # cycle; ref-count alone won't free them
del a, b     # references dropped, but cycle remains
```

Python's garbage collector runs periodically to break cycles, but generation 2 (oldest objects) may never be scanned if the program exits. Debug using:

```python
import gc
gc.collect()  # force collection immediately
leaked = gc.garbage  # objects the collector couldn't free
```

Alternatively, use `weakref` to avoid holding strong references:

```python
import weakref
class Node:
  def __init__(self):
    self.next = None

b_ref = weakref.ref(b)
a.next = b_ref  # doesn't prevent b from being freed
```

### Rust: Lifetimes Prevent Leaks

Rust's lifetime and ownership system makes many leaks impossible:

```rust
fn leak_impossible() {
  let x = vec![1, 2, 3];  // owned by this function
}  // x is dropped here; no leak
```

However, Rust programs CAN leak if you use `mem::forget()` or create cycles with `Rc<RefCell<T>>`:

```rust
use std::rc::Rc;
use std::cell::RefCell;

let a = Rc::new(RefCell::new("A"));
let b = Rc::new(RefCell::new("B"));
*a.borrow_mut() = b.clone();  // a points to b
*b.borrow_mut() = a.clone();  // b points to a (cycle)
drop(a);
drop(b);  // cycle prevents drop; leaks memory
```

Use `Weak` references to break cycles:

```rust
let b_weak = Rc::downgrade(&b);
*a.borrow_mut() = b_weak;  // doesn't prevent b from being freed
```

### Go: Goroutine Leaks

Goroutines that never exit leak both memory (the stack) and resources (any channels they're blocked on):

```go
func leakGoroutine() {
  ch := make(chan int)
  go func() {
    <-ch  // blocks forever
  }()
  // goroutine never exits; ch is never closed
}
```

**Sign:** Process memory grows per goroutine launch. Use `runtime.NumGoroutine()` to detect runaway goroutine counts:

```go
import "runtime"
fmt.Println(runtime.NumGoroutine())  // should grow only during startup
```

Fix: ensure goroutines can exit. Use `context.Context` for cancellation:

```go
func leakFixed(ctx context.Context) {
  ch := make(chan int)
  go func() {
    select {
    case <-ch:
    case <-ctx.Done():
      return  // goroutine exits
    }
  }()
}
```

## Reference Cycle Detection

Circular references are difficult to detect manually. Tools exist for specific languages:

**Python GC cycles:**
```python
import gc
gc.set_debug(gc.DEBUG_SAVEALL)
gc.collect()
print(gc.garbage)  # all cyclical objects
```

**JavaScript WeakMap inspection:**
Use DevTools to search for objects held by WeakMaps or WeakSets. These are often intentional, but sometimes signal accidental long-term retention.

## GC Pressure and Allocation Rate Analysis

High allocation rates don't always indicate leaks. They might instead signal **high GC pressure** (frequent collection runs), which degrades performance even if memory is eventually freed.

**Analysis approach:**
1. Record allocation timelines and heap size over time
2. Plot live object count: does it plateau or grow monotonically?
3. If monotonic growth → leak
4. If sawtooth pattern (growth then drop) → high allocation rate, not a leak (but still bad for latency)

**Tools:**
- Chrome DevTools Allocation Timeline: visualizes sawtooth patterns
- JVM: Use `-XX:+PrintGCDetails` to see GC runs
- Linux: `dmesg` for OOM (out of memory) events

## Production Leak Diagnosis

Production environments require read-only debugging without stopping the service.

**Approach:**
1. Monitor memory over days/weeks; confirm monotonic growth
2. Capture periodic heap snapshots without service interruption (many languages support online snapshots)
3. Compare snapshots from 24 hours apart; look for unusual object growth
4. Cross-reference with user behavior or code deployments

**Tools:**
- Java: jmap (JDK tool) can take heap dumps on running processes
- Node.js: heap snapshots via diagnostic report or profiler
- Python: memory_profiler or py-spy

The key discipline: isolate __which code path__ is allocating, then determine why references aren't released.

## Prevention

1. **Use static analysis:** C/C++ linters (clang-analyzer), JavaScript (ESLint memory plugins)
2. **Write tests under load:** Memory tests should run realistic scenarios for hours, checking for growth
3. **Profile regularly:** Make heap profiling a CI step, flag regressions
4. **Code review checklist:** Event listeners removed? Circular references broken? Goroutines guaranteed to exit?
5. **Language-appropriate patterns:** Prefer weak references, use destructors/del hooks properly, understand your GC's cycle-detection guarantees

Memory leaks are predictable and preventable with discipline and the right tools. The earlier you detect them, the lower the cost.