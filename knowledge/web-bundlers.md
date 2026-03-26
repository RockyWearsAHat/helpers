# Web Bundlers — Webpack, Vite, esbuild, Turbopack, Rollup

JavaScript bundlers combine source files into optimized artifacts for distribution. The landscape includes specialized tools for different use cases: dev-time experience, production optimization, library distribution, and build speed.

## The Bundling Problem and Solution

Browsers historically could not load bare CommonJS or ES module specifiers. **Bundlers solve three problems:**
1. **Module resolution** — Convert ESM `import` statements to code the browser can execute
2. **Tree-shaking** — Eliminate unused code in the dependency graph
3. **Code splitting** — Partition output into chunks loaded on-demand, reducing initial payload

Bundlers differ in philosophy: Webpack optimizes for configuration flexibility, Vite prioritizes dev speed via native ESM, esbuild chases raw compilation speed (written in Go), Rollup focuses on library distribution.

## Webpack: Configuration-Driven Bundling

Webpack is the industry standard for complex applications. Its philosophy: **everything is a module**, configured through a central `webpack.config.js`.

### Core Concepts

**Loaders** transform source files:
- `babel-loader` — transpile ES6 to ES5
- `css-loader` — parse CSS imports and resolve dependencies
- `file-loader` / `asset/resource` — emit binary files (images, fonts)
- `ts-loader` — compile TypeScript

Loaders run in a **pipeline**: a file enters as source code, exits as JavaScript or CSS. Order matters — loaders execute **right-to-left**:
```javascript
module: {
  rules: [{
    test: /\.css$/,
    use: ['style-loader', 'css-loader']  // css-loader runs first
  }]
}
```

