# Test Doubles Deep Dive — Mocks, Stubs, Fakes, Spies, and Dummies

## Overview

Test doubles are controlled substitutes for dependencies—systems or components that the code-under-test interacts with. The confusion around test doubles stems from imprecise terminology: "mock" is used both as an umbrella term and as a specific type. This guide uses the Meszaros taxonomy (Gerard Meszaros, *xUnit Test Patterns*), the most widely adopted classification in professional testing communities.

Understanding which double to use—and when—separates brittle, high-maintenance tests from resilient, focused ones. The wrong choice leads to over-mocking (testing the mock, not the code), tests coupled to implementation details, and flaky E2E scenarios.

## The Meszaros Taxonomy

### Dummy

A dummy is an object passed to the code-under-test but never used. Its only purpose is to satisfy the signature of a method call.

```python
class PaymentProcessor:
    def charge(self, user, payment_gateway, logger):  # logger is a dummy
        return payment_gateway.debit(user.account, 100)

# In test:
logger_dummy = object()  # No behavior, never called
result = processor.charge(user, gateway, logger_dummy)
```

**Use:** When a method parameter is required but irrelevant to the behavior being tested. Dummies clarify intent: this parameter doesn't matter for this test.

### Stub

A stub returns a predetermined response without performing side effects. It answers questions ("What should happen?") but doesn't verify interactions.

```javascript
// Jest example
const mockGateway = {
    debit: jest.fn().mockReturnValue({ success: true, transactionId: '123' })
};

// The stub doesn't care how many times debit is called; it just returns a canned response
expect(processor.charge(user, mockGateway)).toBe(100);
```

**Use:** 
- When the code-under-test depends on an external service (API, database, third-party library) to make decisions.
- When you need different response scenarios (success, error, timeout) without contacting the real dependency.
- State-focused testing: you care about what the code *does*, not how it calls dependencies.

### Fake

A fake is a lightweight, working alternative to a real component—a simplified implementation that behaves correctly but isn't suitable for production.

```python
# Production: PostgreSQL database
# Test: In-memory SQL database (sqlite3) or simple dictionary

class FakeUserRepository:
    def __init__(self):
        self.users = {}
    
    def save(self, user):
        self.users[user.id] = user
    
    def find_by_id(self, user_id):
        return self.users.get(user_id)
```

**Use:**
- Database access layers (testcontainers run real Docker containers; fakes trade realism for speed).
- File systems (use temp directories or in-memory alternatives).
- Message queues (in-memory channel for tests).
- Caching layers.

**Key distinction from stubs:** Fakes have real logic—they compute, store, and retrieve data correctly. Stubs always return hardcoded responses.

### Spy

A spy wraps a real object or partial mock, recording interactions (calls, arguments, return values) while maintaining behavior. Spies sit at the boundary between state-based and interaction-based testing.

```javascript
// Jest spy on real object
const userService = {
    notify: (user, msg) => { console.log(`Notifying ${user.name}`); }
};

const spy = jest.spyOn(userService, 'notify');
processor.charge(user, gatewayStub, userService);

expect(spy).toHaveBeenCalledWith(user, 'Charge succeeded');
expect(spy).toHaveBeenCalledTimes(1);
```

**Use:**
- Verifying that your code called a dependency with the correct arguments.
- Debugging: recording what actually happened under real execution.
- Partial mocking: spy on one method while the rest of the object behaves normally.
- Legacy systems: spy on real code to understand behavior before refactoring.

### Mock (Strict)

A mock is the strictest double: a fully controlled object that defines *expectations* about how it should be called. The test fails if interactions don't match expectations, even if the final output is correct. Mocks are behavior-focused.

```java
// Mockito (Java)
PaymentGateway gateway = mock(PaymentGateway.class);
when(gateway.debit(anyInt())).thenReturn(true);

processor.charge(user, gateway);

// Verifies: debit must be called exactly once with the user's account
verify(gateway, times(1)).debit(user.account, 100);
```

