# Type Checking — Inference, Unification, Polymorphism & Soundness

## Overview

**Type checking** is the process of verifying that expressions in a program are used consistently with their intended types. Type checking spans static verification (at compile time) vs. dynamic (at runtime), explicit annotations (C, Java) vs. inference-based (Haskell, OCaml), and simple (monomorphic) vs. sophisticated (polymorphic, dependent, gradual).

A type checker verifies **type safety**: if a value has type `T`, operations assumed on type `T` will not fail. Modern type systems combine **type inference** (deduce types from usage), **unification** (solve type equations), **constraint generation** (extract type constraints), and **polymorphism** (generalize over types).

## Core Algorithm: Hindley-Milner (HM) Type Inference

### Philosophy

Hindley-Milner inference decouples **type deduction** from **implementation**. Instead of annotating every function and variable, the type checker observes how they are used and infers consistent types. This is feasible because of **parametric polymorphism**: a single function like `length` works for any list type.

### Algorithm W (Robinson, Damas & Milner)

Algorithm W is the canonical HM implementation. It works in three phases:

1. **Constraint Generation ("type inference collecting")**: Traverse the AST and emit type equations from usage patterns.
   - Function call `f(x)` → emit "type of `x` = type of `f`'s parameter"
   - Arithmetic `x + y` → emit "type of `x` is numeric", "type of `y` is numeric"
   - Pattern `if c then e1 else e2` → emit "type of `c` is bool", "types of `e1` and `e2` are equal"

2. **Type Variable Binding**: Each expression gets a fresh type variable (e.g., `a`, `b`, `c`). As constraints accumulate, variables become more specific.
   - Example: `let id x = x` generates "type of `id` = `a -> a`" (polymorphic identity).

3. **Unification (constraint solving)**: Find the **most general unifier (MGU)**—an assignment of type variables that satisfies all constraints.
   - Example: constraints `a = int`, `b = a`, `c = b` → unifier is `a = int, b = int, c = int`.

### Principle of Unification

Unification finds if two type expressions can be made identical by substituting type variables.

```
unify(T1, T2):
  if T1 and T2 are identical: return {}
  if T1 is a variable: return {T1 -> T2}
  if T2 is a variable: return {T2 -> T1}
  if T1 = f(A1..An) and T2 = f(B1..Bn):
    unify component-wise: {A1 unify B1, A2 unify B2, ...}
  else: fail (type mismatch)
```

The **occurs check** prevents infinite types (`a = a -> b` is rejected).

### Example: Type Inference of List Operations

```
let length lst = if is_empty lst then 0 else 1 + length (tail lst)

-- Constraint generation:
-- is_empty lst -> true/false implies lst is list-like
-- 1 + ... implies result is numeric
-- recursive call: length has type (List a -> int)

-- Result type: length :: ∀a. [a] → int
-- (forall a, function takes list of any type, returns int)
```

## Constraint Generation & Polymorphism

### Polymorphic Type Variables

Hindley-Milner distinguishes **monomorphic** and **polymorphic** type variables:

- **Monomorphic (`a`)**: A type variable that must be resolved to a concrete type (e.g., `int`).
- **Polymorphic (`∀a`)**: A type variable universally quantified; instantiated fresh for each use.

```
id x = x                    -- id : ∀a. a → a (polymorphic)
const x y = x               -- const : ∀a b. a → b → a
apply f x = f x             -- apply : ∀a b. (a → b) → a → b
```

When `id` is called on both `int` and `string`, the type checker instantiates:
- `id 42` → `a = int`
- `id "hello"` → `a = string`

### The Occurs Check

Without the occurs check, unification can produce nonsensical types:

```
-- Without occurs check: a = a -> b resolves to infinite type
-- With occurs check: unification fails (a occurs in a -> b)
```

## Polymorphism: Parametric vs. Ad-Hoc

### Parametric Polymorphism

**Parametric polymorphism** is what HM provides: a single function works uniformly across all types matching a type variable.

```
map f xs                    -- ∀a b. (a → b) → [a] → [b]
-- Works for any f, any type a, any type b
```

### Ad-Hoc Polymorphism (Overloading)

**Ad-hoc polymorphism** allows different implementations per type. Languages achieve this via:

- **Function overloading** (C++, Java): multiple `print(int)`, `print(string)` definitions.
- **Type classes** (Haskell): constraints like `∀a. Eq a => a → a → Bool` use constraint **resolution**, not unification.
- **Protocols** (Swift): structural typing; a type conforms to a protocol if it implements required methods.

```
-- Haskell type class (constraint-based ad-hoc polymorphism)
class Eq a where
  (==) :: a → a → Bool

elem :: Eq a => a → [a] → Bool
-- elem works for any type 'a' that implements Eq
```

## Bidirectional Type Checking

Bidirectional checking combines **synthesis** and **checking** modes:

- **Synthesis** (→): infer type from an expression (`expr : type`)
- **Checking** (←): verify an expression matches an expected type (`type => expr`)

This hybrid approach reduces annotation burden while handling cases where bottom-up inference is ambiguous.

```
-- Synthesis: infer from [1, 2, 3]
-- [1, 2, 3] : [int]

-- Checking: verify \x -> x + 1 matches int -> int
-- int -> int <= (\x -> x + 1) requires x: int, result: int
```

**Example (TypeScript-style):**

```typescript
const filter = <T>(pred: (x: T) => boolean, xs: T[]) => ...
const nums = [1, 2, 3];

// Synthesis: infer T = int from argument
filter(x => x > 2, nums)  // T inferred as int, x: int

// Checking: annotated arrow; type of param flows downward
filter((x: number) => x > 2, nums)
```