**Plugins** hook into the bundling lifecycle (can't transform individual files). Key plugins:
- `HtmlWebpackPlugin` — generate HTML with injected script tags
- `DefinePlugin` — replace global variables (e.g., `process.env.NODE_ENV`)
- `MiniCssExtractPlugin` — extract CSS into separate files

### Code Splitting

Webpack supports multiple strategies:
- **Entrypoints** — explicit entry points create separate chunks
- **Dynamic import** — `import()` creates async chunks
- **SplitChunksPlugin** — extract common dependencies into a shared chunk

```javascript
optimization: {
  splitChunks: {
    chunks: 'all',
    cacheGroups: {
      vendor: {
        test: /[\\/]node_modules[\\/]/,
        name: 'vendors',
        priority: 10
      }
    }
  }
}
```

### Downsides

- **Slow cold start** — webpack rebuilds the entire graph on file change, even in watch mode (historically; recent versions improved)
- **Configuration complexity** — loaders, plugins, splitters create a steep learning curve
- **HMR limitations** — hot module replacement often requires framework-specific plugins

## Vite: ESM-First Development

Vite inverts the traditional bundler model for dev: **serve unprocessed source files directly to the browser**, leveraging native ESM.

### Development Mode

In dev, Vite:
1. Starts a **dev server** that serves files as-is
2. Browser requests `index.html` → receives bundled HTML
3. Browser encounters `import './App.tsx'` → requests from server
4. Server returns source file (optionally transpiled for browser compatibility)
5. Prebuilt dependencies (node_modules) are cached indefinitely

**No bundling during development.** The browser executes modules directly. This means:
- HMR is instant — restart only the affected module
- Build tools (TypeScript, JSX) are optimized for single-file transforms, not graph rebuilds

### Production Mode

Production uses **Rollup** under the hood for optimization (tree-shaking, minification, lazy-loaded chunks, proper module bundling). Configuration via `vite.config.ts`:

```javascript
export default defineConfig({
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ['react', 'lodash']
        }
      }
    }
  }
})
```

### Strengths

- Blazingly fast dev experience (no bundling, instant HMR)
- Unified config — dev and prod share the same entry point
- Framework integrations are minimal (plugins for Vue, React JSX)

### Limitations

- Browser must support ESM (not IE11, but that's fine in 2026)
- Dependencies must export ESM or have pre-bundled versions

## esbuild: Go-Based Compilation

esbuild prioritizes **raw compilation speed** by writing the bundler in Go instead of JavaScript. It's 10-100x faster than webpack for large codebases.

Its philosophy: **minimal, opinionated defaults**. Trade-off: fewer configuration knobs. Common use cases:
- Build pipelines where bundler speed is a bottleneck
- CLI tools and development utilities
- Bundling server-side code

### Capabilities

- Transpile TypeScript, JSX natively (no Babel)
- Code splitting and tree-shaking
- Minification and source maps
- Plugin API (Go-based, runs during bundling)

### Limitations

- No built-in CSS support beyond imports (delegates to standard CSS)
- No HMR in esbuild itself (frameworks build HMR on top)
- Plugin ecosystem is smaller than webpack

esbuild is often used as **a foundation**, not a standalone choice. Frameworks like Astro and Vite's production build use esbuild or Rollup for different stages.

## Turbopack: Rust-Based, Incremental

Turbopack (Vercel) promises next-gen bundling via Rust and **incremental computation**. The core innovation: cache and reuse compilation results across builds and across machines.

Not yet production-mature (as of 2026), but demonstrates the direction: **speeding up the full bundler by caching**, not just dev-time optimizations.

## Rollup: Library Bundling

Rollup specializes in **library distribution**. It's optimized for:
- Outputting multiple formats (ESM, CJS, UMD)
- Minimal output size — tree-shakes aggressively
- Plugins for external dependencies

Rollup's philosophy differs from Webpack: **idiomatic ESM input and output**. Use cases:
- Building npm packages
- Frameworks that compile to multiple targets

```javascript
export default {
  input: 'src/index.js',
  output: [
    { file: 'dist/index.esm.js', format: 'es' },
    { file: 'dist/index.cjs.js', format: 'cjs' },
    { file: 'dist/index.umd.js', format: 'umd', name: 'MyLib' }
  ],
  external: ['react', 'lodash']
}
```

## Tree-Shaking: Eliminating Dead Code

**Tree-shaking** removes imports and statements that are never used. It relies on:
- **Static module structure** — ESM `import/export` must be top-level, not dynamic
- **Side-effect analysis** — bundler determines whether code has observable effects

Example:
```javascript
// util.js
export function used() { return 42; }
export function unused() { return 0; }

// app.js
import { used } from './util.js'
console.log(used())
```

A tree-shaking bundler removes the `unused` function from the output. Webpack and Rollup support this via `sideEffects: false` in package.json (signals that imports don't run initialization code).

Trade-off: **static analysis is conservative**. Bundlers can't detect all side effects, so they may keep more code than necessary. Complex conditional logic or reflection can prevent tree-shaking.

## Hot Module Replacement (HMR)

HMR updates running code without a page reload. Requires:
1. **Dev server** that watches source files
2. **HMR client** in the browser that receives update notifications
3. **Module replacement** logic in the app

Frameworks implement HMR on top of bundlers (React Fast Refresh, Vue's HMR, Svelte reactivity). The bundler provides the foundation; framework integration handles state preservation (e.g., Redux state survives a component update).

## Source Maps

Source maps bridge compiled code back to source. A `.js.map` file contains:
- Original source filenames and line numbers
- Variable/function names
- Transpilation mappings

Development includes full source maps (large, slow). Production typically uses hidden source maps (uploaded to monitoring services like Sentry) or omits them entirely.

## Bundle Analysis

Understanding what's in a bundle clarifies optimization opportunities. Tools:
- **Webpack Bundle Analyzer** — visualizes chunk composition
- **source-map-explorer** — maps final output back to source files
- **rollup-plugin-visualizer** — charts dependency sizes

Common findings:
- Duplicate dependencies (multiple versions installed)
- Heavy frameworks included but rarely used
- Polyfills loading for features not needed in target browsers

## Choice Matrix

| Bundler    | Best For                          | Philosophy                      | Trade-Off                      |
|------------|-----------------------------------|---------------------------------|--------------------------------|
| Webpack    | Complex apps, legacy tooling      | Flexible configuration          | Slow dev, steep learning curve |
| Vite       | Modern apps, speed-sensitive      | ESM-native, minimal config      | Requires ESM deps              |
| esbuild    | CLI tools, speed critical         | Minimal, fast                   | Fewer features                 |
| Turbopack  | Large monorepos (when stable)     | Incremental computation         | Very new, not production-ready |
| Rollup     | Libraries, multi-format output    | Idiomatic ESM library bundling  | Less suitable for apps         |

Practical reality: Vite for new projects, esbuild for build scripts, Webpack for legacy systems. Rollup for libraries.