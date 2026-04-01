# Progressive Web Applications — Concepts & Offline-First Architecture

## The Progressive Enhancement Foundation

Progressive Web Applications represent an architectural approach where web applications acquire native-like capabilities incrementally. The core philosophy: start with a baseline web experience that works everywhere, then layer on enhanced capabilities where the platform supports them. A user on an older browser gets a functional site; a user on a modern browser gets installation, offline access, and push notifications — from the same codebase.

This contrasts with the "app or nothing" model where users must install a binary before any interaction occurs. PWAs invert this: engagement precedes installation, and installation itself is optional.

The term encompasses a collection of web platform APIs and architectural patterns rather than a single technology. No binary boundary exists between "a website" and "a PWA" — the progression is continuous.

## Installability — Conceptual Requirements

For a web application to present as installable, several conditions converge:

| Requirement          | Purpose                                    | Nuance                                                                                        |
| -------------------- | ------------------------------------------ | --------------------------------------------------------------------------------------------- |
| Web app manifest     | Declares app metadata, icons, display mode | A JSON file linking identity to presentation preferences                                      |
| Service worker       | Provides offline capability signal         | Must be registered; the browser uses its presence as a heuristic for app-readiness            |
| HTTPS                | Secure context for service worker APIs     | Localhost exempted during development                                                         |
| Engagement heuristic | Browser-determined threshold               | Varies by browser — some require repeated visits, others install on first visit with manifest |

The manifest describes how the app should appear when launched from the home screen:

```json
{
  "name": "Application Name",
  "short_name": "App",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#ffffff",
  "theme_color": "#3367D6",
  "icons": [{ "src": "/icon-192.png", "sizes": "192x192", "type": "image/png" }]
}
```

Display modes create a spectrum of chrome visibility:

| Mode         | Browser Chrome              | Use Case Context                    |
| ------------ | --------------------------- | ----------------------------------- |
| `browser`    | Full browser UI             | Standard web experience             |
| `minimal-ui` | Reduced chrome, back button | Apps needing navigation affordances |
| `standalone` | No browser chrome           | Most PWA installations              |
| `fullscreen` | No system UI                | Immersive experiences, games        |

The install prompt remains one of the more debated UX challenges. Browsers surface installation at their own discretion, and the timing often misaligns with user intent. Applications can intercept the prompt event and defer it to a contextually appropriate moment, but cannot trigger it unprompted — maintaining user agency at the cost of discoverability.

## Service Worker Lifecycle

Service workers operate as a programmable network proxy — a script that sits between the application and the network, intercepting every fetch request the page makes. They run in a separate thread with no DOM access, communicating with pages through message passing.

The lifecycle proceeds through distinct phases:

```
Registration → Installation → Waiting → Activation → Idle/Fetch handling → Termination
```

**Registration**: The page requests a service worker be associated with a scope (a URL path prefix). The browser downloads the script and compares it byte-for-byte against any existing registration.

**Installation**: On first registration or when the script changes, the `install` event fires. This is the conventional moment to pre-cache critical resources — the "app shell." If any cache operation fails during installation, the entire installation fails, ensuring atomic deployment.

**Waiting**: A newly installed service worker does not immediately take control. If an older version is active and pages are still open under it, the new worker waits. This prevents mid-session behavioral changes. The waiting phase can be bypassed (via `skipWaiting()`), but doing so mid-session risks serving responses from a new cache version that conflict with already-loaded page assets.

**Activation**: When no pages are controlled by the previous worker, the new one activates. The `activate` event is conventionally used to clean up old caches — removing cache entries from previous versions that are no longer needed.

**Fetch handling**: Once active, the worker intercepts network requests within its scope. Each request can be handled with full programmatic control — served from cache, forwarded to the network, synthesized entirely, or any combination.

**Termination**: The browser may terminate idle service workers at any time to conserve resources. Workers must not rely on persistent in-memory state between events. Any state that must survive termination belongs in IndexedDB or the Cache API.

### The Update Problem

Service worker updates create a version coordination challenge. The new worker installs while old pages still run under the previous version. Strategies for managing this transition:

- **Lazy activation**: Let the waiting phase complete naturally. Safe but slow — users may run stale code until all tabs close.
- **Skip waiting with reload**: Activate immediately, then signal all controlled pages to reload. Disruptive but ensures consistency.
- **Versioned URL paths**: Route through version-prefixed paths so old and new workers serve non-conflicting resources. Complex but smooth.

No strategy is universally correct. The choice depends on how tolerant the application is of mixed-version states.

## Caching Strategies

The Cache API provides a key-value store keyed on Request objects, returning Response objects. Combined with service worker fetch interception, it enables several caching strategies:

### Cache-First (Cache Falling Back to Network)

```
Request → Check cache → If hit: return cached → If miss: fetch from network → Cache response → Return
```

Suited for resources that change infrequently: fonts, images, versioned asset bundles. Provides the fastest response time for cached content. The trade-off: staleness. Cached responses persist until explicitly evicted. If the cache is never invalidated, users may see outdated content indefinitely.

