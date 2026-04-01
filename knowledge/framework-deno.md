# Deno — Secure JavaScript Runtime, Permissions, and Modern Tooling

## Core Architecture: V8, Rust, and Tokio

Deno is a JavaScript/TypeScript runtime replacing Node.js. It uses **V8** (Google's JavaScript engine) for execution, a **Rust core** for system bindings, and **Tokio** for async I/O. The design philosophy prioritizes **secure defaults, built-in tooling, and web standards**. Unlike Node.js, which wraps V8 and exposes full OS access by default, Deno requires explicit permission grants for file system, network, and environment variable access.

Key differences from Node.js:
- **No package.json or node_modules folder**: Dependencies are fetched from URLs and cached globally in `~/.deno`
- **Permissions are explicit**: `deno run --allow-read /tmp myapp.ts` requires manually granting read access to `/tmp`
- **TypeScript out-of-the-box**: No TypeScript compilation step; Deno transpiles on-the-fly
- **URLs are first-class citizens**: `import { serve } from "https://deno.land/std/http/server.ts"`

## Permissions Model: Explicit Access Control

Deno enforces **coarse-grained permissions** as CLI flags. Without them, scripts fail at runtime when attempting restricted operations:

- `--allow-read=[paths]`: File system read access; paths can be specific directories or omitted for unrestricted
- `--allow-write=[paths]`: File system write access
- `--allow-net=[origins]`: Network access; restrict to specific domains like `--allow-net=example.com` or allow all with bare `--allow-net`
- `--allow-env`: Access to environment variables
- `--allow-run=[commands]`: Execute subprocesses
- `--allow-ffi`: Load dynamic libraries (WebAssembly or shared objects)

A script cannot escalate permissions at runtime; they're fixed at startup. This prevents downloaded code from exfiltrating data or commandeering the system without user awareness. Compare to Node.js, where any dependency can access your file system and network.

## Module System & URL-Based Imports

Deno uses **ES modules exclusively** (no CommonJS). Dependencies are imported directly by URL:

```typescript
import { serve } from "https://deno.land/std/http/server.ts";
import { parse } from "https://deno.land/std/flags/mod.ts";
```

The first download of a URL is fetched and cached in `~/.deno/deps`; subsequent imports use the cached version. To refresh, use `deno cache --reload` or set `DENO_DIR` to control cache location.

**Semantic versioning** is managed by URL: `https://deno.land/std@0.170.0/http/server.ts` pins to version 0.170.0. Code doesn't automatically update when dependencies publish new versions; you upgrade by changing the URL.

Import maps allow aliasing URLs for cleaner local dev:

```json
{
  "imports": {
    "std/": "https://deno.land/std/",
    "oak": "https://deno.land/x/oak/mod.ts"
  }
}
```

Then `import { Router } from "oak"` works locally while resolving to the pinned URL.

## Built-in Tooling: No External Dependency

Unlike Node.js, which outsources formatting/linting to external packages (prettier, eslint), Deno includes **first-party tools**:

- **deno fmt**: Opinionated formatter (no configuration, intentionally strict)
- **deno lint**: Linter detecting common errors (deno_lint, built-in, ~200 rules)
- **deno test**: Test runner with test assertion libraries built-in (`assertEquals`, `assertThrows`, etc.)
- **deno bench**: Benchmarking tool for performance measurement
- **deno doc**: Generate documentation from JSDoc comments

These tools have no install step and no configuration files. `deno fmt` reformats all TypeScript; `deno test` discovers and runs `*_test.ts` files. This reduces setup friction and ensures consistency across projects.

## Fresh Framework: Islands Architecture

**Fresh** is Deno's first-party web framework built on Preact. It implements **islands architecture**: server-renders HTML by default, shipping zero JavaScript to clients. Interactive UI elements (buttons, forms, counters) are **islands** — distinct Preact components hydrated with JavaScript only where interactivity is needed.

Fresh routes are TypeScript/JSX files exporting handlers:

```typescript
// routes/counter.tsx
import { useSignal } from "@preact/signals";

export default function Counter() {
  const count = useSignal(0);
  return <button onClick={() => count.value++}>{count.value}</button>;
}
```

Routes run on the server (template rendering) by default. Islands (in the `islands/` folder) hydrate interactively on the client. This architecture dramatically reduces JavaScript payloads and improves perceived performance. Fresh 2.0 migrated from hot module reloading to Vite, aligning with broader web ecosystem standards.

## Deno Deploy: Edge Runtime

**Deno Deploy** is a serverless platform running Deno code at edge locations worldwide. It provides a `fetch()` handler interface:

```typescript
// serve this globally
export default {
  fetch(req: Request) {
    return new Response("Hello from Deno Deploy!");
  }
};
```

Deploy environment is restrictive: no file system, no subprocess access, limited environment variables. It's designed for stateless handlers responding to HTTP requests. Fresh apps can be deployed to Deploy by setting the backend to a Deploy module that routes requests to Fresh.

## Node Compatibility & JSR Registry

Deno v1.28+ includes `npm:` specifier for importing npm packages directly:

```typescript
import express from "npm:express@4.18.2";
```

This allows gradual migration from Node.js projects without rewriting all dependencies. However, npm packages expecting CommonJS, `require()`, or Node.js-specific APIs may need adaptation. Deno Node compatibility layer provides commonalities like `node:fs`, `node:path`, `node:process`.

**JSR** (Jsr.io) is Deno's native package registry for TypeScript-first packages. JSR packages automatically publish `.d.ts` files; they're consumable from Deno, Node.js, and browsers. JSR replaces npm's role for Deno-native code but is still early-adoption.

## Standard Library & deno.land/std

The **standard library** (`https://deno.land/std/`) provides primitives for CLI args, HTTP servers, testing, collections, encoding (base64, CSV, JSON), datetime utilities, and more. Unlike Node.js, which has a minimalist stdlib expecting npm for most needs, Deno's stdlib is comprehensive and audited. Libraries in deno.land/x are third-party (not vetted by Deno core team).

## Startup Performance & Comparison to Node.js

Deno startup time is typically **2–3x faster** than Node.js (50–100ms vs. 150–300ms for non-trivial apps). The Rust core and cached dependencies contribute. Parsing and transpiling TypeScript happen at runtime, so cold-start includes transpilation overhead; subsequent runs see no improvement (no bytecode caching). In practice, Deno feels snappier than Node.js even accounting for transpilation.

## Reasoning for Security Model

Deno's restrictive permissions are an intentional rejection of Node.js's implicit trust. A script can only do what its process can do; this is the Unix principle ("capabilities" or "principle of least privilege"). When you run `deno run --allow-read=/data myapp.ts`, you're saying "this script can read /data and nothing else." Deno enforces this at the runtime level, not via operating system ACLs.

## See Also
- [runtime-v8](runtime-v8.md) — V8 JavaScript engine and just-in-time compilation
- [bundling-module-systems](bundling-module-systems.md) — ES modules and resolution strategies
- [framework-bun](framework-bun.md) — alternative modern JavaScript runtime with different choices
- [architecture-serverless](architecture-serverless.md) — edge computing and function-as-a-service patterns