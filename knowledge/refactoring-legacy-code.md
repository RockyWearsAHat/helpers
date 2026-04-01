# Working Effectively with Legacy Code — Feathers' Techniques and Mindset

"Legacy Code is code without tests." — Michael Feathers' definition, which shifted the conversation from blaming age to acknowledging absence of automated feedback.

Legacy code is **high-risk to modify** because changes have no safety net. Feathers' seminal book (2004, still relevant) provides a practical playbook for incrementally bringing untested code under test and incrementally improving it.

## The Legacy Code Dilemma

**The core paradox:** To change code safely, you need tests. But to put tests in place, you must change the code first. How do you escape this catch-22?

**The solution:** Identify minimal, safe changes to make the code testable. Break the dependency chains preventing tests from running. Then, with tests in place, refactor freely.

## Identifying Seams

A **seam** is a place to alter program behavior without editing the source code. Seams are the key to writing tests for legacy code without massive refactorings first.

### Types of Seams

**Object Seam (Object-Oriented languages)**

```javascript
export class DatabaseConnector {
  connect() {
    // Real DB connection logic
  }
}

// In tests: extend or inject a fake
export class FakeDatabaseConnector extends DatabaseConnector {
  connect() {
    // Stub: no actual DB call
  }
}

class MyService {
  constructor(connector = new DatabaseConnector()) {
    this.connector = connector; // Seam: can inject fake in tests
  }
}
```

The whole class is a seam you can manipulate in tests.

**Method Seam (Procedural languages)**

Replace function calls via preprocessor directives or linker tricks (less common in modern code, but C/C++ use this).

**Parameterization Seam**

```python
def process_records(file_path="/prod/data.csv", db_connection=None):
    # Seam: default params can be overridden in tests
    if db_connection is None:
        db_connection = create_prod_connection()
    # Use file_path and db_connection
```

Callers can pass test doubles without modifying the function.

### Finding Seams

1. **Identify the hard dependency** blocking your test. Usually: file I/O, external API, database, complex object instantiation.
2. **Ask: What would I need to change to break this dependency?** Look for interfaces, inheritance hierarchies, or parameter injection points.
3. **Create a seam:** Extract an interface, add a parameter, or subclass for testing. Minimal change, reversible.

## Characterization Tests

Before you understand what code *should* do, you need to capture what it *actually* does. **Characterization tests** (also called approval tests, snapshot tests, or golden-master tests) do exactly that.

### How to Write a Characterization Test

1. **Call the legacy method with sample inputs.**
2. **Capture its output** (return value, side effects, errors).
3. **Assert the output equals the captured behavior.**
4. **If behavior changes, the test fails.** You now have evidence something broke.

```python
def test_characterize_pricing_logic():
    # Capture current behavior without understanding it
    order = LegacyOrder(quantity=100, itemPrice=10.0)
    result = order.calculate_price()
    
    # Assert it produces exactly what it produces today
    assert result == 950.0  # Captured: 100 * 10 - 50 bulk discount
    
    # Once you understand the logic, write a real unit test
    # This characterization test becomes a safety net for refactoring
```

### Advantages

- **No spec required.** You're testing "what is," not "what should be." Specs are often outdated or wrong anyway.
- **Quick safety net.** Cover legacy code rapidly without spending weeks reverse-engineering requirements.
- **Regression detection.** If refactoring breaks anything, the characterization test triggers first.
- **Documentation.** The test shows callers how the code actually behaves—often enlightening.

## Breaking Dependencies: Sprout and Wrap Techniques

When you need to add new functionality to a legacy system without refactoring the entire method, **sprout** or **wrap** the new code.

### Sprout Method

Create a focused, testable method for the new logic. Insert a call to it from the legacy code.

```javascript
class TransactionGate {
  postEntries(entries) {
    // ... large, untested method
    for (let entry of entries) {
      entry.postDate();
    }
    // ... more code
  }
}

// Problem: you need to deduplicate entries, but postEntries is hard to test

// Solution: sprout a testable method
class TransactionGate {
  uniqueEntries(entries) {
    // Isolated, testable logic
    const seen = new Set();
    return entries.filter(e => {
      if (seen.has(e.id)) return false;
      seen.add(e.id);
      return true;
    });
  }

  postEntries(entries) {
    const uniqueEntries = this.uniqueEntries(entries); // Minimal change
    for (let entry of uniqueEntries) {
      entry.postDate();
    }
    // ... rest unchanged
  }
}
```

**Why sprout works:**
- New code is isolated, independently testable.
- The diff is minimal: one method declaration, one call insertion.
- Risk is low; you're not changing the complex legacy logic.

### Sprout Class

When the sprouted logic is more substantial, extract an entire class.

