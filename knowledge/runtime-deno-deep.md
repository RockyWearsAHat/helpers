# Deno Runtime — V8, Rust Core, and Permission Enforcement

## Architecture Overview

Deno is a JavaScript/TypeScript runtime built on three pillars:

1. **V8** (Google's JavaScript engine) — executes JavaScript
2. **Rust core** (`deno_core` crate) — manages event loop, async I/O, and system bindings
3. **Tokio** (async runtime) — provides async/await primitives at the OS level

This tri-layer architecture contrasts with Node.js, which wraps V8 in C++ and tightly couples the event loop. Deno's Rust/Tokio foundation enables a cleaner separation of concerns and more granular permission control.

## V8 Integration

Deno embeds V8 similarly to Node.js:

- **V8 context**: Each Deno process runs within a single V8 context (no workers by default, though Web Workers exist)
- **Script execution**: JavaScript is parsed, compiled, and executed by V8 using its standard pipeline (parser → bytecode → TurboFan JIT)
- **Heap snapshots**: V8 manages JavaScript object memory; GC coordination happens via Rust callbacks

Key difference from Node.js: Deno's Rust binding layer is thinner and more explicit. System calls are dispatched via a structured **ops system** rather than V8 extensions directly calling C++ functions.

## Ops System — Structured System IPC

Deno's defining architectural innovation is the **ops system**, a structured interface between JavaScript and Rust:

### Ops as RPC

An "op" is a single system operation (file read, network write, etc.) dispatched from JavaScript to Rust:

```javascript
// JavaScript
const result = await Deno.core.ops.op_read_file("/etc/passwd");

// Rust (internal)
fn op_read_file(path: String) -> Result<Vec<u8>, Error> {
  std::fs::read(path)
}
```

Every system interaction routes through ops. This enables:

- **Permission checking**: Before executing any op, Deno checks the permission table (has `--allow-read`? yes → allow)
- **Sandboxing**: JavaScript cannot directly syscall; all access goes through permissioned ops
- **Versioning**: Ops can change signatures without breaking backward compatibility (V8 extension interface is fragile)

### Op Registry

Deno maintains a **registry** (in Rust) of available ops, keyed by op ID. At startup:

- Each op is assigned a numeric ID
- JavaScript receives a reference to the registry
- Calls to `Deno.core.ops.op_XYZ(args)` look up op ID and dispatch

This indirection (ops vs. direct function pointers) comes with minimal overhead because V8 can inline the lookup.

### Fast Path vs. Slow Path

High-frequency ops (timer queue, event loop wakeups) are optimized:

- **Fast path**: Ops that don't require permission checks or complex Rust logic can be optimized to avoid serialization (using V8's fast API calls)
- **Slow path**: I/O-bound ops (file, network) serialize arguments, check permissions, await Rust futures, deserialize results

## Resource Table — Capability-Based Access Control

Every open file, socket, or system resource is assigned a **resource ID** (u32) and stored in a **resource table**:

```rust
pub struct ResourceTable {
  resources: HashMap<u32, Rc<dyn Resource>>,
  next_id: u32,
}
```

When JavaScript opens a file:

```javascript
const fd = Deno.open("/file.txt");  // Returns resource ID, e.g., 42
await Deno.read(fd, buffer);        // Pass resource ID to ops
```

### Capability Model

