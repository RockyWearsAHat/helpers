# Property-Based Testing

## From Examples to Properties

Traditional example-based testing asserts that specific inputs produce specific outputs: "given input X, expect output Y." This works well for documenting known cases but inherently checks only the scenarios a developer imagines.

Property-based testing inverts the approach: instead of specifying concrete examples, the tester describes **properties that must hold for all valid inputs**, and the framework generates hundreds or thousands of random inputs to search for violations.

```
// Example-based: checks one case
test: reverse([1,2,3]) == [3,2,1]

// Property-based: checks a universal property
for all lists xs:
    reverse(reverse(xs)) == xs
```

The philosophical shift matters: example-based tests answer "does this specific case work?" Property-based tests answer "what must always be true?"

## Generators — Producing Well-Formed Inputs

Generators are the engine of property-based testing. They produce random values of a given type, respecting structural constraints.

### Generator Composition

Generators compose — complex data structures are built from simpler generators.

```
// Primitive generators
gen_int()          → random integer
gen_string()       → random string
gen_float()        → random float (including edge cases: NaN, Inf, -0.0)

// Composed generators
gen_pair(gen_int(), gen_string())  → (42, "xk8q")
gen_list(gen_int())                → [7, -3, 0, 999, -1]

// Domain-specific generators
gen_email()        → structurally valid email address
gen_json()         → arbitrary valid JSON
gen_sorted_list()  → list that's already sorted
```

### Generator Design Considerations

| Concern         | Approach                                                               |
| --------------- | ---------------------------------------------------------------------- |
| Distribution    | Bias toward edge cases (0, empty, max values) while covering the range |
| Size control    | Start small, grow gradually — find minimal failures first              |
| Constraints     | Filter invalid values or construct valid ones directly                 |
| Reproducibility | Seed-based generation for deterministic replay                         |

A common tension: filtering (generate anything, reject invalid) vs. construction (generate only valid). Filtering wastes generated values when valid inputs are rare. Construction requires understanding the input domain deeply but produces relevant inputs efficiently.

```
// Filtering: simple but wasteful if valid inputs are sparse
gen_even_int = gen_int().filter(n → n % 2 == 0)  // half wasted

// Construction: efficient but requires domain encoding
gen_even_int = gen_int().map(n → n * 2)           // all valid

// Filtering breaks down for complex constraints
gen_valid_binary_tree = gen_tree().filter(is_balanced)  // almost all rejected
```

## Shrinking — Finding Minimal Failures

When a property violation is found, the generated input is often large and complex. **Shrinking** automatically reduces the failing input to the smallest case that still triggers the failure.

```
// Original failure: a list of 47 elements triggers a bug
[83, -7, 0, 42, 16, -999, ..., 3, 8, -12]

// After shrinking: minimal reproducing case
[0, -1]
```

### How Shrinking Works

A shrinker for a type produces **simpler** variants of a value. What "simpler" means is type-dependent:

| Type       | Shrink Candidates                      |
| ---------- | -------------------------------------- |
| Integer n  | 0, n/2, n-1 (toward zero)              |
| List xs    | sublists, elements shrunk individually |
| String s   | shorter substrings, simpler characters |
| Pair (a,b) | shrink a holding b, shrink b holding a |
| Tree       | subtrees, nodes with simplified values |

The shrinker iteratively tries simpler values, keeping only those that still fail the property, until no further simplification preserves the failure.

### Integrated vs. Type-Based Shrinking

Two approaches exist:

**Type-based shrinking** defines shrink operations per type. The generator produces a value; the shrinker is a separate function. This requires writing shrinkers for custom types but gives precise control.

**Integrated shrinking** embeds shrinking into the generator itself. The random choices made during generation are shrunk at the choice level, automatically producing simpler values. This works for any generator without extra code but can produce less intuitive shrunk values.

The trade-off: type-based shrinking produces more human-readable minimal cases; integrated shrinking requires less maintenance and handles complex generators more uniformly.

## Writing Good Properties

Not all properties are equally useful. Some patterns consistently produce high-value properties.

### Common Property Patterns

**Round-trip (encode-decode)**: applying an operation and its inverse returns the original.

```
for all x:
    decode(encode(x)) == x
```

Found bugs in: serialization, compression, encryption, URL encoding, pretty-printing.

**Idempotency**: applying an operation twice produces the same result as once.

```
for all x:
    normalize(normalize(x)) == normalize(x)
```

Found bugs in: formatting, database migrations, cache invalidation, UI rendering.

**Commutativity / associativity**: algebraic properties that must hold.

```
for all a, b:
    merge(a, b) == merge(b, a)            // commutativity
for all a, b, c:
    merge(merge(a, b), c) == merge(a, merge(b, c))  // associativity
```

