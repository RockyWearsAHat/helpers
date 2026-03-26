# Code Quality Metrics — Complexity, Coupling, and Meaningful Measurement

Engineering metrics quantify what teams produce and how well. Code metrics specifically measure structural properties: how complex a function is, how interconnected modules are, how much change a codebase experiences. Most metrics are diagnostic tools, not optimization targets.

## Complexity Metrics

### Cyclomatic Complexity (McCabe, 1976)

Counts the number of independent execution paths through code. Each decision point (if, while, for, catch) adds one; each boolean operator (&&, ||) within a condition adds one.

```python
def classify_user(score, verified, admin):
    # CC = 1 (base)
    if score > 100:          # CC += 1 → 2
        if verified:         # CC += 1 → 3
            if admin:        # CC += 1 → 4
                return "VIP"
            else:            # (part of if admin, no new path)
                return "Verified"
    elif score > 50:         # CC += 1 → 5
        return "Regular"
    else:                    # (part of elif, no new path)
        return "Banned"
    # Final CC = 5 (5 independent paths through the function)
```

**Interpretation:**
- CC 1-4: Simple, easily testable
- CC 5-7: Moderate, needs attention
- CC 8-10: Complex, hard to maintain and test
- CC 10+: Refactor before adding features

**Limitation:** Doesn't account for how difficult it is to reason about code. A function with 10 simple if-statements for different types (easy to understand) has the same CC as a 10-level nested nightmare.

### Cognitive Complexity (Sonar, 2017)

Refined by SonarQube to measure mental load, not just path count. Scoring:
- Base: +1 per `if`, `while`, `for`, `catch`, ternary, lambda
- Nesting penalty: Each nested level +1 per decision
- Boolean operators within conditionals: Count the breaks in logic flow

```python
def process_orders(orders, date_filter, priority_filter):
    # Base complexity
    result = []
    
    for order in orders:           # +1 (loop)
        if order.date > date_filter:  # +2 (if + nesting)
            if order.priority > priority_filter:  # +3 (if + 2x nesting)
                if order.status == "pending":     # +4 (if + 3x nesting)
                    result.append(order)
    # Cognitive Complexity = 10
```

**Why it's better:** Nesting penalty reflects actual mental load. Deeply nested code is harder to understand even if the path count is the same.

**Target threshold:** 15 per function is typically reasonable; above 20 is a strong signal to refactor.

## Coupling Metrics

Two types: afferent (incoming dependencies) and efferent (outgoing dependencies).

### Afferent Coupling (Fan-in)

How many other modules depend on this module. High afferent coupling means the module is a hub; changes ripple outward.

```
PaymentService depends on:
  ├── Logger (1 dependent)
  ├── UserRepository (3 dependents)
  └── Database (10 dependents)
```

**High afferent coupling signals:**
- This module is a core abstraction (likely fine).
- Or it's a god object doing too much (likely bad).

### Efferent Coupling (Fan-out)

How many other modules this module depends on. High efferent coupling means tight coupling; changes in dependencies break this module.

```
PaymentService depends on:
  ├── Logger
  ├── UserRepository
  ├── CheckoutService
  ├── EmailNotifier
  ├── AnalyticsTracker
  └── ExternalGateway (6 total)
```

**High efferent coupling signals:**
- This module is a facade or orchestrator (sometimes acceptable).
- Or it has too many responsibilities (needs refactoring).

## Cohesion Metrics

Cohesion measures how well methods in a class/ module relate to each other.

### LCOM (Lack of Cohesion of Methods)

Counts method pairs that don't share instance variables:

```python
class UserManager:
    def __init__(self):
        self.db = Database()
        self.email_service = EmailService()
        self.user_name = ""
        self.user_email = ""
    
    # Method 1: uses user_name, user_email
    def validate_user(self):
        return self.user_name and self.user_email
    
    # Method 2: uses db, email_service
    def send_notification(self):
        self.email_service.send(self.user_email)
        self.db.log("email_sent")
    
    # Method 3: uses only user_name
    def get_initial_caps(self):
        return self.user_name[0].upper()
    
    # LCOM measures cohesion:
    # - validate_user and send_notification share no state → +1 disconnection
    # - validate_user and get_initial_caps share user_name → -1 (connected)
    # - send_notification and get_initial_caps share no state → +1 disconnection
    # High LCOM indicates low cohesion; candidate for splitting into two classes
```

**Interpretation:**
- LCOM close to 0: Methods are cohesive; class has single responsibility.
- LCOM high: Methods are disconnected; class likely violates Single Responsibility Principle.

**Action:** High LCOM → extract class, splitting unrelated responsibilities.

## Useful vs. Vanity Metrics

### Vanity Metrics (Misleading)

