# Gleam — Type-Safe Functional Language for BEAM

## Overview

Gleam is a functional programming language targeting the BEAM virtual machine (used by Erlang and Elixir). The core promise: **statically-typed, pattern-matched code without runtime type errors**, while inheriting BEAM's battle-tested concurrency and fault tolerance. Gleam can also compile to JavaScript, enabling browser code with the same type safety.

Gleam emphasizes a small, focused feature set over maximizing expressiveness. The design philosophy: make invalid states unrepresentable at compile time, eliminate whole classes of bugs at the source.

## Type System: Hindley-Milner

Gleam uses **Hindley-Milner type inference** (like Haskell, OCaml, ReScript). You often omit type annotations; the compiler infers them:

```gleam
pub fn add(a, b) {
  a + b
}
// Compiler infers: fn(Int, Int) -> Int
```

Explicit annotations clarify intent:

```gleam
pub fn greet(name: String) -> String {
  "Hello, " <> name
}
```

### Algebraic Data Types (ADTs)

Define types as a sum (choice) of variants:

```gleam
pub type Result(a, e) {
  Ok(a)
  Error(e)
}

pub type Maybe(a) {
  Some(a)
  None
}
```

**Exhaustiveness checking**: pattern match must handle all cases:

```gleam
fn process(result: Result(Int, String)) {
  case result {
    Ok(value) -> value + 1
    Error(msg) -> panic(msg)
  }
  // Compile error if a case is missing
}
```

Missing a pattern is a compile error, not a runtime bug.

### Generics (Parametric Types)

Define behavior for any type:

```gleam
pub fn identity(x: a) -> a {
  x
}

pub fn map(list: List(a), f: fn(a) -> b) -> List(b) {
  case list {
    [] -> []
    [head, ..tail] -> [f(head), ..map(tail, f)]
  }
}
```

The `a` and `b` are **type variables** (universally quantified). The compiler ensures consistency across all uses.

## Pattern Matching

Pattern matching is the primary control flow mechanism:

```gleam
pub fn classify(n: Int) {
  case n {
    0 -> "zero"
    1 | 2 | 3 -> "small"
    x if x > 100 -> "large"
    _ -> "other"
  }
}

pub fn head_and_tail(list: List(Int)) {
  case list {
    [first, ..rest] -> #(first, rest)
    [] -> panic("empty list")
  }
}

pub fn destructure(pair: #(Int, String)) {
  let #(number, text) = pair
  number + string_length(text)
}
```

**Guards**: additional conditions on patterns via `if`.

**Exhaustiveness**: unmatched cases cause compile errors. This forces programmers to handle all scenarios explicitly.

## Erlang and Elixir Interop

Gleam runs on BEAM, so it can call Erlang and Elixir code:

```gleam
@external(erlang, "erlang", "length")
pub fn erl_length(list: List(a)) -> Int
```

The `@external` attribute marks a function as implemented in Erlang. Likewise, Erlang code can call Gleam functions (they compile to callable modules).

This enables:
- **Gradual adoption**: existing Erlang/Elixir projects can use Gleam incrementally
- **Rich ecosystem**: leverage thousands of BEAM packages
- **Interop without boundary**: works transparently

## OTP and Supervision Trees

Gleam inherits BEAM's **OTP** (Open Telecom Platform) — a framework for building reliable distributed systems. The core pattern: **supervision trees** and **let it crash** philosophy.

### Processes

Lightweight concurrency units (millions can run per machine):

```gleam
import gleam/process

pub fn start_worker() {
  process.new_subject()
  |> fn(subject) {
    process.spawn(fn() {
      loop(subject)
    })
  }
}

fn loop(subject) {
  case process.receive(subject, 5000) {
    Ok(msg) -> {
      handle_message(msg)
      loop(subject)
    }
    Error(Timeout) -> loop(subject)
  }
}
```

Processes communicate via **message passing** (no shared memory).

### Supervision Trees

Organize processes hierarchically:

```gleam
pub fn start_root_supervisor() {
  let assert Ok(root_sup) =
    gleam_otp.supervisor.start_link(fn(children) {
      [
        gleam_otp.supervisor.worker(start_worker_service),
        gleam_otp.supervisor.supervisor(start_child_supervisor),
      ]
    })
  root_sup
}
```

If a child process crashes, the supervisor restarts it. This implements fault tolerance by architecture.

## Result Type and Use Expressions

### Result Type

Instead of exceptions, Gleam uses **Result** for recoverable errors:

```gleam
pub fn divide(a: Int, b: Int) -> Result(Int, String) {
  case b {
    0 -> Error("division by zero")
    _ -> Ok(a / b)
  }
}

pub fn main() {
  case divide(10, 2) {
    Ok(result) -> io.println(int.to_string(result))
    Error(reason) -> io.println("Error: " <> reason)
  }
}
```

No exception propagation. Errors are **explicit and typed**.

### Use Expressions

Flatten nested case statements:

