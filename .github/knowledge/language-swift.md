# Swift Best Practices

## Swift Philosophy

Swift is designed for safety, performance, and expressiveness. It combines protocol-oriented programming with value semantics as first-class concepts.

- **Safety by default**: Optionals prevent null crashes, value types prevent shared mutation.
- **Protocol-oriented**: Prefer protocols over class inheritance.
- **Value types first**: Use structs unless you specifically need reference semantics.

## Value Types vs Reference Types

```swift
// Structs (value type) — preferred for most data types
struct Point {
    var x: Double
    var y: Double

    func distance(to other: Point) -> Double {
        let dx = x - other.x
        let dy = y - other.y
        return (dx * dx + dy * dy).squareRoot()
    }
}

var a = Point(x: 1, y: 2)
var b = a       // COPY — a and b are independent
b.x = 10       // Only b changes, a is untouched

// Classes (reference type) — use for identity, shared state, inheritance
class DatabaseConnection {
    let url: String
    private var isConnected = false

    init(url: String) { self.url = url }

    func connect() { isConnected = true }
}

let conn1 = DatabaseConnection(url: "localhost")
let conn2 = conn1  // REFERENCE — both point to same object
```

**When to use a class:**

- When identity matters (two objects at the same address are "the same").
- When you need inheritance.
- When you need reference semantics (shared, mutable state).
- Interop with Objective-C.

**Everything else → struct.**

## Optionals & Null Safety

```swift
// Optionals — no null pointer crashes
var name: String? = nil

// Safe unwrapping
if let name = name {
    print(name)
}

// Guard (early return pattern)
func greet(_ name: String?) -> String {
    guard let name else {
        return "Hello, stranger"
    }
    return "Hello, \(name)"
}

// Optional chaining
let length = user?.address?.street?.count

// Nil coalescing
let displayName = user.nickname ?? user.name ?? "Anonymous"

// map/flatMap on optionals
let uppercased: String? = name.map { $0.uppercased() }
let parsed: Int? = string.flatMap { Int($0) }

// ❌ NEVER force-unwrap unless you're 100% certain
let value = optional!  // Crashes if nil — avoid this

// ✅ Acceptable force-unwrap: truly impossible to be nil
let url = URL(string: "https://apple.com")!  // Known-valid literal
```

## Protocol-Oriented Programming

```swift
// Define behavior through protocols
protocol Renderable {
    func render() -> String
}

protocol Cacheable {
    var cacheKey: String { get }
    var ttl: TimeInterval { get }
}

// Protocol extensions — default implementations
extension Cacheable {
    var ttl: TimeInterval { 300 }  // Default 5 minutes
}

// Protocol composition
func save(_ item: Renderable & Cacheable) {
    let html = item.render()
    cache.set(item.cacheKey, value: html, ttl: item.ttl)
}

// Conform value types to protocols
struct Article: Renderable, Cacheable {
    let title: String
    let body: String

    var cacheKey: String { "article-\(title.hashValue)" }

    func render() -> String {
        "<h1>\(title)</h1><p>\(body)</p>"
    }
}

// Existential types (any) vs generics (some)
func process(_ items: [any Renderable]) { ... }     // Dynamic dispatch (slower)
func process<T: Renderable>(_ items: [T]) { ... }   // Static dispatch (faster)
func process(_ items: [some Renderable]) { ... }     // Opaque type (Swift 5.7+)
```

## Enums with Associated Values

Swift enums are algebraic data types — far more powerful than C/Java enums.

```swift
enum Result<Success, Failure: Error> {
    case success(Success)
    case failure(Failure)
}

enum NetworkError: Error {
    case notConnected
    case timeout(seconds: Int)
    case httpError(statusCode: Int, body: String)
    case decodingFailed(underlying: Error)
}

enum Route {
    case home
    case profile(userId: String)
    case settings
    case search(query: String, page: Int)
}

// Pattern matching
func handle(_ route: Route) {
    switch route {
    case .home:
        showHome()
    case .profile(let userId):
        showProfile(userId)
    case .search(let query, let page) where page > 1:
        showSearchResults(query, page: page)
    case .search(let query, _):
        showSearchResults(query, page: 1)
    case .settings:
        showSettings()
    }
}

// if case (single pattern)
if case .httpError(let code, _) = error, code == 401 {
    refreshToken()
}
```

