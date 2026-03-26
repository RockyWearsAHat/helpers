# Swift Generics: Type Parameters, Associated Types, and Protocol-Oriented Programming

## Introduction

**Generics** enable writing code that works with any type while maintaining type safety. Swift's generics system combines parametric polymorphism (type parameters) with protocol-oriented design (associated types, where clauses), enabling powerful abstractions without sacrificing compile-time checking.

This distinguishes Swift from dynamic languages (no type safety) and some statically typed languages (rigid inheritance hierarchies). Generics are foundational to modern Swift: arrays, dictionaries, optionals, and asyncSequences all use them.

## Type Parameters

### Basic Generic Functions

A **type parameter** is a placeholder for a concrete type, specified at the call site:

```swift
func swap<T>(_ a: inout T, _ b: inout T) {
    let temp = a
    a = b
    b = temp
}

var x = 1, y = 2
swap(&x, &y)  // T is Int; type inferred from arguments

var name1 = "Alice", name2 = "Bob"
swap(&name1, &name2)  // T is String
```

`swap` works for any type `T` that can be assigned. The compiler generates a specialized version for each concrete type.

### Generic Types

```swift
struct Stack<T> {
    private var items: [T] = []
    
    mutating func push(_ item: T) {
        items.append(item)
    }
    
    mutating func pop() -> T? {
        items.isEmpty ? nil : items.removeLast()
    }
}

let intStack = Stack<Int>()  // Stack of Int
let stringStack = Stack<String>()  // Stack of String
```

### Multiple Type Parameters

```swift
func zip<A, B>(_ first: [A], _ second: [B]) -> [(A, B)] {
    zip(first, second).map { ($0, $1) }
}

let numbers = [1, 2, 3]
let names = ["one", "two", "three"]
let pairs = zip(numbers, names)  // [(1, "one"), (2, "two"), ...]
```

## Constraints on Type Parameters

### Protocol Conformance

Restrict a type parameter to types conforming to a protocol:

```swift
func printAllElements<T: CustomStringConvertible>(_ array: [T]) {
    for element in array {
        print(element)  // T.description available
    }
}

printAllElements([1, 2, "three"])  // ✓ Int, String are CustomStringConvertible
```

### Comparable and Equatable

```swift
func maximum<T: Comparable>(_ array: [T]) -> T? {
    array.max()  // Only Comparable types support max()
}

func contains<T: Equatable>(_ array: [T], _ target: T) -> Bool {
    array.contains { $0 == target }  // Only Equatable types support ==
}
```

### Multiple Constraints

```swift
func describe<T: Equatable & CustomStringConvertible>(_ item: T) -> String {
    "Item: \(item)"  // T must conform to both protocols
}
```

## Associated Types and Protocol Generics

### Associated Types

A **protocol** can declare abstract type members via `associatedtype`. Each conforming type provides a concrete type:

```swift
protocol Container {
    associatedtype Element
    
    func append(_ item: Element)
    func retrieve() -> Element?
}

struct Stack<T>: Container {
    typealias Element = T  // Specify Element type
    
    private var items: [T] = []
    
    func append(_ item: Element) { items.append(item) }
    func retrieve() -> Element? { items.popLast() }
}
```

The compiler infers `Element` from the generic parameter or explicit `typealias`.

### Generic Implementations with AssociatedTypes

```swift
protocol Sequence {
    associatedtype Element
    associatedtype Iterator: IteratorProtocol where Iterator.Element == Element
    
    func makeIterator() -> Iterator
}

struct CountingSequence: Sequence {
    typealias Element = Int
    typealias Iterator = CountingIterator
    
    func makeIterator() -> Iterator { CountingIterator() }
}

struct CountingIterator: IteratorProtocol {
    typealias Element = Int
    private var count = 0
    
    mutating func next() -> Int? {
        count += 1
        return count <= 10 ? count : nil
    }
}
```

## Where Clauses

### Constraining Associated Types

Use `where` clauses to refine generic constraints:

```swift
// Only conform if Element is Equatable
extension Container where Element: Equatable {
    func contains(_ item: Element) -> Bool {
        // Specialized for Equatable elements
        // Implementation details...
        false
    }
}

struct Stack<T: Equatable>: Container {
    // ...contains() available because Element is Equatable
}
```

### Complex Where Clauses

```swift
func combine<T, U: Container>(_ first: [T], _ second: U) -> [T] 
    where U.Element == T {
    // T and U.Element must be the same type
    first + Array(sequence: second)
}

let ints = [1, 2, 3]
let moreInts = Stack<Int>()
_ = combine(ints, moreInts)  // Type args inferred; element types match
```

## Protocol-Oriented Programming

### Protocol Composition

Protocols can inherit from other protocols:

```swift
protocol Drawable {
    func draw()
}

protocol Resizable {
    mutating func resize(by factor: Double)
}

protocol Shape: Drawable, Resizable {
    var area: Double { get }
}

struct Circle: Shape {
    var radius: Double
    
    mutating func resize(by factor: Double) {
        radius *= factor
    }
    
    func draw() { print("Drawing circle") }
    
    var area: Double { Double.pi * radius * radius }
}
```

Conforming types must satisfy all inherited protocols.

### Protocol Default Implementations

Provide default behavior in protocol extensions:

```swift
protocol Numeric {
    static func +(lhs: Self, rhs: Self) -> Self
    static func *(lhs: Self, rhs: Self) -> Self
}

extension Numeric {
    static func squared(_ value: Self) -> Self {
        value * value
    }
}

extension Int: Numeric {
    // Conforming types inherit squared()
}
```

