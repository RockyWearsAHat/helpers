# Java Records and Sealed Classes — Immutable Data, Exhaustive Patterns, and Type Safety

## Overview

Records (Java 16+) eliminate boilerplate for immutable data carriers. Sealed classes (Java 17+) restrict inheritance, enabling exhaustive pattern matching. Together, they reduce verbosity and increase type safety in domain modeling.

## Records

### Declaration and Implicit Members

A record is an immutable data carrier. The compiler generates constructor, accessors, `equals`, `hashCode`, and `toString`.

```java
public record Point(int x, int y) {}

// Equivalent to:
// public final class Point {
//     private final int x;
//     private final int y;
//     public Point(int x, int y) { this.x = x; this.y = y; }
//     public int x() { return x; }  // accessor, not getX()
//     public int y() { return y; }
//     public boolean equals(Object o) { ... }
//     public int hashCode() { ... }
//     public String toString() { ... }
// }

Point p = new Point(10, 20);
System.out.println(p.x());  // 10, not p.getX()
System.out.println(p);      // Point[x=10, y=20]
```

Records are `final` and cannot extend classes (only implement interfaces). Components are `private final`.

### Compact Constructor

A compact constructor omits parameters and field assignments; the compiler generates them.

```java
public record Port(int value) {
    // Compact constructor: parameters and this.value assignment are implicit
    public Port {
        if (value < 0 || value > 65535) {
            throw new IllegalArgumentException("Port out of range: " + value);
        }
        // No explicit field assignment; compiler adds it
    }
}

new Port(8080);    // Valid
new Port(-1);      // Throws
```

### Custom Methods

Records can have methods; they behave like normal classes:

```java
public record Range(int start, int end) {
    public int length() { return end - start; }
    public boolean contains(int x) { return x >= start && x < end; }
}

Range r = new Range(0, 100);
System.out.println(r.contains(50));  // true
```

### Local Records (Java 16+)

Records can be declared inside methods:

```java
void processData(List<Map<String, Object>> data) {
    record Item(String name, Object value) {}
    
    for (var map : data) {
        for (var entry : map.entrySet()) {
            Item item = new Item(entry.getKey(), entry.getValue());
            // Use item
        }
    }
}
```

Local records are useful for intermediate data without polluting class namespace.

### Inheritance and Interfaces

Records cannot extend other records or classes, but can implement interfaces:

```java
public interface Drawable {
    void draw();
}

public record Circle(double radius) implements Drawable {
    @Override
    public void draw() { /* ... */ }
}
```

## Sealed Classes

Sealed classes restrict which classes can extend them, enabling exhaustive type narrowing.

### Declaration

```java
public sealed class Animal permits Dog, Cat, Bird {}

public final class Dog extends Animal {}
public final class Cat extends Animal {}
public final class Bird extends Animal {}

// Other classes cannot extend Animal
// public class Giraffe extends Animal {}  // Compile error
```

Permits list can reference records:

```java
public sealed interface Shape permits Circle, Rectangle, Triangle {}

public record Circle(double radius) implements Shape {}
public record Rectangle(double width, double height) implements Shape {}
public record Triangle(double a, double b, double c) implements Shape {}
```

Sealed classes can be extended by:
- `final` classes (end the hierarchy)
- `sealed` classes (continue sealing)
- `non-sealed` classes (reopen the hierarchy)

```java
public sealed class Animal permits Mammal, Bird {}

public sealed class Mammal extends Animal permits Dog, Cat {}  // sealed

public final class Dog extends Mammal {}  // final

public non-sealed class Cat extends Mammal {}  // others can extend

public class TabbyCat extends Cat {}  // allowed
```

## Pattern Matching

### Instanceof Patterns (Java 16+)

Type-test patterns combine instanceof and type cast:

```java
Object obj = "hello";

// Before: explicit cast
if (obj instanceof String) {
    String s = (String) obj;
    System.out.println(s.toUpperCase());
}

// After: pattern binding
if (obj instanceof String s) {
    System.out.println(s.toUpperCase());  // s is in scope
}

// With guard
if (obj instanceof String s && s.length() > 3) {
    System.out.println(s);  // Only prints if length > 3
}
```

### Record Patterns (Java 21+)

Destructure records directly in patterns:

```java
record Point(int x, int y) {}
record Circle(Point center, double radius) {}

Object obj = new Circle(new Point(10, 20), 5);

// Record pattern with destructuring
if (obj instanceof Circle(Point(var x, var y), var r)) {
    System.out.println("Circle at (" + x + ", " + y + ") radius " + r);
}

// Nested records
record Pair<T>(T first, T second) {}
record Triple<T>(T first, T second, T third) {}

Triple<Integer> t = new Triple<>(1, 2, 3);
if (t instanceof Triple(var a, var b, var c)) {
    System.out.println(a + b + c);  // 6
}
```

