# WebAssembly — Compilation Targets, Execution Models & System Interface

## A Portable Compilation Target

WebAssembly (Wasm) is a binary instruction format designed as a compilation target, not a programming language. Developers do not typically write Wasm directly — they write in a source language (C, C++, Rust, Go, AssemblyScript, and others) and compile to Wasm as an output format, much as they might compile to x86 or ARM machine code.

The design goals center on portability, performance predictability, and security through sandboxing. Wasm executes in a constrained environment where it cannot access memory outside its allocation, cannot make system calls directly, and cannot access the host environment except through explicitly provided interfaces.

This positions Wasm not as a replacement for any particular language, but as an intermediate layer: a common execution format that decouples the source language from the execution environment.

## The Stack-Based Virtual Machine

Wasm's execution model is a stack-based virtual machine. Instructions consume operands from and push results onto an implicit operand stack. A simple addition operates as:

```wasm
i32.const 3    ;; push 3
i32.const 4    ;; push 4
i32.add        ;; pop both, push 7
```

The instruction set is typed — operations distinguish between 32-bit integers (`i32`), 64-bit integers (`i64`), 32-bit floats (`f32`), and 64-bit floats (`f64`). Type checking occurs at validation time (before execution), meaning a well-formed module cannot produce type errors at runtime.

Control flow uses structured constructs — blocks, loops, and conditionals — rather than arbitrary jumps. This structured control flow simplifies validation and compilation to native code, as the stack discipline is always statically verifiable:

```wasm
block $outer
  loop $inner
    ;; loop body
    br_if $outer     ;; conditional break to outer block
    br $inner        ;; unconditional continue
  end
end
```

The absence of goto-style jumps is a deliberate design choice: it makes Wasm modules amenable to streaming compilation (compiling functions as they arrive over the network, before the full module has downloaded) and simplifies security analysis.

### Comparison With Register-Based Models

Most hardware architectures and many bytecode formats (Dalvik, Lua VM) use register-based models. Stack-based models produce more compact bytecodes (no register allocation in the encoding) at the cost of more instructions per operation. For Wasm, compactness matters — modules transit over the network — and the compilation to native code handles register allocation regardless.

| Aspect                 | Stack-Based (Wasm)                       | Register-Based                      |
| ---------------------- | ---------------------------------------- | ----------------------------------- |
| Encoding size          | Smaller — operands implicit              | Larger — register operands explicit |
| Instruction count      | Higher — explicit push/pop               | Lower — direct register naming      |
| Compilation complexity | Moderate — stack→register mapping needed | Lower — closer to hardware model    |
| Validation             | Simpler — stack type discipline          | More complex — liveness analysis    |

## Linear Memory

Wasm modules operate on a linear memory: a contiguous, byte-addressable array of bytes that grows in page-sized increments (64 KiB per page). This memory is the module's entire "heap" — all data structures, strings, and dynamically allocated objects reside within it.

```
Linear memory layout (conceptual):
┌─────────────────────────────────────────────────┐
│ 0x0000: Stack area (grows down in some ABIs)    │
│ ...                                             │
│ 0x1000: Static/global data                      │
│ ...                                             │
│ 0x8000: Heap (grows via memory.grow)             │
│ ...                                             │
│ End of allocated pages                          │
└─────────────────────────────────────────────────┘
```

Key properties of linear memory:

- **Sandboxed**: The host environment maps this memory to a region inaccessible to the rest of the host process. Out-of-bounds accesses trap rather than corrupting host memory.
- **Bounds-checked**: Every load and store instruction is validated against the current memory size. An access past the end triggers a runtime trap.
- **Growable**: The `memory.grow` instruction extends the memory by a specified number of pages. Growth may fail (returning -1) if the host imposes limits.
- **No garbage collection (in the base spec)**: Languages targeting Wasm that require GC must either compile their own allocator into Wasm or use the GC proposal extension (discussed below).

The sandboxed linear memory model means that even if a compiled C program has a buffer overflow vulnerability in its source, that overflow is contained within the linear memory — it cannot reach into the host process's address space. This is a fundamentally different security property from native execution.

