# Rust Error Handling — Result, Option, Panic, and Error Propagation

## Overview

Rust errors fall into two categories: **recoverable** (business errors returned via `Result`) and **unrecoverable** (logic bugs that `panic`). Using the correct category prevents data loss and enables robust systems. Rust's `Result<T, E>` and `Option<T>` types make error handling explicit and type-safe, unlike exceptions in other languages.

## Result<T, E> and Option<T>

**`Result<T, E>`** represents either success (`Ok(T)`) or failure (`Err(E)`):

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}

fn parse_port(s: &str) -> Result<u16, std::num::ParseIntError> {
    s.parse::<u16>()
}
```

**`Option<T>`** represents presence (`Some(T)`) or absence (`None`):

```rust
enum Option<T> {
    Some(T),
    None,
}

fn get_first_word(text: &str) -> Option<&str> {
    text.split(' ').next()  // Returns None if empty
}
```

Both are **composable type-level error types**, enforced at compile time.

## Reading Results: Pattern Matching

The clearest way to handle errors:

```rust
match parse_port("8080") {
    Ok(port) => println!("Port: {}", port),
    Err(e) => eprintln!("Error: {}", e),
}
```

For `Option`:

```rust
match get_first(vec) {
    Some(val) => println!("First: {}", val),
    None => println!("Empty vec"),
}
```

## Convenience Methods on Result and Option

**Query methods** are non-consuming:

```rust
let x = Ok(5);
assert!(x.is_ok());
assert!(!x.is_err());

let y = None;
assert!(!y.is_some());
```

**Extracting values** consumes the Result:

- `unwrap()` — panics if `Err` or `None`; returns the value otherwise
- `expect(msg)` — panics with custom message on error
- `unwrap_or(default)` — returns default if error
- `unwrap_or_else(f)` — applies closure to error value
- `unwrap_or_default()` — returns `T::default()` if error

```rust
let port = parse_port("8080").expect("Invalid port");  // Panics on error

let port = parse_port("bad").unwrap_or(3000);  // Returns 3000 if error

let port = parse_port("bad")
    .unwrap_or_else(|e| {
        eprintln!("Parse failed: {}", e);
        3000
    });
```

**Transforming values** with `map` and `and_then`:

```rust
let result = Ok(5);
let doubled = result.map(|x| x * 2);  // Ok(10)

let result: Result<i32, &str> = Err("bad");
let mapped = result.map(|x| x * 2);  // Err("bad") — map skipped

// Chain operations: if any fails, short-circuit
let result = Ok(5)
    .and_then(|x| Ok(x * 2))
    .and_then(|x| Ok(x + 1));  // Ok(11)

let val = Some(5)
    .and_then(|x| Some(x * 2))
    .or_else(|| Some(0));  // Returns Some(10)
```

## The Question Mark Operator: `?`

**The `?` operator** is syntactic sugar for early error return. It unwraps `Ok` values and returns `Err` immediately:

```rust
fn write_config(name: &str, value: &str) -> io::Result<()> {
    let mut file = File::create("config.txt")?;  // Returns on error
    file.write_all(name.as_bytes())?;            // Returns on error
    file.write_all(value.as_bytes())?;           // Returns on error
    Ok(())
}
```

This is equivalent to:

```rust
fn write_config(name: &str, value: &str) -> io::Result<()> {
    let mut file = match File::create("config.txt") {
        Ok(f) => f,
        Err(e) => return Err(e),
    };
    match file.write_all(name.as_bytes()) {
        Ok(_) => {},
        Err(e) => return Err(e),
    }
    match file.write_all(value.as_bytes()) {
        Ok(_) => {},
        Err(e) => return Err(e),
    }
    Ok(())
}
```

**Constraint**: `?` only works in functions that return `Result` or `Option`:

```rust
fn main() {
    let x = parse_port("8080")?;  // ERROR: main returns ()
}

fn process() -> Result<(), Box<dyn std::error::Error>> {
    let x = parse_port("8080")?;  // OK: returns Result
    Ok(())
}
```

### Converting Between Error Types with `?`

The `?` operator automatically converts error types using `From`:

```rust
fn process() -> Result<(), Box<dyn Error>> {
    let port = parse_port("8080")?;          // ParseIntError -> Box<dyn Error>
    let content = read_to_string("file")?;   // io::Error -> Box<dyn Error>
    Ok(())
}
```

This works because `Box<dyn Error>` implements `From<T>` for many error types.

## Error Conversion: From and Into

The `From` trait enables automatic error conversion:

```rust
impl From<ParseIntError> for ConfigError {
    fn from(err: ParseIntError) -> Self {
        ConfigError::InvalidPort(err.to_string())
    }
}

