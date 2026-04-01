# .NET Runtime Internals

## Overview

The .NET runtime (CLR/CoreCLR) executes managed code through a tiered compilation strategy that balances startup performance (via lightweight interpretation) with peak throughput (via aggressive JIT optimization). Unlike traditional single-tier JIT systems, .NET uses profiling-driven feedback to allocate compilation effort where it matters most.

## Runtime Architecture

### CLR vs. CoreCLR

The **CLR** (Common Language Runtime) is the original .NET Framework runtime (.NET Framework 4.x). **CoreCLR** is the open-source runtime powering .NET Core (.NET 5+), with the same core architecture but cross-platform and lighter.

Both compile **CIL** (Common Intermediate Language, formerly MSIL) to machine code via JIT. Key components:

- **Type system & verification** — CIL is strongly typed; verifier ensures type safety before execution
- **Assembly loading** — .NET assemblies (DLLs/EXEs) are discovered and metadata is inspected
- **Execution engine** — Dispatches native code or interprets CIL transitions

## Assembly Loading & Resolution

### Assembly Discovery

Unlike the JVM's three-tier classloader, .NET uses a single **assembly loader** that searches:

1. Global Assembly Cache (GAC) — system-wide shared assemblies (deprecated in .NET Core)
2. Application base directory — typically where the .exe resides
3. Configured probe paths — specified in config files or API

.NET 5+ simplified this: assemblies are typically deployed transively in the application directory (`bin/`) with the executable. TFMs (target framework monikers) specify which runtime to use.

### Metadata & Type Discovery

.NET assemblies contain rich metadata—every type, method, and field is described in binary headers. The runtime parses this metadata at load time, building internal representations for dispatch and verification.

The **Type Loader** builds a type handle (a pointer to internal type data) for every type referenced at runtime, caching them to avoid repeated parsing.

## Assembly Compilation & Verification

### Verification Phase

CIL bytecode is verified similarly to JVM bytecode—type safety, stack consistency, reference validity. Verification ensures:

- All instructions are valid CIL
- All type references are resolvable
- Stack operations are type-correct
- Security boundaries are respected

However, verification is **deferred** in many paths; hot methods may be JIT-compiled before full verification completes.

### JIT Early in Execution

Unlike the JVM which interprets first, .NET JITs methods on first call (or first hot path). This trades startup cost for simpler system design and better optimization opportunities.

## RyuJIT: The JIT Compiler

### Architecture

**RyuJIT** is .NET's primary JIT compiler, written in C++ with a register-based intermediate representation. It performs:

1. **CIL to IL** — Parse CIL, build control flow graph
2. **IL optimization** — Constant folding, dead code elimination, inlining
3. **Register allocation** — Map variables to CPU registers
4. **Code generation** — Emit machine code with GC info

The compiler prioritizes inlining (more aggressive than JVM) to expose optimization opportunities.

### Compilation Tiers

**.NET 6+** implements **tiered compilation** by default:

| Tier | Compiler | Profiling | Latency | Throughput | Phase |
|------|----------|-----------|---------|-----------|-------|
| 0 | Lightweight JIT | None | ~1ms | Fair | Warmup |
| 1 | Full RyuJIT | Tier 0 counters | ~10ms | Excellent | Hot paths |
| 2 | RyuJIT + optimizations | Tier 1 counters | ~50ms | Best | Very hot |

**Tier 0** uses a fast, non-optimizing backend to get code runnable quickly. When a method exceeds a counter threshold (e.g., 100 calls), the runtime re-JITs it at Tier 1, using profiling data from Tier 0:

```
First call → Tier 0 JIT (lightweight, 10-30μs)
           → Code runs, counters increment
           → Counter threshold reached → Tier 1 JIT (aggressive, 100-500μs)
           → Tier 1 code replaces Tier 0
```

This adaptive strategy recovers the **JIT pause** (time spent compiling) that would otherwise block initial invocations.

## Generational Garbage Collection

### Heap Organization

.NET uses a **generational, mark-and-sweep** collector:

```
Generation 0 (Gen0): Young objects, collected often (~10GB/sec throughput)
Generation 1 (Gen1): Intermediate survivors, collected less often
Generation 2 (Gen2): Long-lived objects, collected rarely
```

