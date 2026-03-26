# Type Theory — Lambda Calculus, Curry-Howard, and Dependent Types

## Foundations: Untyped Lambda Calculus

Lambda calculus, introduced by Alonzo Church in the 1930s, is a minimal computing model where computation is rewriting function applications. Everything is a function.

```
Syntax: x                      (variable)
        λx. M                  (abstraction / function definition)
        M N                    (application / function call)

Example: (λx. x + 1) 5  →  6  (beta reduction)
```

The untyped lambda calculus is **Turing complete** — it can compute anything a Turing machine can compute. Yet without types, subtle issues emerge: the fixed-point combinator Y allows infinite recursion, and not all reductions terminate. Asking "does this program terminate?" is undecidable.

This motivates typing: restrict expressiveness to gain guarantees.

## Simply Typed Lambda Calculus (STLC)

Adding types constrains programs so that **all well-typed terms normalize** — they terminate in finite steps with a value. This is the essence of the Curry approach to types: assign types to terms to ensure desirable properties.

```
Types: A ::= A → B | 1 | Bool

Terms: x (variable)
       λx:A. M (typed abstraction)
       M N (application, only if M:A→B and N:A)
```

Key properties:
- **Type soundness:** Well-typed programs don't crash. If ⊢ M : A, then M either reduces to a value or diverges (doesn't violate its type).
- **Normalization:** Every well-typed term in STLC terminates. No divergence possible.

This makes STLC useful but weak — it cannot express recursion (recursion requires divergence or fix points outside the type system). Real languages add recursion operators or unrestricted recursion with type annotations that trust the programmer.

## Curry-Howard Correspondence: Propositions as Types

The Curry-Howard isomorphism is a profound observation: types in a computation system correspond exactly to logical propositions, and programs correspond to proofs.

| Logic              | Programming        |
| ------------------ | ------------------ |
| Proposition A      | Type A             |
| Proof of A         | Term of type A     |
| Implication A → B  | Function type A→B  |
| Conjunction A ∧ B  | Pair type A × B    |
| Disjunction A ∨ B  | Sum type A ⊕ B     |
| False              | Void (empty type)  |

A proof that "if A then B" is a function taking a proof of A and returning a proof of B. Checking that a program type-checks is checking that a logical argument is valid. This transforms verification from a separate activity into an integral part of programming.

**Practical implication:** Languages like Coq, Lean, and Agda blur the line between theorem prover and programming language. Writing a program means proving a proposition; running the program extracts the computational content of the proof.

## Polymorphism and System F

Parametric polymorphism — code that works for multiple types without runtime type checks — is formalized in **System F** (Girard, 1972).

```
Types: A ::= α | A → B | ∀α. A | ∃α. A

Example: identity : ∀α. α → α
         Λα. λx:α. x
```

`∀α. A` means "for all types α, here's code that works for α." The universal quantifier at the type level lets a single function (or data structure) serve many concrete types.

**Key insight:** Adding universal quantification makes the type system powerful (it can express data structures like lists and trees generically) while preserving normalization by restricting what can happen inside the quantifier body.

System F is used in languages like Haskell, ML, and portions of modern type systems. However, System F alone cannot express recursion efficiently or dependent types.

## Dependent Types

Dependent types extend STLC by allowing types to depend on terms. The type of an array might include its length; the type of a list concatenation result depends on the lengths of inputs.

```
Example (Agda/Coq notation):

Vec : ℕ → Type → Type              -- vectors indexed by length
Vec zero A = ⊥                     -- empty vector has no inhabitants
Vec (succ n) A = A × (Vec n A)     -- n+1 vector is A paired with size-n vector

Semantically: a type is a set depending on input values.
```

**Consequences:**

- **Undecidability of type checking:** The type checker must evaluate terms to compare types, so "does this type check?" becomes a semantic question, not purely syntactic. Type checking may not terminate.
- **Proof relevance:** Not only does a value exist, but evidence of properties persists computationally. A sorted list proof shows both the data and evidence of sortedness.
- **Expressiveness:** Dependent types can express invariants that simple types cannot. A function on vectors automatically proves type-safety of length-dependent operations.

Trade-off: termination is no longer automatic. Languages using dependent types (Coq, Lean, Idris, Agda) require explicit termination proofs or use syntactic criteria (sized types, guardedness).

## Homotopy Type Theory (HoTT)

Homotopy Type Theory reinterprets type theory through the lens of algebraic topology. Types are spaces; terms are points; type equality is homotopy (continuous deformation).

**The insight:** In classical set theory, equality is discrete — two elements either equal or not. In HoTT, equality itself is rich structure. There can be multiple distinct "reasons" (proofs) that two objects are equal, and these reasons themselves can be equal at a higher level.

```
Equality in HoTT:
 a = b : Type      (the type of proofs that a and b are equal)
 refl : a = a      (reflexivity, the canonical proof)

Transport: If P(a) and p : a = b, then transport(p, P) : P(b).
This says: properties at a can be moved along equalities to b.

Identity types form groupoids: equalities have inverses and unique composition.
```

**Practical implications:**

- **Univalence axiom:** Equivalent types are equal. This lets type-level operations (e.g., generic programming over isomorphic types) be formalized naturally.
- **Higher inductive types:** Types can have constructors at multiple "levels," enabling clean definitions of quotients, suspensions, and other homotopic structures.
- **Synthetic mathematics:** HoTT enables developing mathematics (algebraic topology, homotopy groups) inside type theory, with computational content available.

HoTT is more abstract and less settled than earlier type theories — practical tools (Coq, Lean, Agda) support subsets of HoTT but not the full theory, and performance tradeoffs are active research.

## Universe Polymorphism and Type Hierarchy

To avoid logical paradoxes (an object containing all types would be self-referential), type systems introduce universes: levels in a hierarchy.

```
Type : Type₁     (Type is in universe 1)
Type₁ : Type₂    (Type₁ is in universe 2)
...

Predicativity: a term in Type_i can only depend on types from Type_j where j < i.
```

Functions and data can be universe-polymorphic — they work at multiple universe levels uniformly:

```
id : ∀{i} → ∀(A : Type_i) → A → A
```

This avoids encoding techniques and keeps definitions compact.

## Inductive Types and Recursion

Inductive types formalize recursive data structures: lists, trees, natural numbers.

```
Nat : Type
  | zero : Nat
  | succ : Nat → Nat

List A : Type
  | nil : List A
  | cons : A → List A → List A

Recursion via pattern matching (structural recursion):
  length : ∀{A} → List A → Nat
  length [] = 0
  length (x :: xs) = 1 + length xs
```

**Guardedness and termination:** Recursion is only allowed if the recursive call is on a syntactically smaller argument (structural recursion) or under guardedness conditions. This ensures unrestricted functional programming still terminates.

## Practical Applications

**Coq:** Proof assistant and dependently-typed language. Used to verify CompCert (a C compiler), cryptographic protocols, and mathematical theorems. The language enforces termination via strict structural recursion.

**Lean 4:** Modern proof assistant balancing practical verification with mathematical expressiveness. Used by mathematicians formalizing research in combinatorics, analysis, and category theory. Designed for computational efficiency.

**Agda:** Language with dependent types and unicode, oriented toward proof development and program extraction. Offers optional termination checking.

**Practical pattern:** Dependent types excel where properties are invariants — sorting, string safety, protocol correctness. The cost is that proofs become part of the code, and type checking can be slow.

## Cross-References

See also: formal-verification.md, architecture-state-machines.md, paradigm-type-level-programming.md, type-systems-theory.md.