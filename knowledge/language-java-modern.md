# Modern Java (17-21+): Records, Sealed Classes, Pattern Matching, Virtual Threads, and Beyond

## Introduction

Java 17 (LTS) and 21 (LTS) introduced transformative features that modernize the language. Records eliminate boilerplate. Sealed classes enable exhaustive pattern matching. Virtual threads from Project Loom scale concurrency. Structured concurrency APIs ensure resource cleanup. Understanding these features is essential to writing modern Java.

## Records (Java 16+)

Records are immutable data carriers with automatic boilerplate:

```java
// Before: ~50 lines (constructor, getters, equals, hashCode, toString, Serializable)
public class Point {
    private final int x;
    private final int y;
    
    public Point(int x, int y) { this.x = x; this.y = y; }
    public int getX() { return x; }
    public int getY() { return y; }
    public boolean equals(Object o) { /* ... */ }
    public int hashCode() { /* ... */ }
}

// After: 1 line
public record Point(int x, int y) {}
```

The compiler automatically generates:
- A `public` constructor taking all fields
- **Accessor methods named after fields** (not getters): `point.x()` and `point.y()` (get rid of the `get` prefix)
- `equals()`, `hashCode()`, `toString()` based on field values
- Serialization support

### Compact Constructor for Validation

```java
public record Port(int value) {
    public Port {  // Compact constructor (no parameters; acts on fields directly)
        if (value < 0 || value > 65535) {
            throw new IllegalArgumentException("Invalid port: " + value);
        }
    }
}

var port = new Port(8080);  // Validated automatically
```

### Records as API Containers

Records are ideal for DTOs, API responses, and configuration:

```java
public record ApiResponse<T>(int status, T data, String error) {}

var response = new ApiResponse<>(200, userData, null);
if (response.error() != null) {
    System.out.println("Error: " + response.error());
}
```

## Sealed Classes (Java 17+)

Sealed classes restrict which classes can extend them, enabling **exhaustive pattern matching**:

```java
public sealed interface Shape permits Circle, Rectangle, Triangle {}

public record Circle(double radius) implements Shape {}
public record Rectangle(double width, double height) implements Shape {}
public record Triangle(double base, double height) implements Shape {}

public double area(Shape shape) {
    return switch (shape) {
        case Circle c    -> Math.PI * c.radius() * c.radius();
        case Rectangle r -> r.width() * r.height();
        case Triangle t  -> 0.5 * t.base() * t.height();
        // No `default` needed — compiler guarantees exhaustiveness
    };
}
```

Sealed classes prevent accidental subtypes and enable compiler verification. Use them for:
- Domain type hierarchies (ensuring all variants are known)
- Parser/AST nodes (compiler checks you handle all cases)
- Error types (exhaustive error handling)

## Pattern Matching (Java 21+)

### Instanceof Patterns (Java 16+)

```java
// Old way
if (obj instanceof String) {
    String s = (String) obj;
    System.out.println(s.length());
}

// New way
if (obj instanceof String s && s.length() > 5) {
    System.out.println(s);
}
```

Pattern variables eliminate downcast verbosity and scope them to the condition.

### Switch Patterns (Java 17+)

```java
Object obj = ...;
String result = switch (obj) {
    case Integer i    -> String.format("Integer: %d", i);
    case String s     -> String.format("String: %s", s);
    case Double d     -> String.format("Number: %.2f", d);
    case null         -> "Null value";
    default           -> "Unknown type";
};
```

### Guarded Patterns (Java 21+)

Patterns with conditions:

```java
return switch (shape) {
    case Circle c when c.radius() < 1.0 -> "tiny circle";
    case Circle c when c.radius() >= 1.0 -> "normal circle";
    case Rectangle r when r.width() == r.height() -> "square";
    case Rectangle r -> "rectangle";
    default -> "other";
};
```

Pattern matching is transitional; expect extensions in future Java versions.

## Virtual Threads (Project Loom, Java 19+, stable in 21)

Virtual threads are lightweight threads managed by the JVM (green threads / M:N scheduling):

```java
// Create a virtual thread
Thread thread = Thread.ofVirtual()
    .name("virtual-worker")
    .start(() -> {
        System.out.println("Running virtually");
    });

thread.join();
```

Compare to OS threads (~MB stack, kernel-managed) — virtual threads use ~KB and have JVM scheduling.

### Scaling to Millions

```java
try (var executor = Executors.newVirtualThreadPerTaskExecutor()) {
    for (int i = 0; i < 1_000_000; i++) {
        executor.submit(() -> blockingIO());
    }
}  // Wait for all to complete
```

Virtual threads make it practical to use a thread per task instead of thread pools and async callbacks. Blocking I/O now scales.

### Important Caveat: Pinned Threads

