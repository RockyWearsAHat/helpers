# TypeScript Runtime Validation — Schemas, Validation Libraries & Contracts

## Overview

TypeScript types exist only at compile time; at runtime, JavaScript has no built-in way to verify that data matches a type. **Runtime validation libraries** fill this gap by defining schemas (descriptions of expected data shape) and providing functions to validate and parse data against those schemas.

The approach splits into two paradigms: **schema-first** (define schema, derive types) and **code-first** (write types, generate schema). Each has trade-offs. Libraries vary in their feature sets (transforms, refinements, composition), performance profiles, and ecosystem integration (forms, APIs, databases).

## Core Concepts

A validation schema does three things:

1. **Validates:** Returns success or failure when checking data
2. **Parses:** Coerces data to the expected type (e.g., string `"42"` → `number` `42`)
3. **Types:** In code-first libraries, generates TypeScript types from the schema

```typescript
// Example: schema-first (Zod)
const userSchema = z.object({
  id: z.number(),
  name: z.string(),
  email: z.string().email()
});

// Validates and extracts type
type User = z.infer<typeof userSchema>;

const result = userSchema.parse(data);  // Throws on invalid data
// or
const result = userSchema.safeParse(data);  // Returns { success, data } or { success, error }
```

## Zod (Schema-First, Fast)

Zod is the modern default for most TypeScript projects: schema-first design, composable, with excellent error messages.

### Basic Schemas

```typescript
import { z } from "zod";

// Primitives
z.string();
z.number();
z.boolean();
z.date();
z.null();
z.undefined();
z.enum(["admin", "user"]);
z.literal("exact value");

// Collections
z.array(z.string());
z.tuple([z.string(), z.number()]);
z.record(z.number());  // Record<string, number>

// Objects
z.object({
  id: z.number(),
  name: z.string(),
  email: z.string().email()
});

// Extract TypeScript type
type User = z.infer<typeof userSchema>;
```

### Transforms and Refinements

Modify data during parsing or add custom validation:

```typescript
// Transform: modify value
const schema = z.string().transform(val => val.toUpperCase());
schema.parse("hello");  // "HELLO"

// Refinement: custom validation
const schema = z.string().refine(
  val => val.length > 3,
  { message: "Must be longer than 3 chars" }
);

// Pipe multiple transforms
const pipeline = z.string()
  .transform(s => s.trim())
  .transform(s => s.toLowerCase())
  .refine(s => s.length > 0);
```

### Composition

```typescript
const addressSchema = z.object({
  street: z.string(),
  city: z.string(),
  country: z.string()
});

const userSchema = z.object({
  name: z.string(),
  address: addressSchema  // Nest schemas
});

// Combine and extend
const adminSchema = userSchema.extend({
  role: z.literal("admin"),
  permissions: z.array(z.string())
});

// Pick/omit fields
const userPreview = userSchema.pick({ name: true });
const userSafe = userSchema.omit({ password: true });
```

## Zod Advanced Features

### Async Validation

```typescript
const schema = z.object({
  email: z.string().email().refine(
    async (email) => {
      const exists = await checkEmailInDatabase(email);
      return !exists;
    },
    { message: "Email already registered" }
  )
});

await schema.parseAsync(data);
```

### Discriminated Unions

```typescript
const resultSchema = z.discriminatedUnion("status", [
  z.object({ status: z.literal("success"), data: z.any() }),
  z.object({ status: z.literal("error"), error: z.string() })
]);

// Type narrowing works automatically
```

### Error Handling

```typescript
const result = schema.safeParse(data);

if (!result.success) {
  // result.error is a ZodError
  console.log(result.error.issues);  // Array of validation issues
  console.log(result.error.flatten());  // { fieldErrors, formErrors }
}
```

## Yup (Schema-First)

Simpler API than Zod, older but still widely used, especially in form libraries (Formik):

```typescript
import * as yup from "yup";

const userSchema = yup.object().shape({
  name: yup.string().required(),
  email: yup.string().email().required(),
  age: yup.number().positive().integer()
});

await userSchema.validate(data);
// or
userSchema.validateSync(data);
```

Trade-off with Zod: simpler API but less composable, weaker TypeScript integration.

## io-ts (Code-First)

