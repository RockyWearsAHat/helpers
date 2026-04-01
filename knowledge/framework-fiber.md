# Fiber (Go Web Framework)

## Overview

Fiber is a Go web framework with Express-inspired API built on top of fasthttp instead of net/http. It's designed for speed — fasthttp can be 10x faster than net/http for certain workloads due to zero-allocation designs and connection pooling. The tradeoff: incompatibility with net/http middleware and the broader Go HTTP ecosystem.

## Core Concepts

```go
package main

import (
    "log"
    "github.com/gofiber/fiber/v2"
)

func main() {
    app := fiber.New(fiber.Config{
        Prefork:       false,               // multi-process mode
        ServerHeader:  "Fiber",
        StrictRouting: false,               // /foo and /foo/ are the same
        CaseSensitive: false,               // /Foo and /foo are the same
        BodyLimit:     4 * 1024 * 1024,     // 4MB
        ReadTimeout:   10 * time.Second,
        WriteTimeout:  10 * time.Second,
        IdleTimeout:   120 * time.Second,
        ErrorHandler:  customErrorHandler,
    })

    app.Get("/", func(c *fiber.Ctx) error {
        return c.SendString("Hello, World!")
    })

    log.Fatal(app.Listen(":3000"))
}
```

## Routing

```go
// HTTP method routes
app.Get("/users", listUsers)
app.Post("/users", createUser)
app.Put("/users/:id", updateUser)
app.Patch("/users/:id", patchUser)
app.Delete("/users/:id", deleteUser)
app.All("/mirror", handler)        // all methods

// Parameters
app.Get("/users/:id", func(c *fiber.Ctx) error {
    id := c.Params("id")               // string
    intID, _ := c.ParamsInt("id")       // int with built-in conversion
    return c.JSON(fiber.Map{"id": intID})
})

// Optional parameters
app.Get("/users/:name?", handler)

// Wildcards
app.Get("/files/*", func(c *fiber.Ctx) error {
    path := c.Params("*")  // everything after /files/
    return c.SendString(path)
})

// Route constraints
app.Get("/users/:id<int>", handler)        // numeric only
app.Get("/date/:date<datetime>", handler)  // date format
app.Get("/name/:name<alpha>", handler)     // alpha only
app.Get("/uuid/:id<guid>", handler)        // UUID format
app.Get("/file/:name<minLen(3)>", handler) // min length

// Route groups
api := app.Group("/api", apiMiddleware)
v1 := api.Group("/v1")
v1.Get("/users", listUsers)
v1.Post("/users", createUser)

// Mount sub-apps
micro := fiber.New()
micro.Get("/health", healthCheck)
app.Mount("/service", micro)
```

## Context

Fiber's `Ctx` provides Express-like API:

```go
func handler(c *fiber.Ctx) error {
    // Request data
    c.Method()                         // "GET"
    c.Path()                           // "/users/1"
    c.OriginalURL()                    // full URL with query
    c.Protocol()                       // "http" or "https"
    c.IP()                             // client IP
    c.IPs()                            // IP chain from X-Forwarded-For
    c.Hostname()                       // hostname
    c.BaseURL()                        // scheme + host

    // Parameters
    c.Params("id")                     // route params
    c.Query("page", "1")              // query params with default
    c.QueryInt("limit", 20)           // query param as int
    c.Queries()                        // all query params as map

    // Headers
    c.Get("Authorization")            // request header
    c.Get("Content-Type")

    // Body
    c.Body()                          // raw body bytes
    c.BodyParser(&struct{})           // JSON/XML/Form into struct

    // Cookies
    c.Cookies("session_id")           // read cookie
    c.Cookie(&fiber.Cookie{           // set cookie
        Name:     "session_id",
        Value:    "abc123",
        HTTPOnly: true,
        Secure:   true,
        SameSite: "Lax",
        MaxAge:   86400,
    })

    // Response
    c.Status(201)                      // set status (chainable)
    c.SendString("text")              // text/plain
    c.JSON(data)                       // JSON
    c.XML(data)                        // XML
    c.SendFile("./file.pdf")          // file download
    c.Download("./file.pdf", "report.pdf")  // with filename
    c.Redirect("/other", 302)          // redirect
    c.Set("X-Custom", "value")        // response header
    c.Type("json")                    // set Content-Type shorthand
    c.Attachment("report.pdf")        // Content-Disposition

    // Locals (request-scoped variables)
    c.Locals("user", user)
    user := c.Locals("user").(User)

    // Next middleware
    return c.Next()
}
```

## Body Parsing

