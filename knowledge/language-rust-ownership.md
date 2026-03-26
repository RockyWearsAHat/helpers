# Rust Ownership — Deep Dive into Borrowing, Interior Mutability, and Unsafe Memory

## Overview

Rust's memory safety model rests on three pillars: ownership (unique responsibility), borrowing (temporary access), and lifetimes (compile-time scope verification). This prevents data races, use-after-free, and double-free without runtime garbage collection. Understanding when and why to use interior mutability, reference counting, and unsafe patterns separates novice from advanced Rust.

## Ownership Rules (Recap)

1. Each value has exactly one owner.
2. When the owner is dropped, the value is deallocated.
3. Ownership can be moved (transferred) or borrowed (referenced).

```rust
let s = String::from("hello");  // s owns the heap allocation
let t = s;                      // s moved to t; s invalid
// println!("{s}");            // ❌ Compile error
```

## Borrowing: Shared and Mutable References

References allow temporary, non-owning access without moving.

### Immutable Borrowing (`&T`)

Many references can exist simultaneously. The value cannot be mutated while borrowed:

```rust
fn print_length(s: &str) { println!("{}", s.len()); }

let s = String::from("hello");
print_length(&s);  // Borrow s
print_length(&s);  // Borrow again — allowed
println!("{}", s);  // s still valid
```

**Compiler insight:** If you can't prove a mutation happens, the reference is safe. Reads are cheap; the compiler optimizes immutable borrow chains.

### Mutable Borrowing (`&mut T`)

Exactly one mutable reference can exist at a time. No immutable references can coexist:

```rust
let mut x = 5;
let r1 = &mut x;  // Mutable borrow
let r2 = &mut x;  // ❌ Compile error: r1 still active
r1 += 1;
// But after r1 last used:
let r2 = &mut x;  // ✅ Now allowed
*r2 += 1;
```

**Why?** A mutable borrow guarantees exclusive access. No other code can observe stale state mid-mutation.

## Lifetimes: Scope Verification

Lifetimes are compiler annotations that prove references don't outlive their referents:

```rust
// Explicit lifetime: 'a
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}

// Lifetime elision (implicit): common patterns abbreviated
fn first(items: &Vec<i32>) -> Option<&i32> {
    items.first()  // Compiler infers: fn(&Vec<i32>) -> Option<&i32>
}

// Lifetime mismatch — compile error:
let r;
{
    let x = 5;
    r = &x;  // ❌ r lives longer than x
}
println!("{r}");  // Would be dangling reference
```

### Lifetime Elision Rules

The compiler infers lifetimes in common cases:

1. **Each input reference gets its own lifetime.** `fn(x: &T, y: &U)` → `fn(x: &'a T, y: &'b U)`
2. **If one input, its output lifetime is that input's.** `fn(x: &T) -> &S` → `fn(x: &'a T) -> &'a S`
3. **For methods on `&self`, `&mut self`:** output lifetime is `self`'s lifetime.

```rust
impl MyString {
    fn as_str(&self) -> &str {  // Inferred: &'a str
        &self.data
    }
}
```

### Non-Lexical Lifetimes (NLL)

Rust 2018+ refines lifetime scopes: a borrow ends when last used, not at the block boundary:

```rust
let mut x = 5;
let r = &x;
println!("{}", r);  // Last use of r

let r2 = &mut x;     // ✅ Allowed — r is no longer live
*r2 += 1;
```

