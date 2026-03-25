# JavaScript & TypeScript Best Practices

## TypeScript: Type Safety First

### Core Principle
TypeScript exists to catch bugs at compile time. Use it strictly.

```jsonc
// tsconfig.json — non-negotiable settings
{
  "compilerOptions": {
    "strict": true,                    // Enables ALL strict checks
    "noUncheckedIndexedAccess": true,  // array[0] is T | undefined
    "exactOptionalPropertyTypes": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true
  }
}
```

### Type Design

```typescript
// Prefer interfaces for object shapes (extensible via declaration merging)
interface User {
  readonly id: string;
  name: string;
  email: string;
  role: "admin" | "user" | "viewer";  // Union literal types > enums
}

// Use type for unions, intersections, mapped types
type Result<T> = { ok: true; value: T } | { ok: false; error: Error };

// Discriminated unions — the most powerful TypeScript pattern
type Shape =
  | { kind: "circle"; radius: number }
  | { kind: "rectangle"; width: number; height: number }
  | { kind: "triangle"; base: number; height: number };

function area(shape: Shape): number {
  switch (shape.kind) {
    case "circle":    return Math.PI * shape.radius ** 2;
    case "rectangle": return shape.width * shape.height;
    case "triangle":  return 0.5 * shape.base * shape.height;
  }
  // Exhaustiveness check — compiler error if a case is missed
  const _exhaustive: never = shape;
  return _exhaustive;
}

// Utility types
type Readonly<T> = { readonly [K in keyof T]: T[K] };
type Partial<T> = { [K in keyof T]?: T[K] };
type Pick<T, K extends keyof T> = { [P in K]: T[P] };
type Omit<T, K extends keyof T> = Pick<T, Exclude<keyof T, K>>;
type Record<K extends string, V> = { [P in K]: V };

// const assertions for literal types
const ROUTES = {
  home: "/",
  about: "/about",
  users: "/users",
} as const;
type Route = typeof ROUTES[keyof typeof ROUTES]; // "/" | "/about" | "/users"

// Template literal types
type EventName = `on${Capitalize<string>}`;  // "onClick", "onLoad", etc.
```

### Avoid These

```typescript
// ❌ any — defeats the purpose of TypeScript
function process(data: any) { ... }

// ✅ unknown — forces type narrowing
function process(data: unknown) {
  if (typeof data === "string") {
    return data.toUpperCase();
  }
}

// ❌ Enums — have runtime behavior, don't tree-shake well
enum Direction { Up, Down, Left, Right }

// ✅ Union literals — pure types, zero runtime cost
type Direction = "up" | "down" | "left" | "right";

// ❌ Non-null assertion (!) — suppresses the type checker
const el = document.getElementById("app")!;

// ✅ Proper null handling
const el = document.getElementById("app");
if (!el) throw new Error("Missing #app element");
```

## JavaScript (ES2024+)

### Modern Syntax Essentials

```javascript
// Destructuring
const { name, age, ...rest } = user;
const [first, ...remaining] = items;

// Optional chaining + nullish coalescing
const city = user?.address?.city ?? "Unknown";

// Logical assignment
options.timeout ??= 5000;  // Set only if null/undefined
options.retries ||= 3;      // Set if falsy

// Array methods (avoid manual loops)
const adults = users.filter(u => u.age >= 18);
const names = users.map(u => u.name);
const total = prices.reduce((sum, p) => sum + p, 0);
const found = users.find(u => u.id === targetId);
const allValid = items.every(item => item.isValid);
const hasError = items.some(item => item.error);

// Object.groupBy (ES2024)
const byRole = Object.groupBy(users, u => u.role);
// { admin: [...], user: [...] }

// Structured clone (deep copy)
const copy = structuredClone(original);

// at() for negative indexing
const last = array.at(-1);

// Promise.withResolvers (ES2024)
const { promise, resolve, reject } = Promise.withResolvers();
```

### Async/Await Patterns

```javascript
// Sequential (when order matters)
const user = await getUser(id);
const posts = await getPosts(user.id);

// Parallel (independent operations)
const [user, config, notifications] = await Promise.all([
  getUser(id),
  getConfig(),
  getNotifications(id),
]);

// Parallel with error tolerance
const results = await Promise.allSettled([
  fetchFromPrimary(),
  fetchFromBackup(),
]);
const successes = results
  .filter(r => r.status === "fulfilled")
  .map(r => r.value);

// Race (first to resolve wins)
const fastest = await Promise.race([
  fetchFromCDN1(url),
  fetchFromCDN2(url),
]);

// Error handling
async function fetchUser(id) {
  try {
    const response = await fetch(`/api/users/${id}`);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    return await response.json();
  } catch (error) {
    if (error instanceof TypeError) {
      // Network error
      throw new NetworkError("Failed to reach server", { cause: error });
    }
    throw error;
  }
}
```

### Module System

```javascript
// Named exports (preferred — tree-shakeable)
export function createUser(data) { ... }
export const MAX_RETRIES = 3;

// Default export (only for main module entry)
export default class Router { ... }

// Dynamic import (code splitting)
const { Chart } = await import("./chart.js");

// Re-export
export { createUser, deleteUser } from "./users.js";
export type { User } from "./types.js";  // Type-only re-export (TS)
```

## Error Handling

```javascript
// Custom error classes
class AppError extends Error {
  constructor(message, options = {}) {
    super(message, { cause: options.cause });
    this.code = options.code;
    this.statusCode = options.statusCode ?? 500;
  }
}

class NotFoundError extends AppError {
  constructor(resource, id) {
    super(`${resource} ${id} not found`, { code: "NOT_FOUND", statusCode: 404 });
  }
}

class ValidationError extends AppError {
  constructor(errors) {
    super("Validation failed", { code: "VALIDATION_ERROR", statusCode: 400 });
    this.errors = errors;
  }
}

// Error cause chain (ES2022)
try {
  await connectToDatabase();
} catch (error) {
  throw new AppError("Failed to initialize", { cause: error });
}
```

## Tooling

| Tool | Purpose |
|------|---------|
| **ESLint** | Linting (catch bugs, enforce patterns) |
| **Prettier** | Formatting (end style debates) |
| **TypeScript** | Type checking |
| **Vitest** / **Jest** | Testing |
| **Biome** | All-in-one (lint + format, Rust-based, fast) |
| **tsx** | Run TypeScript directly (dev) |
| **esbuild** / **Vite** | Bundling |

## Common Pitfalls

1. **`==` vs `===`**: Always use `===`. The `==` operator has bizarre coercion rules.
2. **`typeof null === "object"`**: Historical bug. Check with `value === null`.
3. **Floating point**: `0.1 + 0.2 !== 0.3`. Use integers (cents, not dollars) for money.
4. **`for...in` on arrays**: Iterates keys (strings), not values. Use `for...of` or array methods.
5. **Async in forEach**: `array.forEach(async ...)` doesn't await. Use `for...of` with await, or `Promise.all` with map.
6. **Object mutation**: Spread `{ ...obj }` is shallow. Use `structuredClone()` for deep copies.

---

*Sources: TypeScript Handbook, MDN Web Docs, TC39 Proposals, Effective TypeScript (Dan Vanderkam), JavaScript: The Good Parts (Douglas Crockford)*
