# Swift Memory Model: ARC, Reference Cycles, Value Types, and Copy-on-Write

## Introduction

Swift's memory is managed via **Automatic Reference Counting (ARC)** — the runtime tracks how many references point to each object, deallocating when the count reaches zero. This avoids garbage collection pauses and manual allocation/deallocation, but introduces subtle patterns and edge cases.

Understanding Swift's memory model is essential for writing performant code, avoiding retain cycles, and reasoning about object lifetimes.

## Automatic Reference Counting (ARC)

### Reference Counting Fundamentals

When a class instance is created, ARC allocates memory. Each strong reference increments a retain count; releasing a reference decrements it. When the count hits zero, the object is deallocated:

```swift
class Person {
    let name: String
    init(name: String) { self.name = name }
    deinit { print("\(name) deallocated") }
}

var alice: Person? = Person(name: "Alice")  // Retain count: 1
var bob = alice                              // Retain count: 2
alice = nil                                  // Retain count: 1
bob = nil                                    // Retain count: 0 → dealloc
```

ARC is **deterministic** — objects deallocate as soon as the last reference is released. This differs from garbage collection, where timing is unpredictable.

### Memory Overhead

Every instance has an **object header** storing:
- Retain count (includes isa pointer on some platforms)
- Deinit function pointer
- Flags (e.g., has weak references)

This overhead is unavoidable for class instances; one reason Swift favors structs.

## Strong, Weak, and Unowned References

### Strong References (Default)

```swift
var person = Person(name: "Charlie")  // Strong — increments retain count
```

Ordinary property and variable declarations create strong references. The referenced object lives as long as there's one strong reference.

### Weak References

A **weak** reference doesn't increment the retain count. When the object is deallocated, the weak reference automatically becomes `nil`:

```swift
class Person {
    var friend: Person?
    weak var mother: Person?
}

var alice: Person? = Person(name: "Alice")
var bob: Person? = Person(name: "Bob")

// Weak reference: doesn't prevent deallocation
alice?.mother = bob
bob = nil  // Bob is deallocated immediately; alice.mother is now nil
```

**Use case**: Breaking retain cycles where child holds weak reference to parent.

### Unowned References

An **unowned** reference assumes the object is still alive when accessed. It doesn't increment retain count and never becomes `nil`:

```swift
class Person {
    let name: String
    unowned var parent: Person?  // Must always point to a living object
    
    init(name: String, parent: Person?) {
        self.name = name
        self.parent = parent  // Unowned — parent must outlive self
    }
}
```

**Safety**: Accessing a deallocated unowned reference causes a runtime crash. Use only when ownership is clear (child can never outlive parent).

### Weak vs Unowned

| Weak | Unowned |
|------|---------|
| Becomes `nil` when target deallocates | Crashes if target is deallocated |
| `Optional` type | Non-optional type |
| Ownership unclear; defensive | Ownership is guaranteed |
| More common in practice | Use sparingly with clear lifetime rules |

## Retain Cycles

### Classic Pattern: Two-Way Reference

```swift
class Person {
    let name: String
    var bestFriend: Person?
    init(name: String) { self.name = name }
    deinit { print("\(name) deallocated") }
}

var alice: Person? = Person(name: "Alice")
var bob: Person? = Person(name: "Bob")

alice?.bestFriend = bob
bob?.bestFriend = alice

alice = nil  // Alice retain count: 1 (held by bob.bestFriend)
bob = nil    // Bob retain count: 1 (held by alice.bestFriend)
// Neither deallocates — memory leak!
```

Both objects hold strong references to each other. When external references are released, neither object is deallocated because they keep each other alive.

### Solution: Weak Capture

```swift
alice?.bestFriend = bob
bob?.bestFriend = alice  // Change to weak

alice = nil
bob = nil  // Both deallocate now
```

Weak breaks the cycle — one direction becomes optional and nullable.

### Parent-Child Cycles

```swift
class Parent {
    var children: [Child] = []
}

class Child {
    weak var parent: Parent?  // Weak back-reference
    
    init(parent: Parent) {
        self.parent = parent
        parent.children.append(self)  // Strong reference down
    }
}
```

Parent holds strong references to children; children hold weak references to the parent. When the parent is deallocated, the children follow.

## Closure Capture and Memory Cycles

### The Capture Problem

Closures capture variables from their enclosing scope. If a closure captures `self` and is stored as a property, a cycle forms:

```swift
class ViewController {
    var name = "ViewController"
    
    lazy var callback: () -> Void = {
        print(self.name)  // Captures self
    }
    
    deinit { print("Deallocating") }
}

var vc: ViewController? = ViewController()
vc = nil  // Not deallocated — callback holds self, property holds callback
```

### Capture Lists

Break cycles with explicit capture lists:

```swift
lazy var callback: () -> Void = { [weak self] in
    guard let self else { return }
    print(self.name)
}
```

**Common patterns**:

- `[weak self]`: Object may be deallocated; `self` becomes optional
- `[unowned self]`: Assume `self` lives as long as the closure exists
- `[unowned self, weak other]`: Mix as needed
- `[weak self = someObject]`: Rename captured reference

