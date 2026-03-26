# TypeScript Type Manipulation — Conditional Types, Mapped Types & Type-Level Programming

## Overview

TypeScript's type system transcends type checking: you can compute types using conditional logic, transformation patterns, and recursive structures. This is **type-level programming** — treating types as values that can be manipulated, validated, and composed at compile time.

The core tools are **conditional types** (type-level if/then/else), **mapped types** (transform all properties), **template literal types** (build strings from types), and **recursive types** (types that reference themselves). Together they enable generic utilities that adapt to input types, domain-driven type constraints, and compile-time validation of API contracts.

## Conditional Types: Type-Level Control Flow

Conditional types encode logic in the type system using the syntax `T extends U ? X : Y`.

```typescript
// Basic conditional: does T extend string?
type IsString<T> = T extends string ? true : false;

type A = IsString<"hello">;        // true
type B = IsString<42>;              // false
```

### The `infer` Keyword

`infer` binds a type variable within a conditional type, enabling type extraction and analysis.

```typescript
// Extract the element type of an array
type ElementType<T> = T extends (infer E)[] ? E : never;
type Numbers = ElementType<number[]>;  // number
type Strings = ElementType<string[]>;  // string

// Extract return type of a function
type ReturnType<F> = F extends (...args: any[]) => infer R ? R : never;
type Fn = () => string;
type FnReturn = ReturnType<Fn>;  // string
```

`infer` can appear multiple times to extract nested structure:

```typescript
type AwaitedType<T> = T extends Promise<infer U> ? U : T;
type V = AwaitedType<Promise<number>>;  // number

// Extract from union of promises
type UnionAwaited = AwaitedType<Promise<string> | Promise<number>>;  // string | number
```

### Distributive Conditional Types

When a conditional type is applied to a **union**, it distributes across each constituent.

```typescript
type ToArray<T> = T extends any ? T[] : never;

// Union distributes:
type A = ToArray<string | number>;  // string[] | number[] (NOT (string | number)[])
```

This is convenient for filtering and transforming union members:

```typescript
// Extract only string members from a union
type StringOnly<T> = T extends string ? T : never;
type S = StringOnly<string | number | boolean>;  // string

// Exclude a type from a union
type Exclude<T, U> = T extends U ? never : T;
type NonStrings = Exclude<string | number | boolean, string>;  // number | boolean
```

To prevent distribution, wrap the type variable in a tuple:

```typescript
type ToArrayNoDistribute<T> = [T] extends [any] ? T[] : never;
type B = ToArrayNoDistribute<string | number>;  // (string | number)[]
```

## Mapped Types: Transform All Properties

Mapped types iterate over keys and transform each property:

```typescript
// Make all properties optional
type Partial<T> = { [K in keyof T]?: T[K] };

// Make all properties readonly
type Readonly<T> = { readonly [K in keyof T]: T[K] };

// Make all properties string values
type Stringify<T> = { [K in keyof T]: string };

interface User { id: number; name: string; }
type UserStrings = Stringify<User>;  // { id: string; name: string; }
```

### Filtering and Remapping Keys

You can filter keys and remap their values:

```typescript
// Only copy methods, skip properties
type MethodsOnly<T> = {
  [K in keyof T as T[K] extends Function ? K : never]: T[K]
};

class API {
  request(url: string) { }
  data = 42;
}
type APIMethods = MethodsOnly<API>;  // { request: (url: string) => void }

// Convert getter names to setter names
type Getters<T> = {
  [K in keyof T as `get${Capitalize<string & K>}`]: () => T[K]
};

interface Props { name: string; age: number; }
type Getters = Getters<Props>;  // { getName: () => string; getAge: () => number; }
```

### Useful Built-in Mapped Type Utilities

- `Partial<T>` — all properties optional
- `Required<T>` — all properties required
- `Readonly<T>` — all properties readonly
- `Pick<T, K>` — select specific keys
- `Omit<T, K>` — exclude specific keys
- `Record<K, T>` — map keys K to type T
- `Extract<T, U>` — union members assignable to U
- `Exclude<T, U>` — union members not assignable to U

## Template Literal Types: Type-Level String Building

Template literal types construct new string types by interpolating type-level values:

```typescript
type Greeting<Name extends string> = `Hello, ${Name}!`;
type G1 = Greeting<"Alice">;  // "Hello, Alice!"
type G2 = Greeting<"Bob">;    // "Hello, Bob!"

// Literal unions become all combinations
type Method = "GET" | "POST";
type Route = "/users" | "/posts";
type Endpoints = `${Method} ${Route}`;
// GET /users | GET /posts | POST /users | POST /posts
```

Common patterns include branded types, prefixing/suffixing, and validating string formats:

```typescript
// Event handler names
type EventHandler<T extends string> = `on${Capitalize<T>}`;
type ClickHandler = EventHandler<"click">;    // "onClick"
type ChangeHandler = EventHandler<"change">; // "onChange"

// Path construction
type Path = `/${string}`;
type ValidPath = Path;  // enforces leading slash at type level
```

## Recursive Types: Self-Referential Structures

Types can reference themselves, enabling encoding of deeply nested structures:

```typescript
// Deep partial: all nested object properties optional
type DeepPartial<T> = T extends object ? {
  [P in keyof T]?: DeepPartial<T[P]>
} : T;

interface Config {
  server: { host: string; port: number };
  db: { url: string };
}

type PartialConfig = DeepPartial<Config>;
// server and db are both optional, and their properties are too
```

Recursive types enable traversal and validation:

```typescript
// Get all paths through a nested object
type Paths<T, P extends string = ""> = T extends object
  ? { [K in keyof T]: Paths<T[K], `${P}${P extends "" ? "" : "."}${string & K}`> }[keyof T]
  : P;

type ConfigPaths = Paths<Config>;
// "server" | "server.host" | "server.port" | "db" | "db.url"

// Limit recursion depth
type DeepPartial<T, D extends number = 5> = D extends 0
  ? T
  : T extends object
    ? { [P in keyof T]?: DeepPartial<T[P], [-1, -1, -1, -1, -1][D]> }
    : T;
```

## Type Introspection: Inspecting Type Structure

Extract and analyze type structure:

```typescript
// Is a type a function?
type IsFunction<T> = T extends Function ? true : false;

// Extract union member count (approximately)
type UnionSize<T> = T extends any ? T : never;

// Get all union members as tuple (requires helper)
type UnionToIntersection<U> = 
  (U extends any ? (k: U) => void : never) extends (k: infer I) => void ? I : never;

// Extract promise or unwrap
type Asyncify<T> = T extends Promise<any> ? T : Promise<T>;
```

## Constraints and Best Practices

**Readability > cleverness.** Complex types are harder to debug and maintain. Use helper types with clear names rather than nesting complex conditionals.

**Limit recursion depth.** Unbounded recursion causes compiler slowness and cryptic errors. Add a depth parameter or bail-out condition.

**Test at the edge.** Conditional types behave unexpectedly with unions, `never`, `any`, and `unknown`. Write tests with `assertType` or use type assertions to verify edge cases.

**Document intent.** Add comments explaining what a complex mapped type does and why. Future maintainers will appreciate it.

**Distribute or don't.** Most of the time you want distribution across unions. When you don't, wrap in tuple. Document the reason.

## Cross-References

See also: [paradigm-type-level-programming.md](paradigm-type-level-programming.md), [web-typescript-patterns.md](web-typescript-patterns.md), [language-typescript-advanced.md](language-typescript-advanced.md)