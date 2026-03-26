# Type-Level Programming — Computation at Compile Time

## Overview

**Type-level programming** treats types as values that can be computed. Rather than hardcoding behavior at runtime, you encode logic in the type system itself. The type checker becomes a proof engine. When code compiles, you've proven invariants hold.

This inverts the traditional type system: rather than types being constraints on values, types become the primary domain of expression.

## Core Intuition

In a simple type system:
```typescript
function add(a: number, b: number): number {
  return a + b;
}
```

The types just label what add expects. Computation is at runtime.

In type-level programming:
```typescript
// TypeScript: encode list length in the type
type Length<T extends any[]> = T['length'];

const x: Length<[1, 2, 3]> = 3; // type is computed at compile time
const y: Length<[1, 2]> = 2;
```

The type system is **Turing-complete** (or nearly so). You can compute with types just as with values, except:
- Computation happens at compile time
- Results must be verifiable by the compiler
- Errors appear at compile time, not runtime

## Dependent Types

### The Concept

A **dependent type** is a type whose value depends on a runtime value (or computes internally).

```
Type = f(value)
```

Rather than:
```
Type Int = Integer
Type String = String
```

You have:
```
Type List(n) = list with exactly n elements
Type Matrix(m, n) = matrix with m rows and n columns
```

The key: **the type captures information about the value**, not just its shape.

### Idris: Pure Dependent Types

**Idris** is designed for dependently typed programming. Types can mention values.

```idris
-- Vectors (lists with length encoded in type)
data Vect : Nat -> Type -> Type where
    Nil  : Vect Z a
    (::) : a -> Vect n a -> Vect (S n) a

-- Append respects lengths: V(m) ++ V(n) = V(m+n)
append : Vect m a -> Vect n a -> Vect (m + n) a
append Nil ys = ys
append (x :: xs) ys = x :: (append xs ys)

-- Type mismatch caught at compile time:
-- append [1,2] [3] : Vect 3 Int  (correct; 2 + 1 = 3)
-- append [1,2] [3,4] would fail; tries to unify Vect 2 with Vect 4
```

In Idris, the function signature **proves** the property: you can't write a function with that type unless it correctly handles lengths.

### Agda: Proof Assistant + Language

**Agda** is a proof assistant where programs are proofs. It goes further than Idris: you write proofs of mathematical theorems, and running the proof is computation.

```agda
-- Proof that addition is commutative
comm : ∀ (m n : ℕ) → m + n ≡ n + m
comm zero n = sym (identity n)
comm (suc m) n = cong suc (comm m n)
```

Agda enables you to **verify correctness statically**. No runtime errors possible for proven properties.

### Dependent Types in Practice

Dependent types are powerful but come with costs:
- **Typechecking is undecidable**: in general, checking if an expression inhabits a dependent type is uncomputable
- **Type errors are verbose**: errors involve proofs; messages are hard to parse
- **Tooling is immature**: IDEs are limited; refactoring is risky
- **Ecosystem is small**: fewer libraries, academic focus

Current use:
- **Theorem provers**: Coq, Lean (used in formal mathematics, not production systems)
- **Research**: exploring type system boundaries
- **Embedded proofs**: critical systems might use dependent types for specific invariants

## Generalized Algebraic Data Types (GADTs)

### The Concept

**GADTs** are a middle ground between simple algebraic data types (ADTs) and full dependent types. They allow the return type of a constructor to vary.

```haskell
-- Standard ADT: all constructors return the same type
data Maybe a = Nothing | Just a

-- GADT: constructors can specify return type more precisely
data Value where
    IntVal :: Int -> Value Int
    StrVal :: String -> Value String
    BoolVal :: Bool -> Value Bool
```

Wait, that's not quite right (in Haskell, you can't have multi-type values like that in a standard data type). Instead:

```haskell
-- Proper GADT encoding: type-safe evaluation
data Expr a where
    IntLit :: Int -> Expr Int
    StrLit :: String -> Expr String
    BoolLit :: Bool -> Expr Bool
    Add :: Expr Int -> Expr Int -> Expr Int

-- Evaluation is safe: can't write Add (StrLit "x") (BoolLit True)
eval :: Expr a -> a
eval (IntLit n) = n
eval (StrLit s) = s
eval (BoolLit b) = b
eval (Add x y) = eval x + eval y
```

The type parameter `a` of `Expr a` is **refined** by the constructor. `IntLit` returns `Expr Int`, not `Expr a`.

Advantages:
- **Type safety**: pattern matching is exhaustive and correct by construction
- **No runtime type checks**: the type system proves cases can't occur
- **Composable**: GADTs work in any language with type system expressivity

### GADTs vs. Dependent Types

| Aspect | GADT | Dependent Type |
|--------|------|-----------------|
| **Type return** | Refined by constructor | Can depend on value |
| **Complexity** | Moderate | High |
| **Tool support** | Better (Haskell, Scala) | Limited (Idris, Agda) |
| **Decidability** | Decidable | Undecidable (needs help) |
| **Expressivity** | Good for ADTs | Can prove arbitrary properties |

## Phantom Types and Tagged Types

### Phantom Types

A **phantom type** is a type parameter that never appears in the value; it's only for compile-time tracking.

```haskell
-- Type parameter 's' is phantom (never used in the value)
data Tagged s a = Tagged a

-- Use: track state or context without runtime cost
data Sealed
data Open

data File s = File FilePath

-- Open file is different type from sealed file
openFile :: FilePath -> IO (File Open)
readFile :: File Open -> IO String
sealFile :: File Open -> IO (File Sealed)
```

At runtime, `File Open` and `File Sealed` are identical; the difference is compile-time proof.

### Branded Types (TypeScript)

In TypeScript (and JavaScript), you can't use types at runtime, so branding is done with symbols:

```typescript
type UserId = string & { readonly __brand: "UserId" };

function brand<T>(x: string): UserId {
    return x as UserId;
}

function getUser(id: UserId) {
    // function won't accept plain string; prevents type confusion
}

getUser("42");                    // error: not a UserId
getUser(brand("42"));             // ok
```

Branded types catch errors like passing user ID as a password (both are strings, but types differ).

## Type Families (Haskell)

**Type families** are functions mapping types to types.

```haskell
-- Open type family (can be extended)
type family IndexOf (xs :: [Type]) (x :: Type) :: Nat where
    IndexOf (x ': xs) x = 0
    IndexOf (y ': xs) x = 1 + IndexOf xs x
    IndexOf [] x = TypeError (Text "Not found")

-- Use: get position of a type in a list
pos :: IndexOf [Int, String, Bool] String
pos = 1  -- Int is 0, String is 1
```

Type families enable:
- **Type-level computation**: mapping types to types
- **Type-level predicates**: deciding whether a type satisfies a condition
- **Functional relationships**: generic instances based on type relationships

## Refinement Types

**Refinement types** take a base type and add a logical predicate.

```
type EvenInt = { n : Int | n % 2 == 0 }
type NonEmptyString = { s : String | length(s) > 0 }
```

A value is only in the refined type if it satisfies the predicate.

### LiquidHaskell

**LiquidHaskell** brings refinement types to Haskell via SMT solver integration:

```haskell
{-@ type Nat = Int { n | n >= 0 } @-}

{-@ incr :: Nat -> Nat @-}
incr x = x + 1  -- type checker verifies this preserves Nat

{-@ div :: Int -> {d : Int | d != 0} -> Int @-}
div x d = x / d  -- requires non-zero denominator
```

Refinement types encode preconditions, postconditions, and invariants, verified by an SMT solver.

### Modern Use

- **TypeScript**: libraries like `fp-ts` use branded types (lightweight refinement)
- **Typescript-esoteric**: some libraries use conditional types + overloads to approximate refinements
- **Go**: `newtype`-like patterns with interfaces encode refinements behaviorally
- **Python**: runtime contracts (pydantic, beartype) simulate refinement checking

