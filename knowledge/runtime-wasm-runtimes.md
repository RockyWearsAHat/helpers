# WebAssembly Runtimes — Engines, Compilation Strategies, and WASI

## Overview

WebAssembly (Wasm) execution requires a runtime that interprets Wasm bytecode or compiles it to machine code. Unlike JavaScript engines, Wasm runtimes vary sharply in compilation strategy: some use interpreters, others use JIT, and others AOT-compile ahead of time. This choice determines startup latency, peak performance, memory usage, and deployment constraints.

## Wasmtime — Cranelift JIT

Wasmtime (Bytecode Alliance) is the dominant open-source Wasm runtime, written in Rust:

### Cranelift JIT Compiler

Wasmtime uses **Cranelift**, a JIT compiler designed for fast compilation (prioritizing code generation speed over optimization height):

- **IR design**: Cranelift compiles Wasm to an intermediate representation (IR), then lowers to machine code
- **Single-pass lowering**: Code generation happens in one pass, avoiding multiple compilation phases
- **Registers allocation**: Cranelift uses a fast linear-scan register allocator, not IRA (iterated register allocation), trading peak quality for speed

Performance model: A Wasm module compiles to machine code in milliseconds, enabling near-native execution immediately (unlike V8, which profiles and re-optimizes). Startup is fast; peak performance is good but not maximized.

### Lazy Compilation

Wasmtime uses **lazy compilation**:

- When a Wasm module is instantiated, only exported functions are compiled immediately
- Imported functions (calls from JavaScript to Wasm) trigger compilation on first call
- Unused internal functions remain uncompiled, reducing memory footprint

### Code Caching

Optional serialization of compiled code:

```rust
let compiled = engine.precompile_module(&wasm_bytes)?;
std::fs::write("compiled.wasm", compiled)?;

let engine = Engine::new(&config)?;
let module = unsafe { Module::deserialize(&engine, &compiled_bytes)? };
```

Precompilation avoids recompilation across runs, critical for serverless use cases.

## Wasmer — Multi-Engine Architecture

Wasmer (open-source) distinguishes itself by supporting **multiple compilation backends**:

### Engine Implementations