### The Data Sharing Challenge

Passing complex data between the host (e.g., JavaScript) and Wasm requires serialization through linear memory. The host writes data at a known offset, the Wasm module reads it, and vice versa. Strings, arrays, and structured data all transit as bytes at agreed-upon memory locations.

This creates an interface boundary cost that is absent in same-language calls. For fine-grained interop (many small calls passing complex objects), this serialization overhead can negate Wasm's computational performance advantage.

## Source Language Compilation — Trade-offs

Different source languages compile to Wasm with different characteristics:

### C and C++

The most mature compilation path. C/C++ map naturally to Wasm's linear memory model — pointers become offsets into linear memory, `malloc` implementations operate within it. The compilation toolchain (typically LLVM-based) is production-grade.

Trade-offs: The resulting Wasm inherits C/C++'s memory management semantics — no GC, manual allocation, and the potential for memory bugs _within_ the sandbox. Existing codebases may require adaptation for the absence of system calls, threads (without extensions), or filesystem access.

### Rust

Compiles to Wasm with similar fidelity to C/C++, with Rust's ownership model providing memory safety guarantees at compile time. The resulting modules tend to be compact because Rust's zero-cost abstractions compile away.

Trade-offs: Rust's standard library has Wasm-aware portions, but some functionality (networking, filesystem) requires WASI or host-provided imports. The learning curve of Rust itself is the primary barrier, not the Wasm compilation.

### Go

Compiles to Wasm but carries its runtime — goroutine scheduler, garbage collector, and runtime support — into the Wasm module. This produces larger binaries (megabytes for trivial programs) compared to C/Rust.

Trade-offs: The goroutine model maps awkwardly to Wasm's single-threaded execution model (without the threads extension). TinyGo, an alternative compiler, produces much smaller binaries by subsetting the runtime, at the cost of some language feature coverage.

### AssemblyScript

A TypeScript-like language designed specifically as a Wasm compilation source. Offers a familiar syntax for web developers without the overhead of compiling a full language runtime.

Trade-offs: The TypeScript resemblance is syntactic — AssemblyScript has different semantics (no dynamic typing, no closures over Wasm-external references in the base model). It targets developers who want Wasm-level performance with a gentler learning curve.

### Garbage-Collected Languages

Languages like Java, C#, Python, and Ruby present a challenge: their runtimes include garbage collectors, exception handling mechanisms, and reflection systems that must be either compiled into Wasm (large modules) or mapped to host capabilities.

The WasmGC proposal adds garbage-collected reference types to Wasm itself, allowing these languages to use the host's GC rather than shipping their own. This is a significant architectural shift — Wasm moves from a "flat memory" model to one that understands managed objects.

| Source Language | Binary Size            | Runtime Inclusion    | GC Story                  | Maturity     |
| --------------- | ---------------------- | -------------------- | ------------------------- | ------------ |
| C/C++           | Small-medium           | Minimal              | None needed               | Production   |
| Rust            | Small                  | Minimal              | Ownership model           | Production   |
| Go              | Large                  | Full runtime         | Included in runtime       | Functional   |
| AssemblyScript  | Small                  | Minimal              | Manual/reference counting | Stable       |
| Java/C#/Kotlin  | Large (without WasmGC) | Substantial          | WasmGC or compiled-in     | Evolving     |
| Python/Ruby     | Very large             | Interpreter included | Included in interpreter   | Experimental |

## The Module/Instance/Memory Model

Wasm's execution model separates concerns into distinct concepts:

**Module**: A compiled, validated binary. Analogous to a shared library on disk — it contains code and data definitions but has no runtime state. Modules are immutable and can be compiled once, then instantiated many times.

**Instance**: A runtime instantiation of a module. Each instance has its own linear memory, global variables, and table state. Multiple instances of the same module are independent — changes in one do not affect another.

**Memory**: A specific linear memory instance. Memories can be created by the module or imported from the host. Sharing memory between instances (or between Wasm and the host) enables shared-memory concurrency patterns but introduces synchronization concerns.

