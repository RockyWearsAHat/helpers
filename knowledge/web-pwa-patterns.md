# Progressive Web Apps (PWA) Patterns — Manifest, Service Workers, Offline & Installation

## Web App Manifest

The manifest is a JSON file that declares app metadata and preferences. Browsers use it to understand installability, display mode, and presentation:

```json
{
  "name": "My App",
  "short_name": "App",
  "description": "A progressive web app",
  "start_url": "/",
  "scope": "/",
  "display": "standalone",
  "orientation": "portrait-primary",
  "theme_color": "#2196F3",
  "background_color": "#FFFFFF",
  "icons": [
    {
      "src": "/icon-192.png",
      "sizes": "192x192",
      "type": "image/png"
    },
    {
      "src": "/icon-512.png",
      "sizes": "512x512",
      "type": "image/png",
      "purpose": "any monochrome"
    }
  ],
  "categories": ["productivity", "utilities"],
  "screenshots": [
    {
      "src": "/screenshot1.png",
      "sizes": "540x720",
      "form_factor": "narrow"
    }
  ]
}
```

Link in HTML:

```html
<link rel="manifest" href="/manifest.json">
```

Key fields:
- **display**: `standalone` (app-like, no browser UI), `minimal-ui`, `fullscreen`, or `browser` (default)
- **start_url**: Page to load when app is launched
- **scope**: URLs under this path are considered part of the app (used for service worker scope and context)
- **theme_color**: Browser chrome color on Android
- **icons**: Sizes, purposes (`any`, `monochrome`, `maskable` for adaptive icons)
- **categories**: App classification for stores
- **screenshots**: Display in install prompts and app stores

## Service Worker Strategies

Service workers intercept network requests and manage caching. Different strategies suit different content types:

### Cache First (Offline Priority)

Check cache first; fall back to network. Ideal for assets (CSS, JS, images) that change infrequently:

```javascript
self.addEventListener('fetch', (event) => {
  event.respondWith(
    caches.match(event.request).then((response) => {
      return response || fetch(event.request).then((response) => {
        // Cache the response for next time
        caches.open('v1').then((cache) => cache.put(event.request, response.clone()));
        return response;
      });
    })
  );
});
```

**Pro**: Reliable offline access, fast load on repeat visits. **Con**: Stale content until cache expires.

### Network First (Online Priority)

Try network first; fall back to cache. Ideal for dynamic content (API responses, user data):

```javascript
self.addEventListener('fetch', (event) => {
  event.respondWith(
    fetch(event.request)
      .then((response) => {
        // Update cache after fetch
        caches.open('v1').then((cache) => cache.put(event.request, response.clone()));
        return response;
      })
      .catch(() => caches.match(event.request))
  );
});
```

**Pro**: Users always get fresh data when online. **Con**: Slow or offline users see stale data.

### Stale While Revalidate

Return cached response immediately; fetch fresh version in background. Ideal for semi-dynamic content:

```javascript
self.addEventListener('fetch', (event) => {
  event.respondWith(
    caches.open('v1').then((cache) => {
      return cache.match(event.request).then((response) => {
        const fetchPromise = fetch(event.request).then((networkResponse) => {
          cache.put(event.request, networkResponse.clone());
          return networkResponse;
        });
        return response || fetchPromise;
      });
    })
  );
});
```

**Pro**: Fast initial response, updates in background. **Con**: Initial response may be stale.

### Network Only

Never cache; always fetch. For sensitive operations (logout, payments):

```javascript
self.addEventListener('fetch', (event) => {
  event.respondWith(fetch(event.request));
});
```

Offline users get network errors; provide a fallback page.

### Cache Only

Serve only from cache; never fetch. For immutable assets with explicit cache busting:

```javascript
self.addEventListener('fetch', (event) => {
  event.respondWith(caches.match(event.request));
});
```

## Offline Patterns

### Progressive Enhancement

A page works offline if critical sections are cached. Less critical sections (comments, recommendations) gracefully degrade:

1. **Precache essential assets** on service worker installation
2. **Cache API responses** as users request them
3. **Detect offline state** via `navigator.onLine` or fetch failures
4. **Show indicator** (banner) when offline

```javascript
window.addEventListener('offline', () => {
  console.log('Offline');
  document.body.classList.add('offline');
});

window.addEventListener('online', () => {
  console.log('Restored connection');
  document.body.classList.remove('offline');
});
```

### Queued Mutations

When offline, queue mutations (form submissions, deletions) locally and sync when online:

```javascript
async function createPost(data) {
  if (navigator.onLine) {
    return fetch('/api/posts', { method: 'POST', body: JSON.stringify(data) });
  } else {
    // Queue for later
    const queue = await db.getQueue();
    queue.push({ action: 'createPost', data });
    return { id: 'pending-' + Date.now() };  // Optimistic ID
  }
}

// When online, sync queue
window.addEventListener('online', async () => {
  const queue = await db.getQueue();
  for (const item of queue) {
    try {
      await fetch(`/api/${item.action}`, { method: 'POST', body: JSON.stringify(item.data) });
      db.removeFromQueue(item.id);
    } catch {
      console.error('Sync failed', item);
    }
  }
});
```

