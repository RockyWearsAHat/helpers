# TypeScript Module System — ESM, CJS, Resolution & Declarations

## Overview

The TypeScript module system bridges source code organization and JavaScript runtime execution. It involves three concerns: **module syntax** (how you write imports/exports), **module resolution** (how identifiers get mapped to files), and **declaration files** (.d.ts) which separate types from implementation.

The system is layered: TypeScript's `moduleResolution` setting controls how it resolves module paths at compile time, but runtime behavior depends on your JavaScript target (ESM or CommonJS) and your bundler's configuration.

## Module Syntax: ESM vs CommonJS

### ESM (ECMAScript Modules)

Modern standard syntax, supported natively in browsers and Node.js ≥12:

```typescript
// Export
export const add = (a: number, b: number): number => a + b;
export default class Calculator { }
export type Point = { x: number; y: number };
export { add, Calculator };
export { add as sum } from "./arithmetic";

// Import
import { add, Calculator } from "./lib";
import DefaultExport from "./lib";
import * as math from "./lib";
import type { Point } from "./lib";
import { add as sum } from "./lib";
```

ESM is the long-term standard. New projects should prefer ESM.

### CommonJS (CJS)

Node.js's original module system, still widespread in npm:

```typescript
// Export
module.exports = { add: (a, b) => a + b };
exports.multiply = (a, b) => a * b;

// Import
const { add, multiply } = require("./lib");
const lib = require("./lib");
```

### Interoperability: ESM ↔ CJS

Mixed module ecosystems create friction:

- **ESM importing CJS:** Works via default import `import cjs from "cjs-package"`, or named import if the CJS module has a default export.
- **CJS importing ESM:** Problematic. ESM is async and top-level, CJS is synchronous. Some bundlers provide workarounds (e.g., transforming ESM to CJS), but the cleanest approach is keeping projects monolithic in module type.
- **Type-only imports:** `import type { Foo }` from CJS works safely since types aren't runtime values.

The `package.json` field determines module type:

```json
{
  "type": "module",          // ESM (default in modern Node.js)
  "exports": {
    ".": "./dist/index.js",  // Maps root import to file
    "./utils": "./dist/utils.js"
  }
}
```

## Module Resolution: Finding Files

`moduleResolution` in `tsconfig.json` controls how TypeScript resolves `import "foo"` to a file.

### Modes

- **node** (classic Node.js behavior, deprecated): Looks in `node_modules`, uses `package.json` only for `main` field.
- **node10 / node16** (modern Node.js): Honors `package.json` `exports` (ESM priority), respects `.d.ts` files, handles CJS/ESM hybrid packages.
- **bundler** (for bundler environments): Like `node16` but assumes bundler will resolve everything; skips `.d.ts` discovery.
- **nodenext** (new default): Alias for `node16` in TS 5.0+; future-proofs to new Node.js resolution.

### Example Resolution: `import { utils } from "my-package"`

With `moduleResolution: "node16"`:

1. Read `node_modules/my-package/package.json`
2. Check `exports` field for exact or wildcard matches
3. If not found, fall back to `main` field
4. If `main` points to `dist/index.js`, look for `dist/index.d.ts` (types first)
5. If not found, look for inline types in `dist/index.js`

With `moduleResolution: "bundler"`:

Similar, but bundler assumes it will resolve everything; doesn't require `.d.ts` files to exist in the package.

### Best Practice

Use `moduleResolution: "node16"` (or `nodenext`) for libraries and `node` only for legacy projects. Bundler mode is for bundler-integrated development only.

## Declaration Files (.d.ts)

Declaration files contain type information without implementation. They're the TypeScript equivalent of C header files.

### Generating Declarations

In `tsconfig.json`:

```json
{
  "compilerOptions": {
    "declaration": true,        // Generate .d.ts from .ts
    "declarationMap": true,     // Source maps for .d.ts
    "emitDeclarationOnly": true // Only emit .d.ts, no JS
  }
}
```

### Ambient Declarations

Declare types without implementation:

```typescript
// lib.d.ts
declare const version: string;
declare function process(input: string): void;
declare class Logger { }
declare namespace Math { }

declare global {
  interface Window { myApp: any; }
}
```

Ambient declarations make types available to the entire project without imports.

### Triple-Slash Directives

Legacy way to reference external type definitions (mostly superseded by imports):

```typescript
/// <reference path="./types.d.ts" />
/// <reference lib="dom" />
/// <reference types="node" />
```

Avoid triple-slash directives in modern projects; use `import type` instead.

## Module Augmentation

Extend existing module types without modifying source:

```typescript
// In your app code
declare module "some-library" {
  export interface LibType {
    newMethod(): void;
  }
}

// Use the augmented type
import { LibType } from "some-library";
const obj: LibType = { newMethod() { } };
```

### Global Augmentation

Add to global scope:

```typescript
declare global {
  interface Window {
    myGlobalAPI: { doSomething(): void };
  }
}

// Access anywhere
window.myGlobalAPI.doSomething();
```

## Path Mapping and baseUrl

Use `paths` and `baseUrl` to simplify imports:

```json
{
  "compilerOptions": {
    "baseUrl": "./src",
    "paths": {
      "@components/*": ["components/*"],
      "@utils/*": ["utils/*"],
      "@/*": ["*"]
    }
  }
}
```

Then import as:

```typescript
import { Button } from "@components/Button";
import { logger } from "@utils/logger";
```

Trade-offs:

- **Pros:** Shorter, more semantic import paths; refactoring friendly
- **Cons:** Requires bundler/runtime to understand the same mapping; can obscure dependency direction

Bundlers (webpack, Vite, esbuild) understand TypeScript path mappings natively. For runtime environments, use node's `subpath exports` in `package.json` instead.

## Project References

Link multiple `tsconfig.json` files for monorepos:

```json
{
  "references": [
    { "path": "../core" },
    { "path": "../utils" }
  ]
}
```

Enables incremental builds and separate type-checking per workspace. Each project compiles independently, and the root project depends on the output.

## @types and the DefinitelyTyped Registry

For untyped JavaScript libraries, community types live in `@types/*`:

```bash
npm install --save-dev @types/lodash
```

TypeScript auto-discovers `@types/*` packages in `node_modules/@types`. Use `skipLibCheck: true` to skip type-checking of declarations (speeds up builds when dealing with many `@types` packages):

```json
{
  "compilerOptions": {
    "skipLibCheck": true
  }
}
```

## Constraints and Patterns

**Maintain single module strategy.** Mixing ESM and CJS in the same project creates complexity. Pick one and use it consistently; only engage interop when integrating legacy code.

**Use `import type` liberally.** It signals compile-time-only imports and enables `importsNotUsedAsValues` checks, catching dead code.

**Generate and version declaration files.** If shipping a library, commit `.d.ts` files to source control or ensure they're built as part of CI.

**Test with different module targets.** Build and test against both `esnext` and `commonjs` targets if your library supports both.

**Document resolution choices.** If using non-standard path mappings or custom `exports` in `package.json`, document the reasoning so future maintainers understand the layout.

## Cross-References

See also: [bundling-module-systems.md](bundling-module-systems.md), [language-typescript-advanced.md](language-typescript-advanced.md), [web-bundlers.md](web-bundlers.md)