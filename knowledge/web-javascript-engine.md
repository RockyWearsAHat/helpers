# JavaScript Engine Internals — Parsing, Compilation & Optimization

## Overview

Modern JavaScript engines transform source code into executable machine code through a multi-stage pipeline that balances competing demands: fast startup (begin executing quickly), peak throughput (run hot code efficiently), and bounded memory usage. The design choices engines make along this spectrum directly influence how performance characteristics emerge in practice.

Understanding engine internals at a conceptual level illuminates why certain code patterns perform differently than others, without prescribing rigid rules — engines evolve, and optimizations that matter today may become irrelevant as implementations change.

## The Execution Pipeline

```
Source Text
    │
    ▼
  Lexer / Scanner
    │  (tokens)
    ▼
  Parser
    │  (AST)
    ▼
  Bytecode Generator
    │  (bytecode)
    ▼
  Interpreter          ← Executes immediately
    │  (profiling data)
    ▼
  Baseline JIT         ← Compiles warm functions
    │  (more profiling)
    ▼
  Optimizing JIT       ← Speculatively optimizes hot functions
    │
    ▼
  Deoptimization       ← Falls back when assumptions break
    │
    ▼
  Interpreter / Lower tier
```

The pipeline is not strictly linear — functions move between tiers based on execution frequency and the validity of optimization assumptions.

## Parsing: Source to Structure

### Lexical Analysis

The scanner converts raw source text into tokens — identifiers, keywords, literals, operators, punctuation. This stage handles:

- Unicode normalization and escape sequences
- Template literal parsing (including nested expressions)
- Regular expression literal disambiguation (the `/` ambiguity)
- Automatic semicolon insertion awareness

### Parsing Strategies

Full parsing (eager parsing) builds a complete AST for every function in the source. This is straightforward but wasteful — many functions in a typical page load are never called, or called much later.

**Lazy parsing** (pre-parsing) addresses this by doing minimal work on function bodies that are not immediately needed:

| Strategy   | Work Done                                     | Output                           | When Used                          |
| ---------- | --------------------------------------------- | -------------------------------- | ---------------------------------- |
| Full parse | Complete syntax analysis                      | Full AST + scope info            | Immediately invoked functions      |
| Lazy parse | Syntax validation only                        | Enough to skip the function body | Functions not yet called           |
| Reparse    | Full parse of previously lazy-parsed function | Full AST                         | When the function is first invoked |

The trade-off: lazy parsing avoids upfront work but means some functions pay the parsing cost twice (once lazy, once full). Engines use heuristics to predict which functions to parse eagerly — IIFEs and small functions are often parsed eagerly since they're likely to execute soon.

### Abstract Syntax Tree

The AST represents the syntactic structure of the program. For engine purposes, this is a transient representation — it's consumed by the bytecode generator and then discarded to reclaim memory.

```
function add(a, b) { return a + b; }

FunctionDeclaration
├── id: Identifier("add")
├── params: [Identifier("a"), Identifier("b")]
└── body: BlockStatement
    └── ReturnStatement
        └── BinaryExpression("+")
            ├── left: Identifier("a")
            └── right: Identifier("b")
```

## Bytecode: The Interpreter's Language

Rather than interpreting the AST directly (which involves expensive tree-walking), engines compile to a compact bytecode representation. Bytecode provides:

- **Compact representation** — smaller memory footprint than AST
- **Efficient dispatch** — bytecode opcodes map to handler functions
- **Profiling insertion points** — natural locations to gather type information

### Bytecode Design Trade-offs

| Choice             | Startup Impact              | Throughput Impact                      |
| ------------------ | --------------------------- | -------------------------------------- |
| Register-based     | Slightly slower to generate | Fewer instructions, potentially faster |
| Stack-based        | Simpler generation          | More instructions, simpler handlers    |
| High-level opcodes | More analysis needed        | Better optimization opportunities      |
| Low-level opcodes  | Faster generation           | More instructions, less semantic info  |

Different engines make different choices here. The bytecode format is an internal implementation detail that can change between versions.

## The Interpreter-First Approach

Modern engines interpret bytecode before compiling to native code. This design choice exists because:

