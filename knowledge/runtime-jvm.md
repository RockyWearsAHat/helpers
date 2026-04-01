# JVM Runtime Internals

## Overview

The Java Virtual Machine executes bytecode through a complex multi-stage pipeline that balances class loading safety, runtime optimization, and memory management. Understanding JVM internals reveals how performance emerges from the interaction between class verification, adaptive compilation strategies, and generational garbage collection.

## Class Loading Architecture

### The Bootstrap Process

Class loading follows three delegation tiers:

1. **Bootstrap ClassLoader** — loads core JDK classes from `rt.jar` (Java < 9) or module files (Java 9+). Purely native, no user override.
2. **Extension ClassLoader** — loads extension APIs from `jre/lib/ext/` or module path extensions.
3. **Application ClassLoader** — loads user application code from the classpath.

The **parent delegation model** ensures that if a child loader receives a request, it first asks its parent. Most classes resolve through the bootstrap loader, reducing redundancy. Custom classloaders can override this pattern for specialized scenarios (e.g., plugin systems, classloader isolation).

### Verification: Defensive Execution

Bytecode verification contains two strategies:

| Strategy | Timing | Scope | Cost | Mode |
|----------|--------|-------|------|------|
| Full verification | Class load | All classes | Higher upfront | Default < Java 13 |
| Runtime verification | First use | On-path methods | Lower upfront, amortized | Modern JVMs |
| `-XX:-BytecodeVerificationLocal` | Skip non-critical | Unverified paths | Risky | Trusted environments |

Verification checks structural invariants: stack frame consistency, type safety, reference validity, privilege boundaries. The verifier treats bytecode as untrusted until proven safe, examining every instruction path.

## Execution Engine & JIT Compilation

### Interpretation vs. Compilation

New code enters **C1 (Client JIT)** after 1,500 invocations (default): a fast, conservative compiler optimizing for startup. Hot methods (10,000+ invocations) graduate to **C2 (Server JIT)** or **Graal**, aggressive optimizers using deep profiling.

**Tiered compilation** (enabled by default in Java 8+) stacks multiple layers:
- **Tier 0**: Interpreter + C1 instrumentation
- **Tier 1**: C1 non-profiling (medium optimization)
- **Tier 2**: C1 profiling (gathers stats)
- **Tier 3**: C2 or Graal (speculative, deep optimization)

Methods downgrade (deoptimize) if optimization assumptions fail—e.g., a monomorphic call site (single receiver type) becomes polymorphic, breaking in-line cache validity.

### Graal and Project Leyden

**GraalVM** provides a general-purpose compiler written in Java, enabling native image compilation (AOT). **Project Leyden** aims to pre-optimize and checkpoint JVM state for faster startup without sacrificing peak throughput.

## Memory Model

### Heap Organization

The JVM heap traditionally divides into **Young** (80% of heap) and **Old** generations:

- Young objects die quickly; the Young gen uses mark-sweep, holding live objects in survivor spaces
- Old objects persist; the Old gen uses more sophisticated algorithms

### Garbage Collectors

| Collector | Gen | Pause | Throughput | Memory | Use Case |
|-----------|-----|-------|-----------|--------|----------|
| Serial GC | Yes | High | Good | Low | Single-threaded apps, embedded |
| Parallel GC | Yes | High | Excellent | Low | Batch jobs, known load |
| G1 | Yes | Low | Good | Moderate | General-purpose, >4GB heaps |
| ZGC | No | Ultra-low (<10ms) | Fair | Moderate | Low-latency, large heaps |
| Shenandoah | No | Low | Fair | Moderate | Interactive, consistent latency |

**G1 (Garbage First)** divides heap into regions (1-32MB each) and collects lowest-garbage regions first. Concurrent marking runs parallel to application threads, reducing pause times to 10-100ms for multi-gigabyte heaps.

**ZGC** uses read barriers to track concurrent mark progress; pauses scale with root set size, not heap size. Handles 16TB+ heaps with <1ms pauses.

**Shenandoah** (Red Hat) relocates objects concurrently without stopping the application, achieving predictable latency for demanding systems.

### Metaspace

Pre-Java 8, class metadata lived in the PermGen (separate heap region, fixed size). **Metaspace** (Java 8+) allocates metadata off-heap, removing PermGen sizing headaches. Metaspace uses arena allocators per classloader; unloaded classloaders' metadata is reclaimed automatically.

## Advanced Features

### JMX & Monitoring

The Java Management Extensions expose runtime metrics through managed beans:

```
MBeanServer → MemoryMXBean (heap/non-heap usage, GC info)
            → ThreadMXBean (thread count, lock contention)
            → CompilationMXBean (JIT method count, time)
            → RuntimeMXBean (uptime, VM version)
```

Remote JMX allows off-process monitoring via `jstatd` or remote debug port.

### Native Image (GraalVM)

Closed-world assumption: statically analyze reachable classes at build time, compile to native executable. Enables instant startup (10-100ms), tiny footprint (10-50MB), but loses dynamic classloading, reflection, and JIT warmup benefits.

### Project Loom: Virtual Threads

Virtual threads (available since Java 19 preview, standardizing in Java 21) run 100,000+ lightweight OS-independent threads on a small pool of kernel threads, eliminating thread-per-request models and context-switch overhead.

## Design Tensions

JVM internals balance competing goals:

- **Startup vs. peak throughput**: Interpreter is fast to start but slow; JIT brings peak performance at the cost of warmup.
- **Safety vs. performance**: Verification ensures correctness but costs microseconds per class; tiered JIT defers verification.
- **Memory footprint vs. optimization**: Logging profiling data aids optimization but increases memory; Graal aims to reduce metadata bloat.

The JVM's adaptive nature—moving between tiers, selecting GC algorithms, deoptimizing when assumptions break—makes deterministic performance analysis difficult but allows single runtime to suit diverse workloads.

## See Also

- Compiler internals, garbage collection fundamentals, JIT compilation trade-offs
- Runtime memory models, reference semantics in concurrency