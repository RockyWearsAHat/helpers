# Swift Concurrency: Async/Await, Structured Concurrency, and Actors

## Introduction

Swift's concurrency model (introduced in Swift 5.5, 2021) provides **structured concurrency** with `async`/`await`, **actor isolation** for thread safety without locks, and integration with **Combine** for reactive programming. This replaces callback-based and manual threading approaches with a safer, more readable paradigm.

The core insight: concurrency structure mirrors the call stack. Tasks form parent-child hierarchies; when a parent completes, all children are cancelled. This eliminates many concurrency bugs inherent in callback-based systems.

## Core Concepts: Async/Await and Suspension

### Async Functions

An `async` function can **suspend** — pause execution and resume later without blocking the underlying thread:

```swift
func fetchUser(id: Int) async throws -> User {
    let data = try await URLSession.shared.data(from: url).0
    return try JSONDecoder().decode(User.self, from: data)
}

// Called from async context
let user = try await fetchUser(id: 42)
```

When `await` is called, the function volunteers to suspend. The thread it runs on becomes free to execute other tasks. This differs from blocking calls (threads stuck waiting) — the same thread can run hundreds of concurrent tasks.

### Safety with Async/Await

- **No callback pyramids**: Code reads sequentially top-to-bottom, matching how execution actually flows
- **Automatic error propagation**: Errors in awaited calls bubble up without explicit wrapping
- **Local variables keep their scope**: Variables live for the lifetime of the async function, not the task
- **Compiler enforces `await` at suspension points**: You can't accidentally forget, unlike callbacks

## Structured Concurrency

### Task Hierarchy

Tasks form a **tree structure**:

```swift
@main
struct App {
    static func main() async {
        async {
            async {
                // Grandchild task
            }
            // Parent waits for all children before exiting
        }
    }
}
```

When a parent exits, all descendant tasks are **automatically cancelled**. This prevents resource leaks and unfinished background work — a common source of crashes in callback-based systems.

### Task and Detached Tasks

```swift
// Inherits parent's task context (priority, cancellation, etc.)
Task {
    let result = try await someAsyncWork()
}

// Independent scope — no automatic cancellation from parent
Task.detached { @MainActor in
    self.updateUI()
}

// Async let binds multiple concurrent operations
async let user = fetchUser()
async let posts = fetchPosts()
let (u, p) = try await (user, posts)  // Wait for both
```

**Key distinction**: `async let` forks work concurrently but waits at the binding site, ensuring all results are available before proceeding. If one fails, the error propagates and the other task is cancelled.

## Actor Isolation and Thread Safety

### The Actor Model

An **actor** is a type that executes all its methods **serially** on a single implicit queue. No two methods run simultaneously. This eliminates data races — multiple tasks can safely access the same actor's state without locks:

```swift
actor UserDatabase {
    private var users: [Int: User] = [:]
    
    func addUser(_ user: User) {
        users[user.id] = user  // Serialized — only one task at a time
    }
    
    func getUser(id: Int) -> User? {
        users[id]
    }
}

// Usage across tasks
let db = UserDatabase()
async {
    await db.addUser(user1)
}
async {
    let user = await db.getUser(id: 1)  // Safe — no race
}
```

Accessing an actor from outside requires `await` to cross the isolation boundary. This visual signal reminds you of the implicit serialization cost.

### MainActor

`@MainActor` is a singleton actor that runs on the main thread. UI updates must occur on the main thread (iOS/macOS requirement):

```swift
@MainActor
class ViewController: UIViewController {
    var label: UILabel
    
    func updateUI(with text: String) {  // Implicitly runs on main
        label.text = text
    }
}

// From background task
Task {
    let result = try await fetchData()
    await viewController.updateUI(with: result)
}
```

Marking a type `@MainActor` propagates the requirement to all methods. Individual methods can also be isolated:

```swift
actor DataStore {
    @MainActor func updateUI() { }  // This method must run on main
}
```

## Sendable and Data-Race Safety

### The Sendable Protocol

A type is `Sendable` if it's safe to **send across isolation boundaries** (actor boundaries, thread boundaries):

```swift
// Safe types (automatic conformance)
extension Int: Sendable { }  // Value types
extension String: Sendable { }
extension Array: Sendable where Element: Sendable { }

// Custom sendable types
struct User: Sendable {
    let id: Int
    let name: String
    // Only immutable stored properties or properly protected mutable state
}

// Immutable classes are sendable
final class Config: Sendable {
    let apiKey: String
    init(apiKey: String) { self.apiKey = apiKey }
}
```

Non-`Sendable` types cannot be safely passed between actor boundaries without compiler errors. This catches data-race bugs at compile time.

### Escaping and Captures

Closures that escape an async context must capture only `Sendable` types:

```swift
func startBackgroundTask(callback: @escaping @Sendable () -> Void) {
    Task.detached {
        await doWork()
        callback()  // Closure captures only Sendable values
    }
}
```

## AsyncSequence and AsyncStream