**Table**: An array of opaque references (primarily function references). Tables enable indirect function calls — the equivalent of function pointers — without exposing raw addresses, maintaining the sandboxing guarantee.

```
Host Environment
├── Module A (compiled binary)
│   ├── Instance A1 → Memory A1, Globals A1, Table A1
│   └── Instance A2 → Memory A2, Globals A2, Table A2
├── Module B (compiled binary)
│   └── Instance B1 → Memory B1 (or shared with A1), Globals B1
└── Shared Memory (optional, imported by multiple instances)
```

This model enables patterns such as:

- Multiple isolated instances for multi-tenant plugin execution
- Shared memory between a Wasm instance and its host for efficient data exchange
- Hot-swapping module versions by instantiating a new module and redirecting calls

## Host Bindings and the Import/Export Interface

Wasm modules interact with their embedding environment through imports and exports:

**Exports**: Functions, memories, tables, and globals that the module makes available to the host. The host calls exported functions to invoke Wasm computation.

**Imports**: Functions, memories, tables, and globals that the module requires the host to provide. The host supplies these at instantiation time. If the host does not provide a required import, instantiation fails.

```javascript
// Conceptual: Host providing imports to a Wasm module
const importObject = {
  env: {
    log: (ptr, len) => {
      // Read string from Wasm memory and log it
      const bytes = new Uint8Array(memory.buffer, ptr, len);
      console.log(new TextDecoder().decode(bytes));
    },
    memory: new WebAssembly.Memory({ initial: 256 }),
  },
};

const instance = await WebAssembly.instantiate(module, importObject);
instance.exports.main(); // Call exported function
```

This import/export boundary is the module's entire world. A Wasm module cannot reach beyond its imports — no ambient access to the network, filesystem, DOM, or any other capability unless the host explicitly provides it. This capability-based security model means the host has precise control over what the module can do.

### The JavaScript-Wasm Interop Cost

In browser contexts, the primary host is JavaScript. Crossing the JS-Wasm boundary has costs:

- **Call overhead**: Each cross-boundary call involves type checking and potential conversion. For hot loops calling Wasm functions millions of times, this overhead is measurable.
- **Data marshaling**: Complex objects must be serialized into linear memory. There is no direct sharing of JavaScript objects with Wasm code.
- **Async mismatch**: Wasm execution is synchronous within a call. Integrating with JavaScript's async patterns requires the host to manage the async boundary.

The general guidance: move the boundary to encompass larger work units. Calling Wasm once to process a large buffer outperforms calling it thousands of times for individual elements.

## WASI — The System Interface

WebAssembly System Interface (WASI) standardizes how Wasm modules interact with operating system functionality outside the browser. Without WASI, each host environment defines its own ad-hoc import conventions for file I/O, network access, clocks, and random number generation.

WASI provides:

- A portable set of function signatures for system operations
- A capability-based security model where the host grants specific filesystem paths, environment variables, or network access at instantiation
- A POSIX-like (but not POSIX-identical) interface that allows existing C/Rust code to compile with minimal modification

```
Without WASI:                        With WASI:
┌──────────┐  ad-hoc imports  ┌───┐  ┌──────────┐  standard WASI  ┌───┐
│ Wasm mod │ ←─────────────── │ H │  │ Wasm mod │ ←────────────── │ H │
└──────────┘  (host-specific) └───┘  └──────────┘  (portable)     └───┘
                                     Same module runs on any WASI host
```

The capability model is a departure from traditional OS security:

| Traditional OS                                               | WASI Capability Model                           |
| ------------------------------------------------------------ | ----------------------------------------------- |
| Process has ambient authority from user identity             | Module has only explicitly granted capabilities |
| Any file accessible to the user is accessible to the process | Only pre-opened directories are accessible      |
| Env vars globally visible                                    | Only granted env vars visible                   |
| Network access unrestricted                                  | Network access gated by capability              |

