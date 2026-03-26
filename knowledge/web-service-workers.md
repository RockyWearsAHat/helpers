# Service Workers — Lifecycle, Caching Strategies & Offline-First Patterns

## Overview

Service workers are JavaScript worker threads running independently from web pages. They intercept network requests, enable offline functionality, and provide a foundation for Progressive Web Applications. A service worker operates as a proxy between your web app and the network, granting them power over caching, background sync, and push notifications — but also responsibility for not breaking the web experience.

Unlike regular web workers, service workers persist across page navigations and closures, persist upgrades, and inherit a global lifecycle managed by the browser.

## Lifecycle Events

### Installation Phase (`install` event)

The browser triggers `install` when a service worker is first encountered or when its code changes (byte-for-byte comparison of the registration URL). This is the moment to cache static assets or initialize resources.

**Key semantics:**
- The `install` event fires once per unique code body
- A service worker is "installing" while the event handler runs
- Calling `event.waitUntil(promise)` delays browser transition to the next phase until the promise settles
- If `waitUntil()` rejects, installation fails and the worker is discarded
- Multiple `install` handlers stack; each must resolve

**Common pattern:**
```javascript
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open('v1').then((cache) => {
      return cache.addAll(['/index.html', '/style.css', '/app.js']);
    })
  );
});
```

Mistakes to avoid: not waiting for cache operations (worker installing but cache operation incomplete), caching versioned assets without update strategy.

### Activation Phase (`activate` event)

The browser fires `activate` after a service worker successfully installs and when there is no active service worker (or the user navigates away from pages controlled by the old worker). This is cleanup time—delete old caches, migrate data.

**Key semantics:**
- `activate` is not guaranteed to fire on first install if pages are already loaded with the old worker
- Pages controlled by old workers must be closed or navigated away before new worker activates
- Calling `self.clients.claim()` forces current pages under the new worker's control immediately (use carefully; can cause incompatibilities if old and new worker differ significantly)
- Multiple handlers' promises are awaited

**Common pattern:**
```javascript
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((names) => {
      return Promise.all(
        names.map((name) => {
          if (name !== 'v1') return caches.delete(name);
        })
      );
    })
  );
});
```

### Fetch Event Interception

The `fetch` event fires for every HTTP request initiated by pages or workers under this service worker's scope. This is where caching and offline behavior live.

**Routing strategies:**

| Strategy | Pattern | Use Case |
|----------|---------|----------|
| **Cache-first** | Check cache first, fall back to network | Static assets: CSS, JS, images |
| **Network-first** | Try network, fall back to cache | API calls, frequently updated content |
| **Stale-while-revalidate** | Return cache immediately, silently refresh | Balance speed with freshness |
| **Cache-only** | Serve only from cache, never network | Offline-only URLs or forced offline mode |
| **Network-only** | Always hit network | Logout endpoints, live data requiring freshness |

**Example (cache-first):**
```javascript
self.addEventListener('fetch', (event) => {
  event.respondWith(
    caches.match(event.request).then((response) => {
      return response || fetch(event.request);
    })
  );
});
```

Subtle issue: cached responses are immutable. Cloning is necessary to read the body twice.

## Service Worker Scope and Lifecycle Span

### Scope Definition

A service worker's scope is the URL path range it controls. Registration at `/sw.js` defaults to scope `/`. Registration at `/api/sw.js` defaults to scope `/api/`.

```javascript
navigator.serviceWorker.register('/sw.js', { scope: '/app/' });
```

Only pages under the service worker's scope trigger its `fetch` events. A service worker at `/app/sw.js` with scope `/app/` does not intercept requests from `/admin/page.html`.

### Persistence and Version Pinning

Once registered, a service worker persists in the browser's storage. Updates occur when:
- The service worker file URL changes (different version string, file hash)
- The registration is explicitly updated via `registration.update()` or automatic 24-hour checks

During update, the browser downloads the file, byte-compares it, and if different, runs `install` for the new version. Crucially, **the old worker remains active controlling pages until all pages referencing the old worker are closed or reload.**

This creates a window where two versions coexist. Communicating changes to the client typically requires post-messages.

## Background Sync and Push Notifications

### Background Sync (Sync Event)

When a fetch fails offline, the service worker can queue it for later retry via the Background Sync API:

```javascript
self.addEventListener('sync', (event) => {
  if (event.tag === 'sync-posts') {
    event.waitUntil(
      fetch('/api/posts', { method: 'POST', body: queuedData })
        .catch(() => Promise.reject()) // Retry on fail
    );
  }
});
```

The browser guarantees retry attempts when connectivity returns.

### Push Notifications (Push Event)

A service worker receives `push` events from a push service:

```javascript
self.addEventListener('push', (event) => {
  const data = event.data.json();
  event.waitUntil(
    self.registration.showNotification('Title', { body: data.message })
  );
});
```

Requires user consent and HTTPS.

## Caching Best Practices and Tools

### Manual Cache API

The Cache API is low-level but explicit:

```javascript
caches.open('version-string').then((cache) => {
  cache.add('/url');  // Fetch and cache
  cache.addAll(['/url1', '/url2']);  // Multiple
  cache.put(request, response);  // Insert directly
});
```

Gotchas: failed responses are cached silently (opaque responses, redirects), no expiration built in.

### Workbox Library

Workbox abstracts common patterns. By precaching, routing, and expiring:

```javascript
import { precacheAndRoute } from 'workbox-precaching';
import { registerRoute } from 'workbox-routing';
import { CacheFirst, NetworkFirst } from 'workbox-strategies';

precacheAndRoute(self.__WB_MANIFEST);

registerRoute(
  ({request}) => request.destination === 'image',
  new CacheFirst({ cacheName: 'images' })
);
```

Workbox handles versioning, cleanup, and route matching.

## Update Flow and Version Management

Service workers don't auto-reload when code changes. The update flow requires:

1. Browser detects new code (registration update check, or explicit `registration.update()`)
2. New worker installs and waits
3. Old worker remains controlling pages
4. When pages reload or close, new worker activates
5. Use `postMessage` from client to trigger reload: `controller.postMessage({type: 'SKIP_WAITING'})`

```javascript
// In new service worker:
self.addEventListener('message', (event) => {
  if (event.data.type === 'SKIP_WAITING') {
    self.skipWaiting();
  }
});
```

## Offline-First Architecture Patterns

Offline-first means designing as if the network is unavailable first, then adding network optimization:

1. **Load from cache immediately** (instant perceived performance)
2. **Synchronize in background when online** (eventual consistency)
3. **Queue mutations offline, replay on sync** (conflict resolution needed)

This inverts traditional "network first" thinking and requires IndexedDB for state, not just Cache API.

## Common Pitfalls

- **Cache poisoning**: Caching error responses (404, 500) indefinitely
- **Stale assets**: Updated code deployed, but old cached versions live forever
- **Memory exhaustion**: Not implementing cache expiration or size limits
- **Scope confusion**: Registering from `/app/sw.js` but expecting global scope
- **Opaque response caching**: CORS responses may cache but not be readable
- **Message ordering**: Assuming `postMessage` ordering; browser may delay delivery

Service workers are powerful but demand careful lifecycle management and versioning discipline.