# Type Systems in Practice — Gradual Typing, Flow Analysis & Design Patterns

## Overview

Type systems are a spectrum from static to dynamic, from simple to sophisticated. Modern languages blur this line with **gradual typing**: TypeScript, Python (type hints), Ruby (Sorbet), Hack (PHP dialect) all support progressive adoption of types. Understanding type system trade-offs—soundness, inference capability, annotation burden, runtime overhead—helps choose between strict typing and permissiveness for different contexts.

The key tension: **Completeness vs usability**. Sound type systems are powerful but require annotations. Gradual systems are flexible but permit runtime errors.

## Gradual Typing: Bringing Types to Dynamic Languages

### The Premise

Gradual typing allows **opt-in type checking**: mix typed and untyped code in the same module. The `any` type acts as a bridge, permitting both typed and untyped regions to interact.

### TypeScript: The Pragmatic Approach

TypeScript is JavaScript with optional static types. Key design choices:

1. **Structural typing (by default)**: Type compatibility is shape-based, not name-based.
   ```ts
   interface Point { x: number; y: number; }
   class Vector { x: number; y: number; }
   const p: Point = new Vector(1, 2); // OK: same shape
   ```

2. **Type inference**: Omit annotations where inference is clear.
   ```ts
   const x = 5; // inferred as number
   const add = (a: number, b: number) => a + b; // return type inferred
   ```

3. **Nominal opt-in via `declare`**: Declare a structural interface as nominal (name-based).
   ```ts
   declare class UserId { private readonly __brand: 'UserId'; }
   ```

4. **Gradual with `any` and `unknown`**:
   - `any`: escape hatch (disables type checking)
   - `unknown`: unknown type (requires narrowing before use)

### Python Type Hints: Optional Annotations

PEP 484 introduced optional type hints. Annotations are metadata, not enforced at runtime:

```python
def greet(name: str) -> str:
    return f"Hello, {name}"

greet(42)  # Runtime OK, type checker complains
```

Type checkers (mypy, Pyright) analyze the code without running it. Benefits:
- Gradual adoption: start with critical paths, annotate incrementally
- No runtime overhead: annotations are syntax only
- IDE support: autocomplete, refactoring

**Common issues**:
- Duck typing conflicts: `def process(obj): obj.method()` accepts anything with a `.method()`, but type checker wants explicit protocol
- Generics complexity: `List[T]` requires covariance/contravariance reasoning

### Ruby Sorbet: Strong Gradual Typing

Sorbet adds ahead-of-time type checking to Ruby. Unlike Python, Sorbet can enforce types at runtime via inline runtime checks:

```ruby
sig { params(name: String).returns(String) }
def greet(name)
  "Hello, #{name}"
end
```

Signature strictness levels:
- **`ignore`**: no checking
- **`false`**: runtime checks disabled, type checking enabled
- **`true`**: strict: requires all parameters and returns typed, no forward references
- **`strict`**: strictest: requires all method signatures

## Static vs Structural vs Nominal Typing

### Nominal Typing: Names Matter

Type identity is name-based. `UserId` and `CustomerId` are different even if both are integers. Common in Java, C#, C++.

```java
class UserId { }
class CustomerId { }
UserId u = new CustomerId(); // Compile error
```

**Advantage**: Prevents accidental confusion (wrong kind of ID).
**Disadvantage**: Forces explicit inheritance. Tight coupling to type names.

### Structural Typing: Shape Matters

Type compatibility is shape-based. If two types have the same members and methods, they're compatible. Go, TypeScript (default) use this.

```typescript
interface Writer { write(s: string): void; }
interface Logger { write(s: string): void; }
const log: Writer = { write: (s) => console.log(s) }; // OK
```

**Advantage**: Flexibility, no ceremony.
**Disadvantage**: Can hide intent. Two unrelated types might accidentally match.

### Hybrid: Branded Types (Phantom Types)

Use a phantom field to distinguish structurally identical types:

```typescript
type UserId = number & { readonly __brand: 'UserId' };
type CustomerId = number & { readonly __brand: 'CustomerId' };

const userId: UserId = 1 as UserId;
const customerId: CustomerId = 2 as CustomerId;
const id: UserId = customerId; // Type error!
```

The `__brand` field is never instantiated; it only exists at type-check time. This enables nominal safety with structural ergonomics.

## Type Narrowing & Type Guards

### Type Narrowing

Narrowing refines a union type to a specific member based on runtime checks:

```typescript
function process(x: string | number) {
  if (typeof x === 'string') {
    // x is narrowed to string here
    console.log(x.toUpperCase());
  } else {
    // x is narrowed to number
    console.log(x.toFixed(2));
  }
}
```

Common narrowing:
- `typeof` checks: `typeof x === 'string'`
- `instanceof` checks: `x instanceof Error`
- Truthiness: `if (x) ...`
- Equality: `if (x === null) ...`
- `in` operator: `if ('method' in obj) ...`

### Type Guards: Custom Narrowing

Type predicates (`is` keyword) define custom narrowing:

```typescript
function isError(e: unknown): e is Error {
  return e instanceof Error;
}

function handle(x: unknown) {
  if (isError(x)) {
    console.log(x.message); // x is Error
  }
}
```

Return type `e is Error` tells the type checker: if this function returns true, **x is an Error**.

## Discriminated Unions (Tagged Unions)

### Pattern

A union of object types sharing a **discriminator** field:

```typescript
type Result<T> = 
  | { status: 'success'; value: T }
  | { status: 'failure'; error: string };

function handle(r: Result<number>) {
  if (r.status === 'success') {
    console.log(r.value); // value is defined
  } else {
    console.log(r.error);  // error is defined
  }
}
```

The type checker understands: if `status === 'success'`, only the success variant is possible. All fields of that variant are accessible.

### Benefits

- **Exhaustiveness checking**: compiler ensures all branches handled
- **Safe unwrapping**: no awkward null checks or type assertions
- **Self-documenting**: the union structure describes valid states

## Algebraic Data Types: Sum & Product Types

### Product Types (Tuples, Structs)

Combine multiple types using **and**:
```typescript
type Point = { x: number; y: number; }; // product
type Triple = [string, number, boolean];        // product
```

### Sum Types (Discriminated Unions)

Combine multiple types using **or**:
```typescript
type Value = number | string | boolean;
type Shape = Circle | Square | Triangle;
```

The distinction matters for type safety. A product requires all fields; a sum requires choosing one branch.

### Pattern Matching: Exhaustive Handling

Functional languages (Haskell, ReScript, Rust) use pattern matching for sum types:

```rust
match result {
    Ok(value) => println!("Success: {}", value),
    Err(e) => println!("Error: {}", e),
}
```

TypeScript approximates this with discriminated unions + narrowing.

## Flow Typing & Inference

### Purpose

**Flow typing** (or "flow-sensitive typing") refines types based on control flow. Unlike nominal systems where type is fixed, flow typing updates the known type at each point.

```typescript
let x: string | number = getValue();
if (typeof x === 'string') {
  // x is string here
  console.log(x.length);
} else {
  // x is number here
  console.log(x.toFixed(2));
}
// x is string | number again
```

### Type Inference

Inference lets the checker deduce types from usage:

```typescript
const nums = [1, 2, 3];           // inferred: number[]
const getFirst = (arr) => arr[0]; // inferred: (arr: any[]) => any
```

**Context-sensitive inference**: use the expected type to infer unknowns:

```typescript
const process: (x: number) => string = (x) => x.toString(); // x inferred as number
```

## Soundness & Escape Hatches

### Sound Type Systems

A **sound** type system proves: if code type-checks, it won't have certain classes of errors at runtime (type errors, null dereferences, etc). Java aims for soundness but has casts. Rust achieves soundness via ownership.

### Gradual Typing Breaks Soundness

Mixing typed and untyped regions via `any` / `unknown` breaks soundness. Type 💧:

```typescript
const x: any = "hello";
const n: number = x; // type error suppressed
const result = n.toFixed(2); // Runtime error at runtime!
```

By design, gradual systems trade soundness for pragmatism.

### Escape Hatches

- **`any`**: trust the programmer (disables all checking)
- **Type assertions** (`as`): override inference (should be rare)
- **`non-null` assertion** (`!`): tell checker "this is not null" (use cautiously)

## Type Checking Strategy: When to Use What

### Static (Strict)

- **Pro**: catches errors before runtime, enables optimization, supports refactoring
- **Con**: annotation overhead, slower feedback loop, false positives
- **Use**: critical code paths (payments, auth, security)

### Gradual

- **Pro**: flexible, adopt incrementally, matches ecosystem evolution
- **Con**: errors slip to runtime, longer debugging, team must discipline
- **Use**: most new code; migrate legacy incrementally

### Dynamic

- **Pro**: minimal ceremony, fast to prototype, lightweight
- **Con**: errors at runtime, weak IDE support, hard to refactor
- **Use**: scripts, tooling, rapid prototypes

## Practical Migration Patterns

### TypeScript from JavaScript

1. Enable `checkJs: true` in `tsconfig.json` (type check JS files)
2. Add JSDoc type hints incrementally:
   ```js
   /** @type {number} */
   const id = 123;
   ```
3. Migrate to TypeScript files (.ts) when critical
4. Enable `strict: true` progressively

### Python Type Hints

1. Use type comments where needed:
   ```python
   x = []  # type: List[int]
   ```
2. Add PEP 484 hints to function signatures
3. Run mypy incrementally, configure to strict mode over time
4. Use protocols for duck-typing:
   ```python
   from typing import Protocol
   class SupportsRead(Protocol):
       def read(self) -> str: ...
   ```

### Sorbet in Ruby

1. Start with `typed: ignore` (no checks)
2. Gradually migrate files to `typed: false` (type checking on)
3. Move critical modules to `typed: true` or `typed: strict`

## See Also

[type-systems-theory](type-systems-theory.md), [language-typescript-advanced](language-typescript-advanced.md), [paradigm-type-level-programming](paradigm-type-level-programming.md), [compilers-type-inference](compilers-type-inference.md)