**Invariant preservation**: operations maintain structural invariants.

```
for all valid_heap h, element e:
    is_valid_heap(insert(h, e))
```

**Equivalence to reference**: a new implementation matches an established one.

```
for all input:
    fast_sort(input) == known_correct_sort(input)
```

**Hard to compute, easy to verify**: some results are easier to check than produce.

```
for all input:
    let result = solve(input)
    verify_solution(input, result) == true
```

### Weak vs. Strong Properties

Properties exist on a spectrum of strength:

| Strength    | Property                                 | What It Catches                         |
| ----------- | ---------------------------------------- | --------------------------------------- |
| Weak        | "doesn't crash"                          | Null dereferences, unhandled exceptions |
| Medium      | "output has correct type/shape"          | Structural errors                       |
| Strong      | "output satisfies specification"         | Logical errors                          |
| Very strong | "equivalent to reference implementation" | Any behavioral deviation                |

Weak properties still catch surprising bugs — "doesn't crash for any input" eliminates entire categories of robustness issues. But they miss logical errors where the function returns a wrong-but-valid value. Combining properties at different strengths builds layered assurance.

## The Oracle Problem

The deepest challenge in property-based testing: **how do you know the output is correct when you don't have the expected answer?**

In example-based testing, the human provides the expected output. In property-based testing, the test must verify correctness programmatically for arbitrary inputs.

### Oracle Strategies

**Self-consistency**: the output must satisfy relationships without knowing the exact value.

```
for all list xs:
    let sorted = sort(xs)
    length(sorted) == length(xs)           // preserves size
    is_sorted(sorted)                       // output is ordered
    is_permutation(sorted, xs)              // same elements
```

These three properties together fully specify sorting, without ever stating what the sorted output should be.

**Differential testing**: compare two implementations.

```
for all input:
    new_parser(input) == reference_parser(input)
```

**Metamorphic testing**: reason about how changes in input should relate to changes in output.

```
// If you add a record, search results should include it
for all db, record, query where matches(record, query):
    search(insert(db, record), query) ⊇ search(db, query)
```

**Inverse functions**: verify through round-tripping.

**Domain invariants**: properties derived from the problem domain that must hold regardless of specific outputs.

## Model-Based Testing

Model-based testing verifies a system against a **simplified reference model** — an abstract version that's easy to reason about but captures essential behavior.

```
// System under test: a production key-value store with caching, persistence, etc.
// Model: a simple dictionary/hashmap

for all sequence of operations [op1, op2, ..., opN]:
    apply operations to both system and model
    after each operation:
        system.get(k) == model.get(k) for all accessed keys
```

The model serves as both oracle and specification. Discrepancies between the system and model indicate either a bug in the system or a gap in the model.

### Strengths of Model-Based Testing

- Tests interaction patterns, not just individual operations
- Explores operation orderings that humans rarely consider
- The model itself serves as executable documentation of intended behavior
- Found bugs in real database engines, file systems, and distributed systems

### Limitations

- Requires building and maintaining the model
- The model can have its own bugs (which sometimes cancel out system bugs)
- Complex system behaviors may be hard to model simply
- Performance characteristics are typically not modeled

## Stateful Property-Based Testing

Stateful testing generates **sequences of operations** against a stateful system, checking invariants at each step.

```
// Define possible operations
operations = [
    Insert(key: gen_string(), value: gen_int()),
    Delete(key: gen_string()),
    Lookup(key: gen_string()),
    Clear()
]

// Generate random sequences
for all sequence of operations:
    execute sequence against system
    after each step:
        verify invariants hold
        verify consistency with model (if using model-based approach)
```

This finds bugs that arise from **specific sequences of interactions** — the kind of bugs that plague concurrent data structures, protocol implementations, and stateful APIs.

### Shrinking Operation Sequences

When a failing sequence is found, shrinking removes unnecessary operations:

```
Original failure: [Insert A, Insert B, Delete A, Insert C, Lookup B, Delete B, Insert A]
After shrinking:  [Insert B, Delete B, Lookup B]
// Minimal case: inserting, deleting, then looking up the same key
```

This is extremely valuable for debugging — instead of a 50-operation reproduction, you get a 3-operation case.

## Property-Based Testing and Fuzzing

Property-based testing and fuzzing share the principle of generating random inputs, but differ in approach and goals.

| Dimension        | Property-Based Testing           | Fuzzing                              |
| ---------------- | -------------------------------- | ------------------------------------ |
| Input generation | Structured, type-aware           | Often byte-level mutation            |
| Oracle           | Programmer-specified properties  | Crashes, hangs, sanitizer violations |
| Shrinking        | Built-in, produces minimal cases | Some fuzzers support minimization    |
| Target           | Functional correctness           | Robustness, security, crashes        |
| Feedback loop    | Random or coverage-guided        | Typically coverage-guided            |
| Domain knowledge | Encoded in generators            | Encoded in seed corpus or grammar    |

