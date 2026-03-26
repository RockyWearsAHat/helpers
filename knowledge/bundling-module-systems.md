# Module Systems & Bundling — From Scripts to Modules to Optimized Bundles

## The Problem Modules Solve

Early web development loaded JavaScript as a series of `<script>` tags. Every script shared a single global namespace. This created collisions, ordering dependencies, and made large-scale development fragile.

```html
<!-- Order matters. Forgetting a script or reordering breaks things silently. -->
<script src="jquery.js"></script>
<script src="utils.js"></script>
<script src="app.js"></script>
<!-- depends on both above -->
```

Module systems emerged to solve three problems: **namespace isolation** (don't pollute globals), **explicit dependencies** (declare what you need), and **encapsulation** (expose only the public API).

## Evolution of Module Patterns

### Global Scripts (Pre-Module Era)

Every variable declared at the top level becomes a global property. Libraries claimed global names (`$`, `_`, `React`) and hoped no one else used the same name.

**Problem:** No encapsulation, no dependency declaration, implicit ordering requirements, collision risk grows with project size.

### IIFE (Immediately Invoked Function Expressions)

```javascript
var MyModule = (function () {
  var privateState = 0;
  return {
    increment: function () {
      return ++privateState;
    },
  };
})();
```

Function scope created privacy. The revealing module pattern exposed public APIs while hiding internals. This was a convention, not a module system — no dependency management, no standard loading mechanism.

### AMD (Asynchronous Module Definition)

Designed for the browser, where scripts load asynchronously over the network.

```javascript
define("myModule", ["dependency1", "dependency2"], function (dep1, dep2) {
  return {
    /* module exports */
  };
});
```

**What it solved:** Async loading, explicit dependency declaration, no globals.

**Trade-offs:** Verbose wrapper syntax, required a loader library in the runtime, callback-based API felt unnatural for synchronous-thinking developers. The factory function wrapper added boilerplate to every file.

### CommonJS

Designed for server environments where modules load synchronously from the local filesystem.

```javascript
const dep = require("./dependency");

function helper() {
  /* ... */
}

module.exports = { helper };
```

**What it solved:** Clean syntax, synchronous `require()` matching how developers think about dependencies, natural fit for server-side code.

**Trade-offs:** Synchronous `require()` is problematic in browsers (network latency). `require()` is dynamic — the argument can be a computed string, making static analysis difficult. `module.exports` can be assigned at any point in execution, making exports determination a runtime property.

**Characteristics relevant to tooling:**

- `require()` calls can appear anywhere — inside conditionals, loops, functions
- The resolved module is determined at runtime, not statically
- Circular dependencies resolve to partially-constructed exports objects
- Module identity is tied to file path (resolved through a specific algorithm)

### ES Modules (ESM)

The language-level standard, designed for static analysis.

```javascript
import { helper } from "./dependency.js";
export function process(data) {
  /* ... */
}
```

**Design choices and their consequences:**

| ESM Property               | Design Rationale                   | Consequence                                             |
| -------------------------- | ---------------------------------- | ------------------------------------------------------- |
| Static `import`/`export`   | Enable compile-time analysis       | Tree shaking becomes possible                           |
| `import` must be top-level | Predictable dependency graph       | No conditional imports (use dynamic `import()` instead) |
| Live bindings              | Exports are references, not copies | Circular dependencies work more predictably             |
| Strict mode by default     | Safer defaults                     | Some legacy code requires adaptation                    |
| Async evaluation           | Browser-compatible                 | Different timing semantics than CommonJS                |

**The CJS-ESM interop challenge:** The two systems have fundamentally different models. CJS uses synchronous evaluation with value copies; ESM uses asynchronous evaluation with live bindings. Bridging them requires compromise — default export mapping, conditional exports in `package.json`, dual-package hazard (loading the same package as both CJS and ESM creates two separate instances).

## Module Systems vs Bundlers: The Conceptual Distinction

A **module system** defines how code is organized, dependencies declared, and modules resolved. It answers: "how do I reference other code?"

A **bundler** is a build tool that processes modules into deployment-optimized output. It answers: "how do I package modules for efficient delivery?"

```
Module System:  defines the dependency graph
                import/export, require(), dependency resolution

Bundler:        transforms the graph for deployment
                concatenation, splitting, minification, optimization
```

These concerns are distinct but intertwined. The design of the module system determines what a bundler can optimize. Static imports enable tree shaking; dynamic `require()` defeats it. The bundler's capabilities influence how developers write module code.

## Tree Shaking — Dead Code Elimination via Import Analysis

Tree shaking removes unused exports from the final bundle. The metaphor: shake the dependency tree and see what falls off (unused code).

### How It Works

1. Build the full module dependency graph from entry points
2. For each module, determine which exports are actually imported by something in the graph
3. Eliminate exports that no live code path references
4. Remove modules entirely if none of their exports are used

```javascript
// math.js
export function add(a, b) {
  return a + b;
}
export function multiply(a, b) {
  return a * b;
}
export function complexInfrequentOperation() {
  /* 500 lines */
}

// app.js
import { add } from "./math.js";
console.log(add(1, 2));
// multiply and complexInfrequentOperation are tree-shaken out
```

### Why Static Analysis Is Required

Tree shaking depends on knowing which exports are used **without executing the code**. This is only possible with ES module static imports.

```javascript
// Statically analyzable — bundler knows exactly what's used
import { add } from "./math.js";

// NOT statically analyzable — which export is used depends on runtime value
const fn = require("./math")[someCondition ? "add" : "multiply"];
```

CommonJS's dynamic nature means `require()` calls generally cannot be tree-shaken. This is a primary reason the ecosystem moved toward ES modules for library distribution.

### Side Effects and Tree Shaking

A module has **side effects** if importing it causes observable changes beyond providing exports — modifying globals, registering event listeners, writing to the DOM, polyfilling APIs.

```javascript
// side-effect-free: safe to remove if exports unused
export function format(date) {
  return date.toISOString();
}

// has side effects: removing this changes program behavior
import "./polyfill.js"; // modifies globals
import "./register-handler"; // registers a listener
```

Bundlers must be conservative: if they can't prove a module is side-effect-free, they must include it even if no exports are used. Package authors can signal side-effect-free status through metadata (e.g., `"sideEffects": false` in `package.json`), but incorrect annotations cause bugs where necessary initialization code gets removed.

**The tension:** Aggressive tree shaking produces smaller bundles but risks removing code with needed side effects. Conservative tree shaking is safer but produces larger bundles.

## Code Splitting

Code splitting divides a bundle into multiple chunks loaded on demand. The goal: load only the code needed for the current user interaction, deferring the rest.

### Why It Matters

A single monolithic bundle forces the user to download and parse everything upfront — including code for features they may never visit. Code splitting trades one large download for multiple smaller ones, improving initial load time.

### Entry Point Splitting

Multiple entry points (e.g., different pages of an application) produce separate bundles. Shared dependencies are extracted into common chunks to avoid duplication.

```
entry: home.js   → chunk-home.js   + chunk-shared.js
entry: admin.js  → chunk-admin.js  + chunk-shared.js
```

### Dynamic Import Splitting

`import()` expressions create split points. The imported module (and its dependencies) become a separate chunk loaded at runtime.

```javascript
// Clicking "Settings" loads the settings code on demand
button.addEventListener("click", async () => {
  const { renderSettings } = await import("./settings.js");
  renderSettings();
});
```

The bundler sees `import('./settings.js')` and carves out a separate chunk. The runtime loads it over the network when the `import()` executes.

### Splitting Strategies and Trade-offs

| Strategy         | Approach                                | Trade-off                                                                             |
| ---------------- | --------------------------------------- | ------------------------------------------------------------------------------------- |
| Route-based      | One chunk per route/page                | Intuitive, but shared code between routes may duplicate or require careful extraction |
| Component-based  | Heavy components split independently    | Fine-grained but many small chunks increase HTTP overhead                             |
| Vendor splitting | Third-party code in separate chunk      | Better caching (vendor code changes less often) but increases request count           |
| Size-based       | Split chunks exceeding a size threshold | Automatic but may split at arbitrary module boundaries                                |

**The fundamental trade-off:** More granular splitting means smaller initial loads but more network requests. HTTP/2 multiplexing reduces the per-request overhead, shifting the optimal balance toward finer splitting. But each chunk still has parsing/evaluation overhead.

## Module Resolution Algorithms

When code says `import 'foo'`, something must determine which file `'foo'` refers to. Resolution algorithms differ by context.

### Node.js Resolution (CommonJS/ESM)

```
require('foo')
  1. Is 'foo' a core module (fs, path, etc.)? → use it
  2. Does 'foo' start with './' or '/'? → resolve as file path
  3. Search node_modules/ directories, walking up from the requiring file
     ./node_modules/foo/ → ../node_modules/foo/ → ../../node_modules/foo/ → ...
  4. Within the package: check package.json "main"/"exports", then index.js
```

**Extension resolution:** Node tries `.js`, `.json`, `.node` extensions. This implicit resolution, convenient for developers, means file dependencies aren't fully specified — different contexts might resolve the same specifier differently.

### Package Exports (`"exports"` field)

Modern `package.json` uses the `"exports"` field to declare entry points with conditions:

```json
{
  "exports": {
    ".": {
      "import": "./dist/esm/index.js",
      "require": "./dist/cjs/index.js",
      "types": "./dist/types/index.d.ts"
    },
    "./utils": "./dist/utils.js"
  }
}
```

This enables **conditional resolution** — the same package serves different files to different consumers (ESM vs CJS, Node vs browser, development vs production). It also encapsulates the package: imports of internal paths not listed in `"exports"` are blocked.

### Bundler Resolution

Bundlers generally start from Node.js resolution but extend it:

- **Alias resolution** — mapping bare specifiers to specific paths (`@/components → src/components`)
- **Browser field** — `package.json`'s `"browser"` field swaps Node-specific modules for browser equivalents
- **Conditions** — bundlers can resolve based on custom conditions beyond `import`/`require`
- **TypeScript path mapping** — `tsconfig.json` paths influence resolution for `.ts` files

The layered resolution creates complexity: the same import specifier can resolve to different files depending on which tool is doing the resolving (Node, bundler, TypeScript, test runner).

## Source Maps

Source maps connect transformed code back to original source, enabling debugging of minified/bundled/transpiled code.

### Structure

A source map is a JSON file containing mappings between positions in the generated code and positions in the original source files. The format uses VLQ (Variable Length Quantity) encoding to compress the position data.

```
generated code position  →  original file, line, column, symbol name
```

### Source Map Concerns

| Concern             | Description                                                                                                                                      |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Accuracy**        | Multi-step transforms (TS → JS → bundle → minify) must compose source maps at each stage. Lost or inaccurate mappings make debugging misleading. |
| **Performance**     | Large source maps add download overhead. Inline source maps bloat bundle size; external source maps require additional requests.                 |
| **Security**        | Source maps in production expose original source code. Can serve them behind authentication or omit from public CDNs.                            |
| **Variable naming** | Minification renames variables. Source maps track name mappings so debuggers can show original names.                                            |

### Approaches to Serving Source Maps

- **Inline** — embedded in the bundle as a data URL. Simplest but increases bundle size.
- **External** — separate `.map` file referenced by a comment in the bundle. Standard approach for production.
- **Hidden** — generated for error reporting tools but not referenced in the bundle. No `//# sourceMappingURL` comment.
- **Omitted** — no production source maps at all. Trading debuggability for reduced exposure.

## Minification and Compression

### Minification

Transforms code to reduce size while preserving behavior: remove whitespace, shorten identifiers, eliminate dead code, simplify expressions.

```javascript
// Before minification
function calculateTotalPrice(items, taxRate) {
  let total = 0;
  for (const item of items) {
    total += item.price * item.quantity;
  }
  return total * (1 + taxRate);
}

// After minification
function a(b, c) {
  let d = 0;
  for (const e of b) d += e.price * e.quantity;
  return d * (1 + c);
}
```

**Minification-hostile patterns:**

- Property access via string keys (`obj["name"]`) can't be renamed because the string might come from external data
- `eval()` and `new Function()` create scopes invisible to static analysis
- Reflection APIs that reference functions/properties by name

### Compression

Minification reduces source size; compression (gzip, Brotli) reduces transfer size. These are complementary, operating at different layers — minification at build time, compression at serve time. Compression still achieves 60-80% further reduction on top of minification.

## The Development Server Model

### Unbundled Development

Instead of bundling everything upfront, serve modules individually and let the browser's native module system handle imports. When the browser requests a module, the server transforms it on-demand (transpile TypeScript, process CSS, resolve bare imports).

```
Browser requests /src/app.tsx
  → Dev server transpiles TSX → JS on the fly
  → Browser parses, encounters: import { Button } from './Button'
  → Browser requests /src/Button.tsx
  → Dev server transpiles on demand
  → ... (recursive for all dependencies)
```

**Advantages:** Startup is near-instant (no upfront bundling). HMR (Hot Module Replacement) only re-processes the changed file, not the entire bundle.

**Trade-offs:** First page load may be slow with many modules (hundreds of HTTP requests). Deep dependency chains create waterfall loading. This model is development-only — production benefits from bundling.

### Hot Module Replacement (HMR)

Replace changed modules in the running application without a full page reload, preserving application state. HMR requires module-level knowledge of how to "accept" updates. Framework integrations handle this for component frameworks, but arbitrary code may not be safely hot-replaceable.

## Import Maps

A browser-native mechanism for controlling module resolution without a bundler:

```html
<script type="importmap">
  {
    "imports": {
      "lodash": "/vendor/lodash-es/lodash.js",
      "react": "https://cdn.example.com/react/18.2.0/react.esm.js",
      "@/utils/": "/src/utils/"
    }
  }
</script>
<script type="module">
  import { debounce } from "lodash"; // resolved via import map
</script>
```

Import maps enable bare specifier resolution (`import 'lodash'`) in browsers without bundling, opening a path toward bundler-free deployments for simpler applications. For complex dependency graphs, bundlers still provide optimizations (tree shaking, code splitting) that import maps alone don't address.

## The Bundle Size vs Request Count Trade-off

A fundamental tension in web delivery:

| Fewer, Larger Bundles                 | More, Smaller Chunks                     |
| ------------------------------------- | ---------------------------------------- |
| Fewer HTTP requests                   | Better cache granularity                 |
| Better compression (more repetition)  | Faster initial load (only needed chunks) |
| Single parse/compile event per bundle | More parallel loading via HTTP/2         |
| Coarse cache invalidation             | Fine-grained cache invalidation          |

| Factor             | Favors Fewer Bundles         | Favors More Chunks                  |
| ------------------ | ---------------------------- | ----------------------------------- |
| HTTP version       | HTTP/1.1 (connection limits) | HTTP/2+ (multiplexing)              |
| Update frequency   | Rarely changing code         | Frequently changing code            |
| Application size   | Small applications           | Large applications with many routes |
| User patterns      | Users visit all features     | Users visit subset of features      |
| Network conditions | High-latency connections     | High-bandwidth connections          |

There is no universally correct answer — the right strategy depends on the specific application, its users, and their network conditions.

## Module System Interoperability

Real-world projects often mix module formats. Build tools handle this through format detection (`package.json` `"type"` field, `.mjs`/`.cjs` extensions), on-the-fly CJS-to-ESM wrapping, default export heuristics, and dual packages shipping both formats via conditional exports.

**The dual-package hazard:** If both CJS and ESM versions of a package are loaded in the same application, they're separate module instances with separate state. Singleton patterns, registries, and instance checks break. Package authors use various strategies to mitigate this (shared state files, single-format publishing, conditional exports that prevent dual loading).

## Ecosystem Patterns

### Package Metadata for Bundlers

Beyond standard Node.js fields, `package.json` has evolved bundler-consumed fields:

```json
{
  "main": "./dist/cjs/index.js",
  "module": "./dist/esm/index.js",
  "types": "./dist/types/index.d.ts",
  "sideEffects": false,
  "exports": {
    /* conditional exports */
  }
}
```

Each field emerged to solve a specific tooling problem. The layered accretion reflects incremental ecosystem evolution rather than upfront design.
