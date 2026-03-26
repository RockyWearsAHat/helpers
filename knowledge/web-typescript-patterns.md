# Advanced TypeScript Patterns — Discriminated Unions, Template Literal Types, Conditional Types & More

Advanced TypeScript patterns leverage the type system to encode domain logic at compile time. Rather than enforcing contracts at runtime, these patterns prevent certain invalid states from being representable in the code.

## Discriminated Unions: Type-Safe State Machines

A **discriminated union** (or **tagged union**) combines multiple possible shapes with a literal **discriminant field** that narrows the type automatically.

### Basic Example

Without discriminated unions:
```typescript
type User = {
  type: 'user';
  name: string;
  email: string;
}

type Admin = {
  type: 'admin';
  name: string;
  email: string;
  permissions: string[];
}

type Account = User | Admin;

function process(account: Account) {
  // TypeScript doesn't know if .permissions exists
  if (account.email === 'admin@example.com') {
    console.log(account.permissions); // ERROR: Property 'permissions' may not exist
  }
}
```

With discriminated unions:
```typescript
type Account = 
  | { type: 'user'; name: string; email: string }
  | { type: 'admin'; name: string; email: string; permissions: string[] };

function process(account: Account) {
  if (account.type === 'admin') {
    // TypeScript now knows account is Admin
    console.log(account.permissions); // OK
  }
}
```

The **discriminant** (`type` field) is a literal type that exhaustively distinguishes each union member.

### Use Cases

- **API response types:** Success vs error, with different shapes
- **State machines:** Idle, loading, success, error states
- **UI components:** Different render paths based on state

### Exhaustiveness Checking

TypeScript can verify you've handled all union cases:
```typescript
type RequestState = 
  | { status: 'idle' }
  | { status: 'loading' }
  | { status: 'success'; data: unknown }
  | { status: 'error'; error: Error };

function render(state: RequestState): string {
  switch (state.status) {
    case 'idle': return 'Ready';
    case 'loading': return 'Loading...';
    case 'success': return `Data: ${state.data}`;
    case 'error': return `Error: ${state.error.message}`;
    // If you forget a case, TypeScript errors here:
    // No case for 'pending' — TS catches it
  }
}

// Even better, use a default case that never executes:
function render(state: RequestState): string {
  switch (state.status) {
    case 'idle': return 'Ready';
    case 'loading': return 'Loading...';
    case 'success': return `Data: ${state.data}`;
    case 'error': return `Error: ${state.error.message}`;
    default: {
      const exhaustive: never = state;
      throw new Error(`Unhandled case: ${exhaustive}`);
    }
  }
}
```

## Template Literal Types: Encode Constraints in Types

**Template literal types** are string types computed from unions and type operations. They represent strings that match specific patterns.

### Basic Example

```typescript
type Greeting = `hello ${string}`;
const msg: Greeting = 'hello world'; // OK
const bad: Greeting = 'goodbye'; // ERROR: Type '"goodbye"' is not assignable

type HexColor = `#${string}`;
const color: HexColor = '#FF5733'; // OK
```

### Practical Uses: Event Emitters

Encode event type safety:
```typescript
type EventMap = {
  'user:login': { userId: string };
  'user:logout': { userId: string };
  'error:occurred': { code: number; message: string };
};

type EventKey = keyof EventMap; // 'user:login' | 'user:logout' | 'error:occurred'

class EventEmitter<T extends EventMap> {
  on<K extends EventKey>(event: K, handler: (data: T[K]) => void): void {
    // handler receives the correct data type for each event
  }

  emit<K extends EventKey>(event: K, data: T[K]): void {
    // emit validates that data matches the event
  }
}

const emitter = new EventEmitter<EventMap>();
emitter.on('user:login', (data) => {
  // data is { userId: string }
  console.log(data.userId);
});