This model makes Wasm+WASI attractive for running untrusted code: a plugin receives access to exactly one directory and no network, regardless of what the host process could access.

WASI is versioned, with `preview1` being widely implemented and `preview2` introducing the component model integration. The transition path is still evolving across runtimes.

## Performance Characteristics

Wasm's performance properties emerge from its design constraints:

**Ahead-of-time predictability**: Wasm can be compiled to native code before execution. Unlike JIT-compiled dynamic languages, there are no warmup phases where code runs interpreted before optimization kicks in. The first call to a function runs at compiled speed.

**Near-native throughput for computational work**: For CPU-bound numeric computation — image processing, physics simulation, cryptographic operations, data compression — Wasm typically achieves within 10-30% of native performance, depending on the workload and the optimizer.

**Where JIT-compiled languages hold their own**: JavaScript engines have decades of optimization for dynamic patterns — polymorphic dispatch, inline caches, speculative optimization. For code that benefits from runtime profiling (where hot paths are optimized based on observed types), JIT compilation can match or exceed ahead-of-time Wasm compilation for those specific patterns.

| Workload Type                | Wasm Advantage                                | JIT Language Advantage                      |
| ---------------------------- | --------------------------------------------- | ------------------------------------------- |
| Tight numeric loops          | Significant — predictable types, no GC pauses | Minimal                                     |
| String manipulation          | Moderate — depends on encoding handling       | JIT string optimizations may apply          |
| Object-heavy graph traversal | Depends on GC proposal adoption               | GC-integrated languages avoid serialization |
| DOM manipulation             | None — must cross the JS boundary             | Direct access, optimized bindings           |
| Startup time                 | Fast — streaming compilation                  | Varies — parsed and JIT-compiled on demand  |
| Memory usage                 | Predictable — linear memory model             | GC overhead but flexible allocation         |

The common misconception is that Wasm is "always faster." In practice, Wasm provides consistent, predictable performance with a high floor, while JIT-compiled languages have a lower floor but potentially competitive ceiling for workloads their optimizers handle well.

### Streaming Compilation

Wasm's structured format enables compilation while the binary is still downloading. A browser can compile function bodies as they arrive over the network, so that by the time the download completes, most of the module is already compiled. This is a significant advantage for large modules and makes Wasm startup time proportional to download time rather than download-plus-compile time.

## The Component Model

The component model represents Wasm's evolution toward language-agnostic composability. Where the base Wasm spec defines modules with a low-level numeric interface, the component model introduces:

**WIT (Wasm Interface Type)**: A declarative interface definition language that describes high-level types — strings, lists, records, variants, enums — independent of any source language. Components communicate through WIT interfaces rather than raw memory offsets.

```wit
// Conceptual WIT interface
interface image-processor {
    record dimensions {
        width: u32,
        height: u32,
    }

    resize: func(image: list<u8>, target: dimensions) -> list<u8>
    detect-faces: func(image: list<u8>) -> list<dimensions>
}
```

**Language-agnostic linking**: A component written in Rust can import an interface implemented by a component written in Go, with the component model handling type translation. No shared memory or ABI agreement beyond WIT is required.

**Composability without shared memory**: Components have isolated linear memories. Data crossing component boundaries is copied through canonical ABI translation. This isolation prevents components from corrupting each other but introduces copying costs at boundaries.

The component model trades raw interop efficiency for safety and language independence. Whether this trade-off serves a given use case depends on the boundary-crossing frequency and data volumes involved.

## Beyond the Browser

Wasm's portability has driven adoption in contexts far from its browser origins:

### Edge Computing

Wasm modules start in microseconds (compared to milliseconds for containers and seconds for VMs), making them attractive for serverless edge functions. The sandboxing model provides tenant isolation without the overhead of process or VM boundaries.

Trade-offs: The ecosystem of WASI-capable runtimes is younger than container runtimes. Debugging tooling, observability, and operational practices are less mature. Cold start advantage diminishes for long-running workloads where startup time is amortized.

### Plugin Systems

