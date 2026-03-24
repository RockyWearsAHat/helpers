# C++ Best Practices (Modern C++20/23)

## Core Principles

Modern C++ is NOT the C++ of the 1990s. The C++ Core Guidelines (Stroustrup & Sutter) define the modern standard.

- **RAII (Resource Acquisition Is Initialization)**: Bind resource lifetime to object lifetime. No manual `new`/`delete`.
- **Type safety**: Use the type system to prevent bugs at compile time.
- **Zero-cost abstractions**: Abstractions compile away to efficient machine code.
- **Const correctness**: Mark everything `const` that shouldn't change.

## RAII & Smart Pointers

```cpp
// ❌ NEVER do this in modern C++
Widget* w = new Widget();
// ... if exception thrown here, memory leaks
delete w;

// ✅ Smart pointers — automatic lifetime management
auto w = std::make_unique<Widget>();     // Unique ownership (most common)
auto shared = std::make_shared<Widget>(); // Shared ownership (reference counted)
std::weak_ptr<Widget> weak = shared;      // Non-owning observer

// ✅ Stack allocation when possible (preferred over heap)
Widget w{};  // Destroyed automatically when scope exits

// ✅ Containers manage their own memory
std::vector<int> v = {1, 2, 3};  // No manual allocation
std::string s = "hello";          // No char* management
```

**Rules:**
- `unique_ptr` by default (sole ownership).
- `shared_ptr` only when ownership is genuinely shared.
- `weak_ptr` to break cycles or for non-owning observers.
- Raw pointers only for non-owning references (and prefer references `&` instead).
- **Never call `new`/`delete` directly.** Use `make_unique`/`make_shared`.

## Move Semantics

```cpp
// Move constructor — steal resources instead of copying
class Buffer {
    std::unique_ptr<char[]> data_;
    size_t size_;

public:
    // Move constructor
    Buffer(Buffer&& other) noexcept
        : data_(std::move(other.data_))
        , size_(std::exchange(other.size_, 0))
    {}

    // Move assignment
    Buffer& operator=(Buffer&& other) noexcept {
        data_ = std::move(other.data_);
        size_ = std::exchange(other.size_, 0);
        return *this;
    }
};

// std::move doesn't move — it casts to rvalue reference
auto v2 = std::move(v1);  // v1 is now in a valid but unspecified state
```

**Rules of thumb:**
- Mark move operations `noexcept` (enables optimizations in containers).
- After `std::move`, don't use the moved-from object (except to assign or destroy).
- Pass sink parameters by value and move:
```cpp
void setName(std::string name) {  // Takes by value
    name_ = std::move(name);       // Then moves into member
}
```

## Const Correctness

```cpp
// Mark everything const that shouldn't change
const int max_size = 100;
const std::string& getName() const;  // Method doesn't modify object

// constexpr — evaluated at compile time
constexpr int factorial(int n) {
    return n <= 1 ? 1 : n * factorial(n - 1);
}
constexpr int f5 = factorial(5);  // Computed at compile time

// consteval (C++20) — MUST be evaluated at compile time
consteval int square(int n) { return n * n; }

// const references to avoid copies
void process(const std::vector<int>& data);  // No copy, no modification
```

## C++20 Features

### Concepts (constrained templates)
```cpp
// Before: cryptic error messages from templates
// After: clear constraints

template<typename T>
concept Sortable = requires(T a) {
    { a < a } -> std::convertible_to<bool>;
    { a == a } -> std::convertible_to<bool>;
};

template<Sortable T>
void sort(std::vector<T>& v) {
    std::ranges::sort(v);
}

// Abbreviated function templates
void print(const auto& value) {
    std::cout << value << '\n';
}
```

### Ranges
```cpp
#include <ranges>

// Lazy, composable pipeline (like Rust iterators)
auto results = numbers
    | std::views::filter([](int n) { return n % 2 == 0; })
    | std::views::transform([](int n) { return n * n; })
    | std::views::take(10);

for (int n : results) {
    std::cout << n << '\n';
}
```

### Coroutines
```cpp
// Generator (co_yield)
std::generator<int> fibonacci() {
    int a = 0, b = 1;
    while (true) {
        co_yield a;
        auto next = a + b;
        a = b;
        b = next;
    }
}

// Async (co_await) — library-dependent
Task<Response> fetchData(std::string url) {
    auto response = co_await http::get(url);
    co_return response;
}
```

### Three-way comparison (spaceship operator)
```cpp
struct Point {
    int x, y;
    auto operator<=>(const Point&) const = default;  // Generates all 6 comparison operators
};
```

### std::format (C++20) / std::print (C++23)
```cpp
std::string msg = std::format("Hello, {}! You are {} years old.", name, age);
std::println("Result: {:.2f}", 3.14159);  // C++23
```

## C++23 Features

```cpp
// std::expected (like Rust's Result)
std::expected<int, std::string> parse(std::string_view sv) {
    int value;
    auto [ptr, ec] = std::from_chars(sv.data(), sv.data() + sv.size(), value);
    if (ec != std::errc{})
        return std::unexpected("parse failed");
    return value;
}

// std::optional monadic operations
std::optional<int> result = get_value()
    .transform([](int v) { return v * 2; })
    .or_else([] { return std::optional{42}; });

// Deducing this (explicit object parameter)
struct Widget {
    void process(this auto&& self) {
        // Works for both lvalue and rvalue
    }
};

// std::mdspan (multi-dimensional span)
std::mdspan<int, std::extents<int, 3, 3>> matrix(data.data());
```

## Error Handling in C++

```cpp
// Use exceptions for errors that can't be handled locally
// Use std::expected/std::optional for expected failures

// Exception best practices:
// 1. Throw by value, catch by const reference
try {
    process();
} catch (const std::runtime_error& e) {
    log_error(e.what());
}

// 2. Use noexcept for functions that don't throw
void swap(Widget& a, Widget& b) noexcept;

// 3. Custom exception hierarchy
class AppError : public std::runtime_error {
    using std::runtime_error::runtime_error;
};

class NotFoundError : public AppError {
    using AppError::AppError;
};
```

## Common Pitfalls

1. **Dangling references**: Returning reference to local variable. UB.
2. **Use-after-move**: Using object after `std::move`. Compile but wrong.
3. **Slicing**: Assigning derived to base by value — derived part is lost.
4. **UB from signed integer overflow**: Unlike unsigned, signed overflow is undefined.
5. **Forgetting virtual destructor**: Base class with virtual methods needs `virtual ~Base() = default;`.
6. **Implicit conversions**: Use `explicit` on single-argument constructors.

## Tooling

| Tool | Purpose |
|------|---------|
| **clang-tidy** | Static analysis + modernization suggestions |
| **clang-format** | Code formatting |
| **AddressSanitizer (ASan)** | Memory errors (use-after-free, buffer overflow) |
| **ThreadSanitizer (TSan)** | Data races |
| **UndefinedBehaviorSanitizer (UBSan)** | Undefined behavior detection |
| **Valgrind** | Memory leak detection |
| **CMake** | Build system (de facto standard) |
| **Conan** / **vcpkg** | Package management |

---

*Sources: C++ Core Guidelines (Stroustrup/Sutter), Effective Modern C++ (Scott Meyers), A Tour of C++ (Stroustrup), CppReference, C++ Weekly (Jason Turner)*
