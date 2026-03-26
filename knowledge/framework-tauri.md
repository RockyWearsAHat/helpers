# Tauri — Rust Backend, Webview Frontend, Security-First Desktop Apps

## Core Value: Lightweight System Integration

Tauri bundles a **Rust runtime** with a **native webview** (WebKit, WebView2, GTK) into a minimal package (600KB–5MB vs. Electron's ~150MB). The architecture inverts Electron's model: instead of embedding a JavaScript runtime to call native code, Tauri embeds a webview to call Rust code. This prioritizes **system security by default** and drastically reduces resource consumption, particularly on Linux and macOS.

The frontend is pure web tech (React, Vue, Svelte—framework-agnostic). The backend is Rust with full OS access. Communication flows through a bridge: frontend sends messages to Rust, Rust responds via IPC built on `tauri::invoke()`.

## Command Invocation: The Core IPC Pattern

Rust functions decorated with `#\[tauri::command\]` become synchronous JavaScript APIs. The frontend calls them via `invoke()`:

```rust
// src-tauri/src/lib.rs
#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
  std::fs::read_to_string(path)
    .map_err(|e| e.to_string())
}
```

```javascript
// src/index.js
const content = await invoke('read_file', { path: '/tmp/file.txt' });
```

Commands are **explicitly declared** in `src-tauri/tauri.conf.json` under `allowlist.core.invoke` (though by Tauri 2.0 this is often inferred). Each command is a distinct, type-safe entry point with no wildcard access. This "explicit is secure" philosophy contrasts sharply with Electron's contextBridge model where developers must carefully curate what to expose.

## State Management: Typed Application State

Tauri provides a **global, thread-safe state object** initialized at startup:

```rust
#[derive(Clone)]
pub struct AppState {
  counter: i32,
}

// In main()
let state = AppState { counter: 0 };
tauri::Builder::default()
  .manage(state)
  .build()
  .run();
```

Commands access state via `State<T>` parameter:

```rust
#[tauri::command]
fn increment(mut state: State<AppState>) {
  state.counter += 1;
}
```

Tauri's state is **immutable by default** (borrowed references) or mutable via `Mutex<T>`. This forces explicit synchronization and makes data flow visible in type signatures, unlike JavaScript's implicit global scope pollution.

## Plugins & Extensibility

Tauri 2.0 introduced a **plugin system** (replacing ad-hoc capability flags). Official plugins handle common tasks: file system dialogs, native notifications, deep linking, HTTP client. Custom plugins are Rust crates exporting a `TauriPlugin<R>` implementing hooks for setup, window creation, IPC command registration.

Plugins compose: a Tauri app can use official plugins (dialog, clipboard, os) alongside custom ones for domain-specific logic. Each plugin opts into capabilities—declarative and inspectable.

## Security Model: Allowlist & Content Security Policy

**Allowlist** is Tauri's primary security mechanism. A `src-tauri/tauri.conf.json` allowlist declares what the frontend can do:

```json
{
  "allowlist": {
    "core": {
      "window": ["close", "setTitle"],
      "dialog": ["open", "save"]
    },
    "fs": {
      "readFile": ["$DOWNLOAD/*", "$DESKTOP/*"]
    },
    "shell": {
      "open": ["https://*"]
    }
  }
}
```

- **Window commands**: which BrowserWindow methods are accessible
- **File system scopes**: which directories the frontend can read/write; **glob patterns** are enforced at runtime
- **Shell execution**: which protocols `shell.open()` (open URL via system browser) permits
- **Network**: restricted to declared domains for `fetch()`

**Content Security Policy (CSP)** further hardens the frontend. A restrictive CSP bans inline scripts, only allows local scripts, and restricts fetch to allowlisted origins. Misconfigured CSP is common and silently breaks apps; errors appear in browser console.

This multi-layer approach—allowlist + CSP + Rust type safety on the backend—shifts security burden away from developer discipline (as in Electron's preload scripts) toward declarative constraints enforced by the framework.

## Mobile: Tauri 2.0 & Beyond

Tauri 2.0 introduced **first-class mobile support** (iOS, Android via Kotlin/Swift glue). The same Rust backend and web frontend compile to mobile platforms. However, **mobile is newer and less battle-tested** than desktop. Platform-specific APIs require platform-specific Rust plugins. The promise is code reuse; the reality is platform differences still require conditional logic.

## Lifecycle & Plugins vs. CLI Commands

Tauri apps start via `tauri::Builder::default().invoke_handler(...).run()`. Lifecycle events fire:
- `ready`: app ready, main window created
- `window_created`: new window opened
- `window_close`: window closing
- `before_close`: intercept close (confirm unsaved changes)

Plugins can hook these events. CLI commands (`tauri command --some-flag`) provide a declarative interface to Rust functions for scripting or background tasks; less common than web frontend, but useful for automation.

## Comparison to Electron

| Aspect | Tauri | Electron |
|--------|-------|----------|
| **Bundle size** | 5–20MB | 150–200MB |
| **Memory per app** | 30–50MB | 100–150MB |
| **Webview** | Native (OS webview) | Chromium |
| **IPC verbosity** | Explicit allowlist + commands | Preload + contextBridge |
| **Security posture** | Deny-by-default with allowlist | Trust-based with explicit exposure |
| **Cross-platform consistency** | Subtle rendering differences | Identical rendering |
| **Ecosystem maturity** | Growing, fewer community packages | Large, well-established |
| **Rust knowledge required** | Yes | No |
| **Platform-specific quirks** | WebKit vs. WebView2 CSS, JavaScript API differences | Minimal—Chromium everywhere |

## Ecosystem & Adoption

Tauri is the emerging alternative for developers prioritizing security and resource efficiency, especially teams comfortable with Rust. Web frameworkswork unchanged; the main cost is learning Rust semantics and the plugin ecosystem. Community plugins are smaller than Electron's npm ecosystem, but core functionality (file dialogs, notifications, deep linking) is covered by official plugins.

## See Also
- [framework-electron](framework-electron.md) — alternative desktop framework with different tradeoffs
- [language-rust](language-rust.md) — Rust type system and safety model
- [security-supply-chain](security-supply-chain.md) — Rust dependency security and crate auditing
- [desktop-application-patterns](desktop-application-patterns.md) — general desktop architecture concepts