1. **Compilation takes time** — JIT compilation is not free; interpreting immediately provides faster time-to-first-execution
2. **Not all code is hot** — much code executes once or rarely; compiling it wastes time and memory
3. **Profiling needs execution** — optimal compilation requires type information that can only be gathered by running the code
4. **Memory pressure** — compiled machine code is larger than bytecode; only compiling hot code manages memory budget

The interpreter also serves as a fallback — when optimized code must be abandoned (deoptimization), execution continues in the interpreter. Even the interpreter employs optimizations: threaded dispatch (handlers jump directly to the next handler), inline specialization for common patterns, and superinstructions that fuse frequent bytecode sequences.

## JIT Compilation Tiers

### Tiering Concepts

JIT compilation operates in tiers, each trading compilation cost for code quality:

```
Tier 0: Interpreter
  - No compilation cost
  - Collects type profile data
  - Suitable for cold code

Tier 1: Baseline / Template JIT
  - Quick compilation, moderate code quality
  - May still include profiling instrumentation
  - Suitable for warm code

Tier 2: Optimizing JIT
  - Expensive compilation, high code quality
  - Uses speculative optimizations based on profiling
  - Suitable for hot code

(Some engines have additional intermediate tiers)
```

### Tier Transition Triggers

Functions advance to higher tiers based on execution counters and heuristics:

| Signal                 | Meaning                                              |
| ---------------------- | ---------------------------------------------------- |
| Invocation count       | Function called many times                           |
| Loop iteration count   | Loop body executed many times (on-stack replacement) |
| Time in interpreter    | Function consuming significant interpreter time      |
| Bytecode size          | Very large functions may be deprioritized            |
| Deoptimization history | Frequently deoptimized functions may be penalized    |

**On-Stack Replacement (OSR)** allows long-running functions with hot loops to be promoted mid-execution at a loop back-edge, without waiting for the function to return. This is critical for initialization code that spends thousands of iterations in a loop.

## Speculative Optimization

### The Core Idea

Dynamic languages lack static type information. The optimizing compiler works around this by observing what types actually flow through the code during interpreted execution, then compiling code that assumes those observed types will continue.

```
function compute(x, y) {
    return x + y;
}
```

If profiling shows `x` and `y` have always been integers, the optimizing compiler generates machine code for integer addition — far cheaper than the general `+` operator which must handle numbers, strings, BigInts, and object coercion.

### Type Guards

Speculative code includes guard checks that verify assumptions still hold:

```
Optimized code (conceptual):
  check x is Integer    → if not, deoptimize
  check y is Integer    → if not, deoptimize
  result = integer_add(x, y)
  check no overflow     → if overflow, deoptimize
  return result
```

Guards are cheap when they pass (typically a comparison and branch) but trigger expensive deoptimization when they fail.

### Optimization Techniques

| Technique              | What It Does                     | Assumption                        |
| ---------------------- | -------------------------------- | --------------------------------- |
| Type specialization    | Generate code for specific types | Types remain stable               |
| Inlining               | Replace call with callee body    | Callee identity is stable         |
| Escape analysis        | Allocate objects on stack        | Object doesn't escape function    |
| Dead code elimination  | Remove unreachable code          | Branch conditions are predictable |
| Common subexpression   | Reuse computed values            | No side effects between uses      |
| Range analysis         | Eliminate bounds checks          | Index values stay in range        |
| Load/store elimination | Cache property reads             | No aliased modifications          |

### Inlining Decisions

Function inlining — replacing a call site with the callee's body — is among the most impactful optimizations because it unlocks further optimizations across the inlined boundary. However, it increases code size, affecting instruction cache utilization. Engines balance callee size, call frequency, polymorphism degree, call depth, and total code size budgets.

## Deoptimization

Deoptimization occurs when a guard check fails — the optimized code's assumption is violated, and execution must fall back to a lower tier. Common triggers include type changes (integer variable receives a string), shape changes (object gains/loses properties), callee changes at polymorphic call sites, prototype chain modification, integer overflow, and `arguments` object materialization.

### The Deoptimization Process

The engine pauses optimized execution, reconstructs interpreter state from compiled-code-position-to-bytecode mappings ("deopt info"), and resumes in the interpreter. The function may later be recompiled with updated type information or penalized to remain in a lower tier.

