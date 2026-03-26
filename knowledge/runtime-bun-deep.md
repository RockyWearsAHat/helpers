# Bun Runtime — Engine, Event Loop, and Toolchain Architecture

## Core Runtime Architecture

Bun is a JavaScript runtime written in **Zig** and powered by **JavaScriptCore** (JSC), Apple's JavaScript engine from Safari. This choice differentiates it from Node.js (V8) and Deno (V8) and has profound architectural implications.

### JavaScriptCore vs. V8

**JavaScriptCore** is a bytecode-based engine with a different optimization philosophy than V8:

- **Bytecode-first**: JSC compiles to bytecode immediately, then optimizes through tiered compilation (LLInt → DFG → FTL)
- **Lower startup overhead**: Avoids V8's need to parse, create AST, build metadata structures — faster cold execution
- **Memory efficiency**: JSC typically uses less memory than V8 for equivalent workloads
- **Optimization profiles**: Uses inline caches and guard-based optimization; less aggressive speculative optimization than TurboFan

Startup time penalty: Node.js ~25ms, Bun ~5ms on typical scripts. This explains Bun's appeal for CLI tools, batch processing, and serverless cold starts.

**Trade-off**: JSC's compiled-bytecode model means peak performance (after warmup) can lag V8, which specializes in long-running optimization. For CLI scripts, this is irrelevant; for servers running hours, V8 may pull ahead in hot paths.

### Zig Implementation

Bun is ~90% Zig, with minimal C/C++. **Zig** is a systems language combining:

- **Manual memory management** without garbage collection (Bun uses an arena allocator for object pools)
- **Low-level performance**: Direct access to CPU features, alignment control, instruction counting
- **Comptime metaprogramming**: Compile-time code execution allows generating specialized code paths for different platforms (x64, ARM64)
- **Smaller binaries**: Zig compilation produces leaner executables than C++, fitting Bun into a single ~100MB binary

This architecture choice—Zig + JSC, not C++ + V8—explains Bun's rapid development velocity and tight integration. The entire codebase can be modified cohesively instead of wrestling with V8's C++ class hierarchies.

## Event Loop & Async I/O

Bun originally used **libuv** (same as Node.js) but has progressively replaced it with **custom Zig-based I/O primitives**:

### libuv Residuum

For compatibility, Bun still exposes Node.js `event` methods (`on`, `once`, `emit`), but the core event loop has been refactored:

- File I/O: Uses OS-specific syscalls directly (Linux: `io_uring`, macOS/BSD: `kqueue`, Windows: IOCP) rather than libuv's abstraction
- Network: Integrates with platform event notification systems
- Timers: Native timer queues managed by Zig code

### Async/Promise Architecture

Promises execute via a microtask queue, consistent with web standards. Bun's implementation:

- **Microtask batching**: Executes all pending microtasks after each event loop tick
- **Native Promise optimization**: Promise chains can be optimized by JSC's compiler without allocation overhead (using hidden class transitions)
- **Async/await desugaring**: Syntactic sugar over state machines; Zig code generates efficient state transitions without intermediate Promises

## Built-in APIs: Bun.* Namespace

Bun adds non-standard APIs optimized for systems programming tasks:

### Bun.serve() — Native HTTP Server

Standard Node.js requires external packages (Express, Hapi) or raw `http.Server`. Bun includes a built-in HTTP server:

```typescript
Bun.serve({
  port: 3000,
  fetch(req) {
    return new Response("Hello");
  },
});
```

Under the hood:

- Uses **platform-native HTTP/1.1 parser** (not Node's http.parser module)
- Direct socket binding to port via OS APIs
- Request/response objects map to native C structs, avoiding allocation overhead
- Built-in HTTP/1.1 keep-alive and pipelined requests

Performance: ~59k req/s on simple workloads (vs. Node.js ~19k req/s), largely due to reduced allocation and tighter syscall integration.

### Bun.file() — Async File System

Lazy file handles and zero-copy where possible:

```typescript
const f = Bun.file("/path/to/file");
const buffer = await f.arrayBuffer();  // Mmap when possible
```

Also `Bun.write()` for atomic file operations, `Bun.glob()` for efficient file pattern matching.

### Bun.FFI — Foreign Function Interface

Direct C library calling without native modules:

```typescript
const lib = Bun.dlopen("libc.so", {
  strlen: { args: ["cstring"], returns: "uint32" },
});
lib.symbols.strlen("hello");  // Direct call, no overhead
```

Implemented via Zig's `@cDefine` and direct symbol loading.

## Package Manager: Binary Lockfile

Bun's package manager differs from npm/Yarn:

### Installation Strategy

- Uses **Cargo-style workspaces** for monorepos
- Installs into `node_modules/` for Node.js compatibility but uses a **binary lockfile** (`bun.lock.b`) instead of `package-lock.json`
- Binary format advantage: Faster parse, smaller file size, cryptographic integrity checks built-in

### Auto-Install Feature

Executing `bun run script.ts` when a dependency is missing automatically invokes `bun install` for that package. This reduces the ceremony of "install dep → import dep" and mirrors Deno's URL-based auto-fetch.

### Workspace Resolution

```toml
# bunfig.toml
[workspace]
members = ["packages/*"]

[dependencies]
local-pkg = "workspace:*"
```

Zig code resolves workspace references at install time, avoiding symlinks and duplicate copies.

## Bundler Architecture

Bun includes a fast bundler (`bun build`) written in Zig:

- **AST-based tree-shaking**: Parses source to AST, tracks exports, removes unused code
- **Single-pass codegen**: Outputs bundle without intermediate phases
- **Plugins**: Zig + JavaScript plugin interface for custom loaders

Performance: Bundles 10k React components in ~269ms (vs. esbuild ~571ms), partly due to parallelization and Zig's memory efficiency.

## Test Runner

`bun test` provides Jest-compatible test execution:

- Parallel test file execution (multiple processes)
- JSC's native debugging integration for `console.log`, stack traces
- Snapshot management with binary format for diffs
- Coverage via instrumentation during parsing

## TypeScript Compilation

TypeScript transpiles on-the-fly via Zig's built-in transformer, not via a separate process. Type annotations are stripped; no type checking (use external `tsc --noEmit` for that), keeping execution fast.

## Single Binary Delivery

The entire runtime, bundler, test runner, package manager, and plugins ship as a single ~100MB executable. No secondary downloads, version mismatches, or `.node` module compilation. This design choice enables seamless distribution and portability.

## Limitations and Trade-offs

- **JSC's optimization plateau**: Warm performance on long-running servers may not match V8's aggressive speculation
- **Ecosystem integration**: C++ native modules (`.node` files) are not supported; Bun.FFI is the alternative
- **TypeScript no-op**: Type stripping without checking can mask errors
- **Platform-specific**: Tight OS integration means less cross-platform uniformity than Node.js
- **Community maturity**: Smaller ecosystem of Bun-native libraries compared to Node.js/npm