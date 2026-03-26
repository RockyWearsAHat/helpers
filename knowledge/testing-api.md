# API Testing — Contract Testing, Mocking, Integration, and Fuzzing

## Overview

API testing verifies that service interfaces work correctly, are reliable, and integrate well. Tests range from unit-like (mock external calls) to integration (verify provider behavior) to end-to-end (verify consumer-provider contracts).

See [testing-contract.md](testing-contract.md) for contract testing depth. This note covers the broader API testing landscape: integration approaches, mocking patterns, fuzzing, and backward compatibility.

## API Testing Levels

### Unit-Like: Mocking External APIs

When testing code that calls external services (payment gateway, weather API, third-party auth), mock the external service to isolate unit tests:

```javascript
// Example: Unit test with mocked HTTP calls
import { checkout } from './checkout';
import * as stripe from './stripe-client';

jest.mock('./stripe-client');

test('checkout calls stripe with correct amount', () => {
  stripe.charge.mockResolvedValue({ id: 'ch_123', status: 'succeeded' });
  
  const result = checkout({ items: [{ price: 100 }] });
  
  expect(stripe.charge).toHaveBeenCalledWith({
    amount: 100,
    currency: 'usd',
  });
  expect(result.success).toBe(true);
});
```

**Tradeoff**: Fast and reliable, but doesn't verify the client actually calls Stripe correctly. Separate integration tests required.

### Integration: Testing Against Real Dependencies

Integration tests spin up dependent services (database, message queue, external API) or use test doubles (WireMock, Prism) that implement the API contract.

#### Using Test Doubles: WireMock

WireMock is a library and SOAP/REST API mock server. Define responses that match request patterns:

```javascript
// Define mock behavior
import WireMock from 'wiremock';

const mock = new WireMock();
mock.stub()
  .withName('Get User')
  .withRequest('GET', 'https://api.example.com/users/123')
  .withResponse('application/json', {
    id: 123,
    name: 'Alice',
    role: 'admin',
  });

// Start mock server
mock.listen(8080);

// Test against mock
const user = await fetchUser(123);  // Points to http://localhost:8080
expect(user.name).toBe('Alice');

// Verify calls
expect(mock.verifyCount(1)).toBe(true);  // Verify called once
```

**Advantages**:
- Fully under test control; can inject errors, slow responses, timeouts
- Deterministic; same request always gets same response
- No external service dependency; tests run offline

**Disadvantages**:
- Mock must match real API; drifts over time
- Doesn't catch bugs in real API behavior (e.rror handling, edge cases)

#### Using Test Doubles: Prism

Prism generates mock servers from OpenAPI specifications. If your API is defined in OpenAPI/Swagger, Prism automatically creates a mock:

```bash
# Start mock server from OpenAPI spec
prism mock openapi.yaml --listen 8080

# Requests match spec; responses are generated or hardcoded in spec
curl http://localhost:8080/users/123
# Returns response defined in openapi.yaml
```

**Advantage**: Single source of truth (spec) drives both mock and real API.

**Disadvantage**: Spec must be accurate and kept in sync with implementation.

### Integration: Contract Testing with Pact

Pact is a contract testing framework. Consumer (client) defines expected provider (server) behavior; provider verifies it implements the contract.

#### Consumer Side

```javascript
import { Pact } from '@pact-foundation/pact';

const provider = new Pact({
  consumer: 'UserDashboard',
  provider: 'UserService',
});

describe('UserService contract', () => {
  it('returns user by ID', async () => {
    await provider
      .addInteraction({
        state: 'user 123 exists',
        uponReceiving: 'GET /users/123',
        withRequest: {
          method: 'GET',
          path: '/users/123',
        },
        willRespondWith: {
          status: 200,
          body: {
            id: 123,
            name: 'Alice',
          },
        },
      })
      .executeTest(async (mockProvider) => {
        const user = await fetchUser(123, mockProvider.baseUrl);
        expect(user.name).toBe('Alice');
      });
  });

  afterAll(() => provider.finalize());  // Write contract to pact file
});
```

#### Provider Side

Provider runs the same tests against its own implementation (not mocked):

```javascript
import { Verifier } from '@pact-foundation/pact';

const verifier = new Verifier({
  providerBaseUrl: 'http://localhost:3000',
  pactUrls: ['./pacts/userdashboard-userservice.json'],
});

// Run against real provider
verifier
  .verifyProvider()
  .then((output) => console.log(output))
  .catch((error) => {
    console.error('Contract verification failed:', error);
    process.exit(1);
  });
```

