# Core Software Engineering Principles

Universal principles that apply across all languages, paradigms, and project sizes.

## SOLID Principles (Robert C. Martin)

### Single Responsibility Principle (SRP)
A class/module should have one, and only one, reason to change. Each unit of code should do one thing well. When a module serves multiple concerns, changes to one concern risk breaking the other.

### Open/Closed Principle (OCP)
Software entities should be open for extension but closed for modification. Add new behavior by writing new code (inheritance, composition, plugins), not by editing existing working code.

### Liskov Substitution Principle (LSP)
Subtypes must be substitutable for their base types without altering program correctness. If `S` extends `T`, any code expecting `T` must work correctly with `S`. Violations: throwing unexpected exceptions, strengthening preconditions, weakening postconditions.

### Interface Segregation Principle (ISP)
No client should be forced to depend on methods it doesn't use. Prefer many small, specific interfaces over one large general-purpose interface. Fat interfaces couple unrelated consumers.

### Dependency Inversion Principle (DIP)
High-level modules should not depend on low-level modules. Both should depend on abstractions. Abstractions should not depend on details — details should depend on abstractions. Use dependency injection, service locators, or factory patterns.

## DRY — Don't Repeat Yourself
Every piece of knowledge must have a single, unambiguous, authoritative representation within a system. Duplication is not just repeated code — it's repeated *knowledge*. Two functions that happen to look the same but represent different concepts are NOT duplication. Premature DRY (abstracting too early) creates coupling worse than the duplication it removes.

## KISS — Keep It Simple, Stupid
The simplest solution that works is usually the best. Complexity is the enemy of reliability. Every line of code is a liability — it must be understood, tested, and maintained. Before adding an abstraction, ask: "Does this make the code easier to understand for the next person?"

## YAGNI — You Aren't Gonna Need It
Don't build features or abstractions for hypothetical future requirements. Implement things when you actually need them, not when you foresee that you might. The cost of building something you don't need includes: building it, testing it, documenting it, and maintaining it forever.

## Separation of Concerns (SoC)
Divide a program into distinct sections, each addressing a separate concern. A concern is a set of information that affects the code. Examples: business logic vs. data access vs. presentation. MVC, MVVM, hexagonal architecture, and microservices are all applications of SoC.

## Composition Over Inheritance
Favor object composition (has-a) over class inheritance (is-a). Inheritance creates tight coupling between parent and child. Composition allows mixing behaviors flexibly at runtime. Inheritance hierarchies deeper than 2-3 levels are almost always a design smell.

## Law of Demeter (Principle of Least Knowledge)
A method should only talk to its immediate friends — not to strangers. Avoid chaining calls like `a.getB().getC().doSomething()`. Each unit should have limited knowledge about other units.

## Convention Over Configuration (CoC)
Provide sensible defaults so developers only need to specify unconventional aspects. Reduces boilerplate, speeds onboarding, and creates consistency. Always allow overriding conventions when needed.

## Principle of Least Astonishment (POLA)
Code should behave the way most developers would expect. Function names should describe what they do. Side effects should be obvious. Default behaviors should be safe.

## Fail Fast
Detect and report errors as early as possible. Validate inputs at system boundaries. Use type systems, assertions, and precondition checks. A bug caught at compile time costs orders of magnitude less than one caught in production.

## Encapsulation
Hide internal implementation details and expose only what's needed. Public APIs should be minimal and stable. Internal state accessible only through controlled interfaces. This protects invariants and allows changing implementation without breaking consumers.

## Cohesion and Coupling
- **High cohesion**: Related functionality lives together. Each module does one conceptual thing.
- **Low coupling**: Modules have minimal dependencies on each other. Changes in one module don't cascade.

---

*Sources: Robert C. Martin (Clean Architecture, SOLID), Andy Hunt & Dave Thomas (The Pragmatic Programmer), Erich Gamma et al. (GoF), Kent Beck (XP/YAGNI), Karl Lieberherr (Law of Demeter)*
