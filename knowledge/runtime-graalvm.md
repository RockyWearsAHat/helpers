# GraalVM — Polyglot Runtime, Truffle Framework, and Graal JIT

## Overview

GraalVM (Oracle) is a polyglot virtual machine enabling JavaScript, Python, Ruby, R, Java, and other languages to run in the same process with seamless interoperability. Unlike traditional runtimes (V8, CPython, JVM) which optimize a single language, GraalVM uses a **common compilation infrastructure** (Truffle + Graal) to optimize multiple languages uniformly.

## Architecture Layers

### Layer 1: Truffle Language Framework

**Truffle** is a Java library for implementing language interpreters:

- Language implementations are written as **Abstract Syntax Tree (AST) interpreters** in Java
- AST nodes are ordinary Java objects; executing the program walks the tree
- Specialization: Nodes replace themselves with optimized versions based on runtime behavior

Example (simplified Ruby `+` operator):

```java
@NodeChildren({ "left", "right" })
abstract class AddNode extends Node {
  @Specialization(guards = "isInteger(a, b)")
  int add(Object a, Object b) { return (int) a + (int) b; }

  @Specialization(guards = "isDouble(a, b)")
  double add(Object a, Object b) { return (double) a + (double) b; }
  // ... more specializations
}
```

At runtime, the node starts unspecialized. When first called with integers, it specializes to `addInt()`. If a string is later passed, the node despecializes or adds another specialization.

### Layer 2: Partial Evaluation

GraalVM uses **partial evaluation** to compile Truffle ASTs to machine code:

- The GraalVM compiler analyzes the AST and visitor methods statically
- It **partially evaluates**: Treats the interpreter loop (`NodeContinuation.execute()`) as compile-time-constant
- Result: The interpreter loop is unrolled away, leaving only domain-specific code

Example: An interpreter with a generic `execute()` dispatcher compiles away. The compiled code is as if the specialization was hand-written:

```java
// Interpreted: dispatch to correct node, call execute()
// Compiled (after partial eval): direct inlined add operation
```

This magic is why Truffle interpreters rival hand-optimized compilers. The cost is **compilation time**: partial evaluation is expensive; runtimes use profiling to decide when to compile.

## Graal JIT Compiler

**Graal** is a Java-based JIT compiler integrated with GraalVM:

### Compilation Pipeline

1. **Profiling**: Interpreter collects type feedback (seen types, branch outcomes, call targets)
2. **Tier 1 compilation**: Limited optimization (inlining depth, loop unrolling) for fast feedback
3. **Tier 2 compilation**: Aggressive optimization (speculative inlining, dead code elimination, escape analysis)
4. **Deoptimization**: If assumptions fail (e.g., inlined method is overridden), revert to interpreter

### Specialization-Based Optimization

Unlike V8 (which uses inline caches), Graal uses **method specialization**:

- If a frequently-called method is polymorphic (receives objects of different types), Graal generates multiple specialized versions
- Call site is patched to direct call + type check
- Speeds up virtual dispatch without V8's method lookup overhead

### Graal IR (Intermediate Representation)

Graal compiles to its own IR, not LLVM:

- High-level IR (HIR): Represents operations on objects, not bytes
- Mid-level IR (MIR): Nodes have explicit effects (reads, writes, allocations)
- Low-level IR (LIR): Machine-level operations (moves, arithmetic)
- Machine code generation: LIR is converted to native assembly

Advantage: Graal can represent domain-specific optimizations (e.g., Ruby method lookup as a high-level node) that LLVM backends cannot understand.

## Polyglot Execution Model

### Language Interop

GraalVM languages share a common object model:

```javascript
// JavaScript calling Ruby
const ruby = await Polyglot.evaluate("ruby", "class Adder; def add(a, b); a + b; end; end");
const result = ruby.add(1, 2);  // Type coercion handled automatically
```

### Value Wrapping and Unwrapping

- When crossing language boundaries, values are wrapped in a **PolyglotValue** object
- The wrapper tracks the source language type and implements **automatic coercion**
- Example: JavaScript `1 + rubyBigNum` coerces the Ruby BigNum to JavaScript and performs addition

### Shared Heap

All languages allocate from the **same garbage collector** (G1GC or ZGC, depending on configuration):

- No serialization/deserialization at language boundaries
- No copying between language runtimes; pointers are direct

This tight integration enables efficient polyglot applications (e.g., JavaScript calling specialized Python for ML, then processing results in Ruby).

## Native Image — Ahead-of-Time (AOT) Compilation

GraalVM **Native Image** compiles a JVM application (including Truffle interpreters) to a **standalone binary**:

### Process

1. **Analysis phase**: Trace which code is reachable from the entry point
2. **Compilation**: Graal JIT compiles reachable code to machine code (no interpreter)
3. **Linking**: Link runtime libraries (GC, thread scheduler) into the binary
4. **Output**: Standalone ELF/Mach-O binary with no JVM dependency

