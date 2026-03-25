# Type Systems — Theory & Practical Application

## Type System Spectrum

```
No types        Weak/Dynamic        Strong/Dynamic        Strong/Static           Dependent Types
  ↓               ↓                     ↓                     ↓                       ↓
Assembly     JavaScript (weak)      Python, Ruby         Rust, Haskell, Java     Idris, Agda, Coq
             PHP, Perl             Elixir, Clojure    TypeScript, C#, Go, Swift   Lean, F*
```

### Key Distinctions
- **Static vs Dynamic**: Types checked at compile time vs runtime
- **Strong vs Weak**: Whether implicit type coercion happens (`"1" + 1` → `"11"` in JS = weak)
- **Nominal vs Structural**: Types identified by name (Java) vs shape (TypeScript, Go interfaces)
- **Sound vs Unsound**: Does the type system actually guarantee what it claims? (TypeScript is intentionally unsound; Rust is sound)

## Generics / Parametric Polymorphism

### The Basic Idea
Write code that works for any type while maintaining type safety:
```typescript
// Without generics: lose type information
function first(arr: any[]): any { return arr[0]; }

// With generics: preserve type information
function first<T>(arr: T[]): T { return arr[0]; }
first([1, 2, 3]);        // TypeScript infers: number
first(["a", "b", "c"]);  // TypeScript infers: string
```

### Bounded Generics (Constraints)
```typescript
// T must have a .length property
function longest<T extends { length: number }>(a: T, b: T): T {
    return a.length >= b.length ? a : b;
}

longest("abc", "de");     // OK: strings have length
longest([1, 2], [1]);     // OK: arrays have length
longest(10, 20);          // Error: numbers don't have length
```

```rust
// In Rust: trait bounds
fn largest<T: PartialOrd>(list: &[T]) -> &T {
    let mut largest = &list[0];
    for item in list {
        if item > largest { largest = item; }
    }
    largest
}
```

```java
// In Java: bounded type parameters
public <T extends Comparable<T>> T max(T a, T b) {
    return a.compareTo(b) >= 0 ? a : b;
}
```

## Variance — The Tricky Part

Variance describes how subtyping between complex types relates to subtyping between their components.

Given: `Dog extends Animal`

| Variance | `Container<Dog>` vs `Container<Animal>` | Example |
|----------|----------------------------------------|---------|
| Covariant | `Container<Dog>` is subtype of `Container<Animal>` | Read-only collections, return types |
| Contravariant | `Container<Animal>` is subtype of `Container<Dog>` | Write-only, function parameters |
| Invariant | No subtype relationship | Mutable collections |

### Why Mutable Collections Must Be Invariant
```java
// If List<Dog> were a subtype of List<Animal> (covariant):
List<Dog> dogs = new ArrayList<>();
List<Animal> animals = dogs;      // If this were allowed...
animals.add(new Cat());           // ...we could add a Cat to a Dog list!
Dog dog = dogs.get(0);            // ClassCastException! Cat is not a Dog.

// Java's actual solution:
List<? extends Animal> readable = dogs;   // Covariant (read-only)
List<? super Dog> writable = animals;     // Contravariant (write-only)
```

### Variance in Different Languages
```typescript
// TypeScript: Arrays are covariant (unsound by design!)
let dogs: Dog[] = [new Dog()];
let animals: Animal[] = dogs;  // Allowed! (unsound)

// Kotlin: Declaration-site variance
interface Source<out T> { fun get(): T }   // Covariant (out = produce)
interface Sink<in T> { fun put(t: T) }     // Contravariant (in = consume)

// Rust: Lifetime variance matters
// &'a T is covariant in 'a (longer lifetime can substitute shorter)
// &'a mut T is invariant in T (no substitution allowed)

// C#: Declaration-site variance on interfaces
interface IEnumerable<out T> { ... }       // Covariant
interface IComparer<in T> { ... }          // Contravariant
```

### The PECS Rule (Java)
**Producer Extends, Consumer Super**
- If you only READ from a collection → `<? extends T>` (covariant)
- If you only WRITE to a collection → `<? super T>` (contravariant)
- If you READ and WRITE → use exact type (invariant)

## Algebraic Data Types (ADTs)

### Sum Types (Tagged Unions / Discriminated Unions)
A value that is one of several possible variants:

```rust
// Rust: enums are sum types
enum Shape {
    Circle(f64),                     // radius
    Rectangle(f64, f64),             // width, height
    Triangle(f64, f64, f64),         // three sides
}

fn area(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle(r) => std::f64::consts::PI * r * r,
        Shape::Rectangle(w, h) => w * h,
        Shape::Triangle(a, b, c) => {
            let s = (a + b + c) / 2.0;
            (s * (s - a) * (s - b) * (s - c)).sqrt()
        }
    } // Compiler ensures all variants handled!
}
```

