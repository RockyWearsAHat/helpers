# Advanced Testing Patterns

## Beyond Unit Tests

This guide covers testing patterns beyond basic unit tests — the advanced strategies that catch the bugs unit tests miss. Property-based testing, mutation testing, contract testing, chaos engineering, and more.

## Property-Based Testing

Instead of writing specific test cases, describe properties that should always hold. The framework generates hundreds of random inputs and tries to break your code.

```python
# Hypothesis (Python)
from hypothesis import given, strategies as st

@given(st.lists(st.integers()))
def test_sort_is_idempotent(xs):
    """Sorting twice gives the same result as sorting once."""
    assert sorted(sorted(xs)) == sorted(xs)

@given(st.lists(st.integers()))
def test_sort_preserves_length(xs):
    """Sorting doesn't add or remove elements."""
    assert len(sorted(xs)) == len(xs)

@given(st.lists(st.integers()))
def test_sort_preserves_elements(xs):
    """Sorting returns the same elements."""
    assert sorted(sorted(xs)) == sorted(xs)
    assert set(sorted(xs)) == set(xs)
    # Also check counts (multiset equality)
    from collections import Counter
    assert Counter(sorted(xs)) == Counter(xs)

@given(st.lists(st.integers(), min_size=2))
def test_sort_is_ordered(xs):
    """Every element is ≤ the next element."""
    result = sorted(xs)
    for a, b in zip(result, result[1:]):
        assert a <= b
```

```haskell
-- QuickCheck (Haskell — the original)
prop_reverse_involution :: [Int] -> Bool
prop_reverse_involution xs = reverse (reverse xs) == xs

prop_sort_idempotent :: [Int] -> Bool
prop_sort_idempotent xs = sort (sort xs) == sort xs
```

**Key properties to test:**
- **Idempotence**: `f(f(x)) == f(x)` (sort, normalize, sanitize)
- **Round-trip**: `decode(encode(x)) == x` (serialization, compression)
- **Invariants**: Size preserved, elements preserved, ordering maintained
- **Commutativity**: `f(a, b) == f(b, a)` where expected
- **Associativity**: `f(f(a, b), c) == f(a, f(b, c))` where expected

**Shrinking**: When a property fails, the framework automatically reduces the failing input to the minimal case that still fails. A 500-element list failure shrinks to `[1, 0]`.

## Mutation Testing

Tests your tests. Automatically introduces bugs (mutations) into your code and checks that your test suite catches them. If a mutation survives (tests still pass), your tests have a gap.

```
Mutation operators:
- Replace > with >=, == with !=, + with -
- Remove conditional branches
- Replace return values with defaults (0, null, empty)
- Remove method calls
- Negate boolean expressions

Tools:
- Stryker (JavaScript/TypeScript/.NET)
- pitest (Java)
- mutmut (Python)
- cargo-mutants (Rust)
```

**Reading results:**
- **Mutation score** = killed mutations / total mutations
- A mutation score of 100% means every code mutation is caught by at least one test
- Mutation testing is slow (runs entire test suite per mutation) — use on critical code, not everything

## Contract Testing (API Boundaries)

Test that API consumers and producers agree on the contract, without running both together.

```
Pact (Consumer-Driven Contracts):

1. Consumer writes a test defining expected interactions:
   "When I send GET /users/42, I expect 200 with { name: string, age: number }"

2. Pact generates a contract file from the consumer test.

3. Provider runs the contract against its real implementation:
   "Does my API actually return { name: string, age: number } for GET /users/42?"

4. If both sides pass, they're compatible.
```

**When to use**: Microservices, mobile apps talking to backends, any system with separately deployed components that communicate via APIs.

**Not the same as**: Integration testing (which actually runs both systems). Contract testing works offline.

## Snapshot Testing

Capture output once, then assert future runs match the snapshot. Good for UI rendering, serialization output, and compiler/transpiler output.

```javascript
// Jest snapshot
test('renders user profile', () => {
  const tree = renderer.create(<UserProfile user={testUser} />).toJSON();
  expect(tree).toMatchSnapshot();
});

// First run: creates __snapshots__/UserProfile.test.js.snap
// Future runs: compares against saved snapshot
// Update: jest --updateSnapshot (when changes are intentional)
```

**Gotchas:**
- Snapshots must be reviewed in code review — don't blindly update
- Avoid snapshotting timestamps, random IDs, or other non-deterministic values
- Large snapshots become meaningless (nobody reviews a 500-line diff)

## Fuzzing

Feed random/malformed input to find crashes, hangs, and security vulnerabilities. The fuzzer mutates inputs guided by code coverage — it learns which inputs explore new code paths.