## Push Notifications

Service workers can receive push events from a server and display notifications:

```javascript
// Register for push notifications
const registration = await navigator.serviceWorker.ready;
const subscription = await registration.pushManager.subscribe({
  userVisibleOnly: true,
  applicationServerKey: urlBase64ToUint8Array(publicKey),
});

// Send subscription to server (the server uses it to send notifications later)
await fetch('/subscribe', {
  method: 'POST',
  body: JSON.stringify(subscription),
});

// In service worker, receive push
self.addEventListener('push', (event) => {
  const data = event.data.json();
  self.registration.showNotification(data.title, {
    body: data.body,
    icon: '/icon.png',
    tag: 'notification',  // Replaces previous notification with same tag
  });
});

// Handle notification click
self.addEventListener('notificationclick', (event) => {
  clients.openWindow(event.notification.data.url);
});
```

Push notifications require:
- Service worker
- User permission
- HTTPS
- Subscription server (VAPID keys for authentication)

## App Shell Pattern

The app shell is a minimal skeleton (navigation, header, layout) cached at install time. Content loads dynamically:

```html
<!-- app-shell.html -->
<!DOCTYPE html>
<html>
  <head>
    <style>/* Critical CSS only */</style>
  </head>
  <body>
    <header>Navigation</header>
    <main id="content">Loading...</main>
    <script src="/app.js"></script>
  </body>
</html>
```

On first load:
1. Browser fetches HTML
2. Service worker caches it (if installed)
3. Subsequent visits load from cache instantly
4. Content (via fetch API) updates dynamically

The shell ensures the app feels fast; content fills in progressively.

## Background Sync

Background sync registers a sync event that fires when the device regains connectivity. Useful for retrying failed requests without user interaction:

```javascript
// In app
async function sendData(data) {
  try {
    await fetch('/api/data', { method: 'POST', body: JSON.stringify(data) });
  } catch {
    // Register sync tag
    const registration = await navigator.serviceWorker.ready;
    await registration.sync.register('sync-data');
  }
}

// In service worker
self.addEventListener('sync', (event) => {
  if (event.tag === 'sync-data') {
    event.waitUntil(
      // Retry logic here
      retryFailedRequests()
    );
  }
});
```

Background sync runs once when connectivity resumes; use periodic sync for repeated tasks.

## Periodic Background Sync

Similar to background sync, but runs on a schedule (e.g., every hour). Requires user permission and sufficient device battery:

```javascript
const registration = await navigator.serviceWorker.ready;
await registration.periodicSync.register('update-data', {
  minInterval: 24 * 60 * 60 * 1000,  // 24 hours
});

// In service worker
self.addEventListener('periodicsync', (event) => {
  if (event.tag === 'update-data') {
    event.waitUntil(fetchAndCacheData());
  }
});
```

Browsers may throttle periodic sync based on battery and usage patterns.

## Share Target

Share Target allows the app to receive shares from the device's share sheet:

```json
{
  "share_target": {
    "action": "/share",
    "method": "POST",
    "enctype": "mulipart/form-data",
    "params": {
      "title": "title",
      "text": "text",
      "url": "url",
      "files": [
        {
          "name": "file",
          "accept": ["image/*", "application/pdf"]
        }
      ]
    }
  }
}
```

When a user shares a photo or URL to your app, the browser POSTs to `/share` with the data.

## File Handling

File Handling allows the OS to open files (e.g., `.todo`, `.markdown`) with the PWA:

```json
{
  "file_handlers": [
    {
      "action": "/open-file",
      "accept": {
        "text/markdown": [".md", ".markdown"],
        "text/plain": [".txt"]
      },
      "icons": [
        {
          "src": "/icon.png",
          "sizes": "256x256",
          "type": "image/png"
        }
      ]
    }
  ]
}
```

The OS associates these MIME types and extensions with your app. Users can open files from the file manager or email, and your app launches with the file data.

## Install Prompt

Browsers (especially Chrome) show an install prompt automatically if installability criteria are met. Custom install prompts are possible:

```javascript
let deferredPrompt;

window.addEventListener('beforeinstallprompt', (event) => {
  event.preventDefault();  // Prevent automatic prompt
  deferredPrompt = event;  // Save for later
  showInstallButton();  // Show custom button
});

document.querySelector('#install-btn').addEventListener('click', async () => {
  if (deferredPrompt) {
    deferredPrompt.prompt();  // Show system prompt
    const userChoice = await deferredPrompt.userChoice;
    console.log(userChoice.outcome);  // 'accepted' or 'dismissed'
    deferredPrompt = null;
  }
});
```

Criteria for installability:
- Valid manifest (`name`, `icons`, etc.)
- HTTPS
- Service worker with fetch handler
- At least 192×192 icon

## See also

- [web-pwa.md](web-pwa.md) — PWA concepts, offline-first architecture
- [web-service-workers.md](web-service-workers.md) — Service worker lifecycle, caching, advanced patterns
- [mobile-offline-first.md](mobile-offline-first.md) — Offline-first databases, sync strategies