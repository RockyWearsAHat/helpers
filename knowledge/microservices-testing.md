# Microservices Testing — Pyramid, Contract Testing, Integration, CDC & Test Containers

## Testing Pyramid: The Distributed Layer

The traditional testing pyramid (unit → integration → e2e) becomes more complex in microservices:

```
            ▲
            │ End-to-End (few, slow, expensive)
            │
            ├ Service Integration (moderate)
            │
            ├ Service-Level (many fast tests)
            │
            └ Unit Tests (thousands, instant)
```

### Unit Tests (Foundation)

Test individual functions/methods in isolation, mocking all external dependencies. Fast (ms), run on every commit, catch bugs cheaply.

**In microservices:** Each service still has unit tests. Same principles as monolith. Mocking becomes critical—mock external service calls to stay fast.

### Service-Level Integration Tests

Test a single service in isolation, with real database and message broker (but other services mocked).

**Purpose:** Verify service behavior under realistic conditions without full system.

Example:

```python
# Test OrderService alone; mock PaymentService
@test
def test_order_creation_reserves_inventory():
    # Setup: real OrderService database
    db = start_test_database()
    service = OrderService(db, mock_payment_client)
    
    # Action
    response = service.create_order(customer_id=1, items=[...])
    
    # Assert: verify database changed correctly
    assert db.get_order(response.order_id).status == "CONFIRMED"
    assert db.get_reservation(...).quantity == expected
```

**Tools:** Test containers (Testcontainers library), embedded databases, in-memory message brokers for testing.

### Contract Testing: Verifying Service Pairs

Verify that two services' interactions match expectations **without running both services**. Consumer-driven contracts define the contract; both services test independently against it.

**Instead of:**
```
↑ Spin up both services + run integration test (slow, fragile)
```

**Do:**
```
Consumer:  writes test → generates contract artifact
Provider:  downloads contract → verifies implementation matches
↑ No need to coordinate; both run independently, fast
```

#### Consumer-Driven Contract (CDC) Workflow

**(1) Consumer specifies expected behavior:**

```python
# What the consumer (OrderService) expects from PaymentService
contract = {
    "description": "charge credit card",
    "request": {
        "method": "POST",
        "path": "/charge",
        "body": {"customer_id": 1, "amount": 50.00}
    },
    "response": {
        "status": 200,
        "body": {"charge_id": "...", "status": "PROCESSED"}
    }
}
```

**(2) Consumer generates and publishes contract:**

```bash
$ pact-generate contract.json → contract.json in shared repo/broker
```

**(3) Provider verifies it satisfies the contract:**

```python
# PaymentService tests itself against the contract
provider_verifier.verify_contract("contract.json")
```

If PaymentService can't satisfy it, provider fails the verify step pre-deployment.

**(4) Deployment gates:**

- Consumer can deploy if provider's verify passed
- Provider can deploy if all consumer contracts still pass

#### Types of Contracts

**Request-Response:** HTTP synchronous calls (most common).

```
"GET /api/users/42" → 200 {"name": "Alice", "email": "alice@..."}
```

**Message/Event:** Asynchronous events.

```
"inventory.reserved event includes: order_id, item_ids, quantities"
```

**Tools:** Pact (multi-language), Spring Cloud Contract (JVM), Jest/Mocha (JavaScript).

### Service Integration Tests

Test small groups of services together (not full system). Example: OrderService + PaymentService + InventoryService, but mock NotificationService.

**When to do:**
- Multiple-service workflows that can't be tested via contract alone
- Resource contention scenarios (database pools, message broker throughput)
- Eventual consistency behavior

**Tradeoff:** Slower than contract tests but faster than full e2e; catches integration issues without full system complexity.

---

## End-to-End Testing: Full System

Spin up all services, run workflows, verify outcomes. Slow and fragile (many moving parts).