```gleam
pub fn process_user(id: Int) -> Result(String, String) {
  use user <- result.try(fetch_user(id))
  use profile <- result.try(fetch_profile(user.profile_id))
  Ok(profile.bio)
}
```

Desugars to:

```gleam
pub fn process_user(id: Int) -> Result(String, String) {
  case fetch_user(id) {
    Ok(user) ->
      case fetch_profile(user.profile_id) {
        Ok(profile) -> Ok(profile.bio)
        Error(e) -> Error(e)
      }
    Error(e) -> Error(e)
  }
}
```

The `use` keyword avoids pyramid-of-doom callback nesting, making error handling readable.

## Opaque Types

Hide implementation details while maintaining type safety:

```gleam
pub opaque type UserId {
  UserId(Int)
}

pub fn new_user_id(id: Int) -> UserId {
  UserId(id)
}

pub fn user_id_to_int(id: UserId) -> Int {
  let UserId(value) = id
  value
}
```

Outside the module, `UserId` is an abstract type. Users can't construct it directly, only via public constructors. This enforces invariants.

## Immutability and Functional Updates

All data is immutable. "Updating" creates a new copy:

```gleam
pub type Person {
  Person(name: String, age: Int)
}

pub fn birthday(person: Person) -> Person {
  Person(..person, age: person.age + 1)
}
```

The `..` syntax is structural — it copies all fields and overrides the specified ones. Efficient implementations avoid full copies (persistent data structures).

## BEAM Target

Gleam compiles to BEAM bytecode (Erlang AST):

```bash
gleam build   # compiles .gleam to .beam
```

Execution:

```bash
gleam run     # runs the project via BEAM
```

BEAM features available:
- **Concurrency**: millions of lightweight processes
- **Distribution**: cluster communication
- **Hot code reloading**: update code without stopping the system
- **Preemptive scheduling**: fair scheduling across processes
- **Garbage collection**: per-process, no stop-the-world

## JavaScript Target

Gleam also compiles to JavaScript:

```gleam
pub fn greet_js(name: String) -> String {
  "Hello from JavaScript, " <> name
}
```

Running in the browser:

```bash
gleam build --target javascript
```

The same type safety and pattern matching work in the browser. JavaScript output includes TypeScript definitions for interop.

### Browser APIs

External functions bind to JavaScript:

```gleam
@external(javascript, "window.location", "href")
pub fn get_current_url() -> String
```

## Package Manager and Build Tool

Gleam includes a built-in package manager and build tool:

```bash
gleam new myapp         # scaffold project
gleam add gleam_json    # add dependency
gleam build             # compile
gleam test              # run tests
gleam docs              # generate docs
```

No separate Cargo.toml, package.json, or Makefile. One tool unifies the workflow.

## Common Patterns

### Data Validation

Ensure valid state at compile time:

```gleam
pub opaque type Email {
  Email(String)
}

pub fn new_email(str: String) -> Result(Email, String) {
  case string.contains(str, "@") {
    True -> Ok(Email(str))
    False -> Error("invalid email")
  }
}
```

Invalid emails can't be constructed. The type system enforces validity.

### Pipeline Processing

Functional data transformation:

```gleam
[1, 2, 3, 4, 5]
|> list.filter(fn(x) { x > 2 })
|> list.map(fn(x) { x * 2 })
|> list.reduce(0, fn(a, b) { a + b })
// Result: 24
```

### Supervised System

Build a resilient service:

```gleam
pub fn main() {
  let assert Ok(_) =
    gleam_otp.supervisor.start_link(start_supervisor)
  process.sleep_forever()
}

fn start_supervisor(children) {
  [
    gleam_otp.supervisor.worker(start_http_server),
    gleam_otp.supervisor.worker(start_background_job_processor),
  ]
}
```

## Strengths and Limitations

**Strengths:**
- **Type safety**: catches errors at compile time, not production
- **Concurrency**: BEAM is battle-tested for millions of concurrent connections
- **Fault tolerance**: supervision trees + "let it crash" philosophy
- **Simplicity**: small language, easy to learn
- **Interop**: seamless use of Erlang/Elixir ecosystem

**Limitations:**
- **Smaller ecosystem**: fewer packages than Python, JavaScript, Rust
- **Compile-time overhead**: pattern match exhaustiveness checking is strict (good, but slower iteration)
- **JavaScript target immaturity**: fewer BEAM features available in JS
- **Adoption**: nascent community compared to established languages

## Use Cases

- **Backend services**: microservices with fault tolerance and concurrency
- **Real-time systems**: messaging platforms, chat, IoT
- **Distributed systems**: leverage BEAM clustering
- **Frontend**: browser code with type safety
- **Gradual Erlang/Elixir migration**: typed layer on top of existing systems

## see also

- [language-erlang.md](language-erlang.md) — BEAM, OTP, let it crash philosophy
- [language-elixir.md](language-elixir.md) — Erlang's Ruby-like sibling
- [paradigm-concurrent-models.md](paradigm-concurrent-models.md) — actor model, message passing
- [paradigm-functional-programming.md](paradigm-functional-programming.md) — immutability, pattern matching