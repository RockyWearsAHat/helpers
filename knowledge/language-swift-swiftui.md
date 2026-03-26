# SwiftUI Architecture: Declarative UI, View Protocol, and State Management

## Introduction

**SwiftUI** (iOS 13+) is a **declarative** framework for building native UIs. Unlike imperative frameworks (UIKit) where you manually update views, SwiftUI expresses the desired state and the framework redraws when state changes. This model is reactive, composable, and significantly reduces boilerplate.

The architecture hinges on the `View` protocol, property wrappers (`@State`, `@Binding`, etc.), and a **dependency injection system** via environment and property accessors.

## Declarative Model vs Imperative

### Imperative (UIKit)

```swift
class ViewController: UIViewController {
    @IBOutlet weak var label: UILabel!
    
    override func viewDidLoad() {
        super.viewDidLoad()
        label.text = "Count: 0"  // Direct mutation
    }
    
    @IBAction func increment() {
        let current = Int(label.text?.dropFirst(7) ?? "0") ?? 0
        label.text = "Count: \(current + 1)"  // Manual view update
    }
}
```

You maintain the relationship between state and UI manually. If you forget to update the label, the UI becomes stale.

### Declarative (SwiftUI)

```swift
struct ContentView: View {
    @State private var count = 0
    
    var body: some View {
        VStack {
            Text("Count: \(count)")  // Expression of desired state
            Button("Increment") {
                count += 1  // State change triggers re-render
            }
        }
    }
}
```

You describe *what* the UI should look like given the state. SwiftUI handles re-rendering.

**Key insight**: The UI is a **pure function of state**. Change state → UI updates automatically.

## The View Protocol

### Core Requirements

```swift
protocol View: Identifiable {
    associatedtype Body: View
    var body: Body { get }
}
```

A `View` is any type with a `body` property that returns another `View`. Every SwiftUI component is a view.

### View Composition

Views compose declaratively, building a hierarchy:

```swift
struct ContentView: View {
    var body: some View {
        VStack {
            HeaderView()
            ListItemView(item: "Item 1")
            ListItemView(item: "Item 2")
            FooterView()
        }
    }
}

struct HeaderView: View {
    var body: some View {
        Text("Header").font(.headline)
    }
}

struct ListItemView: View {
    let item: String
    var body: some View {
        Text(item)
    }
}
```

SwiftUI compiles this hierarchy into an efficient internal representation.

## Property Wrappers for State Management

### @State

**@State** is the simplest state storage — the framework manages memory:

```swift
struct ContentView: View {
    @State private var isOn = false
    
    var body: some View {
        Toggle("Power", isOn: $isOn)  // $ creates a binding
    }
}
```

`@State` is private (scoped to a single view) and **invalidates the view** when changed, triggering a re-render.

**Lifetime**: The state persists across re-renders for the same view instance. If the view is destroyed and recreated, the state resets.

### @Binding

**@Binding** creates a two-way connection to another view's state:

```swift
struct ContentView: View {
    @State private var count = 0
    
    var body: some View {
        VStack {
            Text("Count: \(count)")
            CounterControl(count: $count)  // Pass binding
        }
    }
}

struct CounterControl: View {
    @Binding var count: Int
    
    var body: some View {
        HStack {
            Button("-") { count -= 1 }
            Button("+") { count += 1 }
        }
    }
}
```

`CounterControl` modifies the parent's state through the binding. Changes in the child update the parent's state.

**Key**: The `$` prefix converts `@State` or `@Binding` into a **binding** (get + set access), not a value.

### @StateObject

**@StateObject** manages lifecycle of an observable object:

```swift
class UserViewModel: ObservableObject {
    @Published var isLoggedIn = false
    @Published var username = ""
}

struct LogInView: View {
    @StateObject private var viewModel = UserViewModel()
    
    var body: some View {
        VStack {
            TextField("Username", text: $viewModel.username)
            Button("Log In") { viewModel.isLoggedIn = true }
        }
        .disabled(!viewModel.isLoggedIn)
    }
}
```