**Cyclomatic Complexity targets above 10** under the assumption that lower is always better. But a simple function with a 10-case switch statement (CC 11) is often clearer than a 6-level if-else (CC 6). Blind optimization toward low CC can actually reduce readability.

**Code churn (change frequency).** Raw churn says nothing about quality. A file changed 100 times might be a core, well-maintained module or a disaster. Context matters.

**Test coverage percentage.** 90% coverage is meaningless if tests are shallow (just calling methods, not asserting behavior). 60% coverage with deep, meaningful tests is more valuable.

### Metrics That Matter

**Cyclomatic + Cognitive Complexity together:** CC identifies path complexity; CC identifies mental load. Use both.

**Afferent/Efferent Coupling:** High efferent coupling in entry points (main, controllers, facades) is acceptable. High efferent coupling in core business logic suggests design problems.

**LCOM:** Strong signal of cohesion. High LCOM + high efferent coupling often means "class doing too much."

**Code churn + complexity:** A file with high CC *and* high churn is dangerous. It changes frequently and is hard to understand. Prioritize refactoring this.

**Test count + test execution time:** If you have 1000 tests running in 5 minutes, that's a healthy signal (test suite provides rapid feedback). If 100 tests take 30 minutes, feedback loop is too slow; tests need isolation (mocking, test parallelization).

## Tools and Enforcement

### SonarQube

Industry standard for code quality gatekeeping:

```yaml
# sonar-project.properties
sonar.sources=src
sonar.exclusions=**/*Test.java

# Quality gates: fail if these exceed thresholds
sonar.qualitygate.threshold.cognitive_complexity=15
sonar.qualitygate.threshold.cyclomatic_complexity=20
sonar.qualitygate.threshold.duplicated_lines_density=3
```

SonarQube analyzes code, tracks trends, and can block CI/CD if quality gates fail.

### ESLint + Plugins (JavaScript)

```javascript
// .eslintrc.js
module.exports = {
  rules: {
    "complexity": ["warn", 10],  // Warn if CC > 10
    "max-depth": ["warn", 3],    // Warn if nesting > 3
    "max-lines": ["warn", 200],  // Warn if file > 200 lines
  }
};
```

### Pylint (Python)

```python
# pylintrc
[DESIGN]
max-locals=15
max-arguments=5
max-attributes=7
max-branches=12
```

Pylint enforces naming conventions, detects unused imports, checks complexity.

### CodeClimate

Cloud-based SaaS; integrates with GitHub. Issues pull request comments on complex/duplicated code:

```
This method has a cognitive complexity of 18
Suggestions:
  - Extract to private method
  - Replace nested if with guard clauses
```

## DORA Metrics (Team-Level)

Not code-specific, but context for code quality:

- **Deployment Frequency:** How often code ships. High frequency + stable systems = good process.
- **Lead Time for Changes:** Days from commit to production. Short lead time = ability to iterate.
- **Mean Time to Recovery (MTTR):** How fast you recover from incidents. Low MTTR = good monitoring/rollback + high confidence.
- **Change Failure Rate:** % of deployments causing incidents. Low rate = good testing + code review.

High-performing teams ship frequently (weeks → days), with short lead times and low incident rates. Code metrics (complexity, coverage) are inputs to these outcomes, not substitutes.

## Interpreting Metrics: Context Matters

**High complexity in one place (controller, factory) ≠ High complexity everywhere.** It's OK for an HTTP handler to be complex (many branches for different requests). It's bad for a business logic function.

**Coupling between layers ≠ Coupling within layers.** Tightly coupled controllers to frameworks is expected. Tightly coupled business logic to database is a problem.

**Churn in stable code ≠ Churn in strategic code.** If your authentication module changes every week, that's instability. If your admin UI changes weekly, that's normal.

## Practical Workflow

1. **Run static analysis on new code** (merge requests, pull requests) before review.
2. **Set thresholds based on history:** If your codebase averages CC 8, a new function with CC 15 is an outlier worth scrutiny.
3. **Use metrics for discovery, not punishment.** Metrics identify problem areas; humans decide how to fix them.
4. **Trend metrics over time.** Is complexity increasing? Declining? Plateauing? Trends matter more than snapshots.
5. **Pair metrics with testing:** High complexity + low test coverage is a red flag. High complexity + comprehensive tests is more acceptable.
6. **Review and update thresholds as team grows:** A 10-person startup can afford lower quality than a 100-person engineering org; adjust thresholds as risk tolerance changes.

## Summary: Metrics as Conversation Starters

Code metrics are **diagnostic tools**, not optimization targets. Use them to identify hotspots, start conversations about design, and track team progress. Blindly optimizing metrics inverts priorities; use metrics to serve better code, not the reverse.