# Go Interfaces: Implicit Satisfaction, Composition & Type Dynamics

## Introduction

Go interfaces are a minimal, powerful abstraction mechanism. Unlike languages with explicit interface declaration, Go uses **structural typing**: any type automatically satisfies an interface if it implements all its methods. No inheritance, no inheritance hierarchy—just the methods that exist.

This design choice makes interfaces more flexible and encourages smaller, focused abstractions. Interfaces compose naturally and promote the principle "accept interfaces, return structs."

## Implicit Interface Satisfaction

An interface is satisfied implicitly when a type implements all the interface's methods:

```go
type Writer interface {
    Write(p []byte) (n int, err error)
}

type File struct { /* ... */ }
func (f *File) Write(p []byte) (n int, err error) { /* ... */ }

var w Writer = &File{}  // *File automatically satisfies Writer
```

No explicit declaration like `type File implements Writer` is needed. The Go compiler checks method signatures at assignment time. Method names, parameter types, and return types must match exactly (including variadic slots).

**Receiver type matters:** A method on a pointer receiver `(*T)` satisfies an interface; a method on a value receiver `(T)` does not automatically work with pointers. `*T` can call both receiver types (because Go auto-dereferences); `T` can only call value receiver methods.

## Interface Composition

Interfaces can embed other interfaces, creating composition:

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

`ReadWriter` now includes all methods from both `Reader` and `Writer`. This is purely structural—no runtime inheritance. Large interfaces like this are often seen as a code smell; consider whether callers really need all methods or just a subset.

## The Empty Interface `interface{}`

The empty interface has no methods, so every type satisfies it:

```go
var x interface{} = 42
var y interface{} = "hello"
var z interface{} = []int{1, 2, 3}
```

The empty interface is useful (and necessary) only in a few cases:
1. **Generic data structures** before Go generics (pre-1.18): maps, slices, function arguments
2. **Reflection and dynamic dispatch**: runtime type checking
3. **JSON unmarshaling into unknown schema**: `json.Unmarshal(data, &interface{}{})`

With **Go 1.18+ generics**, most uses of `interface{}` should be replaced by type parameters. The type-safe version is clearer and catches errors at compile time.

## Type Assertions and Type Switches

A **type assertion** extracts the concrete type from an interface value:

```go
var w io.Writer = someFile
f, ok := w.(*os.File)  // Type assertion with comma-ok
if ok {
    fmt.Println("was *os.File, can now use File methods")
}
```

Without the comma-ok pattern, a type assertion that fails causes a panic. Always use the two-value form in production.

A **type switch** is a control structure for handling multiple possible types:

```go
switch v := x.(type) {
case int:
    fmt.Printf("int: %d\n", v)
case string:
    fmt.Printf("string: %s\n", v)
case []byte:
    fmt.Printf("bytes\n")
default:
    fmt.Printf("unknown type\n")
}
```

Type switches are the Go idiomatic way to dispatch on runtime type. Each case's variable `v` has the appropriate type without manual casting.

## Common Interfaces in the Standard Library

Understanding these interfaces is essential to writing idiomatic Go code:

- **`io.Reader`**: `Read(p []byte) (n int, err error)` — any source of bytes (files, network, buffers, decompressors)
- **`io.Writer`**: `Write(p []byte) (n int, err error)` — any destination for bytes
- **`io.Closer`**: `Close() error` — any resource that must be released
- **`error`**: `Error() string` — Go's exception mechanism (just a method)
- **`fmt.Stringer`**: `String() string` — custom `String()` representation used by `fmt.Printf("%v")`
- **`sort.Interface`**: `Len()`, `Less(i, j int) bool`, `Swap(i, j int)` — making types sortable
- **`context.Context`**: value, deadline, cancellation, and deadline management across goroutines

Many of these are composed from smaller interfaces. `io.ReadWriter` is just `Reader` + `Writer` together.

## Interface Segregation and Size

Smaller, focused interfaces are better than large, general ones:

```go
// Good: each caller depends on only what it uses
type Logger interface {
    Log(msg string)
}

type ConfigReader interface {
    Read() (Config, error)
}

// Avoid: one large interface
type Framework interface {
    Log(msg string)
    Read() (Config, error)
    Process(data []byte) []byte
    // ... 20 more methods
}
```

Large interfaces are harder to implement (you must implement everything) and force callers to depend on more than they need. The Unix philosophy applies: do one thing well.

## Accept Interfaces, Return Structs

This is the Go convention for library design:

```go
// Good function signature
func ProcessData(r io.Reader) (Result, error) {
    // Accept the minimal interface needed
}

// Not recommended
func ProcessData(f *os.File) (Result, error) {
    // Now the function is tightly coupled to *os.File
}

// Good: return concrete struct
type Result struct {
    Count   int
    Digest  [32]byte
}

// Avoid: return interface when you know the concrete type
func New() io.Reader {
    return &Reader{}  // Caller can't use specific Reader methods
}
```

Accepting interfaces keeps your function flexible and testable (callers can pass mocks or alternative implementations). Returning structs is concrete—callers get known methods and avoid boxing overhead.

## Interface Values and Runtime Representation

An interface value contains two pointers: a pointer to the type's method table (itab) and a pointer to the actual data:

```go
var r io.Reader = someFile  // Points to (itab for *os.File + data)
```

Interface method calls involve dynamic dispatch (lookup in itab), which has minimal overhead but is not zero. For very tight loops, consider avoiding interface indirection or using concrete types with generics.

**nil interface values:** An interface variable can be `nil`, but an interface holding a `nil` pointer is not nil itself:

```go
var r io.Reader = nil  // r is nil
var f *os.File
r = f  // r is not nil even though it contains a nil pointer
if r == nil {
    // false — r is not nil, but r.(io.Reader) would be nil
}
```

This distinction is a common source of confusion and subtle bugs.

## Guidelines

- Use interfaces to define behavior, not data structures.
- Keep interfaces small (1-3 methods when possible).
- Compose larger behaviors from smaller interfaces.
- Rely on implicit satisfaction to avoid breaking changes and tight coupling.
- Use type assertions/switches only when necessary; prefer accepting interfaces instead.
- Design for "accept interfaces, return structs"—it makes testing easier and APIs clearer.

See also: [language-go](language-go.md), [language-go-patterns.md](language-go-patterns.md), [api-design.md](api-design.md)