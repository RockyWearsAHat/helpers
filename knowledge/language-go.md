# Go Conventions and Idioms

## Go Philosophy

Go is deliberately simple. Embrace the simplicity — don't fight it.

- **Simplicity over cleverness.** If a junior developer can't read it, it's too clever.
- **Explicit over implicit.** Go has no hidden control flow (no exceptions, no implicit interfaces, no inheritance).
- **Composition over inheritance.** Go has no classes. Embed structs, define small interfaces.
- **A little copying is better than a little dependency.**

## Error Handling

Go's most distinctive pattern. Errors are values, not exceptions.

```go
// The standard pattern
result, err := doSomething()
if err != nil {
    return fmt.Errorf("doing something: %w", err)  // Wrap with context
}

// Error wrapping (Go 1.13+)
if err != nil {
    return fmt.Errorf("failed to parse config %s: %w", path, err)
}

// Error checking with errors.Is / errors.As
if errors.Is(err, os.ErrNotExist) {
    // Handle file not found
}

var pathErr *os.PathError
if errors.As(err, &pathErr) {
    fmt.Println("Failed path:", pathErr.Path)
}

// Custom error types
type ValidationError struct {
    Field   string
    Message string
}

func (e *ValidationError) Error() string {
    return fmt.Sprintf("validation error on %s: %s", e.Field, e.Message)
}

// Sentinel errors
var (
    ErrNotFound     = errors.New("not found")
    ErrUnauthorized = errors.New("unauthorized")
)
```

**Conventions:**
- Check errors at every call site. Discarding with `_ = doSomething()` should be a conscious, justified choice.
- Wrap errors with context: `fmt.Errorf("operation context: %w", err)`.
- Use `%w` (not `%v`) for wrapping — preserves the error chain.
- Prefer returning errors over panicking in libraries. Panic is typically reserved for unrecoverable programmer errors.

## Interfaces

```go
// Interfaces are implicit — no "implements" keyword
type Reader interface {
    Read(p []byte) (n int, err error)
}

// Any type with a Read method satisfies io.Reader automatically

// Keep interfaces small (1-3 methods)
type Stringer interface {
    String() string
}

// Accept interfaces, return structs
func Process(r io.Reader) (*Result, error) {
    data, err := io.ReadAll(r)
    // ...
}

// Define interfaces where they're used, not where they're implemented
// (consumer-side interfaces)
```

## Goroutines & Channels

```go
// Goroutines — lightweight concurrent functions
go processItem(item)

// Channels — communication between goroutines
ch := make(chan Result, 10)  // Buffered channel

go func() {
    result, err := compute()
    ch <- Result{Value: result, Err: err}
}()

res := <-ch  // Receive

// Select — multiplex channel operations
select {
case msg := <-msgCh:
    handle(msg)
case err := <-errCh:
    handleError(err)
case <-ctx.Done():
    return ctx.Err()
case <-time.After(5 * time.Second):
    return errors.New("timeout")
}

// WaitGroup — wait for goroutines to finish
var wg sync.WaitGroup
for _, item := range items {
    wg.Add(1)
    go func() {
        defer wg.Done()
        process(item)
    }()
}
wg.Wait()

// errgroup — WaitGroup + error handling (golang.org/x/sync/errgroup)
g, ctx := errgroup.WithContext(ctx)
for _, url := range urls {
    g.Go(func() error {
        return fetch(ctx, url)
    })
}
if err := g.Wait(); err != nil {
    return err
}
```

**Concurrency conventions:**
- "Don't communicate by sharing memory; share memory by communicating" is a core Go proverb.
- `context.Context` is the standard mechanism for cancellation and timeouts.
- Closing channels from the sender side avoids races; receivers should not close.
- `sync.Mutex` works well for simple shared state; channels suit coordination patterns.

## Context

```go
// Always pass context as the first parameter
func GetUser(ctx context.Context, id string) (*User, error) {
    // Check for cancellation
    select {
    case <-ctx.Done():
        return nil, ctx.Err()
    default:
    }

    return db.QueryUser(ctx, id)
}

// Set timeouts
ctx, cancel := context.WithTimeout(parentCtx, 5*time.Second)
defer cancel()  // Always defer cancel

result, err := GetUser(ctx, "123")
```

## Structs & Methods

```go
// Struct definition
type User struct {
    ID        string    `json:"id"`
    Name      string    `json:"name"`
    Email     string    `json:"email"`
    CreatedAt time.Time `json:"created_at"`
}

// Constructor function (not a method)
func NewUser(name, email string) *User {
    return &User{
        ID:        uuid.New().String(),
        Name:      name,
        Email:     email,
        CreatedAt: time.Now(),
    }
}

// Value receiver (doesn't modify the struct)
func (u User) DisplayName() string {
    return u.Name
}

// Pointer receiver (can modify, avoids copy for large structs)
func (u *User) UpdateEmail(email string) {
    u.Email = email
}

// Embedding (composition, not inheritance)
type Admin struct {
    User            // Embedded — Admin gets all User methods
    Permissions []string
}
```

## Project Structure

```
myproject/
├── cmd/
│   └── myapp/
│       └── main.go        # Entry point
├── internal/               # Private packages (not importable externally)
│   ├── handler/
│   │   └── user.go
│   ├── service/
│   │   └── user.go
│   └── repository/
│       └── user.go
├── pkg/                    # Public packages (importable by others)
│   └── validator/
│       └── validator.go
├── go.mod
├── go.sum
└── README.md
```

## Testing

```go
// Table-driven tests — the Go standard
func TestAdd(t *testing.T) {
    tests := []struct {
        name     string
        a, b     int
        expected int
    }{
        {"positive", 2, 3, 5},
        {"negative", -1, -2, -3},
        {"zero", 0, 0, 0},
    }

    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            got := Add(tt.a, tt.b)
            if got != tt.expected {
                t.Errorf("Add(%d, %d) = %d, want %d", tt.a, tt.b, got, tt.expected)
            }
        })
    }
}

// Benchmarks
func BenchmarkSort(b *testing.B) {
    for b.Loop() {
        sort.Ints(data)
    }
}

// Testable examples (also serve as documentation)
func ExampleAdd() {
    fmt.Println(Add(2, 3))
    // Output: 5
}
```

## Common Patterns

```go
// Functional options (for configurable constructors)
type Option func(*Server)

func WithPort(port int) Option {
    return func(s *Server) { s.port = port }
}

func WithTimeout(d time.Duration) Option {
    return func(s *Server) { s.timeout = d }
}

func NewServer(opts ...Option) *Server {
    s := &Server{port: 8080, timeout: 30 * time.Second}
    for _, opt := range opts {
        opt(s)
    }
    return s
}

srv := NewServer(WithPort(9090), WithTimeout(10*time.Second))

// Defer for cleanup
func ReadFile(path string) ([]byte, error) {
    f, err := os.Open(path)
    if err != nil {
        return nil, err
    }
    defer f.Close()
    return io.ReadAll(f)
}
```

## Tooling

- `go fmt` / `gofmt` — Format code (the standard, universally expected in Go projects).
- `go vet` — Catch common bugs.
- `golangci-lint` — Meta-linter (runs many linters together).
- `go test -race` — Detect data races.
- `go test -cover` — Coverage reporting.
- `go mod tidy` — Clean up dependencies.

---

*Sources: Effective Go (golang.org), Go Code Review Comments, Go Proverbs (Rob Pike), 100 Go Mistakes and How to Avoid Them (Teiva Harsanyi), Standard library documentation*