```go
type CreateUser struct {
    Name  string `json:"name" xml:"name" form:"name" validate:"required,min=2"`
    Email string `json:"email" xml:"email" form:"email" validate:"required,email"`
    Age   int    `json:"age" xml:"age" form:"age" validate:"gte=0,lte=150"`
}

func createUser(c *fiber.Ctx) error {
    var input CreateUser
    if err := c.BodyParser(&input); err != nil {
        return c.Status(400).JSON(fiber.Map{"error": err.Error()})
    }

    // Validate with go-playground/validator
    if err := validate.Struct(input); err != nil {
        return c.Status(422).JSON(fiber.Map{"errors": formatErrors(err)})
    }

    // Query params into struct
    type Filters struct {
        Page  int    `query:"page"`
        Limit int    `query:"limit"`
        Sort  string `query:"sort"`
    }
    var filters Filters
    c.QueryParser(&filters)

    return c.Status(201).JSON(user)
}
```

## Middleware

### Built-in Middleware

```go
import (
    "github.com/gofiber/fiber/v2/middleware/logger"
    "github.com/gofiber/fiber/v2/middleware/recover"
    "github.com/gofiber/fiber/v2/middleware/cors"
    "github.com/gofiber/fiber/v2/middleware/limiter"
    "github.com/gofiber/fiber/v2/middleware/csrf"
    "github.com/gofiber/fiber/v2/middleware/compress"
    "github.com/gofiber/fiber/v2/middleware/cache"
    "github.com/gofiber/fiber/v2/middleware/timeout"
    "github.com/gofiber/fiber/v2/middleware/monitor"
    "github.com/gofiber/fiber/v2/middleware/requestid"
)

app.Use(recover.New())
app.Use(logger.New(logger.Config{
    Format: "${time} ${status} ${method} ${path} ${latency}\n",
}))
app.Use(cors.New(cors.Config{
    AllowOrigins: "https://example.com",
    AllowMethods: "GET,POST,PUT,DELETE",
}))
app.Use(compress.New())
app.Use(requestid.New())
```

| Middleware   | Purpose                       |
| ------------ | ----------------------------- |
| `logger`     | Request logging               |
| `recover`    | Panic recovery                |
| `cors`       | CORS handling                 |
| `limiter`    | Rate limiting                 |
| `csrf`       | CSRF protection               |
| `compress`   | Gzip/Brotli/Deflate           |
| `cache`      | Response caching              |
| `timeout`    | Request timeout               |
| `monitor`    | Metrics dashboard at /metrics |
| `requestid`  | Request ID header             |
| `helmet`     | Security headers              |
| `session`    | Session management            |
| `filesystem` | Virtual filesystem            |
| `basicauth`  | Basic authentication          |
| `keyauth`    | API key authentication        |

### Rate Limiter

```go
app.Use(limiter.New(limiter.Config{
    Max:        100,
    Expiration: 1 * time.Minute,
    KeyGenerator: func(c *fiber.Ctx) string {
        return c.IP()
    },
    LimitReached: func(c *fiber.Ctx) error {
        return c.Status(429).JSON(fiber.Map{
            "error": "too many requests",
        })
    },
}))
```

### Custom Middleware

```go
func authMiddleware(c *fiber.Ctx) error {
    token := c.Get("Authorization")
    if token == "" {
        return c.Status(401).JSON(fiber.Map{"error": "unauthorized"})
    }

    user, err := validateToken(strings.TrimPrefix(token, "Bearer "))
    if err != nil {
        return c.Status(401).JSON(fiber.Map{"error": "invalid token"})
    }

    c.Locals("user", user)
    return c.Next()
}

// Apply
api := app.Group("/api", authMiddleware)
```

## Static Files

```go
app.Static("/", "./public")
app.Static("/assets", "./static", fiber.Static{
    Compress:  true,
    ByteRange: true,
    Browse:    false,
    Index:     "index.html",
    MaxAge:    3600,
})
```

## Template Engines

```go
import "github.com/gofiber/template/html/v2"

engine := html.New("./views", ".html")
app := fiber.New(fiber.Config{Views: engine})

app.Get("/", func(c *fiber.Ctx) error {
    return c.Render("index", fiber.Map{
        "Title": "Hello",
        "Users": users,
    }, "layouts/main")
})
```

Supported engines: HTML, Handlebars, Mustache, Pug, Django, Jet, Amber.

## WebSocket

```go
import "github.com/gofiber/websocket/v2"

app.Use("/ws", func(c *fiber.Ctx) error {
    if websocket.IsWebSocketUpgrade(c) {
        return c.Next()
    }
    return fiber.ErrUpgradeRequired
})

app.Get("/ws/:room", websocket.New(func(c *websocket.Conn) {
    room := c.Params("room")
    for {
        mt, msg, err := c.ReadMessage()
        if err != nil {
            break
        }
        if err := c.WriteMessage(mt, msg); err != nil {
            break
        }
    }
}))
```

## File Upload

```go
func uploadHandler(c *fiber.Ctx) error {
    // Single file
    file, err := c.FormFile("document")
    if err != nil {
        return c.Status(400).JSON(fiber.Map{"error": err.Error()})
    }
    c.SaveFile(file, fmt.Sprintf("./uploads/%s", file.Filename))

    // Multiple files
    form, err := c.MultipartForm()
    if err != nil {
        return err
    }
    files := form.File["documents"]
    for _, file := range files {
        c.SaveFile(file, fmt.Sprintf("./uploads/%s", file.Filename))
    }

    return c.JSON(fiber.Map{"uploaded": len(files)})
}
```

