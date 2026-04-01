# Effect Systems — Algebraic Effects, Handlers & Language Design

## Overview

An **effect system** types computational effects: I/O, exceptions, nondeterminism, mutable state. Traditional systems (Haskell's IO type, Java's checked exceptions) conflate effect tracking with particular implementations. **Algebraic effects and handlers** decouple the description of effects from their handlers, enabling multiple interpretations of the same code.

Central idea: programs return values *and* announce what effects they might perform. Different handlers interpret those effects differently.

## Motivation: Why Effects Matter

Consider error handling in Python:

```python
def divide(x, y):
    if y == 0:
        raise ValueError("division by zero")
    return x / y
```

**Questions the type system can't answer**:
- Will this function throw?
- If I call it twice, are both errors possible?
- Can I switch from throwing errors to returning Optional?

In a language with effect systems:

```
divide : (x: Int, y: Int) -> Int with {Error}
```

The type **explicitly declares** that divide _might_ throw an error (from the `Error` effect). A type-safe handler can intercept and reinterpret it.

## Classical Effects (Pre-Algebraic)

### Haskell IO Monad

Haskell tracks effects via **monads**. IO is opaque—it can do anything:

```haskell
main :: IO ()
main = do
  putStrLn "Hello"       -- IO effect
  filename <- getLine    -- IO effect
  let x = 1 + 2          -- No effect (pure)
  content <- readFile filename  -- IO effect
  putStrLn content
```

**Problem**: IO monad is a black box. You can't intercept and reinterpret IO before running it. The monadic structure is about sequencing, not effect interpretation.

### Java Checked Exceptions

Java explicitly types throws:

```java
void divide(int x, int y) throws ArithmeticException { ... }
```

**Problem**: exceptions can only be caught at runtime. You can't reinterpret the effect at the type level. Also, checked exceptions are tedious and widely disliked.

### Limitations

Classical approaches:
- Commit to one interpretation (IO in Haskell, exceptions in Java)
- Can't reuse code with different effect handlers
- Mixed concerns: effect declaration, sequencing, error handling

## Algebraic Effects: The Architecture

An algebraic effect is:

1. **Effect constructor**: describes an effect request (e.g., `Read : String -> String`)
2. **Handler**: interprets the effect, returns a result
3. **Computation**: declares which effects it uses

### Key Property: Multiple Interpretations

Same code with different handlers:

```
Computation C uses {Logger}:
  log("Starting")
  result = compute()
  log("Done")

Handler 1: Print to stdout
Handler 2: Collect into list   [log("Starting"), ...]
Handler 3: Send to server
```

### Formal Structure

An algebraic effect is an **operation** with **inputs** and **outputs**:

```
effect State(s):
    get : () -> s
    put : s -> ()
```

A **handler** provides implementations:

```
handle stateHandler(init):
  case get(): return init
  case put(newval): bind with handler(newval)
```

## Koka Language

