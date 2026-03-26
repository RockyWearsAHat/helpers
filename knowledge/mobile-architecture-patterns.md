# Mobile Architecture Patterns — MVC/MVP/MVVM/MVI, Clean Architecture, State Management & Dependency Injection

## Overview

Mobile applications require architecture patterns that separate concerns across presentation, business logic, and data layers. The mobile constraint set—limited memory, expensive network, session interruption, platform-specific lifecycle—drives pattern adoption. Patterns have evolved from MVC toward more testable, state-controlled models like MVVM and MVI.

## The Architecture Pattern Spectrum

### MVC (Model-View-Controller)

MVC separates code into three responsibilities:

- **Model**: Business logic, data, validation. Platform-independent.
- **View**: UI rendering and layout. Platform-specific (XML in Android, Storyboards in iOS).
- **Controller**: Receives user input, updates Model, notifies View of changes.

**Mobile Implementation**: Android Activities often act as Controllers+Views (mixed responsibility). iOS UIViewController similarly conflates controller and view.

**Strengths**: Simple conceptual model, good for prototypes.

**Weaknesses**: View and Controller coupling makes testing difficult. Lifecycle management (Activity recreation on rotation) spreads logic across framework callbacks, not model updates. The "massive ViewController/Activity" problem results.

### MVP (Model-View-Presenter)

MVP inverts the dependency: the Presenter orchestrates the Model and tells the View what to render. The View becomes **passive** (no business logic).

```
Model ← Presenter → View
```

- **Model**: Pure business logic, no platform dependencies.
- **View**: Dumb renderer. Implements an interface (IUserView). Receives commands from Presenter.
- **Presenter**: Receives user events from View, updates Model, calls View methods to render results.

**Example (Android or iOS)**:

```kotlin
interface UserView {
  fun showUser(name: String, email: String)
  fun showError(message: String)
}

class UserPresenter(val model: UserRepository) {
  var view: UserView? = null
  
  fun loadUser(id: String) {
    try {
      val user = model.getUser(id)
      view?.showUser(user.name, user.email)
    } catch (e: Exception) {
      view?.showError(e.message ?: "Unknown error")
    }
  }
}
```

**Strengths**: Testable Presenter (mocks the View). Clean separation of concerns. View is replaceable (animate differently, test with a spy).

**Weaknesses**: More boilerplate than MVC. View still holds transient state (scroll position, focus). Presenter doesn't know about view lifecycle, leading to memory leaks if Presenter holds a reference to a destroyed View.

### MVVM (Model-View-ViewModel)

MVVM uses **data binding** to automatically sync ViewModel state changes to the View.

- **Model**: Business logic and data.
- **View**: Declarative UI (binds to ViewModel properties). Platform-specific.
- **ViewModel**: Holds UI state (selected filter, pagination), derived from Model. Publishes observable properties.

The binding framework (Android Data Binding, SwiftUI @State, LiveData) watches ViewModel properties and updates View automatically. View observes ViewModel; ViewModel does not reference View.

```kotlin
class UserViewModel(val repository: UserRepository) : ViewModel() {
  val user = MutableLiveData<User>()
  val loading = MutableLiveData(false)
  
  fun loadUser(id: String) {
    loading.value = true
    viewModelScope.launch {
      try {
        user.value = repository.getUser(id)
      } finally {
        loading.value = false
      }
    }
  }
}

// In Activity: observe user. UI updates automatically when user LiveData changes.
viewModel.user.observe(this) { user ->
  nameView.text = user.name
}
```

**Strengths**: Automatic synchronization eliminates glue code. ViewModel survives configuration changes (Android) or is retained across View dismissals. TestableViewModel (inject mock repository).

**Weaknesses**: Learning curve for binding frameworks. Too much state in ViewModel can become complex. Binding errors are hard to debug.

**Platform Adoption**: MVVM is standard on Android (Jetpack ViewModel + LiveData/StateFlow). iOS adopted the pattern with combined View+ViewModel in SwiftUI (computed @State).

### MVI (Model-View-Intent)

MVI enforces **unidirectional data flow**: all state changes flow through a single intent stream, the model updates, and the view re-renders.

