# WebAssembly — Binary Format, Memory Model & Runtimes

## Overview

WebAssembly (Wasm) is a binary intermediate representation (IR) designed for portable, efficient execution. Unlike web technologies, Wasm is not restricted to browsers — it targets any environment: servers, embedded systems, and edge devices. The key insight is **stack machine discipline**: a linear memory model, structured control flow, and typed instructions enable near-native performance while maintaining sandboxing.

## Binary Format (.wasm)

### Structure

Every Wasm module is a binary file composed of sections. Each section contains type-specific data:

```
magic number: 0x00 0x61 0x73 0x6d (\0asm)
version: 0x01 0x00 0x00 0x00 (v1)
[section]* 
```

Sections are identified by an ID and include:

- **Type**: Function signatures (parameters, return types)
- **Import**: External imports (functions, tables, memory, globals)
- **Function**: Declaration of functions in the module (references type signatures)
- **Table**: Definition of tables (indirect function calls)
- **Memory**: Linear memory declarations (pages, limits)
- **Global**: Global variables (mutable or immutable)
- **Export**: Which definitions are exposed to the host
- **Start**: Entry point function (optional)
- **Element**: Initial values for tables
- **Data**: Initial memory contents (copied at instantiation)
- **DataCount**: Metadata for data segments
- **Code**: Function bodies (the actual bytecode)
- **Custom**: Application-specific metadata (debugging, version info)

Each instruction is encoded as a single byte (opcode) followed by optional immediate operands. Variable-length encoding (LEB128) compresses large numbers.

### Type System

Wasm has four value types:

- `i32`, `i64`: integers
- `f32`, `f64`: IEEE 754 floats
- `v128`: SIMD vectors (128-bit, MVP feature)
- `funcref`, `externref`: reference types (enabling closures and callbacks)

Reference types break the "no pointers" rule intentionally — they allow safely storing function pointers and opaque host objects without exposing raw memory.

## Text Format (WAT)

WAT (WebAssembly Text) is a human-readable S-expression syntax. Every `.wasm` binary maps to an equivalent `.wat` source:

```lisp
(module
  (func $add (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add)
  
  (export "add" (func $add)))
```

**Parsing WAT:**
- `local.get N`: push local variable N
- `local.set N`: pop and store to local N
- `i32.add`: pop two values, push sum
- Inline format: `(result i32)` declares return type; stack discipline is verified at compile time

**Advantages of WAT:**
- Debugging: inspect module structure and function bodies
- Hand-writing: small modules, tests, educational examples
- Tooling: decompilers (`wasm2wat`) convert binaries back

Text and binary are isomorphic — any WAT converts to binary, and binary disassembles to WAT.

## Memory Model

### Linear Memory

Wasm defines a **single linear address space**: contiguous bytes indexed 0 to 2^32-1 (32-bit) or 2^64-1 (64-bit). Memory is allocated in **pages** (64 KB each). Initial and maximum page counts are declared at module instantiation:

```lisp
(memory 10)         ;; min 10 pages (640 KB)
(memory 10 100)     ;; min 10, max 100 pages
```

**Load/Store Instructions:**
```lisp
i32.load  addr      ;; read 4 bytes at addr
i32.store addr val  ;; write 4 bytes
i32.load8_u addr    ;; read 1 byte, zero-extend
```

Operations include alignment hints and offset parameters for structured memory access.

### Shared Memory (Threads)

Multiple workers share memory if declared with `shared`:

```lisp
(memory 10 shared)
```

Operations become atomic:
- `i32.atomic.load`, `i32.atomic.store`
- `i32.atomic.rmw.add` (read-modify-write)
- `memory.atomic.wait`, `memory.atomic.notify` (futex-like primitives)

**Key discipline**: no lock-free synchronization guarantees — memory ordering follows JavaScript semantics (sequential consistency for data-race-free programs).

## Module System: Imports & Exports

Modules are not monolithic. Dependencies are declared explicitly:

```lisp
(module
  ;; Import a function
  (import "env" "log" (func $log (param i32)))
  
  ;; Export a function
  (export "process" (func $process))
  
  (func $process (param i32) (result i32)
    local.get 0
    call $log
    i32.const 42))
```

**Imports** bind to host-provided implementations. **Exports** define the module's public API. This enables:

- **Shared libraries**: modules import utilities, re-export them
- **Dependency injection**: host controls behavior
- **Composition**: multiple modules interact via imports/exports

Both functions and memories can be imported/exported, allowing multiple modules to share the same memory instance.

## WASI: System Interface

WASM cannot directly access OS resources (files, sockets, environment). **WASI** (WebAssembly System Interface) standardizes how modules request access:

```lisp
(import "wasi_snapshot_preview1" "fd_write" 
  (func $fd_write (param i32 i32 i32 i32) (result i32)))
```

WASI defines capabilities-oriented access:

- **File descriptors**: opened files, sockets, pipes
- **fd_write**: write to file descriptor
- **fd_read**: read from file descriptor
- **proc_exit**: exit with status code
- **environ_get**: read environment variables

### Preview 2

WASI Preview 1 had limitations (file descriptor model, no streams). **Preview 2** improves with:

- **Component Model integration**: structured I/O types
- **Instance resources**: files, sockets as first-class objects
- **Better error handling**: typed results instead of errno patterns

## Component Model

The **Component Model** extends Wasm beyond single linear modules. A component can:

- Define **interfaces** (type-safe contracts)
- **Compose** multiple instances with dependencies
- Isolate **capabilities** per component

```lisp
(component
  (import "logger" (instance $logger
    (export "log" (func (param string)))))
  
  (core module $main
    ;; reference $logger.log))
  
  (instance $main (instantiate $main)))
```

Components enable:
- **Dependency injection**: host wires dependencies
- **Modular security**: each component has explicit capabilities
- **Version compatibility**: interfaces abstract implementations

This is the long-term vision for Wasm ecosystem — beyond individual modules to full applications.

## Languages Targeting Wasm

**Systems languages** (primary targets):
- **Rust**: excellent Wasm support via `wasm32-*` targets; most Wasm ecosystem tooling
- **C/C++**: Emscripten toolchain compiles to Wasm
- **Go**: growing support, experimental features

**High-level languages**:
- **Python**: Pyodide compiles CPython to Wasm (browser execution)
- **Java**: experimental runtimes (WasmGC proposal enables managed languages)
- **AssemblyScript**: TypeScript-like syntax, explicit memory control

**Niche**:
- Kotlin, Scala (via JVM to Wasm)
- Various specialized languages (TinyGo focuses on embedded Wasm)

Most production Wasm is Rust → `.wasm` or C++ → Emscripten → Wasm.

## Runtimes

### Wasmtime

Wasmtime (Bytecode Alliance, sponsored by Mozilla/Fastly) is a standalone JIT runtime:

- **Just-in-time compilation**: Cranelift backend compiles Wasm to native code
- **CLI**: `wasmtime run module.wasm [args]`
- **Embedding**: C, Rust, Python APIs
- **WASI support**: full Preview 1, partial Preview 2
- **Threads**: shared memory and atomic operations

```bash
wasmtime module.wasm arg1 arg2
```

### Wasmer

Wasmer focuses on ease-of-use and multi-target deployment:

- **Multiple backends**: Cranelift, LLVM, Singlepass
- **Cross-platform**: Windows, macOS, Linux, BSD, WebAssembly
- **Containerization**: can package Wasm as OCI containers
- **Language bindings**: Rust, Python, Node.js

### WasmEdge

WasmEdge (LF Edge, by Second State) targets edge and IoT:

- **Lightweight**: minimal overhead, low memory footprint
- **GPU support**: experimental NVIDIA GPU integration
- **Customizable**: plugin system for host functions

### Browser Runtimes

Modern browsers (Chrome, Firefox, Safari, Edge) have built-in Wasm engines:

- **V8**: Chrome, Node.js
- **SpiderMonkey**: Firefox
- **JavaScriptCore**: Safari
- All JIT-compile to native code

Browser execution is sandboxed — modules cannot access files or network directly (WASI unavailable).

## Performance Characteristics

**Overhead**:

- **Startup**: JIT compilation takes time; AOT (ahead-of-time) compilation reduces it
- **Memory**: module footprint + runtime state; smaller than VMs, larger than native binaries
- **Overhead near zero**: optimized Wasm often matches C/C++ native performance

**Advantages**:

- **Predictability**: no garbage collection pauses (GC is optional with Reference Types)
- **Portability**: compile once, run anywhere (x86, ARM, RISC-V, etc.)
- **Security**: sandbox prevents arbitrary memory access

## Common Use Cases

- **High-performance browser apps**: games, image editors, CAD
- **Serverless functions**: Fastly compute@edge, Shopify functions
- **Embedded systems**: IoT, firmware (via WasmEdge, Wasmtime)
- **Plugin systems**: applications embedding Wasm for extensibility
- **Multi-language backends**: Python via Pyodide, Java via WasmGC

## see also

- [web-webassembly.md](web-webassembly.md) — high-level overview and execution model
- [compilers-type-inference.md](compilers-type-inference.md) — type validation in Wasm
- [runtime-jvm.md](runtime-jvm.md) — comparison: JVM memory model