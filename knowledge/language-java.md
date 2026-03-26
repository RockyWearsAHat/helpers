# Java Conventions and Idioms (Modern Java 21+)

## Modern Java: Not Your Grandfather's Java

Java 21 (LTS) introduced transformative features. Write modern Java, not Java 8 patterns.

## Records (Java 16+)

Immutable data carriers — replace verbose POJOs.

```java
// Before: 50+ lines of class, getters, equals, hashCode, toString
// After:
public record User(String name, String email, int age) {}

// Automatic: constructor, getters (name(), email(), age()), equals, hashCode, toString
var user = new User("Alice", "alice@example.com", 30);
System.out.println(user.name());  // "Alice"

// Custom validation in compact constructor
public record Port(int value) {
    public Port {
        if (value < 0 || value > 65535) {
            throw new IllegalArgumentException("Invalid port: " + value);
        }
    }
}

// Records with custom methods
public record Range(int start, int end) {
    public int length() { return end - start; }
    public boolean contains(int value) { return value >= start && value < end; }
}
```

## Sealed Classes (Java 17+)

Restrict which classes can extend a type — enables exhaustive pattern matching.

```java
public sealed interface Shape permits Circle, Rectangle, Triangle {}

public record Circle(double radius) implements Shape {}
public record Rectangle(double width, double height) implements Shape {}
public record Triangle(double base, double height) implements Shape {}

// Compiler guarantees all subtypes are handled
public double area(Shape shape) {
    return switch (shape) {
        case Circle c    -> Math.PI * c.radius() * c.radius();
        case Rectangle r -> r.width() * r.height();
        case Triangle t  -> 0.5 * t.base() * t.height();
        // No default needed — compiler knows all cases are covered
    };
}
```

## Pattern Matching (Java 21+)

```java
// instanceof pattern matching (Java 16+)
if (obj instanceof String s && s.length() > 5) {
    System.out.println(s.toUpperCase());
}

// Switch pattern matching (Java 21)
String describe(Object obj) {
    return switch (obj) {
        case Integer i when i > 0 -> "positive integer: " + i;
        case Integer i            -> "non-positive integer: " + i;
        case String s             -> "string of length " + s.length();
        case null                 -> "null";
        default                   -> "unknown: " + obj.getClass();
    };
}

// Record patterns (Java 21) — destructure records in patterns
static double calculateArea(Shape shape) {
    return switch (shape) {
        case Circle(var r)          -> Math.PI * r * r;
        case Rectangle(var w, var h) -> w * h;
        case Triangle(var b, var h)  -> 0.5 * b * h;
    };
}
```

## Virtual Threads (Java 21) — Project Loom

Lightweight threads managed by the JVM. One million concurrent threads is trivial.

```java
// Before: thread pools, reactive programming complexity
// After: simple blocking code that scales

// Create virtual threads
try (var executor = Executors.newVirtualThreadPerTaskExecutor()) {
    List<Future<String>> futures = urls.stream()
        .map(url -> executor.submit(() -> fetch(url)))
        .toList();

    List<String> results = futures.stream()
        .map(f -> {
            try { return f.get(); }
            catch (Exception e) { throw new RuntimeException(e); }
        })
        .toList();
}

// Structured concurrency (preview)
try (var scope = new StructuredTaskScope.ShutdownOnFailure()) {
    var user = scope.fork(() -> findUser(id));
    var order = scope.fork(() -> findOrder(id));

    scope.join().throwIfFailed();

    return new UserOrder(user.get(), order.get());
}
```

**Key point:** Virtual threads make blocking I/O cheap. You don't need reactive frameworks (WebFlux, RxJava) for most I/O-bound workloads anymore.

## Streams API

```java
// Pipeline processing
List<String> activeUserEmails = users.stream()
    .filter(User::isActive)
    .map(User::email)
    .sorted()
    .toList();  // Java 16+ (replaces .collect(Collectors.toList()))

// Collectors
Map<String, List<User>> byRole = users.stream()
    .collect(Collectors.groupingBy(User::role));

Map<String, Long> roleCounts = users.stream()
    .collect(Collectors.groupingBy(User::role, Collectors.counting()));

String csv = names.stream()
    .collect(Collectors.joining(", "));

// Parallel streams (only for CPU-bound, large datasets)
long count = hugeList.parallelStream()
    .filter(this::expensive)
    .count();
```

## Optional

```java
// Represent possibly-absent values — replaces null returns
Optional<User> findUser(String id) {
    return Optional.ofNullable(userMap.get(id));
}

// Chaining operations (don't call .get() without checking)
String displayName = findUser(id)
    .map(User::name)
    .orElse("Unknown");

// orElseThrow for required values
User user = findUser(id)
    .orElseThrow(() -> new NotFoundException("User " + id));

// ❌ Don't use Optional as a field type or method parameter
// ❌ Don't use Optional.of(null) — it throws NPE
// ✅ Use Optional only for return types
```

## Text Blocks & String Templates

```java
// Text blocks (Java 15+)
String json = """
    {
        "name": "%s",
        "age": %d
    }
    """.formatted(name, age);

// String templates (preview in Java 21+)
String msg = STR."Hello, \{name}! You are \{age} years old.";
```

## Exception Handling

```java
// Use specific exceptions, not generic ones
// ❌ throw new Exception("something failed");
// ✅ throw new OrderNotFoundException(orderId);

// Custom exception hierarchy
public class AppException extends RuntimeException {
    private final ErrorCode code;

    public AppException(ErrorCode code, String message) {
        super(message);
        this.code = code;
    }

    public AppException(ErrorCode code, String message, Throwable cause) {
        super(message, cause);
        this.code = code;
    }
}

// Try-with-resources (always for AutoCloseable)
try (var conn = dataSource.getConnection();
     var stmt = conn.prepareStatement(sql)) {
    // Resources auto-closed even if exception thrown
}

// Prefer unchecked exceptions for application errors
// Checked exceptions are for recoverable conditions only
```

## Dependency Injection

```java
// Constructor injection (preferred — immutable, testable)
public class UserService {
    private final UserRepository repository;
    private final EmailService emailService;

    public UserService(UserRepository repository, EmailService emailService) {
        this.repository = repository;
        this.emailService = emailService;
    }
}

// With Spring
@Service
public class UserService {
    private final UserRepository repository;

    public UserService(UserRepository repository) {  // Auto-injected
        this.repository = repository;
    }
}
```

## Project Structure (Maven/Gradle Standard)

```
src/
├── main/
│   ├── java/
│   │   └── com/example/myapp/
│   │       ├── Application.java
│   │       ├── controller/
│   │       ├── service/
│   │       ├── repository/
│   │       └── model/
│   └── resources/
│       └── application.yml
└── test/
    └── java/
        └── com/example/myapp/
            ├── service/
            │   └── UserServiceTest.java
            └── integration/
```

## Tooling

| Tool | Purpose |
|------|---------|
| **Gradle** / **Maven** | Build system |
| **JUnit 5** | Testing framework |
| **Mockito** | Mocking for tests |
| **SpotBugs** | Static analysis (bugs) |
| **Error Prone** | Compile-time bug detection (Google) |
| **Checkstyle** | Style enforcement |
| **JaCoCo** | Code coverage |
| **jlink** | Custom runtime images (smaller deployments) |

---

*Sources: Effective Java (Joshua Bloch), Java Language Specification, JEP proposals (Loom, Amber, Valhalla), Spring documentation, Modern Java in Action*