Applications embedding a Wasm runtime can execute user-provided plugins in a sandboxed environment with capability-controlled access. The host provides APIs through imports; the plugin cannot exceed its granted capabilities.

This pattern appears in databases (user-defined functions), web proxies (request/response transformation), game engines (mod scripting), and content management systems. The capability model is the differentiator over dynamic language embedding — a misbehaving plugin cannot access the filesystem or crash the host.

### Blockchain and Smart Contracts

Several blockchain platforms use Wasm as their smart contract execution format, leveraging deterministic execution (same inputs produce same outputs across all nodes) and the sandboxing guarantees.

Trade-offs: Determinism requires careful control over floating-point behavior, memory allocation patterns, and any source of non-determinism. The base Wasm spec provides deterministic semantics for most operations, but NaN bit patterns and resource limits require additional platform-level specification.

### Portable CLI Tools

Compiling command-line tools to Wasm+WASI produces binaries that run on any platform with a WASI runtime, without cross-compilation or platform-specific builds. A single `.wasm` file replaces per-platform binaries.

Trade-offs: Runtime installation is a prerequisite — users need a Wasm runtime. Performance may lag native compilation for I/O-heavy tools where WASI overhead accumulates.

## The Security Model

Wasm's security properties derive from its constrained execution model:

**Memory isolation**: Linear memory is bounds-checked. Buffer overflows are contained within the linear memory region — they may corrupt the Wasm module's own data but cannot reach host memory.

**No ambient authority**: Wasm modules have no inherent access to the filesystem, network, DOM, or any system resource. Every capability must be explicitly imported from the host. A module that imports nothing can only compute — it cannot observe or affect the outside world.

**Control flow integrity**: Structured control flow and typed function calls prevent code injection attacks that rely on manipulating return addresses or function pointers. Indirect calls go through typed tables, not raw addresses.

**Validation before execution**: Every Wasm module is validated (type-checked, structure-verified) before it can execute. Malformed or type-inconsistent modules are rejected entirely.

### Security Boundaries and Their Limits

The sandbox protects the host from the module, but several boundary conditions deserve attention:

| Threat                            | Sandbox Protects? | Notes                                                                |
| --------------------------------- | ----------------- | -------------------------------------------------------------------- |
| Memory corruption of host         | Yes               | Linear memory is isolated                                            |
| Denial of service (infinite loop) | Partially         | Host must implement fuel/gas metering or timeouts                    |
| Side-channel attacks (timing)     | No                | Wasm execution timing is observable                                  |
| Logic bugs within the module      | No                | The sandbox does not prevent internal corruption                     |
| Import abuse                      | Depends           | Security depends on what the host provides as imports                |
| Supply chain (malicious module)   | Partially         | The sandbox contains damage, but granted capabilities can be misused |

The import surface is the critical security decision. A Wasm module with filesystem write access to the root directory is sandboxed in theory but dangerous in practice. The capability model only works when capabilities are granted with appropriate narrowness.

### Spectre and Timing Considerations

Wasm's execution in shared processes (particularly browsers) inherits Spectre-class vulnerability concerns. Mitigations include site isolation (running different origins in different processes), timer resolution reduction, and compiler-level speculation barriers. These mitigations are host-environment concerns rather than Wasm-spec concerns, but they affect Wasm's effective security profile in practice.

## Current Trajectory and Evolving Proposals

The Wasm specification evolves through a proposal process. Notable directions:

- **Threads**: Shared memory and atomic operations for parallel computation
- **SIMD**: Fixed-width vector operations for data-parallel workloads
- **GC**: Managed reference types, enabling efficient compilation from GC languages
- **Exception handling**: Structured exception support for languages that use them
- **Component model**: High-level composability through WIT interfaces
- **Stack switching**: Coroutine and green-thread support without compile-time transforms

Each proposal extends Wasm's reach into new domains while maintaining the core properties of safety, portability, and predictable performance. The tension in the standardization process is between keeping Wasm minimal (simple to implement, reason about, and secure) and expanding it to support the breadth of programming paradigms that target it.