## Effect Systems

**Effect systems** track side effects in types, making it explicit what a function can do.

### Koka Language

```koka
// 'e' is effect variable; ' is effect quantifier
fun greet(name : string) : console ()
    println("Hello, " + name)

// Can be parametric over effects
fun map-effect(f: (a) -> e b, xs : list<a>) : e list<b>
    match xs
        Cons(x, rest) -> Cons(f(x), map-effect(f, rest))
        Nil -> Nil
```

Koka uses **effect rows**: `f : (int) -> <div, st<a>> int` means `f` can divide (may fail) and access state `a`.

### Benefits

- **Purity tracking**: compiler knows which functions are pure
- **Composition**: pure functions compose safely; effectful functions are explicit
- **Optimization**: pure functions can be reordered, parallelized, memoized

**Downside**: effect systems add complexity. Most mainstream languages don't have them; Go's error handling is a lightweight alternative.

## Linear Types: Rust's Ownership Model

**Linear types** enforce that a value is used exactly once. Rust embeds this in ownership rules:

```rust
let x = String::from("hello");
let y = x;          // x moved; no longer accessible
println!("{}", x);  // error: x was moved

let s = String::from("world");
println!("{}", s);  // ok: s is read (Copy types allow this)
println!("{}", s);  // also ok
```

Linear types guarantee:
- **No use-after-free**: value consumed, so can't be used again
- **No double-free**: each resource has exactly one owner
- **Implicit cleanup**: when a linear value goes out of scope, its resources are freed

Rust's ownership system is type-level enforcement of resource management. Types encode **how resources flow**, and the compiler verifies correctness.

## Session Types

**Session types** describe the valid sequences of messages in a protocol.

```haskell
-- A server session: receive Int, send String response
data ServerSession = ReceiveInt Int ServerSession
                   | End

-- Type-safe communication: the type enforces protocol order
server :: ServerSession
server = ReceiveInt 42 End
```

Session types enable:
- **Protocol verification**: compiler checks message orders are valid
- **Deadlock freedom**: static analysis can prove protocols won't deadlock
- **Implementation guidance**: the type tells you what function should do

Session types are research-heavy; limited in production (some Scala libraries, academic languages). The idea is powerful but adoption is hindered by complexity.

## Practical Type-Level Programming

### When It Makes Sense

- **Matrix/vector libraries**: encode dimensions to prevent shape mismatches
- **Parsing/DSLs**: GADTs elegantly express syntax trees with type safety
- **Database queries**: encode schema in types; invalid queries won't compile
- **Resource management**: linear types (Rust) guarantee cleanup
- **Proofs of correctness**: critical systems (avionics, cryptography)

### Tools and Ecosystems

- **Haskell**: most mature; type families, GADTs, rank-N types built in
- **TypeScript**: branded types + conditional types + operator overloading (hacky, but works)
- **Scala**: GADTs, implicit evidences, shapeless library
- **Rust**: ownership is implicit type-level programming; trait system enables generic proofs
- **Java**: generics are weak (erasure); some advanced libraries use reflection + annotations

### Pitfalls

- **Compilation time explodes**: type checker combinatorial search can be slow
- **Error messages are cryptic**: types get deeply nested; errors are hard to parse
- **Refactoring is risky**: changing a type signature may invalidate many proofs elsewhere
- **Learning curve**: type-level programming takes time; not every developer is comfortable
- **Maintenance burden**: code is more abstract; future maintainers may not understand the design

## See Also

- [Type Systems — Theory & Practical Application](type-systems-theory.md) — foundation
- [Functional Programming](functional-programming.md) — languages with advanced types
- [Language Haskell Conventions and Idioms](language-haskell.md) — GADT and type family examples
- [Language Rust Conventions and Idioms](language-rust.md) — ownership and linear types