Mocks specify *how* the code should behave (call sequences, argument constraints, timing), not just *what* it outputs.

**Use:**
- Interaction-based testing: the logic is in *how* you coordinate with dependencies, not in state changes.
- Command-Query Separation: when side effects (sending emails, logging to external systems) matter.
- Protocol verification: ensuring distributed systems communicate correctly.

**Critical distinction:** Stubs don't care how many times they're called. Mocks fail the test if they're called the wrong number of times, with wrong arguments, or in the wrong order.

## Sociable vs. Solitary Unit Tests

These terms describe testing philosophies, not specific tools.

### Solitary Tests (Mockist, London School)

Solitary unit tests isolate the code-under-test by mocking *all* collaborators—even simple domain objects. Each test is an island.

```python
# Solitary approach
def test_charge_deducts_from_balance():
    user_stub = Mock()
    user_stub.account_balance = 1000
    gateway_mock = Mock()
    gateway_mock.debit.return_value = True
    
    processor = PaymentProcessor()
    result = processor.charge(user_stub, gateway_mock)
    
    # Verify interaction: gateway.debit was called
    gateway_mock.debit.assert_called_once()
```

**Strengths:**
- Fast: no Database, no I/O.
- Pinpoints failures: if a test fails, the problem is in the code-under-test, not its dependencies.
- Exposes tight coupling: if you have to mock 15 collaborators, your design is too coupled.
- Supports incremental development: you can test a class before its dependencies exist.

**Weaknesses:**
- Tests implementation details: mocking internal collaborators means you're verifying *how* the code works, not *what* it does. Refactoring the internal structure breaks tests even if behavior is unchanged.
- High false confidence: all mocks are correct, but the real system might not work.
- Maintenance burden: every time a call signature changes, update the mock—and the test.

### Sociable Tests (Classicist, Chicago School)

Sociable tests use real collaborators where feasible, mocking only external boundaries (databases, APIs, third-party services).

```python
# Sociable approach (no mocks for domain objects)
def test_charge_deducts_from_balance():
    user = User(account_balance=1000)
    account = Account(balance=1000)
    gateway_stub = Mock()
    gateway_stub.debit.return_value = True
    
    processor = PaymentProcessor()
    result = processor.charge(user, gateway_stub)
    
    # Verify state, not interactions
    assert user.account_balance == 900
```

**Strengths:**
- Loose coupling to implementation: refactor internals freely without breaking tests.
- Realistic validation: collaborators behave realistically; integration bugs surface earlier.
- Fewer mocks to maintain.

**Weaknesses:**
- Slower: may involve setup, teardown, network, or I/O.
- Harder to isolate failures: if the test fails, is it your code or a collaborator bug?
- Requires stable, testable collaborators: can't test if dependencies are untestable.

### Choosing Between Them

**Solitary is better for:**
- Complex business logic where behavior is determined by interactions (state machine workflows, protocol negotiation).
- Pure functions and value objects (no I/O).
- Rapid feedback cycles during active development.

**Sociable is better for:**
- Domain logic with simple collaborators (e.g., User, Account).
- E2E-like integration tests (where some mocking is necessary for speed).
- Refactoring-heavy phases: sociable tests give more confidence.

Many teams use a **mixed approach**: sociable tests for core domain logic, solitary tests for complex orchestration and boundary logic.

## Over-Mocking Anti-Pattern

Over-mocking occurs when developers mock everything reflexively, resulting in tests that verify the mock's behavior instead of the real system's.

```javascript
// Anti-pattern: testing the mock
const userRepository = jest.mock();
const emailService = jest.mock();
const notificationService = jest.mock();

userRepository.find.mockReturnValue(user);
emailService.send.mockReturnValue(true);

processor.charge(user, repository, emailService, notificationService);

// Verifying the mock was called correctly—not that charging works!
expect(emailService.send).toHaveBeenCalled();
```