- **Singlepass compiler**: Single-pass compilation, minimal IR, for ultra-fast startup (trade-off: less optimization)
- **Cranelift**: Same as Wasmtime (can be plugged in as Wasmer's middle-tier)
- **LLVM backend**: Invokes LLVM for aggressive optimization; slower compilation but better peak performance

### Singlepass Engine Details

The Singlepass compiler prioritizes **minimal compilation overhead**:

- Compiles directly from Wasm bytecode to machine code in one traversal
- No intermediate IR; avoids data structure allocation and traversal
- Register allocation is simpler (greedy, not optimal)
- Trade-off: Generated code is larger, less optimized

Ideal for ephemeral workloads (one-time scripts, short-lived functions) where startup dominates; poor for long-running servers.

### Engine Selection

At runtime:

```rust
use wasmer::{Store, Engine, Singlepass, Cranelift, LLVM};

let engine = Singlepass::default();  // Fastest startup
let engine = Cranelift::default();   // Balanced
let engine = LLVM::default();         // Best peak perf
let store = Store::new(&engine);
```

Different modules can use different engines in the same process.

## WasmEdge — LLVM AOT Compilation

WasmEdge (Linux Foundation) focuses on **ahead-of-time (AOT) compilation**:

### AOT Strategy

- Pre-compile Wasm modules to native machine code before deployment
- Store AOT native code alongside Wasm
- At runtime, load pre-compiled native code, skipping JIT entirely

Benefits:

- **Zero startup latency**: No compilation overhead; native code is ready immediately
- **Predictable performance**: No JIT warmup pauses
- **Reduced resource usage**: AOT compilation happens offline; runtime memory is minimal

Trade-off: AOT requires an offline compilation step and platform-specific builds (x86_64 binary cannot run on ARM).

### LLVM Backend

WasmEdge uses LLVM for AOT compilation, enabling aggressive optimization:

- Wasm → LLVM IR → machine code
- LLVM's passes (dead code elimination, loop unrolling, vectorization) apply to Wasm

### Deployment Model

```bash
wasmedgec app.wasm app.so  # Offline: compile to native .so
wasmedge app.so            # Runtime: load pre-compiled
```

WasmEdge is optimized for edge computing and embedded systems where predictability and minimal resource footprint matter more than development convenience.

## Browser Engines — Liftoff and TurboFan

Browsers (Chrome, Firefox, Safari) implement Wasm engines optimized for web performance:

### Chrome/V8: Liftoff + TurboFan

**Liftoff** is baseline Wasm compiler:

- Compiles Wasm to machine code quickly with minimal optimization
- Stack allocation strategy for runtime speed (avoids register pressure analysis)
- Enables fast function entry (useful for frequently-called functions)

**TurboFan** is the optimizing tier:

- Monitors Liftoff-compiled code; profiles hot paths
- Re-compiles hot functions with aggressive optimization
- Uses type feedback, inline caching, and speculative optimization

Tiered approach ensures fast startup (Liftoff ready in milliseconds) + peak performance (TurboFan warmup).

### Firefox: SpiderMonkey Wasm

Firefox implements a baseline (Ion) and optimizing (Baseline) Wasm compiler. Similar tiered philosophy as Chrome.

## WASI — WebAssembly System Interface

WASI is a standardized set of system calls for Wasm modules:

### API Surface

WASI provides:

- **File operations**: `fd_read`, `fd_write`, `fd_seek` (file descriptor model)
- **Directory operations**: `path_open`, `path_rename`, `path_unlink`
- **Process control**: `proc_exit`, `environ_get`, `args_get`
- **Clocks**: `clock_time_get` for wall-clock and monotonic time

### Capability-Based Access

WASI uses **capability-based security**:

- File descriptors are opaque integers; Wasm cannot forge them
- Opening a directory grants access only to files within that directory
- No ambient access to the file system; all access is explicitly granted

Example (pseudo-code):

```c
// WASI module
int root_fd = fd_open("/tmp", ...);  // Open /tmp directory
int file_fd = fd_open_relative(root_fd, "file.txt", ...);  // Open relative to root
fd_read(file_fd, buffer, 1024);  // Read from file
```

### I/O Model

WASI I/O is synchronous at the Wasm level (module calls `fd_read`, blocks until data available). The hosting runtime manages the wait (via select/epoll/async-await).

## Memory Model — Linear Memory

All Wasm runtimes implement **linear memory**:

- Wasm module has a linear address space (contiguous memory area)
- Accessible via `i32.load`, `i32.store` opcodes (load/store from memory)
- Host (JavaScript, Rust, etc.) can access the memory buffer

### Shared Memory (Wasm Threads)

Wasm supports shared linear memory (`SharedArrayBuffer` in JavaScript):

```javascript
const shared_memory = new WebAssembly.Memory({ shared: true, initial: 256, maximum: 512 });
const module1 = new WebAssembly.Instance(wasm1, { env: { memory: shared_memory } });
const module2 = new WebAssembly.Instance(wasm2, { env: { memory: shared_memory } });
```

Both modules access the same memory; race conditions are possible, requiring atomic operations (`i32.atomic.load`, `i32.atomic.store`, `i32.atomic.compare_exchange`).

## Component Model — Runtime Support

WebAssembly **Component Model** is an emerging standard for composing Wasm modules with explicit interface contracts:

### Component Definition

A component is a Wasm module plus type definitions (WIT format):

```wit
package my:component;

world example {
  export greet: func(name: string) -> string
}
```

### Runtime Support

Runtimes like Wasmtime and Wasmer support component loading:

```rust
use wasmtime::component::*;

let engine = Engine::default();
let component = Component::from_file(&engine, "component.wasm")?;
let mut linker = Linker::new(&engine);
let instance = linker.instantiate(&mut store, &component)?;
```

The linker automatically handles:
- Type checking between component exports and imports
- Memory sharing between modules
- Resource management (file handles, sockets)

## Performance Comparison

| Runtime | Compilation Strategy | Startup Latency | Peak Performance | Memory |
|---------|---|---|---|---|
| Wasmtime | Cranelift JIT | ~10-50ms | Near-native | Moderate |
| Wasmer (Singlepass) | Single-pass JIT | ~1-5ms | Good | Low |
| Wasmer (Cranelift) | Cranelift JIT | ~10-50ms | Near-native | Moderate |
| Wasmer (LLVM) | LLVM JIT | ~100-500ms | Best | Higher |
| WasmEdge | AOT | ~0ms | Best | Very low |
| Chrome V8 | Liftoff + TurboFan | ~5-10ms (baseline) | Best (after warmup) | Moderate |

**Workload fit**:

- **One-off scripts**: Singlepass or AOT wins
- **Edge/embedded**: WasmEdge AOT
- **Long-running servers**: Wasmtime or LLVM backend (peak performance matters)
- **Web browsers**: V8 Liftoff/TurboFan (tiered optimization built-in)

## Exception Handling and Stack Traces

Wasm has limited exception semantics. Runtimes implement:

- **Trap mechanism**: Illegal ops (divide by zero, out-of-bounds memory) trigger traps
- **Stack unwinding**: Wasm stack is unwound; function frames are discarded
- **Host exception mapping**: Host language exceptions (JavaScript Error, Rust panic) are caught at module boundaries

Stack traces across Wasm/host boundaries remain challenging; debuggers rely on DWARF debug info embedded in Wasm modules.

## Multi-Threading (Wasm Threads)

Wasm modules support multi-threading via `WebAssembly.Worker` (browser) or `wasm-bindgen-rayon` (shared-memory parallelism):

- Multiple module instances share linear memory
- Atomic operations (`i32.atomic.compare_exchange`) handle synchronization
- Runtimes must ensure memory isolation: no memory access outside the module's linear address space

Most Wasm runtimes execute a single module instance per thread; true parallelism requires explicit OS threads.