# Bun — JavaScriptCore Runtime, All-in-One Toolchain, and Node.js API Compatibility

## Core Architecture: JavaScriptCore, Zig, and All-in-One Design

Bun is a JavaScript runtime designed for **startup speed and developer experience**. It uses **JavaScriptCore** (Apple's JavaScript engine from Safari) instead of V8, implemented in **Zig** (a systems language offering performance and safety). Bun bundles a package manager (`bun install`), bundler (`bun build`), test runner (`bun test`), and TypeScript transpiler—tools that typically require external dependencies in Node.js ecosystems.

Startup on Linux: Bun ~5ms vs. Node.js ~25ms. This speedup flows from JavaScriptCore's optimization strategies and Zig's native code quality. For CLI scripts, batch processing, or serverless functions, startup matters; for long-running servers, it's negligible.

## Node.js Compatibility Layer

Bun implements the **Node.js API surface**, allowing many Node.js packages and scripts to run without modification:

```typescript
import { readFileSync } from "fs";
import { Server } from "http";

const content = readFileSync("./file.txt", "utf-8");
const server = new Server((req, res) => {
  res.write(content);
  res.end();
});
server.listen(3000);
```

This code runs identically in Node.js and Bun. Bun resolves `import "fs"` to its Node.js compat implementation, not the built-in `node:fs` module.

**Caveats**: Not all Node.js packages work out-of-the-box. Packages requiring specific async behavior, worker threads, or native modules (C++ addons) may fail. Bun's approach is "aim for 80% compatibility," not perfect emulation.

## Built-in Package Manager & Auto-Install

`bun install` replaces `npm install`, parsing `package.json` and installing to `node_modules/`. It's faster than npm (parallel downloads, fewer locks) and features **auto-install**: if a script imports a package not in `node_modules`, Bun automatically installs it. This removes the manual "install lib, import lib" ceremony.

```bash
# Run script; bun auto-installs dependencies on first import
bun run server.ts
```

The `bunfig.toml` file (Bun's package config) controls resolution, transpilation, and runtime options. When `npm` or `yarn` is present, Bun respects the lock file to maintain deterministic installs across teams.

## Bundler: Built-in Code Splitting & Tree Shaking

`bun build` bundles JavaScript for distribution. It natively handles:

- **Code splitting**: Separate chunk files for shared dependencies
- **Tree shaking**: Remove unused code
- **Minification**: Compact output
- **CSS/JSX inlining**: Process styles and JSX in one step
- **Multiple entry points and formats** (ESM, CommonJS, UMD)

Unlike webpack or Rollup, Bun's bundler has no separate config files; bundle behavior is driven by CLI flags:

```bash
bun build ./src/index.ts --outdir ./dist --minify --sourcemap --target web
```

This single command replaces a typical webpack config with 50+ lines.

## Test Runner: Minimal Setup

`bun test` discovers and runs `*.test.ts` / `*.test.js` files:

```typescript
import { describe, it, expect } from "bun:test";

describe("math", () => {
  it("adds numbers", () => {
    expect(1 + 1).toBe(2);
  });
});
```

Tests run in parallel with snapshots, mocking, and setup/teardown hooks. No jest config or babel setup. The tradeoff: less configuration flexibility than jest, but significantly faster test startup for quick iterations.

## Foreign Function Interface (FFI): Calling C

Bun's **FFI** allows invoking C library functions directly from JavaScript:

```typescript
const dylib = new Bun.FFI.Library("libssl");
const SHA256 = dylib.symbols.SHA256_Init;

// Call C function directly
const buffer = SHA256(input);
```

This enables embedding performance-critical C code without node-gyp or native module compilation. Use cases: cryptographic operations, image processing, or interfacing with system libraries.

## SQLite Driver & API

Bun includes a built-in **SQLite driver**, hydrated into JavaScript objects:

```typescript
import { Database } from "bun:sqlite";

const db = new Database("./data.db");
db.exec("CREATE TABLE posts (id INTEGER PRIMARY KEY, title TEXT)");

const stmt = db.prepare("SELECT * FROM posts WHERE id = ?");
const post = stmt.get(1);
```

This avoids the `better-sqlite3` npm dependency and native module compilation. SQLite is embedded directly in the runtime. Performance suits typical web apps; for massive-scale analytics, dedicated databases are appropriate.

## Macros: Compile-Time Code Generation

Bun **macros** execute TypeScript functions at build time and replace their invocation with the result:

```typescript
// math.ts
export function times(a: number, b: number) {
  return a * b;
}

// app.ts
import { times } from "./math.ts" with { type: "macro" };
const result = times(3, 4); // Evaluated at build time → result = 12
```

This enables compile-time optimizations, type-safe configuration templating, and DSL embedding without imposing runtime overhead.

## JavaScript Transpilation & Type Stripping

Bun transpiles JSX and TypeScript on-the-fly:

```bash
bun run index.tsx  # Automatically transpiles JSX and .ts syntax
```

No separate `tsc` invocation or Babel config. Bun's transpiler is optimized for speed and understands Node.js/Bun idioms (e.g., top-level `await`). TypeScript type errors are warnings, not build failures; this allows running partially-typed code during development.

## Concurrency: Workers & Single-Threaded Default

By default, Bun code runs on a single thread. The **Web Workers API** (`new Worker()`) spins up additional threads for CPU-bound work. Workers communicate via `postMessage()` and message handlers, mirroring the browser API. This is more ergonomic than Node.js `worker_threads` for teams wanting portability across server/client.

## Watch Mode & Hot Reloading

`bun --watch run script.ts` restarts the process on file changes. This is useful for development but not a substitute for hot module reloading (HMR) in frameworks like Fresh (Deno) or Next.js. Bun doesn't preserve state across restarts; it's a soft restart.

## Performance Characteristics & Benchmarks

Bun's `bun:test` runner is faster than jest at scale (hundreds of tests) due to parallelization and no V8 profiling overhead. Bundling with `bun build` is faster than webpack or esbuild. Package installation with `bun install` is faster than npm/yarn.

However, **benchmark context matters**: these comparisons favor Bun's design (parallelization, minimal config, fast startup). For large monorepos or complex build pipelines, Bun may hit its limitations; webpack's ecosystem is broader, jest's configurability is deeper.

## Ecosystem Maturity & Community

Bun is younger than Node.js and Deno. The ecosystem of Bun-native packages is small; most rely on npm compatibility. CI/CD tooling (GitHub Actions, etc.) increasingly supports Bun, but edge platforms may not have prebuilt binaries. Adoption is growing in startups and performance-focused teams, but production deployments in enterprises are less common.

## Tradeoffs: Simplicity vs. Legacy Compatibility

Bun optimizes for throughput and developer ergonomics, accepting that some Node.js packages won't work perfectly. The philosophy: "build the common case fast and ergonomically; power users can reach for Node.js." This contrasts with Deno's philosophy ("start from scratch, secure by default") and Node.js's ("99% backward compatible, even if it slows us down").

## See Also
- [framework-deno](framework-deno.md) — alternative modern runtime with different design priorities
- [runtime-v8](runtime-v8.md) — V8 engine (Node.js standard) vs. JavaScriptCore architecture
- [web-javascript-engine](web-javascript-engine.md) — JavaScript engine internals and JIT compilation
- [bundling-module-systems](bundling-module-systems.md) — bundling strategies and tree-shaking