### AsyncSequence

An `AsyncSequence` is like `Sequence`, but iteration can suspend:

```swift
for try await line in process.standardOutput.lines {
    print(line)  // Waits for each line without blocking
}
```

Common async sequences:
- `URLSession.bytes(from:)` — streaming HTTP response
- `FileHandle.bytes` — file reading
- `Process.standardOutput.lines` — shell command output
- Custom async sequences via `AsyncStream`

### AsyncStream

Create a custom source of async values:

```swift
let stream = AsyncStream<Int> { continuation in
    var count = 0
    let timer = Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { _ in
        count += 1
        continuation.yield(count)
        if count >= 10 {
            continuation.finish()
        }
    }
}

for await value in stream {
    print(value)
}
```

Continuation is `Sendable`, so you can pass it across task boundaries safely.

## TaskGroup

### Spawning Multiple Concurrent Tasks

`TaskGroup` runs subtasks concurrently and collects results:

```swift
func fetchAllUsers(ids: [Int]) async throws -> [User] {
    var users: [User] = []
    
    try await withThrowingTaskGroup(of: User.self) { group in
        for id in ids {
            group.addTask {
                try await fetchUser(id: id)
            }
        }
        
        for try await user in group {
            users.append(user)
        }
    }
    
    return users
}
```

**Behavior**:
- Tasks run concurrently (not sequentially)
- `withThrowingTaskGroup` exits when all tasks complete
- If any task throws, the group cancels remaining tasks and propagates the error
- Non-throwing variant: `withTaskGroup`

## Cancellation and Task Lifecycle

### Cancellation

Tasks can be cancelled, signaling "stop working":

```swift
let task = Task {
    try await doLongWork()
}

// Later, cancel
task.cancel()
```

Inside an async function, check cancellation status:

```swift
func doLongWork() async throws {
    for i in 0..<1000 {
        try Task.checkCancellation()  // Throws CancellationError
        await doWork(i)
    }
}
```

Structured concurrency automatically cancels child tasks when the parent is cancelled — no manual tracking needed.

### Task Scheduling and Priority

Tasks can have priorities (`.high`, `.normal`, `.low`, `.background`):

```swift
Task(priority: .high) {
    await importantWork()
}
```

Priority hints to the scheduler; it doesn't force preemption.

## Combine vs Async/Await

### Conceptual Difference

| Combine | Async/Await |
|---------|------------|
| Publisher-Subscriber pattern (event streams) | Sequential function calls with suspension |
| Declarative transformations (map, filter) | Imperative control flow (if, for loops) |
| Time-based operators (debounce, throttle) | Straightforward sequencing |
| Multicasting (multiple subscribers) | Single-use values |

### When to Use Each

**Combine**: Reactive UIs with frequent updates from multiple sources. SwiftUI `@ObservedObject` integrates well.

```swift
@ObservedObject var viewModel = ViewModel()

var body: some View {
    Text(viewModel.displayText)  // Updates when publisher emits
}
```

**Async/Await**: One-shot operations (API calls, file I/O), sequential logic, simpler mental model.

```swift
async {
    let data = try await fetchData()
    let processed = transform(data)
    save(processed)
}
```

Modern Swift tends toward async/await for simplicity; Combine remains useful for complex reactive scenarios.

## Continuations

### Manual Suspension Points

Adapt callback-based APIs to async/await using continuations:

```swift
func fetchData() async throws -> Data {
    return try await withCheckedThrowingContinuation { continuation in
        let task = URLSession.shared.dataTask(with: url) { data, _, error in
            if let error = error {
                continuation.resume(throwing: error)
            } else if let data = data {
                continuation.resume(returning: data)
            }
        }
        task.resume()
    }
}
```

**Safety**: 
- `withCheckedThrowingContinuation`: Checked at runtime (debug build verifies exactly one resume)
- `withUnsafeThrowingContinuation`: Unchecked (faster, requires explicit verification)

Continuations must be resumed exactly once. Forgetting to resume leaks the task forever.

## Integration and Ecosystem

### Async Context Requirements

Many async functions must run in an async context. Starting an async context requires `Task`:

```swift
// Button action (not async)
Button("Fetch") {
    Task {
        let user = try await fetchUser()
        await updateUI(user)
    }
}
```

Prefer `task(id:priority:_:)` for SwiftUI lifecycle:

```swift
.task {
    let user = try await fetchUser()
}
```

This restarted if the ID changes; cancellation is automatic on view dismissal.

### Testing with Async/Await

XCTest supports async test methods:

```swift
func testFetchUser() async throws {
    let user = try await fetchUser(id: 1)
    XCTAssertEqual(user.name, "Alice")
}
```

No need for expectations or completion handlers.

## See Also

Related concepts: [paradigm-concurrent-models.md](paradigm-concurrent-models.md), [language-kotlin-coroutines.md](language-kotlin-coroutines.md), [web-event-loop.md](web-event-loop.md), [mobile-ios-patterns.md](mobile-ios-patterns.md)