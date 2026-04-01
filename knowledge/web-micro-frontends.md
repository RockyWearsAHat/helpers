# Web Micro-Frontends — Module Federation, Composition & Deployment Independence

## Overview

Micro-frontends decompose a single-page application into independently built and deployed modules. Instead of a monolithic frontend, separate teams own distinct features or pages, each compiled as a container that exposes and consumes modules at runtime. This enables autonomous deployment, technology diversity, and organizational scaling at the cost of complexity in dependency management and cross-module communication.

## Build-Time vs. Runtime Integration

### Build-Time: Monorepo + Package Publishing

Micro-frontends can be built at compile time: each feature publishes a library (NPM package, versioned artifact), and the shell imports and re-exports components during its own build. Changes to a feature require the shell to rebuild and redeploy.

**Tradeoffs:**
- Simple dependency resolution (standard package managers)
- Longer deployment cycles (shell redeploy on feature changes)
- Shared dependency versions are known at build time
- No runtime isolation between features

### Runtime: Module Federation & Containers

**Module Federation** (Webpack 5+) creates a "container" entry point that exposes modules asynchronously. A host loads remote containers from URLs at runtime, discovering and initializing shared dependencies on demand. Each container acts as a separate build artifact that can be deployed independently.

**Mechanism:**

1. Remote container publishes a `remoteEntry.js` that exposes modules via `get()` method
2. Host's ModuleFederationPlugin references remotes by URL
3. Dynamic import of a remote module triggers async container loading
4. Shared scope negotiation: host and container agree on library versions
5. If a required version is unavailable, each uses its own bundled copy (causing duplication)

```javascript
// webpack.config.js (host)
new ModuleFederationPlugin({
  name: 'host',
  remotes: {
    app1: 'app1@http://localhost:3001/remoteEntry.js',
    app2: 'app2@http://localhost:3002/remoteEntry.js',
  },
  shared: ['react', 'react-dom'],
})

// webpack.config.js (remote)
new ModuleFederationPlugin({
  name: 'app1',
  exposes: {
    './Button': './src/Button.jsx',
    './Form': './src/Form.jsx',
  },
  shared: ['react', 'react-dom'],
})
```

**Tradeoffs:**
- Independent deployment (no host rebuild required)
- Runtime overhead (async loading, shared scope negotiation)
- Debugging complexity (multiple origins, async stack traces)

## Shared Dependencies & Version Negotiation

When multiple containers need the same library, duplication is wasteful. Module Federation defines shared modules: if the host already loaded React, a remote should use the host's instance rather than bundling its own.

**Eager vs. Lazy Sharing:**

- **Lazy (default):** Shared modules load when first requested. Requires an async boundary (not in synchronous entry point).
- **Eager:** Modules provided synchronously in the initial chunk. All versions are always downloaded, so recommended only for the host shell.

```javascript
shared: {
  react: {
    singleton: true,        // at most one instance at runtime
    strictVersion: false,    // accept similar versions
    requiredVersion: false,  // don't enforce version from package.json
    eager: false,            // async load by default
  },
}
```

**Version Resolution:**

- Relative paths (e.g., `./lib`) always provided, no version checking
- Package names (e.g., `react`) matched to `requiredVersion` in package.json
- Nested node_modules can expose different versions; module federation allows multiple if they don't conflict

## Routing & Layout Composition

Micro-frontends typically use one of two routing models:

### Shell Manages Routes

The shell routes to remote apps as large chunks:

```javascript
<Router>
  <Route path="/dashboard" component={Dashboard} />  // local
  <Route path="/admin/*" component={lazy(() => 
    import('admin/AdminApp')  // remote module
  )} />
</Router>
```

**Tradeoff:** The shell is tightly coupled to the URL space. Adding a new top-level route requires shell changes.

### Independent Micro-Apps (single-spa pattern)

Each micro-app registers its own routes and lifecycle:

```javascript
registerApplication({
  name: '@org/admin',
  app: System.import('@org/admin'),
  activeWhen: '/admin',
  customProps: { apiUrl: 'http://api.example.com' }
})
```

**Tradeoff:** Teams own their feature routing, but navigating between them requires higher-order coordination (typically via global router or message passing).

## Cross-App Communication

### Direct Method Calls (Anti-pattern)

Importing and calling functions between apps tightly couples them and defeats independence.

### Message-Based: Window Events, Custom Events

Apps emit and listen on `window` or broadcast channel:

```javascript
// App A
window.dispatchEvent(new CustomEvent('user:login', { detail: user }))

// App B
window.addEventListener('user:login', (e) => {
  console.log('User logged in:', e.detail)
})
```

**Tradeoff:** Simple but error-prone (event naming collisions, hard to debug). Works for loosely coupled flows (notifications, auth changes).

### Shared State Management

A central store (Redux, Jotai, Zustand) with root-level provider:

```javascript
// host
<StoreProvider store={globalStore}>
  <DashboardApp />
  <AdminApp />
</StoreProvider>
```

Each app dispatches and subscribes. **Tradeoff:** Tightly couples app contracts to store schema; breaks independence if store changes.

### Callback Props & Portals

Host passes functions and DOM targets to remotes:

```javascript
<RemoteApp onUserSelect={user => setSelected(user)} notificationPortal={ref} />
```

Works for parent-child flows; harder for sibling communication.

## Design System & Shared Components

Many teams extract a component library as its own container:

```javascript
// components-app remoteEntry.js
export Button from './Button.jsx'
export Form from './Form.jsx'
export Sidebar from './Sidebar.jsx'

// host & other apps
import { Button } from 'components/Button'
```

**Tradeoff:** Centralized governance (single design language), but changes to the library must be carefully versioned to avoid breaking consumers. Consider versioning shared components the same way npm packages are versioned.

## Deployment Independence

Each remote can be deployed on its own schedule:

1. Increment version of remoteEntry.js (or use a manifest)
2. Deploy to CDN
3. Host refreshes and loads new code on next page navigation (or refresh)
4. No host redeploy required (host can reach out to update metadata)

**Cache Busting:** Version remoteEntry.js or use query params:
```
http://localhost:3001/remoteEntry.js?v=1.2.3
```

**Blue-Green:** Maintain two remoteEntry endpoints and switch via load balancer or DNS.

## Pitfalls & Tradeoffs

- **Global Name Collisions:** Each build must have a unique `output.uniqueName` to avoid collision in global scope
- **Shared Scope Complexity:** Difficult to reason about at scale; easy to break with transitive dependencies
- **Testing:** Unit tests often pass; integration failures emerge at runtime when shared scopes don't match
- **Performance:** Each container adds a round trip (loading remoteEntry.js). Consider loading multiple remotes in parallel or bundling frequently-used modules eagerly
- **Debugging:** Stack traces cross boundaries; browser DevTools show minified container code without source maps by default

## Alternatives & Context

Module federation is specific to Webpack 5+. Other approaches:

- **Monorepo with strict boundaries** (Nx, Turborepo): Build-time isolation, single deployment
- **iframes:** Complete isolation but high overhead and communication complexity
- **Native Modules (Bit, Storybook 7+):** Component-level sharing without full application boundaries
- **Smart bundles at edge:** Serve different bundles per region/user-agent; no runtime federation needed

Module federation shines when teams are large, releases are frequent, and deployment speed outweighs architectural simplicity.