Resource IDs act as **opaque capabilities**. JavaScript cannot forge a resource ID (it's just a u32, but the resource table validates it exists). This design ensures:

- **No ambient authority**: You cannot access a resource you haven't opened
- **Fine-grained revocation**: Closing a resource invalidates its ID
- **Ownership tracking**: Rust's ownership system ensures resources are dropped when no longer held

Contrast with Node.js, where file descriptors (or handles) can sometimes be accessed indirectly via internal APIs, bypassing the module system.

## Permissions Model — Compile-Time and Runtime Enforcement

### Compile-Time Enforcement

Permissions are checked when ops are invoked, not at module load time. This means:

- Code importing a module doesn't require permission
- Actually *using* a module (calling op_read_file) triggers the check

```javascript
import * as fs from "fs";  // OK
const content = fs.readFileSync("/secret");  // PermissionDenied error if no --allow-read
```

### Runtime Permission Levels

- **Coarse-grained**: `--allow-read` opens all files (or with paths: `--allow-read=/tmp`)
- **No escalation**: Permissions cannot be granted at runtime; they're fixed at startup
- **Query permissions**: `Deno.permissions.query({ name: "read", path: "/tmp" })` checks if a permission is granted

### Global Permission State

Permissions live in a thread-local (at the Rust level) or global state, managed by the `PermissionInspector`. Every op checks this state before proceeding.

## Snapshot Mechanism — Startup Optimization

Deno uses a **V8 snapshot** to accelerate startup:

### Snapshot Creation

- During build, Deno compiles all built-in modules (TypeScript std library, Deno namespace APIs)
- V8's `CreateSnapshot` function serializes the heap state
- Result: `snapshot.bin` (~20MB), containing pre-compiled bytecode and object layouts

### Snapshot Loading

- At runtime, instead of parsing `deno/std/*.ts`, Deno loads `snapshot.bin`
- V8 deserializes the snapshot into the V8 isolate, restoring heap state
- Result: ~1 second faster startup compared to parsing from source

Trade-off: Snapshot must be regenerated if built-in modules change; requires a release rebuild.

## FFI — Foreign Function Interface

Deno can call native C libraries without native modules:

```typescript
const libcrypto = Deno.dlopen("libcrypto.so", {
  SHA256: { parameters: ["buffer", "usize", "buffer"], result: "buffer" },
});
```

### Implementation

- Deno uses `libffi` or similar to generate trampolines
- Type definitions describe memory layout (pointer, struct, array sizes)
- Calls cross the Rust/C boundary directly, without JavaScript overhead

### Sandboxing Implications

FFI requires explicit `--allow-ffi` permission. Loaded libraries execute with the process's privilege level (no further sandboxing). This is more powerful than ops (which can enforce custom checks) but requires trust.

## WebAssembly Integration

Deno's WebAssembly support uses V8's built-in Wasm engine:

- **Instantiation**: `new WebAssembly.Instance(wasmModule)` uses V8's standard Wasm compilation (Liftoff baseline + TurboFan optimization)
- **WASI support**: Deno can expose WASI ops, allowing Wasm modules to invoke file/network operations
- **Memory isolation**: Wasm linear memory is sandboxed within the module's memory space

## Module Loading

### URL-Based Imports

Deno resolves imports as URLs:

```typescript
import { serve } from "https://deno.land/std@0.200.0/http/server.ts";
```

### Resolution Algorithm

1. **Local cache**: Check `~/.deno/dep_cache` for cached downloads
2. **Fetch**: If not cached, HTTP GET from the URL
3. **Code storage**: Save to cache with URL hash as key
4. **Compile**: TypeScript transpiled to JavaScript by Rust, then executed

### Lockfile (`deno.lock`)

Optional JSON file pinning dependency versions and content hashes:

```json
{
  "https://deno.land/std/http/server.ts": {
    "checksum": "sha256-...",
    "integrity": "sha512-..."
  }
}
```

Ensures reproducible builds; can be committed to version control.

## Event Loop and Async Execution

Deno's event loop is tightly integrated with Tokio:

### Tokio Executor

- All async I/O (file, network, timers) uses Tokio's runtime
- JavaScript `async/await` compiles (via V8) to Rust `futures`
- When JavaScript awaits a Promise, a Rust future is spawned; when it completes, a microtask is queued

### Microtask Queue

- After each Tokio event loop iteration, Deno processes V8 microtasks (Promise callbacks, `.then()`, etc.)
- Ensures JavaScript's `async/await` ordering is preserved

### Stack Trace Preservation

Rust futures preserve stack info using `backtrace` crate, enabling V8 to report meaningful stack traces across async boundaries.

## Performance Characteristics

- **Startup**: ~500ms for first-time module load (due to TypeScript transpilation and snapshot deserialization)
- **Warm execution**: Comparable to Node.js once V8 has warmed up
- **Memory**: Slightly higher than Node.js due to snapshot overhead and Rust metadata
- **I/O throughput**: On par with Node.js for typical workloads; scales well with Tokio's work-stealing scheduler

## Type System Integration

TypeScript support is runtime, not compile-time:

- Deno parses `.ts` files using `swc` (a Rust-based TypeScript parser)
- Type annotations are **stripped**: JavaScript cannot access them at runtime
- No type checking at execution time; use `deno check` for static analysis
- This keeps runtime fast but requires explicit type validation.

## Restrictions and Guarantees

- **No C++ native modules**: Only FFI or built-in ops can call native code
- **Heap isolation**: Each Deno process gets its own V8 isolate; no shared state between scripts
- **Deterministic initialization**: Snapshot ensures consistent startup state
- **No reflection of internals**: JavaScript cannot inspect permission state deeply; introspection is limited to `Deno.permissions`