Deoptimization of one function can cascade to other functions that inlined it, since inlined code shares the same compilation unit. A single type change in a widely-inlined utility function can have outsized performance impact.

## Hidden Classes / Shapes / Maps

### The Dynamic Property Problem

JavaScript objects are bags of key-value pairs that can change at any time. Naively, every property access requires a dictionary lookup — hash the property name, probe the hash table, handle collisions. This is far too slow for property-heavy code.

### The Shape Concept

Engines observe that many objects have the same properties added in the same order. Objects sharing this pattern share a single "shape" (also called "hidden class" or "map") describing which properties exist, their storage offsets, property attributes, and transition relationships to other shapes.

```
Conceptual shape transitions:

Empty shape
    │ add "x"
    ▼
Shape{x: offset 0}
    │ add "y"
    ▼
Shape{x: offset 0, y: offset 1}
```

Objects sharing a shape store property values in a flat array; the shape provides name-to-offset mapping. Property access becomes an offset load rather than a dictionary lookup.

### Shape Transitions

When a property is added to an object, the engine transitions it from its current shape to a new shape. These transitions form a tree:

```
Shape A (empty)
├── add "x" → Shape B {x}
│   ├── add "y" → Shape C {x, y}
│   └── add "z" → Shape D {x, z}
└── add "y" → Shape E {y}
    └── add "x" → Shape F {y, x}
```

Shapes C and F both have properties `x` and `y`, but in different order — they are different shapes. This is why property initialization order affects shape sharing.

### Implications

Objects that share shapes allow the engine to:

- Use the same optimized property access code for all objects of that shape
- Perform inline caching (see below) effectively
- Reduce per-object memory since shapes are shared

Objects that frequently change shape — adding/deleting properties dynamically — tend to fall off the fast path and may be represented with slower dictionary-mode storage.

## Inline Caches

### The Mechanism

An inline cache (IC) remembers the result of a property lookup at a specific code location, so subsequent executions can skip the lookup entirely.

```
Conceptual IC states:

Uninitialized → first access triggers lookup, caches result
    ▼
Monomorphic   → one shape seen, direct offset access
    ▼
Polymorphic   → few shapes seen (2-4), linear check of cached shapes
    ▼
Megamorphic   → many shapes seen, fall back to generic lookup
```

### Monomorphic vs Polymorphic Sites

| IC State    | Access Speed                         | Typical Scenario                                |
| ----------- | ------------------------------------ | ----------------------------------------------- |
| Monomorphic | Fastest — single check + offset load | All objects at this site share one shape        |
| Polymorphic | Fast — small chain of checks         | A few related shapes (e.g., subclass hierarchy) |
| Megamorphic | Slow — generic hash lookup           | Many unrelated object shapes                    |

The optimizing compiler uses IC feedback to specialize property accesses. A monomorphic IC is straightforward to optimize; a megamorphic site provides little useful type information and typically cannot be well-optimized.

### IC and Optimization Interaction

IC data collected during interpreted execution feeds directly into the optimizing compiler's decisions:

- Monomorphic property access → compile to a shape check + direct offset load
- Monomorphic call site → inline the callee
- Polymorphic access → compile a type dispatch chain
- Megamorphic access → compile a generic operation (limited optimization possible)

## Garbage Collection

JavaScript runtimes manage memory automatically. Objects are allocated on a managed heap, and a garbage collector reclaims memory from objects no longer reachable from the program's roots (global scope, stack, registered callbacks).

### Generational Collection

The **generational hypothesis** observes that most objects die young. Engines exploit this by dividing the heap into generations:

```
Young Generation (nursery)
├── Small, collected frequently
├── Uses fast copying/scavenging collection
├── Objects that survive are promoted to old generation
└── Short pause times due to small heap size

Old Generation (tenured)
├── Large, collected less frequently
├── Uses mark-sweep or mark-compact
├── Objects here tend to be long-lived
└── Collection is more expensive
```

### Collection Strategies

