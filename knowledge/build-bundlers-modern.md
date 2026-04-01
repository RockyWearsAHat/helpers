# Modern JavaScript Bundlers — Vite, esbuild, SWC, Rspack, Turbopack

## The Modern Bundler Landscape

The JavaScript bundler ecosystem has stratified into **language-specific tooling** (Rust/Go implementations for speed) and **ESM-first design** that avoids bundling in development. Webpack (2015–2020s) was the monolithic omnibus. Today, tools are specialized: Vite excels at dev experience, esbuild dominates library builds, Rspack targets monorepos, Turbopack aims at production scaling.

### Bundling's Core Problem

Bundlers solve three tasks simultaneously:

1. **Dependency resolution**: traverse imports, build a dependency graph, handle version conflicts
2. **Transformation**: apply loaders/plugins (TypeScript → JavaScript, CSS processing, image optimization, minification)
3. **Chunking**: partition code into bundles that balance initial load time, caching efficiency, and lazy loading

Trade-offs between these tasks divide the ecosystem.

---

## Vite: ESM-First Development

**Philosophy**: Leverage native ES modules in the browser during development; use Rollup for production. Minimize the dev→prod gap via ESM throughout.

### Dev Server Architecture

```
Source files (main.ts, lib.ts, styles.css)
         ↓
    Vite server
    (no bundling)
         ↓
Browser receives bare imports
     ↓
Browser requests lib.ts
     ↓
Vite serves transformed lib.ts (TS→JS on-demand)
```

**Mechanism**:
- Dev server runs natively as ES modules. Browser imports `main.ts` → Vite intercepts, transpiles to JS, serves it.
- Each import is a separate HTTP request. Browser parallelizes downloads; network latency replaces bundling latency.
- **HMR (Hot Module Replacement)**: file changes → browser reloads only changed module. Single file invalidation, not full rebuild.

**Advantages**:
- Server startup: <50ms (no dependency analysis)
- Rebuild on file change: 10–30ms (only transform one file)
- Exact source map mapping: breakpoints align with source
- Works with any framework (React, Vue, Svelte)

**Limitations**:
- Requires modern browser with ES module support (dead in IE11)
- Build output still must be bundled for production
- Older libraries (CommonJS) require pre-bundling overhead during dev startup
- HMR not guaranteed for all language features (mutations in state managers may require full reload)

### Production Build

Uses Rollup under the hood. Configuration:

```javascript
export default {
  build: {
    rollupOptions: {
      output: {
        manualChunks: (id) => {
          if (id.includes('node_modules')) return 'vendor';
          if (id.includes('components')) return 'components';
        }
      }
    }
  }
}
```

---

## Esbuild: Speed Through Go

**Philosophy**: Minimal, single-purpose tool written in Go. No configuration by default; opt-in complexity.

### Speed Advantage

Native binary (Go → machine code, not JavaScript). Benchmarks:
- Typical webpack build (5000 lines): 5–10 seconds
- Esbuild equivalent: 50–100ms (50–100x faster)

Speed comes from:
1. Compiled language (no interpreter overhead)
2. Parallel file I/O
3. Single-pass lexing and parsing
4. Minimal intermediate representations

### Capabilities

- **Transpilation**: TypeScript, JSX, newer syntax features
- **Tree-shaking**: dead-code elimination on ESM modules
- **Minification**: built-in, faster than Terser
- **Code splitting**: multiple entry points produce separate chunks
- **Plugins** (recent): extensibility, though ecosystem smaller than webpack

### Use Cases

- Library bundling (single file, fast iteration)
- Build accelerators (use esbuild inside Webpack via plugins)
- Monorepo task runners (build many packages in parallel)
- Vite's production bundler (selected by Rollup integration, though esbuild can also be used)

### Trade-offs

- **Plugin API**: less mature than webpack; some builds require fallback to Webpack
- **Chunk optimization**: esbuild's heuristics for code splitting are simpler; Webpack offers finer control
- **Ecosystem**: 200 npm packages depend primarily on esbuild (vs 2000+ for webpack)

---

## SWC: Rust-Based Babel Replacement

**Philosophy**: Drop-in replacement for Babel (JavaScript transpiler) using Rust. Speed + compatibility.

### What SWC Replaces

Babel transforms:
- JSX → React.createElement() or custom JSX
- TypeScript → JavaScript (type erasure only, no type checking)
- ES2020+ → ES5 (via presets like @babel/preset-env)

SWC does all this 10–20x faster (Rust vs JavaScript).

### Integration Patterns

- **Next.js**: SWC is default transpiler (since v12); Babel fallback for unsupported plugins
- **Vite**: optional via Vite plugin; Esbuild is default
- **Vitest**: uses SWC for test transformation
- **Standalone**: used where Babel was, e.g., in Webpack via swc-loader

### Limitations

- **Plugin ecosystem**: Babel has 100+ community plugins (emotion, styled-components); SWC's plugin API is stable but smaller ecosystem
- **Exact compatibility**: some Babel transforms have subtle edge cases; SWC implements the spec more strictly
- **Configuration**: `.swcrc` simpler than `.babelrc`, but less flexible for advanced scenarios

---

## Rspack: Rust Webpack

Reimplements Webpack architecture in Rust. Maintains Webpack configuration compatibility while offering speed improvements.

### Design

- **Same config schema** as Webpack
- **Rust implementation** of parsing, bundling, optimization
- **Webpack ecosystem** plugins work unchanged

### Performance

