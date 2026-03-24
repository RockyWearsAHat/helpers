# Design Patterns (Gang of Four + Modern Additions)

The 23 classic patterns from Gamma, Helm, Johnson & Vlissides (1994), plus modern additions. Each pattern solves a recurring design problem.

## Creational Patterns — How objects are created

### Singleton
Ensure a class has exactly one instance with a global access point. Use for: configuration, connection pools, loggers. **Caution:** Acts as global state. Makes testing harder. Prefer dependency injection when possible.

### Factory Method
Define an interface for creating objects, but let subclasses decide which class to instantiate. Defers instantiation to subclasses. Use when: the exact type isn't known until runtime.

### Abstract Factory
Create families of related objects without specifying concrete classes. Use when: you need to support multiple platforms/themes/databases with interchangeable component sets.

### Builder
Construct complex objects step by step. Separate construction from representation. Use when: objects require many optional parameters, or the construction process has multiple steps. Modern usage: fluent APIs (`query.select("name").where("age > 18").limit(10)`).

### Prototype
Create new objects by cloning existing ones. Use when: object creation is expensive and objects differ only slightly. Languages with `clone()` or spread operators (`{...obj}`) make this trivial.

## Structural Patterns — How objects are composed

### Adapter
Convert the interface of a class into another interface clients expect. Wraps an incompatible object to make it work with existing code. Use when: integrating third-party libraries or legacy systems.

### Bridge
Separate an abstraction from its implementation so both can vary independently. Use when: you want to avoid a cartesian explosion of subclasses (e.g., Shape × Color).

### Composite
Compose objects into tree structures to represent part-whole hierarchies. Clients treat individual objects and compositions uniformly. Use for: file systems, UI component trees, organizational charts.

### Decorator
Attach additional responsibilities to an object dynamically. Alternative to subclassing for extending behavior. Use for: logging, caching, authentication middleware, stream transformations.

### Facade
Provide a simplified interface to a complex subsystem. Hides internal complexity behind a clean API. Does not add new functionality — just makes existing functionality easier to use.

### Flyweight
Share common state across many objects to minimize memory usage. Use when: large numbers of similar objects exist (characters in a text editor, particles in a game).

### Proxy
Provide a surrogate or placeholder for another object. Controls access to the original. Types: **lazy proxy** (deferred initialization), **protection proxy** (access control), **remote proxy** (network calls), **caching proxy**.

## Behavioral Patterns — How objects communicate

### Observer (Pub/Sub)
Define a one-to-many dependency where when one object changes state, all dependents are notified. Foundation for event systems, reactive programming, and MVC. Modern variants: RxJS Observables, event emitters, signal-based reactivity.

### Strategy
Define a family of algorithms, encapsulate each one, and make them interchangeable. Use when: you need to select an algorithm at runtime. Example: different sorting strategies, payment processors, compression algorithms.

### Command
Encapsulate a request as an object, allowing parameterization, queuing, logging, and undo. Use for: undo/redo systems, task queues, macro recording, transaction management.

### Iterator
Provide a way to access elements of a collection sequentially without exposing the underlying representation. Built into most modern languages (`for...of`, `for...in`, Python iterators, Rust `IntoIterator`).

### Mediator
Define an object that encapsulates how a set of objects interact. Promotes loose coupling by preventing objects from referring to each other explicitly. Use for: chat rooms, air traffic control, form field interactions.

### State
Allow an object to alter its behavior when its internal state changes. The object appears to change its class. Use when: behavior depends heavily on state and state transitions are explicit (TCP connection, order lifecycle, UI modes).

### Template Method
Define the skeleton of an algorithm in a base class, letting subclasses override specific steps without changing the algorithm's structure. Use for: frameworks with customizable hooks.

### Visitor
Represent an operation to be performed on elements of an object structure. Add new operations without modifying the classes. Use for: compiler AST processing, document format converters, report generation on complex structures.

### Chain of Responsibility
Pass a request along a chain of handlers. Each handler decides to process the request or pass it to the next handler. Use for: middleware pipelines (Express.js, Django), event bubbling, logging chains.

### Memento
Capture and externalize an object's internal state so it can be restored later. Use for: undo/redo, checkpointing, transactional rollback.

### Interpreter
Define a grammar and an interpreter for a language. Use for: DSLs, query languages, regular expression engines, math expression evaluators.

## Modern Patterns (Post-GoF)

### Repository
Mediates between the domain and data mapping layers. Abstracts data access behind a collection-like interface. Standard in DDD (Domain-Driven Design).

### Dependency Injection (DI)
Supply dependencies from outside rather than creating them internally. Enables testability and loose coupling. Frameworks: Spring (Java), .NET DI, Angular, inversify (TS).

### Middleware / Pipeline
Chain processing steps where each step can transform the request/response or short-circuit. Core pattern in web frameworks (Express, Koa, ASP.NET, Django).

### Circuit Breaker
Prevent cascading failures in distributed systems. After N failures, "open" the circuit — immediately reject requests for a cooldown period. Then "half-open" to test recovery.

---

*Sources: Gamma, Helm, Johnson & Vlissides (Design Patterns, 1994), Martin Fowler (Patterns of Enterprise Application Architecture), refactoring.guru, sourcemaking.com*
