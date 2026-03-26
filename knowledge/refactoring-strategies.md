# Refactoring Strategies — Fowler's Catalog and Patterns

Refactoring is the disciplined process of changing code's internal structure without altering its external behavior. Martin Fowler's 2nd Edition refactoring catalog (2018) consolidates decades of practice into a comprehensive toolkit.

The fundamental principle: **refactor with tests in place.** Without automated feedback, refactoring is just changing code and hoping.

## Core Extraction Refactorings

### Extract Function / Extract Method

**When:** A code fragment can be grouped logically, or a comment explains what a block does.

**Purpose:** Improve readability, create reusable logic, reduce scope of local variables.

**Key decisions:**
- Extract when the method name clarifies intent better than a comment
- If the extracted piece is too small, inlining may be better
- Local variable capture complicates extraction; pass parameters instead
- Extract before refactoring: allows safe, reversible changes

**Example:** Dense calculation logic becomes a named method with clear parameters.

### Extract Class

**When:** A class is doing more than one job, or has private methods that interact with only a subset of fields.

**Process:**
1. Create new class for the subset of responsibility
2. Move data fields and methods to the new class
3. Replace internal calls with delegating to the new class
4. Expose new class or keep it as internal implementation detail

**Signals:** High Feature Envy (one method using many fields of another class), or a name that contains "and" or "or."

### Extract Variable (Introduce Explaining Variable)

**When:** An expression is hard to understand at a glance.

```javascript
// Before: unreadable
return order.quantity * order.itemPrice -
  Math.max(0, order.quantity - 500) * order.itemPrice * 0.05;

// After: clear intent
const basePrice = order.quantity * order.itemPrice;
const bulkDiscount = Math.max(0, order.quantity - 500) * order.itemPrice * 0.05;
return basePrice - bulkDiscount;
```

## Movement Refactorings

### Move Method / Move Field

**When:** A method in class A uses data or calls methods from class B more than from A (Feature Envy), or a field is used primarily by another class.

**Process:**
1. Create method/field skeleton in target class
2. Move implementation, adjusting references
3. Update calling sites to use new location
4. Delete old definition (if nothing else uses it)

**Trade-off:** Moving can expose coupling; use Move Method to discover where responsibilities actually belong.

### Move Statements into Function

**When:** Code at multiple call sites has the same prefix or suffix work (e.g., logging, validation) that should be inside the function.

**Inverse:** Move Statements to Callers — when a function's logic should remain outside to preserve flexibility.

## Replacing Complex Logic

### Replace Conditional with Polymorphism

**When:** A switch statement on a type code repeats logic across branches, making adding new types require modifying multiple places.

```javascript
// Before: type-based switching
switch (bird.type) {
  case 'EuropeanSwallow':
    return "average";
  case 'AfricanSwallow':
    return (bird.numberOfCoconuts > 2) ? "tired" : "average";
  case 'NorwegianBlueParrot':
    return (bird.voltage > 100) ? "scorched" : "beautiful";
}

// After: polymorphism
class EuropeanSwallow {
  get plumage() { return "average"; }
}
class AfricanSwallow {
  get plumage() { return (this.numberOfCoconuts > 2) ? "tired" : "average"; }
}
class NorwegianBlueParrot {
  get plumage() { return (this.voltage > 100) ? "scorched" : "beautiful"; }
}
```

**Benefits:**
- Adding new types doesn't require touching existing code (Open-Closed Principle)
- Each class encapsulates its own logic
- Subclasses can override responsibly; callers don't need to know the type

**Prerequisite:** First replace the type code with subclasses or Strategy objects. Then move logic branches into methods on each type.

### Replace Nested Conditional with Guard Clauses

**When:** A method has deeply nested if-else chains where the real work happens in the innermost block.

```javascript
// Before: nested hell
if (isActive) {
  if (hasPermission) {
    if (isValid) {
      // ... 50 lines of real work
    }
  }
}

// After: guard clauses
if (!isActive) return;
if (!hasPermission) return;
if (!isValid) return;
// ... 50 lines of real work
```

**Principle:** Guard clauses make the happy path obvious; exceptional cases exit early.