[Koka](https://koka-lang.github.io/) is designed from the ground up with algebraic effects.

### Effect Declaration

Types declare effects explicitly:

```koka
fun factorial(n: int): <div, exn> int {
  if (n == 0) then 1
  else if (n < 0) then throw("negative")
  else n * factorial(n - 1)
}

// Type means:
// - `div` (divergence): might not terminate
// - `exn` (exception): might throw
```

### Effect Syntax

Define effect:

```koka
effect reader {
  fun ask() : config
}
```

Handler:

```koka
fun withReader(config: config, f: () -> <reader, e> a) : <e> a {
  handle(f) {
    fun ask() -> config
  }
}
```

Use:

```koka
fun foo() -> <reader> string {
  cfg <- ask()
  return cfg.name
}

main() {
  withReader(Config("test")) { foo() }
}
```

### Killer Feature: Effect Polymorphism

Functions parameterized over effects:

```koka
fun traverse(xs: list(a), f: (a) -> <e> b) : <e> list(b) {
  match xs {
    Nil -> Nil
    Cons(x, rest) -> Cons(f(x), traverse(rest, f))
  }
}

// traverse works with ANY effect: <exn>, <io>, <state>, combinations
traverse([1,2,3], fun(n) { throw("error") })  // <exn>
traverse([1,2,3], fun(n) { print(n); n*2 })   // <io>
```

## Eff Language

[Eff](https://www.eff-lang.org/) is a research language emphasizing effect-based reasoning.

### Effect Rows

Effects are organized in **rows** (similar to row-polymorphic records):

```eff
let div_by_zero : 'a.
  (() -> 'a with div_by_zero) -> 'a =
  fun f -> try f () with
  | effect DivideByZero -> 0
```

The function handles the `DivideByZero` effect, returning 0.

### Handlers as First-Class

In Eff, handlers are functions:

```eff
let logger : 'a.
  (unit -> 'a with logger) -> 'a =
  fun f ->
    try f () with
    | effect Log(msg) -> print_endline msg
```

## OCaml 5 Effects

[OCaml 5](https://ocaml.org/) introduced native algebraic effects (as `Effect` module), moving away from monadic IO.

### Example: Simulating Async/Await

```ocaml
type _ Effect.t +=
  | Sleep : float -> unit Effect.t
  | Yield : unit Effect.t

let sleep duration = Effect.perform (Sleep duration)

let rec scheduler : 'a. (unit -> 'a) -> 'a =
  fun f ->
  match f () with
  | x -> x
  | effect (Sleep d) k ->
      (* k is the continuation; reschedule later *)
      scheduler (fun () -> Effect.Deep.continue k ())
  | effect Yield k ->
      scheduler (fun () -> Effect.Deep.continue k ())
```

Enables **lightweight concurrency** without async/await syntax sugar.

## Comparison: Effects vs Monads

### Monads (Haskell IO)

```haskell
main :: IO ()
main = 
  putStrLn "start" >>= \_ ->
  readFile "file.txt" >>= \content ->
  putStrLn content
```

**Pros**:
- Explicit sequencing (monadic bind)
- Pure values separate from effects

**Cons**:
- Boilerplate (>>= chains)
- Can't reinterpret IO after sequence defined
- One interpretation per monad type

### Free Monads

Free monads separate effect description from interpretation:

```haskell
data Free f a
  = Pure a
  | Free (f (Free f a))

data LogF a = Log String a

type LogProgram a = Free LogF a

log' :: String -> LogProgram ()
log' msg = Free (Log msg (Pure ()))

runLog :: LogProgram a -> [String]
runLog (Pure a) = []
runLog (Free (Log msg rest)) = msg : runLog rest
```

**Pros**: multiple interpretations

**Cons**: still requires monadic boilerplate; less ergonomic than algebraic effects

### Algebraic Effects

```koka
fun myProgram() -> <logger> unit {
  log("start")
  content <- readFile("file.txt")
  log(content)
}
```

**Pros**: cleaner syntax, effect polymorphism, multiple handlers

**Cons**: less mature ecosystem, fewer languages support them

## Effect Polymorphism: The Power

Functions that work with *any* effect set:

```koka
fun map(xs: list(a), f: (a) -> <e> b) : <e> list(b) {
  match xs {
    Nil -> Nil
    Cons(x, rest) -> Cons(f(x), map(rest, f))
  }
}

// All valid:
map([1,2,3], fun(n) { n * 2 })                  // <pure>
map([1,2,3], fun(n) { async compute(n) })       // <async>
map([1,2,3], fun(n) { if (bad) throw("e") ; n }) // <exn>
```

Compare to Haskell:

```haskell
-- Requires explicit type class
class Monad m => Traversable t m a where
  traverse :: (Monad m) => (a -> m b) -> t a -> m (t b)

-- Must explicitly instantiate for each monad
```

Algebraic effects **decompose effects from sequencing**, enabling reuse.

## Practical Effect Tracking: Java

Java's checked exceptions are a proto-effect system (not algebraic, but explicit effect tracking):

```java
void processFile() throws IOException, ParseException {
    // Declares what checked exceptions this can throw
}
```

**Why proto-effects?**
- Effects are explicitly typed (IOException)
- But can't reinterpret or handle with different semantics
- Exceptions are the only effect mechanism

**Modern evolution**: project Loom (virtual threads) and Records/Sealed Classes start to enable effect-like patterns, but Java doesn't have algebraic effects.

## Effect Systems in the Wild

### Languages with algebraic effects

- **Koka**: first-class, effect polymorphism
- **Eff**: research language, handlers as functions
- **OCaml 5**: native Effect module
- **Scala 3** (proposed): via context functions and implicits

### Languages with classical effects

- **Haskell**: IO monad, typeclasses for effects
- **Rust**: Result/Option types (not true effects, but similar discipline)

## Trade-offs & Challenges

### Benefits

- **Modularity**: reuse code with different effect interpretations
- **Type safety**: effects explicit in signature
- **Composability**: multiple effects combine naturally

### Challenges

- **Performance**: effect checking may require overhead (mitigation: JIT specialization)
- **Complexity**: learning curve (monads already steep; effects steeper)
- **Ecosystem maturity**: fewer libraries, less tooling than monadic systems
- **Integration**: hard to retrofit into languages designed around monads (e.g., Haskell)

## Encoding Effects Without Language Support

In languages without built-in effect systems, use patterns:

1. **Reader monad** (dependency injection):
```haskell
type Config = { apiKey :: String, ... }
type App a = Config -> IO a
```

2. **Free monad** (effect description):
```haskell
data Effect a = ...
type Program a = Free Effect a
```

3. **Effect rows via records** (empirical):
```haskell
type Effects a = { throw :: e -> a, log :: String -> a, ... }
```

These are workarounds; algebraic effects are cleaner with language support.

## See Also

- **Type systems**: compilers-type-inference.md (type checking effects)
- **Monads**: math-category-theory.md (monad laws, composition)
- **Functional programming**: functional-programming.md (purity vs effects)
- **Exception handling**: api-design.md (error communication patterns)