The overlap grows as both fields mature. Coverage-guided property testing uses code coverage feedback to steer generation toward unexplored paths. Grammar-based fuzzing produces structured inputs similar to property-based generators. Hybrid approaches combine the correctness focus of property-based testing with the exploration depth of fuzzing.

## Coverage-Guided Property Generation

Standard property-based testing generates inputs blindly — the generator doesn't know which code paths have been exercised. Coverage-guided generation uses instrumentation feedback to bias generation toward inputs that explore new branches.

```
// Standard: random generation, may exercise same paths repeatedly
generate → run → check property → repeat

// Coverage-guided: feedback loop steers toward new code paths
generate → run → measure coverage →
    if new coverage: save input as interesting
    mutate interesting inputs to explore further
```

This approach finds bugs faster in systems with complex branching logic. It's particularly effective for:

- Parsers with many production rules
- Protocol implementations with numerous state transitions
- Numeric code with boundary conditions

The trade-off: coverage-guided generation adds infrastructure complexity and runtime overhead. For straightforward data structures and pure functions, simple random generation often suffices.

## Where Property-Based Testing Excels

**Serialization and parsing** — perhaps the highest-value application. Round-trip properties catch encoding bugs, character handling issues, and boundary conditions.

**Data structures** — invariant preservation properties verify structural correctness after every operation (e.g., red-black tree color invariants, heap ordering, balanced depth).

**Compilers and interpreters** — equivalence properties confirm optimization preserves semantics and compilation preserves meaning.

**Distributed systems** — linearizability, commutativity, convergence properties verify that replicas converge after synchronization.

## Where Property-Based Testing Is Harder to Apply

### User Interfaces

UI behavior is hard to specify as universal properties. What properties should hold for "clicking a button opens a dialog"? UI testing tends toward example-based scenarios because the "correct" behavior is often visual and contextual.

Partial approaches: property-test the UI data model and state machine, even if rendering is tested via examples.

### Systems with Complex Preconditions

When valid inputs occupy a tiny fraction of the input space, generators struggle. A function requiring a valid SQL query, a specific authentication state, and a particular database schema has preconditions that are expensive to generate randomly.

Approaches:

- Build generators that construct valid inputs by design
- Use stateful testing to build up valid state through operation sequences
- Focus property testing on components with simpler input domains

### Side-Effecting Operations

Properties are easiest to express for pure functions. When functions send emails, write to databases, or call external services, both generation and verification become more complex.

Approaches:

- Test pure logic separately from side effects
- Use mocking or sandboxed environments for property tests
- Focus properties on the deterministic decision logic

## Properties as Specification Discovery

An underappreciated benefit of property-based testing: the act of writing properties forces precise thinking about what the code should do.

When a developer considers "what must always be true about this function?" they often discover:

- Ambiguities in the requirements
- Edge cases that weren't considered
- Implicit assumptions that should be documented
- Invariants that the implementation accidentally violates

This specification-discovery effect has value even before the first test runs. The properties become executable documentation of the system's contract.

## Practical Integration

Property-based and example-based tests complement each other:

| Example-Based Tests          | Property-Based Tests            |
| ---------------------------- | ------------------------------- |
| Document known edge cases    | Discover unknown edge cases     |
| Regression for specific bugs | Broad coverage                  |
| Easy to understand and debug | Minified failures via shrinking |
| Fast, deterministic          | Slower, stochastic              |

A common pattern: property tests for broad coverage, example tests for known edge cases and regression, property tests to explore boundaries.

Because property-based tests use random generation, tests may fail on one run and pass on the next. Seed recording, failure case databases (saving minimal failing cases for re-checking), and fixed seeds in CI address this. Performance matters at scale — keep individual checks fast, use size-bounded generators in CI, and profile generators when iteration counts are high.

## Properties, Types, and Verification

Type systems, property-based testing, and formal verification express specifications at different levels:

- **Types** guarantee structural properties — you can't pass a string where an integer is expected
- **Property tests** verify behavioral properties — sorting actually sorts, for many random inputs
- **Formal verification** proves properties about all executions — the algorithm is correct for every possible input

Richer type systems (dependent types, refinement types) encode more properties statically, reducing the need for runtime property checks. In languages with standard type systems, property-based testing covers ground that types cannot.

## Designing for Property Testability

Systems that are easy to property-test share characteristics: pure functions with clear contracts, algebraic structure (operations satisfying laws), composable components with independent behavior, and observable state. This creates a virtuous cycle — designing for property testability also produces cleaner, more modular architecture.
