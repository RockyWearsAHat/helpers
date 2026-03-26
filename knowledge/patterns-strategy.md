# Strategy Pattern — Algorithm Selection, Runtime Polymorphism & Composition

The Strategy pattern encapsulates a family of algorithms, making each one independently replaceable at runtime. Instead of encoding multiple algorithms as conditional branches within a single class, each algorithm lives in its own strategy object. The context delegates to the strategy it holds.

## Core Structure

Three participants define the pattern:

**Context** — Maintains a reference to a concrete strategy. Accepts strategy objects (often via constructor or setter). Delegates work to the strategy without caring which one it gets. Does not know how the algorithm works, only that the strategy implements the expected interface.

**Strategy Interface** — Declares the contract all concrete strategies must follow. Typically a single method (e.g., `execute()`, `calculate()`, `process()`). The context calls this method.

**Concrete Strategies** — Implement the strategy interface. Each encapsulates a specific algorithm variant. Algorithms can differ drastically; the only requirement is conformance to the interface.

## Motivation: From Conditionals to Composition

Without the pattern, algorithm selection looks like this:

```python
class Navigator:
    def plan_route(self, origin, destination, mode):
        if mode == "car":
            return self._fastest_route(origin, destination)
        elif mode == "public_transit":
            return self._transit_route(origin, destination)
        elif mode == "bike":
            return self._scenic_route(origin, destination)
        else:
            raise ValueError("Unknown mode")
```

This creates several problems:

- The class grows with each new algorithm
- Editing existing algorithms risks breaking others
- Testing requires mocking the entire navigator
- Adding a new algorithm requires modifying the context
- The class violates single responsibility (logic selection + algorithm execution)

With Strategy, the Navigator delegates to a strategy object:

```python
class Navigator:
    def __init__(self, routing_strategy):
        self.strategy = routing_strategy
    
    def plan_route(self, origin, destination):
        return self.strategy.compute(origin, destination)
    
    def set_strategy(self, routing_strategy):
        self.strategy = routing_strategy

class FastestRouteStrategy:
    def compute(self, origin, destination):
        # Dijkstra or similar
        pass

class TransitStrategy:
    def compute(self, origin, destination):
        # Public transit graph traversal
        pass
```

Adding a new algorithm is now decoupled: create a new strategy class, no changes to Navigator.

## Function-Based Strategies

Modern languages blur the line between Strategy and simple functions. In languages with first-class functions (Python, JavaScript, Java 8+), you don't need strategy classes at all:

```python
navigator = Navigator(strategy=fastest_route_algorithm)

# Or pass a lambda:
navigator = Navigator(strategy=lambda orig, dest: compute_scenic_route(orig, dest))
```

This eliminates boilerplate but sacrifices explicit interfaces (harder to discover what parameters a strategy needs) and static type checking (in languages that support it).

## Strategy vs. Dependency Injection

Strategy and dependency injection overlap but solve different problems:

**Dependency Injection** — Your class declares all dependencies upfront; a container wires them at initialization. Focuses on *decoupling the graph of object dependencies*.

**Strategy** — Your class defines a single contract (the strategy interface) and swaps implementations at runtime. Focuses on *selecting a specific algorithm variant*.

You can combine them: a DI container injects the appropriate strategy into a context object.

## Examples and Use Cases

**Pricing Strategies** — E-commerce system with multiple pricing rules: bulk discount, loyalty pricing, promotional pricing, regional pricing. Each is a strategy.

```python
class ShoppingCart:
    def __init__(self, pricing_strategy):
        self.strategy = pricing_strategy
    
    def total(self, items):
        return self.strategy.calculate(items)

class BulkDiscountStrategy:
    def calculate(self, items):
        subtotal = sum(item.price * item.qty for item in items)
        if sum(item.qty for item in items) > 100:
            return subtotal * 0.9
        return subtotal
```

**Sorting/Filtering** — Data processing where the algorithm varies by context. QuickSort vs. MergeSort, filter by price vs. by date. Each algorithm is a strategy.

**Compression Algorithms** — File handling where compression method is chosen at runtime based on file type or user preference.

**Validation Rules** — Different validation strategies for different user types (customer, admin, guest).

## Trade-offs and Limitations

**Strengths:**
- Open/Closed Principle: new strategies added without modifying context
- Isolates algorithm details; context remains simple
- Runtime switching without recreating objects
- Each strategy is independently testable
- Eliminates large conditional chains

**Weaknesses:**
- Creates many small classes even for simple algorithms
- Client code must know about Strategy instances; the context doesn't hide this complexity
- Strategy interface must be generic enough to fit all algorithms; can become bloated or overly abstract
- Unnecessary for systems with only one or two algorithms that never change

## Anti-patterns and Pitfalls

**Over-strategizing** — Not every algorithm variant needs a strategy class. If algorithms are stable and unlikely to change, keep them as functions or methods. If there's only a vague interface (each algorithm has different parameters), this is a sign Strategy may not fit.

**God Strategy** — A strategy interface with 50 parameters to accommodate all algorithms. This signals poor abstraction; reconsider the problem domain.

**Strategy proliferation without coordination** — 20 pricing strategies but no way to compose them (apply tax, then discount, then coupon). Consider a chain-of-responsibility approach.

**Serialization issues** — If strategies must be persisted (saved to a database), you need a way to serialize/deserialize arbitrary strategy implementations. This can become complex.

## Related Patterns

- **State** — Superficially similar (both use composition and delegate to helper objects), but State allows the helper to modify the context's state. Strategy keeps context immutable.
- **Template Method** — Inheritance-based approach to algorithm variants. Strategy is composition-based.
- **Dependency Injection** — Often used together; DI wires the appropriate strategy.
- **Command** — Also parameterizes behavior, but Command is about *operations* (deferred execution, undo) rather than *algorithm families*.
- **Chain of Responsibility** — When algorithms need to be tried in sequence until one succeeds.

## Modern Perspectives

In functional and modern OO languages, Strategy's value proposition has shifted. First-class functions, closures, and higher-order functions reduce the bureaucratic cost of traditional Strategy class hierarchies. The pattern is less about "use classes for algorithms" and more about "identify the varying algorithm and make it pluggable."

Even so, the pattern's core insight—*separate algorithm selection from algorithm execution*—remains valid. Whether you implement it as explicit classes or as function parameters is a matter of language idiom and project style.