`@StateObject` is used for reference types (objects conforming to `ObservableObject`). The framework manages the object's lifecycle; mutations trigger re-renders if marked `@Published`.

**Difference from @State**: @State for value types, @StateObject for reference types.

### @ObservedObject

**@ObservedObject** subscribes to an external object's state changes:

```swift
struct DetailView: View {
    @ObservedObject var viewModel: UserViewModel  // Passed in from parent
    
    var body: some View {
        Text(viewModel.username)
    }
}

struct ListingView: View {
    @StateObject private var viewModel = UserViewModel()
    
    var body: some View {
        NavigationStack {
            DetailView(viewModel: viewModel)  // Pass the object
        }
    }
}
```

`@ObservedObject` does **not** own the object — it only listens for changes. The parent (`ListingView`) owns it via `@StateObject`.

### @EnvironmentObject

**@EnvironmentObject** injects dependencies globally without threading them through every parameter:

```swift
class Settings: ObservableObject {
    @Published var fontSize: Double = 14
}

@main
struct MyApp: App {
    @StateObject private var settings = Settings()
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(settings)  // Inject into view tree
        }
    }
}

struct TextDisplay: View {
    @EnvironmentObject var settings: Settings
    
    var body: some View {
        Text("Hello").font(.system(size: settings.fontSize))
    }
}
```

Any descendant of the view injected with `environmentObject()` can access `settings`. This avoids prop drilling.

### @Environment

**@Environment** accesses system-provided values:

```swift
struct ContentView: View {
    @Environment(\.colorScheme) var colorScheme  // Dark mode?
    @Environment(\.isEnabled) var isEnabled
    @Environment(\.font) var font
    
    var body: some View {
        if colorScheme == .dark {
            Text("Dark mode").foregroundColor(.white)
        } else {
            Text("Light mode")
        }
    }
}
```

`@Environment` is read-only and provides computed environment variables defined by SwiftUI or custom app logic.

## ViewBuilder: Transform Code into Views

**@ViewBuilder** is a result builder that allows multiple statements to construct a view:

```swift
@ViewBuilder
func conditionalContent(_ show: Bool) -> some View {
    if show {
        Text("Shown")
    } else {
        Text("Hidden")
    }
    
    if show {
        Image("icon")
    }
}

struct ContentView: View {
    var body: some View {
        VStack {
            conditionalContent(true)
        }
    }
}
```

Without `@ViewBuilder`, you'd need to write `Group { if show { ... } }` or other workarounds. The builder handles control flow transparently.

**Compiler magic**: `@ViewBuilder` transforms multiple statements into a single view expression.

## Preference Keys

**Preference keys** propagate values up the view hierarchy (opposite of environment):

```swift
struct MaxWidthPreferenceKey: PreferenceKey {
    static var defaultValue: CGFloat = 0
    
    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = max(value, nextValue())
    }
}

struct ChildView: View {
    @State private var width: CGFloat = 0
    
    var body: some View {
        Text("Child")
            .background(GeometryReader { geo in
                Color.clear
                    .preference(key: MaxWidthPreferenceKey.self, value: geo.size.width)
            })
    }
}

struct ParentView: View {
    @State private var maxWidth: CGFloat = 0
    
    var body: some View {
        VStack {
            ChildView()
            ChildView()
        }
        .onPreferenceChange(MaxWidthPreferenceKey.self) { value in
            maxWidth = value
        }
    }
}
```

Preferences bubble up, allowing parents to react to layout metrics from children.

## GeometryReader: Layout Introspection

**GeometryReader** provides access to the available space and position:

```swift
struct ContentView: View {
    var body: some View {
        GeometryReader { geometry in
            VStack {
                Text("Width: \(geometry.size.width)")
                Text("Height: \(geometry.size.height)")
                
                Rectangle()
                    .fill(Color.blue)
                    .frame(width: geometry.size.width / 2)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }
}
```

