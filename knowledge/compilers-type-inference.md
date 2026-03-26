# Type Inference — Hindley-Milner, Algorithms & Practical Limitations

## Overview

Type inference deduces the types of expressions from their usage, reducing annotation burden. **Hindley-Milner (HM)** type inference is the foundational algorithm, enabling polymorphic type deduction without explicit annotations in source code. It works via **constraint generation** (scan the program, collect type equations from usage) and **constraint solving** (unification—solve for type variables). Modern systems extend HM with bidirectional checking, flow sensitivity, and structural typing, each trading simplicity for expressiveness.

## Hindley-Milner (HM) & Algorithm W

### Core Concept
HM generates type equations from how expressions are used, then solves them. A function call `f(x)` implies the argument `x` has the type expected by `f`'s parameter. All such constraints form a system of equations; unification finds a solution (most general unifier, MGU).

Example: given `f(x) = x + 1`, the system generates:
- `typeof(f) = T1 → T2` (f takes T1, returns T2)
- `T1 + 1 → T2`, which means T1 must support `+` (constraint)
- `T2 = int` (result of `+` is int)

Unification solves: `f : int → int`.

### Algorithm W
Interleaves constraint generation and unification (vs. collecting all constraints first). Scans the AST depth-first, when encountering a type equation, solves it immediately and applies the solution to subsequent equations. Efficient: single pass over the code. Linear in source size (with good unification implementation). Basis for OCaml, Haskell, and early ML systems.

```
W(Γ, e) returns (type, substitution)
- Variable x: lookup x in Γ, return (type[x], {})
- Application f(e): 
  - (T_f, σ1) = W(Γ, f)
  - (T_e, σ2) = W(σ1(Γ), e)
  - Create T_result (fresh type variable)
  - σ3 = unify(σ2(T_f), T_e → T_result)
  - Return (σ3(T_result), σ3∘σ2∘σ1)
```

The returned substitution is the most general type consistent with all observed uses.