### Replace Temp with Query

**When:** A temporary variable stores an intermediate result used only locally, and recomputing it is cheap compared to complexity of passing it around.

```javascript
// Before: temp variable
let basePrice = anOrder.quantity * anOrder.itemPrice;
if (basePrice > 1000) {
  return basePrice * 0.95;
} else {
  return basePrice * 0.98;
}

// After: extract as method
function getBasePrice() {
  return anOrder.quantity * anOrder.itemPrice;
}
if (getBasePrice() > 1000) {
  return getBasePrice() * 0.95;
} else {
  return getBasePrice() * 0.98;
}

// Better: replace conditional with polymorphism or move to class method
```

**Trade-off:** Reduces local variable scope but increases method call overhead. Fine for clarity unless profiling shows it's a hotspot.

## Working with Function Signatures

### Introduce Parameter Object

**When:** A function has many parameters with conceptual relationships, or multiple functions share the same cluster of parameters.

```javascript
// Before: too many params
function getTemperatureReadings(station, from, to) { }
function reportTemps(minTemp, maxTemp, readings) { }

// After: cohesive object
class DateRange { constructor(from, to) { } }
function getTemperatureReadings(station, dateRange) { }
function reportTemps(reading, dateRange) { }
```

**Benefit:** Params with shared meaning form a true object; that object can gain its own methods and validation.

### Change Function Declaration (Rename Function, Add/Remove Parameters)

**When:** A function's name doesn't reflect its intent, or parameters no longer make sense.

**Strategy:** Rename first (with tests), then add/remove params incrementally using method overloading (in statically typed languages) or temporary delegators.

## Data Transformation

### Encapsulate Record / Encapsulate Collection

**When:** Raw data structures (maps, arrays) are manipulated throughout the codebase, making it hard to enforce invariants.

```javascript
// Before: direct manipulation
const person = { name: "Alex", age: 30 };
person.age = -5; // Bug! No validation

// After: encapsulation
class Person {
  #age;
  setAge(value) {
    if (value < 0) throw new Error("Invalid age");
    this.#age = value;
  }
}
```

### Replace Primitive with Object

**When:** A primitive value (string, number) carries domain meaning and needs validation or methods.

```javascript
// Before: string for phone
const contact = { phone: "(555) 123-4567" }; // Is this valid?

// After: PhoneNumber object with validation
class PhoneNumber {
  constructor(value) {
    if (!this.isValid(value)) throw new Error("Invalid phone");
    this.value = value;
  }
}
```

## Safe Refactoring Practice

**IDE support:** Most modern IDEs automate basic refactorings (extract function, rename, move). Use IDE refactoring commands—they're tested and reversible.

**Testing:** Run full test suite after each small refactoring. If tests pass, the refactoring is safe; if they fail, the change broke external behavior.

**Version control:** Commit before refactoring. If you get lost, `git diff` shows exactly what changed.

**Composability:** Combine small, safe refactorings to achieve large structural changes. Never try to refactor and change functionality simultaneously.

## Automated Refactoring Tools

Tools assist with refactoring safety:
- **SonarQube, ESLint, Pylint:** Identify code smells (complexity, duplication, unused variables) triggering refactoring.
- **IDE built-ins:** VS Code, IntelliJ, PyCharm handle rename, extract function, move method with semantic awareness.
- **Language-specific:** Rust's `cargo fix`, Go's `gofmt`, Python's `autopep8` enforce structural consistency.
- **AST-based:** Tool like `jscodeshift` scripts precise transformations across large codebases (e.g., API migration).

## Red Flags and When NOT to Refactor

- **Refactoring near a deadline is a risk.** Prioritize shipping over internal beautification unless the code blocks the feature.
- **Refactoring without tests is guesswork.** Write tests first; refactor only when you have feedback.
- **Refactoring for someone else's style** (tabs vs spaces, camelCase vs snake_case) is noise. Use formatters (Prettier, autopep8) automatically.
- **Micro-refactorings (renaming a variable)** in unpushed code don't need ceremony. Refactor liberally before pushing.

The goal of refactoring: **reduce complexity gradually, improve readability for future developers, and keep code cost-of-change low.**