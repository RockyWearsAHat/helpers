# Flutter — Dart Runtime, Rendering Pipeline & Mobile Development

## Dart Runtime & Compilation Strategy

Flutter uses **Dart**, a language optimized for UI development:

- **AOT (Ahead-of-Time) compiled** to native code on device (ARM, x86). No bytecode interpretation; machine code runs directly.
- **JIT (Just-In-Time)** available during development for hot reload (changes recompile in milliseconds).
- **Sound null safety** enforced at compile time; no null pointer exceptions at runtime.
- **Garbage collection**: Generational GC with pause times typically <20ms.

Dart's type system includes type inference and union types (sealed classes), making it safer than languages like Go while remaining concise.

## Widget Tree & Rendering Pipeline

Flutter's **declarative UI model** centers on the widget tree:

1. **Widget Tree**: Immutable descriptions of UI (like React components)
2. **Element Tree**: Stateful wrappers around widgets that manage lifecycle
3. **Render Tree**: Geometric layout objects (size, positioning)
4. **Layer Tree**: Painted visual layers

**Key difference from imperative frameworks**: You describe what the UI should look like, not how to mutate it. When state changes, Flutter diffs the widget tree and applies only necessary updates to the render tree.

### The Three Trees Pattern

```dart
// Widget tree - immutable, lightweight declarations
class Counter extends StatefulWidget {
  @override
  State<Counter> createState() => _CounterState();
}

class _CounterState extends State<Counter> {
  int count = 0;
  
  @override
  Widget build(BuildContext context) {
    // Rebuild called when setState() fires; returns a new widget tree
    return Column(children: [
      Text('$count'),
      ElevatedButton(
        onPressed: () => setState(() => count++),
        child: Text('Increment'),
      ),
    ]);
  }
}
// Element tree created/updated automatically; stores state between builds
// Render tree calculated from layout constraints
// Layer tree sent to graphics engine
```

## Rendering Engines: Skia vs Impeller

### Skia (Legacy, still supported)

Skia is a **2D graphics engine** used in Chrome and Android. It:

- Rasterizes all drawing calls (even text) into textures
- Is CPU-dependent for complex paths
- Has proven stability on all platforms
- Can be slower on low-end devices with heavy effects

### Impeller (Modern, default on iOS)

Impeller is Google's **next-generation renderer**:

- Uses **Vulkan** (Android) or **Metal** (iOS) for GPU-accelerated rendering
- Pre-records all draw calls into command buffers for predictable performance
- Eliminates frame stutters from GC or shader compilation
- Much faster on high-end devices; still rolling out on Android

Impeller is the future; Skia remains a fallback for compatibility.

## State Management Solutions

Choosing a state management library is the first architectural decision in any Flutter app.

### Provider

Provider is the **simplest and most accessible** option:

```dart
final counterProvider = StateNotifierProvider((ref) => CounterController());

class Counter extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final count = ref.watch(counterProvider);
    return Text('$count');
  }
}
```

Pros: Minimal boilerplate, small API surface, good for learning. Cons: Can lead to scattered state if misused.

### Riverpod

Riverpod is **Provider 2.0**—removes widget-tree dependency:

```dart
final counterNotifier = StateNotifierProvider<CounterNotifier, int>((ref) {
  return CounterNotifier();
});

// Riverpod providers are top-level functions, not inside widgets
// Enables better testability and composition
```

Pros: Compile-safe provider references (no typos), automatic dependency invalidation, works with code generation. Cons: Steeper learning curve.

### Bloc (BLoC Pattern)

BLoC (Business Logic Component) separates UI from business logic:

```dart
class CounterBloc extends Bloc<CounterEvent, int> {
  CounterBloc() : super(0) {
    on<IncrementEvent>((event, emit) => emit(state + 1));
  }
}

// UI layer
class Counter extends BlocBuilder<CounterBloc, int> {
  @override
  Widget build(BuildContext context, int state) {
    return Text('$state');
  }
}
```

