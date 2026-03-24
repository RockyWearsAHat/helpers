# Rust Best Practices

## Core Concepts: Ownership & Borrowing

Rust's defining feature — memory safety without garbage collection. The compiler enforces these rules at compile time.

### Ownership Rules
1. Each value has exactly one owner.
2. When the owner goes out of scope, the value is dropped.
3. Ownership can be transferred (moved) or borrowed (referenced).

```rust
// Move — ownership transferred, original unusable
let s1 = String::from("hello");
let s2 = s1;  // s1 is MOVED to s2
// println!("{s1}");  // ❌ Compile error: value used after move

// Clone — explicit deep copy
let s1 = String::from("hello");
let s2 = s1.clone();  // s1 is still valid
println!("{s1} {s2}");  // ✅ Both valid

// Copy — automatic for stack types (i32, f64, bool, char, tuples of Copy types)
let x = 5;
let y = x;  // Copy, not move
println!("{x} {y}");  // ✅ Both valid
```

### Borrowing
```rust
// Immutable reference (&T) — many allowed simultaneously
fn length(s: &str) -> usize {
    s.len()
}

// Mutable reference (&mut T) — exactly one at a time, no immutable refs active
fn push_greeting(s: &mut String) {
    s.push_str(", world!");
}

// The rules:
// 1. Any number of &T references, OR
// 2. Exactly one &mut T reference
// (never both at the same time)
```

### Lifetimes
```rust
// Lifetime annotations tell the compiler how long references live
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}

// Lifetime elision — compiler infers in common cases:
// 1. Each input reference gets its own lifetime
// 2. If one input lifetime, output gets that lifetime
// 3. If &self is an input, output gets self's lifetime
fn first_word(&self) -> &str { ... }  // Compiler infers lifetimes
```

## Error Handling: Result & Option

Rust has no exceptions. Errors are values.

```rust
// Result<T, E> — operation that can fail
fn parse_port(s: &str) -> Result<u16, ParseIntError> {
    s.parse::<u16>()
}

// The ? operator — propagate errors concisely
fn read_config(path: &str) -> Result<Config, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;  // Returns Err early if fails
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

// Option<T> — value that might not exist
fn find_user(id: u64) -> Option<User> {
    users.iter().find(|u| u.id == id).cloned()
}

// Combinators — transform without unwrapping
let port: u16 = env::var("PORT")
    .ok()                          // Result → Option
    .and_then(|s| s.parse().ok())  // Parse, ignore errors
    .unwrap_or(8080);              // Default value

// Map, and_then, unwrap_or_else, map_err
let name = user
    .map(|u| u.name.clone())
    .unwrap_or_else(|| "Anonymous".to_string());
```

### Custom Error Types

```rust
use thiserror::Error;  // The standard crate for error types

#[derive(Error, Debug)]
enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found: {resource} with id {id}")]
    NotFound { resource: &'static str, id: u64 },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("IO error")]
    Io(#[from] std::io::Error),
}

// For application entry points, use anyhow for ergonomic error handling
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let config = load_config()
        .context("Failed to load configuration")?;
    Ok(())
}
```

## Structs, Enums, and Traits

```rust
// Structs
#[derive(Debug, Clone, PartialEq)]
struct User {
    id: u64,
    name: String,
    email: String,
}

impl User {
    // Associated function (constructor pattern)
    fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            id: generate_id(),
            name: name.into(),
            email: email.into(),
        }
    }

    // Method (takes &self)
    fn display_name(&self) -> &str {
        &self.name
    }
}

// Enums — algebraic data types (far more powerful than C/Java enums)
enum Command {
    Quit,
    Echo(String),
    Move { x: i32, y: i32 },
    Color(u8, u8, u8),
}

// Pattern matching — must be exhaustive
fn execute(cmd: Command) {
    match cmd {
        Command::Quit => process::exit(0),
        Command::Echo(msg) => println!("{msg}"),
        Command::Move { x, y } => move_cursor(x, y),
        Command::Color(r, g, b) => set_color(r, g, b),
    }
}

// Traits — shared behavior (like interfaces)
trait Summary {
    fn summarize(&self) -> String;

    // Default implementation
    fn preview(&self) -> String {
        format!("{}...", &self.summarize()[..50])
    }
}

impl Summary for User {
    fn summarize(&self) -> String {
        format!("{} <{}>", self.name, self.email)
    }
}

// Trait bounds
fn notify(item: &impl Summary) {
    println!("Breaking: {}", item.summarize());
}

// Multiple bounds
fn process<T: Summary + Clone + Debug>(item: &T) { ... }

// Where clause (cleaner for complex bounds)
fn process<T>(item: &T)
where
    T: Summary + Clone + Debug,
{ ... }
```

## Iterators

```rust
// Iterators are zero-cost abstractions — compile to same assembly as loops
let sum: i32 = numbers.iter().filter(|&&n| n > 0).sum();

let names: Vec<String> = users
    .iter()
    .filter(|u| u.active)
    .map(|u| u.name.clone())
    .collect();

// Iterator adaptors (lazy — nothing happens until consumed)
// .map(), .filter(), .flat_map(), .take(), .skip(), .zip()
// .chain(), .enumerate(), .peekable(), .inspect()

// Consumers (drive iteration)
// .collect(), .sum(), .count(), .any(), .all()
// .find(), .position(), .fold(), .for_each()
```

## Concurrency

```rust
// Threads
use std::thread;

let handle = thread::spawn(|| {
    expensive_computation()
});
let result = handle.join().unwrap();

// Channels (message passing)
use std::sync::mpsc;

let (tx, rx) = mpsc::channel();
thread::spawn(move || {
    tx.send(42).unwrap();
});
let value = rx.recv().unwrap();

// Shared state (Mutex + Arc)
use std::sync::{Arc, Mutex};

let counter = Arc::new(Mutex::new(0));
let handles: Vec<_> = (0..10).map(|_| {
    let counter = Arc::clone(&counter);
    thread::spawn(move || {
        let mut num = counter.lock().unwrap();
        *num += 1;
    })
}).collect();

// Async (tokio)
#[tokio::main]
async fn main() {
    let (a, b) = tokio::join!(fetch_data(), fetch_config());
}
```

**Rust's concurrency guarantee**: Data races are compile-time errors. `Send` and `Sync` marker traits enforce thread safety automatically.

## Common Crates

| Crate | Purpose |
|-------|---------|
| **serde** | Serialization/deserialization (JSON, TOML, YAML, etc.) |
| **tokio** | Async runtime |
| **reqwest** | HTTP client |
| **axum** / **actix-web** | Web frameworks |
| **sqlx** | Async SQL (compile-time checked queries) |
| **clap** | CLI argument parsing |
| **tracing** | Structured logging/tracing |
| **thiserror** | Derive Error traits for library errors |
| **anyhow** | Ergonomic error handling for applications |
| **rayon** | Data parallelism (parallel iterators) |

## Clippy Lints

Run `cargo clippy` on every commit. Key categories:
- **correctness**: Bugs and undefined behavior.
- **suspicious**: Code that's probably wrong.
- **style**: Idiomatic Rust conventions.
- **performance**: Unnecessary allocations and copies.
- **complexity**: Overly complex code that can be simplified.

---

*Sources: The Rust Programming Language (Klabnik & Nichols), Rust by Example, Rust API Guidelines, Rust Design Patterns, Clippy documentation*