### Unification
Solves type equations. Given `T1 = T2`:
- If both are identical, succeed.
- If one is a type variable alpha, substitute alpha for the other (occurs check: reject if alpha appears in the other's structure, preventing infinite types).
- If both are constructors (e.g., list types), recurse on arguments.
- Otherwise, fail (e.g., int ≠ string).

Multiple equations unified sequentially; each solution applied to the rest. The combined substitution is the MGU.

Issue: occurs check can be expensive. Some implementations skip it (unsafe), catching issues at runtime (not standard).

## Let-Polymorphism

HM alone doesn't generalize over type variables within a let-binding. Consider:
```ml
let id = \x. x in (id 1, id "hello")
```

Without generalization, HM infers `id : T → T` from the first call (`id 1`), binding T to int, so `id "hello"` fails.

**Let-polymorphism** quantifies type variables in let-bound definitions: `id : ∀T. T → T`. Each use of `id` instantiates T to a fresh variable, enabling polymorphism. The rule:

```
Γ ⊢ let x = e1 in e2 : T
if Γ ⊢ e1 : ∀α1...αn. T1  (where α1...αn are free type vars in T1 not in Γ)
then Γ[x := ∀α1...αn. T1] ⊢ e2 : T
```

Let-binding is the only place where universal quantification is introduced. Lambdas do not generalize (monomorphic). This restriction is deliberate—it keeps inference decidable and efficient.

## Constraint-Based Inference

Rather than Algorithm W's interleaved approach, constraint-based inference collects all equations, then solves the entire system. Advantages:

- Cleaner logic: separate constraint generation from solving.
- Extensibility: add new constraints (e.g., class constraints in Haskell) without rewriting the solver.
- Parallelization: solve independent constraints concurrently.
- Better error messages: accumulated constraints show the full context.

Disadvantage: may generate many redundant constraints; unification may not fail early. Haskell uses a constraint-based approach extended with **qualified types**: type `Eq a => a → a` says "type a must implement Eq." The type checker collects Eq constraints and verifies instances exist.

## Bidirectional Type Checking

Two modes: **\\ inference mode** (bottom-up: deduce type from usage) and **checking mode** (top-down: verify an expression matches an expected type). Bidirectional checking passes a type "expectation" down the AST and propagates constraints up.

Advantages:
- Reduces annotations in complex code. If a lambda's parameter has an expected type from context, no annotation needed.
- Better error messages: "expected T, got S" vs. inferring an unrelated type.
- Simpler rules for higher-rank polymorphism (forall types nested in function arguments).

Example: `fn g<'a>(f: for<'b> fn(&'b T) -> &'b S) { ... }`
Rust uses bidirectional checking. Checking mode verifies the higher-rank rank-2 type, simplifying constraint generation.

Drawback: requires an expected type (top-level expressions still need annotations or defaults). TypeScript uses hybrid: inference in many contexts, checking when types are given.

## Flow-Sensitive Typing

The type of a variable changes as the control flow passes through conditionals or explicit type guards. Not type inference per se, but a refinement mechanism.

Example in TypeScript:
```typescript
let x: string | number = ...;
if (typeof x === "string") {
  x.length;  // x : string in this block
} else {
  x + 1;     // x : number in this block
}
```

The type checker tracks which guards have been passed; inside each branch, x has a narrower type. Enable sophisticated pattern matching with minimal annotation.

Flow-sensitive analysis is decidable for simple cases (typeof guards, null checks) but can become complex with assignments in loops (fixed-point computation required). Rust does flow-sensitive borrowing via the borrow checker, updating permission state as references are moved.

## TypeScript Structural Inference

Most languages use **nominal typing**: identity of types is based on declaration location or name (`class Person` ≠ `class Employee` even if both have the same fields). Nominal types require explicit type annotations.

TypeScript uses **structural typing** by default: types are equivalent if they have the same shape (fields and methods). Inference exploits this: two objects with identical properties are compatible without shared base class. Combined with gradual typing (untyped code is implicitly `any`, coercing to structural types), TypeScript infers types from object literals:

```typescript
const person = { name: "Alice", age: 30 };
// Inferred { name: string; age: number; }
```

Trade-off: structural typing allows implicit subtyping and is flexible for refactoring, but can hide intent (two unrelated types might accidentally be compatible). Type-tests and explicit annotations are needed for precision.

## Rust Lifetime Inference

Rust's **ownership system** requires explicit lifetimes in function signatures—the duration a reference is valid. Full lifetime annotations would be tedious; Rust uses **lifetime elision rules** to infer lifetimes in common cases:

1. Each parameter with a reference gets a distinct lifetime.
2. If there's exactly one input lifetime, it's used for all outputs.
3. Methods with `&self` use `&self`'s lifetime for outputs.

Example: `fn search<'a>(haystack: &'a str, needle: &str) -> Option<&'a str>` can be written `fn search(haystack: &str, needle: &str) -> Option<&str>` (compiler infers `'a`).

Lifetime inference is local (doesn't cross function boundaries) and cannot generalize lifetimes (can't infer `for<'a>` patterns). Programmers must add explicit `for<'a>` for higher-rank lifetimes. This preserves explicitness—a key Rust philosophy—while reducing boilerplate.

## Type Inference Limitations & When Annotations Matter

### Undecidability Boundaries
Unrestricted higher-rank polymorphism (`for<'a> ...` nested in function parameters) is undecidable with Hindley-Milner. Systems supporting it (Rust, GHC with `-XRankNTypes`) require explicit annotations for rank-2+ types.

### Implicit Polymorphism & Ambiguity
Polymorphic code needs instantiation context. `let f = \x. x` is type `∀T. T → T`, but which T? Ambiguous. Haskell defaults to `_`, requiring an annotation to disambiguate. Python's `def f(x): return x` is fully untyped; no inference until an annotation is added.

### Performance vs. Inference Depth
Deep type inference can thrash the type checker (exponential search space in complex constraint systems). TypeScript, Go, and mypy use "local type inference" (context flows downward only a limited distance) to keep checking fast. Haskell's constraint-based inference (transitively demanding instances) can time out on cyclic or deeply nested types.

### Mutual Recursion & Forward References
Multisweep analysis (first infer all top-level signatures, then check bodies) is needed for mutual recursion. Single-pass inference fails. Practical systems require declaration ordering or explicit annotations.

### Module Boundaries
Type inference stops at module/file boundaries. Exported types must be explicitly annotated; callers don't see internal inference. This is a feature (encapsulation, explicit contracts) but requires annotations at boundaries.

## Practical Examples

- **Haskell**: Full HM + let-polymorphism, constraint-based inference for type classes
- **OCaml**: HM with mutable references (complicates polymorphism; requires careful type handling)
- **Rust**: Unification-based + lifetime elision + flow-sensitive borrowing
- **TypeScript**: Structural typing + flow-sensitive narrowing + gradual typing (real-world pragmatism)
- **Python**: No static type inference by default; mypy adds optional gradual typing with limited inference

## Key References

- *Damas & Milner* (1982): The original HM paper
- Bernstein: "Damas-Hindley-Milner Inference Two Ways" (pedagogical derivation)
- Odersky: "Type Systems as Constraints Between Types and Effects" (constraint-based generalization)
- Dunfield & Krishnaswami: "Complete and Easy Bidirectional Typechecking for Higher-Rank Polymorphism"
- Rust reference: lifetime elision rules