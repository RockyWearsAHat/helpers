# JavaScript Runtime Internals — V8, Event Loop, and Garbage Collection

## Overview

Modern JavaScript runtimes manage complex execution models involving event-driven I/O, garbage collection pauses, and JIT compilation. V8 (Chrome, Node.js) dominates, but understanding these internals applies across runtimes. The event loop drives concurrency; JIT compilation delivers performance; GC pauses introduce latency.

## The Event Loop: Single-Threaded Concurrency

### Phases and Queue Priorities

The JavaScript event loop runs in a single thread but interleaves blocking I/O with event handlers. Each iteration processes:

1. **Execute microtasks** (fully drain queue):
   - Promise reactions (`.then()`, `.catch()`, `.finally()`)
   - `queueMicrotask()` callbacks
   - MutationObserver reactions
   - Process.nextTick (Node.js only)

2. **Check timers**: Execute setTimeout/setInterval callbacks whose delay elapsed

3. **Execute callbacks** from I/O completion (fs.readFile, network requests)

4. **Render**, if browser (layout, paint, requestAnimationFrame)

5. Repeat

The key insight: **microtasks always execute before the next timer**. This means:

```javascript
setTimeout(() => console.log('timer'), 0);
Promise.resolve().then(() => console.log('promise'));
// Output: promise, timer
```

The promise microtask drains before the timer queue advances.

### Blocking and Starvation

A long synchronous script blocks the entire loop. Event handlers (clicks, I/O) don't fire until the script completes. This is why long computations are offloaded to web workers (browser) or worker_threads (Node.js). Microtasks can starve timers: if microtasks continuously re-queue themselves, timers never fire.

### The Stack and Call Stack Traces

Each function call pushes a frame onto the stack. When a promise chain breaks (async/await, promise handlers), the stack is lost. Developers see `Promise { <pending> }` in debuggers, not the call path. Modern runtimes track **async stack traces** by linking stack contexts across async boundaries, but this has runtime cost and is often disabled in production.

## JIT Compilation: From Interpreter to Machine Code

### Tiers and Optimization

V8 uses tiered compilation:

1. **Ignition (Interpreter)**: Bytecode interpreter. Fast startup, limited optimization
2. **TurboFan (JIT)**: Full optimizing compiler. Compiles hot code to machine code after profiling data shows patterns

Not all code is compiled—only hot paths justify compilation overhead. A function called once is interpreted; called 10,000 times and inlined into a loop, it's compiled.

### Type Specialization and Deoptimization

V8 assumes types based on runtime observations. If a function always receives integers, V8 specializes the code for integers. If later a string is passed, **deoptimization** occurs—the optimized code is discarded, execution falls back to the interpreter, and type assumptions are invalidated.

This mechanism enables aggressive optimization for monomorphic code (single observed type) while handling polymorphic code (multiple types) gracefully but slowly.

### Inline Caching and Hidden Classes

When accessing `obj.property`, V8 doesn't lookup "property" in a hash table every time. Instead, it caches the memory offset of the property within the object's memory layout. Repeated access to the same object shape is nearly free (single indirect memory load).

### Hidden Classes and Shape Transitions

All objects with the same property set share a **hidden class** descriptor. When properties are added or deleted, the object transitions to a new hidden class:

```javascript
const obj1 = { a: 1 };           // Hidden class: {a: offset=0}
const obj2 = { a: 2 };           // Same hidden class (IC cache hit)

const obj3 = {};
obj3.a = 1;                       // Transition to new hidden class
obj3.b = 2;                       // Another transition
```

Objects with different construction patterns have different hidden classes, defeating IC caching and forcing polymorphic lookups. This is why predictable object structure (e.g., always adding properties in the same order) improves performance. Class syntax encourages this discipline.

##Garbage Collection: Generational and Incremental

### The Generational Hypothesis

Most objects die young. GC strategy: partition the heap into **generations**:

- **Young generation**: Small, collected frequently. Most objects are freed here
- **Old generation**: Large, collected rarely. Most collections are quick

