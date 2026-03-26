# TypeScript Compiler Internals — Type Inference, Structural Typing, Module Resolution & Performance

## Overview

TypeScript's compiler is a multi-phase beast: parser → AST → type checker → code generator. Understanding its internals explains surprising behaviors (excess property checking, inference limits), informs tsconfig tuning, and guides large project architecture (project references, incremental builds).

## Type Inference Algorithm

TypeScript uses a **bidirectional** inference strategy combining bottom-up (expression → type) and top-down (context → type) analysis.

### Basic Inference (Bottom-Up)

The compiler infers types from literals and operations:

```typescript
let x = 5;           // x: number (literal type narrowed to number)
let arr = [1, 2, 3]; // arr: number[]
let obj = {a: 1};    // obj: {a: number}
```

### Contextual Typing (Top-Down)

Expected types flow down from assignments, function calls, array literals:

```typescript
// Context flows from return type annotation
function process(fn: (x: number) => string) {
    return fn(42);
}

process((x) => x.toString());  // x: number (from context, not inference)

// Array context
const tuple: [number, string] = [42, "hello"];  // Types enforced
const items: string[] = ["a", "b"];
```

### Generic Type Parameter Inference

Inference extends type variables:

```typescript
function identity<T>(x: T): T { return x; }

const n = identity(42);      // T inferred as number
const s = identity("hello"); // T inferred as string

// Multiple parameters and constraints
function merge<T, U>(x: T, y: U): T & U {
    return {...x, ...y} as T & U;
}
const result = merge({a: 1}, {b: 2});  // T = {a: 1}, U = {b: 2}
```

### Best/Common Type

When no explicit type exists, the compiler computes a best common type:

```typescript
const values = [1, "two", 3];  // (string | number)[] — common type of union
const funcs = [() => 1, () => "hello"];  // (() => number | string)[]
```

### Inference Limits

Inference stops at type variable boundaries. You must annotate:

```typescript
// ❌ Function return type cannot be inferred from body alone
const add = (a: number, b: number) => a + b;  // add: (a: number, b: number) => number — OK because + result is number

// ❌ Complex generic inference fails; annotate
function process<T extends Record<string, any>>(obj: T, key: keyof T) {
    return obj[key];  // Return type inferred, but often wrong without annotation
}

// ✅ Annotate to guide
function process<T extends Record<string, any>>(obj: T, key: keyof T): T[typeof key] {
    return obj[key];
}
```

## Structural Typing

TypeScript uses **structural subtyping**: if two types have the same shape, they're compatible (unlike nominal typing in Java/C# where types must be explicitly related):

```typescript
interface Cat {
    meow(): void;
}

class Dog {
    meow() { console.log("woof"); }  // Same method
}

const c: Cat = new Dog();  // ✅ Compatible; both have meow()
```

**Consequence:** Type equivalence is shape-based. Renaming breaks nothing if shape matches:

```typescript
type UserA = {name: string, age: number};
type UserB = {name: string, age: number};

const user: UserA = {name: "Alice", age: 30};
const b: UserB = user;  // ✅ Compatible; same shape
```

### Excess Property Checking

TypeScript adds a **nominal layer** in object literals: extra properties are rejected at assignment:

```typescript
interface Config {
    url: string;
}

// ❌ Compile error: typo "urll" not in Config
const cfg: Config = {url: "example.com", urll: "wrong"};

// ✅ OK — no excess checking without literal
const cfg2: Config = {url: "example.com", urll: "wrong"} as any;
const obj = {url: "example.com", urll: "wrong"};
const cfg3: Config = obj;  // ✅ Object literal assigned fresh; excess properties ignored
```

**Why?** Excess properties often indicate typos. The check only applies to fresh object literals, not variables.

## Variance Annotations

Variance controls how generic types relate when their type arguments differ. TypeScript infers variance but allows explicit control via `in`/`out` keywords (TypeScript 4.7+):

```typescript
// Covariant (default): T in output position only
interface Producer<out T> {
    get(): T;
}

// Contravariant: T in input position only
interface Consumer<in T> {
    accept(x: T): void;
}

// Invariant: T in both; can't assign if types differ
interface Container<T> {
    get(): T;
    set(x: T): void;
}

const catProducer: Producer<Cat> = ...;
const animalProducer: Producer<Animal> = catProducer;  // ✅ Covariant (Cat is Animal)

const animalConsumer: Consumer<Animal> = ...;
const catConsumer: Consumer<Cat> = animalConsumer;  // ✅ Contravariant (accepts any, including cats)

const catContainer: Container<Cat> = ...;
const animalContainer: Container<Animal> = catContainer;  // ❌ Invariant (type mismatch)
```

## Declaration Merging

Interfaces and modules can be merged, enriching definitions:

```typescript
// First declaration
interface User {
    name: string;
}

// Merged declaration
interface User {
    age: number;
}

// Result: User has both name and age
const user: User = {name: "Alice", age: 30};

// Module merging (namespace)
namespace MyLib {
    export const version = "1.0";
}

namespace MyLib {
    export function greet() { console.log("hi"); }
}

MyLib.version;
MyLib.greet();  // Both accessible
```

## Module Resolution

TypeScript resolves imports via multiple strategies (node, node16, bundler, classic). Each prioritizes differently:

### `node` (default, CommonJS-like)

1. Check relative paths (file + index).
2. Check `node_modules`.
3. Check `baseUrl` (if set).
4. Check `paths` (if set in tsconfig).

```typescript
// import from './utils' → checks ./utils.ts, ./utils/index.ts
// import from 'lodash' → checks node_modules/lodash (package.json → main/exports)
```

### `node16` / `nodenext` (modern, ESM-aware)

Respects ES module semantics: checks `package.json` `exports` field:

```json
{
    "exports": {
        ".": {
            "import": "./dist/index.mjs",
            "require": "./dist/index.js"
        },
        "./utils": {
            "import": "./dist/utils.mjs",
            "require": "./dist/utils.js"
        }
    }
}
```

### `baseUrl` and `paths`

Map module names to directories:

```json
{
    "compilerOptions": {
        "baseUrl": ".",
        "paths": {
            "@app/*": ["src/*"],
            "@types/*": ["types/*"]
        }
    }
}
```

```typescript
import {User} from "@app/models/user";  // Resolves to src/models/user.ts
```

## Project References

Multi-project monorepos use project references to compile separately and reuse compiled outputs:

```json
// tsconfig.json (root)
{
    "files": [],
    "references": [
        {"path": "./packages/core"},
        {"path": "./packages/ui"}
    ]
}

// packages/core/tsconfig.json
{
    "compilerOptions": {"outDir": "dist"},
    "include": ["src/**/*"]
}

// packages/ui/tsconfig.json
{
    "compilerOptions": {"outDir": "dist"},
    "references": [{"path": "../core"}],
    "include": ["src/**/*"]
}
```

**Benefit:** Incremental builds. Only recompile changed projects.

## tsconfig Deep Dive

### Strict Mode (`strict: true`)

Enables all strict type checks. Equivalent to:

```json
{
    "compilerOptions": {
        "noImplicitAny": true,
        "noImplicitThis": true,
        "strictNullChecks": true,
        "strictFunctionTypes": true,
        "strictBindCallApply": true,
        "strictPropertyInitialization": true,
        "alwaysStrict": true
    }
}
```

**Start with strict; relax only where necessary.**

### Module and Target

- `module`: Output format (`esnext`, `es2020`, `commonjs`, `umd`).
- `target`: JavaScript version to emit for (`es5`, `es2020`, `esnext`).

```json
{
    "module": "esnext",     // Keep imports as-is; bundler handles
    "target": "es2020"      // Emit for modern browsers
}
```

### Key Options for Large Projects

```json
{
    "compilerOptions": {
        "skipLibCheck": true,           // Skip type-checking node_modules (often mistyped)
        "forceConsistentCasingInFileNames": true,  // Enforce case sensitivity (Windows issue)
        "incremental": true,            // Cache incremental builds
        "tsBuildInfoFile": ".tsbuildinfo",
        "noEmitOnError": true,          // Don't emit .js if errors exist
        "declaration": true,            // Emit .d.ts
        "declarationMap": true,         // Map declarations to source
        "sourceMap": true,              // Maps for debugging
        "rootDir": "./src",
        "outDir": "./dist"
    }
}
```

## Performance Tuning

1. **Enable incremental builds:** `incremental: true` + `.tsbuildinfo` file.
2. **Skip lib checking:** `skipLibCheck: true` (don't type-check all of node_modules).
3. **Use project references:** Split monorepos into independent projects.
4. **Reduce `include` scope:** Don't include test files in main compile.
5. **Disable declaration generation** in dev builds: declarative emit is slow.
6. **Use `ts-node --transpile-only`** for development (skip type-checking).
7. **Profile with `--diagnostics`:** `tsc --diagnostics --listFilesOnly > report.txt`

## Best Practices

- **Annotate function signatures,** not every variable. Let inference fill in.
- **Use `unknown` not `any`.** Forces explicit type narrowing.
- **Avoid `as` casts.** Prefer type guards and contraints.
- **Enable `strict` mode.** Never disable individual checks; that's configuration debt.
- **Don't rely on inference for complex generic types.** Annotate.
- **Use `Partial<T>`, `Pick<T, K>`, `Record<K, V>`** for mapped types instead of ad-hoc interfaces.

## See Also

- **language-javascript-typescript.md** — TypeScript idioms and conventions
- **web-typescript-patterns.md** — Advanced patterns (discriminated unions, branded types)
- **type-systems-theory.md** — Type theory (variance, soundness, subtyping)
- **compilers-type-inference.md** — General type inference algorithms (Hindley-Milner)