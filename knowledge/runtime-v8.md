# V8 Engine Internals

## Overview

V8 (Chrome's JavaScript engine) transforms source code into machine code through a sophisticated multi-stage pipeline optimized for web performance. The engine balances immediate execution (via bytecode interpretation) with peak optimization (via speculative JIT compilation), using profiling feedback to drive decisions about what and how to compile.

## Parsing Pipeline

### Lexing & Tokenization

The **Scanner** converts source text into tokens, handling:

- Regular expression literal disambiguation (the `/` ambiguity—division operator vs. regex start)
- Template literal nesting (backtick expressions containing nested templates)
- Automatic semicolon insertion (ASI) prediction
- Unicode escape sequences and surrogate pair handling

### Parse Strategies

V8 uses **lazy parsing** for functions not immediately needed:

| Strategy | Output | When Used | Tradeoff |
|----------|--------|-----------|----------|
| Full parse | Complete AST | Immediately invoked FN | None; ready to execute |
| Lazy parse | Syntax shell only | Functions defined but not called | Parse again on invocation |
| Pre-parse | Minimal validation | Inner functions | Skip expensive analysis, stay compatible |

Lazy parsing avoids building ASTs for code never executed (e.g., feature-detection branches, unused utilities). When a function becomes hot or explicitly invoked, V8 reparses to full depth.

### AST to Bytecode

The parser emits an **Abstract Syntax Tree**; the bytecode generator walks it, emitting bytecode for the **Ignition** interpreter. V8 does not generate machine code from AST directly—bytecode is the universal intermediate form.

## Ignition Interpreter

### Bytecode Design

Ignition bytecode targets a **register machine**, not a stack machine. This reduces memory footprint and simplifies optimization passes:

```
Ignition bytecode: LdaGlobal <index>, StaProperty <reg>, Add <reg1>, <reg2>
                   (load global, store property, add two registers)
```

Advantages over stack machines:
- Explicit operand lifetimes aid escape analysis
- Predictable register pressure eases JIT planning
- Smaller bytecode size (fewer instructions)

### Dispatch & Execution

The bytecode loop in C++ iterates through bytecode handlers, dispatching via **bytecode offset** or **computed goto**:

```cpp
while (bytecode = *pc++) {
  switch(bytecode) {
    case ADD: sp[0] += sp[1]; break;
    case CALL: result = Call(target, args); break;
  }
}
```

V8 generates bytecode handlers from TurboFan components at startup, enabling consistent optimization across interpreter and compiled code.

## Hidden Classes & Inline Caches

### The Shape Problem

JavaScript objects are dynamic: properties can be added, deleted, or reordered at runtime. Naive property access requires a hashtable lookup on every access—expensive.

V8 groups objects with identical property structures into **hidden classes** (called "shapes" or "maps" in internal terminology). Each object holds a pointer to its shape, encoding:

```
Shape: {prop1: offset 24, prop2: offset 32, prop3: ~}
Object A: [hidden_class] → [props at offsets 24, 32]
Object B: [hidden_class] → [props at offsets 24, 32]  // Same shape
```

Adding a property transitions to a new shape via **shape chains**:

```
{} → {x: 0} → {x: 0, y: 1} → {x: 0, y: 1, z: 2}
```

If two code paths add properties in different orders, they fork into separate shapes.

### Inline Caches

An **inline cache** at each property access site records:

1. **Monomorphic**: One shape observed. Cache stores shape + property offset. Next access skips lookup.
2. **Polymorphic**: 2-4 shapes observed. Cache holds small lookup table.
3. **Megamorphic**: >4 shapes. Cache is bypassed; full hashtable lookup occurs.

```javascript
function getAge(person) { return person.age; }
getAge({age: 30});      // IC: cache shape for {age}
getAge({age: 35});      // IC hit: same shape
getAge({name: "x", age: 4}); // IC misses: new shape, IC becomes polymorphic
```

Polymorphic ICs are slower than monomorphic but faster than megamorphic. Developers writing shape-polymorph-prone loops can trigger deoptimization.

## TurboFan Compiler

### Speculative Optimization

**TurboFan** is V8's aggressive optimizing compiler, built on three pillars:

1. **Profiling feedback** — The interpreter logs type information: "this.x is always a number," "this function never returns null."
2. **Speculation** — TurboFan assumes the observed types continue. It emits direct numeric operations instead of polymorphic branching.
3. **Deoptimization** — If speculation fails (e.g., `this.x` becomes a string), the runtime bails back to bytecode.

Speculative optimization is risky but fast: assume the best case and recover on failure.

### Optimization Phases

TurboFan's compilation pipeline:

1. **Graph construction** — Bytecode → Sea of Nodes (IR)
2. **Early simplifications** — Constant folding, dead code elimination
3. **Inlining** — Inline hot call sites and small functions
4. **Loop optimizations** — Unrolling, invariant code motion
5. **Type inference** — Propagate types through the graph
6. **Escape analysis** — Determine which objects never escape; stack-allocate them
7. **Late simplifications** — Machine-level optimizations
8. **Code generation** — Emit machine code with deoptimization checks

### Deoptimization

When an assumption breaks—e.g., an inline cache becomes polymorphic—the runtime pauses execution, discards the optimized code, and falls back to bytecode/Ignition. The bailout is cheap if rare, but frequent bailouts signal poor code patterns (shape squashing, accidental polymorphism).

## Memory & GC

### Generational Heap

V8 divides the heap into **Young** and **Old** spaces:

- Young (new-space): ~2MB. Collections are fast (mark-sweep).
- Old (pointer-space): ~300MB+. Collections use incremental mark-sweep to avoid long pauses.

Most objects die in the Young gen; survivors are promoted to Old.

### Orinoco GC

The **Orinoco** incremental collector (Node.js 10+, Chrome 63+) splits garbage collection across multiple small pauses instead of one large pause:

```
Mark slice 1 → Resume app → Mark slice 2 → Resume app → Sweep phase
(a few ms)                  (a few ms)     (background)
```

This keeps pause times under 50ms for interactive applications.

## Advanced Features

### WebAssembly Integration

V8 compiles WebAssembly modules to machine code using **Liftoff** (baseline) and **TurboFan** (optimized tier). WASM functions can be called from JavaScript and vice versa with minimal overhead.

### Adaptive Optimization

V8 constantly profiles code to decide optimization strategy:

- Frequently accessed function? Compile with TurboFan.
- Rarely accessed function? Stay in Ignition to save memory.
- Function becoming polymorphic? Deoptimize and mark for recompilation.

This adaptive strategy means the engine's behavior changes over application lifetime—a cold function gradually warms and shifts to optimized code.

## Design Philosophy

V8's performance stems from bridging interpretation and compilation:

- **Bytecode** provides a universal intermediate between parser and execution
- **Profiling** informs optimization decisions
- **Deoptimization** enables speculative assumptions to be safe
- **Hidden classes** make dynamic object access predictable
- **Tiering** (Ignition → TurboFan) balances startup and peak performance

The tension: more profiling and optimization improves peak performance but increases memory and compilation latency. V8 prioritizes responsiveness for interactive workloads, making different trade-offs than JVMs targeting server batch jobs.

## See Also

- JavaScript engine optimization, shape polymorphism, inline caching principles
- JIT compilation strategy, speculative optimization trade-offs
- WebAssembly compilation architecture