**Workflow**:
1. Consumer test records contract to `pact` file
2. Contract pushed to central repository (broker)
3. Provider fetches contract, runs verification
4. Both sides confirm they agree on interactions

**Advantage**: Decouples consumer and provider; no need to run full integration test suite. Single source of truth: the Pact contract.

See [testing-contract.md](testing-contract.md) for deeper contract testing philosophy.

## API Fuzzing

Fuzzing sends random, malformed, or edge-case inputs to APIs to find crashes, hangs, or security flaws.

### How Fuzzing Works

1. **Generate inputs**: Random bytes, mutated valid inputs, grammar-based generation
2. **Send to endpoint**: POST, PUT requests with fuzzy payloads
3. **Monitor responses**: Crashes, errors, timeouts, security indicators
4. **Corpus building**: Save inputs that trigger new code paths for future fuzing runs
5. **Report findings**: Reproducible crash inputs

### Tools

#### REST Assured (Java)

REST Assured is a testing library with fuzzing support:

```java
public class APIFuzzTest {
  @Test
  public void fuzzUserEndpoint() {
    String[] payloads = {
      "{\"name\": \"\"}",
      "{\"name\": null}",
      "{\"name\": \"" + "A".repeat(10000) + "\"}",
      "{\"name\": \"<script>alert(1)</script>\"}",
      "{\"role\": -1}",
      "{\"id\": 9999999999999999999L}",
    };

    for (String payload : payloads) {
      given()
        .contentType("application/json")
        .body(payload)
      .when()
        .post("/users")
      .then()
        .statusCode(anyOf(200, 400, 422))  // Accept valid and validation errors
        .time(lessThan(5000L));  // Must respond < 5s, not hang
    }
  }
}
```

#### API Fuzzing in Security Testing Tools

OWASP ZAP and Burp Suite include fuzzing capabilities (see testing-security.md). Attack payloads (SQL injection, XSS, XXE) are injected into query params, post bodies, headers.

### Fuzzing Discipline

Fuzzing requires thoughtful validation:

1. **Define valid response space**: What status codes and shapes are acceptable? (200/400/500 all OK?)
2. **Monitor for badness**: Crashes, log errors, security warnings
3. **Reproduce crashes**: Save input payloads that cause issues for regression tests
4. **False positive filter**: Timeouts due to slow network ≠ application bug

Bad fuzzing example:
```javascript
// Too permissive; accepts any response, misses bugs
fuzz(() => {
  const res = api.post('/users', randomPayload());
  expect(res).toBeDefined();  // Passes if response exists, no matter what
});
```

Better:
```javascript
fuzz(() => {
  const res = api.post('/users', randomPayload());
  expect([200, 400, 422, 500]).toContain(res.status);  // Accept defined set
  expect(res.time).toBeLessThan(5000);  // Detect hangs
  if (res.status >= 500) fail(`Unexpected server error: ${res.body}`);
});
```

## Backward Compatibility Testing

APIs evolve. Ensure changes don't break existing clients:

### Schema Versioning

Define response schema versions. Clients pin to a version:

```http
GET /api/v1/users/123
GET /api/v2/users/123
```

**Tradeoff**: Easy to maintain multiple versions, but code duplication and maintenance burden.

### Schema Evolution Rules (Semantic Versioning for APIs)

Adopt rules for what changes are safe (backward-compatible):

| Change | Safe? | Reason |
|--------|-------|--------|
| Add optional field | Yes | Clients ignore unknown fields |
| Remove optional field | No* | Clients expecting field break |
| Rename field | No | Clients using old name break |
| Change field type (string → int) | No | Clients parsing as string fail |
| Require previously-optional field | No | Existing clients don't send it |
| Relax response validation (e.g., email → string) | Yes | Clients accept wider values |

*Deprecated fields can be removed after clients migrate (rolling release).

### Testing Backward Compatibility

```javascript
// Contract: new API must be consumable by old clients
test('v2 response parseable as v1 client expects', () => {
  const v2Response = {
    id: 123,
    name: 'Alice',
    email: 'alice@example.com',
    newField: 'v2-specific data',  // v1 client ignores
  };

  // v1 client parser
  const user = parseUserV1(v2Response);
  expect(user.id).toBe(123);
  expect(user.name).toBe('Alice');
  // Doesn't fail if newField missing; compatible
});
```

### Deprecation Workflow

