# Go Testing: Table-Driven Tests, Benchmarks, Fuzzing & Quality Patterns

## Introduction

Go's testing philosophy emphasizes simplicity and clarity. The `testing` package is minimal; there's no assertion library or test framework in the standard library. Tests are plain Go functions following naming conventions, making testing approachable and keeping dependencies small.

The testing culture values **table-driven tests** (parametrized test data), **benchmarking** for performance regression detection, and **fuzzing** for property-based input validation.

## Package and File Conventions

Test files use the `_test.go` suffix:

```go
// calc.go
func Add(a, b int) int {
    return a + b
}

// calc_test.go
package calc

import "testing"

func TestAdd(t *testing.T) {
    result := Add(2, 3)
    if result != 5 {
        t.Errorf("Add(2, 3) = %d, want 5", result)
    }
}
```

Test functions take `*testing.T` and must start with `Test`. The `testing` package is imported normally; Go's tooling automatically discovers `_test.go` files.

Subtests and sub-benchmarks use `t.Run()` for organization:

```go
func TestParsing(t *testing.T) {
    t.Run("valid input", func(t *testing.T) {
        // ...
    })
    t.Run("invalid input", func(t *testing.T) {
        // ...
    })
}
```

This produces output like `TestParsing/valid_input`, making failures easy to locate.

## Table-Driven Tests

The idiomatic Go pattern for parametrized testing:

```go
func TestAdd(t *testing.T) {
    tests := []struct {
        name   string
        a, b   int
        want   int
    }{
        {"positive", 2, 3, 5},
        {"zero", 0, 0, 0},
        {"negative", -1, 1, 0},
    }
    
    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            if got := Add(tt.a, tt.b); got != tt.want {
                t.Errorf("Add(%d, %d) = %d, want %d", tt.a, tt.b, got, tt.want)
            }
        })
    }
}
```

**Benefits:**
- Easy to add test cases: just append to the slice.
- Clear failure messages: includes the test case name.
- Readable: test logic is separate from test data.
- Scalable: handles dozens of cases without code duplication.

Table-driven tests should be used for all but the simplest functions.

## Benchmarking

Benchmark functions start with `Benchmark`:

```go
func BenchmarkAdd(b *testing.B) {
    for i := 0; i < b.N; i++ {
        Add(2, 3)
    }
}
```

Run benchmarks with `go test -bench=.`:

```bash
go test -bench=. -benchmem      # Include memory stats
go test -bench=. -benchtime=10s # Run each for 10 seconds (default 1s)
go test -bench=. -count=5       # Run each benchmark 5 times for stability
go test -bench=BenchmarkAdd     # Run specific benchmark
```

Output example:
```
BenchmarkAdd-12              10000000   100 ns/op   0 B/op   0 allocs/op
```

The `-12` is GOMAXPROCS (number of goroutines). `100 ns/op` is the average time per operation. `0 B/op` and `0 allocs/op` are memory stats.

**Guidelines:**
- Benchmark hot paths, not everything.
- Reset the timer inside the loop if initialization is expensive:
  ```go
  func BenchmarkParse(b *testing.B) {
      input := expensiveSetup()
      b.ResetTimer()  // Don't count setup time
      for i := 0; i < b.N; i++ {
          Parse(input)
      }
  }
  ```
- Use `b.ReportAllocs()` to force allocation reporting.
- Regularly run benchmarks to catch regressions.

## Fuzzing (Go 1.18+)

Go's native fuzzing is coverage-guided property-based testing:

```go
func FuzzParse(f *testing.F) {
    f.Add("valid input")
    f.Add("")
    f.Add("@#$%")
    
    f.Fuzz(func(t *testing.T, input string) {
        result, err := Parse(input)
        
        // Invariant: no panic, result is reasonable
        if result != nil {
            if len(result) < 0 {
                t.Errorf("negative length: %d", len(result))
            }
        }
    })
}
```

Run fuzz tests with `go test -fuzz=.`:

```bash
go test -fuzz=FuzzParse -fuzztime=30s  # Run for 30 seconds
go test -fuzz=FuzzParse -fuzztime=10x  # Run 10 iterations
```

Fuzzing automatically generates inputs and discovers failing cases. Failed cases are saved in `testdata/fuzz/` for regression testing:

```
testdata/fuzz/FuzzParse/
  54a3c7f0a2b1
  7f9e5b3c2d1a
```

These corpus files are automatically replayed on future test runs, preventing regressions.

## Test Fixtures and testdata

For file-based test data, use the `testdata/` directory:

```go
// main_test.go
func TestParseFile(t *testing.T) {
    data, err := ioutil.ReadFile("testdata/input.json")
    if err != nil {
        t.Fatal(err)
    }
    result, err := ParseJSON(data)
    if err != nil {
        t.Errorf("ParseJSON failed: %v", err)
    }
    // assertions...
}
```

`testdata/` is a special Go convention. Go automatically skips vendored modules' testdata, and testing tools understand the pattern. Use `testdata/` for:
- JSON/YAML fixtures
- Golden files (expected output for snapshot testing)
- Binary test inputs
- Configuration templates

## httptest for HTTP Testing