**Best uses:**
- Critical user journeys (shouldn't break)
- Catch integration surprises (contracts + service tests may pass but e2e fails)
- Performance/load testing

**Anti-pattern:** Treating e2e as primary testing strategy. Should be 5-10% of tests by count, not 40%.

### Reducing E2E Brittleness

**Use Test Environments:** Dedicated test environment per development branch or feature. Parallel test environments reduce contention.

**Data Isolation:** Each test run gets fresh data; tests don't interfere. Can use:
- Fresh database per test run (slower but safest)
- Database transactions that rollback after test (faster)
- Data cleanup fixtures (fastest but error-prone)

**Timeouts & Retries:** Network is unreliable. Tests should retry on transient failures:

```python
retry_on_timeout(lambda: assert_resource_exists(response_id), max_attempts=3)
```

**Avoid Test Interdependence:** Tests should be runnable in any order, in parallel.

---

## Change Data Capture (CDC) Testing

Verify that data propagates correctly between services via CDC pipelines (usually event streams or database replication).

### CDC Test Pattern

```python
@test
def test_customer_email_propagates_via_cdc():
    # Setup: Customer service database and CDC pipeline
    customer_service_db = start_database()
    cdc_pipeline = start_kafka_pipeline()
    cache_sink = start_redis_cache_sink(cdc_pipeline)
    
    # Action: Update in customer service
    customer_service_db.update_customer(id=1, email="newemail@...")
    
    # Assert: Changes appear downstream
    wait_for(lambda: cache_sink.get_customer(1)["email"] == "newemail@...")
```

**Key patterns:**
- Use test containers to run CDC infrastructure locally
- Set short timeouts to detect slow propagation
- Verify both schema migrations and data values propagate

---

## Test Containers & Fixture Management

**TestContainers** (Java/Python/Go library) spins up real infrastructure (Postgres, Kafka, Redis) in Docker for each test run.

### Advantages

- Real service behavior (no mocking quirks)
- Repeatable (fresh container each run)
- Isolated (parallel tests, no cross-contamination)

### Example

```python
@pytest.fixture
def postgres():
    container = PostgresContainer()
    container.start()
    yield container
    container.stop()

@test
def test_order_persistence(postgres):
    db = postgres.get_connection()
    # Real Postgres; not a mock
```

### Startup Performance

Cold-starting a container adds 5-10 seconds. Optimize by:

- Using shared containers for entire test suite (not per-test)
- Lazy initialization (start only when needed)
- Lightweight images (Alpine instead of Ubuntu)

---

## Test Doubles for Services

When testing one service, others are mocked. Types of mocks:

| Type | Behavior |
|------|----------|
| **Stub** | Returns fixed response; no logic |
| **Mock** | Verifies correct calls were made; returns responses |
| **Fake** | Working implementation; not production-grade (in-memory database instead of Postgres) |
| **Spy** | Records calls; delegates to real implementation |

### Service Mocking Strategies

**Direct mocking at client level:**

```python
# OrderService tests
payment_client = MockPaymentClient()
service = OrderService(db, payment_client)
payment_client.set_response(charge_result=success)
```

**HTTP mock servers** (WireMock, Prism):

```yaml
# WireMock stub: when called with POST /charge, return 200
POST /charge:
  response:
    status: 200
    body: {"charge_id": "...", "status": "PROCESSED"}
```

Start WireMock server; route real HTTP calls to it during tests.

**Choosing approach:**
- Direct mocking: simpler, faster, less realistic
- HTTP mock servers: more integration-like, catches serialization issues, slower

---

## Testing Complex Workflows (Sagas)

Saga workflows (compensation logic, event chains) are hard to test. Use layered approach:

**Layer 1: Unit tests** — Test compensation functions in isolation.

```python
def test_refund_compensates_charge():
    assert refund(charge_id=1) reverses the charge
```

**Layer 2: Saga state machine tests** — Test workflow orchestrator without calling real services.

```python
def test_saga_rollback_on_inventory_failure():
    saga = OrderSaga(mock_payment, mock_inventory)
    saga.handle_event("InventoryFailed")
    assert saga.published_events include "PaymentRefunded"
```

**Layer 3: Integration tests** — Test saga with real services.

```python
def test_order_fulfillment_saga_e2e():
    # Real services, real databases via test containers
    # Verify payment → inventory → shipping flow succeeds
    # Verify compensation path works on failure
```

---

## Testing in Production

Microservices architectures can support production testing:

**Feature flags:** Deploy partially-built features; enable for subset of users.

**Canary deployments:** Route 5% traffic to new version; observe metrics. Gradually increase.

**Synthetic monitoring:** Automated tests that call production APIs periodically; alert on failures. Catches issues real users encounter.

**Chaos engineering:** Deliberately break components (kill services, inject latency) in production to verify resilience. Tools: Gremlin, LitmusChaos.

**Tradeoffs:** Production testing is powerful but risky. Requires strong monitoring, incident response, and team discipline.

---

## Test Strategy Summary

| Layer | Speed | Coverage | When to Use |
|-------|-------|----------|------------|
| **Unit** | 1-10ms | 70% | Every feature; core logic |
| **Contract** | 100ms | 80% | Service interfaces |
| **Service Integration** | 1-5s | 85% | Complex workflows |
| **E2E** | 10-60s | 95% | Critical user journeys |
| **Production** | Real | 100% | Ongoing health; edge cases |

**Ideal distribution:** 60% unit, 20% contract/service integration, 10% e2e, 10% production.

---

## References & Related Concepts

See also: contract testing (Pact), test containers, CDC patterns, chaos engineering, distributed tracing, synthetic monitoring.