| Strategy           | How It Works                                      | Trade-offs                                                |
| ------------------ | ------------------------------------------------- | --------------------------------------------------------- |
| Scavenge (copying) | Copy live objects to new space, discard old space | Fast for young gen (few survivors), wastes half the space |
| Mark-sweep         | Mark reachable objects, sweep unmarked            | No copying overhead, leaves fragmentation                 |
| Mark-compact       | Mark reachable, then compact to eliminate gaps    | No fragmentation, but moving objects is expensive         |
| Reference counting | Track references per object                       | Immediate reclamation, cannot handle cycles alone         |

### Reducing GC Pauses

Stop-the-world pauses are problematic for interactive applications. Engines employ techniques to reduce pause impact:

- **Incremental marking**: Spread marking work across many small steps between JavaScript execution slices
- **Concurrent sweeping**: Sweep unreachable memory on background threads while JavaScript continues
- **Concurrent marking**: Perform some marking work on background threads (requires careful synchronization)
- **Lazy sweeping**: Defer actual deallocation until memory is needed
- **Write barriers**: Track old-to-young pointers to avoid scanning the entire old generation during young-gen collection

## The JIT Compilation Tax

JIT compilation is not free — it imposes costs that must be weighed against the performance gains:

| Cost   | Description                                                       |
| ------ | ----------------------------------------------------------------- |
| Time   | Compilation consumes CPU time that could execute application code |
| Memory | Compiled code is 5-10× larger than bytecode                       |
| Warmup | Functions execute slowly until compilation completes              |
| Jank   | Compilation on the main thread can cause visible pauses           |

Mitigation approaches include background compilation on worker threads, tiered compilation that only promotes code justifying the cost, code caching to disk for reuse across loads, and compilation budgets that limit per-timeslice work to avoid jank.

Workloads that see limited JIT benefit include short-lived scripts that finish before optimization thresholds, highly polymorphic code that prevents specialization, code dominated by cold paths, and memory-constrained environments where compiled code overhead is prohibitive.

## Engine Internals and Practical Performance

### Object Shape Stability

When objects have consistent shapes (same properties, same order), engines can use optimized property access paths. Shapes that are unstable (properties added conditionally, deleted, or added in varying order) push objects toward dictionary-mode representation.

```
// Shapes align — engine can share shape and use ICs effectively
function createPoint(x, y) {
    const p = {};
    p.x = x;    // transition: empty → {x}
    p.y = y;    // transition: {x} → {x, y}
    return p;
}

// Shape may vary — conditional properties create shape divergence
function createRecord(data) {
    const r = {};
    r.name = data.name;
    if (data.age) r.age = data.age;          // some objects get {name, age}
    if (data.email) r.email = data.email;    // others get {name, email}
    return r;                                // shapes may not align
}
```

### Monomorphic Call Sites

Call sites that consistently call the same function enable the optimizing compiler to inline the callee. Polymorphic call sites — where different functions are called depending on runtime conditions — limit inlining and specialization.

The degree of polymorphism that matters depends on the engine's IC implementation. A site seeing 2-4 shapes may still be handled efficiently; beyond that, engines typically give up on specialization.

### Deoptimization Triggers in Practice

| Pattern                                        | Why It Causes Issues                                    |
| ---------------------------------------------- | ------------------------------------------------------- |
| Type instability in function arguments         | Compiled code expects consistent types                  |
| Modifying object prototypes after optimization | Invalidates compiled property access assumptions        |
| `eval` / `with` usage                          | Prevents lexical scope analysis needed for optimization |
| Type coercion in arithmetic                    | Mixing types in numeric code triggers guard failures    |

### The Trade-offs Engines Navigate

| Dimension    | Startup Priority      | Throughput Priority      | Memory Priority          |
| ------------ | --------------------- | ------------------------ | ------------------------ |
| Parsing      | Lazy parse everything | Eager parse hot paths    | Lazy parse + discard AST |
| Compilation  | Interpret only        | Aggressive tiering       | Conservative tiering     |
| Optimization | Skip                  | Speculative + inline     | Limit inlining depth     |
| GC           | Simple scavenger      | Concurrent + incremental | Compact aggressively     |
| Code cache   | Don't cache           | Cache to disk            | Evict aggressively       |

No single configuration is optimal. Engines continuously evolve, and workloads vary enormously — a game engine, a spreadsheet, and a server-side handler exercise very different engine paths.