The `net/http/httptest` package simplifies HTTP testing:

```go
func TestAPI(t *testing.T) {
    handler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        w.WriteHeader(http.StatusOK)
        w.Write([]byte("OK"))
    })
    
    // Create a test server
    server := httptest.NewServer(handler)
    defer server.Close()
    
    // Make a request
    resp, err := http.Get(server.URL + "/path")
    if err != nil {
        t.Fatal(err)
    }
    
    // Verify response
    if resp.StatusCode != http.StatusOK {
        t.Errorf("status = %d, want 200", resp.StatusCode)
    }
}
```

`httptest.NewServer` creates a test HTTP server on a random port. Use `httptest.NewRequest` and `httptest.NewRecorder` for unit testing handlers without a full server:

```go
recorder := httptest.NewRecorder()
req := httptest.NewRequest("GET", "/path", nil)
handler := MyHandler()
handler.ServeHTTP(recorder, req)

if recorder.Code != http.StatusOK {
    t.Errorf("status = %d, want 200", recorder.Code)
}
```

## Race Detector

Go's race detector finds concurrent data races during test execution:

```bash
go test -race ./...
```

This instruments the binary to track memory access. Overhead is significant (~20-50% slower, higher memory), but catches subtle concurrency bugs:

```go
func TestConcurrency(t *testing.T) {
    var counter int
    done := make(chan bool)
    
    go func() {
        counter++  // Race: unsynchronized write
        done <- true
    }()
    
    counter++  // Race: unsynchronized read
    <-done
}
```

The race detector flags this during test execution. Use it regularly on concurrent code.

## Coverage

Go provides built-in coverage reporting:

```bash
go test -cover ./...           # Print coverage percentage
go test -coverprofile=coverage.out ./...
go tool cover -html=coverage.out
```

Coverage shows which lines of code have been executed by tests. High coverage is necessary but not sufficient—covered does not mean tested well.

Guidelines:
- Aim for 70-90% coverage for business logic.
- Don't obsess over 100%—error paths and rare cases often don't need heavy testing.
- Use coverage to identify untested code, not to enforce a metric.

## Integration vs. Unit Tests

**Unit tests** exercise one function or method in isolation, often with mocks:

```go
func TestUserService_Create(t *testing.T) {
    mock := &MockDB{}  // Mocked dependency
    svc := NewUserService(mock)
    
    user := svc.Create("alice")
    if user.Name != "alice" {
        t.Errorf("user.Name = %q, want alice", user.Name)
    }
}
```

**Integration tests** verify that multiple components work together with real dependencies:

```go
func TestUserService_Integration(t *testing.T) {
    if testing.Short() {
        t.Skip("skipping integration test")
    }
    
    db := setupTestDB()  // Real database or container
    defer db.Close()
    svc := NewUserService(db)
    
    user := svc.Create("alice")
    retrieved := svc.Get(user.ID)
    if retrieved.Name != "alice" {
        t.Errorf("retrieved.Name = %q, want alice", retrieved.Name)
    }
}
```

Run integration tests selectively:

```bash
go test ./...                   # Unit tests only
go test -run Integration ./...  # Only integration tests
go test -short ./...            # Skip long-running tests
```

## External Test Packages and Mocking

To avoid circular dependencies or test private functions, use `package mypackage_test` (note the `_test` suffix on the package name):

```go
// main.go
package calc

func unexported() int {
    return 42
}

// main_test.go
package calc_test  // External to the calc package

func TestExported(t *testing.T) {
    // Can import and test the public API
}
```

For mocking, Go prefers simple interfaces:

```go
type Reader interface {
    Read() (string, error)
}

type MockReader struct {
    Data   string
    Closed bool
}

func (m *MockReader) Read() (string, error) {
    m.Closed = true
    return m.Data, nil
}
```

Popular mocking libraries like `testify` (for assertions) and `gomock` (for generated mocks) exist but are opt-in. Standard library testing is sufficient for most cases.

## Golden Files

Golden files store expected output for snapshot testing:

```go
func TestFormat(t *testing.T) {
    result := Format(inputData)
    
    goldenPath := "testdata/format_golden.txt"
    golden, _ := ioutil.ReadFile(goldenPath)
    
    if string(result) != string(golden) {
        t.Errorf("Format output changed:\n%s\n\nWant:\n%s", result, golden)
        // Optionally: ioutil.WriteFile(goldenPath, result, 0644) # Update golden
    }
}
```

Golden files are useful for complex outputs (serialized data, formatted text, generated code). Update them deliberately; don't auto-update without review.

## Guidelines

- Write table-driven tests for all but trivial functions.
- Add benchmarks for performance-critical code; run them regularly.
- Use fuzzing on parsers and protocol handlers.
- Keep unit tests isolated; mock external dependencies.
- Use integration tests to verify component interactions, but run them separately.
- Enable the race detector on concurrent code (`-race`).
- Aim for 70-90% coverage, focusing on critical paths.
- Test public APIs; private function testing indicates design issues.

See also: [language-go.md](language-go.md), [language-go-patterns.md](language-go-patterns.md), [testing-philosophy.md](testing-philosophy.md)