## Error Handling

```go
// Custom error handler
app := fiber.New(fiber.Config{
    ErrorHandler: func(c *fiber.Ctx, err error) error {
        code := fiber.StatusInternalServerError
        var e *fiber.Error
        if errors.As(err, &e) {
            code = e.Code
        }
        return c.Status(code).JSON(fiber.Map{
            "error":   true,
            "message": err.Error(),
        })
    },
})

// In handlers
func getUser(c *fiber.Ctx) error {
    user, err := findUser(c.Params("id"))
    if err != nil {
        return fiber.NewError(404, "User not found")
    }
    return c.JSON(user)
}
```

## Testing

```go
func TestGetUser(t *testing.T) {
    app := setupApp()

    req := httptest.NewRequest("GET", "/users/1", nil)
    req.Header.Set("Authorization", "Bearer test-token")

    resp, err := app.Test(req, -1)  // -1 = no timeout
    assert.NoError(t, err)
    assert.Equal(t, 200, resp.StatusCode)

    var body map[string]interface{}
    json.NewDecoder(resp.Body).Decode(&body)
    assert.Equal(t, "Alice", body["name"])
}

func TestCreateUser(t *testing.T) {
    app := setupApp()

    body := `{"name":"Bob","email":"bob@test.com"}`
    req := httptest.NewRequest("POST", "/users",
        strings.NewReader(body))
    req.Header.Set("Content-Type", "application/json")

    resp, err := app.Test(req)
    assert.NoError(t, err)
    assert.Equal(t, 201, resp.StatusCode)
}
```

## Graceful Shutdown

```go
go func() {
    if err := app.Listen(":3000"); err != nil {
        log.Panic(err)
    }
}()

quit := make(chan os.Signal, 1)
signal.Notify(quit, os.Interrupt, syscall.SIGTERM)
<-quit

if err := app.ShutdownWithTimeout(5 * time.Second); err != nil {
    log.Fatal(err)
}
```

## Prefork

Fiber can spawn multiple processes (like Node.js cluster):

```go
app := fiber.New(fiber.Config{
    Prefork: true,  // spawn process per CPU core
})
```

Prefork uses `SO_REUSEPORT` on Linux — each child process gets its own accept loop. Great for throughput, but shared state (like in-memory stores) won't work across prefork processes.

## Database Integration

```go
// GORM
import "gorm.io/gorm"

func setupDB() *gorm.DB {
    db, _ := gorm.Open(postgres.Open(dsn), &gorm.Config{})
    db.AutoMigrate(&User{})
    return db
}

app.Get("/users", func(c *fiber.Ctx) error {
    db := c.Locals("db").(*gorm.DB)
    var users []User
    db.Find(&users)
    return c.JSON(users)
})

// sqlx
import "github.com/jmoiron/sqlx"

func listUsers(c *fiber.Ctx) error {
    db := c.Locals("db").(*sqlx.DB)
    var users []User
    db.Select(&users, "SELECT * FROM users ORDER BY id")
    return c.JSON(users)
}
```

## Swagger

```go
import swagger "github.com/arsmn/fiber-swagger/v2"

app.Get("/swagger/*", swagger.HandlerDefault)
```

## Comparison: Fiber vs Gin vs Echo

| Feature          | Fiber                  | Gin       | Echo      |
| ---------------- | ---------------------- | --------- | --------- |
| HTTP library     | fasthttp               | net/http  | net/http  |
| API style        | Express-like           | Unique    | Unique    |
| Performance      | Fastest (10x net/http) | Very fast | Very fast |
| net/http compat  | **No**                 | Yes       | Yes       |
| io.Reader/Writer | Limited                | Full      | Full      |
| Zero allocation  | Router                 | No        | No        |
| Prefork          | Built-in               | Manual    | Manual    |
| Go stdlib compat | Limited                | Full      | Full      |

### The fasthttp Tradeoff

**Pros**: Dramatically fewer allocations, connection reuse, faster parsing, zero-alloc router.

**Cons**: Doesn't implement `net/http.Handler` or `http.ResponseWriter`. Much of the Go ecosystem (middleware libraries, HTTP utilities, cloud SDKs) assumes `net/http` interfaces. Fiber wraps these differences but you may hit incompatibilities with libraries that expect `net/http` types. Context values are pooled and recycled — you **must not** hold references to `*fiber.Ctx` after the handler returns.

```go
// WRONG — ctx is recycled after handler returns
app.Get("/", func(c *fiber.Ctx) error {
    go func() {
        time.Sleep(time.Second)
        fmt.Println(c.Path())  // DANGER: c may be reused
    }()
    return c.SendString("ok")
})

// CORRECT — copy what you need
app.Get("/", func(c *fiber.Ctx) error {
    path := c.Path()  // copy before goroutine
    go func() {
        time.Sleep(time.Second)
        fmt.Println(path)
    }()
    return c.SendString("ok")
})
```
