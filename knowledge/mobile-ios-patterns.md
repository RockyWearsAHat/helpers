# iOS Development Patterns — UIKit, SwiftUI, Architecture & Memory

## UIKit vs SwiftUI Lifecycle

iOS developers must choose between two UI frameworks with fundamentally different paradigms.

### UIKit (Imperative)

UIKit is the **original iOS framework** (since iOS 2). Event-driven, imperative, and based on view controllers:

```swift
class ViewController: UIViewController {
  @IBOutlet weak var label: UILabel!
  
  override func viewDidLoad() {
    super.viewDidLoad()
    label.text = "Hello"
  }
  
  @IBAction func buttonTapped() {
    label.text = "Button tapped"
  }
}
```

Key lifecycle events:
- `viewDidLoad()`: Called once when view is loaded
- `viewWillAppear()`: Before the view appears
- `viewDidAppear()`: After the view is visible
- `viewWillDisappear()`, `viewDidDisappear()`: On exit

UIKit requires manual view management; you reference views directly and update them via method calls. State lives in the view controller.

### SwiftUI (Declarative)

SwiftUI (iOS 13+) is **reactive and declarative**:

```swift
struct ContentView: View {
  @State var text = "Hello"
  
  var body: some View {
    VStack {
      Text(text)
      Button("Tap me") {
        text = "Button tapped"
      }
    }
  }
}
```

State changes trigger a re-render of the view hierarchy. No manual view management. The framework handles lifecycle events implicitly.

### Lifecycle Differences

| Event | UIKit | SwiftUI |
|-------|-------|---------|
| View created | `viewDidLoad()` | View struct initialized |
| View appears | `viewWillAppear()`, `viewDidAppear()` | `.onAppear()` modifier |
| State changes | Explicit: `label.text = ...` | Implicit: `@State` modification triggers re-render |
| Memory cleanup | `deinit` in view controller | Automatic (View is value type) |

**Hybrid Approach**: Many production apps use both. UIKit for complex screens that are easier to test and debug; SwiftUI for simpler UI. Use `UIViewControllerRepresentable` and `UIViewRepresentable` to bridge them.

## MVVM-C (Model-View-ViewModel Coordinator)

MVVM separates UI logic from business logic; the Coordinator pattern manages navigation.

### The Pattern

```swift
// Model
struct User {
  let id: Int
  let name: String
}

// ViewModel (observable for SwiftUI or Combine)
class UserListViewModel: ObservableObject {
  @Published var users: [User] = []
  @Published var isLoading = false
  
  func fetchUsers() async {
    isLoading = true
    users = await userService.getUsers()
    isLoading = false
  }
}

// View
struct UserListView: View {
  @StateObject var viewModel = UserListViewModel()
  
  var body: some View {
    List(viewModel.users) { user in
      Text(user.name)
    }
    .onAppear { Task { await viewModel.fetchUsers() } }
  }
}

// Coordinator (manages navigation flow)
class AppCoordinator {
  func showUserList() {
    let vm = UserListViewModel()
    let view = UserListView(viewModel: vm)
    // Navigate...
  }
}
```

The ViewModel contains:
- **Computed state** (`@Published` properties for SwiftUI or Combine subscribers)
- **Business logic** (networking, caching, calculations)
- **No UI references** (testable in isolation)

The Coordinator:
- Owns the navigation stack
- Instantiates view controllers / SwiftUI views
- Decides which screen comes next based on user actions

## Combine vs Async/Await

Both are Swift's reactive programming models:

### Combine (Framework, iOS 13+)

Combine uses the **Publisher-Subscriber pattern**:

```swift
class UserViewModel: ObservableObject {
  @Published var user: User?
  
  func fetchUser(id: Int) {
    URLSession.shared.dataTaskPublisher(for: url)
      .decode(type: User.self, decoder: JSONDecoder())
      .receive(on: DispatchQueue.main)
      .assign(to: &$user)
  }
}

// SwiftUI automatically subscribes to @Published
```

Combine excels at:
- Chaining async operations (map, flatMap, filter)
- Combining multiple async sources
- UI bindings via `@Published`

Downsides: Steep learning curve, verbose syntax.

### Async/Await (Language feature, iOS 15+)

Async/await is **structured concurrency**:

```swift
class UserViewModel: ObservableObject {
  @Published var user: User?
  
  func fetchUser(id: Int) async {
    do {
      let (data, _) = try await URLSession.shared.data(from: url)
      let user = try JSONDecoder().decode(User.self, from: data)
      await MainActor.run { self.user = user }
    } catch {
      print("Error: \(error)")
    }
  }
}

// Call it:
Task { await viewModel.fetchUser(id: 1) }
```

Async/await is:
- Easier to read (imperative, not reactive)
- More familiar to JavaScript developers
- Safer (structured task cancellation)
- The modern Apple recommendation

**Recommendation**: Use async/await for straightforward sequential operations. Use Combine for complex event streams (e.g., user input debouncing, merging multiple sources).

## Core Data vs SwiftData

Both are local persistence frameworks.

### Core Data (Legacy, iOS 3+)

Core Data is an **object-relational mapper**:

```swift
@NSManaged var id: NSNumber
@NSManaged var name: String

let fetchRequest = NSFetchRequest<User>(entityName: "User")
fetchRequest.predicate = NSPredicate(format: "id == %@", userId)
let results = try! context.fetch(fetchRequest)
```

Core Data:
- Powerful: relationships, predicates, migrations
- Complex: steep learning curve, NSFetchRequest boilerplate
- Concurrent: requires handling multiple managed object contexts

### SwiftData (Modern, iOS 17+)

SwiftData is Apple's **Swift-native replacement**:

```swift
@Model final class User {
  var id: Int
  var name: String
}

// Query:
@Query var users: [User]

// Modify:
modelContext.insert(newUser)
```

SwiftData:
- Simple declarative syntax
- Automatic persistence
- Built-in SwiftUI integration via `@Query`
- Limited platform support (iOS 17+ only)

**Choice**: For iOS 17+ apps, use SwiftData. For broader compatibility, use Core Data. Hybrid: SwiftData for new code, Core Data for legacy screens.

## Testing: XCTest & XCUITest

iOS provides two testing frameworks:

### XCTest (Unit & Integration Tests)

```swift
class UserViewModelTests: XCTestCase {
  var viewModel: UserViewModel!
  
  override func setUp() {
    super.setUp()
    viewModel = UserViewModel(service: MockUserService())
  }
  
  func testFetchUserLoadsData() async {
    await viewModel.fetchUser(id: 1)
    XCTAssertNotNil(viewModel.user)
  }
}
```

Run with `CMD+U` in Xcode. Good for testing ViewModels and business logic in isolation.

### XCUITest (UI Tests)

```swift
func testUserListTap() {
  let app = XCUIApplication()
  app.launch()
  app.tables.cells.element(boundBy: 0).tap()
  XCTAssertTrue(app.navigationBars["User Details"].exists)
}
```

XCUITest:
- Launches the app in a separate process
- Interacts with UI elements via accessibility identifiers
- Slow but comprehensive

**Accessibility Testing**: XCTest can audit accessibility with `XCUIApplication().performAccessibilityAudit()`. XCUITest can verify VoiceOver compatibility.

## Accessibility (VoiceOver)

VoiceOver is iOS's screen reader. Developers must support:

- **Accessibility Labels**: What each UI element does
- **Accessibility Hints**: Extra context (e.g., "double-tap to expand")
- **Accessibility Container**: Group related elements

```swift
Text("Profile")
  .accessibilityLabel("Profile section")
  .accessibilityAddTraits(.isButton)

VStack {
  HStack { /* contents */ }
    .accessibilityElement(children: .combine)
    .accessibilityLabel("Contact card")
}
```

To test: Settings > Accessibility > VoiceOver, then swipe with 3 fingers. UI should be fully navigable without sight.

## Code Signing & Provisioning

Deploying to the App Store requires certificates and provisioning profiles:

### Code Signing

- **Development Certificate**: Signed by Apple; identifies you as developer. Stored in Keychain.
- **Distribution Certificate**: Used for App Store builds. Different from development.

### Provisioning Profiles

A provisioning profile maps:
- A Bundle ID (app identifier, e.g., `com.example.myapp`)
- Your certificate(s)
- Device UDIDs (for development only)
- Entitlements (push, iCloud, etc.)

**Xcode Managed Signing** (default): Xcode automatically generates and renews profiles. Simplest for most developers.

**Manual Signing**: Full control via Apple Developer account. Required for enterprise builds or complex entitlements.

### App Store Review

The App Store review process:
1. **Submission** via Xcode / Transporter
2. **Initial Review** (1-2 days): Apple checks for crashes, violations of guidelines
3. **Resolution** if rejected (common reasons: crashing, unclear purpose, missing privacy policy)
4. **Approval**: App available on App Store

Common rejections:
- Using private APIs
- No clear privacy policy
- Misleading screenshots
- Subscriptions without transparent terms

## Memory Management (ARC)

Swift uses **Automatic Reference Counting (ARC)** at compile time:

```swift
class User {
  var name: String
}

var user: User? = User(name: "Alice")
user = nil  // ARC deallocates user when refcount hits 0
```

### Retain Cycles (Common Pitfall)

```swift
class ViewController: UIViewController {
  let service = Service()
  
  override func viewDidLoad() {
    service.callback = {
      // This closure captures self, which captures service...
      // Retain cycle: self -> service -> closure -> self
      self.updateUI()
    }
  }
}

// Fix: Use [weak self]
service.callback = { [weak self] in
  self?.updateUI()
}
```

**Closure captures**: Always use `[weak self]` or `[unowned self]` in closures that outlive the scope.

**Delegate pattern**: Use `weak` for delegate properties to avoid cycles:

```swift
protocol UserDelegate: NSObjectProtocol {
  func userDidUpdate()
}

class UserService {
  weak var delegate: UserDelegate?  // weak to prevent cycle
}
```

## See Also

- **architecture-clean-hexagonal** — General clean architecture principles
- **mobile-react-native** — Alternative cross-platform framework
- **testing-philosophy** — General testing strategies