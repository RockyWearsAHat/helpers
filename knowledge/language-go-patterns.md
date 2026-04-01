# Go Design Patterns — Error Handling, Interfaces, Testing, Dependency Injection & Context

## Overview

Go has no exceptions, inheritance, or implicit interfaces. Instead, Go codifies patterns through simplicity: explicit error returns, composition, and small interfaces. These patterns emerge naturally from the language's constraints and philosophy.

## Error Handling

Go's distinctive error model treats errors as values. Idiomatic Go is loud about errors; they flow explicitly through return values.

### Sentinel errors

Pre-defined error values for common failure modes:

```go
var (
    ErrNotFound      = errors.New("not found")
    ErrInvalidConfig = errors.New("invalid config")
    ErrPermission    = fmt.Errorf("permission denied")
)

func findUser(id int) (*User, error) {
    if id < 1 {
        return nil, ErrNotFound
    }
    return &User{ID: id}, nil
}

// Check with ==
result, err := findUser(0)
if err == ErrNotFound {
    // Handle not found case
}
```

### Error Wrapping and Context (Go 1.13+)

Wrap errors with context using `fmt.Errorf("%w", err)`:

```go
func loadConfig(path string) (*Config, error) {
    data, err := os.ReadFile(path)
    if err != nil {
        return nil, fmt.Errorf("reading config from %s: %w", path, err)
    }

    cfg := &Config{}
    err = json.Unmarshal(data, cfg)
    if err != nil {
        return nil, fmt.Errorf("parsing config: %w", err)
    }

    return cfg, nil
}

// Caller can inspect wrapped error
data, err := loadConfig("app.json")
if err != nil {
    if errors.Is(err, os.ErrNotExist) {
        // Handle missing file
    }
}
```

### Custom Error Types

Implement the `error` interface (`Error() string`) for domain-specific errors:

```go
type ValidationError struct {
    Field   string
    Message string
}

func (e *ValidationError) Error() string {
    return fmt.Sprintf("validation error on field %s: %s", e.Field, e.Message)
}

// Usage
func validateEmail(email string) error {
    if !strings.Contains(email, "@") {
        return &ValidationError{Field: "email", Message: "missing @"}
    }
    return nil
}

// Inspect with errors.As
err := validateEmail("invalid")
var valErr *ValidationError
if errors.As(err, &valErr) {
    fmt.Printf("Field: %s\n", valErr.Field)
}
```

### Error Propagation Pattern

Check and wrap at every level:

```go
func ProcessFile(path string) error {
    file, err := os.Open(path)
    if err != nil {
        return fmt.Errorf("opening file: %w", err)  // Don't lose context
    }
    defer file.Close()

    scanner := bufio.NewScanner(file)
    for scanner.Scan() {
        err := processLine(scanner.Text())
        if err != nil {
            return fmt.Errorf("processing line: %w", err)
        }
    }

    if err := scanner.Err(); err != nil {
        return fmt.Errorf("scanning file: %w", err)
    }

    return nil
}
```

## Interfaces: Small, Implicit, Composable

Go interfaces are implicit: a type satisfies an interface if it implements all its methods. No `implements` keyword.

### Small Interfaces

One- or two-method interfaces are the norm:

```go
type Reader interface {
    Read(p []byte) (n int, err error)
}

type Writer interface {
    Write(p []byte) (n int, err error)
}

type ReadWriter interface {
    Reader
    Writer
}
```

**Why small?** Easy to implement, compose, and satisfy. Large interfaces are implementation-specific; small ones are universally useful.

### Implicit Satisfaction

Any type with matching methods satisfies the interface:

```go
type File struct { /* ... */ }
func (f *File) Read(p []byte) (int, error) { /* ... */ }

type StringReader struct{ s string; pos int }
func (sr *StringReader) Read(p []byte) (int, error) { /* ... */ }

// Both Reader
var readers []io.Reader
readers = append(readers, file, stringReader)  // Any mixture
```

### Interface{} and Type Assertions

`interface{}` (empty interface) is satisfied by any type:

```go
func Print(v interface{}) {
    fmt.Println(v)  // Works for anything
}

// Type assertion
val, ok := v.(string)  // ok is true if v is a string
if ok {
    fmt.Println("String:", val)
}

// Type switch
switch v := v.(type) {
case string:
    fmt.Println("String:", v)
case int:
    fmt.Println("Int:", v)
case error:
    fmt.Println("Error:", v)
default:
    fmt.Println("Unknown:", v)
}
```

## Table-Driven Tests

Structure tests as data. Powerful for parameterized testing:

```go
func TestAdd(t *testing.T) {
    tests := []struct {
        name     string
        a, b     int
        expected int
    }{
        {name: "positive", a: 2, b: 3, expected: 5},
        {name: "negative", a: -1, b: 1, expected: 0},
        {name: "zero", a: 0, b: 0, expected: 0},
    }

    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            result := Add(tt.a, tt.b)
            if result != tt.expected {
                t.Errorf("Add(%d, %d) = %d, want %d", tt.a, tt.b, result, tt.expected)
            }
        })
    }
}
```