emitter.emit('user:login', { userId: '123' }); // OK
emitter.emit('user:login', { userId: '123', extra: 'bad' }); // ERROR: extra property
```

### Union Distribution

Template literal types distribute across unions:
```typescript
type Keys = 'foo' | 'bar';
type Prefixed = `prefix_${Keys}`; // 'prefix_foo' | 'prefix_bar'
```

This enables generic string manipulations in the type system.

## Mapped Types: Transform Type Structures

**Mapped types** iterate over keys in a type and produce new types. They're the TypeScript equivalent of `map()` over objects.

### Basic Example

Make all properties optional:
```typescript
type User = { name: string; email: string; age: number };
type Partial<T> = { [K in keyof T]?: T[K] };
type OptionalUser = Partial<User>; // { name?: string; email?: string; age?: number }
```

Make all properties readonly:
```typescript
type ReadonlyUser = { readonly [K in keyof User]: User[K] };
```

### Practical: API Serialization

```typescript
type Model = {
  id: number;
  name: string;
  createdAt: Date;
  updatedAt: Date;
};

// User API sees only serializable types
type Serialized<T> = {
  [K in keyof T]: T[K] extends Date ? string : T[K]
};

type SerializedModel = Serialized<Model>; 
// { id: number; name: string; createdAt: string; updatedAt: string }
```

### Key Manipulation

Prefix/suffix keys:
```typescript
type Getters<T> = {
  [K in keyof T as `get${Capitalize<string & K>}`]: () => T[K]
};

type UserGetters = Getters<User>;
// { getName: () => string; getEmail: () => string; getAge: () => number }
```

## Conditional Types: Type-Level If-Then-Else

**Conditional types** (`T extends U ? X : Y`) enable logic in the type system. They're checked at compile time.

### Basic Example

```typescript
type IsString<T> = T extends string ? true : false;
type A = IsString<'hello'>; // true
type B = IsString<number>; // false
```

### The `infer` Keyword: Extract Types from Shapes

`infer` captures types from within a conditional check:

```typescript
// Extract the element type from an array
type ExtractArray<T> = T extends (infer R)[] ? R : never;
type A = ExtractArray<string[]>; // string
type B = ExtractArray<number>; // never (not an array)

// Extract the return type of a function
type ReturnType<T> = T extends (...args: any[]) => infer R ? R : never;
type A = ReturnType<(x: number) => string>; // string
```

### Practical: Promise/Async Handling

```typescript
type Awaited<T> = T extends Promise<infer U> ? U : T;
type A = Awaited<Promise<string>>; // string
type B = Awaited<number>; // number

// Use in async function return types
async function fetchUser(): Promise<User> { ... }
type UserData = Awaited<ReturnType<typeof fetchUser>>; // User
```

### Distributive Conditional Types

Conditional types distribute over union types:
```typescript
type Flatten<T> = T extends Array<infer R> ? R : T;
type A = Flatten<string[] | number>; // string | number

// Equivalent to (string | number), not "flatten all types"
```

To prevent distribution, wrap the type in an array:
```typescript
type FlattenOnce<T> = T extends Array<infer R> ? R : T;
type FlattenWrapped<T> = [T] extends [Array<infer R>] ? R : T;

type A = FlattenOnce<string[] | number>; // string | number (distributes)
type B = FlattenWrapped<string[] | number>; // string[] | number (no distribution)
```

## The `satisfies` Operator: Validate Without Widening

### The Problem

```typescript
const config = { apiUrl: 'https://api.example.com', port: 3000 };
// config type is { apiUrl: string; port: number }
// We lose information that 'port' is specifically 3000

const port = config.port; // type: number (not 3000)
```

### The Solution

```typescript
type Config = { apiUrl: string; port: number };
const config = { apiUrl: 'https://api.example.com', port: 3000 } satisfies Config;

// config type is still { apiUrl: 'https://api.example.com'; port: 3000 }
const port = config.port; // type: 3000 (literal preserved)
```

`satisfies` validates the value against a type **without widening** the inferred type. Useful for:
- Ensuring an object matches a schema while preserving literal types
- Configuration objects with discriminated unions
- CSS-in-JS where you preserve exact color values for autocomplete

## Branded Types: Semantic Type Safety

**Branded types** use a `__brand` property to create distinct types at compile time, preventing accidental mixing.

```typescript
type UserId = string & { readonly __brand: 'UserId' };
type Email = string & { readonly __brand: 'Email' };