Result: A binary that starts in ~10-50ms (vs. JVM warmup of 1+ seconds) and is ready for execution immediately.

### Reflection Configuration

Static analysis cannot detect all runtime reflection:

```java
String className = getUserInput();
Class<?> cls = Class.forName(className);  // Dynamic class loading
Object instance = cls.newInstance();
```

Native Image cannot determine which classes are instantiated at compile-time. Developers must provide a **reflection configuration** (JSON):

```json
[
  {
    "name": "com.example.Adder",
    "methods": [{ "name": "<init>", "parameterTypes": [] }],
    "fields": [{ "name": "result" }]
  }
]
```

Alternatively, use **configuration agents** (bytecode instrumentation during profiling) to record reflective accesses, then generate the config automatically.

### Substrate VM

The **Substrate VM** is the runtime used in Native Image binaries:

- Minimal JVM: No interpreter, JIT, or class loader
- Pre-compiled code only
- Custom garbage collector (serial, parallel, or ZGC)
- Stripped metadata: Class definitions are baked into the binary

Memory overhead is low: A "Hello World" Native Image binary is ~10MB (vs. ~500MB for a JVM distribution).

## Sulong — LLVM Language Support

**Sulong** (subset of LLVM) enables GraalVM to execute C/C++ code:

- C/C++ is compiled to LLVM IR
- Sulong interprets LLVM IR using Truffle
- LLVM IR can be JIT-compiled by Graal for performance

Benefits:

- Seamless C/C++ interop: Call C functions from JavaScript, pass callbacks to C, etc.
- No FFI bridge: C code runs in-process, shared memory with host language

Example (JavaScript calling C):

```javascript
const clib = Polyglot.eval("llvm", "path/to/lib.c");
clib.c_function(arg1, arg2);
```

## Performance Model and Tuning

### Profiling-Guided Optimization

GraalVM relies heavily on **profile feedback**:

1. Interpreter runs and collects type information
2. Compilation threshold is reached (e.g., 10k loop iterations)
3. Method is compiled using profile feedback
4. If profile assumptions break, reoptimize

Trade-off: Interpreter overhead is significant initially; peak performance requires warmup. For CLI tools and short-lived processes, V8 or Bun may be faster.

### Tiered Compilation Levels

```
Level 0: Interpreter (always available)
Level 1: Tier 1 Graal (fast, limited optimization)
Level 2: Tier 2 Graal (aggressive optimization)
Level 3+: Specialized compilation (polyglot boundary handling, intrinsics)
```

### Memory Overhead

- **Interpreted**: Full AST nodes, metadata → higher memory
- **Compiled**: Native code + metadata → lower per-method memory, but higher total due to multiple specializations

GraalVM typically uses 2-3x more memory than specialized runtimes (V8, CPython) for equivalent workloads due to multi-language support overhead.

## Partial Evaluation Deep Dive

Partial evaluation is GraalVM's core innovation. Simplified example:

```java
// Generic interpreter loop
public int interpret(Node root, Frame frame) {
  return root.execute(frame);  // Dynamic dispatch
}

// After partial evaluation:
public int interpret_specialized(Frame frame) {
  // Direct add operation, all branches eliminated
  return (int)frame.getLocal(0) + (int)frame.getLocal(1);
}
```

The compiler:

1. Statically analyzes `execute()` with the profile (e.g., always AddNode)
2. Inlines AddNode's execute() method
3. Removes guard checks (already specialized)
4. Unrolls AST traversal
5. Generated code is nearly as fast as hand-written C

Trade-off: Compilation time is high (~100-1000ms per method), but one-time cost.

## Use Cases and Limitations

### Strengths

- **Polyglot agility**: Mix languages within one process
- **Peak performance**: Competitive with single-language runtimes after warmup
- **AOT guarantees**: Native Image enables predictable startup and zero JVM overhead
- **Debugging across languages**: Single debugger for all languages

### Limitations

- **Startup overhead**: JVM warmup (if not using Native Image) is slow
- **Memory footprint**: All languages compiled in process; larger binary than single-language runtimes
- **Ecosystem fragmentation**: Polyglot features not available in single-language modes
- **Type coercion surprises**: Auto-coercion at language boundaries can mask bugs
- **Garbage collection pauses**: Full GC pauses (even in low-pause collectors) can exceed specialized runtimes

## Practical Configuration

GraalVM ships with various profiles:

- **`graalvm-core`**: Minimal runtime (Java + Truffle framework)
- **`graalvm-ee`** (Enterprise Edition): Optimized compiler, profiling tools, advanced GC
- **Community Edition**: Free, open-source, includes most features

Minimal Native Image reduction:

```bash
native-image --no-fallback -H:+ReportUnsupportedElementsAtRuntime myapp MyApp
```

This enables aggressive AOT elimination but may crash if unsupported reflection is encountered at runtime.