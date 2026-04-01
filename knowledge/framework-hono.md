# Hono

## Overview

Hono (炎 — "flame" in Japanese) is an ultra-lightweight, multi-runtime web framework. It runs on Cloudflare Workers, Deno, Bun, Node.js, Vercel Edge, AWS Lambda, and more — write once, deploy anywhere. At ~14KB, it's smaller than Express but offers more built-in features.

## Multi-Runtime Support

Hono's runtime-agnostic design uses the Web Standards API (Request, Response):

```typescript
// Same code runs on every runtime
import { Hono } from "hono";

const app = new Hono();
app.get("/", (c) => c.text("Hello Hono!"));

// Entry points differ per runtime:
// Cloudflare Workers
export default app;

// Bun
export default { port: 3000, fetch: app.fetch };

// Node.js
import { serve } from "@hono/node-server";
serve(app, { port: 3000 });

// Deno
Deno.serve(app.fetch);

// AWS Lambda
import { handle } from "hono/aws-lambda";
export const handler = handle(app);
```

| Runtime            | Adapter             | Notes                       |
| ------------------ | ------------------- | --------------------------- |
| Cloudflare Workers | Built-in            | Primary target, zero-config |
| Bun                | Built-in            | Native `Bun.serve`          |
| Deno               | Built-in            | `Deno.serve`                |
| Node.js            | `@hono/node-server` | Polyfills Web API           |
| Vercel Edge        | `@hono/vercel`      | Edge Functions              |
| AWS Lambda         | `hono/aws-lambda`   | API Gateway + Lambda        |
| Fastly Compute     | `hono/fastly`       | Compute@Edge                |
| Netlify Edge       | Built-in            | Edge Functions              |

## Routing

Hono uses a trie-based router that matches patterns efficiently:

```typescript
const app = new Hono();

// Basic routes
app.get("/users", listUsers);
app.post("/users", createUser);
app.get("/users/:id", getUser);
app.put("/users/:id", updateUser);
app.delete("/users/:id", deleteUser);
app.all("/mirror", handler); // all methods

// Wildcard
app.get("/files/*", (c) => {
  const path = c.req.path; // full matched path
});

// Optional parameter
app.get("/users/:id{[0-9]+}", (c) => {
  // regex constraint — only matches numeric IDs
  const id = c.req.param("id");
});

// Route groups
const api = new Hono();
api.get("/users", listUsers);
api.post("/users", createUser);
app.route("/api/v1", api);

// Chaining
app.get("/a", handlerA).post("/b", handlerB).delete("/c", handlerC);
```

### Router Options

Hono offers multiple routers with different tradeoffs:

| Router         | Import                       | Characteristics                                       |
| -------------- | ---------------------------- | ----------------------------------------------------- |
| `Hono`         | Default                      | Smart router — auto-selects best                      |
| `RegExpRouter` | `hono/router/reg-exp-router` | Pattern-compiled, fastest for static                  |
| `TrieRouter`   | `hono/router/trie-router`    | Trie-based, good all-around                           |
| `SmartRouter`  | `hono/router/smart-router`   | Picks optimal router at init                          |
| `LinearRouter` | `hono/router/linear-router`  | O(n) scan, startup-optimized (for Lambda cold starts) |

## Context Object

The `c` (Context) object is the central API:

```typescript
app.get("/users/:id", async (c) => {
  // Request data
  const id = c.req.param("id"); // path parameters
  const page = c.req.query("page"); // ?page=1
  const queries = c.req.queries("tags"); // ?tags=a&tags=b → ['a', 'b']
  const body = await c.req.json(); // JSON body
  const formData = await c.req.formData(); // form data
  const text = await c.req.text(); // raw text body
  const blob = await c.req.blob(); // binary
  const header = c.req.header("Authorization"); // request header
  const url = c.req.url; // full URL
  const method = c.req.method; // HTTP method
  const path = c.req.path; // path only
  const matched = c.req.matchedRoutes; // matched route stack

  // Response helpers
  return c.text("Hello", 200); // text/plain
  return c.json({ id, name: "Alice" }); // application/json
  return c.html("<h1>Hello</h1>"); // text/html
  return c.body(stream); // raw body
  return c.redirect("/other"); // 302 redirect
  return c.redirect("/other", 301); // 301 redirect
  return c.notFound(); // 404
  return c.newResponse("body", 200, { "X-Custom": "value" });

  // Headers
  c.header("X-Request-Id", crypto.randomUUID());
  c.header("Cache-Control", "public, max-age=3600");

  // Status
  c.status(201);
  return c.json({ created: true });

  // Variables (like Express locals — request-scoped)
  c.set("user", { id: 1, name: "Alice" });
  const user = c.get("user");

  // Env/bindings (Cloudflare Workers)
  const db = c.env.DB; // D1 binding
  const kv = c.env.KV_STORE; // KV binding
});
```

