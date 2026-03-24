# Testing Strategies

## The Test Pyramid (Martin Fowler / Mike Cohn)

The test pyramid is a metaphor for how to balance test types:

```
       /  E2E  \        Few — slow, expensive, brittle
      /----------\
     / Integration \    Moderate — test component boundaries
    /----------------\
   /    Unit Tests    \  Many — fast, cheap, focused
  /____________________\
```

### Unit Tests (Base — Many)
- Test individual functions/methods in isolation.
- Fast (milliseconds), no I/O, no network, no database.
- Mock/stub external dependencies.
- Should make up 70-80% of your test suite.
- Assert one concept per test.
- Test names should describe the behavior being verified.

### Integration Tests (Middle — Moderate)
- Test the interaction between components (API + database, service + cache).
- Slower than unit tests (may hit real databases, filesystems, or HTTP).
- Verify that wiring between components works correctly.
- Use test databases/containers (Docker, testcontainers).
- Should make up 15-20% of your test suite.

### End-to-End Tests (Top — Few)
- Test entire user workflows through the real system.
- Slowest, most brittle (depend on UI, network, timing).
- Use for critical happy paths only (login, checkout, core business flow).
- Should make up 5-10% of your test suite.
- Tools: Playwright, Cypress, Selenium.

## Test-Driven Development (TDD)

The Red-Green-Refactor cycle:
1. **Red**: Write a failing test for the behavior you want.
2. **Green**: Write the minimum code to make the test pass.
3. **Refactor**: Clean up the code while keeping tests green.

**Benefits:** Forces interface design before implementation, builds confidence in refactoring, produces inherently testable code, serves as living documentation.

**When to use:** Business logic, algorithms, data transformations. Less valuable for UI/exploratory work.

## Behavior-Driven Development (BDD)

Write tests in natural language that describe behavior from the user's perspective:
```
Given a user with an active subscription
When they request a premium feature
Then they should see the feature content
```

Tools: Cucumber, SpecFlow, pytest-bdd. Best for collaboration between developers and non-technical stakeholders.

## Test Qualities (F.I.R.S.T.)

- **Fast**: Tests should run in seconds, not minutes.
- **Independent**: No test should depend on another's outcome or execution order.
- **Repeatable**: Same results every time, in any environment.
- **Self-validating**: Pass or fail — no manual inspection needed.
- **Timely**: Written close in time to the code they test.

## What to Test

- **Happy paths**: Does the normal case work?
- **Edge cases**: Empty inputs, boundary values, off-by-one.
- **Error paths**: Invalid input, network failures, permission denied.
- **Regression tests**: One test per bug fix — prevent re-introduction.
- **Contract tests**: Verify API producers and consumers agree on schemas.

## What NOT to Test

- Framework internals (trust your libraries).
- Private methods directly (test through public API).
- Trivial getters/setters with no logic.
- Implementation details — test behavior, not structure.

## Test Doubles

| Type | Purpose |
|------|---------|
| **Stub** | Returns canned responses. No assertions. |
| **Mock** | Verifies interactions (was method X called with args Y?). |
| **Spy** | Records calls for later assertion. |
| **Fake** | Working implementation with shortcuts (in-memory DB). |
| **Dummy** | Passed as argument but never used. |

**Best practice:** Prefer stubs over mocks. Over-mocking makes tests brittle and coupled to implementation.

## Code Coverage

- Coverage measures which lines/branches were **executed**, not which were **tested correctly**.
- 80% line coverage is a reasonable target. 100% is usually not worth the cost.
- Branch coverage is more valuable than line coverage.
- Low coverage is a signal. High coverage is not a guarantee.

---

*Sources: Martin Fowler (Practical Test Pyramid), Kent Beck (TDD By Example), Gerard Meszaros (xUnit Test Patterns), Google Testing Blog*
