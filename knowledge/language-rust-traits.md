# Rust Traits — Trait Bounds, Objects, Associated Types, and Marker Traits

## Overview

Traits define shared behavior across types. They are Rust's primary mechanism for abstraction and polymorphism. Traits enable generic programming, code reuse, and the ability to write functions that work with many types that implement a common interface. Understanding trait bounds, trait objects, associated types, and marker traits separates novice from production-grade Rust.

## Defining and Implementing Traits

A trait defines a set of method signatures that implementing types must provide. Unlike classes, traits contain only a contract, not state.

```rust
pub trait Summary {
    fn summarize(&self) -> String;
}
```

Types implement the trait using `impl Trait for Type`:

```rust
impl Summary for NewsArticle {
    fn summarize(&self) -> String {
        format!("{}, by {}", self.headline, self.author)
    }
}
```

**Default implementations** let trait authors provide fallback behavior; implementors can override or inherit it:

```rust
pub trait Summary {
    fn summarize(&self) -> String {
        String::from("(Read more...)")
    }
}

impl Summary for NewsArticle {} // Uses default
```

## Trait Bounds and Generic Functions

**Trait bounds** constrain generic types to those implementing specific traits. They're the foundation of generic programming in Rust.

### `impl Trait` Syntax

For simple cases, `impl Trait` in parameter position is concise:

```rust
pub fn notify(item: &impl Summary) {
    println!("Breaking news! {}", item.summarize());
}
```

### Generic Trait Bounds

For complex constraints, use explicit generics:

```rust
pub fn notify<T: Summary>(item: &T) {
    println!("Breaking news! {}", item.summarize());
}
```

This forces both parameters to the same concrete type (unlike `impl Trait`, which allows each to differ):

```rust
pub fn notify<T: Summary>(item1: &T, item2: &T) {
    // Both must be the same type
}
```

### Multiple Trait Bounds

The `+` operator stacks bounds:

```rust
pub fn notify(item: &(impl Summary + Display)) {
    println!("{}: {}", item.summarize(), item);
}

// Or with generic syntax:
pub fn notify<T: Summary + Display>(item: &T) { }

// With `where` for clarity:
pub fn notify<T>(item: &T)
where
    T: Summary + Display,
{ }
```

### Returning Trait-Implementing Types

`impl Trait` in return position works **only for a single concrete type**:

```rust
fn returns_summarizable() -> impl Summary {
    SocialPost { /* ... */ }
}
```

Cannot return different types (e.g., conditional `NewsArticle` or `SocialPost`):

```rust
fn returns_either(switch: bool) -> impl Summary {
    if switch {
        NewsArticle { /* ... */ }  // Compile error: different type
    } else {
        SocialPost { /* ... */ }
    }
}
```

Use **trait objects** (`dyn`) instead.

## Trait Objects: Dynamic Dispatch

**Trait objects** (`dyn Trait`) enable returning different types that share a trait. They trade compile-time polymorphism for runtime dispatch. The compiler generates virtual method tables (vtables).

```rust
pub fn returns_either(switch: bool) -> Box<dyn Summary> {
    if switch {
        Box::new(NewsArticle { /* ... */ })
    } else {
        Box::new(SocialPost { /* ... */ })
    }
}

let article: Box<dyn Summary> = Box::new(NewsArticle { /* ... */ });
println!("{}", article.summarize());
```

**Constraints on object safety**: Not all traits can become trait objects. `dyn Trait` requires:

1. **No `Self` in method signatures** (without `Self: Sized`), since the concrete type is erased
2. **No generic type parameters** on methods (create ambiguity with unknown concrete type)
3. **All methods must not return `Self`** (would require upcasting)

Example: `Clone` cannot be `dyn Clone` because `fn clone(&self) -> Self` directly references `Self`.

## Associated Types

**Associated types** are type placeholders within traits. They differ from generic type parameters: each implementation binds them to one specific type; the caller doesn't name them.

```rust
pub trait Iterator {
    type Item;  // Associated type
    fn next(&mut self) -> Option<Self::Item>;
}

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> { /* ... */ }
}
```

**Associated types vs. generics**: Use associated types when each implementation chooses one type (not multiple). Use generics when a function can work with many types simultaneously:

```rust
// Associated type: one type per impl
trait Iter {
    type Item;
}

// Generic: many types per function call
fn process<T>(items: &[T]) { }
```

## Supertraits and Blanket Implementations

**Supertraits** are trait dependencies—implementing a child trait requires implementing its parent:

```rust
pub trait Summarize: Display {  // Display is a supertrait
    fn summarize(&self) -> String;
}

impl Summarize for Article {
    fn summarize(&self) -> String { /* ... */ }
    // Must also implement Display since it's a supertrait
}
```

**Blanket implementations** provide an implementation for all types satisfying a trait bound:

```rust
impl<T: Display> ToString for T {
    fn to_string(&self) -> String {
        format!("{}", self)
    }
}

// Now all types implementing Display automatically implement ToString
let num_str: String = 42.to_string();
```

## The Orphan Rule and Coherence

Rust allows implementing a trait on a type **only if at least one is local to the crate**. This prevents conflicting implementations across crates and maintains coherence:

```rust
// In crate A:
impl Display for SocialPost { }  // OK: SocialPost is local

impl Display for Vec<T> { }  // ERROR: both Display and Vec are external

// But you can wrap externals in a newtype:
struct Wrapper(Vec<T>);
impl Display for Wrapper { }  // OK: Wrapper is local
```

## Marker Traits

**Marker traits** contain no methods but signal compiler capabilities or semantic properties:

### `Copy`

Types implementing `Copy` are bit-copied on assignment instead of moved. Requires `Clone`:

```rust
#[derive(Copy, Clone)]
struct Point(i32, i32);

let p1 = Point(1, 2);
let p2 = p1;  // Copy: p1 still usable
```

### `Send` and `Sync`

- **`Send`**: Safe to move across thread boundaries (no internal shared references that aren't synchronized)
- **`Sync`**: Safe to reference from multiple threads (interior mutability is thread-safe)

```rust
fn spawn_thread<T: Send + 'static>(val: T) {
    thread::spawn(move || { /* val is moved */ });
}
```

### `Sized`

Indicates the type size is known at compile time. Most generics implicitly require `Sized`:

```rust
fn generic<T>(x: T) { }  // T: Sized by default

fn generic<T: ?Sized>(x: &T) { }  // ?Sized: T may be unsized (e.g., dyn Trait)
```

### `Unpin`

Opposite of `Pin`; indicates the value can be safely moved after pinning (the default for most types).

## Derive Macros and Automatic Trait Implementation

`#[derive(...)]` generates common trait implementations:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
struct User {
    id: u32,
    name: String,
}
```

Derive macros work only on types whose fields implement those traits. Custom derive macros can implement arbitrary traits.

## Conditional Trait Implementation

Traits can be implemented conditionally on type bounds:

```rust
impl<T: Display + PartialOrd> Pair<T> {
    fn cmp_display(&self) {
        if self.x >= self.y {
            println!("Largest: {}", self.x);
        }
    }
}
```

This creates a method available only when `T` implements both `Display` and `PartialOrd`.

## Object-Safe vs Non-Object-Safe Traits

**Object-safe traits** can be converted to `dyn Trait`:

```rust
pub trait Draw {
    fn draw(&self);  // Object-safe: uses &self, no Self return type
}
```

**Non-object-safe traits** cannot:

```rust
pub trait Clone {
    fn clone(&self) -> Self;  // Non-object-safe: returns Self
}

// dyn Clone is an error
```

## Best Practices

1. **Use trait bounds to express generic constraints**, not to force users into runtime dispatch
2. **Prefer `impl Trait` for return types** of single concrete types (compile-time, zero-cost)
3. **Use `dyn Trait` when types vary and cannot be known at compile time** (runtime cost)
4. **Use associated types for single-type bindings**, generics for many potential types
5. **Leverage blanket implementations** to avoid repetitive boilerplate
6. **Respect the orphan rule**; avoid type wrapper patterns unless necessary
7. **Understand marker trait semantics** (`Send`, `Sync`, `Copy`, `Sized`) before using them

## See Also

- `language-rust-ownership.md` — Interior mutability and `Sized`
- `language-rust-async.md` — Traits in async contexts (Future, Stream)
- `api-design.md` — Trait-based API design principles