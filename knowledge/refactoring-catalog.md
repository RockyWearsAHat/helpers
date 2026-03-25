# Refactoring Catalog

## Core Philosophy

> "Refactoring is the process of changing a software system in a way that does not alter the external behavior of the code yet improves its internal structure." — Martin Fowler

**When to refactor:** Rule of Three — first time you do something, just do it. Second time, wince at the duplication. Third time, refactor.

**Prerequisites:** Always have tests before refactoring. Refactoring without tests is just changing code and hoping.

## Extract / Inline Refactorings

### Extract Function

**When:** You have a code fragment that can be grouped together, or a comment explaining what a block does.

```python
# Before
def print_invoice(invoice):
    # print banner
    print("*" * 40)
    print("** Customer Invoice **")
    print("*" * 40)
    # calculate outstanding
    outstanding = sum(o.amount for o in invoice.orders)
    # ...

# After
def print_banner():
    print("*" * 40)
    print("** Customer Invoice **")
    print("*" * 40)

def calculate_outstanding(invoice):
    return sum(o.amount for o in invoice.orders)

def print_invoice(invoice):
    print_banner()
    outstanding = calculate_outstanding(invoice)
    # ...
```

### Extract Variable (Introduce Explaining Variable)

**When:** A complex expression is hard to understand.

```javascript
// Before
return (
  order.quantity * order.itemPrice -
  Math.max(0, order.quantity - 500) * order.itemPrice * 0.05 +
  Math.min(order.quantity * order.itemPrice * 0.1, 100)
);

// After
const basePrice = order.quantity * order.itemPrice;
const quantityDiscount =
  Math.max(0, order.quantity - 500) * order.itemPrice * 0.05;
const shipping = Math.min(basePrice * 0.1, 100);
return basePrice - quantityDiscount + shipping;
```

### Inline Function / Variable

**When:** The function body is as clear as the name, or an intermediate variable adds no value.

```python
# Before
def get_rating(driver):
    return 2 if more_than_five_late_deliveries(driver) else 1

def more_than_five_late_deliveries(driver):
    return driver.number_of_late_deliveries > 5

# After (if the condition is obvious enough)
def get_rating(driver):
    return 2 if driver.number_of_late_deliveries > 5 else 1
```

### Extract Class

**When:** A class is doing two things (violates SRP). Split it.

```python
# Before: Person has phone number logic mixed in
class Person:
    def __init__(self, name, area_code, number):
        self.name = name
        self.area_code = area_code
        self.number = number

    def telephone_number(self):
        return f"({self.area_code}) {self.number}"

# After
class TelephoneNumber:
    def __init__(self, area_code, number):
        self.area_code = area_code
        self.number = number

    def __str__(self):
        return f"({self.area_code}) {self.number}"

class Person:
    def __init__(self, name, telephone):
        self.name = name
        self.telephone = telephone
```

## Moving Features

### Move Function / Field

**When:** A function references elements of another context more than its own.

### Slide Statements

**When:** Related code is scattered. Move declarations near their first use.

### Replace Loop with Pipeline

**When:** A loop does filtering, mapping, or reducing.

```javascript
// Before
const results = [];
for (const person of people) {
  if (person.department === "engineering") {
    results.push(person.name.toUpperCase());
  }
}

// After
const results = people
  .filter((p) => p.department === "engineering")
  .map((p) => p.name.toUpperCase());
```

## Simplifying Conditional Logic

### Decompose Conditional

**When:** A complex conditional (if-then-else) is hard to read.

```python
# Before
if date.before(SUMMER_START) or date.after(SUMMER_END):
    charge = quantity * winter_rate + winter_service_charge
else:
    charge = quantity * summer_rate

# After
if is_winter(date):
    charge = winter_charge(quantity)
else:
    charge = summer_charge(quantity)
```

### Consolidate Conditional Expression

**When:** Multiple conditions yield the same result.

```python
# Before
def disability_amount(employee):
    if employee.seniority < 2: return 0
    if employee.months_disabled > 12: return 0
    if employee.is_part_time: return 0
    # compute disability...

# After
def disability_amount(employee):
    if is_not_eligible_for_disability(employee):
        return 0
    # compute disability...
```

