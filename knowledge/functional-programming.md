# Functional Programming

## Core Principles

Functional programming (FP) is a paradigm where computation is treated as the evaluation of mathematical functions. State changes and mutable data are avoided.

### 1. Pure Functions

A function is pure if:

- Given the same input, it always returns the same output (deterministic).
- It has no side effects (doesn't modify external state, do I/O, etc.).

```python
# Pure
def add(a, b):
    return a + b

# Impure (depends on external state)
total = 0
def add_to_total(x):
    global total
    total += x  # Side effect: modifies external state
    return total
```

**Why pure functions matter:**

- **Testable**: No mocking, no setup. Input → output.
- **Cacheable**: Same input = same output → memoize freely.
- **Parallelizable**: No shared state → safe to run concurrently.
- **Composable**: Combine small pure functions into larger ones.

### 2. Immutability

Data never changes after creation. Instead of modifying, create new values.

```javascript
// ❌ Mutation
const user = { name: "Alice", age: 30 };
user.age = 31; // Mutated in place

// ✅ Immutable update
const updated = { ...user, age: 31 }; // New object, original unchanged
```

```rust
// Rust enforces immutability by default
let x = 5;
// x = 6;  // ❌ Compile error
let x = 6;  // ✅ Shadowing creates a new binding
```

**Benefits:**

- No race conditions in concurrent code.
- Easy to reason about — data doesn't change under you.
- Time-travel debugging (keep old states).
- Structural sharing makes it efficient (persistent data structures).

### 3. First-Class & Higher-Order Functions

Functions are values — they can be passed as arguments, returned from other functions, and stored in variables.

```javascript
// Higher-order function: takes a function as argument
function filter(arr, predicate) {
  const result = [];
  for (const item of arr) {
    if (predicate(item)) result.push(item);
  }
  return result;
}

const adults = filter(people, (person) => person.age >= 18);

// Higher-order function: returns a function
function multiplier(factor) {
  return (n) => n * factor;
}

const double = multiplier(2);
const triple = multiplier(3);
double(5); // 10
triple(5); // 15
```

### 4. Function Composition

Build complex behavior by combining simple functions.

```javascript
// Manual composition
const process = (x) => format(validate(parse(x)));

// Compose utility
const compose =
  (...fns) =>
  (x) =>
    fns.reduceRight((v, f) => f(v), x);
const pipe =
  (...fns) =>
  (x) =>
    fns.reduce((v, f) => f(v), x);

const processUser = pipe(normalize, validate, enrichWithDefaults, save);
```

```haskell
-- Haskell: composition is a built-in operator
process = format . validate . parse
```

### 5. Currying & Partial Application

Transform a multi-argument function into a chain of single-argument functions.

```javascript
// Curried function
const add = (a) => (b) => a + b;
const add5 = add(5);
add5(3); // 8

// Partial application
const fetchFromAPI = (baseUrl) => (endpoint) => (params) =>
  fetch(`${baseUrl}${endpoint}?${new URLSearchParams(params)}`);

const fetchFromMyAPI = fetchFromAPI("https://api.example.com");
const fetchUsers = fetchFromMyAPI("/users");
fetchUsers({ page: 1, limit: 10 });
```

```haskell
-- Haskell: all functions are automatically curried
add :: Int -> Int -> Int
add a b = a + b

add5 = add 5    -- Partial application
add5 3           -- 8
```

### 6. Declarative Over Imperative

Describe _what_ to compute, not _how_ to compute it step by step.

```python
# Imperative (how)
result = []
for item in items:
    if item.price > 100:
        result.append(item.name.upper())

# Declarative (what)
result = [item.name.upper() for item in items if item.price > 100]
```

```javascript
// Imperative
let total = 0;
for (let i = 0; i < orders.length; i++) {
  if (orders[i].status === "completed") {
    total += orders[i].amount;
  }
}

// Declarative
const total = orders
  .filter((o) => o.status === "completed")
  .reduce((sum, o) => sum + o.amount, 0);
```

## Advanced Concepts

### Monads (Practical Explanation)

A monad is a design pattern for chaining operations that involve context (nullability, errors, async, lists, etc.). **Don't overcomplicate it: it's just a way to compose functions that return wrapped values.**

```typescript
// The "Maybe" monad: chain operations that might be null
// Without monadic thinking:
const street = user && user.address && user.address.street;

// With monadic chaining (Optional chaining IS a monad operation):
const street = user?.address?.street;

// Result monad: chain operations that might fail
fetchUser(id)
  .andThen((user) => fetchPosts(user.id))
  .andThen((posts) => renderTimeline(posts))
  .unwrapOr(defaultTimeline);
```

```rust
// Rust's ? operator is monadic chaining for Result
fn process() -> Result<Output, Error> {
    let data = read_file()?;      // Returns Err early if fails
    let parsed = parse(data)?;     // Same
    let result = transform(parsed)?;
    Ok(result)
}
```

```haskell
-- Haskell: the classic monad example
-- do notation desugars to >>= (bind) chains
main :: IO ()
main = do
    name <- getLine        -- IO monad
    let greeting = "Hello, " ++ name
    putStrLn greeting
```

**The monad laws** (for the mathematically inclined):

1. **Left identity**: `return a >>= f  ≡  f a`
2. **Right identity**: `m >>= return  ≡  m`
3. **Associativity**: `(m >>= f) >>= g  ≡  m >>= (\x -> f x >>= g)`

### Lazy Evaluation

Compute values only when needed. Enables working with infinite data structures.

```haskell
-- Haskell is lazy by default
naturals = [1..]           -- Infinite list
take 5 naturals            -- [1, 2, 3, 4, 5] — only computes 5 elements

-- Fibonacci as an infinite list
fibs = 0 : 1 : zipWith (+) fibs (tail fibs)
take 10 fibs               -- [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]
```

```python
# Python generators are lazy
def fibonacci():
    a, b = 0, 1
    while True:
        yield a
        a, b = b, a + b

from itertools import islice
list(islice(fibonacci(), 10))  # [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]
```

```rust
// Rust iterators are lazy
let result: Vec<i32> = (0..)
    .filter(|n| n % 2 == 0)
    .map(|n| n * n)
    .take(5)
    .collect();  // [0, 4, 16, 36, 64]
```

### Referential Transparency

An expression is referentially transparent if it can be replaced with its value without changing program behavior.

```haskell
-- Referentially transparent
let x = 2 + 3
-- We can replace x with 5 everywhere and nothing changes

-- NOT referentially transparent
let x = readLine()  -- Different every time
```

This property enables:

- **Equational reasoning**: Prove things about code algebraically.
- **Compiler optimizations**: Common subexpression elimination, memoization.
- **Fearless refactoring**: Extract/inline functions without fear.

## FP in Different Languages

| Feature          | Haskell        | Rust              | Scala           | Kotlin            | JS/TS      | Python     | Java        |
| ---------------- | -------------- | ----------------- | --------------- | ----------------- | ---------- | ---------- | ----------- |
| Pure functions   | Enforced       | Convention        | Convention      | Convention        | Convention | Convention | Convention  |
| Immutability     | Default        | Default           | Preferred       | `val` default     | `const`    | Convention | `final`     |
| Pattern matching | ✅ Native      | ✅ Native         | ✅ Native       | ✅ `when`         | ❌ Limited | ✅ 3.10+   | ✅ 21+      |
| ADTs (sum types) | ✅ Native      | ✅ `enum`         | ✅ `sealed`     | ✅ `sealed`       | ✅ Unions  | ❌ Manual  | ✅ `sealed` |
| HO functions     | ✅             | ✅ Closures       | ✅              | ✅ Lambdas        | ✅ Arrows  | ✅ Lambdas | ✅ Lambdas  |
| Lazy evaluation  | Default        | Iterators         | `lazy`          | `lazy`/`Sequence` | Generators | Generators | Streams     |
| Monads           | ✅ First-class | `Result`/`Option` | ✅              | `Result`          | `Promise`  | ❌ Manual  | `Optional`  |
| Currying         | Auto           | Manual            | Auto            | Manual            | Manual     | Manual     | Manual      |
| Type classes     | ✅             | Traits            | Implicits/Given | ❌                | ❌         | ❌         | ❌          |
| TCO              | ✅             | Limited           | `@tailrec`      | `tailrec`         | ❌         | ❌         | ❌          |

## Practical FP Guidelines

1. **Start with pure functions.** Push side effects to the edges of your system.
2. **Use immutable data structures by default.** Mutate only when performance demands it.
3. **Prefer map/filter/reduce over loops.** Declarative code is easier to understand.
4. **Use types to model your domain.** Sum types (unions) for states, product types (records) for data.
5. **Compose small functions.** Each function does one thing well.
6. **Avoid shared mutable state.** If you must share, use immutable data or controlled mutation.
7. **Don't go full FP in a non-FP language.** Use FP ideas pragmatically. You don't need monads in Python.

---

_Sources: Structure and Interpretation of Computer Programs (Abelson/Sussman), Functional Programming in Scala (Chiusano/Bjarnason), Haskell Programming from First Principles, Professor Frisby's Mostly Adequate Guide to Functional Programming, Real World Haskell_