Native code, System.out, or synchronized blocks can **pin** a virtual thread to its carrier OS thread, negating the benefit:

```java
synchronized (lock) {
    // If this blocks, the OS thread is pinned
    // Other virtual threads on that OS thread starve
    heavyComputation();
}
```

Use `ReentrantLock` or `StampedLock` instead; they don't pin.

## Structured Concurrency (Java 19+, preview API)

Nursery-like API ensuring all spawned tasks complete:

```java
import jdk.incubator.concurrent.StructuredTaskScope;

try (var scope = new StructuredTaskScope.ShutdownOnFailure()) {
    var future1 = scope.fork(task1());
    var future2 = scope.fork(task2());
    
    scope.join();  // Wait for all; rethrow any exception
    
    return new Result(future1.resultNow(), future2.resultNow());
}
```

Guarantees:
- All forked tasks complete before exiting scope
- If one fails, others are cancelled
- Cleaner than manual thread management or CompletableFuture chains

## Scoped Values (Java 21, preview)

Thread-local-like values without the overhead of ThreadLocal or context variables:

```java
static final ScopedValue<String> USER = ScopedValue.forType(String.class);

void authenticate(String userId) {
    ScopedValue.where(USER, userId).run(() -> {
        // Inside this scope, USER.get() returns userId
        System.out.println("Current user: " + USER.get());
        
        // Value inherited by spawned virtual threads
        Thread.ofVirtual().start(() -> {
            System.out.println("In child: " + USER.get());  // Still visible
        });
    });
}
```

More efficient and safer than ThreadLocal, especially for virtual threads.

## Panama FFI (Foreign Function Interface, Java 19+, preview as of 21)

Call native C code and use native memory without JNI boilerplate:

```java
import java.lang.foreign.*;

// Simple mapping
Arena arena = Arena.ofConfined();
MemorySegment segment = arena.allocateArray(ValueLayout.JAVA_INT, 10);

// Access like an array
for (int i = 0; i < 10; i++) {
    segment.setAtIndex(ValueLayout.JAVA_INT, i, i * i);
}

// Memory safety guaranteed; no segfaults in safe code
```

Replaces JNI for interop with native libraries (C, system APIs). Still evolving.

## Vector API (Java 16+, incubating)

SIMD operations for bulk data processing:

```java
import jdk.incubator.vector.*;

int[] a = new int[256];
int[] b = new int[256];
IntVector va = IntVector.fromArray(IntVector.SPECIES_256, a, 0);
IntVector vb = IntVector.fromArray(IntVector.SPECIES_256, b, 0);
IntVector vc = va.add(vb);
vc.intoArray(new int[256], 0);
```

Exploits CPU SIMD instructions for 10-50x speedup on suitable workloads (matrix ops, image processing).

## Value Types (Preview, Java 19+)

Immutable aggregate types (like records but with value semantics — no identity):

```java
value class Point {
    int x;
    int y;
}

// Value class instances are inlined; no indirection
```

**Status:** Still in preview; expected to stabilize in Java 25+. When ready, arrays of value classes will have dense layouts (no pointers) and superior cache usage.

## Pattern Matching: Record Patterns (Java 21+)

Destructure nested records in patterns:

```java
record Address(String street, String city) {}
record Person(String name, Address addr) {}

String getCity(Person p) {
    return switch (p) {
        case Person(_, Address(_, String city)) -> city;
    };
}
```

This replaces deep getter chains with declarative unpacking.

## GraalVM and Beyond

Java's trajectory includes:
- **GraalVM Native Image:** Compile to native binary (instant startup, AOT optimization)
- **Project Amber:** More pattern matching improvements, named tuples
- **Project Panama:** Full interop with C libraries
- **Project Leyden:** Faster startup without sacrificing runtime performance

## Common Patterns

### Sealed + Records + Pattern Matching

The holy trinity for type-safe error handling:

```java
sealed interface Result<T> permits Success, Failure {
    record Success<T>(T value) implements Result<T> {}
    record Failure<T>(String error) implements Result<T> {}
}

String extractValue(Result<?> result) {
    return switch (result) {
        case Result.Success<?> s -> s.value().toString();
        case Result.Failure<?> f -> "Error: " + f.error();
    };
}
```

### Virtual Threads + Structured Concurrency

Replaces thread pools and async:

```java
try (var scope = new StructuredTaskScope.ShutdownOnFailure()) {
    var results = urls.stream()
        .map(url -> scope.fork(() -> fetchUrl(url)))
        .toList();
    scope.join();
    return results.stream().map(Future::resultNow).toList();
}
```

## See Also

- **language-java** — Java conventions and fundamentals
- **runtime-jvm** — JVM internals, GC, JIT compilation
- **concurrency-patterns** — structured concurrency concepts