fn load_config() -> Result<Config, ConfigError> {
    let port = "8080".parse::<u16>()?;  // ParseIntError -> ConfigError
    Ok(Config { port })
}
```

**`Into` is the reciprocal**: automatically generated from `From`.

## Custom Error Types

### Simple Enum-Based Errors

```rust
#[derive(Debug)]
enum UserError {
    NotFound,
    DuplicateEmail,
    InvalidAge,
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            UserError::NotFound => write!(f, "User not found"),
            UserError::DuplicateEmail => write!(f, "Email already in use"),
            UserError::InvalidAge => write!(f, "Age invalid"),
        }
    }
}

impl std::error::Error for UserError {}
```

### Using `thiserror` Crate

`thiserror` automates boilerplate:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum UserError {
    #[error("user not found")]
    NotFound,
    
    #[error("email already in use")]
    DuplicateEmail,
    
    #[error("invalid age: {0}")]
    InvalidAge(u32),
    
    #[error(transparent)]
    Io(#[from] io::Error),
}

// Automatically implements Display, Error, From<io::Error>
```

### Using `anyhow` Crate

`anyhow::Result` wraps any error type; `anyhow::Error` provides context:

```rust
use anyhow::{Context, Result};

fn load_config() -> Result<Config> {
    let data = std::fs::read_to_string("config.json")
        .context("failed to read config file")?;
    
    serde_json::from_str(&data)
        .context("config is not valid JSON")?;
    
    Ok(config)
}
```

**Difference**: `thiserror` for library-specific errors; `anyhow` for application code and third-party error handling.

## Error Handling Patterns

### Bail Early, Return Late

Keep unhappy paths short:

```rust
fn process(data: &str) -> Result<String, ParseError> {
    // Validate and bail first
    if data.is_empty() {
        return Err(ParseError::Empty);
    }
    
    // Happy path logic follows
    let result = complex_parse(data)?;
    Ok(format_result(result))
}
```

### Error Chains and Context

Propagate context up the call stack:

```rust
fn parse_users_file(path: &str) -> Result<Vec<User>> {
    let content = std::fs::read_to_string(path)
        .context("failed to read users file")?;
    
    serde_json::from_str(&content)
        .context("users file is invalid JSON")?;
    
    Ok(users)
}

// Caller knows: JSON parsing failed in users file, which failed to read
```

### Recoverable vs. Unrecoverable

**Use `Result` for recoverable errors**:

```rust
fn connect(host: &str, port: u16) -> Result<Connection, IoError> {
    // Callers can retry, fall back, etc.
    TcpStream::connect((host, port))
}
```

**Use `panic!` for unreconcoverable programmer errors**:

```rust
fn vector_first<T>(v: &[T]) -> &T {
    &v[0]  // Panics if empty; logic error
}

fn safe_first<T>(v: &[T]) -> Option<&T> {
    v.first()  // Returns None; handled gracefully
}
```

## Panic vs. Result

**`panic!`** immediately terminates the thread with an error message:

```rust
panic!("Critical condition: {}", reason);
```

By default, panics **unwind** the stack, running destructors (Drop impls). Use `panic = 'abort'` in `Cargo.toml` for release builds to crash immediately:

```toml
[profile.release]
panic = 'abort'
```

**`catch_unwind`** catches panics (advanced; not recommended for normal control flow):

```rust
use std::panic;

let result = panic::catch_unwind(|| {
    // Code that might panic
});
```

## Must-Use Results

The `#[must_use]` attribute on `Result` causes compiler warnings if the result is ignored:

```rust
#[must_use]
fn critical_write(data: &[u8]) -> Result<()> { }

critical_write(b"data");  // WARNING: unused `Result`
```

Always handle critical operations:

```rust
critical_write(b"data")?;  // OK
critical_write(b"data").expect("write failed");  // OK
let _ = critical_write(b"data");  // OK: explicitly ignored
```

## Best Practices

1. **Use `Result` for expected failures**, `panic!` for bugs
2. **Provide context when propagating errors** (`anyhow::Context`)
3. **In libraries, define custom error types** (`thiserror`); in apps, use `anyhow`
4. **Use `?` to reduce boilerplate** in error-prone functions
5. **Never ignore `Result` values**; use `expect()` if unsure
6. **Implement `From` for automatic error conversion**
7. **Keep error enum variants focused and meaningful** (not one huge `Other` variant)
8. **Prefer `Option::ok_or()` over panicking on `None`**:

```rust
let val = vec.get(0).ok_or(MyError::NotFound)?;
```

## See Also

- `language-rust-ownership.md` — `Result` and ownership
- `language-rust-async.md` — Error handling in async contexts
- `api-error-handling.md` — REST API error conventions