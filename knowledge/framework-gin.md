# Gin (Go Web Framework)

## Overview

Gin is the most popular Go web framework. Built on httprouter, it provides ~40x the throughput of net/http's DefaultServeMux for parameterized routes due to its radix tree router. It's minimal — routing + middleware + binding + rendering — and doesn't prescribe architecture.

## Router

Gin's router is a radix tree (compressed trie). No regex — path parameters and wildcards only:

```go
r := gin.Default()  // includes Logger + Recovery middleware

r.GET("/users/:id", getUser)        // named parameter
r.GET("/files/*filepath", getFile)  // wildcard — catches rest of path
r.POST("/users", createUser)
r.PUT("/users/:id", updateUser)
r.DELETE("/users/:id", deleteUser)
r.Any("/mirror", handleAny)         // all HTTP methods
```

### Route Groups

```go
api := r.Group("/api/v1")
api.Use(authMiddleware())
{
    api.GET("/users", listUsers)
    api.POST("/users", createUser)

    admin := api.Group("/admin")
    admin.Use(adminOnly())
    {
        admin.DELETE("/users/:id", deleteUser)
    }
}
```

## Context

`gin.Context` is the heart of every request handler — carries request data, response writer, metadata, and abort/error control:

```go
func getUser(c *gin.Context) {
    // Path parameters
    id := c.Param("id")

    // Query parameters
    page := c.DefaultQuery("page", "1")
    limit := c.Query("limit")  // "" if missing

    // Headers
    token := c.GetHeader("Authorization")

    // Set metadata (available to later middleware/handlers)
    c.Set("userID", 42)
    val, exists := c.Get("userID")

    // Response
    c.JSON(200, gin.H{"user": user})
}
```

### Context Methods for Response

| Method                            | Content-Type           |
| --------------------------------- | ---------------------- |
| `c.JSON(code, obj)`               | application/json       |
| `c.XML(code, obj)`                | application/xml        |
| `c.YAML(code, obj)`               | application/x-yaml     |
| `c.ProtoBuf(code, obj)`           | application/x-protobuf |
| `c.String(code, fmt, args...)`    | text/plain             |
| `c.HTML(code, name, obj)`         | text/html              |
| `c.Data(code, contentType, data)` | custom                 |
| `c.File(filepath)`                | auto-detected          |
| `c.Redirect(code, url)`           | redirect               |
| `c.Stream(func)`                  | streaming              |

## Parameter Binding

Gin maps request data to structs using struct tags:

```go
type CreateUserInput struct {
    Name     string `json:"name" binding:"required,min=2,max=100"`
    Email    string `json:"email" binding:"required,email"`
    Age      int    `json:"age" binding:"gte=0,lte=150"`
    Role     string `json:"role" binding:"oneof=admin user moderator"`
    Password string `json:"password" binding:"required,min=8"`
}

func createUser(c *gin.Context) {
    var input CreateUserInput
    if err := c.ShouldBindJSON(&input); err != nil {
        c.JSON(400, gin.H{"error": err.Error()})
        return
    }
    // input is validated
}
```

### Binding Sources

| Method             | Source             | Content-Type     |
| ------------------ | ------------------ | ---------------- |
| `ShouldBindJSON`   | Body               | application/json |
| `ShouldBindXML`    | Body               | application/xml  |
| `ShouldBind`       | Body (auto-detect) | any              |
| `ShouldBindQuery`  | Query string       | N/A              |
| `ShouldBindUri`    | URL parameters     | N/A              |
| `ShouldBindHeader` | Headers            | N/A              |

```go
// Bind URI parameters
type UserURI struct {
    ID int `uri:"id" binding:"required,gt=0"`
}

func getUser(c *gin.Context) {
    var uri UserURI
    if err := c.ShouldBindUri(&uri); err != nil {
        c.JSON(400, gin.H{"error": err.Error()})
        return
    }
}
```

`ShouldBind*` returns errors. `Bind*` (without Should) also sets 400 status — prefer `ShouldBind*` for control.

## Validation

Gin uses `go-playground/validator`. Common tags:

| Tag              | Meaning                        |
| ---------------- | ------------------------------ |
| `required`       | Must be present and non-zero   |
| `email`          | Valid email format             |
| `min=N`, `max=N` | String length or numeric value |
| `gte=N`, `lte=N` | Greater/less than or equal     |
| `oneof=a b c`    | Must be one of the values      |
| `uuid`           | Valid UUID                     |
| `url`            | Valid URL                      |
| `ip`             | Valid IP address               |
| `alphanum`       | Alphanumeric only              |
| `len=N`          | Exact length                   |

### Custom Validators

```go
import "github.com/go-playground/validator/v10"

func strongPassword(fl validator.FieldLevel) bool {
    pw := fl.Field().String()
    return len(pw) >= 8 && hasUppercase(pw) && hasDigit(pw)
}

// Register at startup
if v, ok := binding.Validator.Engine().(*validator.Validate); ok {
    v.RegisterValidation("strongpw", strongPassword)
}

// Use in struct tag
type Input struct {
    Password string `json:"password" binding:"required,strongpw"`
}
```

## Middleware

Middleware are just handlers that call `c.Next()`:

```go
func requestTimer() gin.HandlerFunc {
    return func(c *gin.Context) {
        start := time.Now()
        c.Next()  // process request
        duration := time.Since(start)
        c.Writer.Header().Set("X-Response-Time", duration.String())
        log.Printf("%s %s %d %v", c.Request.Method, c.Request.URL.Path,
            c.Writer.Status(), duration)
    }
}

func authMiddleware() gin.HandlerFunc {
    return func(c *gin.Context) {
        token := c.GetHeader("Authorization")
        if token == "" {
            c.AbortWithStatusJSON(401, gin.H{"error": "unauthorized"})
            return  // c.Abort stops the chain
        }
        userID, err := validateToken(token)
        if err != nil {
            c.AbortWithStatusJSON(401, gin.H{"error": "invalid token"})
            return
        }
        c.Set("userID", userID)
        c.Next()
    }
}

r.Use(requestTimer())  // global middleware
```

