# Electron — Main/Renderer Process Model, IPC, and Desktop App Constraints

## Core Architecture: Process Model

Electron applications consist of a **main process** and one or more **renderer processes**, a design that exposes the architectural split inherent in desktop applications. The main process runs Node.js and has full OS access—file system, native APIs, system dialogs. Each renderer process is a Chromium instance running web content in isolation, with no direct OS access by default. This separation exists for both security and architectural clarity: the main process owns application lifecycle and system integration; renderers focus on UI.

## Main Process Responsibilities

The main process runs continuously and manages:

- **Application lifecycle**: `app.whenReady()`, `app.quit()`, `app.on('activate')` for macOS dock (Windows/Linux quit when all windows close)
- **Window creation**: `new BrowserWindow()` with options for size, position, frame type, transparency
- **Native integrations**: File dialogs (`dialog.showOpenDialog()`), native menus, system tray, notifications
- **Event handlers and background tasks**: Timers, network operations, background audio
- **State management**: Application-wide data accessible across renderer processes

A typical main process creates windows, listens for IPC messages, and delegates UI work to renderers. Blocking the main process impacts window responsiveness and makes the app feel sluggish.

## Renderer Process & Context Isolation

Each renderer process loads HTML/CSS/JavaScript from `window.loadFile()` or `window.loadURL()`. By default, a renderer has no Node.js access. **Context isolation** (enabled in `webPreferences: { contextIsolation: true }`) further restricts a renderer's environment: it runs in a separate V8 context, isolated from preload scripts. This prevents malicious web content from directly accessing Node.js APIs even if the renderer is compromised by JavaScript injection or XSS.

**Preload scripts** bridge this gap. They run before the renderer's main world loads and execute in a partially-privileged context with access to both Node.js and Electron APIs. They cannot directly modify the renderer's DOM or global scope; instead, they use `contextBridge` to explicitly expose selected APIs.

## Inter-Process Communication Patterns

IPC channels are **developer-defined names** through which main and renderer processes exchange JSON-serializable messages. Three primary patterns:

**Pattern 1: One-way (Send)**
```
Renderer: ipcRenderer.send('channel-name', data)
Main: ipcMain.on('channel-name', (event, data) => { /* handle */ })
```
Renderer initiates; main receives and handles. No return value. Useful for notifications or one-off actions like "change window title."

**Pattern 2: Two-way (Invoke/Handle)**
```
Renderer: const result = await ipcRenderer.invoke('channel', data)
Main: ipcMain.handle('channel', async (event, data) => { return result })
```
Renderer sends and waits for a response. Returns a Promise. Patterns for file dialogs, database queries, or any async main-process work. Errors thrown in a handler are serialized; only the message property reaches the renderer.

**Pattern 3: Main Initiated**
```
Main: webContents.send('channel', data)
Renderer: ipcRenderer.on('channel', (event, data) => { /* handle */ })
```
Main pushes notifications to one or many renderers (e.g., "download completed," "system theme changed"). 

**Security consideration**: IPC channels carry the full attack surface between worlds. Always validate and sanitize data. Preload scripts expose a curated API; do not expose raw `ipcRenderer.send` or `ipcRenderer.invoke` directly to renderer code.

## Window & Browser Management

`BrowserWindow` options control isolation and capabilities:

- **webPreferences**: `preload` path, `contextIsolation: true` (recommended for security), `nodeIntegration: false` (never set true), `sandbox: true`
- **sandbox**: If true, the renderer runs in OS-level sandbox (stricter isolation, performance cost)
- **CSP**: Content Security Policy via `<meta>` tag restricts what the renderer can load and execute
- **allowlist**: Electron v13+ supports per-window permission allowlists for native modules and file system access

`BrowserWindow` instances can create other windows, share state via the main process, or communicate via IPC. Closing all windows quits the app on Windows/Linux; on macOS, it typically keeps the app running (handle `app.on('window-all-closed')`).

## Native Module Integration

Electron apps load npm packages that use native Node.js modules (built with `node-gyp` or compiled C++ via Prebuild). Since Electron bundles its own Node.js version, native modules must be compiled for Electron's specific platform and architecture. Tools like `electron-rebuild` recompile modules after Electron install; some packages ship prebuilt binaries via `prebuild` or `node-pre-gyp`.

Mismatched architectures (compiling for Node 18 but targeting Electron 27) break `require()` at runtime with ABI errors.

## Auto-Updates & Distribution

Electron apps do not auto-update by default. Libraries like `electron-updater` integrate ASAR-based differential updates: the main process checks a remote server for a newer version, downloads a differential patch, and exits to allow the OS installer to apply the update. 

Distribution patterns:
- **Forge**: Meta-build tool simplifying packaging for Windows (MSI, NSIS, Squirrel), macOS (.dmg, .zip, notarization), Linux (.deb, .rpm). One config, cross-platform builds.
- **Builder**: Alternative with similar goals (package.json `build` config, supports many formats).
- **Manual**: Manually create installers and S3 distribution.

Signing and notarization (especially macOS) is critical for distribution; unsigned apps show "unidentified developer" warnings or are blocked outright. Windows apps benefit from code signing; Linux rarely requires it.

## Performance Characteristics

Startup time for a minimal Electron app is ~500ms–1s depending on hardware and included modules. Baseline app size with minimal dependencies is ~150MB compressed (Chromium dominates). Memory overhead is ~100MB per window (Chromium renderer process). For comparison, Tauri achieves smaller footprints by using native webviews; Electron trades size for consolidation (one Chromium, one Node.js runtime).

## Conceptual Tradeoffs vs. Tauri

Electron offers **full cross-platform consistency** — a given codebase runs identically on Windows, macOS, Linux — because Chromium and Node.js are platform-independent. This is powerful for teams without native expertise. Tauri uses native webviews (WebKit on macOS/iOS, WebView2 on Windows, GTK WebKit on Linux), reducing bundle size but introducing subtle platform differences in rendering and JavaScript APIs. Electron's strength is unified behavior; Tauri's is resource efficiency.

## See Also
- [desktop-application-patterns](desktop-application-patterns.md) — broader desktop architecture concepts
- [runtime-v8](runtime-v8.md) — JavaScript engine powering renderers
- [security-supply-chain](security-supply-chain.md) — native module and binary security implications