## Middleware

```typescript
import { Hono } from "hono";
import { logger } from "hono/logger";
import { cors } from "hono/cors";
import { bearerAuth } from "hono/bearer-auth";
import { prettyJSON } from "hono/pretty-json";
import { secureHeaders } from "hono/secure-headers";
import { timing } from "hono/timing";
import { compress } from "hono/compress";
import { etag } from "hono/etag";
import { cache } from "hono/cache";
import { csrf } from "hono/csrf";

const app = new Hono();

// Built-in middleware
app.use("*", logger());
app.use("*", cors({ origin: "https://example.com" }));
app.use("*", secureHeaders());
app.use("*", compress());
app.use("*", etag());
app.use("/api/*", bearerAuth({ token: "secret" }));
app.use("*", prettyJSON());
app.use("*", timing());
app.use("*", csrf());

// Custom middleware
app.use("*", async (c, next) => {
  const start = Date.now();
  await next();
  const duration = Date.now() - start;
  c.header("X-Response-Time", `${duration}ms`);
});

// Route-specific middleware
app.get("/admin/*", adminAuth, adminHandler);
```

### Custom Auth Middleware

```typescript
import { createMiddleware } from "hono/factory";

const authMiddleware = createMiddleware<{
  Variables: { user: { id: string; role: string } };
}>(async (c, next) => {
  const token = c.req.header("Authorization")?.replace("Bearer ", "");
  if (!token) return c.json({ error: "Unauthorized" }, 401);

  const user = await verifyToken(token);
  if (!user) return c.json({ error: "Invalid token" }, 401);

  c.set("user", user);
  await next();
});

app.use("/api/*", authMiddleware);
app.get("/api/me", (c) => {
  const user = c.get("user"); // typed!
  return c.json(user);
});
```

## Validators

### Zod Integration

```typescript
import { zValidator } from '@hono/zod-validator';
import { z } from 'zod';

const createUserSchema = z.object({
  name: z.string().min(1).max(100),
  email: z.string().email(),
  age: z.number().int().positive().optional(),
});

app.post(
  '/users',
  zValidator('json', createUserSchema),
  async (c) => {
    const data = c.req.valid('json');  // typed as z.infer<typeof createUserSchema>
    const user = await db.createUser(data);
    return c.json(user, 201);
  }
);

// Validate query parameters
const querySchema = z.object({
  page: z.coerce.number().default(1),
  limit: z.coerce.number().default(20).max(100),
});

app.get('/users', zValidator('query', querySchema), async (c) => {
  const { page, limit } = c.req.valid('query');
  ...
});

// Validate path parameters
app.get('/users/:id', zValidator('param', z.object({ id: z.string().uuid() })), (c) => {
  ...
});
```

### Valibot Integration

```typescript
import { vValidator } from "@hono/valibot-validator";
import * as v from "valibot";

const UserSchema = v.object({
  name: v.pipe(v.string(), v.minLength(1)),
  email: v.pipe(v.string(), v.email()),
});

app.post("/users", vValidator("json", UserSchema), (c) => {
  const data = c.req.valid("json");
});
```

## RPC Mode

Type-safe client-server communication — like tRPC but simpler:

```typescript
// Server
const routes = app
  .get("/api/users", async (c) => {
    const users = await getUsers();
    return c.json(users);
  })
  .post("/api/users", zValidator("json", createUserSchema), async (c) => {
    const data = c.req.valid("json");
    const user = await createUser(data);
    return c.json(user, 201);
  });

export type AppType = typeof routes;

// Client (type-safe!)
import { hc } from "hono/client";
import type { AppType } from "./server";

const client = hc<AppType>("http://localhost:3000");

const res = await client.api.users.$get();
const users = await res.json(); // fully typed

const res2 = await client.api.users.$post({
  json: { name: "Alice", email: "alice@test.com" }, // typed input
});
```

