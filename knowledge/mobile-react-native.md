# React Native — Architecture, Performance & Ecosystem

## The Old Bridge Architecture

React Native historically used an **asynchronous JSON bridge** between JavaScript and native layers. The bridge serialized all cross-layer communication into JSON, creating three separate threads: the JavaScript thread, the native thread, and the bridge thread. This design was simple but had severe performance implications—every JS-to-native call had batching delays, serialization overhead, and no type safety. Large data transfers were slow; UI responsiveness suffered.

## The New Architecture (2024+)

The modern architecture replaces the bridge with three core pillars:

### 1. JSI (JavaScript Interface)

JSI is a **thin C++ API** that allows JavaScript to hold references to native objects directly, without serialization. Instead of JSON batching, functions are called synchronously with types preserved. This enables:

- Real-time data binding between JS and native
- Native views without intermediary serialization
- Synchronous method calls when needed
- Type-safe interoperability at the C++ level

### 2. Fabric

Fabric is a **new React rendering engine** that:

- Replaces the old UIManager with a direct C++ renderer
- Renders to native layers (UIView, Android View) synchronously
- Reduces layout passes and improves frame consistency
- Enables better concurrent rendering and priority management

Fabric runs on the same thread pool as the JavaScript thread, allowing React's layout calculations to complete without round-trips to the native side.

### 3. TurboModules

TurboModules are **lazy-loaded native modules** with:

- Compile-time code generation (Codegen) defining interfaces
- On-demand instantiation only when imported
- Type-safe method signatures
- No require-time initialization overhead

Unlike legacy native modules that loaded on app startup, TurboModules load when needed, reducing app startup time.

## Hermes Engine

Hermes is Meta's **JavaScript engine built for mobile**. Unlike JavaScriptCore:

- Compiles bytecode AOT before app distribution (faster startup)
- Optimized for low-memory devices
- Smaller bundle size (~70% reduction on disk)
- Predictable garbage collection and peak memory usage

Hermes is now the default runtime for React Native apps. It trades some JIT performance for startup speed and memory efficiency—the right trade-off for mobile.

## Navigation Patterns

**React Navigation** is the community standard. Navigation state is decoupled from component state via a navigation stack. Options:

- **Stack Navigator**: Native-like push/pop with animated transitions
- **Bottom Tabs**: Persistent tab buttons, children preserved between tabs (with screen options)
- **Drawer**: Slide-out menu, often combined with Stack
- **Native Stack**: Uses native navigation views (UINavigationController, Fragment backstack) for better feel

Deep linking requires configuring the linking configuration object to map URLs to route names and parameters. State persistence requires saving and restoring the navigation state on app close/resume.

## Expo Ecosystem

**Expo** is an abstraction layer over React Native:

- Managed cloud build service (EAS Build) for Android and iOS
- OTA update service (EAS Update) for pushing JS-only changes without Play Store/App Store review
- Pre-built native SDK (the Expo Go app reads managed Expo projects)
- SDK API: Camera, Notifications, Contacts, Location, Audio/Video, etc.—all managed with uniform APIs

Expo sits at a higher level than bare React Native. You can eject to bare React Native, but this requires managing native code. Most Expo projects use **Prebuild**, which generates native code from a config for EAS to build.

## OTA Updates

**EAS Update** serves new JavaScript bundles over the network:

- Updates deployed to a **channel** (e.g., production, staging)
- Apps can be configured to check for updates on launch
- Native code changes still require a new app store release; only JS/assets can be OTA-updated
- Updates are versioned and can be rolled back server-side

This enables rapid iteration without Play Store/App Store delays.

## Native Modules & FFI

React Native provides two paths to native code:

1. **Native Modules (TurboModules)**: Write Kotlin/Swift classes, expose methods to JS via Codegen. Works across the JSI bridge.
2. **React Native New Architecture (JSI)**: Direct C++ interop. Hermes supports JSI modules; native modules transition to JSI gradually.

For simple integrations, use community packages (e.g., react-native-camera). For complex logic, write TurboModules.

## Performance Optimization

### At the JS Level
- Use FlatList/FlashList with keyExtractor for long lists; avoid map() renders
- Memoize components (React.memo) to prevent unnecessary re-renders
- Use useCallback and useMemo sparingly—profiler-driven
- Defer non-critical work with InteractionManager.runAfterInteractions()

### Bridge Optimization
- Batch JS-to-native calls; avoid sending large objects frequently
- Use Hermes for better GC and startup time
- Lazy-load modules; avoid importing expensive code early

### Native Optimization
- Keep heavy computation on native threads; avoid blocking the JS thread
- Use native views where possible; avoid JavaScript animations for 60 FPS

### Memory
- Profile with Xcode (iOS) and Android Studio (Android)
- Watch for memory leaks in image caching and observers
- Unsubscribe from listeners in useEffect cleanup

## Comparison with Flutter

React Native and Flutter are the dominant cross-platform frameworks, with fundamentally different approaches:

| Aspect | React Native | Flutter |
|--------|--------------|---------|
| **Language** | JavaScript (Hermes engine) | Dart (compiled to native) |
| **UI Rendering** | Native views (Fabric) | Custom (Skia/Impeller) |
| **Bridge** | JSI (direct C++) | Platform channels (slower) |
| **Startup** | ~0.5s (Hermes) | ~0.2s (AOT compiled) |
| **Package Ecosystem** | npm (large, variable quality) | pub.dev (curated) |
| **Developer Experience** | Hot reload, wide tooling | Fast compilation, less fragmentation |
| **Performance** | Near-native after New Architecture | Consistent, predictable |

React Native excels for projects needing heavy JavaScript logic or existing web code. Flutter excels for performance-critical UIs and teams without JS experience.

## Common Pitfalls

- **Blocking the JS thread**: Heavy synchronous work freezes UI. Use native threads or WorkManager.
- **Not using a state management library**: Prop drilling and re-render cascades. Use Zustand, Redux, Jotai, or Recoil.
- **Mixing managed (Expo) and bare (custom native)**: Each has different tooling assumptions. Pick one strategy.
- **Over-using custom native modules**: Often a community package exists. Search first.
- **Not profiling**: Assumes React Native is slow. Profile with DevTools and native tools.

## See Also

- **flutter** — Alternative cross-platform framework with different architecture
- **architecture-patterns** — General architecture principles
- **language-javascript-typescript** — JavaScript fundamentals