### Replace Nested Conditional with Guard Clauses

**When:** Deep nesting makes the normal path hard to see.

```python
# Before
def pay_amount(employee):
    if employee.is_separated:
        result = separated_amount()
    else:
        if employee.is_retired:
            result = retired_amount()
        else:
            result = normal_pay_amount()
    return result

# After (guard clauses)
def pay_amount(employee):
    if employee.is_separated: return separated_amount()
    if employee.is_retired: return retired_amount()
    return normal_pay_amount()
```

### Replace Conditional with Polymorphism

**When:** A switch/case or if-chain dispatches on type.

```python
# Before
def calculate_area(shape):
    if shape.type == "circle":
        return math.pi * shape.radius ** 2
    elif shape.type == "rectangle":
        return shape.width * shape.height
    elif shape.type == "triangle":
        return 0.5 * shape.base * shape.height

# After
class Circle:
    def area(self): return math.pi * self.radius ** 2

class Rectangle:
    def area(self): return self.width * self.height

class Triangle:
    def area(self): return 0.5 * self.base * self.height
```

### Introduce Special Case (Null Object)

**When:** Many places check for a special value (often null) and do the same thing.

```python
# Before — scattered null checks
if customer is None:
    name = "occupant"
else:
    name = customer.name

# After — special case object
class UnknownCustomer:
    @property
    def name(self): return "occupant"
    @property
    def billing_plan(self): return BillingPlan.basic()
```

## Organizing Data

### Replace Magic Number with Symbolic Constant

```python
# Before
if speed > 9.8: ...  # what is 9.8?

# After
GRAVITATIONAL_CONSTANT = 9.8
if speed > GRAVITATIONAL_CONSTANT: ...
```

### Replace Primitive with Object (Value Object)

**When:** A primitive carries domain meaning (phone numbers, currencies, coordinates).

```python
# Before
order.priority = "high"

# After
class Priority:
    def __init__(self, value):
        assert value in ("low", "normal", "high", "rush")
        self._value = value
    def higher_than(self, other):
        return self._index > other._index
```

### Replace Derived Variable with Query

**When:** A variable can be calculated from other data.

```python
# Before (mutable derived state)
class ProductionPlan:
    def __init__(self):
        self._adjustments = []
        self._production = 0  # derived, gets out of sync

    def add_adjustment(self, adj):
        self._adjustments.append(adj)
        self._production += adj.amount  # manual sync

# After (computed on demand)
class ProductionPlan:
    @property
    def production(self):
        return sum(a.amount for a in self._adjustments)
```

## Code Smells → Refactoring Map

| Smell                  | Refactoring                                       |
| ---------------------- | ------------------------------------------------- |
| Long Function          | Extract Function                                  |
| Large Class            | Extract Class, Extract Superclass                 |
| Long Parameter List    | Introduce Parameter Object, Preserve Whole Object |
| Divergent Change       | Extract Class (split by axis of change)           |
| Shotgun Surgery        | Move Function, Inline Class (consolidate)         |
| Feature Envy           | Move Function to the envied class                 |
| Data Clumps            | Extract Class, Introduce Parameter Object         |
| Primitive Obsession    | Replace Primitive with Object                     |
| Repeated Switches      | Replace Conditional with Polymorphism             |
| Lazy Element           | Inline Function, Inline Class                     |
| Speculative Generality | Remove Dead Code, Inline Function                 |
| Temporary Field        | Extract Class, Introduce Special Case             |
| Message Chains         | Hide Delegate, Extract Function                   |
| Middle Man             | Remove Middle Man, Inline Function                |
| Comments (deodorant)   | Extract Function, Rename                          |

## Refactoring Safety Checklist

1. **Tests pass** before you start
2. **Small steps**: One refactoring at a time, test after each
3. **Commit frequently**: Each refactoring is a commit point
4. **IDE support**: Use automated refactorings when available (rename, extract, move)
5. **No behavior change**: If tests change, you're not refactoring — you're rewriting
6. **Reverse if wrong**: Every refactoring has an inverse (Extract ↔ Inline, Move up ↔ Move down)

---

_Primary source: Martin Fowler's "Refactoring" (2nd edition, 2018). Supplemented with practical examples across languages._
