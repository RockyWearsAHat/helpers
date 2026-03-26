# Web Bundlers — Webpack, Vite, esbuild, Rollup, Turbopack & Ecosystem

## Overview

Web bundlers combine source files (JavaScript, CSS, images, etc.) into optimized artifacts for distribution. They solve dependency resolution, code splitting, asset optimization, and source map generation. The landscape spans webpack (extensible, powerful, complex), Vite (modern, ESM-first, dev-focused), esbuild (speed via Go, minimal config), Rollup (tree-shaking specialist), Turbopack (Rust-based, incremental), and others. Each tool reflects different trade-offs: developer experience vs. build speed vs. configuration flexibility.

## Core Bundling Concept

Bundling performs three tasks:

1. **Dependency resolution:** Traverse `import` / `require` statements, build a dependency graph
2. **Transformation:** Apply loaders/plugins (transpile TypeScript, process CSS, optimize images)
3. **Output generation:** Combine into chunks optimized for: bundle size, load time, code splitting, lazy loading

```
Source files → Parse graph → Transform → Chunk & optimize → Output bundles
  main.js       resolve deps      transpile    code split      main.js
  lib.js        collect assets     minify       lazy split      lib.abc.js
  style.css                        css proces   vendor split    vendor.def.js
                                                                 style.css
```

## Webpack

**Philosophy:** Extensible, everything-is-a-module. If a tool can't solve it, write a loader or plugin.

**Architecture:**

- **Entry point:** Where bundling starts (e.g., `src/index.js`)
- **Loaders:** Transform files before bundling. Applied per file type.
  ```javascript
  module: {
    rules: [
      { test: /\.ts$/, use: 'ts-loader' },
      { test: /\.css$/, use: ['style-loader', 'css-loader'] },
      { test: /\.png$/, use: 'file-loader' }
    ]
  }
  ```
- **Plugins:** Operate on the entire compilation process. Control output generation, optimization, environment handling.
  ```javascript
  plugins: [
    new HtmlWebpackPlugin({ template: 'index.html' }),
    new MiniCssExtractPlugin()
  ]
  ```

**Strengths:**
- Mature ecosystem: 2,000+ loaders and plugins
- Fine-grained control: customize nearly every step
- Advanced features: dynamic imports, multi-entry, federated modules
- Handles all asset types natively (images, fonts, HTML, etc.)

**Weaknesses:**
- Configuration complexity: 50+ options, hard to reason about
- Build speed: 10–30s for medium projects (disk I/O + plugin overhead)
- Dev server slower than ESM-based tools
- Steep learning curve for newcomers

**Use case:** Large applications with complex build requirements; teams with expertise.

## Vite

**Philosophy:** Leverage native ES modules in development; Rollup for production.

**Architecture:**

- **Dev server:** Serves source files as ES modules directly to the browser. No bundling during development.
  ```bash
  # Main.js imports lib.js → Browser requests lib.js → Vite serves raw file
  # Instant: no bundling step, HMR (hot module replacement) is file-based
  ```
- **Production build:** Uses Rollup under the hood. Config:
  ```javascript
  export default {
    build: {
      rollupOptions: {
        output: {
          manualChunks: (id) => {
            if (id.includes('node_modules')) return 'vendor';
          }
        }
      }
    }
  }
  ```

**Strengths:**
- Dev experience: instant server start, fast HMR (100ms updates)
- Minimal config: works with zero config for simple projects
- ESM production output: smaller bundle size via tree-shaking
- Excellent TypeScript support out-of-the-box

**Weaknesses:**
- Requires modern browser support (ESM in dev; IE not supported)
- Plugin ecosystem smaller than webpack (though growing)
- Tree-shaking only works with ESM libraries (CommonJS drags dead code)

**Use case:** Modern web apps (React, Vue, Svelte), new projects, rapid iteration.

## Esbuild

**Philosophy:** Speed via Go implementation. Minimal features; opt-in complexity.

**Architecture:**

- Written in Go; compiles to native binary → 10–100x faster than Node.js bundlers
- Simple API: single entry point, no plugins (until recently)
- Transforms: TypeScript, JSX, tree-shaking, minification built-in

```javascript
// esbuild config
import * as esbuild from 'esbuild';

await esbuild.build({
  entryPoints: ['src/index.ts'],
  bundle: true,
  outfile: 'dist/index.js',
  minify: true,
  target: 'esnext',
  external: ['react']  // Don't bundle external packages
});
```

**Strengths:**
- Speed: 10–50ms builds (vs seconds for webpack)
- Low overhead: minimal startup, no plugin system overhead
- TypeScript and JSX native support
- Good enough for most use cases

**Weaknesses:**
- Plugin API immature compared to webpack
- Can't handle every build customization
- Smaller ecosystem
- Community support less mature (though improving)

**Use case:** Libraries, CLI tools, monorepo monoliths where build speed matters.

## Rollup

**Philosophy:** ES module specialist. Optimal tree-shaking and minimal output.

**Architecture:**

- Primary use: libraries and NPM packages (not applications)
- Generates minimal wrapper code; ES module output is human-readable
- Configuration:
  ```javascript
  export default {
    input: 'src/index.js',
    output: { file: 'dist/mylib.js', format: 'esm' },
    external: ['lodash'],  // Don't bundle dependencies
    plugins: [
      resolve(),           // Find node_modules packages
      commonjs(),          // Convert CommonJS to ESM
      minify()
    ]
  };
  ```

**Output formats:**
- `esm`: ES Module (modern, tree-shakeable)
- `cjs`: CommonJS (Node.js, backwards compatible)
- `umd`: Universal (browser + Node.js)
- `iife`: Immediately-invoked function expression (browser global)