```python
class ReportGenerator:
    def generate_report(self, data):
        # Complex legacy code...
        aggregates = self._calculate_aggregates(data)  # New: move to class
        return aggregates

# Sprout: independent, testable class
class DataAggregator:
    def calculate(self, data):
        return { "sum": sum(data), "avg": sum(data) / len(data) }

# Updated legacy code
class ReportGenerator:
    def __init__(self):
        self.aggregator = DataAggregator()
    
    def generate_report(self, data):
        aggregates = self.aggregator.calculate(data)
        return aggregates
```

**When to sprout a class:**
- The extracted logic has cohesion (belongs together).
- It's usable elsewhere (not just a workaround for untestable code).
- It reduces complexity of the legacy method.

### Wrap Method

When new behavior should happen *before* or *after* existing code (not within it), wrap the old method.

**Steps:**
1. Rename the old method (e.g., `postEntries` → `postEntriesThenOldImpl`).
2. Create a new method with the original name.
3. Call the old impl from new method, adding new logic before/after.
4. Write tests for the new logic.

```javascript
class PaymentProcessor {
  processPayment(amount) {
    // Old: writes to DB without logging
    this.db.insert("transactions", { amount });
  }

  // Wrap: new method handles logging + calls old impl
  processPaymentWrapped(amount) {
    this.logger.info(`Processing payment: ${amount}`);
    this.processPaymentThenOldImpl(amount);
    this.logger.info(`Payment processed`);
  }
  
  // Rename old to impl
  processPaymentThenOldImpl(amount) {
    this.db.insert("transactions", { amount });
  }
}

// Update callers to use processPaymentWrapped instead
```

**When to wrap vs. sprout:**
- **Sprout** when the new logic doesn't depend on the old method's behavior, or the new logic is self-contained.
- **Wrap** when you need to interleave new behavior with existing code (pre/post actions).

## Dependency-Breaking Techniques

### Fake Out (Subclass and Override)

Override a method in a test subclass to return a stub.

```java
// Legacy code you can't modify
public class PaymentGateway {
  public void charge(double amount) {
    ExternalService.callLiveAPI(amount); // Hard dependency
  }
}

// Test seam via subclass
public class FakePaymentGateway extends PaymentGateway {
  @Override
  public void charge(double amount) {
    // Stub: no external call
    recordedCharges.add(amount);
  }
}
```

### Sprout Parameter Dependencies

Add parameters to receive test doubles without changing the function signature in production.

```python
def fetch_user_data(user_id, http_client=None):
    if http_client is None:
        http_client = create_http_client()  # Production default
    return http_client.get(f"/users/{user_id}")

# In test, inject fake client
fake_client = FakeHttpClient()
result = fetch_user_data(123, http_client=fake_client)
```

### Extract and Override

Extract a method that creates the problematic dependency, then override it in a test subclass.

```javascript
class DataProcessor {
  process() {
    const data = this.loadData(); // Problematic dependency
    return this.analyze(data);
  }
  
  loadData() {
    return fetch("/api/data"); // Hard to test
  }
  
  analyze(data) {
    // Real logic, testable
  }
}

class TestDataProcessor extends DataProcessor {
  loadData() {
    return { /* test data */ }; // Override with stub
  }
}

// Test seam: inject test subclass
const processor = new TestDataProcessor();
processor.process(); // Uses test data
```

## Getting Tests in Place: A Strategy

1. **Write a characterization test** for the entire legacy method. This is your baseline safety net.
2. **Identify seams** — places you can break dependencies without modifying legacy code.
3. **Sprout or wrap** new behavior, unit-testing it independently.
4. **Incrementally introduce fakes**, subclasses, or dependency injection.
5. **Once you have tests, refactor** to improve structure.

## Strangler Fig for Large-Scale Legacy Replacement

For entire systems (not just functions), the **strangler fig pattern** incrementally replaces legacy functionality:

- Identify a vertical slice (e.g., "user authentication").
- Build new implementation in parallel.
- Route new requests to new system, old requests to legacy.
- Gradually shift traffic; decommission legacy when done.

This allows 24/7 operation during gradual replacement, avoiding risky "big bang" rewrites.

## Anti-Patterns to Avoid

- **Refactoring without tests first.** You'll break things and have no alarm.
- **Trying to understand everything before adding tests.** Characterization tests let you test *before* understanding.
- **Large sporuting/wrapping changes.** Keep the diff minimal. One new method, one call. If the diff is big, you've missed a seam.
- **Ignoring naming.** `FakeDatabaseConnector`, `TestUserRepository`, `MockEmailService` — make test doubles obvious.

## Mindset Shift

Legacy code isn't the enemy; lack of feedback is. Tests provide feedback. Once you have that feedback, the code becomes safe to improve. The path forward isn't "rewrite it all"; it's "get to tests incrementally, then refactor steadily."