This reduces boilerplate; conforming types only override when behavior differs.

## Opaque Types (Some)

### Using Opaque Return Types

`some` abstracts away the concrete type while maintaining type identity:

```swift
protocol View {
    associatedtype Body: View
    var body: Body { get }
}

func makeButton() -> some View {
    Text("Click me")  // Concrete type hidden; only View protocol visible
}

// Equivalent to:
// func makeButton() -> AnyView { ... }
// but preserves type identity for optimization
```

**Benefit**: Callers treat the result as a `View` without knowing the concrete type. The compiler retains the identity (not type-erased to a single `AnyView` wrapper).

### Constraints on Opaque Types

```swift
func filter<T>(_ array: [T], by predicate: (T) -> Bool) -> some Collection {
    array.filter(predicate)  // Returns concrete Array, but we expose Collection
}
```

`some Collection` says "some type conforming to Collection," allowing specialization without exposing the specific type.

## Existential Types (Any)

### Type Erasure

**Existential types** use `any` keyword to erase type identity, enabling homogeneous collections:

```swift
protocol Animal {
    func makeSound()
}

let animals: [any Animal] = [
    Dog(),
    Cat(),
    Bird()
]

for animal in animals {
    animal.makeSound()  // Dynamic dispatch on protocol method
}
```

Each element can be a different concrete type, as long as it conforms to `Animal`.

### Performance Implication

Existential types use dynamic dispatch (slower than static dispatch). Each method call incurs a lookup:

```swift
// Static dispatch (fast)
let dog = Dog()
dog.makeSound()  // Compiler knows concrete type at compile time

// Dynamic dispatch (slower)
let animal: any Animal = Dog()
animal.makeSound()  // Lookup method at runtime
```

### Existential vs Opaque

| Opaque (`some`) | Existential (`any`) |
|-----------------|-------------------|
| Concrete type known to compiler | Type erased at runtime |
| Static dispatch (fast) | Dynamic dispatch (slower) |
| Return type hides identity | Enables heterogeneous collections |
| "I know what type this is, but I'm not telling you" | "Could be any type conforming to protocol" |

## Generic Specialization

### Compiler Optimization

The Swift compiler **specializes** generic code for common types:

```swift
func processArray<T>(_ array: [T]) -> Int {
    array.count
}

processArray([1, 2, 3])  // Compiler generates specialized version for [Int]
processArray(["a", "b"])  // Compiler generates specialized version for [String]
```

Each specialization is a full copy of the generic code with the type parameter replaced. This enables optimizations like inlining and devirtualization but increases binary size (code bloat).

### Code Size Trade-off

Excessive specialization bloats binaries. Swift can trade specialization for dynamic dispatch:

```swift
// This may not be specialized if T has many instantiations
func expensiveOperation<T>(_ value: T) { }

// Force specialization (if you know it's justified)
@inlinable
func inlinedOperation<T>(_ value: T) { }
```

## Conditional Conformance

### Extending Generics with Conditions

A generic type can conform to a protocol only if its type parameter meets a condition:

```swift
extension Stack: Equatable where T: Equatable {
    static func ==(lhs: Stack<T>, rhs: Stack<T>) -> Bool {
        lhs.items == rhs.items
    }
}

let stack1 = Stack<Int>()
let stack2 = Stack<Int>()
let equal = stack1 == stack2  // ✓ Stack<Int> conforms to Equatable

let stringStack1 = Stack<String>()
let stringStack2 = Stack<String>()
let equal2 = stringStack1 == stringStack2  // ✓ Stack<String> conforms
```

Without the condition, `Stack<T>` wouldn't be `Equatable` in general (only if `T` is).

### Conformance Dependencies

```swift
extension Array: Hashable where Element: Hashable {
    func hash(into hasher: inout Hasher) {
        for element in self {
            hasher.combine(element)
        }
    }
}

let hashable: [Int] = [1, 2, 3]  // Hashable
let notHashable: [NSObject] = []  // Not Hashable (NSObject isn't Hashable)
```

## Type Erasure Patterns

### Using AnyEquatable

When you need a homogeneous collection of equatable types from different types:

```swift
struct AnyEquatable {
    private let _equals: (Any) -> Bool
    
    init<T: Equatable>(_ value: T) {
        self._equals = { other in
            (other as? T).map { value == $0 } ?? false
        }
    }
    
    func equals(_ other: Any) -> Bool {
        _equals(other)
    }
}

let mixed: [AnyEquatable] = [
    AnyEquatable(42),
    AnyEquatable("hello"),
    AnyEquatable(3.14)
]

print(mixed[0].equals(42))  // true
```

This pattern captures concrete type information in a closure, enabling type-polymorphic comparisons.

## Generics in SwiftUI

### Generic Modifiers

```swift
struct Container<Content: View>: View {
    let content: Content
    
    var body: some View {
        VStack {
            content
        }
        .padding()
    }
}

Container {
    Text("Hello")  // Content type is inferred as Text
}
```

SwiftUI heavily uses generics for flexible, type-safe views.

## See Also

Related concepts: [type-systems-theory.md](type-systems-theory.md), [paradigm-type-level-programming.md](paradigm-type-level-programming.md), [language-swift.md](language-swift.md), [language-swift-swiftui.md](language-swift-swiftui.md)