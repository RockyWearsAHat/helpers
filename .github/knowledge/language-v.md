# V Best Practices

## V Philosophy

V is a simple, fast compiled language inspired by Go and Rust. It emphasizes simplicity, fast compilation (compiles itself in under a second), and safety without complexity.

- **Simple**: No classes, no inheritance, no exceptions, no macros, no generics metaprogramming.
- **Fast compilation**: Full self-compilation in ~0.5 seconds. Incremental builds are near-instant.
- **Safe**: Immutable by default, no null, optional/result types, bounds checking.

## Core Syntax

```v
// Variables — immutable by default
name := 'Alice'
age := 30

// Mutable
mut counter := 0
counter += 1

// No null — use Option types
fn find_user(id int) ?User {
    if id in users {
        return users[id]
    }
    return none
}

// Strings
greeting := 'Hello, ${name}!'
multiline := "
    this is
    a multiline string
"

// Arrays
mut numbers := [1, 2, 3, 4, 5]
numbers << 6              // append
doubled := numbers.map(fn (n int) int { return n * 2 })
evens := numbers.filter(fn (n int) bool { return n % 2 == 0 })

// Maps
mut scores := map[string]int{}
scores['Alice'] = 95
scores['Bob'] = 87
```

## Functions

```v
// Basic function
fn add(a int, b int) int {
    return a + b
}

// Multiple return values
fn divmod(a int, b int) (int, int) {
    return a / b, a % b
}

q, r := divmod(17, 5)

// Option return (can fail)
fn parse_port(s string) ?int {
    port := s.int()
    if port < 1 || port > 65535 {
        return error('invalid port: ${s}')
    }
    return port
}

// Result handling with or {}
port := parse_port('8080') or {
    eprintln('Error: ${err}')
    return
}

// Higher-order functions
fn apply(f fn (int) int, x int) int {
    return f(x)
}

result := apply(fn (n int) int { return n * n }, 5)
```

## Structs

```v
struct User {
    name  string       // immutable by default
    email string
mut:
    age   int          // mutable fields after mut:
    score f64
}

// Methods
fn (u User) greet() string {
    return 'Hello, I am ${u.name}'
}

fn (mut u User) birthday() {
    u.age += 1
}

// Embedded structs (composition over inheritance)
struct Admin {
    User              // embed all User fields
    role string
}

admin := Admin{
    name: 'Alice'
    email: 'alice@test.com'
    age: 30
    role: 'superadmin'
}
println(admin.greet())  // inherits User methods
```

## Enums and Match

```v
enum Color {
    red
    green
    blue
}

fn color_hex(c Color) string {
    return match c {
        .red { '#FF0000' }
        .green { '#00FF00' }
        .blue { '#0000FF' }
    }
}

// Sum types
type Expr = IntLit | BinOp | UnaryOp

struct IntLit { val int }
struct BinOp { left Expr  op string  right Expr }
struct UnaryOp { op string  operand Expr }

fn eval(e Expr) int {
    return match e {
        IntLit { e.val }
        BinOp { /* recursive eval */ 0 }
        UnaryOp { /* recursive eval */ 0 }
    }
}
```

## Concurrency

```v
import sync

// Spawn lightweight threads
fn compute(id int, mut wg sync.WaitGroup) {
    defer { wg.done() }
    // do work
    println('Worker ${id} done')
}

mut wg := sync.new_waitgroup()
for i in 0 .. 10 {
    wg.add(1)
    spawn compute(i, mut wg)
}
wg.wait()

// Channels
ch := chan int{cap: 10}

spawn fn [ch] () {
    for i in 0 .. 10 {
        ch <- i
    }
    ch.close()
}()

for {
    val := <-ch or { break }
    println(val)
}
```

## Error Handling

```v
// Option type (?T) — can return none or error
fn read_config(path string) ?Config {
    content := os.read_file(path) or {
        return error('cannot read ${path}: ${err}')
    }
    return json.decode(Config, content) or {
        return error('invalid JSON: ${err}')
    }
}

// Propagate errors with ?
fn setup() ? {
    config := read_config('config.json')?  // propagate on error
    db := connect(config.db_url)?
    start_server(config.port)?
}

// Handle at call site
setup() or {
    eprintln('Setup failed: ${err}')
    exit(1)
}
```

## Testing

```v
// Tests are in the same file or _test.v files
fn test_add() {
    assert add(2, 3) == 5
    assert add(-1, 1) == 0
}

fn test_parse_port() {
    assert parse_port('8080') or { 0 } == 8080
    assert parse_port('99999') == none
}

// Run: v test .
// Run specific: v test file_test.v
```

## Key Rules

1. **Immutable by default.** Use `mut` explicitly when mutation is needed.
2. **No null.** Use option types (`?T`) and handle with `or {}` blocks.
3. **No exceptions.** All errors flow through return values (`?T` / `!T`).
4. **`match` must be exhaustive.** The compiler requires all enum variants and sum type members to be handled.
5. **Prefer composition.** V has no inheritance — embed structs for code reuse.
6. **Use `defer`** for cleanup — file handles, locks, temporary state.

---

*Sources: V Documentation (vlang.io), V Language Reference, V by Example, V Standard Library*