This test passes if the mocks are set up correctly even if the real `UserRepository.find()` or `EmailService.send()` crashes.

**Causes:**
- Habit: developers mock out of reflex, not reasoning.
- Tight coupling in the design: if you need to mock 8 things to test one, the code is over-integrated.
- Fear of I/O: avoiding databases and APIs leads to excessive test doubles.

**Signals of over-mocking:**
- Test setup is longer than the test itself.
- Changing a mock's return value fixes an unrelated test.
- Tests fail when you refactor method calls internally, even though behavior is unchanged.

**Solution:** Mock only external boundaries (database, HTTP, third-party APIs). Use real objects for domain logic, configuration, and utility classes. If setup is complex, consider integration tests or fixtures.

## HTTP Mocking Strategies

### Network-Level Mocking (Service Workers)

**Mock Service Worker (MSW)** intercepts HTTP requests at the network layer before they leave the browser/Node.js process.

```javascript
// setup.js
import { server } from './mocks/server';

beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterAll(() => server.close());

// handlers.js
import { http, HttpResponse } from 'msw';

export const handlers = [
  http.get('/api/users/:id', ({ params }) => {
    return HttpResponse.json({ id: params.id, name: 'Alice' });
  }),
];
```

**Advantages:**
- Same mock configuration works in unit tests, E2E tests, and local development.
- Transparent to the application: no code changes needed.
- Network-layer fidelity: catches serialization bugs, header issues, timing.
- Supports realistic scenarios: response delays, partial failures, redirects.

**When to use:** Component tests, E2E tests, development servers, any scenario where you want realistic HTTP behavior.

### Library-Level Mocking (nock, jest)

**nock** (Node.js) and Jest mocks mock at the HTTP client library level, intercepting `fetch()` or `axios()` calls.

```javascript
// nock (Node.js)
nock('https://api.example.com')
  .get('/users/1')
  .reply(200, { id: 1, name: 'Alice' });

const user = await fetch('https://api.example.com/users/1').then(r => r.json());
expect(user.name).toBe('Alice');
```

**Advantages:**
- Lightweight, no server setup.
- Works in Node.js environments without a browser context.
- Fine-grained control: mock specific headers, status codes, error sequences.

**Disadvantages:**
- Library-specific: `nock` works for `fetch()` and `axios()` but might not work with custom HTTP clients.
- Not transparent: your application must use the mocked library.

### WireMock (Java/Polyglot)

WireMock runs as a standalone HTTP mock server, useful for testing multiple services and verifying HTTP interactions.

```yaml
# stub-mappings.json
{
  "request": { "method": "POST", "url": "/api/charges" },
  "response": { "status": 201, "body": { "transactionId": "123" } }
}
```

**Advantages:**
- Language-agnostic: mock any HTTP client.
- Useful for integration testing with multiple services.
- Supports complex scenarios: delays, state verification, proxying.

**Use:** When testing HTTP interactions between services or ensuring a client behaves correctly with various HTTP responses.

## Database Testing: Fakes vs. Testcontainers

### Fakes (In-Memory Alternatives)

Fakes like SQLite (for PostgreSQL tests) or in-memory dictionaries speed up tests by trading realism for isolation.

```python
# Production: PostgreSQL
# Test: SQLite in-memory
import sqlite3

connection = sqlite3.connect(':memory:')
repository = UserRepository(connection)
repository.save(User(id=1, name='Alice'))
user = repository.find(1)
assert user.name == 'Alice'
```

**Advantages:**
- Instant startup, no Docker or network overhead.
- Deterministic and fully isolated from other tests.
- Great for unit tests of data access layers.

**Disadvantages:**
- Database dialect differences: SQLite != PostgreSQL. SQL that works in one might fail in the other.
- Missing features: some databases have unique indexing, constraints, or functions.
- Discoversy bias: bugs specific to production databases won't be caught.

### Testcontainers