Use cases:
- Function literals in typed contexts: `map (λx. x + 1)`
- Higher-order functions: compiler knows expected arg types
- Overload resolution: expected return type disambiguates

## Type Reconstruction & Rigid Type Variables

**Type reconstruction** is the process of recovering types from inference where source code omits them. In languages like OCaml, source code reveals no type signatures, yet types are fully reconstructed.

```ocaml
(* No annotations *)
let id x = x            (* inferred: id : 'a → 'a *)
let apply f x = f x     (* inferred: apply : ('a → 'b) → 'a → 'b *)
let compose f g x = f (g x)  (* inferred: compose : ('b → 'c) → ('a → 'b) → 'a → 'c *)
```

**Rigid type variables** prevent accidental instantiation. In generic instantiation, rigidity ensures a type variable stays abstract:

```
-- filter : ∀a. (a → bool) → [a] → [a]
-- When applied to (int → bool), the 'a is rigid within filter's
-- implementation; it cannot be unified with different types
```

## Gradual Typing & Type Graduality

**Gradual typing** allows coexistence of static and dynamic code without sacrificing either:

- **Static fragment**: fully typed, type checker verifies safety.
- **Dynamic fragment**: `dynamic` type; casts to/from `dynamic` are permitted at runtime.
- **Boundary checking**: casts across static/dynamic boundaries can fail at runtime.

```typescript
// TypeScript (partial gradual typing via 'any')
function greet(name: string) {
  return `Hello, ${name}`;
}

const x: any = 42;
greet(x);  // Type error suppressed (cast to any); runtime fails
```

Gradual typing trades full static safety for flexibility:
- No cast overhead in fully-typed code
- Gradual migration from dynamic to static
- Runtime errors at boundaries (gradual == gradual "casts")

**Soundness tradeoff**: A gradual system may not be fully sound (well-typed code can crash at runtime due to `dynamic` casts), but it is **gradually sound** (typed code is safe unless it involves dynamic boundaries).

## Soundness & Completeness

### Soundness

A type system is **sound** if:

> If a program type-checks, the program cannot exhibit a type error at runtime.

Soundness is stronger than just "no crashes"—it means operations on a value of type `T` will behave as specified for type `T`.

Examples of unsound type systems:
- Java's array covariance: `Integer[] x = new Integer[1]; Object[] y = x; y[0] = "oops";` (ArrayStoreException at runtime).
- TypeScript's structural typing: `interface A { x: int }; interface B { x: int }; let a: A = {}; let b: B = a;` (both have `x`, but incompatible implementations crash at runtime if fields are accessed).

### Completeness

A type system is **complete** if:

> If a program cannot exhibit a type error, the program type-checks.

Completeness is rare in practice because:
1. Undecidability: type checking can be undecidable (dependent types).
2. Conservatism: type checkers refuse some safe programs to preserve tractability.

Example: Haskell's monomorphism restriction (deliberately incomplete):

```haskell
-- Monomorphism restriction: 'plus' becomes monomorphic even though it could be polymorphic
plus = (+)  -- inferred: plus : Int -> Int -> Int (not ∀a. Num a => a → a → a)
```

## Type Inference Limitations

### When Inference Fails

1. **Overloading ambiguity**: `print(x)` is ambiguous without `x`'s type.
2. **Recursive types**: `let f x = f x` generates `a = a`, which is unsound (infinite type).
3. **Implicit quantification**: `let f = []; f` where `f` should be `∀a. [a]`, but treating it as monomorphic breaks reuse.

### Solutions

- **Explicit annotations**: `let f: ∀a. [a] = []`
- **Whole-program inference**: Collect all constraints globally, then solve (expensive).
- **Rank-1 restriction**: Allow polymorphic types only at the top level (HM preserves this).
- **Rank-N polymorphism**: Allow polymorphic types nested in function arguments (requires explicit annotation at use sites).

## Modern Extensions

### Local Type Inference

**Local type inference** (Pierce & Turner) limits inference to "local" regions—expression boundaries—rather than global constraint solving. This makes inference tractable while supporting higher-rank polymorphism:

```
-- Rank-2 function (second argument is polymorphic)
poly f = (f 1, f "hello")  -- f must be ∀a. a → b for some b per call

-- Annotation needed at call site if inferring rank-2 would be ambiguous
poly (\x -> x)  -- Error: cannot infer rank-2 polymorphic lambda
poly ((\x -> x) :: ∀a. a → a)  -- OK: explicitly instantiated
```

### Visible Type Application

Languages like Haskell and Scala offer **visible type application**—explicit instantiation of polymorphic types:

```haskell
-- Without visible application: read :: Show a => String -> a
read "42" :: Int       -- Type annotation forces a = Int

-- With visible application (GHC extension):
read @Int "42"         -- Explicitly instantiate a = Int
```

## Practical Implementation: OCaml

OCaml's type checker is a production implementation of approximately HM with extensions:

1. **Constraint generation**: single pass through AST
2. **Unification**: Robinson's algorithm with occurs check
3. **Polymorphic variants & objects**: structural polymorphism beyond functions
4. **Weak type variables**: prevent polymorphic generalization in certain contexts

```ocaml
let id x = x;;              (* val id : 'a -> 'a *)
let const x y = x;;         (* val const : 'a -> 'b -> 'a *)
let twice f x = f (f x);;   (* val twice : ('a -> 'a) -> 'a -> 'a *)
```

## See Also

- **compilers-type-inference**: Focused dive into HM and Algorithm W
- **cs-type-system-practice**: Gradual typing, type narrowing, and modern practices
- **paradigm-type-level-programming**: Computation using type-level features
- **language-rust**: Trait-based ad-hoc polymorphism example
- **language-typescript-advanced**: Practical bidirectional checking in TypeScript