**Strengths:**
- Tree-shaking: _only_ includes code that's imported; dead code is eliminated
- Library output: generates clean, minimal bundles for NPM
- Format flexibility: single config generates multiple output formats
- Transparent: easy to reason about what's in the bundle

**Weaknesses:**
- Not suitable for large applications (lacks built-in code splitting features)
- Configuration required for libraries (specify formats, externals, etc.)
- Smaller plugin ecosystem than webpack

**Use case:** NPM libraries, UI component packages, utils.

## Turbopack

**Philosophy:** Rust-based incremental bundling for monorepos and large applications.

**Architecture:**

- Still in development (public beta); aims to be webpack replacement
- Incremental bundling: rebuild only changed modules
- Integrated into Next.js 13+
- Configuration: webpack-compatible (gradual migration path)

```javascript
// turbo.json in monorepo root
{
  "tasks": {
    "build": {
      "outputs": ["dist/**"],
      "cache": true
    }
  }
}
```

**Strengths:**
- Speed: rebuilds are incremental, drastically faster for large projects
- Monorepo-aware: understands workspace dependencies
- Backward compatible with webpack config (easier migration)
- Rust backend: memory efficient

**Weaknesses:**
- Early stage: APIs and features still stabilizing
- Documentation sparse compared to mature tools
- Smaller ecosystem
- Risk of abandonment (though backed by Vercel)

**Use case:** Monorepos, large applications, next-generation builds (emerging).

## Other Tools

**SWC:** Rust-based JS/TS compiler. Often used as a loader in webpack or as a standalone transpiler (not a full bundler).

**Parcel:** Zero-config bundler. Asset types auto-detected. Good for quick projects; less flexibility than webpack.

**Web Assembly bundlers:** Native support in modern bundlers (webpack, Vite, esbuild), but specialized tools (wasm-pack for Rust) handle Wasm-specific optimization.

## Comparative Matrix

| Aspect | Webpack | Vite | Esbuild | Rollup | Turbopack |
|--------|---------|------|---------|--------|-----------|
| **Config required** | High | Low | Low-Medium | Medium | Medium |
| **Dev speed** | Slow (5–10s) | Very fast (instant) | Very fast | Medium | Very fast |
| **Build speed** | Slow (10–30s) | Fast (2–5s) | Ultra-fast (50ms–1s) | Fast (1–3s) | Fast (incremental) |
| **Tree-shaking** | Good | Excellent | Good | Excellent | Excellent |
| **Code splitting** | Excellent | Excellent | Manual | Limited | Excellent |
| **Ecosystem** | Huge (2000+ plugins) | Growing | Small | Medium | Growing |
| **TypeScript** | Via ts-loader | Native | Native | Via plugin | Native |
| **Use case** | Large SPA apps | Modern web dev | Libraries, CLI | NPM packages | Monorepos (emerging) |
| **Learning curve** | Steep | Gentle | Gentle | Moderate | Moderate |
| **Production ready** | Yes (since 2016) | Yes (mature 4.x) | Yes (1.0+ stable) | Yes (mature) | Beta |

## Common Bundler Features

### Code Splitting

Breaking a bundle into chunks loaded on-demand.

```javascript
// Vite/webpack example
// main.js imports heavy-lib only when needed
const HeavyLib = await import('./heavy-lib.js');

// Output:
// main.js (50KB) - loads first
// heavy-lib-abc.js (200KB) - loads on user action
```

**Benefits:** Smaller initial load, faster Time-to-Interactive.

**Trade-off:** Extra HTTP requests (mitigated by HTTP/2 multiplexing).

### Tree-shaking

Removing unused code. Requires ES modules and no side effects:

```javascript
// lib.js
export function usedFunc() { }
export function unusedFunc() { }

// app.js
import { usedFunc } from './lib.js';
usedFunc();

// Output: only usedFunc included, unusedFunc dropped
```

**Requirement:** Both library and app must use ESM (not CommonJS), and library must be pure (no side effects in module scope).

### Source Maps

Maps compiled/minified code back to original source for debugging:

```javascript
// webpack config
devtool: 'source-map'  // Full map (dev only)
// or
devtool: 'eval-cheap-module-source-map'  // Trade-off: smaller, eval-based
```

**Dev:** Full `source-map` (maps every byte).
**Prod:** Often disabled (security risk, slower upload) or uploaded to error tracking service.

### Dynamic Imports (Async Chunks)

```javascript
// Webpack/Vite
const module = await import('./module.js');  // Separate chunk, loaded async
```

### Environment Variables

Bundlers inject `process.env.NODE_ENV` and custom vars at build time:

```javascript
// Vite
// .env.production
VITE_API_URL=https://api.prod.com

// app.js
const api = import.meta.env.VITE_API_URL;  // Token replaced at build
```

## Migration Strategies

**From webpack → Vite:**
- Move loaders to Vite plugins
- Simplify config (most built-in)
- Test HMR (different API)
- Check dependency compatibility (needs ESM-friendly libs)

**From webpack → esbuild:**
- Useful for monorepo builds or libraries
- May lose fine-grained control (plugins, edge cases)
- Requires testing on target environments

**From any → webpack (upgrade):**
- Config usually backward compatible
- Update loaders/plugins incrementally
- Profile build speed; identify bottlenecks

## Trade-offs Summary

| Scenario | Best Choice | Reason |
|----------|-------------|--------|
| Rapid development | Vite | Instant dev server, HMR |
| Library + multi-format output | Rollup | Tree-shaking, format flexibility |
| Speed critical (CLI, libs) | Esbuild | 10–100x faster compilation |
| Complex large SPA | Webpack | Mature, extensible, ecosystem |
| Monorepo, next-gen | Turbopack | Incremental, workspace-aware |
| Enterprise migration | Webpack upgrade | Risk minimization |