**Variable-length objects (LOH)** >85KB live separately in the **Large Object Heap** to avoid fragmentation.

**Pinned Object Heap (POH)**, added in .NET 5, segregates pinned objects to reduce GC bookkeeping.

### GC Modes

| Mode | Threads | Pause Latency | Throughput | Use |
|------|---------|---------------|-----------|-----|
| Workstation | Single | Low | Fair | Interactive/desktop |
| Server | Per-core threads | Higher | Excellent | Server/backend |
| Interactive | Concurrent | Low | Fair | UI apps (Windows only) |

**Workstation GC** pauses the entire app for collections, favoring latency — suitable for UI. **Server GC** runs per-CPU background threads during collection, maximizing throughput for batch jobs.

### Concurrent Marking

Modern .NET (7+) performs background marking concurrent with application threads, reducing pause times. When marking completes, a brief **final pause** sweeps unmarked objects.

## Memory: Span<T> & Memory<T>

### Stack-Safe References

Traditional .NET uses **boxing** for value types, copying structs when passed. This wastes memory and pressures GC.

**Span<T>** (C# 7.2+) references existing memory without allocation:

```csharp
int[] array = new int[100];
Span<int> span = array;
Process(span);  // No copy, no allocation
```

Spans are **stack-allocated** and scoped — they reference external memory but are freed when the stack frame exits. This eliminates hundreds of allocations in hot loops.

**Memory<T>** is the heap-allocated variant, usable across async boundaries (unlike Span).

## Ahead-of-Time Compilation (NativeAOT)

### Closed-World Assumption

**NativeAOT** (available since .NET 7, production-ready in .NET 8+) compiles .NET apps entirely to native code at build time, eliminating the JIT:

```
.NET app → dotnet publish -c Release -p:PublishAot=true → native.exe
```

Advantages:
- Instant startup (~10-100ms vs. 500ms+)
- Single executable (no runtime dependency)
- Smaller deployment footprint (~20-50MB vs. 200MB+)

Constraints:
- Closed-world: all reachable types must be known at build time
- No dynamic loading, reflection (unless explicitly pre-registered)
- Smaller code (dead code eliminated) but cannot JIT warmup

AOT is ideal for microservices, containerized workloads, and cloud functions where startup matters and code is static.

## Hot Reload & Minimal APIs

### .NET Minimal APIs

**Minimal APIs** (ASP.NET Core 6+) strip boilerplate from web services:

```csharp
var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();
app.MapGet("/", () => "Hello!");
app.Run();
```

This reduces compilation overhead and improves startup time for simple services.

### Hot Reload

**.NET 6+** supports **hot reload** in development: modify code and reload in place without restarting. The runtime appliesCodecs CIL edits to running code, updating method tables. This accelerates inner-loop debugging.

## Blazor WebAssembly

**Blazor WASM** compiles C# to WebAssembly and runs in the browser, reusing the .NET runtime:

- Browser downloads the .NET runtime (~3-5MB) + app assemblies
- JIT or AOT compiles CIL to WASM at runtime or build-time
- .NET code executes in the browser with near-native performance

Blazor bridges server/.NET development with web frontend constraints, enabling C# developers to write client-side UIs.

## Design Philosophy

.NET runtime internals balance:

- **Startup vs. peak**: Tier 0 JIT enables fast code generation; Tier 1/2 optimize later
- **Safety vs. performance**: Verification ensures correctness but defers; JIT assumes verified code
- **Generality vs. specialization**: Single runtime handles Windows/Linux/Mac; AOT specializes
- **Memory footprint vs. optimization**: Span<T> reduces GC pressure; NativeAOT eliminates runtime overhead

Compared to the JVM (which interprets first, JIT later), .NET JIT-compiles immediately but with light optimizations. This trades faster cold startup (JIT pauses) for simpler, more predictable compilation decisions.

## See Also

- Just-in-time compilation architecture, tiered compilation strategy
- Generational garbage collection, concurrent marking techniques
- Stack allocation and reference lifetimes, memory safety without GC