### Switch Expressions with Patterns (Java 21+)

Switch expressions use pattern matching:

```java
// Type and value patterns
String describe(Object obj) {
    return switch (obj) {
        case Integer i -> "Integer: " + i;
        case String s -> "String: " + s;
        case Point(int x, int y) -> "Point(" + x + ", " + y + ")";
        case null -> "Null";
        default -> "Other";
    };
}

// Guarded patterns
String categorize(Number n) {
    return switch (n) {
        case Integer i when i < 0 -> "Negative int";
        case Integer i when i == 0 -> "Zero";
        case Integer i -> "Positive int";
        case Long l when l > 1_000_000 -> "Large long";
        case Long l -> "Small long";
        case Double d -> "Double";
        default -> "Unknown";
    };
}

// Sealed type exhaustiveness
double area(Shape shape) {
    return switch (shape) {
        case Circle(double r) -> Math.PI * r * r;
        case Rectangle(double w, double h) -> w * h;
        case Triangle(double a, double b, double c) -> {
            double s = (a + b + c) / 2;  // Heron's formula
            yield Math.sqrt(s * (s - a) * (s - b) * (s - c));
        }
        // No default needed; compiler verifies exhaustiveness
    };
}
```

### Compiler Exhaustiveness Checking

With sealed classes and sealed interfaces, the compiler verifies all cases are handled without a `default`:

```java
public sealed interface Result permits Success, Failure {}
public record Success(String value) implements Result {}
public record Failure(Exception error) implements Result {}

String unwrap(Result r) {
    return switch (r) {
        case Success(var v) -> v;
        case Failure(var e) -> throw e;
        // No default; compiler is satisfied (all permits covered)
    };
}
```

If you add a new `Result` implementation, the compiler warns on existing switches.

## Sealed Hierarchies and ADTs

Sealed classes + records model algebraic data types effectively:

```java
public sealed interface Expr permits Lit, BinOp, Var {}

public record Lit(double value) implements Expr {}
public record Var(String name) implements Expr {}
public record BinOp(Expr left, String op, Expr right) implements Expr {}

// Type-safe evaluation
double eval(Expr expr, Map<String, Double> env) {
    return switch (expr) {
        case Lit(var v) -> v;
        case Var(var name) -> env.get(name);
        case BinOp(var l, var op, var r) -> {
            double lv = eval(l, env);
            double rv = eval(r, env);
            yield switch (op) {
                case "+" -> lv + rv;
                case "-" -> lv - rv;
                case "*" -> lv * rv;
                case "/" -> lv / rv;
                default -> throw new IllegalArgumentException("Unknown op: " + op);
            };
        }
    };
}
```

No null checks needed; the Type system guarantees exhaustiveness.

## Pattern Matching with instanceof and Null Handling (Java 21+)

New pattern types enhance type narrowing:

```java
// Null pattern
Object obj = null;
if (obj instanceof String) {
    // unreachable; obj is null
}

if (obj instanceof String s) {
    // unreachable; obj is null
}

// Explicit null handling
Object x = ...;
return switch (x) {
    case null -> "null";
    case String s -> s;
    case Integer i -> "int: " + i;
    default -> "other";
};
```

## Performance and Memory

Records are zero-overhead abstractions (final, all optimization-friendly). The JVM inlines accessor calls:

```java
Point p = new Point(10, 20);
int x = p.x();  // Inlined to direct field access
```

Pattern matching for instanceof and switch expressions is JIT-compiled to efficient bytecode with guard conditions evaluated inline.

Sealed classes enable:
- Aggressive inlining (compiler knows all subclasses)
- Devirtualization (fewer virtual method lookups)
- Tighter memory layout (fewer polymorphic surprises)

## Migration and Interop

### Backwards Compatibility

Records are not raw data containers; they're final classes. Code expecting inheritance will break:

```java
// Old code expecting a base class
public class LegacyData {
    protected int x;
    protected int y;
}

// Migrate to record (breaks subclassing)
public record NewData(int x, int y) {}

// If inheritable, keep class; records are data-only
```

### Records in Frameworks

Frameworks using reflection must handle records specially:

```java
Class<?> c = Point.class;
RecordComponent[] components = c.getRecordComponents();
// RecordComponent#getName(), getType() for introspection

// Serialization libraries (Gson, Jackson) support records natively (21+)
```

## See Also

- Pattern matching (JEP 405, 420, 427, 432)
- Records (JEP 384, 395)
- Sealed classes (JEP 397, 409)
- ADT patterns in functional languages
- Type narrowing and flow analysis