Benchmarks show 5–10x faster than Webpack for large projects.

### Trade-offs

- **Compatibility**: not 100% (some plugins rely on JavaScript callbacks)
- **Maintenance burden**: two implementations (JS and Rust) must stay in sync
- **Maturity**: newer than Webpack; community smaller

---

## Turbopack: Next.js Native Bundler

Developed by Vercel for Next.js; written in Rust (via SWC's Turbo compiler infrastructure).

### Design Differentiator

- **App Router integration**: understands Next.js's new file-based routing
- **Streaming builds**: can stream compiled modules to browser incrementally
- **Incremental bundling** in production (re-bundle only changed routes)

### Positioning

Targets production bundling for Next.js 13+. Still experimental; not universal (tightly coupled to Next.js).

---

## Code Splitting Strategies

Chunking balances (1) initial load size, (2) cache efficiency, (3) lazy loading, (4) parallel requests.

### Route-Based Splitting

```javascript
// webpack
const Home = lazy(() => import('./pages/Home'))
const Admin = lazy(() => import('./pages/Admin'))
```

Each route loads its own chunk on demand. Ideal for large apps. Trade-off: more initial overhead with many routes.

### Vendor Splitting

```javascript
// webpack
{
  optimization: {
    splitChunks: {
      cacheGroups: {
        vendor: { test: /[\\/]node_modules[\\/]/, name: 'vendor' }
      }
    }
  }
}
```

Separates third-party deps from app code. Vendors change less frequently, so cache busting only affects app chunk. Trade-off: vendor chunk may be large; only beneficial if stable across releases.

### Manual Chunks

```javascript
entryPoints: {
  main: './src/main.js',
  worker: './src/worker.js',
  polyfills: './src/polyfills.js'
}
```

Explicit control; useful for web workers, shared initializers, or monorepo packages.

---

## Tree Shaking (Dead Code Elimination)

Removes unused exports from ES modules via static analysis. Depends on:

1. **ESM syntax** (static `import`/`export`, not dynamic `require`)
2. **Bundler support** for marking exports as unused
3. **Minifier** (Terser) to strip the marked code

### Limitations

- **CommonJS** modules cannot be tree-shaken (require() is dynamic)
- **Side effects**: if module code runs on import (e.g., polyfills), tree-shaker must keep entire module
  - Libraries declare `"sideEffects": false` in package.json if safe to prune
- **Dynamic imports** (`import(variable)`) are conservative: bundler includes both branches
- **Incomplete optimization**: bundlers mark dead code; minifiers remove it; misalignment wastes space

### Bundle Analysis Tools

Visualize what made it into the bundle:

- [webpack-bundle-analyzer](https://github.com/webpack-contrib/webpack-bundle-analyzer): treemap chart
- [Bundle Phobia](bundlephobia.com): analyze npm package size
- [Esbuild --metafile](https://esbuild.github.io/api/#metafile): JSON report of inputs/outputs

---

## Module Federation (Micro-Frontends)

**Module Federation** (Webpack 5 feature, now standard): share code between independently built applications at runtime.

### Use Case

Monorepo with 3 applications:
- App A (dashboard): builds independently, deploys independently
- App B (reports): same
- App C (auth): shared library

Without federation: apps rebuild and redeploy together if shared code changes. With federation: App C publishes its latest build; Apps A and B load it at runtime.

### Mechanism

```javascript
// App C (provider)
new ModuleFederationPlugin({
  name: 'auth',
  exposes: { './useAuth': './src/useAuth.ts' },
  shared: ['react', 'react-dom']
})

// App A (consumer)
new ModuleFederationPlugin({
  remotes: { auth: 'auth@http://localhost:3001/remoteEntry.js' },
  shared: ['react', 'react-dom']
})
```

Runtime loader fetches `remoteEntry.js` from provider, negotiates shared dependency versions, loads consumer's code.

### Trade-offs

- **Deployment complexity**: versioning shared dependencies becomes critical
- **Debugging**: runtime errors span multiple codebases; stack traces harder to interpret
- **Performance**: remote fetches add latency; shared dependency versioning mismatch causes duplicates
- **Cache invalidation**: provider updates require consumer awareness

---

## Dynamic Imports and Lazy Loading

```javascript
const module = await import('./heavy-feature.js')
```

Dynamic imports allow code splitting without explicit chunk configuration. Patterns:

- **Route-based**: lazy-load route components
- **Interaction-based**: load on click
- **Conditional**: load based on feature detection or environment

### HMR with Dynamic Imports

Module replacement in dev works if the import reference persists. Pitfall: redefining the import invalidates the lazy-loaded module.

---

## Bundle Size and Performance

### Typical Optimization Sequence

1. Tree-shake unused ESM exports
2. Code-split routes and heavy dependencies
3. Minify (remove whitespace, shorten identifiers)
4. Compression (gzip/brotli at rest and in transit)
5. Format choice: ESM > CommonJS > UMD (smaller transpilation overhead)

### Metrics

- **Initial JS**: size of main bundle; impacts Time to Interactive
- **Largest Contentful Paint (LCP)**: browser rendering speed
- **First Input Delay (FID)**: JS execution blocking user interaction

Modern bundlers optimize all three; choice depends on project constraints.

---

## See Also

- **build-compilation-incremental.md** — HMR inside Vite and watch mode implementations
- **build-systems-deep.md** — Caching and dependency graphs apply to bundlers
- **web-monorepo-tooling.md** — Multi-platform bundlers and monorepo orchestration