**Best practice**: Use `[weak self]` by default; switch to `[unowned self]` only when you're certain of the lifetime.

### Non-Escaping Closures

If a closure doesn't escape its scope, no cycle can form:

```swift
func withUserDefaults<T>(_ key: String, default defaultValue: T, _ block: (inout T) -> Void) -> T {
    var value = UserDefaults.standard.object(forKey: key) as? T ?? defaultValue
    block(&value)  // Closure exits before this function returns
    return value
}

withUserDefaults("count", default: 0) { count in
    count += 1  // No cycle — closure doesn't capture self
}
```

Non-escaping closures (default unless marked `@escaping`) are implicitly safe from cycles.

## Value Types vs Reference Types and Memory Implications

### Structs: Stack + Inline

Structs are typically allocated on the stack (or inlined into their container). No reference counting overhead:

```swift
struct Point {
    var x: Double
    var y: Double
}

var p = Point(x: 1, y: 2)  // Allocated on stack
var q = p                   // Copy — two independent instances
```

Assignment copies the entire value. For small structs, this is cheap. For large structs with many fields, copying costs proportionally more.

### Classes: Heap + Reference Semantics

Classes are heap-allocated and referenced. Every variable holds a pointer:

```swift
class Person {
    var name: String
    var age: Int
}

var alice = Person()  // Allocated on heap; alice holds a pointer
var bob = alice       // Both point to the same heap object
bob.name = "Bob"      // Modifies shared object; alice sees the change
```

### Copy-on-Write

Swift optimizes common struct patterns with **copy-on-write (CoW)**. An array shares internal storage until one copy is mutated:

```swift
var a = [1, 2, 3]
var b = a            // Shares buffer with a
a.append(4)          // Copy triggered; a gets its own buffer
                     // b's buffer unmodified
```

This gives the safety of value semantics with performance approaching reference semantics for large collections.

## Autoreleasepool

Swift minimizes autorelease usage (primarily an Objective-C concept), but it appears in Cocoa interop:

```swift
for image in largeImageArray {
    autoreleasepool {
        let thumbnail = createThumbnail(for: image)
        saveThumbnail(thumbnail)  // Autoreleased objects freed here
    }
}
```

In tight loops processing many objects, `autoreleasepool` forces deallocation between iterations, preventing memory bloat. Most Swift code doesn't need this.

## Memory Layout and Performance

### Instance Size

Use `MemoryLayout` to inspect memory layout:

```swift
struct Point { var x: Double; var y: Double }
print(MemoryLayout<Point>.size)        // 16 bytes (2 × 8)
print(MemoryLayout<Point>.alignment)   // 8 bytes
print(MemoryLayout<Point>.stride)      // 16 bytes (size with padding)
```

For classes, the instance stores only the object header on the stack; the full object lives on the heap.

### Enum Memory

Enums with associated values use **tagged unions**:

```swift
enum Result {
    case success(String)      // Tag 0; payload: String (16+ bytes)
    case failure(NSError)     // Tag 1; payload: NSError reference
}

print(MemoryLayout<Result>.size)  // 25 bytes (1 tag byte + 24 max payload)
```

The size is the size of the largest case plus space for the tag.

## Debugging and Profiling

### Detecting Retain Cycles

**Xcode Memory Debugger**:
1. Set a breakpoint
2. Debug → Memory Graph Debugger
3. Look for purple cycle icons

**Manual inspection**:

```swift
print("Retain count:", CFGetRetainCount(object))  // Unofficial API
```

### Leak Detection

Xcode's **Leak Instrument** (Xcode → Open Developer Tool → Instruments → Leaks) detects unreleased allocations:
- Red leak: Definitive memory leak
- Yellow potential: May or may not be a leak; inspect further

## Optimization Techniques

### Reducing Reference Counting Overhead

```swift
// Inefficient: Array of references
var cache: [NSObject] = large_array  // Each element counted separately

// More efficient: Inline where possible
let points: [Point] = ...  // No reference counting for value types

// Unownerd in contexts where lifetime is guaranteed:
unowned var delegate: MyDelegate?
```

### Struct vs Class Trade-off

**Use struct** for:
- Value semantics desired (no shared mutation)
- Small size (fitting comfortably in registers/cache)
- No identity requirement
- Performance-critical code

**Use class** for:
- Reference semantics needed
- Identity matters (two objects at same address are "the same")
- Inheritance required
- Large mutable object shared across many contexts

## Bridging with Objective-C

Swift classes are compatible with Objective-C. Bridging happens automatically for many standard types:

```swift
let array: [String] = ["a", "b"]
let nsArray = array as NSArray  // Toll-free bridging
```

Reference counting rules remain the same; ARC manages both Swift and Objective-C objects transparently.

## See Also

Related concepts: [memory-management.md](memory-management.md), [language-swift.md](language-swift.md), [hardware-memory-hierarchy.md](hardware-memory-hierarchy.md), [compilers-garbage-collection.md](compilers-garbage-collection.md)