Within the young generation, V8 uses a **bump allocator**: allocate by advancing a pointer. When space is exhausted, collect by copying live objects to the other semi-space (Cheney's algorithm). This is cache-friendly and low-overhead.

### Mark-and-Sweep for Old Generation

Old-generation objects are collected less frequently via mark-and-sweep:

1. **Mark**: Starting from roots (stack, global variables), traverse the reachability graph, marking live objects
2. **Sweep**: Scan the heap, freeing unmarked objects

This is slower than generational collection but amortizes to near-zero pause time when done incrementally.

### Incremental and Concurrent Collection

Pausing the entire program to collect is unacceptable: GC pauses longer than 50ms are user-perceivable. V8 uses:

- **Incremental marking**: Mark phase runs in small time slices, interleaved with the main program. When the program allocates, marking work progresses.
- **Concurrent marking**: A background thread marks while the main thread executes. Conflicts are resolved via write barriers: when the main thread modifies an old-generation object, a write barrier adds it to a work queue for concurrent marking to revisit.

The goal: GC pauses under 10ms, imperceptible to users.

### Write Barriers and Cross-Generation References

When an old-generation object holds a reference to a young-generation object, a **write barrier** records this. Without barriers, minor collections would miss reachability through old-generation references and incorrectly free young objects.

## Memory Leaks and WeakRef/FinalizationRegistry

### Common Leak Patterns

1. **Detached DOM nodes**: Removing a DOM node from the tree doesn't free its memory if JavaScript holds a reference. Event listeners attached to detached nodes also prevent collection.
2. **Circular event listeners**: `obj.on('event', () => obj.close())` creates a reference cycle. The event emitter holds the listener; the listener captures `obj` in its closure.
3. **Large objects in closures**: A callback closes over a large object; the callback is stored globally; the object becomes unreachable to application logic but not to GC.

Debugging leaks: Use heap snapshots to compare before/after states, identify retained objects, trace their reference chain backward.

### WeakRef: Pointers That Don't Prevent Collection

`WeakRef` holds a reference to an object that won't prevent garbage collection:

```javascript
const weakRef = new WeakRef(largeObject);
// ... later
const ref = weakRef.deref();  // null if collected, else object
```

Use case: Caches. A cache holding a weak reference to objects—if memory is low and the object is only reachable via the cache, it's collected. When dereferenced later, the cache entry is stale.

### FinalizationRegistry: Cleanup When Objects Are Collected

`FinalizationRegistry` invokes a callback after an object is collected:

```javascript
const registry = new FinalizationRegistry((heldValue) => {
  console.log(`Object with id ${heldValue} was collected`);
});

registry.register(obj, 'unique-id');
```

Use case: Freeing external resources (WebGL textures, native memory from WebAssembly). When the wrapper object is collected, cleanup code runs. Caveat: timing is unpredictable; don't rely on it for critical operations.

## Structured Clone and Serialization Boundaries

### What Structured Clone Copies

`structuredClone()` (supported in modern runtimes and workers) serializes and deserializes objects, breaking references and enabling safe object transfer between threads:

- Primitives (numbers, strings, booleans, null, undefined) are copied
- Objects, arrays, typed arrays, and maps are recursive copied (new identity)
- Functions, symbols, and DOM nodes **cannot** be cloned (error thrown)
- Cycles are detected; circular references don't cause infinite recursion

### PostMessage and Worker Data Transfer

When passing data to a worker via `worker.postMessage()`, the object is structured-cloned by default. Alternatively, **Transferable** objects (ArrayBuffer, ImageBitmap, MessagePort, OffscreenCanvas) are transferred—ownership moves, zero-copy to the worker.

```javascript
const buffer = new ArrayBuffer(1024);
worker.postMessage({ buffer }, [buffer]);
// buffer is now detached in the main thread
```

After transfer, accessing the buffer in the main thread throws `TypeError`.

## Runtime Platforms: V8 vs. JSC vs. SpiderMonkey

### V8 (Chrome, Node.js, Edge)

Developed by Google. Emphasis: startup speed (ignition tier), throughput (TurboFan optimization), and tooling (DevTools).

### JavaScriptCore (Safari, Bun)

Apple's engine. Emphasis: low memory footprint, reliability. JSC uses a different JIT strategy (multiple tiers, different heuristics). Bun uses JSC for startup speed (faster than V8 for server startups) but sacrifices long-term throughput.

### SpiderMonkey (Firefox)

Mozilla's engine. Similar architecture to V8 but developed independently. Less used for server-side work.

### Deno and Node.js Event Models

**Node.js** uses libuv (event loop library) for I/O event demultiplexing. **Deno** uses Tokio (Rust async runtime). By 2024, both interoperate with the same JavaScript semantics, but Deno offers stricter permissions (file access, network). The runtime choice affects I/O performance, not event loop semantics.

## Performance Implications

### Bridging the Interpretation-Compilation Gap

Startup time (interpreted code) favors small, frequently-run loops. Long-lived servers favor compiled code (warm). Hybrid strategies (lazy Tiering, profile-guided compilation) balance both.

### Heap Size and GC Pressure

Large heap sizes reduce GC frequency but increase pause time. Small heaps cause frequent collections. Node.js `--max-old-space-size=4096` increases heap size at the cost of longer pauses.

### The cost of Variability

Write code that's predictable to the JIT. Avoid hidden class transitions (different property addition orders), type polymorphism, and megamorphic lookups. Frameworks like TypeScript encourage static types, which guide predictable patterns.

## See Also

- [Event Loop & Asynchronous Execution Models](web-event-loop.md)
- [WebAssembly and Native Integration](web-webassembly.md)
- [Performance Web Vitals and Runtime Metrics](performance-web-vitals.md)