# Express.js

## Middleware Pipeline

Express is fundamentally a middleware pipeline. Each request flows through a stack of functions in order. Each middleware receives `(req, res, next)` and either responds or calls `next()`.

```js
const express = require("express");
const app = express();

// Middleware execution order matters
app.use(express.json()); // 1. Parse JSON bodies
app.use(express.urlencoded({ extended: true })); // 2. Parse form data
app.use(cors()); // 3. CORS headers
app.use(helmet()); // 4. Security headers
app.use(morgan("combined")); // 5. Request logging
app.use("/api", rateLimiter); // 6. Rate limiting (path-scoped)

app.get("/api/users", getUsers); // 7. Route handler
app.use(errorHandler); // 8. Error handler (MUST be last)
```

### Middleware Types

| Type           | Signature                                                    | Registration                          |
| -------------- | ------------------------------------------------------------ | ------------------------------------- |
| Application    | `(req, res, next)`                                           | `app.use(fn)`                         |
| Router         | `(req, res, next)`                                           | `router.use(fn)`                      |
| Error-handling | `(err, req, res, next)`                                      | `app.use(fn)` — **4 params required** |
| Built-in       | `express.json()`, `express.static()`, `express.urlencoded()` | `app.use(fn)`                         |
| Third-party    | `cors()`, `helmet()`, etc.                                   | `app.use(fn)`                         |

### Error Handling

```js
// Async errors must be caught and passed to next()
app.get("/api/users/:id", async (req, res, next) => {
  try {
    const user = await User.findById(req.params.id);
    if (!user) return res.status(404).json({ error: "Not found" });
    res.json(user);
  } catch (err) {
    next(err); // passes to error middleware
  }
});

// Express 5 auto-catches async errors. For Express 4, use a wrapper:
const asyncHandler = (fn) => (req, res, next) =>
  Promise.resolve(fn(req, res, next)).catch(next);

app.get(
  "/api/users",
  asyncHandler(async (req, res) => {
    const users = await User.find();
    res.json(users);
  }),
);

// Error middleware (4 params — Express identifies it by arity)
app.use((err, req, res, next) => {
  console.error(err.stack);
  const status = err.status || 500;
  res.status(status).json({
    error:
      process.env.NODE_ENV === "production" ? "Internal error" : err.message,
  });
});
```

## Routing

### Basic Routes

```js
app.get("/users", listUsers);
app.post("/users", createUser);
app.get("/users/:id", getUser); // req.params.id
app.put("/users/:id", updateUser);
app.patch("/users/:id", patchUser);
app.delete("/users/:id", deleteUser);

// Multiple handlers (middleware chain)
app.get("/admin", authenticate, authorize("admin"), adminDashboard);

// Pattern matching
app.get("/files/*", serveFile); // wildcard
app.get("/user/:id(\\d+)", getUser); // regex constraint
```

### Router (modular routes)

```js
// routes/users.js
const router = express.Router();

router.param("id", async (req, res, next, id) => {
  req.user = await User.findById(id);
  if (!req.user) return res.status(404).json({ error: "User not found" });
  next();
});

router.get("/", async (req, res) => {
  const { page = 1, limit = 20, sort = "-createdAt" } = req.query;
  const users = await User.find()
    .sort(sort)
    .skip((page - 1) * limit)
    .limit(Number(limit));
  res.json(users);
});

router.get("/:id", (req, res) => res.json(req.user));
router.put("/:id", validateBody(userSchema), async (req, res) => {
  Object.assign(req.user, req.body);
  await req.user.save();
  res.json(req.user);
});

module.exports = router;

// app.js
app.use("/api/users", require("./routes/users"));
```

## Request Object

```js
req.params; // Route parameters: /users/:id → { id: '123' }
req.query; // Query string: ?page=2&sort=name → { page: '2', sort: 'name' }
req.body; // Parsed body (requires body-parser middleware)
req.headers; // Request headers (lowercase keys)
req.cookies; // Parsed cookies (requires cookie-parser)
req.ip; // Client IP (respects trust proxy)
req.method; // HTTP method
req.path; // URL path (without query string)
req.hostname; // Host from Host header
req.protocol; // 'http' or 'https'
req.secure; // true if HTTPS
req.xhr; // true if X-Requested-With: XMLHttpRequest
req.get("header"); // Get specific header value
```

## Response Object

```js
res.status(201).json({ id: 1, name: "Alice" });
res.send("Hello"); // auto content-type
res.sendFile(path.join(__dirname, "file.pdf"));
res.download("/path/to/file.pdf", "report.pdf");
res.redirect(301, "/new-url");
res.set("X-Custom", "value"); // set header
res.cookie("token", jwt, { httpOnly: true, secure: true, sameSite: "strict" });
res.clearCookie("token");
res.type("json"); // set Content-Type
res.links({ next: "/page/2" }); // Link header
res.format({
  // Content negotiation
  "text/plain": () => res.send("text"),
  "application/json": () => res.json({ text: "json" }),
});
```

