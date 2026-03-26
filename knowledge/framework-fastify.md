# Fastify

## Core Architecture

Fastify is a high-performance Node.js web framework built around a plugin-based architecture. It consistently benchmarks 3-5x faster than Express due to schema-based serialization, efficient routing (find-my-way), and minimal overhead.

### Key Design Principles

- **Schema-first**: JSON Schema drives validation AND serialization (fast-json-stringify)
- **Encapsulation**: plugins create isolated contexts — child plugins can't modify parent state
- **Hooks not middleware**: lifecycle hooks replace Express-style middleware for predictable ordering
- **Logging built-in**: pino logger integrated at the framework level, not bolted on

## Plugin System

Plugins are the fundamental unit of composition. Everything is a plugin — routes, decorators, hooks.

```javascript
// Plugin with options and encapsulation
async function myPlugin(fastify, opts) {
  fastify.decorate("utility", () => "shared");
  fastify.addHook("onRequest", async (req, reply) => {
    /* ... */
  });
  fastify.get("/endpoint", handler);
}

// Register with prefix and options
fastify.register(myPlugin, { prefix: "/api/v1", customOpt: true });
```

### Encapsulation Rules

| Scope                    | Behavior                                                   |
| ------------------------ | ---------------------------------------------------------- |
| Parent context           | Visible to all child plugins                               |
| Child context            | Isolated — siblings and parent cannot see decorators/hooks |
| `fastify-plugin` wrapped | Breaks encapsulation — decorates parent context            |
| `skip-override` metadata | Same as fastify-plugin — hoists to parent                  |

```javascript
const fp = require("fastify-plugin");

// This decorator becomes visible to ALL sibling plugins
module.exports = fp(async function (fastify, opts) {
  fastify.decorate("db", dbConnection);
});
```

### Autoload

`@fastify/autoload` scans directories and registers each file as a plugin:

```javascript
fastify.register(require("@fastify/autoload"), {
  dir: path.join(__dirname, "plugins"), // shared plugins (use fp wrapper)
});
fastify.register(require("@fastify/autoload"), {
  dir: path.join(__dirname, "routes"), // route plugins (encapsulated)
  options: { prefix: "/api" },
});
```

## Hooks Lifecycle

Hooks execute in a strict order. Understanding this sequence is critical.

```
Incoming Request
  └→ onRequest        — auth checks, rate limiting
    └→ preParsing     — modify raw stream before parsing (compression)
      └→ preValidation — modify body before schema validation
        └→ preHandler  — business logic guards
          └→ handler   — route handler executes
            └→ preSerialization — modify payload before JSON.stringify
              └→ onSend        — modify serialized string/buffer
                └→ onResponse  — logging, metrics (after response sent)

onError — called when an error is thrown at any point
onTimeout — called when request exceeds connectionTimeout
onRequestAbort — called when client aborts
```

### Hook Signatures

```javascript
// Request/Reply hooks
fastify.addHook("onRequest", async (request, reply) => {});

// preParsing gets the raw payload stream
fastify.addHook("preParsing", async (request, reply, payload) => {
  return modifiedPayload; // must return stream
});

// preSerialization gets the JS object before serialization
fastify.addHook("preSerialization", async (request, reply, payload) => {
  return { ...payload, timestamp: Date.now() };
});

// onSend gets the serialized string
fastify.addHook("onSend", async (request, reply, payload) => {
  return payload; // string or Buffer
});
```

## Schema-Based Validation & Serialization

### JSON Schema Validation

Fastify compiles JSON schemas with Ajv for input validation:

```javascript
const opts = {
  schema: {
    params: {
      type: "object",
      properties: { id: { type: "integer" } },
      required: ["id"],
    },
    querystring: {
      type: "object",
      properties: { limit: { type: "integer", default: 20, maximum: 100 } },
    },
    body: {
      type: "object",
      properties: {
        name: { type: "string", minLength: 1 },
        email: { type: "string", format: "email" },
      },
      required: ["name", "email"],
    },
    response: {
      200: {
        type: "object",
        properties: {
          id: { type: "integer" },
          name: { type: "string" },
        },
      },
    },
  },
};

fastify.post("/users/:id", opts, async (request, reply) => {
  // request.body is already validated
  // response is serialized using fast-json-stringify (2-5x faster than JSON.stringify)
  return { id: request.params.id, name: request.body.name };
});
```

### TypeBox Integration

TypeBox provides TypeScript-inferred schemas:

```typescript
import { Type, Static } from "@sinclair/typebox";

const UserSchema = Type.Object({
  id: Type.Integer(),
  name: Type.String({ minLength: 1 }),
  email: Type.String({ format: "email" }),
  role: Type.Union([Type.Literal("admin"), Type.Literal("user")]),
});

type User = Static<typeof UserSchema>;

fastify.post<{ Body: User }>(
  "/users",
  {
    schema: { body: UserSchema },
  },
  async (request) => {
    const user: User = request.body; // fully typed
  },
);
```

### Fluent Schema

```javascript
const S = require("fluent-json-schema");

const bodySchema = S.object()
  .prop("name", S.string().required())
  .prop("age", S.integer().minimum(0))
  .prop("tags", S.array().items(S.string()));
```

## Decorators