Code-first approach: write codecs (encode/decode pairs), derive schemas and types:

```typescript
import * as t from "io-ts";

const User = t.type({
  id: t.number,
  name: t.string,
  email: t.string
});

type User = t.TypeOf<typeof User>;

// Validate
const result = User.decode(data);  // Either<ValidationError[], User>
```

Strength: seamless TypeScript integration; types and validation always in sync. Weakness: steeper learning curve, fewer users than Zod.

## Typebox (Schema-First, High Performance)

Typebox generates JSON Schema and compiles validators for speed:

```typescript
import { Type } from "@sinclair/typebox";

const User = Type.Object({
  id: Type.Number(),
  name: Type.String(),
  email: Type.String({ format: "email" })
});

type User = Static<typeof User>;

const validate = TypeCompile(User);
validate(data);  // boolean, very fast
```

Useful when validation speed is critical (high-throughput APIs).

## Valibot (Tree-Shakable)

Modern, tree-shakable library designed for minimal bundle size:

```typescript
import { object, string, email, parse } from "valibot";

const userSchema = object({
  name: string(),
  email: email()
});

parse(userSchema, data);
```

Good for client-side validation where bundle size matters.

## Schema-First vs Code-First

| Aspect | Schema-First (Zod, Yup) | Code-First (io-ts) |
|--|--|--|
| **Define** | Write schema object | Write codec/decoder |
| **Types** | `z.infer<T>` extracts types | Types and codecs same |
| **Community** | Larger (Zod dominates) | Smaller, FP-heavy |
| **Learning** | Easier | Steeper, FP concepts |
| **Performance** | Good | Good |
| **Ecosystem** | Rich (forms, APIs, etc) | Smaller |

For most projects, **Zod is the default choice**. Use io-ts if you prefer functional programming or need maximum correctness guarantees.

## Integration Patterns

### Form Libraries (React)

```typescript
// React Hook Form + Zod
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";

const { register, handleSubmit, formState: { errors } } = useForm({
  resolver: zodResolver(userSchema)
});
```

Zod, Yup, and io-ts all integrate with form libraries via resolver adapters.

### API Frameworks

```typescript
// Express + Zod (express-openapi-validator style)
app.post("/users", (req, res) => {
  const result = userSchema.safeParse(req.body);
  if (!result.success) {
    return res.status(400).json({ errors: result.error.flatten() });
  }
  // result.data is typed and validated
});
```

### tRPC (Type-Safe RPC)

tRPC uses Zod (or other) schemas to validate server procedure inputs:

```typescript
import { z } from "zod";
import { t } from "./trpc";

const appRouter = t.router({
  createUser: t.procedure
    .input(z.object({ name: z.string(), email: z.string().email() }))
    .mutation(async ({ input }) => {
      // input is fully typed and validated
    })
});
```

### OpenAPI Generation

Some libraries (e.g., Zod → OpenAPI via `@asteasolutions/zod-to-openapi`) convert schemas to OpenAPI specs:

```typescript
const userSchema = z.object({
  id: z.number().openapi({ description: "User ID" }),
  name: z.string()
});

// Generates OpenAPI schema automatically
```

## Performance Considerations

- **Zod & io-ts:** ~1-5µs per validation (compiled, no reflection)
- **Typebox (compiled):** <1µs per validation; fastest option
- **Yup:** Similar to Zod but slower for complex schemas
- **Valibot:** Comparable to Zod but tree-shakable

For APIs handling thousands of requests/second, micro-benchmark and consider Typebox or precompiled validators.

## Constraints and Patterns

**Validation is not a security layer.** Never rely on client-side validation alone. Always validate on the server.

**Choose one library per project.** Mixing Zod, Yup, and io-ts creates confusion and duplication.

**Schema as documentation.** A well-written schema serves as inline API documentation. Use descriptive property names and add `.describe()` for context.

**Separate validation from transformation.** Perform validation first (reject invalid data), then transform (parse/coerce) to application types.

**Generate schemas from data, not vice versa.** If you have a Prisma model or database schema, generate Zod schemas from it using tools like `prisma-zod-generator`, not the reverse.

## Cross-References

See also: [web-forms-validation.md](web-forms-validation.md), [api-design.md](api-design.md), [api-error-handling.md](api-error-handling.md)