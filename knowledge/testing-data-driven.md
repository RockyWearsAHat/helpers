# Data-Driven and Generative Testing

## Concept

Data-driven testing decouples test logic from test data. Instead of writing separate tests for each scenario, you define a set of inputs with expected outputs, and one test harness iterates through all cases.

Generative testing goes further: instead of hand-writing test data, the test framework automatically generates inputs to discover bugs you didn't anticipate.

## Table-Driven Tests

The simplest form of data-driven testing. Often associated with Go, but applicable to any language.

**Pattern:**
```
Define a table of test cases (input, expected output, metadata)
For each row, run the test logic
Assert that actual output matches expected
```

**Example:**
```
tests := []struct {
  input    string
  want     int
  caseName string
}{
  { "hello", 5, "simple string" },
  { "", 0, "empty string" },
  { "🎉", 1, "emoji" },
}

for _, tt := range tests {
  t.Run(tt.caseName, func(t *testing.T) {
    got := len(tt.input)
    if got != tt.want {
      t.Errorf("got %d, want %d", got, tt.want)
    }
  })
}
```

**Advantages:**
- Written once, amortized over many test cases
- Easy to read: test data as declarative table
- Easy to add cases: add a row
- Clear failure messages: test data is visible

**Limitations:**
- You still hand-write all test cases
- Easy to miss edge cases you didn't think of
- Maintenance burden grows with table size

## Parameterized Tests

Broader concept: run the same test logic with different parameter sets. Supported natively in most frameworks (JUnit, pytest, etc.).

**Mental model:** Tables for simpler patterns, parameterized fixtures for complex setup.

## Fuzzing: Coverage-Guided Generation

Fuzzing automatically generates random inputs to a function, looking for crashes or assertion failures.

**Coverage-guided fuzzing** (AFL, libFuzzer, go-fuzz) monitors code coverage: inputs that exercise new code paths are kept and mutated further. This focuses generation on interesting inputs rather than purely random data.

**Mechanism:**
1. Run the fuzzer on a function
2. Fuzzer generates random inputs
3. If input crashes or hangs, save it as a test case
4. If input exercises new code path, save and mutate it
5. Iterate until timeout

**Characteristic:** Finds crashes, buffer overflows, parser bugs, integer overflows. Excellent at breaking systems through unexpected input combinations.

**Limitation:** Requires a crash signal. Fuzzing doesn't know *what correct behavior is*—only that the program shouldn't crash.

## Property-Based Testing

Rather than example-based tests ("given this input, expect this output"), property-based testing defines properties ("for any input matching this specification, this invariant holds").

**Example property:** "For any two integers, reverse(reverse(list)) should equal list"

**Frameworks:** QuickCheck (Haskell), Hypothesis (Python), fast-check (JavaScript), Proptest (Rust).

**Mechanism:**
1. Define a property (invariant that should hold)
2. Framework generates random inputs matching the type spec
3. Runs the function and checks if property holds
4. If property fails, shrinks the failing input to the minimal counterexample
5. Reports the simplified failure case

**Advantage over fuzzing:** Automatically validates the *correctness criterion*, not just crashes. Generates hundreds of test cases without hand-writing them.

**Advantage over example tests:** Catches edge cases you didn't anticipate. "For any integer" catches negative numbers, zero, MAX_INT, etc. automatically.

**Limitation:** Requires defining the right property. Incorrect properties pass meaningless tests.

## Model-Based Testing

Compares the actual system against a simplified model (mock). Generates a sequence of operations and verifies that the system and model stay in sync.

**Example:** Testing a Set, compare against an array-based model. Generate add/remove/contains operations, verify both behave identically.

**Benefit:** Catches implementation bugs that violate the abstract specification. Model is simpler to verify than implementation.

**Tools:** fast-check (model-based runners), QuickCheck (state machines).

## Metamorphic Testing

Tests properties of function relationships rather than absolute correctness.

**Example:** Sorting. Hard to define "correct" sorting without writing sort code. But you can define: "sorted list of [1,2,3] and [3,1,2] should have the same elements" (different assertion than exact value match).

**Benefit:** Tests properties you can reason about without defining exact outputs.

## When to Use Each

- **Example tests (table-driven):** Simple functions with clear cases. Use when you know the edge cases.
- **Fuzzing:** Security-critical parsers, file format readers, anything that might crash. Run long-term in CI/nightly.
- **Property-based:** Complex logic where you know invariants. Algorithms, data structures, business rules.
- **Model-based:** Stateful systems (databases, caches, queues). Verify behavior against simplified spec.
- **Metamorphic:** When correctness is hard to specify but relationships are clear.

## Coverage and Tradeoffs

Property-based and fuzzing-based tests generate more scenarios than hand-written tests. They find edge cases.

**Tradeoff:** Slower to execute, harder to debug (random inputs are less human-readable than examples), harder to reason about failure (why did this random input fail?).

**Balance:** Combination approach. Hand-written example tests for fast feedback. Property-based for comprehensive coverage in nightly runs. Fuzzing for security-critical paths.

## Mental Model

Generative testing asks: "What would break this if I throw random inputs at it?" instead of "Does this work for cases I predicted?"

Hand-written tests scale linearly: more cases, more tests. Generated tests scale exponentially: each property or model covers many input combinations automatically.

See also: testing-philosophy, testing-advanced-patterns, formal-verification