Testcontainers spin up real Docker containers for each test, running actual databases.

```java
// Testcontainers (Java)
@Container
static PostgreSQLContainer<?> postgres = new PostgreSQLContainer<>("postgres:15")
    .withDatabaseName("test_db");

@Test
void testUserRepository() {
    HikariConfig config = new HikariConfig();
    config.setJdbcUrl(postgres.getJdbcUrl());
    config.setUsername(postgres.getUsername());
    config.setPassword(postgres.getPassword());
    
    UserRepository repo = new UserRepository(new HikariDataSource(config));
    repo.save(new User("Alice"));
    
    User user = repo.findByName("Alice");
    assert user.getName().equals("Alice");
}
```

**Advantages:**
- Real database: catches dialect-specific bugs, query optimization issues, constraint violations.
- Comprehensive: full feature set of the production database.
- Isolation: each test gets a fresh, clean container.

**Disadvantages:**
- Slower startup (tens of seconds, not milliseconds).
- Docker overhead: requires Docker to be running.
- Network calls: still I/O-bound.

**Best practice:** Use fakes for fast unit tests of query builders and simple business logic. Use testcontainers for integration tests of complex queries, transactions, and state transitions. Some teams run fakes in CI (fast feedback) and testcontainers locally (realistic validation).

## Mocking Libraries by Language

### JavaScript: Jest

```javascript
// Manual mocks
const mock = jest.fn();
mock.mockReturnValue(42);
mock.mockReturnValueOnce(42).mockReturnValueOnce(43);
mock.mockRejectedValue(new Error('Network error'));
mock.mockImplementation((x) => x * 2);

// Spying on real objects
jest.spyOn(userService, 'notify');
expect(userService.notify).toHaveBeenCalledWith(user, 'Success');

// Clearing and resetting
jest.clearAllMocks();      // Clear call history
jest.resetAllMocks();      // Clear history + implementation
jest.restoreAllMocks();    // And restore original implementations
```

### Python: unittest.mock

```python
from unittest.mock import Mock, patch, MagicMock

# Mocking
gateway_mock = Mock()
gateway_mock.debit.return_value = True
gateway_mock.debit.side_effect = Exception("Network error")

# Spying with patch
with patch('module.UserRepository') as mock_repo:
    mock_repo.return_value.find.return_value = user
    result = processor.charge(user, mock_repo)
    mock_repo.return_value.find.assert_called_once()

# Automatic mocks: MagicMock auto-creates nested attributes
magic = MagicMock()
magic.level1.level2.method().return_value  # No AttributeError
```

### Java: Mockito

```java
// Basic mocking
PaymentGateway gateway = mock(PaymentGateway.class);
when(gateway.debit(anyInt())).thenReturn(true);

// Verification
processor.charge(user, gateway);
verify(gateway).debit(user.account);
verify(gateway, times(2)).debit(anyInt());

// Spying on real objects
UserService realService = spy(new UserService());
doReturn(user).when(realService).find(1);
```

### Go: GoMock, testify

```go
// GoMock (interface-based)
type PaymentGateway interface {
    Debit(accountID int) error
}

// In test:
mockCtrl := gomock.NewController(t)
gateway := mocks.NewMockPaymentGateway(mockCtrl)

gateway.EXPECT().
    Debit(user.AccountID).
    Return(nil).
    Times(1)

processor.Charge(user, gateway)
mockCtrl.Finish()
```

## Conclusion

Test doubles exist on a spectrum from simple stubs to strict mocks. Choosing the right double depends on:

1. **What you're testing:** Behavior (use mocks) or state (use stubs/fakes)?
2. **Why you're testing:** Discovering bugs (sociable), verifying design (solitary), understanding behavior (spies)?
3. **External dependencies:** Real external services (mock), internal domain logic (use real objects when feasible).

Over-mocking signals design problems—not necessarily testing problems. If you find yourself mocking everything, consider refactoring to reduce coupling before writing more tests.