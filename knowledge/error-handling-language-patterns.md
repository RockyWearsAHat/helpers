# Error Handling Language Patterns — Exceptions, Result Types, and Functional Approaches

## Overview

Error handling philosophy divides along a fundamental axis: **exceptions (implicit control flow) vs. result types (explicit values)**. How a language handles errors shapes code clarity, performance, and correctness guarantees. This encompasses checked exceptions (Java, early C#), unchecked exceptions (Python, Ruby), result types (Rust), optional types, panic/recover mechanisms (Go), and monadic approaches (Haskell, functional languages).

The choice isn't purely technical—it's a statement about whether failures are exceptional (worthy of unwinding) or expected (values to handle inline).

## Exception-Based Error Handling

### Checked vs. Unchecked Exceptions (Java, Python, C#)

**Checked exceptions (Java)** force callers to handle or declare thrown exceptions at compile time. Declared via `throws` in the method signature.

```java
public void uploadFile(String path) throws IOException, ParseException {
    File file = new File(path);
    if (!file.exists()) throw new FileNotFoundException(path);
    // parse and upload
}

// Caller MUST handle
try {
    uploadFile("data.txt");
} catch (IOException e) {
    log.error("Upload failed", e);
} catch (ParseException e) {
    log.error("Parse failed", e);
}
```

**Strengths:** Compiler forces acknowledgment of failure modes. API contracts are explicit.

**Weaknesses:** Boilerplate cascades—`throws IOException` propagates up call stacks. Generic catch clauses (catching `Exception`) negate the benefit. Can't easily add new exception types to existing APIs without breaking all clients. Many Java libraries escape this via unchecked runtime exceptions.

**Unchecked exceptions (Python, Ruby, also C# and modern Java)** throw freely; catching is optional.

```python
def upload_file(path):
    if not os.path.exists(path):
        raise FileNotFoundError(path)
    # ...

# Caller may or may not catch
try:
    upload_file("data.txt")
except FileNotFoundError:
    print("File not found")
```

**Strengths:** Less boilerplate. Exceptional cases don't pollute signatures.

**Weaknesses:** Error conditions are invisible at call sites (silent failures). Stack unwinding makes debugging harder—original context is lost. Performance overhead in exceptions (stack capture, unwinding). Easy to ignore errors inadvertently.

### Exception Semantics: When to Use, When Not To

**Good use cases:**
- Logic errors: `IndexError`, `KeyError`—bugs, not recoverable conditions
- Resource exhaustion: low memory, disk full
- I/O failures: network, file system (truly unpredictable)
- Propagating through many layers where explicit error passing is tedious

**Anti-patterns:**
- Using exceptions for control flow. `try { return Integer.parseInt(s); } catch (NumberFormatException) {}` to test if `s` is numeric wastes performance.
- Bare catches: `catch (Exception e) {}` hides bugs
- Rethrowing without context: `catch (IOException e) { throw e; }` loses error metadata

**Best practice: Translate at boundaries.** Don't let low-level implementation details (SQL errors, HTTP codes) leak to callers. Wrap in a domain-meaningful exception.

```java
public User findById(long id) throws UserNotFoundException {
    try {
        return database.query("SELECT * FROM users WHERE id = ?", id);
    } catch (SQLException e) {
        throw new UserNotFoundException(id, e);  // wraps SQL detail
    }
}
```

---

## Result and Option Types (Rust, Swift, Haskell, Scala)

### Rust: Result<T, E> and Option<T>

Rust has **no exceptions**. Errors are values returned explicitly.

**Option<T>:** Represents "something or nothing"—the absence of a value.

```rust
pub fn divide(a: i32, b: i32) -> Option<i32> {
    if b == 0 {
        None
    } else {
        Some(a / b)
    }
}

// Caller unpacks with pattern matching
match divide(10, 2) {
    Some(result) => println!("Result: {}", result),
    None => println!("Cannot divide by zero"),
}

// Or use combinators
let result = divide(10, 2).map(|x| x * 2).unwrap_or(0);
```

**Result<T, E>:** Represents "success (T) or failure (E)"—recoverable errors.

```rust
#[derive(Debug)]
enum ParseError {
    InvalidFormat,
    OutOfRange,
}

pub fn parse_age(s: &str) -> Result<u32, ParseError> {
    s.parse::<u32>()
        .map_err(|_| ParseError::InvalidFormat)
        .and_then(|age| {
            if age > 150 {
                Err(ParseError::OutOfRange)
            } else {
                Ok(age)
            }
        })
}

// Caller is forced to handle the Result
let age: Result<u32, ParseError> = parse_age("25");
match age {
    Ok(a) => println!("Age: {}", a),
    Err(e) => eprintln!("Parse error: {:?}", e),
}
```

The **`?` operator** propagates errors within a function that returns `Result`.

```rust
fn process_file(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let data = std::fs::read_to_string(path)?;  // ? returns early on Err
    let parsed = parse_data(&data)?;
    Ok(parsed)
}
```

**Compiler enforces handling.** You can't accidentally ignore a `Result`; otherwise the code doesn't compile. This is Rust's killer feature: **type safety + explicit error propagation = no hidden failures**.

**Strengths:**
- Errors are values, not hidden control flow—visible in signatures
- Composable: map, and_then, or_else combine operations cleanly
- Zero runtime cost (no unwinding, no stack captures)
- Compiler ensures you handle errors

**Weaknesses:**
- Verbose for simple cases: "I expect this to succeed" requires explicit unwrap/expect, not implicit
- Error types must be designed upfront (or use `Box<dyn Error>`)
- Learning curve: monadic combinators (map, and_then) less familiar to imperative programmers

### Swift: try/throw and Result<Success, Failure>

Swift blends exceptions and result types.

**throw/try/catch (checked exceptions):**

```swift
enum ParseError: Error {
    case invalidFormat
    case outOfRange
}

func parseAge(from string: String) throws -> Int {
    guard let age = Int(string) else {
        throw ParseError.invalidFormat
    }
    guard age <= 150 else {
        throw ParseError.outOfRange
    }
    return age
}

// Must use try
do {
    let age = try parseAge(from: "25")
    print("Age: \(age)")
} catch ParseError.invalidFormat {
    print("Invalid format")
} catch ParseError.outOfRange {
    print("Out of range")
}
```

**Result<Success, Failure> (async-first):**

Modern async Swift prefers `async/await` with `throws`:

```swift
func fetchUser(id: Int) async throws -> User {
    let url = URL(string: "https://api.example.com/users/\(id)")!
    let (data, _) = try await URLSession.shared.data(from: url)
    return try JSONDecoder().decode(User.self, from: data)
}

// Caller
do {
    let user = try await fetchUser(id: 42)
    print(user)
} catch {
    print("Error: \(error)")
}
```

**Typed throws (Swift 5.5+):**

```swift
func processData(_ data: Data) throws(ParsingError) -> [Item] {
    // Can only throw ParsingError, not any Error
}
```

---

## Go: Error Values + Panic/Recover

Go rejects exceptions entirely. Go uses **error values** (the standard approach) and reserves **panic/recover** for truly exceptional cases.

### Error Values (Go convention)

Functions return `(value, error)` tuples. **Errors are just values.**

```go
func Open(name string) (string, error) {
    file, err := os.Open(name)
    if err != nil {
        return "", fmt.Errorf("open file: %w", err)
    }
    defer file.Close()
    return os.ReadFile(name)
}

// Caller checks error explicitly
data, err := Open("data.txt")
if err != nil {
    log.Fatal(err)
}
```

**Strengths:**
- Zero hidden control flow; errors are visible
- No performance cost
- Simple debugging: errors flow explicitly through code
- Forces discipline

**Weaknesses:**
- Boilerplate: repeated `if err != nil` checks
- Easy to forget handling (though `golangci-lint` can catch this)
- Error context must be manually propagated with wrapping (`fmt.Errorf("%w")`)

### Panic and Recover

**Panic** is Go's emergency exit—closer to exceptions than error values.

```go
func panicExample() {
    defer func() {
        if r := recover(); r != nil {
            fmt.Println("Recovered from:", r)
        }
    }()
    
    panic("Something went very wrong")  // stops execution
}
```

**Intended uses (rare):**
- Programmer errors: impossible state, violated invariants (`panic("unreachable")`)
- Critical resource exhaustion that can't be recovered from
- Testing or experiments

**Anti-pattern:** Using panic as a substitute for error handling. Panic should not be caught and ignored; it signals a true exceptional condition, not a business error.

```go
// BAD: Using panic for control flow
if err := validateUser(user); err != nil {
    panic(err)  // wrong—this should be an error return
}

// GOOD: Return error normally
if err := validateUser(user); err != nil {
    return fmt.Errorf("invalid user: %w", err)
}
```

---

## Monadic Error Handling (Haskell, Scala, Functional Programming)

### Either Monad

**Either a b** represents "Left (error) or Right (success)." In statically-typed functional languages, it's the standard error pattern.

```haskell
data Either a b = Left a | Right b

parseAge :: String -> Either String Int
parseAge s = case reads s of
    [(age, "")] -> if age >= 0 && age <= 150
                   then Right age
                   else Left "Age out of range"
    _ -> Left "Parse error"

-- Using do-notation (monadic bind)
loadUserProfile :: FilePath -> Either String UserProfile
loadUserProfile path = do
    content <- readFile path `catchLeft` \_ -> Left "Cannot read file"
    user <- parseJSON content
    profile <- fetchProfile (userId user)
    return profile

-- Error propagates early on Left
case loadUserProfile "user.json" of
    Left error -> putStrLn $ "Error: " ++ error
    Right profile -> print profile
```

**Strengths:**
- Type-safe: the type signature declares possible failures
- Composable: `do` notation chains operations, short-circuiting on Left
- Stateless, purely functional—no implicit control flow

**Weaknesses:**
- Requires understanding monads—not intuitive to imperative programmers
- Syntactic overhead compared to exceptions in typed languages
- Error types must be unified in Either (often wrapped in a sum type)

### Railway-Oriented Programming

F# formalization of error handling via composition:

```fsharp
type Result<'TSuccess, 'TError> = 
    | Ok of 'TSuccess
    | Error of 'TError

let (>>=) result f =
    match result with
    | Ok v -> f v
    | Error e -> Error e

let parseAge s =
    match Int32.TryParse(s) with
    | true, age when age >= 0 && age <= 150 -> Ok age
    | true, _ -> Error "Age out of range"
    | false, _ -> Error "Parse error"

let loadUser path =
    readFile path
    >>= parseJSON
    >>= validateUser
    >>= saveToDatabase
```

The "tracks" are success (Ok) and error (Error); operations flow along tracks, derailing early on Error.

---

## Error Handling Philosophy: Fail Fast vs. Graceful Degradation

### Fail Fast

Stop immediately on error, raise/return early, propagate up.

**When:** Business logic errors, precondition violations, invalid input.

```rust
pub fn transfer(from: Account, to: Account, amount: i64) -> Result<(), TransferError> {
    if amount <= 0 {
        return Err(TransferError::InvalidAmount);  // fail immediately
    }
    if from.balance < amount {
        return Err(TransferError::InsufficientFunds);
    }
    // proceed
}
```

### Graceful Degradation

Continue with partial results or fallbacks; log and proceed.

**When:** Optional features, cached data that may be stale, non-critical operations.

```javascript
async function enrichUserProfile(user) {
    const profile = { ...user };
    
    try {
        profile.avatar = await fetchAvatar(user.id);
    } catch (e) {
        // avatar failed, but user profile still valid
        console.warn("Failed to load avatar:", e);
        profile.avatar = null;
    }
    
    return profile;  // succeeds despite missing avatar
}
```

### When to Use Each

- **Fail fast:** Input validation, domain invariants, state-dependent operations
- **Graceful degradation:** I/O (network, cache), optional enhancements, analytics

Mixing is common: fail fast at entry, gracefully degrade at boundaries.

---

## Cross-Language Comparison

| Aspect | Exceptions (Java, Python) | Result Types (Rust, Go) | Monadic (Haskell) | Panic/Recover (Go) |
|--------|----------|------------|-------|------|
| **Visibility** | Hidden in stack trace | Explicit in return type | Explicit in type | Stops execution |
| **Compiler Enforcement** | Checked (Java) or none | Result<T,E> must unpack | Type-checked | None |
| **Performance** | Stack capture overhead | Zero runtime cost | Zero (lazy) | Expensive |
| **Boilerplate** | try/catch verbosity | Error handling repetitive | Combinator learning curve | Rare |
| **Composability** | One finally block | Chainable (map, ?) | Chainable (do notation) | None |
| **Best For** | Unexpected failures | Expected, recoverable errors | Pure FP / Type safety | Programmer bugs only |

---

## See Also

- [Rust Error Handling](language-rust-error-handling.md)
- [Go Design Patterns](language-go-patterns.md)
- [API Error Handling](api-error-handling.md)
- [Error Reporting Systems](error-reporting-systems.md)
- [Distributed Systems Error Handling](error-handling-distributed.md)