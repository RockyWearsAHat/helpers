# TypeScript Decorators — Stage 3 Spec, Metadata & Dependency Injection

## Overview

Decorators are functions that modify class declarations, methods, properties, or parameters at definition time. TypeScript supports two decorator specifications: the **legacy experimental decorators** (historically TypeScript's interpretation) and the **Stage 3 ECMAScript proposal** (modern standard, default in TS 5.0+).

Both enable **metaprogramming** — writing code that inspects and transforms code structure at runtime. Common use cases include dependency injection frameworks (tsyringe, inversify), ORM decorators (TypeORM, MikroORM), and API route definitions (NestJS).

## Stage 3 Decorators (Modern Standard)

Stage 3 is the current JavaScript proposal. TypeScript enables it by default in 5.0+ with `experimentalDecorators: false` (or omitted).

### Class Decorators

Wrap the class constructor and can replace it:

```typescript
function sealed(constructor: Function) {
  Object.seal(constructor);
  Object.seal(constructor.prototype);
}

@sealed
class User {
  name: string = "Alice";
}

const user = new User();
// Cannot add properties to User or its instance
```

A class decorator receives the constructor function and returns the same or a new one:

```typescript
function addTimestamp<T extends { new(...args: any[]): {} }>(constructor: T) {
  return class extends constructor {
    createdAt = new Date();
  };
}

@addTimestamp
class Document { }

const doc = new Document();
console.log(doc.createdAt);  // Date instance
```

### Method Decorators

Intercept method calls by wrapping the descriptor:

```typescript
function memoize(target: any, propertyKey: string, descriptor: PropertyDescriptor) {
  const originalMethod = descriptor.value;
  const cache = new Map();

  descriptor.value = function(...args: any[]) {
    const key = JSON.stringify(args);
    if (cache.has(key)) return cache.get(key);

    const result = originalMethod.apply(this, args);
    cache.set(key, result);
    return result;
  };

  return descriptor;
}

class Math {
  @memoize
  fibonacci(n: number): number {
    return n <= 1 ? n : this.fibonacci(n - 1) + this.fibonacci(n - 2);
  }
}
```

Signature: `(target: any, propertyKey: string | symbol, descriptor: PropertyDescriptor): PropertyDescriptor | void`

### Property Decorators

Access and modify property initialization:

```typescript
function readonly(target: any, propertyKey: string) {
  Object.defineProperty(target, propertyKey, {
    writable: false,
    configurable: false
  });
}

class Config {
  @readonly
  apiKey = "secret";
}

const cfg = new Config();
cfg.apiKey = "changed";  // Error: cannot assign to read-only property
```

Signature: `(target: any, propertyKey: string | symbol): void`

### Parameter Decorators (Metadata)

Mark parameters for framework consumption (no direct effect):

```typescript
function validate(target: any, propertyKey: string | undefined, parameterIndex: number) {
  // Metadata stored; frameworks like tsyringe read it
}

class Service {
  process(@validate data: string) { }
}
```

Signature: `(target: any, propertyKey: string | symbol | undefined, parameterIndex: number): void`

## Metadata Reflection

The `reflect-metadata` library enables runtime type introspection. Enable with:

```json
{
  "compilerOptions": {
    "experimentalDecorators": true,
    "emitDecoratorMetadata": true
  }
}
```

```typescript
import "reflect-metadata";

function logType(target: any, propertyKey: string) {
  const type = Reflect.getMetadata("design:type", target, propertyKey);
  console.log(`${propertyKey}: ${type.name}`);
}

class User {
  @logType
  name: string = "";
}

// Logs: name: String
```

Metadata types include:
- `design:type` — property type
- `design:paramtypes` — function parameter types
- `design:returntype` — function return type

## Decorator Factories

Return a decorator function to parameterize behavior:

```typescript
function log(prefix: string = "LOG") {
  return (target: any, propertyKey: string, descriptor: PropertyDescriptor) => {
    const originalMethod = descriptor.value;

    descriptor.value = function(...args: any[]) {
      console.log(`${prefix}: ${propertyKey} called`);
      return originalMethod.apply(this, args);
    };

    return descriptor;
  };
}

class API {
  @log("DEBUG")
  fetch(url: string) { }
}
```

## Dependency Injection Frameworks

### tsyringe

Lightweight DI container using decorators:

```typescript
import { container, injectable, inject } from "tsyringe";

@injectable()
class Logger { }

@injectable()
class Service {
  constructor(@inject("logger") public logger: Logger) { }
}

const service = container.resolve(Service);
```

### inversify

Enterprise-grade DI with advanced features:

```typescript
import { Container, injectable, inject } from "inversify";
import "reflect-metadata";

const TYPES = { Logger: Symbol.for("Logger") };

@injectable()
class Logger { }

@injectable()
class Service {
  constructor(@inject(TYPES.Logger) public logger: Logger) { }
}

const container = new Container();
container.bind(TYPES.Logger).to(Logger);
container.bind("Service").to(Service);

const service = container.get(Service);
```

Both frameworks leverage `emitDecoratorMetadata` to extract constructor types at runtime.

## Legacy Experimental Decorators

The older TypeScript-specific decorator implementation, now superseded. Enable with `experimentalDecorators: true` (without Stage 3).

Differences from Stage 3:
- Method decorator signature includes `descriptor` as third parameter (Stage 3 wraps it differently)
- Decorator order is different (Stage 3 applies bottom-up)
- Property decorators run at class definition time (Stage 3 runs at construction time)

Legacy decorators are **deprecated**. New projects should use Stage 3. Existing legacy projects work but should migrate.

## Migration from Legacy to Stage 3

Most decorators work unchanged. Key differences:

1. **Method decorator descriptor handling:** Stage 3 is more permissive; both read/replace behavior work.
2. **Decorator application order:** Reversed. Matters only if decorators have interdependencies.
3. **Property decorator timing:** Stage 3 applies to each instance; legacy applies to the prototype.

Test existing decorator implementations after changing `experimentalDecorators`.

## Compiler Requirements

Enable decorators in `tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2020",  // Or higher; Stage 3 requires ES2020+
    "lib": ["ES2020"],
    "experimentalDecorators": false,  // Stage 3 (default in 5.0+)
    "emitDecoratorMetadata": true      // If using metadata reflection
  }
}
```

- **Stage 3 requires:** `experimentalDecorators: false` (or omitted)
- **Legacy requires:** `experimentalDecorators: true`

## Constraints and Patterns

**Decorators are metaprogramming.** They obscure behavior by hiding side effects in the declaration. Overuse makes code harder to trace. Use them for cross-cutting concerns (logging, validation, DI) only; avoid for core logic.

**Metadata reflection adds runtime overhead.** Each decorated element reflects its type at runtime. Measure performance in loops or frequently-instantiated classes.

**Stage 3 is the standard.** Use it for new code. Only use legacy if maintaining existing codebases that depend on legacy behavior.

**Document decorator contracts.** If writing custom decorators, document what they do (side effects, parameter requirements, return value).

**Type safety isn't guaranteed:** Decorators operate at runtime. A decorator can't enforce that a class implements an interface; it can only modify structure. Combine with runtime validation for full safety.

## Cross-References

See also: [language-typescript-advanced.md](language-typescript-advanced.md), [web-typescript-patterns.md](web-typescript-patterns.md), [architecture-ddd.md](architecture-ddd.md)