Pros: Explicit event->state flow, testable, scales to large apps. Cons: Boilerplate for simple features.

### GetX

GetX is a **full framework** with state, routing, and dependency injection:

```dart
class Counter extends GetWidget<CounterController> {
  @override
  Widget build(BuildContext context) {
    return Obx(() => Text('${controller.count}'));
  }
}
```

Pros: All-in-one solution, very concise. Cons: Opinionated, large dependency, hides implicit dependencies.

**Recommendation**: Start with Provider or Riverpod for small-to-medium apps; move to BLoC as complexity grows.

## Platform Channels (Native Interop)

Flutter communicates with native code via **platform channels**:

```dart
// Dart side
const channel = MethodChannel('com.example.myapp/location');
final location = await channel.invokeMethod('getLocation');

// Kotlin side
val channel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "com.example.myapp/location")
channel.setMethodCallHandler { call, result ->
  when (call.method) {
    "getLocation" -> result.success(getCurrentLocation())
    else -> result.notImplemented()
  }
}
```

Platform channels use **method invocation** (not direct C++), so there's serialization overhead. For low-latency work (audio, graphics), write pure native code and call out sparingly.

## Package Ecosystem (pub.dev)

The Dart package ecosystem is smaller than npm but more curated:

- **Governance**: Package maintainers are encouraged to follow Effective Dart guidelines
- **Versioning**: Semantic versioning strictly enforced
- **Quality**: pub.dev rates packages on "likes," documentation, and test coverage
- **Popular packages**: GetX, Riverpod, Bloc, Freezed (code generation), Hive (local DB), Sqflite (SQLite wrapper)

The trade-off: fewer niche packages, but package quality is higher on average.

## Material vs Cupertino Design

Flutter provides two design systems:

- **Material** (Google): Familiar from Android, web. Default UI kit with buttons, cards, dialogs, etc.
- **Cupertino** (Apple): iOS native feel. Different interactions (swipe gestures, slide transitions).

Most apps use Material on both platforms; some use Cupertino on iOS only. Platform detection:

```dart
if (Platform.isIOS) {
  return CupertinoButton(...);
} else {
  return ElevatedButton(...);
}
```

## Flame Game Engine

For game development, **Flame** is an open-source 2D game engine built on Flutter:

```dart
class MyGame extends FlameGame {
  late Player player;
  
  @override
  Future<void> onLoad() async {
    player = Player();
    add(player);
  }
  
  @override
  void update(double dt) {
    player.update(dt); // dt = time since last frame
  }
}
```

Flame provides:

- Physics simulation (collision, gravity) via Forge2D
- Sprite and animation handling
- Input handling
- Particle effects

Flame games run on the same Impeller renderer as regular Flutter apps, yielding high frame rates.

## Desktop & Web Compilation

Flutter extends beyond mobile:

- **Desktop (Linux, macOS, Windows)**: Compile to native desktop apps; same codebase as mobile.
- **Web**: Compile to WebAssembly or JavaScript canvaskit renderer. Performance is good but not as smooth as native (canvas redraws every frame).
- **Limitations**: Platform-specific APIs (camera, sensors) require platform-channel workarounds on desktop/web.

## Common Trade-offs & Pitfalls

- **Rendering overhead**: Custom rendering (Skia/Impeller) uses more GPU than native widgets. Noticeable on low-end devices.
- **Large app size**: Dart runtime and engine bundled (~40MB base). Not suitable for ultra-minimal apps.
- **Platform expectations**: Users expect native feel (iOS gestures, Material animations). Designers must test on both.
- **Debugging**: Hot reload is fast, but full rebuild is slow. Profile with DevTools to catch performance regressions early.
- **Null safety adoption**: Early Flutter code used var/dynamic. New code uses full null safety; requires gradual migration.

## See Also

- **language-dart-flutter** — Dart language conventions and idioms
- **mobile-react-native** — Alternative cross-platform framework
- **gamedev-patterns** — General game development architecture (Flame is a specialized framework)