- **Model**: Single source of truth for app state. Immutable.
- **View**: Declarative, renders the entire Model state.
- **Intent**: User actions or system events (user tapped button, timer fired).

```
User Action → Intent → Model (updated) → View (re-renders)
```

**Example**:

```kotlin
sealed class UserIntent {
  data class LoadUser(val id: String) : UserIntent()
  object Retry : UserIntent()
}

data class UserState(
  val user: User? = null,
  val loading: Boolean = false,
  val error: String? = null
)

class UserModel(val repository: UserRepository) {
  fun reduce(state: UserState, intent: UserIntent): UserState {
    return when (intent) {
      is UserIntent.LoadUser -> state.copy(loading = true)
      UserIntent.Retry -> state.copy(loading = true, error = null)
    }
  }
}
```

**Strengths**: Predictable state transitions. Time-travel debugging possible. Entire app behavior reproducible from intent log. Testable reducer functions.

**Weaknesses**: More verbose than MVVM. Requires event sourcing mindset. Side effects (async calls) must be handled separately.

**Adoption**: Popular in reactive frameworks (RxJava, Kotlin Flows). Cycle.js pioneered this model on web; mobile adoption follows.

## Clean Architecture on Mobile

Clean Architecture (Uncle Bob) organizes code into concentric layers by dependency direction: dependencies point inward toward business logic, never outward.

```
┌─────────────────────────────────┐
│         UI / Presenter          │
├─────────────────────────────────┤
│     Use Cases / Interactors     │
├─────────────────────────────────┤
│     Entities / Domain Models    │
├─────────────────────────────────┤
│  Frameworks & Drivers (Android) │
└─────────────────────────────────┘
```

**Mobile Practice**:

- **Entity/Domain**: `User`, `Transaction` (platform-independent, pure Kotlin/Swift).
- **Use Cases**: `GetUserUseCase`, `ValidatePasswordUseCase`. Returns `Result<T>` or exceptions.
- **Presenter**: Calls use cases, formats for UI.
- **Platform Layer**: Android framework, iOS UIKit, networking clients.

**Benefit**: Business logic is fully testable without mocks. UI frameworks are swappable (native → React Native → Flutter).

**Drawback**: More files and layers. Small projects can over-engineer.

## Unidirectional Data Flow (Redux-like)

Data flows one direction: actions dispatch → state updates → view re-renders. Prevents inconsistency from multiple sources mutating state.

**Core Concepts**:

- **Actions**: Events that express intent. Immutable.
- **Reducer/Store**: Pure function: `(state, action) → newState`. No side effects.
- **View**: Subscribes to state, calls `dispatch(action)` on user input.

**Redux Pattern on Mobile**:

```kotlin
// Action
data class UserLoaded(val user: User)

// State
data class AppState(val users: List<User> = emptyList())

// Reducer
fun appReducer(state: AppState, action: Any): AppState {
  return when (action) {
    is UserLoaded -> state.copy(users = state.users + action.user)
    else -> state
  }
}

// Store (simplified)
class Store(initial: AppState) {
  var state = initial
  val subscribers = mutableListOf<(AppState) -> Unit>()
  
  fun dispatch(action: Any) {
    state = appReducer(state, action)
    subscribers.forEach { it(state) }
  }
  
  fun subscribe(fn: (AppState) -> Unit) = subscribers.add(fn)
}
```

**Libraries**: Redux.js (web), Redux Saga; on mobile, MobX, Riverpod (Flutter), Redux-like patterns in Kotlin Flows.

**Tradeoffs**: Immutability prevents bugs but requires careful copy patterns. Middleware is needed for async effects (thunks).

## State Management Frameworks

### LiveData (Android Jetpack)

Android-specific observable holder emitting to ViewModel lifecycle:

```kotlin
val user: LiveData<User> = MutableLiveData()
```

**Scope**: Tied to ViewModel lifecycle. Automatically removes observers when Activity/Fragment destroyed.

**Limitations**: Single value (not a stream). No backpressure. Being superseded by StateFlow.

### StateFlow (Kotlin Coroutines)

Reactive stream of state updates. Part of Kotlin Flow API.

```kotlin
val user: StateFlow<User?> = MutableStateFlow(null)
```

