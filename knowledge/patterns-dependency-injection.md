# Dependency Injection Patterns

Dependency injection (DI) is a design pattern that removes the responsibility of object creation and wiring from a class, delegating it to an external container or factory. Instead of a class creating its dependencies (tight coupling), the class receives them from the outside. This decouples classes, improves testability, and makes systems more flexible and maintainable.

## Core Concepts

### Inversion of Control (IoC)

Dependency injection is a specialized form of IoC. Rather than your code controlling the flow of dependency creation ("I will create my own collaborators"), you surrender control to a container ("tell me what I need and I'll provide it"). The container resolves the dependency graph, handles object lifecycles, and performs wiring.

### Service Locator (Antipattern)

A service locator is a registry where objects ask for dependencies: `PaymentService payment = ServiceLocator.get(PaymentService.class)`. While it reduces constructor noise, it:

- Hides dependencies (they're buried in method bodies)
- Makes testing harder (you must mock the locator)
- Makes the code fragile (a missing dependency fails at runtime, not startup)
- Creates implicit coupling to the locator

**Prefer injection.** It makes dependencies explicit and catches wiring errors at container initialization time, not request time.

## Injection Techniques

### Constructor Injection

Dependencies are passed via the constructor. **Recommended approach.**

```java
class OrderService {
    private final PaymentGateway payment;
    private final Warehouse warehouse;
    
    public OrderService(PaymentGateway payment, Warehouse warehouse) {
        this.payment = Objects.requireNonNull(payment);
        this.warehouse = Objects.requireNonNull(warehouse);
    }
    
    public void processOrder(Order order) {
        payment.process(order.getTotal());
        warehouse.reserve(order.getItems());
    }
}
```

**Pros:**
- Dependencies are explicit and immutable — all required collaborators visible at instantiation
- Impossible to construct an incomplete object
- Easy to pass different implementations for testing
- Thread-safe by default (immutable fields)

**Cons:**
- Constructor parameter list can grow long (sign that the class has too many responsibilities)

### Property Injection (Setter Injection)

Dependencies are set via public setter methods after construction.

```java
class OrderService {
    private PaymentGateway payment;
    private Warehouse warehouse;
    
    public void setPayment(PaymentGateway payment) {
        this.payment = payment;
    }
    
    public void setWarehouse(Warehouse warehouse) {
        this.warehouse = warehouse;
    }
}
```

**Pros:**
- Shorter constructor parameter lists
- Can provide default implementations or make dependencies optional

**Cons:**
- Object can be partially initialized (state is mutable and fragile)
- Dependencies are hidden — reader must search the class to find all setters
- Thread-safety concerns if setters are called after object is in use
- Container doesn't guarantee all required dependencies are set

**Use case:** Optional dependencies or when constructor lists become unmanageable. Combine with constructor injection where possible: required via constructor, optional via setters.

### Method Injection

Dependencies are passed to the method that uses them.

```java
class OrderService {
    public void processOrder(Order order, PaymentGateway payment, Warehouse warehouse) {
        payment.process(order.getTotal());
        warehouse.reserve(order.getItems());
    }
}
```

**Pros:**
- Highly flexible — different dependencies for different calls
- Clean constructor

**Cons:**
- Exposes infrastructure to callers (they must know how to construct dependencies)
- Not suitable for long-lived objects

**Use case:** Utility methods, command handlers, or stateless processors. Rarely used in typical application code.

### Interface Injection

The service exposes a setter interface that the container calls.

```java
interface PaymentInjecter {
    void setPayment(PaymentGateway payment);
}

class OrderService implements PaymentInjecter {
    private PaymentGateway payment;
    
    @Override
    public void setPayment(PaymentGateway payment) {
        this.payment = payment;
    }
}
```

**Cons:**
- Verbose boilerplate
- Not widely used; superseded by annotations and reflection

## Dependency Injection Containers

Containers automate object creation, dependency resolution, and lifecycle management. They read configuration (XML, annotations, code) and construct objects on demand.

### Spring Framework (Java)

```java
// Annotation-based configuration
@Configuration
public class OrderConfig {
    @Bean
    public PaymentGateway paymentGateway() {
        return new StripePaymentGateway();
    }
    
    @Bean
    public OrderService orderService(PaymentGateway payment, Warehouse warehouse) {
        return new OrderService(payment, warehouse);
    }
}

// Usage
@Component // Spring creates and manages this
class OrderController {
    @Autowired // Spring injects dependencies
    private OrderService orderService;
}
```

**Strengths:** Mature, extensive ecosystem, excellent for enterprise Java.

### .NET Dependency Injection

```csharp
var services = new ServiceCollection();
services.AddSingleton<PaymentGateway>(new StripePaymentGateway());
services.AddScoped<OrderService>();
services.AddScoped<OrderController>();

var provider = services.BuildServiceProvider();
var controller = provider.GetRequiredService<OrderController>();
```

**Strengths:** Built into .NET, minimal boilerplate, type-safe.

### Dagger (Java/Android)

```java
@Component
interface AppComponent {
    OrderService orderService();
}

@Module
class OrderModule {
    @Provides
    PaymentGateway providePayment() {
        return new StripePaymentGateway();
    }
}
```

**Strengths:** Compile-time dependency graph validation, zero reflection overhead, excellent for Android.

### Guice (Java)

```java
class OrderModule extends AbstractModule {
    @Override
    protected void configure() {
        bind(PaymentGateway.class).to(StripePaymentGateway.class);
        bind(OrderService.class);
    }
}

Injector injector = Guice.createInjector(new OrderModule());
OrderService service = injector.getInstance(OrderService.class);
```

**Strengths:** Lightweight, straightforward API, good for modular applications.

## Object Lifetimes

Containers manage when objects are created and destroyed. Common lifetimes:

| Lifetime   | Behavior                                       | Best For                                    |
| ---------- | ---------------------------------------------- | ------------------------------------------- |
| **Transient** | New instance every request                 | Stateless services, no caching needed        |
| **Scoped**   | One instance per scope (e.g., per HTTP request) | Request-scoped state, DbContext, UOW        |
| **Singleton** | One instance for the entire application     | Shared infrastructure (cache, config, logger)|

```java
@Scope
@Bean
class RequestScopedService {}  // One per request in web context

@Bean
class SingletonService {}  // One for the app; Spring caches it
```

**Pitfall:** Singleton objects holding mutable scoped state. Example: a singleton service storing per-request user identity as a field. Results in cross-request data leaks.

## DI and Testing

Dependency injection makes testing straightforward — substitute real implementations with mocks or test doubles.

```java
class OrderServiceTest {
    @Test
    public void processOrderChargesPayment() {
        PaymentGateway mockPayment = mock(PaymentGateway.class);
        Warehouse mockWarehouse = mock(Warehouse.class);
        
        OrderService service = new OrderService(mockPayment, mockWarehouse);
        service.processOrder(testOrder);
        
        verify(mockPayment).process(eq(100.0));
    }
}
```

Without DI, you'd need to mock static methods or global state — much harder and fragile.

## When DI Hurts

### Over-Engineering Simple Code

Not every class needs dependency injection. A utility or stateless formatter can simply create what it needs:

```java
// Don't inject this
class StringUtils {
    public static String capitalize(String s) {
        return s.substring(0, 1).toUpperCase() + s.substring(1);
    }
}

// Use it directly
String result = StringUtils.capitalize("hello");  // Fine
```

Injecting `StringUtils` adds ceremony without benefit.

### Deep Dependency Chains

If A needs B, B needs C, C needs D, etc., the container can resolve it—but the design stinks. Deep chains indicate classes have too many responsibilities.

```
OrderService → PaymentGateway → BankService → SecurityProvider → CertificateManager

Too deep. Split responsibilities.
```

### Constructor Parameter Explosion

```java
class OrderService {
    public OrderService(
        PaymentGateway payment,
        Warehouse warehouse,
        NotificationService notify,
        AuditLog audit,
        Configuration config,
        // ...15 more...
    ) { }
}
```

**This is a design problem.** Constructor DI makes it visible: you need too much. Refactor into smaller, focused classes.

### Container Magic and Discoverability

Reflection-based containers (Spring, Guice) can inject objects by convention, but the wiring is implicit and invisible in the code. Makes the system harder to trace:

```java
@Component
class PaymentService {}

@Component
class OrderService {
    @Autowired
    private PaymentService payment;  // Where does this come from? Not obvious.
}
```

Trade-off: reduce boilerplate vs. explicitness.

## Best Practices

- **Use constructor injection by default.** It forces you to be explicit about dependencies and makes the class usable outside the container.
- **Avoid circular dependencies.** If A needs B and B needs A, one should use property injection or the design needs rethinking.
- **Keep scopes simple.** Understand transient vs. scoped vs. singleton. Misunderstanding lifetimes causes subtle bugs.
- **Don't inject the container itself.** If a class asks for the container (service locator), you've lost DI's benefits.
- **Test with real dependencies, not mocks, when possible.** Mocks hide design issues. Real objects reveal dependency problems.
- **Use DI containers for wiring, not for business logic.** Configuration should be in the container. Logic should be testable without it.

## See Also

- **patterns-factory** — When you need complex object creation logic separate from DI
- **architecture-patterns** — Hexagonal architecture and ports depend on DI  
- **architecture-clean-hexagonal** — Using DI to enforce dependency inversions at boundaries