## Static Files

```js
app.use(express.static("public")); // serves /public/*
app.use("/assets", express.static("public")); // serves at /assets/*
app.use(
  express.static("public", {
    maxAge: "1d", // Cache-Control
    etag: true, // ETag
    index: "index.html", // Default file
    dotfiles: "deny", // Reject .hidden files
  }),
);
```

## Security Middleware Stack

```js
const helmet = require("helmet");
const cors = require("cors");
const rateLimit = require("express-rate-limit");

// Helmet sets security headers: CSP, HSTS, X-Frame-Options, etc.
app.use(helmet());

// CORS — be specific in production
app.use(
  cors({
    origin: ["https://myapp.com"],
    methods: ["GET", "POST", "PUT", "DELETE"],
    credentials: true,
    maxAge: 86400,
  }),
);

// Rate limiting
const limiter = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 100, // requests per window
  standardHeaders: true,
  legacyHeaders: false,
  message: { error: "Too many requests" },
});
app.use("/api/", limiter);

// Stricter limit for auth endpoints
const authLimiter = rateLimit({ windowMs: 60 * 1000, max: 5 });
app.use("/api/auth/login", authLimiter);
```

## Sessions and Cookies

```js
const session = require("express-session");
const RedisStore = require("connect-redis").default;

app.use(
  session({
    store: new RedisStore({ client: redisClient }),
    secret: process.env.SESSION_SECRET,
    resave: false,
    saveUninitialized: false,
    cookie: {
      secure: true, // HTTPS only
      httpOnly: true, // no JS access
      sameSite: "strict",
      maxAge: 24 * 60 * 60 * 1000, // 24 hours
    },
  }),
);
```

## File Uploads

```js
const multer = require("multer");

const upload = multer({
  storage: multer.diskStorage({
    destination: "uploads/",
    filename: (req, file, cb) => {
      const unique = `${Date.now()}-${Math.round(Math.random() * 1e9)}`;
      cb(null, `${unique}-${file.originalname}`);
    },
  }),
  limits: { fileSize: 5 * 1024 * 1024 }, // 5MB
  fileFilter: (req, file, cb) => {
    const allowed = ["image/jpeg", "image/png", "image/webp"];
    cb(null, allowed.includes(file.mimetype));
  },
});

app.post("/upload", upload.single("avatar"), (req, res) => {
  res.json({ filename: req.file.filename, size: req.file.size });
});

app.post("/gallery", upload.array("photos", 10), (req, res) => {
  res.json({ count: req.files.length });
});
```

## Graceful Shutdown

```js
const server = app.listen(3000);

function shutdown(signal) {
  console.log(`${signal} received. Closing server...`);
  server.close(() => {
    // Close database connections, Redis, etc.
    mongoose.connection.close(false, () => {
      process.exit(0);
    });
  });
  // Force shutdown after timeout
  setTimeout(() => process.exit(1), 10000);
}

process.on("SIGTERM", () => shutdown("SIGTERM"));
process.on("SIGINT", () => shutdown("SIGINT"));
```

## Production Checklist

- **Security**: helmet, CORS, rate limiting, input validation, parameterized queries, CSRF tokens
- **Performance**: compression middleware, response caching, connection pooling, clustering
- **Reliability**: graceful shutdown, health check endpoint, structured logging (pino/winston)
- **Ops**: `NODE_ENV=production`, `trust proxy` if behind reverse proxy, PM2 or systemd for process management

## Clustering

```js
const cluster = require("cluster");
const os = require("os");

if (cluster.isPrimary) {
  const numCPUs = os.cpus().length;
  for (let i = 0; i < numCPUs; i++) cluster.fork();
  cluster.on("exit", (worker) => {
    console.log(`Worker ${worker.process.pid} died, restarting...`);
    cluster.fork();
  });
} else {
  const app = require("./app");
  app.listen(3000);
}
```

## Comparison to Alternatives

| Feature     | Express                 | Fastify           | Koa                 | Hono                |
| ----------- | ----------------------- | ----------------- | ------------------- | ------------------- |
| Performance | Baseline                | ~2x Express       | ~1.5x               | ~3x+                |
| Middleware  | `(req, res, next)`      | Hooks + plugins   | `async (ctx, next)` | `(c, next)`         |
| Validation  | Manual/third-party      | Built-in (Ajv)    | Manual              | Built-in (Zod)      |
| TypeScript  | Bolted on               | First-class       | Bolted on           | First-class         |
| Ecosystem   | Largest                 | Growing           | Small               | Growing             |
| Runtime     | Node.js                 | Node.js           | Node.js             | Node/Deno/Bun/Edge  |
| Best for    | Legacy, large ecosystem | APIs, performance | Minimalism          | Edge, multi-runtime |

Express 5 (in beta for years) adds: async error handling, `req.query` getter, removed deprecated APIs, and requires Node 18+.