// Constructor functions (these run at build time; TypeScript erases them)
const UserId = (id: string): UserId => id as UserId;
const Email = (email: string): Email => email as Email;

function sendMessage(to: Email, from: Email): void { ... }

const userId = UserId('user123');
const email = Email('alice@example.com');

sendMessage(email, email); // OK
sendMessage(userId, email); // ERROR: UserId is not assignable to Email
sendMessage(email, userId); // ERROR: UserId is not assignable to Email
```

Trade-off: Branded types are erased at runtime; they add compile-time safety only. Useful for domain-specific types (IDs, tokens, validated strings).

## Builder Pattern: Fluent, Type-Safe APIs

The builder pattern chains method calls, each returning `this` (or a specialized type). Combined with overloads, it enables compile-time validation.

```typescript
class QueryBuilder {
  private filters: Array<{ field: string; value: any }> = [];
  private limit?: number;

  where(field: string, value: any): this {
    this.filters.push({ field, value });
    return this;
  }

  limit(n: number): this {
    this.limit = n;
    return this;
  }

  build(): Query {
    return { filters: this.filters, limit: this.limit };
  }
}

const query = new QueryBuilder()
  .where('age', 18)
  .where('country', 'US')
  .limit(10)
  .build();
```

More advanced: **overloading methods depending on prior chain state** to enforce ordering:

```typescript
class AnimationBuilder {
  private duration: number | null = null;

  duration(ms: number): Omit<AnimationBuilder, 'duration'> & { easing: (type: string) => void } {
    this.duration = ms;
    return this;
  }
  
  easing(type: 'ease' | 'linear'): this {
    return this;
  }
}
```

(In practice, this is complex; TypeScript's structural typing makes it tricky. Discriminated unions + conditional types are often clearer.)

## Type-Safe Event Emitters

Combine discriminated unions, template literal types, and generics:

```typescript
type EventMap = {
  'login': { userId: string };
  'logout': { userId: string };
  'error': { code: number };
};

class TypedEmitter<T extends EventMap> {
  private listeners: Map<string, Set<(...args: any[]) => void>> = new Map();

  on<K extends keyof T>(
    event: K,
    handler: (data: T[K]) => void
  ): void {
    if (!this.listeners.has(String(event))) {
      this.listeners.set(String(event), new Set());
    }
    this.listeners.get(String(event))!.add(handler);
  }

  emit<K extends keyof T>(event: K, data: T[K]): void {
    this.listeners.get(String(event))?.forEach(h => h(data));
  }
}

const emitter = new TypedEmitter<EventMap>();
emitter.on('login', (data) => console.log(data.userId)); // data typed correctly
emitter.emit('login', { userId: '123' }); // OK
emitter.emit('login', { invalid: true }); // ERROR
```

## Module Augmentation: Extend External Types

**Module augmentation** merges additional type definitions into an existing module, useful for custom plugins or extensions.

```typescript
// custom-plugin.ts
declare module 'express' {
  interface Request {
    userId?: string;
  }
}

// Later in code:
app.use((req, res, next) => {
  req.userId = extractUserFromToken(req.headers.authorization);
  next();
});

app.get('/profile', (req, res) => {
  // req.userId is now a known property
  console.log(req.userId);
});
```

Without module augmentation, TypeScript doesn't know about the `userId` property.

## Summary

These patterns encode domain constraints in the type system, preventing invalid states and enabling compile-time validation:

| Pattern | Problem Solved | Trade-Off |
|---------|----------------|-----------|
| Discriminated Unions | Exhaustiveness, narrow types safely | Requires explicit discriminant |
| Template Literal Types | Pattern matching on strings | Limited to string unions |
| Mapped Types | Avoid repetition in type definitions | Complex syntax, harder to read |
| Conditional Types + infer | Extract types from structures | Distribute over unions unintentionally |
| satisfies | Preserve literal types while validating | Less common than `as` casts |
| Branded Types | Semantic type safety | No runtime protection, verbose setup |
| Builder Pattern | Fluent, chainable APIs | Trade-off between clarity and type safety |
| Module Augmentation | Extend external types | Fragile if external library changes |

The goal of advanced patterns: shift error detection from runtime to compile time, making errors loud and impossible to ship.