```go
// Go native fuzzing (1.18+)
func FuzzParseJSON(f *testing.F) {
    // Seed corpus
    f.Add([]byte(`{"key": "value"}`))
    f.Add([]byte(`[1, 2, 3]`))
    f.Add([]byte(`null`))

    f.Fuzz(func(t *testing.T, data []byte) {
        var result interface{}
        err := json.Unmarshal(data, &result)
        if err != nil {
            return // invalid JSON is fine, just don't crash
        }
        // Round-trip property
        encoded, err := json.Marshal(result)
        if err != nil {
            t.Fatal("Marshal failed on valid data:", err)
        }
        var result2 interface{}
        if err := json.Unmarshal(encoded, &result2); err != nil {
            t.Fatal("Round-trip broken:", err)
        }
    })
}
```

**Tools**: AFL/AFL++, libFuzzer, go-fuzz, jazzer (Java), atheris (Python)

**What fuzzers find**: Buffer overflows, integer overflows, infinite loops, unhandled edge cases, assertion violations, memory corruption.

## Chaos Engineering

Deliberately inject failures in production (or staging) to build confidence in the system's resilience.

```
Principles (Netflix):
1. Define "steady state" as measurable output (requests/sec, error rate)
2. Hypothesize: "Steady state will continue during the experiment"
3. Inject real-world events: kill processes, network partition, disk full
4. Try to disprove the hypothesis

Tools:
- Chaos Monkey (Netflix): Randomly kills production instances
- Litmus (Kubernetes): Chaos experiments for k8s
- Gremlin: Enterprise chaos platform
- tc (Linux): Network delay/loss/corruption
- kill -9: The original chaos tool
```

## Test Doubles Taxonomy

```
Dummy   — passed around but never used. Fills a parameter.
Stub    — returns canned answers. No logic.
Spy     — records calls for later verification.
Mock    — pre-programmed with expectations. Verifies behavior.
Fake    — working implementation but unsuitable for production
          (in-memory database, fake HTTP server).

Prefer fakes > stubs > mocks.
Fakes test behavior. Mocks test implementation details.
```

## Approval Testing

Like snapshot testing but for complex outputs — CLI output, report generation, email templates, document rendering.

```python
# approvaltest (Python)
import approvaltests

def test_report_generation():
    report = generate_monthly_report(test_data)
    approvaltests.verify(report)
    # First run: creates .approved file for human review
    # Future runs: compares against approved output
```

## Test Organization Patterns

### Arrange-Act-Assert (AAA)
```python
def test_withdraw_sufficient_funds():
    # Arrange
    account = Account(balance=100)

    # Act
    account.withdraw(30)

    # Assert
    assert account.balance == 70
```

### Given-When-Then (BDD)
```gherkin
Feature: Account withdrawal

  Scenario: Sufficient funds
    Given an account with balance $100
    When I withdraw $30
    Then the balance should be $70
```

### Test Fixture Patterns
```python
# Builder pattern for test data
user = UserBuilder().with_name("Alice").with_age(30).active().build()

# Object Mother (factory methods)
user = TestUsers.active_admin()
user = TestUsers.expired_trial_user()

# Prefer builders for variations, Object Mother for common archetypes
```

## Advanced Assertions

```python
# Soft assertions (check all, report all failures)
from assertpy import assert_that, soft_assertions

with soft_assertions():
    assert_that(user.name).is_equal_to("Alice")
    assert_that(user.age).is_greater_than(0)
    assert_that(user.email).contains("@")
    # All three checked; all failures reported together

# Custom matchers
class IsValidEmail:
    def __eq__(self, other):
        return isinstance(other, str) and "@" in other and "." in other
    def __repr__(self):
        return "IsValidEmail()"

assert user.email == IsValidEmail()
```

## Testing Anti-Patterns

1. **Testing implementation, not behavior**: Assert internal state instead of observable output. Breaks on every refactor.
2. **Flaky tests ignored**: A flaky test is worse than no test. It teaches the team to ignore failures.
3. **Slow test suites**: If tests take 30 minutes, developers stop running them. Keep the fast feedback loop.
4. **Testing the framework**: Verifying that `if` statements work, that the ORM saves, that HTTP returns 200 for valid routes. Test YOUR logic.
5. **100% coverage worship**: Coverage measures lines executed, not correctness. 100% coverage with bad assertions catches nothing.

---

*Sources: "Growing Object-Oriented Software Guided by Tests" (Freeman & Pryce), "Unit Testing Principles, Practices, and Patterns" (Khorikov), Hypothesis docs, Pact docs, Netflix Chaos Engineering*