```typescript
// TypeScript: discriminated unions
type Shape =
    | { kind: "circle"; radius: number }
    | { kind: "rectangle"; width: number; height: number }
    | { kind: "triangle"; sides: [number, number, number] };

function area(shape: Shape): number {
    switch (shape.kind) {
        case "circle": return Math.PI * shape.radius ** 2;
        case "rectangle": return shape.width * shape.height;
        case "triangle": /* ... */
    } // TypeScript: exhaustiveness checking with --strict
}
```

```haskell
-- Haskell: algebraic data types
data Shape = Circle Double
           | Rectangle Double Double
           | Triangle Double Double Double

area :: Shape -> Double
area (Circle r) = pi * r * r
area (Rectangle w h) = w * h
area (Triangle a b c) = let s = (a + b + c) / 2
                         in sqrt (s * (s-a) * (s-b) * (s-c))
```

### Product Types (Records / Structs / Tuples)
A value that contains ALL of several fields — `(A, B, C)` has an A AND a B AND a C.

### The Option/Maybe Pattern (Null Safety)
```rust
// Rust: Option<T> replaces null
fn find_user(id: u64) -> Option<User> {
    // Returns Some(user) or None
}

// Must handle both cases explicitly
match find_user(42) {
    Some(user) => println!("Found: {}", user.name),
    None => println!("Not found"),
}

// Chaining
let name = find_user(42)
    .map(|u| u.name)
    .unwrap_or_else(|| "Unknown".to_string());
```

### The Result Pattern (Error Handling)
```rust
// Rust: Result<T, E> replaces exceptions
fn parse_config(path: &str) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path)?;  // ? propagates error
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}
```

## Phantom Types — Types With No Runtime Representation

```rust
use std::marker::PhantomData;

struct Meters;
struct Seconds;

struct Quantity<Unit> {
    value: f64,
    _unit: PhantomData<Unit>,
}

impl<U> Quantity<U> {
    fn new(value: f64) -> Self {
        Quantity { value, _unit: PhantomData }
    }
}

// Compile error: can't add meters and seconds
// fn add<U>(a: Quantity<U>, b: Quantity<U>) -> Quantity<U>
let distance = Quantity::<Meters>::new(100.0);
let time = Quantity::<Seconds>::new(9.58);
// distance + time;  // Won't compile! Different types.
```

## Type-Level Programming

### Conditional Types (TypeScript)
```typescript
type IsString<T> = T extends string ? "yes" : "no";
type A = IsString<string>;   // "yes"
type B = IsString<number>;   // "no"

// Practical: Extract return type of a function
type ReturnOf<T> = T extends (...args: any[]) => infer R ? R : never;
type X = ReturnOf<() => string>;  // string
```

### Associated Types (Rust)
```rust
trait Iterator {
    type Item;  // Associated type — each impl specifies the concrete type
    fn next(&mut self) -> Option<Self::Item>;
}

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> { /* ... */ }
}
```

## Practical Type Safety Patterns

### Newtype Pattern (Prevent Primitive Obsession)
```rust
struct UserId(u64);
struct OrderId(u64);

fn get_order(user: UserId, order: OrderId) -> Order { /* ... */ }

// Can't accidentally swap arguments:
// get_order(order_id, user_id);  // Compile error!
```

### Builder Pattern with Types (Typestate)
```rust
struct NoUrl;
struct HasUrl(String);

struct RequestBuilder<U> {
    url: U,
    headers: Vec<(String, String)>,
}

impl RequestBuilder<NoUrl> {
    fn new() -> Self { RequestBuilder { url: NoUrl, headers: vec![] } }
    fn url(self, url: &str) -> RequestBuilder<HasUrl> {
        RequestBuilder { url: HasUrl(url.into()), headers: self.headers }
    }
}

impl RequestBuilder<HasUrl> {
    fn send(self) -> Response { /* Only callable after url is set */ }
}

// RequestBuilder::new().send();        // Compile error!
// RequestBuilder::new().url("...").send();  // OK!
```

### Making Illegal States Unrepresentable
```typescript
// Bad: many invalid combinations possible
interface User {
    type: "guest" | "registered" | "admin";
    email?: string;      // Required for registered/admin, not guest
    adminLevel?: number; // Only for admin
}

// Good: impossible to construct an invalid user
type User =
    | { type: "guest" }
    | { type: "registered"; email: string }
    | { type: "admin"; email: string; adminLevel: number };
```

## Soundness vs Practicality

| Language | Sound? | Why? |
|----------|--------|------|
| Rust | Yes | Borrow checker, lifetime system, no null |
| Haskell | Yes* | Pure, immutable by default (*unsafe escape hatches exist) |
| Java | Mostly | Generics erasure, null, array covariance are unsound |
| TypeScript | No | Intentionally unsound for usability (`any`, array covariance) |
| Python (mypy) | Gradual | Type: ignore, Any, incomplete stubs |
| Go | Yes-ish | Simpler type system means fewer soundness holes, but `interface{}` |

**TypeScript's philosophy:** "We'd rather let some incorrect programs through than reject correct ones." This is a deliberate design choice — soundness is sacrificed for developer productivity.

---

*"Types are not just for the compiler — they're documentation that the compiler checks for you."*