Decorators extend Fastify, Request, or Reply with custom properties:

```javascript
// Server-level: available everywhere
fastify.decorate("config", loadConfig());
fastify.decorate("authenticate", async (request) => {
  /* ... */
});

// Request-level: fresh per request, use null + setter pattern
fastify.decorateRequest("user", null);

// Reply-level
fastify.decorateReply("sendSuccess", function (data) {
  this.code(200).send({ success: true, data });
});
```

**Critical**: decorators are set once at startup. For per-request state, decorate with `null` and set in hooks.

## Serialization

Response schemas do double duty — they validate what you send AND strip extra fields (security), PLUS use `fast-json-stringify` instead of `JSON.stringify`:

```javascript
// Only id and name are sent — email is stripped
response: {
  200: {
    type: 'object',
    properties: {
      id: { type: 'integer' },
      name: { type: 'string' },
    },
  },
}
```

No response schema = falls back to `JSON.stringify` (slower, no field stripping).

## Content Type Parsing

```javascript
// Custom content type parser
fastify.addContentTypeParser(
  "application/yaml",
  { parseAs: "string" },
  (req, body, done) => {
    try {
      done(null, yaml.parse(body));
    } catch (err) {
      done(err);
    }
  },
);

// Catch-all parser
fastify.addContentTypeParser("*", (req, body, done) => {
  let data = "";
  body.on("data", (chunk) => {
    data += chunk;
  });
  body.on("end", () => done(null, data));
});
```

## Logging (Pino)

Fastify uses pino — the fastest Node.js logger (~5x faster than winston):

```javascript
const fastify = require("fastify")({
  logger: {
    level: "info",
    transport: {
      target: "pino-pretty", // dev only — structured JSON in production
    },
  },
});

// Request-scoped logging (auto-includes reqId)
fastify.get("/", async (request) => {
  request.log.info({ userId: 123 }, "processing request");
});
```

## Testing

Fastify has a built-in `inject` method — no HTTP server needed:

```javascript
const build = require("./app");

test("GET /users returns 200", async () => {
  const app = build();
  const response = await app.inject({
    method: "GET",
    url: "/users",
    headers: { authorization: "Bearer token" },
  });
  expect(response.statusCode).toBe(200);
  expect(response.json()).toEqual([{ id: 1, name: "Alice" }]);
});
```

## Common Plugins

| Plugin                    | Purpose                           |
| ------------------------- | --------------------------------- |
| `@fastify/cors`           | CORS headers                      |
| `@fastify/rate-limit`     | Per-route or global rate limiting |
| `@fastify/websocket`      | WebSocket support via ws          |
| `@fastify/static`         | Static file serving               |
| `@fastify/jwt`            | JWT auth                          |
| `@fastify/cookie`         | Cookie parsing                    |
| `@fastify/session`        | Server-side sessions              |
| `@fastify/swagger`        | OpenAPI spec generation           |
| `@fastify/swagger-ui`     | Swagger UI                        |
| `@fastify/multipart`      | File uploads                      |
| `@fastify/helmet`         | Security headers                  |
| `@fastify/under-pressure` | Health checks + load shedding     |

## TypeScript

```typescript
import Fastify, { FastifyRequest, FastifyReply } from "fastify";

const server = Fastify({ logger: true });

// Typed route with generics
interface IQuerystring {
  limit: number;
  offset: number;
}
interface IParams {
  id: string;
}

server.get<{ Querystring: IQuerystring; Params: IParams }>(
  "/items/:id",
  async (request, reply) => {
    const { limit, offset } = request.query; // typed
    const { id } = request.params; // typed
  },
);
```

## Express Comparison

| Aspect                     | Express                    | Fastify               |
| -------------------------- | -------------------------- | --------------------- |
| Requests/sec (hello world) | ~15k                       | ~75k                  |
| JSON serialization         | JSON.stringify             | fast-json-stringify   |
| Validation                 | Manual / express-validator | Built-in JSON Schema  |
| Logging                    | None (add morgan/winston)  | Pino built-in         |
| Plugin isolation           | None (global middleware)   | Encapsulated contexts |
| Async error handling       | Wrap in try/catch          | Native async support  |
| TypeScript                 | @types/express             | First-class support   |

## WebSocket

```javascript
fastify.register(require("@fastify/websocket"));

fastify.get("/ws", { websocket: true }, (socket, req) => {
  socket.on("message", (msg) => {
    socket.send(`echo: ${msg}`);
  });
});
```

## Rate Limiting

```javascript
fastify.register(require("@fastify/rate-limit"), {
  max: 100,
  timeWindow: "1 minute",
  keyGenerator: (req) => req.ip,
  errorResponseBuilder: (req, context) => ({
    statusCode: 429,
    error: "Too Many Requests",
    message: `Rate limit exceeded, retry in ${context.after}`,
  }),
});

// Per-route override
fastify.get("/expensive", { config: { rateLimit: { max: 10 } } }, handler);
```

## Graceful Shutdown

```javascript
const closeListeners = fastify.close.bind(fastify);
["SIGINT", "SIGTERM"].forEach((signal) => {
  process.on(signal, async () => {
    await fastify.close(); // drains connections, runs onClose hooks
    process.exit(0);
  });
});
```
