# Modern C++ — Structured Bindings, Concepts, Coroutines, and C++17-23 Features

## Overview

C++17 introduced structured bindings, std::optional, and fold expressions. C++20 brought concepts (constrained templates), ranges, and coroutines—the most significant additions since C++11. C++23 adds std::expected, monadic operations on optional, and incremental refinement. Modern C++ is less about raw features and more about expressing intent and avoiding footguns.

## Structured Bindings (C++17)

### Basic Syntax and Unpacking

```cpp
std::pair<int, std::string> p = {42, "hello"};
auto [value, name] = p;  // value = 42, name = "hello"

std::array<int, 3> arr{1, 2, 3};
auto [x, y, z] = arr;
```

Works with:
- Tuples (`std::tuple`)
- Pairs
- Arrays
- Any type with accessible members (or custom `get<>` and tuple traits)
- Class/struct member variables (C++17 extension)

### Caveat: Copy vs. Reference

```cpp
auto [x, y] = p;       // Copies p's members
auto& [x, y] = p;      // References p's members
auto&& [x, y] = p;     // Rvalue reference (perfect forwarding)
```

Bindings default to copy. Use explicit reference to avoid overhead.

## std::optional and std::variant

### std::optional: Single Value or Nothing

```cpp
std::optional<int> maybe_int = std::nullopt;
if (maybe_int) {
    std::cout << maybe_int.value();  // or *maybe_int
}
```

Replaces null pointers or sentinel values. semantics: either contains a value or is empty (not both). Monadic operations (C++23):

```cpp
auto result = maybe_int
    .transform([](int x) { return x * 2; })
    .or_else([]() { return std::optional(0); });
```

### std::variant: Discriminated Union

```cpp
std::variant<int, std::string, double> data;
data = 42;  // data.index() == 0

if (std::holds_alternative<int>(data)) {
    int val = std::get<int>(data);
}
```

Type-safe union. `std::visit` pattern-matches:

```cpp
auto printer = [](const auto& x) { std::cout << x; };
std::visit(printer, data);  // Works for any alt type
```

### std::any: Type Erasure

```cpp
std::any val = 42;
val = std::string("hello");
int x = std::any_cast<int>(val);  // Throws if wrong type
```

Stores any type, but requires runtime downcasting. Less safe than variant (wrong casts throw), but handles unknown types.

## Fold Expressions (C++17)

Binary fold operators over parameter packs:

```cpp
template<typename... Args>
auto sum(Args&&... args) {
    return (0 + ... + args);  // Sums all args
}

template<typename... Ts>
void print(const Ts&... ts) {
    (std::cout << ... << ts);  // Prints all args sequentially
}
```

Folds reduce parameter pack into a single expression. Left/right associativity matters for non-commutative operations (subtraction, division).

## Concepts and Constraints (C++20)

### Defining Constraints

```cpp
template<typename T>
concept Arithmetic = std::is_arithmetic_v<T>;

template<Arithmetic T>
T add(T a, T b) { return a + b; }
```

Concepts enforce **type requirements** at compile time. Without concepts:

```cpp
template<typename T>
T add(T a, T b) { return a + b; }
// Error: line 999: no operator+ for class Foo
```

With concepts, the compiler rejects invalid types upfront with clear error messages.

### Requires Clauses

```cpp
template<typename T>
concept Comparable = requires(T a, T b) {
    a < b;
    a == b;
};

template<typename T>
    requires Comparable<T>
T max(T a, T b) { return a < b ? b : a; }
```

`requires` clause validates syntactic and semantic requirements. Example: "T must support `<` and `==`."

### Concept Libraries

Standard library provides `std::range`, `std::regular`, `std::total_order`, `std::Invocable`. These compose into more specific constraints:

```cpp
template<std::regular T>
void process(T x);  // T must be copyable, comparable, etc.
```

## Ranges (C++20)

### Range-Based Algorithms

```cpp
std::vector v{1, 2, 3, 4, 5};
auto result = v | std::views::filter([](int x) { return x > 2; })
              | std::views::transform([](int x) { return x * 2; });
```

Chained algorithms with pipes. Each filter creates a lazy view; no intermediate containers. More readable than nested std:: calls.

### Views and Lazy Evaluation

Views are non-owning, composable ranges. `std::views::filter` doesn't copy; it wraps the original and applies the predicate on iteration.

```cpp
auto odds = v | std::views::filter([](int x) { return x % 2; });
// odds is a lazy filter; no storage, no computation until iteration
```

Efficiency: avoids temporary containers and multiple passes.

## Coroutines (C++20)

### Generator Pattern

```cpp
std::generator<int> fibonacci(int n) {
    int a = 0, b = 1;
    while (a <= n) {
        co_yield a;  // Suspend and return a
        std::tie(a, b) = std::make_pair(b, a + b);
    }
}

for (int fib : fibonacci(100)) {
    std::cout << fib << "\n";
}
```

`co_yield` suspends the coroutine and returns control to the caller. When the loop resumes (next iteration), the coroutine continues from where it left off.