## Async/Await & Structured Concurrency

```swift
// Async function
func fetchUser(id: String) async throws -> User {
    let (data, response) = try await URLSession.shared.data(from: url)
    guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
        throw NetworkError.httpError(statusCode: 0, body: "")
    }
    return try JSONDecoder().decode(User.self, from: data)
}

// Sequential
let user = try await fetchUser(id: "123")
let posts = try await fetchPosts(userId: user.id)

// Parallel with async let
async let user = fetchUser(id: "123")
async let config = fetchConfig()
async let notifications = fetchNotifications()
let (u, c, n) = try await (user, config, notifications)

// TaskGroup for dynamic parallelism
func fetchAll(ids: [String]) async throws -> [User] {
    try await withThrowingTaskGroup(of: User.self) { group in
        for id in ids {
            group.addTask { try await fetchUser(id: id) }
        }
        var users: [User] = []
        for try await user in group {
            users.append(user)
        }
        return users
    }
}

// Actor (safe shared mutable state)
actor Cache {
    private var storage: [String: Data] = [:]

    func get(_ key: String) -> Data? {
        storage[key]
    }

    func set(_ key: String, value: Data) {
        storage[key] = value
    }
}

let cache = Cache()
await cache.set("key", value: data)  // Actor-isolated — thread-safe
```

## Codable (Serialization)

```swift
struct User: Codable {
    let id: String
    let name: String
    let email: String
    let createdAt: Date

    enum CodingKeys: String, CodingKey {
        case id, name, email
        case createdAt = "created_at"  // Map JSON keys to Swift names
    }
}

// Encode
let encoder = JSONEncoder()
encoder.dateEncodingStrategy = .iso8601
let data = try encoder.encode(user)

// Decode
let decoder = JSONDecoder()
decoder.dateDecodingStrategy = .iso8601
let user = try decoder.decode(User.self, from: data)

// Decode array
let users = try decoder.decode([User].self, from: data)
```

## Error Handling

```swift
// Typed errors (thrown and caught)
enum ValidationError: Error, LocalizedError {
    case tooShort(minimum: Int)
    case invalidFormat(String)
    case duplicateEmail

    var errorDescription: String? {
        switch self {
        case .tooShort(let min): "Must be at least \(min) characters"
        case .invalidFormat(let msg): "Invalid format: \(msg)"
        case .duplicateEmail: "Email already in use"
        }
    }
}

// throw / try / catch
func validate(_ email: String) throws -> String {
    guard email.contains("@") else {
        throw ValidationError.invalidFormat("missing @")
    }
    return email.lowercased()
}

do {
    let email = try validate(input)
    save(email)
} catch let error as ValidationError {
    showError(error.localizedDescription)
} catch {
    showError("Unexpected error: \(error)")
}

// try? — convert to optional (swallow error)
let email = try? validate(input)  // nil if throws

// Result type for async callbacks
func fetch(completion: @escaping (Result<Data, Error>) -> Void) { ... }
```

## Swift Conventions

- **Argument labels**: `move(from: a, to: b)` — reads like English.
- **Omit first argument label** when it's obvious: `contains(_ element:)`.
- **Use `self` only when required** (closures, disambiguation).
- **Prefer `let` over `var`** — immutable by default.
- **Use `guard` for early exits**, `if let` for optional binding within a scope.
- **Use extensions to organize** conformances and functionality logically.

## Tooling

| Tool                      | Purpose                             |
| ------------------------- | ----------------------------------- |
| **SwiftLint**             | Style and convention enforcement    |
| **SwiftFormat**           | Code formatting                     |
| **XCTest**                | Unit/integration testing (built-in) |
| **Swift Testing**         | Modern test framework (Swift 5.10+) |
| **Instruments**           | Performance profiling               |
| **Swift Package Manager** | Dependency management               |

---

_Sources: Swift Programming Language (Apple), Swift API Design Guidelines, Protocol-Oriented Programming in Swift (WWDC), Swift Evolution proposals_