**Benefits:** Clear test structures, easy to add cases, parallelizable (`t.Parallel()`), generates isolated subtest names.

### Subtests with Shared Setup

```go
func TestDatabase(t *testing.T) {
    db := setupDB(t)  // Shared setup
    defer db.Close()

    tests := []struct {
        name string
        id   int
        want string
    }{
        {name: "user1", id: 1, want: "Alice"},
        {name: "missing", id: 999, want: ""},
    }

    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            user := db.GetUser(tt.id)
            if user.Name != tt.want {
                t.Errorf("expected %q, got %q", tt.want, user.Name)
            }
        })
    }
}
```

## Dependency Injection Without Frameworks

Go favors explicit, constructor-based DI. No reflection or annotations needed:

### Constructor Pattern

Injected dependencies as constructor arguments:

```go
type Service struct {
    repo   Repository
    logger Logger
}

func NewService(repo Repository, logger Logger) *Service {
    return &Service{repo: repo, logger: logger}
}

func (s *Service) Process(id string) error {
    s.logger.Info("processing", id)
    item, err := s.repo.Get(id)
    if err != nil {
        return fmt.Errorf("fetching: %w", err)
    }
    // ...
    return nil
}

// Usage
var repo Repository = NewMockRepo()  // Test or prod implementation
var logger Logger = NewConsoleLogger()
svc := NewService(repo, logger)
```

### Functional Options (Advanced)

For many optional parameters, use a variadic option function:

```go
type Config struct {
    Timeout  time.Duration
    MaxRetry int
    Logger   Logger
    TLS      bool
}

type Option func(*Config)

func WithTimeout(d time.Duration) Option {
    return func(cfg *Config) {
        cfg.Timeout = d
    }
}

func WithLogger(l Logger) Option {
    return func(cfg *Config) {
        cfg.Logger = l
    }
}

func NewClient(opts ...Option) *Client {
    cfg := &Config{
        Timeout:  30 * time.Second,
        MaxRetry: 3,
    }
    for _, opt := range opts {
        opt(cfg)
    }
    return &Client{cfg: cfg}
}

// Flexible, extensible usage
client := NewClient(
    WithTimeout(10 * time.Second),
    WithLogger(customLogger),
)
```

## Context: Request-Scoped Data and Cancellation

`context.Context` propagates deadlines, cancellation signals, and request-scoped values:

### Timeout and Cancellation

```go
ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
defer cancel()

result, err := fetchWithContext(ctx)
if err == context.DeadlineExceeded {
    // Handle timeout
}
```

### Cancellation Propagation

```go
func handleRequest(ctx context.Context) error {
    // Context can be cancelled by caller or timeout
    ch := make(chan string)
    go func() {
        ch <- expensiveOperation()
    }()

    select {
    case result := <-ch:
        return processResult(result)
    case <-ctx.Done():
        return ctx.Err()  // Cancelled or timed out
    }
}
```

### Values (Sparingly)

Store request-scoped values (request ID, user info, trace IDs):

```go
type contextKey string
const userKey contextKey = "user"

ctx := context.WithValue(context.Background(), userKey, "alice")

// Retrieve
user := ctx.Value(userKey).(string)
```

**Constraint:** Keys are `interface{}` — define typed constants to avoid collisions. Use sparingly; explicit function parameters are preferable.

## Code Generation

Go leverages code generation for boilerplate:

```go
//go:generate stringer -type=Status
type Status int

const (
    Pending Status = iota
    Running
    Done
)

// Generates Status.String() method
```

Run `go generate ./...` to create methods, mocks, builders:

```sh
$ go generate ./...
// Produces status_string.go with String() methods
```

**Common generators:**
- `stringer` — `String()` for enum-like types.
- `mockgen` — Interface mocks for testing.
- `protoc` — Protocol buffer code.
- Custom `main.go` scripts with `//go:generate` directives.

## Best Practices

- **Check errors explicitly.** `if err != nil` is verbose but clear.
- **Wrap errors with context.** Never lose the original error.
- **Define small interfaces.** One method when possible, two max.
- **Compose interfaces.** `ReadWriter = Reader + Writer`.
- **Avoid `interface{}`.** Use generics (Go 1.18+) or concrete types.
- **Use table-driven tests.** Parametrize test cases.
- **Pass context to all I/O operations.** Allows timeouts and cancellation.
- **Avoid goroutine leaks.** Always drain or cancel goroutines.

## See Also

- **language-go.md** — Go idioms and conventions
- **error-handling-patterns.md** — Error handling across languages
- **design-patterns.md** — Gang of Four patterns (some adapted for Go)
- **concurrency-patterns.md** — Goroutines and channels