`geometry.size`, `geometry.frame(in:)` allow views to size themselves relative to available space.

## Animations

### Implicit Animation

`withAnimation` animates state changes:

```swift
struct ContentView: View {
    @State private var scale: CGFloat = 1.0
    
    var body: some View {
        VStack {
            Rectangle()
                .fill(Color.blue)
                .scaleEffect(scale)
            
            Button("Scale Up") {
                withAnimation(.easeInOut(duration: 0.5)) {
                    scale = 2.0
                }
            }
        }
    }
}
```

All changes within `withAnimation` are animated toward their final values.

### View Modifier Animation

```swift
.animation(.easeInOut, value: scale)  // Animate when scale changes
```

This animation runs whenever `scale` changes.

### Transition

```swift
if showDetail {
    DetailView()
        .transition(.move(edge: .trailing))
}
```

Transitions define how views appear/disappear.

## Navigation Patterns

### NavigationStack (iOS 16+)

```swift
struct ContentView: View {
    @State private var path: [String] = []
    
    var body: some View {
        NavigationStack(path: $path) {
            List {
                NavigationLink("Detail A", value: "detail-a")
                NavigationLink("Detail B", value: "detail-b")
            }
            .navigationDestination(for: String.self) { value in
                DetailView(id: value)
            }
        }
    }
}
```

`NavigationStack` manages the navigation hierarchy with a `path` array. Pushing is simple: `path.append(newValue)`.

### Sheet/Popover

```swift
struct ContentView: View {
    @State private var showSheet = false
    
    var body: some View {
        Button("Show Modal") { showSheet = true }
            .sheet(isPresented: $showSheet) {
                ModalView()
            }
    }
}
```

`.sheet()` presents a modal overlay. `.popover()` shows a popover.

## View Modifier Chaining

SwiftUI's fluent interface chains modifiers:

```swift
Text("Hello")
    .font(.headline)
    .foregroundColor(.blue)
    .padding()
    .background(Color.gray)
    .cornerRadius(8)
    .shadow(radius: 4)
```

Order matters. Modifiers applied later affect earlier layers in the rendering stack.

## Performance Optimization

### Identifying Re-renders

Wrap views in a debug helper:

```swift
struct DebugView: View {
    let name: String
    var body: some View {
        print("Rendering \(name)")
        return AnyView(...)  // Your content
    }
}
```

Excessive prints indicate unnecessary re-renders.

### Isolating State

`@State` and `@StateObject` should be as close to the view that uses them as possible:

```swift
// ✗ Bad: State in parent, many children re-render
struct ParentView: View {
    @State private var count = 0
    var body: some View {
        VStack {
            InputView(value: $count)
            HeavyComputationView()  // Re-renders unnecessarily
        }
    }
}

// ✓ Better: Move state to the child that needs it
struct ParentView: View {
    var body: some View {
        VStack {
            InputViewWithState()
            HeavyComputationView()  // Not affected by count changes
        }
    }
}

struct InputViewWithState: View {
    @State private var count = 0
    var body: some View { ... }
}
```

## Preview and Testing

### Xcode Previews

```swift
#Preview {
    ContentView()
        .environmentObject(Settings())
}

#Preview("Dark Mode") {
    ContentView()
        .preferredColorScheme(.dark)
}
```

Previews update live as you edit. Specify multiple previews with different configurations.

### Testing SwiftUI

```swift
func testButtonTap() {
    let view = ContentView()
    XCTAssertEqual(view.count, 0)
    // Tap button... (requires UI testing framework)
}
```

SwiftUI structure is often best tested indirectly via integration tests or by extracting business logic into view models.

## See Also

Related concepts: [mobile-ios-patterns.md](mobile-ios-patterns.md), [language-swift.md](language-swift.md), [web-state-management.md](web-state-management.md), [architecture-clean-hexagonal.md](architecture-clean-hexagonal.md)