# Error Handling Patterns Across Languages

## Pattern 1: Return Error Codes (C, Go)

The oldest pattern. Functions return a status value indicating success or failure.

**C convention:** Return 0 for success, non-zero for error.

```c
int result = open_file("data.txt");
if (result != 0) { handle_error(result); }
```

**Go convention:** Return `(value, error)` tuples. Check error at every call site.

```go
file, err := os.Open("data.txt")
if err != nil {
    return fmt.Errorf("open data: %w", err)
}
```

**Strengths:** Explicit, no hidden control flow, zero runtime cost.
**Weaknesses:** Verbose. Easy to forget checking the return value. Error context must be manually propagated.

## Pattern 2: Exceptions (Java, C#, Python, Ruby, C++)

Errors are objects thrown up the call stack until caught. Separates the "happy path" from error handling.

**Checked exceptions (Java):** Compiler forces you to handle or declare them. Good for recoverable errors. Bad for boilerplate — `throws IOException` cascades.

**Unchecked/runtime exceptions (Python, Ruby, C#):** Throw freely, catch where appropriate. Less boilerplate but error conditions are invisible at the call site.

**Best practices:**

- Catch specific exceptions, not base `Exception`.
- Don't use exceptions for flow control (they're expensive and misleading).
- Include context: `raise ValueError(f"Expected positive, got {value}")`.
- Translate exceptions at layer boundaries (don't leak implementation details).
- Always clean up resources: use `try/finally`, `with`, `using`, or RAII.

## Pattern 3: Callbacks and Promises (JavaScript/Node.js)

Asynchronous error handling patterns.

**Error-first callbacks (legacy Node.js):**

```javascript
fs.readFile("data.txt", (err, data) => {
  if (err) {
    console.error(err);
    return;
  }
  process.stdout.write(data);
});
```

**Promises:**

```javascript
fetch("/api/data")
  .then((response) => response.json())
  .catch((err) => console.error("API failed:", err));
```

**async/await (modern standard):**

```javascript
try {
  const response = await fetch("/api/data");
  const data = await response.json();
} catch (err) {
  console.error("API failed:", err);
}
```

**Best practices:**

- Never ignore promise rejections — add `.catch()` or wrap in `try/catch`.
- `async` functions always return promises — callers must handle rejection.
- Use `Promise.allSettled()` when you need all results regardless of individual failures.

## Pattern 4: Result/Option Types (Rust, Haskell, Kotlin, Swift)

Encode success/failure in the type system. The compiler forces exhaustive handling.

**Rust:**

```rust
fn parse_number(s: &str) -> Result<i32, ParseIntError> {
    s.parse::<i32>()  // Returns Result<i32, ParseIntError>
}

match parse_number("42") {
    Ok(n) => println!("Parsed: {}", n),
    Err(e) => eprintln!("Error: {}", e),
}
```

**Swift:**

```swift
enum Result<Success, Failure: Error> {
    case success(Success)
    case failure(Failure)
}
```

**Option/Maybe** for nullable values:

- Rust: `Option<T>` — `Some(value)` or `None`
- Haskell: `Maybe a` — `Just a` or `Nothing`
- Kotlin: `T?` nullable types with `?.` safe-call operator
- Swift: `Optional<T>` with `if let` unwrapping

**Strengths:** Compiler-enforced handling. No forgotten error paths. Composable with `.map()`, `.and_then()`, `?` operator.
**Weaknesses:** Learning curve. Can be verbose without language sugar.

## Universal Error Handling Principles

1. **Don't swallow errors silently.** An empty `catch {}` is almost always a bug.
2. **Fail fast at system boundaries.** Validate inputs early, reject bad data immediately.
3. **Include context.** Error messages should say _what_ failed, _why_, and with _what input_.
4. **Translate errors at layer boundaries.** A database error should become a domain error before reaching the API layer.
5. **Log errors with structured data** — not just a string message.
6. **Distinguish recoverable from fatal errors.** Retry network timeouts; don't retry null pointer dereferences.
7. **Clean up resources** regardless of success or failure (RAII, try-with-resources, defer, using).

---

_Sources: andreabergia.com (error handling survey), Rust Book (Result/Option), Go Blog (error handling), MDN (Promise/async-await)_