### Network-First (Network Falling Back to Cache)

```
Request → Attempt network fetch → If success: cache and return → If failure: return cached version
```

Suited for content where freshness matters: API responses, news feeds, user-generated content. Provides current data when online and graceful degradation when offline. The trade-off: latency. Every request pays the network roundtrip cost, with cache serving only as a fallback. On slow connections, perceived performance suffers compared to cache-first.

### Stale-While-Revalidate

```
Request → Return cached version immediately → Simultaneously fetch from network → Update cache with fresh response
```

Suited for resources where slight staleness is acceptable but performance matters: profile images, semi-dynamic content, configuration. Provides near-instant responses while keeping the cache fresh for next time. The trade-off: the user sees the _previous_ version until the next request. For rapidly changing content, this one-request lag may be unacceptable.

### Network-Only and Cache-Only

These represent the extremes. Network-only bypasses caching entirely — appropriate for non-cacheable resources like analytics pings. Cache-only never checks the network — appropriate for resources populated during the install phase that should never change.

### Strategy Selection Considerations

| Factor                | Cache-First Favors | Network-First Favors | SWR Favors        |
| --------------------- | ------------------ | -------------------- | ----------------- |
| Change frequency      | Low                | High                 | Medium            |
| Latency sensitivity   | High               | Low                  | High              |
| Freshness requirement | Low                | High                 | Medium            |
| Offline importance    | High               | Medium               | Medium            |
| Bandwidth cost        | Reduces            | Does not reduce      | Partially reduces |

In practice, applications mix strategies by resource type: cache-first for static assets, network-first for API calls, stale-while-revalidate for user avatars.

## Offline-First vs Online-First Architecture

These represent fundamentally different assumptions about connectivity:

**Online-first**: The application assumes network availability. Offline support is a fallback — cached versions of pages, error screens when the network is unavailable. The architecture centers on server-rendered or server-fetched content with caching as a performance and resilience layer.

**Offline-first**: The application assumes intermittent or absent connectivity. Local data is the primary source of truth, synchronized with the server when possible. The architecture centers on local storage with synchronization as a background concern.

The offline-first approach introduces substantial complexity:

- **Conflict resolution**: When multiple clients modify the same data offline, merging changes on reconnection requires conflict strategies — last-write-wins, operational transforms, CRDTs, or manual resolution.
- **Storage management**: Local data stores must handle the full operational dataset, not just a cache. IndexedDB becomes a primary database, not an auxiliary store.
- **Sync orchestration**: Background sync must handle partial failures, retry logic, and ordering constraints. The Background Sync API provides a browser-managed retry mechanism, but the application must define what "sync" means for its data model.
- **UI state divergence**: The local state may diverge from server state for extended periods. The UI must communicate sync status without alarming users or hiding important conflicts.

Most applications land between the extremes. A document editor benefits from offline-first thinking. A real-time stock ticker has little use for it. A messaging app might store message history offline-first but treat presence indicators as online-only.

## Background Sync and Push Notifications

### Background Sync

The Background Sync API allows deferred network operations. When a user takes an action offline (submitting a form, posting a message), the application registers a sync event. The browser fires this event when connectivity returns — even if the user has left the page.

```javascript
// Page registers a sync
navigator.serviceWorker.ready.then((reg) => {
  reg.sync.register("outbox-sync");
});

// Service worker handles the sync event
self.addEventListener("sync", (event) => {
  if (event.tag === "outbox-sync") {
    event.waitUntil(flushOutbox());
  }
});
```

The sync event fires with best-effort timing — the browser batches syncs to optimize battery and network usage. Periodic Background Sync extends this to recurring tasks (refreshing content on a schedule), though browser support and permission gating vary significantly.

### Push Notifications

Push notifications involve a three-party protocol: the application server, a push service (operated by the browser vendor), and the service worker. The application server sends a push message to the push service, which delivers it to the browser, which wakes the service worker to handle it.

The permission model is a significant UX consideration. Users have developed "notification fatigue" — prompting for notification permission on first visit typically yields low acceptance rates. Contextual permission requests (after a user-initiated action that implies interest) tend to perform substantially better.

Push without visible notification is restricted in most browsers — the service worker must show a notification or the browser may revoke push permission. This prevents silent background tracking under the guise of push.

## The App Shell Architecture

The app shell pattern separates an application into two parts:

- **Shell**: The minimal HTML, CSS, and JavaScript needed to render the UI chrome — navigation, layout, loading states. Cached aggressively during service worker installation.
- **Content**: Dynamic data loaded into the shell at runtime, from cache or network.

```
First visit:  Network → Cache shell + content
Return visit: Cache → Instant shell render → Fetch fresh content
```

This architecture optimizes perceived performance. The shell renders immediately from cache, giving users a responsive frame while content loads. It mirrors the native app experience where the app UI appears instantly and data populates progressively.

Trade-offs:

- Shell updates require service worker updates — the traditional web model of "deploy and it's live" becomes mediated by the service worker lifecycle
- The shell must be designed to look reasonable before content loads — skeleton screens, loading indicators
- Server-side rendering and app shell can conflict: SSR wants to deliver content-populated HTML, while app shell wants to deliver a content-free frame

## PWA vs Native — The Trade-off Space

| Dimension             | PWA Strengths                     | Native Strengths                                        |
| --------------------- | --------------------------------- | ------------------------------------------------------- |
| Distribution          | URL-based, no app store required  | App store discovery, curation                           |
| Installation friction | Zero to minimal                   | Download, install, storage commitment                   |
| Update deployment     | Immediate via service worker      | Store review, user-initiated update                     |
| Platform API access   | Growing but constrained           | Full hardware/OS API access                             |
| Performance ceiling   | Bounded by browser runtime        | Direct hardware access, AOT compilation                 |
| Offline capability    | Service worker caching            | Full local runtime                                      |
| Cross-platform        | Single codebase by definition     | Per-platform development (or cross-platform frameworks) |
| Monetization          | Web payment APIs, no platform fee | In-app purchases, store billing                         |
| Discoverability       | Search engines, URLs, links       | App store search, editorial features                    |

Areas where the boundary blurs over time: file system access, Bluetooth, USB, NFC, and biometric authentication have all gained web API counterparts with varying browser support. The trend is toward convergence, though the gap in hardware-intensive domains (AR/VR, advanced GPU compute, background processing) remains significant.

Platform vendors have mixed incentives regarding PWA capability. Full PWA feature parity would reduce app store relevance, creating tension between web openness and platform ecosystem control.

## Storage APIs and Quota

PWAs interact with several storage mechanisms:

| API                        | Purpose                         | Characteristics                                         |
| -------------------------- | ------------------------------- | ------------------------------------------------------- |
| Cache API                  | Request/Response pairs          | Service-worker-accessible, persistent                   |
| IndexedDB                  | Structured data, large datasets | Transactional, indexed, asynchronous                    |
| localStorage               | Small key-value data            | Synchronous, 5-10MB typical limit, blocks main thread   |
| Origin Private File System | File-like storage               | File system semantics without actual disk file exposure |

Storage quota varies by browser and platform. The Storage API's `navigator.storage.estimate()` provides approximate quota and usage, but actual limits depend on disk space, browser policy, and whether the origin has been granted persistent storage.

**Persistent vs best-effort storage**: By default, browsers may evict stored data under storage pressure (low disk space). The `navigator.storage.persist()` API requests that the browser treat the origin's storage as persistent. Granting is discretionary — some browsers auto-grant for installed PWAs or frequently visited sites; others prompt the user.

**Storage eviction**: Under the best-effort model, browsers evict origins following LRU (least recently used) policies. An infrequently visited PWA with large cached data is a prime eviction candidate. Applications handling important user data should request persistence and implement export/backup mechanisms as insurance.

## The Convergence Boundary

PWAs occupy a design space between web pages and native applications. This positioning creates both their value and their limitations:

**What convergence enables**: A single codebase reaches all platforms with native-like UX. Users engage before committing to installation. Developers deploy instantly without store mediation. Deep linking works because URLs are the addressing mechanism.

**What convergence constrains**: The security sandbox limits hardware access. Background execution is restricted to prevent resource abuse. The permission model gates capabilities behind user consent, reducing seamless integration. Platform-specific UI conventions (navigation patterns, gesture handling, notification grouping) may not map cleanly to web primitives.

**The uncanny valley risk**: A PWA that almost-but-not-quite matches native behavior can feel more broken than a web app that embraces its web nature. Pursuit of native fidelity can backfire when the platform abstraction leaks — a subtle scroll physics difference, a missing system integration, an unexpected navigation behavior.

The most effective PWA architectures tend to lean into web strengths (linkability, progressive disclosure, instant access) rather than attempting pixel-perfect native mimicry. The goal is not to be indistinguishable from native, but to deliver value through the unique properties the web platform provides.

## Architectural Decision Framework

When evaluating PWA architecture for a given application:

**Connectivity assumptions**: What percentage of usage occurs offline or on unreliable connections? Offline-first adds complexity that serves no purpose for always-connected kiosk applications.

**Data sensitivity**: Offline storage for sensitive data introduces client-side security considerations that server-only architectures avoid.

**Update frequency**: Applications requiring instant global updates may find the service worker cache mediation layer introduces unwanted latency in propagating changes.

**Platform integration depth**: Applications requiring deep OS integration (custom keyboards, background location tracking, inter-app communication) may find web APIs insufficient.

**User expectation**: Users installing from an app store expect native behavior conventions. Users accessing via URL expect web behavior. PWAs must navigate between these expectation sets.

The PWA approach tends to serve well for content-driven applications, utilities with moderate complexity, and any scenario where reducing installation friction directly impacts business metrics. It tends to serve less well for performance-critical applications, deep platform integrations, and contexts where app store presence is a distribution requirement.