Pre-NLL this would be an error (r's lifetime extended to block end).

## Interior Mutability: Controlled Mutation Behind Immutable References

Sometimes the borrow checker is too strict. Interior mutability allows mutation through `&T` (not `&mut T`), but **you** verify safety:

### `Cell<T>` — Single-threaded, Copy-only mutation

`Cell` doesn't track borrows at runtime; it requires `T: Copy` or moves. Use for value types:

```rust
use std::cell::Cell;

let x = Cell::new(5);
x.set(10);               // Mutate through &
let val = x.get();       // Extract Copy of value
println!("{}", val);     // 10
```

**Tradeoff:** Cannot borrow `T` mutably; can only replace wholesale.

### `RefCell<T>` — Single-threaded, checked borrow at runtime

`RefCell` enforces the borrowing rules at runtime (panics on violation):

```rust
use std::cell::RefCell;

let s = RefCell::new(String::from("hello"));

// Immutable borrow
{
    let r1 = s.borrow();
    let r2 = s.borrow();  // Multiple immutable borrows OK
    println!("{}, {}", *r1, *r2);
}

// Mutable borrow (exclusive)
{
    let mut r = s.borrow_mut();  // Panics if immutable borrow still active
    r.push_str(" world");
}
```

**Tradeoff:** Runtime overhead; can panic if misused. Use when the compiler's static analysis is insufficient (e.g., recursive struct mutations, callbacks).

### `Mutex<T>` — Thread-safe interior mutability

For multi-threaded code, `std::sync::Mutex` provides interior mutability with synchronization:

```rust
use std::sync::Mutex;
use std::thread;

let counter = Mutex::new(0);
let mut handles = vec![];

for _ in 0..10 {
    let c = Mutex::clone(&counter);  // Arc needed in practice (see below)
    handles.push(thread::spawn(move || {
        let mut num = c.lock().unwrap();
        *num += 1;
    }));
}

for h in handles { h.join().unwrap(); }
println!("{}", *counter.lock().unwrap());  // 10
```

**Tradeoff:** Lock overhead; poisoning (lock marked unusable if panicked). Use `parking_lot` for faster, non-poisoning locks.

### `UnsafeCell<T>` — Lowest-level interior mutability

`UnsafeCell` is the primitive: it allows `&T` to produce `*mut T` (unsafe pointer). All other interior mutability types are built on it:

```rust
use std::cell::UnsafeCell;

let x = UnsafeCell::new(5);
unsafe {
    *x.get() = 10;  // Raw pointer mutation
}
println!("{}", x.into_inner());  // 10
```

**No runtime checks.** You prove correctness.

## Reference Counting: Shared Ownership

`Rc<T>` (single-threaded) and `Arc<T>` (atomic, thread-safe) allow multiple owners. When the last owner drops, the value is deallocated:

```rust
use std::rc::Rc;

let list = Rc::new(vec![1, 2, 3]);
let owner1 = list.clone();  // Shallow clone; increments refcount
let owner2 = list.clone();

println!("{:?}", owner1);
println!("{:?}", owner2);
// list, owner1, owner2 all point to same heap allocation
// Deallocated when all are dropped
```

**Combined with interior mutability:**

```rust
use std::rc::Rc;
use std::cell::RefCell;

let shared = Rc::new(RefCell::new(vec![1, 2, 3]));
let owner1 = shared.clone();
let owner2 = shared.clone();

owner1.borrow_mut().push(4);
println!("{:?}", owner2.borrow());  // [1, 2, 3, 4]
```

**Thread-safe variant:**

```rust
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

let counter = Arc::new(Mutex::new(0));
let mut handles = vec![];

for _ in 0..10 {
    let c = Arc::clone(&counter);
    handles.push(thread::spawn(move || {
        *c.lock().unwrap() += 1;
    }));
}
```

Arc is zero-cost in single-threaded code (no atomic operations).

## Unsafe and Raw Pointers

`unsafe` blocks allow operations the compiler can't verify: dereferencing raw pointers, calling FFI, mutating statics, etc.

```rust
let x = 5;
let raw_ptr = &x as *const i32;   // Take address as raw pointer

unsafe {
    println!("{}", *raw_ptr);      // Dereference — undefined behavior if invalid
}
```

**Six unsafe operations:**
1. Dereference raw pointers
2. Call unsafe functions
3. Access/modify mutable static items
4. Implement unsafe traits
5. Access fields of a union
6. Inline assembly

**Philosophy:** Unsafe is not "unchecked code." It's "code whose safety I'm verifying manually." Encapsulate it in safe abstractions (like library authors do).

## Pin<T>: Self-Referential Structs and Async

`Pin` prevents moving a value after it's been pinned (when it contains self-references or async state machines):

```rust
use std::pin::Pin;

let mut x = 5;
let mut pinned = Pin::new(&mut x);

// Can't move pinned.
// Compiler prevents: std::mem::replace, std::mem::swap, take ownership
```

**Async use case:** Futures often contain self-references. `Pin` ensures they're never moved once polled:

```rust
async fn example() {
    let x = 5;
    function_taking_ref(&x).await;  // Requires Pin to guarantee x won't move
}
```

## Best Practices

- **Default to owned types.** Move semantics are fast; borrowing is an optimization.
- **Prefer `&T` and `&mut T` to interior mutability.** Interior mutability is for special cases.
- **Use `Arc<Mutex<T>>` for thread-shared mutable state,** not `Arc<RefCell<T>>` (panic, not sync).
- **Avoid unsafe unless necessary.** If you're using it, document the invariants explicitly.
- **`Rc` is rarely the answer.** Consider whether ownership sharing makes sense (often signals design issues).

## See Also

- **language-rust.md** — Rust idioms and error handling
- **language-rust-async.md** — Async, futures, and structured concurrency
- **type-systems-theory.md** — Type system concepts (variance, subtyping)