**Advantage**: Same value semantics as LiveData, but works with suspend functions. Composable with other Flow operators.

### SwiftUI @State and @StateObject

SwiftUI uses property wrappers to declare observable state:

```swift
@State var count = 0
@StateObject var viewModel = UserViewModel()
```

Recomposition on state change (similar to React hooks).

## Navigation Patterns

### Stack-Based (Push/Pop)

Screens layer as a stack. "Back" pops to previous screen. Common in hierarchical UIs (settings → detail → edit).

Android: Fragment transactions, Navigation component. iOS: UINavigationController, NavigationStack in SwiftUI.

### Tab-Based

Multiple independent stacks under tabs. Switching tabs returns to last visited screen in that tab.

Android: BottomNavigationView. iOS: UITabBarController.

### Modal/Overlay

Screen appears above current screen. Typically user dismisses via button or back.

Android: DialogFragment. iOS: Modally presented UIViewController, Sheet in SwiftUI.

## Dependency Injection on Mobile

### Manual Injection (Constructor)

```kotlin
class UserRepository(val apiClient: ApiClient, val db: Database)
class UserViewModel(val repo: UserRepository)
```

**Scales to**: Small projects. Verbose for 20+ dependencies.

### Service Locator Anti-Pattern

```kotlin
object ServiceLocator {
  fun getUserRepo() = UserRepository(apiClient, db)
}
```

**Problem**: Hidden dependencies. Testing requires manual setup.

### Hilt (Android)

Annotation-based dependency injection with code generation.

```kotlin
@HiltViewModel
class UserViewModel @Inject constructor(
  val repository: UserRepository
) : ViewModel()

@AndroidEntryPoint
class MainActivity : AppCompatActivity()
```

Hilt manages lifecycle, scopes (Singleton, ViewModel, Activity), and constructor resolution.

### Koin (Kotlin Multiplatform)

Lightweight, DSL-based DI. Works across Android, iOS, backend.

```kotlin
val koinModule = module {
  single { UserRepository(get(), get()) }
  viewModel { UserViewModel(get()) }
}

// In app:
val viewModel: UserViewModel by viewModel()
```

**Tradeoff**: More runtime overhead than Hilt (no compile-time generation). Better for multiplatform projects.

### Manual Hilt/Koin Decisions

- **Hilt**: Android-only, compile-safe, built by Google.
- **Koin**: Multiplatform flexibility, simpler DSL, runtime.
- **Manual**: Control, no magic, good for small codebases.

## Modularization

Organizing code into modules (feature modules, core modules) reduces build time, improves IDE responsiveness, and enforces layering.

**Structure**:

```
app/
core/
  domain/
  data/
  ui/ (shared components)
feature/
  users/
    domain/
    data/
    ui/
  posts/
    domain/
    data/
    ui/
```

**Dependency Direction**: `feature:users:ui` depends on `feature:users:domain`, depends on `core:domain`. Never back-depend.

**Tool Support**: Android Gradle, CocoaPods (iOS), Swift Package Manager (iOS).

**Benefit**: Parallel feature development. Clear contract boundaries.

## Architectural Decision Framework

**Choose MVC/MVP if**: Prototyping, simple screens, legacy codebase.

**Choose MVVM if**: Medium complexity, data binding framework available (Jetpack, SwiftUI), good balance of simplicity and testability.

**Choose MVI if**: Complex state, debuggability required, time-travel testing needed, team comfortable with functional programming.

**Use Clean Architecture if**: Large codebase, domain logic is complex, multiple client platforms (web, mobile, backend).

**Add DI if**: 5+ dependencies per class or modular codebase.

**Modularize if**: Team size > 3 or build time > 30 seconds.

## See Also

- [Mobile Development Patterns](mobile-development-patterns.md) — platform considerations, UI frameworks
- [Android Development Patterns](mobile-android-patterns.md) — View system, Compose, Jetpack specifics
- [iOS Development Patterns](mobile-ios-patterns.md) — UIKit, SwiftUI, lifecycle details
- [Architecture Patterns](architecture-patterns.md) — layered, hexagonal, microservices (not mobile but foundational)
- [Patterns: Event-Driven](patterns-event-driven.md) — connects to MVI/intent flow