## JSX / TSX Support

Hono has built-in JSX support for server-rendered HTML:

```tsx
import { Hono } from "hono";
import { html } from "hono/html";

const app = new Hono();

const Layout = (props: { children: any; title: string }) => (
  <html>
    <head>
      <title>{props.title}</title>
    </head>
    <body>{props.children}</body>
  </html>
);

app.get("/", (c) => {
  return c.html(
    <Layout title="Home">
      <h1>Welcome</h1>
      <p>Built with Hono JSX</p>
    </Layout>,
  );
});

// html tagged template (alternative)
app.get("/alt", (c) => {
  const name = "World";
  return c.html(html`<h1>Hello ${name}!</h1>`);
});
```

## Streaming

```typescript
import { streamSSE, streamText } from "hono/streaming";

// Server-Sent Events
app.get("/sse", async (c) => {
  return streamSSE(c, async (stream) => {
    let id = 0;
    while (true) {
      await stream.writeSSE({
        data: JSON.stringify({ time: Date.now() }),
        event: "tick",
        id: String(id++),
      });
      await stream.sleep(1000);
    }
  });
});

// Text streaming
app.get("/stream", (c) => {
  return streamText(c, async (stream) => {
    for (const chunk of ["Hello", " ", "World"]) {
      await stream.write(chunk);
      await stream.sleep(100);
    }
  });
});
```

## WebSocket

```typescript
import { upgradeWebSocket } from "hono/cloudflare-workers"; // or hono/bun, hono/deno

app.get(
  "/ws",
  upgradeWebSocket((c) => ({
    onOpen(event, ws) {
      console.log("Connected");
    },
    onMessage(event, ws) {
      ws.send(`Echo: ${event.data}`);
    },
    onClose() {
      console.log("Disconnected");
    },
    onError(event, ws) {
      console.error("Error:", event);
    },
  })),
);
```

## Testing

```typescript
import { Hono } from "hono";

const app = new Hono();
app.get("/users/:id", (c) => c.json({ id: c.req.param("id") }));

// Test using app.request() — no server needed
describe("GET /users/:id", () => {
  it("returns user", async () => {
    const res = await app.request("/users/1");
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.id).toBe("1");
  });

  it("with headers", async () => {
    const res = await app.request("/users/1", {
      headers: { Authorization: "Bearer token" },
    });
    expect(res.status).toBe(200);
  });

  it("POST with JSON", async () => {
    const res = await app.request("/users", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: "Alice" }),
    });
    expect(res.status).toBe(201);
  });
});
```

## OpenAPI Integration

```typescript
import { OpenAPIHono, createRoute, z } from "@hono/zod-openapi";

const route = createRoute({
  method: "get",
  path: "/users/{id}",
  request: {
    params: z.object({
      id: z.string().openapi({ param: { name: "id", in: "path" } }),
    }),
  },
  responses: {
    200: {
      content: { "application/json": { schema: UserSchema } },
      description: "User found",
    },
    404: { description: "User not found" },
  },
});

const app = new OpenAPIHono();
app.openapi(route, async (c) => {
  const { id } = c.req.valid("param");
  return c.json(user, 200);
});

app.doc("/doc", { openapi: "3.0.0", info: { title: "API", version: "1.0.0" } });
```

## Comparison

| Feature         | Hono                   | Express             | Fastify             |
| --------------- | ---------------------- | ------------------- | ------------------- |
| Size            | ~14KB                  | ~200KB              | ~350KB              |
| Runtime         | Multi (edge-first)     | Node.js only        | Node.js only        |
| Web Standards   | Yes (Request/Response) | req/res objects     | req/reply objects   |
| TypeScript      | First-class            | @types/express      | Built-in            |
| Validation      | Zod/Valibot middleware | Manual              | JSON Schema         |
| RPC             | Built-in hc client     | None                | None                |
| Performance     | Top-tier               | Baseline            | 3-5x Express        |
| Edge deployment | Native                 | Requires adaptation | Requires adaptation |
| JSX             | Built-in               | None                | None                |