### Customization Points: promise_type

Each coroutine type defines a `promise_type`. The compiler generates boilerplate that calls `co_yield`, `co_return`, and exception handling through the promise.

```cpp
template<typename T>
class Generator {
public:
    struct promise_type {
        T current_value;
        Generator get_return_object() { return Generator(this); }
        std::suspend_never initial_suspend() { return {}; }
        std::suspend_always final_suspend() noexcept { return {}; }
        void unhandled_exception() { throw; }
        std::suspend_always yield_value(T x) {
            current_value = x;
            return {};
        }
    };
};
```

Complex but powerful: coroutines can be generators, async tasks, or stateful algorithms.

## Modules (C++20)

### Module Declaration and Export

```cpp
export module MyModule;

export namespace math {
    int add(int a, int b);
}

// implementation
int math::add(int a, int b) { return a + b; }
```

```cpp
import MyModule;

int x = math::add(1, 2);
```

No header guards, no include search path confusion. Modules decompose dependencies and reduce compile times (source parsing only once).

### Adoption Challenges

Compiler support (GCC, Clang, MSVC) was incomplete until 2024. Projects using modules require C++20 support and compatible build systems. Legacy projects still use headers.

## std::format (C++20) and String Formatting

```cpp
#include <format>

std::string msg = std::format("Hello, {}! You have {} points.", name, score);
```

Type-safe string formatting without printf's format-string mismatch errors. Extension of Python's format syntax to C++.

## std::expected (C++23)

### Result Type With Error Information

```cpp
std::expected<int, std::string> divide(int a, int b) {
    if (b == 0) return std::unexpected("Division by zero");
    return a / b;
}

auto result = divide(10, 2);
if (result) {
    std::cout << result.value();
} else {
    std::cout << result.error();
}
```

Carries success value OR error information, similar to Rust's `Result<T, E>`. More ergonomic than exceptions for control flow; monadic operations:

```cpp
auto res = divide(10, 2)
    .transform([](int x) { return x * 2; })
    .or_else([](const std::string& e) { return divide(5, 1); });
```

## constexpr Everything (C++20+)

Moving computation to compile time:

```cpp
constexpr int factorial(int n) {
    return n <= 1 ? 1 : n * factorial(n - 1);
}

int arr[factorial(5)];  // Array size is 120, determined at compile time
```

Constraints: constexpr functions can only use operations valid at compile time (no dynamic allocation traditionally; C++20 relaxes this).

### consteval and constinit

```cpp
consteval int compile_only() { return 42; }  // Must be compile-time

constinit int global = compile_only();  // Guarantees compile-time init
```

`consteval` forces compile-time evaluation. `constinit` guarantees a variable is initialized at compile time (replaces some uses of `constexpr` for guarantees).

## Move Semantics: Deep Dive

### Lvalue vs. Rvalue References

```cpp
void process(const MyClass& x);   // Lvalue reference (object persists)
void process(MyClass&& x);        // Rvalue reference (temporary)

MyClass obj;
process(obj);              // Calls lvalue version
process(MyClass());        // Calls rvalue version
process(std::move(obj));   // Forces rvalue call
```

Rvalue references enable:
- **Move constructors**: Steal resources from temporaries instead of copying
- **Perfect forwarding**: Generic code preserving lvalue/rvalue distinction

### Move Constructor Implementation

```cpp
class Vector {
    int *data;
    size_t size;
public:
    Vector(Vector&& other) noexcept
        : data(other.data), size(other.size) {
        other.data = nullptr;
        other.size = 0;
    }
};
```

After move, the source object is left in a valid but unspecified state (often empty). This is fast: pointer transfer, not deep copy.

### Rule of Five

Define all or none: destructor, copy constructor, move constructor, copy assignment, move assignment. Omitting move operations forces copy fallback.

## RAII: Resource Acquisition Is Initialization

The foundational pattern:

```cpp
{
    std::lock_guard<std::mutex> lock(mutex);
    // Critical section
}  // lock is released here, even if exception thrown
```

Scoped lifetime of objects ensures cleanup. Uses:
- Lock guards (automatic unlock)
- Unique pointers (automatic delete)
- File handles (automatic close)
- Memory allocation (automatic free)

## Performance and Optimization

### Zero-Cost Abstractions

C++ abstractions compile away: inline functions, template specialization, and compile-time computation incur no runtime overhead.

### Exception Safety Guarantees

- **Strong**: Either succeeds or leaves state unchanged (expensive, typically via copy-and-swap)
- **Weak**: May leave object in changed but valid state (typical for allocations)
- **No-throw**: Never throws (required for destructors)

Choose guarantee level based on use case.

## See Also

- [Memory Management and Allocation Strategies](memory-management.md)
- [Design Patterns (Gang of Four and Modern Additions)](design-patterns.md)
- [Concurrency Patterns and Lock-Free Programming](algorithms-concurrency.md)
- [Modern C — C11 to C23](language-c-modern.md)