1. **Add new field/endpoint**: v2 alongside v1
2. **Deprecate v1**: Send `Deprecation` header and link to migration guide
3. **Monitor usage**: Track v1 usage; set sunset window (6 months, 1 year)
4. **Migrate clients**: Work with teams to move to v2
5. **Remove v1**: After sunset window, retire

```http
HTTP/1.1 200 OK
Deprecation: true
Sunset: Wed, 31 Mar 2027 00:00:00 GMT
Link: <https://api.example.com/docs/migration>; rel="deprecation"

{ "id": 123, "name": "Alice" }
```

## API Testing Patterns

### 1. Setup and Teardown (Arrangement, Act, Assert)

```javascript
describe('User API', () => {
  let userId;

  beforeEach(async () => {
    // Setup: Create test data
    const res = await api.post('/users', { name: 'Test User' });
    userId = res.body.id;
  });

  afterEach(async () => {
    // Teardown: Clean up
    await api.delete(`/users/${userId}`);
  });

  it('updates user name', async () => {
    // Act
    const res = await api.put(`/users/${userId}`, { name: 'Updated' });
    
    // Assert
    expect(res.status).toBe(200);
    expect(res.body.name).toBe('Updated');
  });
});
```

### 2. Authentication and Authorization

```javascript
describe('Protected API', () => {
  it('requires authentication', async () => {
    const res = await api.get('/admin/stats');
    expect(res.status).toBe(401);
  });

  it('requires admin role', async () => {
    const res = await api
      .get('/admin/stats')
      .set('Authorization', `Bearer ${userToken}`);  // User, not admin
    expect(res.status).toBe(403);
  });

  it('allows admin to access', async () => {
    const res = await api
      .get('/admin/stats')
      .set('Authorization', `Bearer ${adminToken}`);
    expect(res.status).toBe(200);
  });
});
```

### 3. Error Scenarios

```javascript
describe('Error handling', () => {
  it('returns 400 on invalid JSON', async () => {
    const res = await api
      .post('/users')
      .set('Content-Type', 'application/json')
      .send('not valid json');
    expect(res.status).toBe(400);
    expect(res.body.error).toBeDefined();
  });

  it('returns 404 for missing resource', async () => {
    const res = await api.get('/users/nonexistent');
    expect(res.status).toBe(404);
  });

  it('returns 500 and logs when database fails', async () => {
    // Mock database to throw
    database.query.mockRejectedValue(new Error('Connection lost'));
    
    const res = await api.get('/users/123');
    expect(res.status).toBe(500);
    expect(logger.error).toHaveBeenCalled();
  });
});
```

### 4. Pagination and Large Responses

```javascript
test('paginated list correctly handles cursors', async () => {
  const page1 = await api.get('/items?limit=10');
  expect(page1.body.items.length).toBe(10);
  expect(page1.body.nextCursor).toBeDefined();

  const page2 = await api.get(`/items?limit=10&cursor=${page1.body.nextCursor}`);
  expect(page2.body.items[0].id).not.toBe(page1.body.items[0].id);  // Different items
});
```

## Testing Tools Summary

| Tool | Purpose | Fit |
|------|---------|-----|
| **Jest, Mocha** | Unit + integration testing | General API testing |
| **Postman/Newman** | Manual + automated API testing | Small teams, exploratory |
| **REST Assured** | BDD-style API testing (Java) | Enterprise Java |
| **Pact** | Consumer-driven contract testing | Microservices |
| **WireMock** | Mock HTTP service | Integration testing |
| **Prism** | OpenAPI mocking | Spec-first testing |
| **k6** | Load testing APIs | Performance/scale testing |
| **OWASP ZAP / Burp** | Security-focused fuzzing | Security testing |

## Integration with CI/CD

```yaml
# GitHub Actions: Run API test suite on every push
name: API Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: test
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: 18
      
      - run: npm install
      - run: npm run test:api        # Run integration tests
      - run: npm run test:contract   # Verify contract compatibility
      - run: npm run test:security   # Security fuzzing
```

## Summary

API testing spans unit (mocks), integration (test doubles, real services), and contract (consumer-provider verification) levels. Use mocks (jest.mock, WireMock) for speed; use integration tests against test doubles (Prism) for determinism; use contract testing (Pact) to decouple services. Fuzz APIs to find crashes and edge cases. Maintain backward compatibility with schema versioning and deprecation workflows. Integrate tests into CI/CD; run on every commit.