### Built-in Middleware

- `gin.Logger()` — request logging
- `gin.Recovery()` — panic recovery (returns 500)
- `gin.Default()` = `gin.New()` + Logger + Recovery

### Abort vs Next

- `c.Next()` — calls next handler in chain, returns after all downstream handlers complete
- `c.Abort()` — stops calling remaining handlers (but current handler continues executing)
- `c.AbortWithStatusJSON(code, obj)` — abort + set response

## File Upload

```go
// Single file
func uploadFile(c *gin.Context) {
    file, err := c.FormFile("file")
    if err != nil {
        c.JSON(400, gin.H{"error": err.Error()})
        return
    }
    dst := filepath.Join("./uploads", filepath.Base(file.Filename))
    c.SaveUploadedFile(file, dst)
    c.JSON(200, gin.H{"filename": file.Filename, "size": file.Size})
}

// Multiple files
func uploadMultiple(c *gin.Context) {
    form, _ := c.MultipartForm()
    files := form.File["files"]
    for _, file := range files {
        c.SaveUploadedFile(file, filepath.Join("./uploads", file.Filename))
    }
}

// Set max memory for multipart
r.MaxMultipartMemory = 8 << 20  // 8 MB
```

## Graceful Shutdown

```go
srv := &http.Server{
    Addr:    ":8080",
    Handler: r,
}

go func() {
    if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
        log.Fatalf("listen: %s\n", err)
    }
}()

quit := make(chan os.Signal, 1)
signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
<-quit
log.Println("Shutting down...")

ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
defer cancel()
if err := srv.Shutdown(ctx); err != nil {
    log.Fatal("Server forced to shutdown:", err)
}
```

## Testing

```go
func TestGetUser(t *testing.T) {
    gin.SetMode(gin.TestMode)
    r := setupRouter()

    w := httptest.NewRecorder()
    req, _ := http.NewRequest("GET", "/users/1", nil)
    req.Header.Set("Authorization", "Bearer test-token")
    r.ServeHTTP(w, req)

    assert.Equal(t, 200, w.Code)

    var response map[string]interface{}
    json.Unmarshal(w.Body.Bytes(), &response)
    assert.Equal(t, "Alice", response["name"])
}

// Test with JSON body
func TestCreateUser(t *testing.T) {
    body := `{"name":"Bob","email":"bob@test.com"}`
    req, _ := http.NewRequest("POST", "/users",
        strings.NewReader(body))
    req.Header.Set("Content-Type", "application/json")

    w := httptest.NewRecorder()
    r.ServeHTTP(w, req)
    assert.Equal(t, 201, w.Code)
}
```

## CORS

```go
import "github.com/gin-contrib/cors"

r.Use(cors.New(cors.Config{
    AllowOrigins:     []string{"https://example.com"},
    AllowMethods:     []string{"GET", "POST", "PUT", "DELETE"},
    AllowHeaders:     []string{"Origin", "Content-Type", "Authorization"},
    ExposeHeaders:    []string{"Content-Length"},
    AllowCredentials: true,
    MaxAge:           12 * time.Hour,
}))
```

## Swagger Integration

```go
import swaggerFiles "github.com/swaggo/files"
import ginSwagger "github.com/swaggo/gin-swagger"

// @Summary Get user by ID
// @Param id path int true "User ID"
// @Success 200 {object} User
// @Router /users/{id} [get]
func getUser(c *gin.Context) { ... }

r.GET("/swagger/*any", ginSwagger.WrapHandler(swaggerFiles.Handler))
```

```bash
swag init  # generates docs from annotations
```

## Framework Comparison

| Feature             | Gin                | Echo                | Fiber              | Chi                 |
| ------------------- | ------------------ | ------------------- | ------------------ | ------------------- |
| Router              | httprouter (radix) | custom (radix)      | fasthttp (radix)   | net/http compatible |
| net/http compatible | Yes                | Yes                 | No (fasthttp)      | Yes                 |
| Middleware style    | HandlerFunc chain  | echo.MiddlewareFunc | fiber.Handler      | net/http middleware |
| Auto binding        | Tags-based         | Tags-based          | Tags-based         | Manual              |
| Validation          | go-playground      | go-playground       | go-playground      | bring your own      |
| Performance         | Very fast          | Very fast           | Fastest (fasthttp) | Fast                |
| Maturity            | Highest adoption   | Strong community    | Growing fast       | Minimal + powerful  |

Key tradeoff: Fiber uses fasthttp (not net/http), which means incompatibility with much of the Go standard library ecosystem. Gin and Chi are net/http-compatible.

## Common Patterns

### Error Handling

```go
type AppError struct {
    Code    int    `json:"code"`
    Message string `json:"message"`
}

func errorHandler() gin.HandlerFunc {
    return func(c *gin.Context) {
        c.Next()
        if len(c.Errors) > 0 {
            err := c.Errors.Last()
            switch e := err.Err.(type) {
            case *AppError:
                c.JSON(e.Code, e)
            default:
                c.JSON(500, gin.H{"error": "internal server error"})
            }
        }
    }
}
```

### Rate Limiting

```go
import "github.com/ulule/limiter/v3/drivers/middleware/gin"

rate, _ := limiter.NewRateFromFormatted("100-M")  // 100 per minute
store := memory.NewStore()
middleware := mgin.NewMiddleware(limiter.New(store, rate))
r.Use(middleware)
```
