# Web Storage APIs — localStorage, IndexedDB, Cache API & OPFS

## Overview

Modern web applications store data client-side using multiple storage mechanisms, each with different trade-offs: localStorage/sessionStorage for small key-value data, IndexedDB for large structured data, Cache API for HTTP response caching, and Origin Private File System (OPFS) for file-like access. Understanding storage quotas, eviction policies, partitioning, and performance characteristics is essential for building resilient offline-capable applications.

---

## localStorage & sessionStorage

### LocalStorage

Persistent key-value storage scoped to origin (protocol + domain + port).

```javascript
// Store
localStorage.setItem('user_id', '12345');
localStorage.setItem('preferences', JSON.stringify({ theme: 'dark' }));

// Retrieve
const userId = localStorage.getItem('user_id');

// Remove
localStorage.removeItem('user_id');

// Clear all
localStorage.clear();
```

**Characteristics:**
- Survives browser restart
- ~5-10 MB per origin (browser-dependent; Chrome/Firefox typically 10 MB)
- Synchronous (blocks until complete; can cause jank on large writes)
- Per-origin isolation (same-origin policy enforced)

### SessionStorage

Same API as localStorage but cleared when tab closes.

```javascript
sessionStorage.setItem('temporary_state', JSON.stringify({ ...}));
```

**Use cases:**
- Tab-specific state (not persisted across tabs)
- Temporary data during wizard flows
- Reducing data reuse across sessions (privacy-forward)

### Key Limitations

1. **Synchronous blocking.** Large writes (1+ MB) can freeze UI.
2. **String-only.** Must JSON.stringify/parse objects.
3. **No query API.** Must iterate all keys to find data.
4. **No transactions.** Partial writes possible if quota exceeded mid-operation.

---

## IndexedDB

NoSQL database in browser supporting structured queries, indexes, transactions, and large data volumes.

### Basic Operations

```javascript
// Open or create database
const request = indexedDB.open('myapp', 1); // version 1

request.onupgradeneeded = (event) => {
  const db = event.target.result;
  
  // Create object store (like table)
  if (!db.objectStoreNames.contains('users')) {
    const store = db.createObjectStore('users', { keyPath: 'id' });
    store.createIndex('email', 'email', { unique: true });
  }
};

request.onsuccess = (event) => {
  const db = event.target.result;
  
  // Add data
  const tx = db.transaction(['users'], 'readwrite');
  const store = tx.objectStore('users');
  store.add({ id: 1, name: 'Alice', email: 'alice@example.com' });
  
  // Query by key
  store.get(1).onsuccess = (e) => console.log(e.target.result);
  
  // Query by index
  store.index('email').get('alice@example.com').onsuccess = (e) => {
    console.log(e.target.result); // { id: 1, name: 'Alice', ... }
  };
  
  // Range query
  const range = IDBKeyRange.bound(1, 10);
  store.getAll(range).onsuccess = (e) => console.log(e.target.result);
};
```

### Transactions

Atomic units of work. All-or-nothing semantics.

```javascript
const tx = db.transaction(['users', 'posts'], 'readwrite');
const userStore = tx.objectStore('users');
const postStore = tx.objectStore('posts');

userStore.put({ id: 1, name: 'Bob' });
postStore.add({ id: 100, userId: 1, text: 'Hello' });

tx.oncomplete = () => console.log('Both ops committed');
tx.onerror = () => console.log('Both rolled back on error');
```

### Cursors (Iteration)

Iterate over ranges of data.

```javascript
const tx = db.transaction(['users']);
const store = tx.objectStore('users');

store.openCursor().onsuccess = (event) => {
  const cursor = event.target.result;
  if (cursor) {
    console.log(cursor.key, cursor.value); // key, object
    cursor.continue(); // Fetch next
  }
};

// Range cursor
const range = IDBKeyRange.lowerBound(5);
store.openCursor(range).onsuccess = (event) => {
  // Cursor over entries with key >= 5
};
```

### Indexes

Query by non-key fields.

```javascript
const store = db.createObjectStore('users', { keyPath: 'id' });
store.createIndex('email_idx', 'email', { unique: true }); // Unique constraint
store.createIndex('age_idx', 'age', { unique: false }); // Allows duplicates

// Query
db.transaction('users').objectStore('users')
  .index('email_idx').get('alice@example.com').onsuccess = (e) => {
    console.log(e.target.result); // Fast lookup
  };

// Range on indexed field
const range = IDBKeyRange.bound(18, 65);
db.transaction('users').objectStore('users')
  .index('age_idx').getAll(range).onsuccess = (e) => {
    console.log(e.target.result); // All users age 18-65
  };
```

### Characteristics

- **Large capacity:** 50+ MB typical (browser-dependent; often tied to storage quota)
- **Asynchronous:** Non-blocking; events/promises
- **Structured data:** Objects, arrays, arbitrary types (IndexedDB serializes/deserializes)
- **Transactions:** ACID semantics
- **Indexes:** Query efficiency
- **Per-origin:** Same-origin policy enforced

---

## Storage Quota & Eviction

### Quota Management

```javascript
// Check available storage
navigator.storage.estimate().then(estimate => {
  const percentUsed = (estimate.usage / estimate.quota) * 100;
  console.log(`${percentUsed.toFixed(2)}% of ${estimate.quota} bytes used`);
});

// Request persistent storage (user approval)
navigator.storage.persist().then(isPersistent => {
  if (isPersistent) {
    console.log('Storage persisted; OS will not evict');
  } else {
    console.log('Storage volatile; may be evicted under pressure');
  }
});
```

### Quota Limits

Browser-dependent:
- **Chrome/Firefox:** ~50% of available disk space (up to ~2 GB typical; can be more on generous devices)
- **Safari:** Stricter; ~50 MB default (depending on settings)

### Eviction Policies

**Persistent storage:** Won't evict unless user explicitly clears or quota disputes force it.

**Volatile storage (default):** Browser may evict under memory/disk pressure using LRU (least-recently-used):
- Least active origins evicted first
- Can happen even during active session
- No predictability; data loss possible

**Request persistent mode for critical apps** (offline functionality, data-critical):

```javascript
if (navigator.storage && navigator.storage.persist) {
  navigator.storage.persist().then(isPersistent => {
    console.log(`Storage ${isPersistent ? 'persisted' : 'volatile'}`);
  });
} else {
  console.warn('Persistent storage not available');
}
```

---

## Cache API

HTTP response caching API designed for service workers and offline support.

### Basic Usage

```javascript
// Open or create cache
caches.open('my-cache-v1').then(cache => {
  // Cache responses from network
  cache.add('https://example.com/data.json'); // Fetch + cache
  
  // Or cache manually
  const response = new Response(JSON.stringify({ data: 'value' }), {
    headers: { 'Content-Type': 'application/json' }
  });
  cache.put('https://example.com/data.json', response);
  
  // Retrieve from cache
  cache.match('https://example.com/data.json').then(resp => {
    console.log(resp.json()); // { data: 'value' }
  });
});

// List all caches
caches.keys().then(names => {
  console.log(names); // ['my-cache-v1', 'my-cache-v2', ...]
});

// Delete old cache
caches.delete('my-cache-v1');
```

### Service Worker Integration

```javascript
// In service worker
self.addEventListener('fetch', (event) => {
  // Cache-first strategy
  event.respondWith(
    caches.match(event.request)
      .then(cached => {
        if (cached) return cached;
        return fetch(event.request).then(response => {
          caches.open('my-cache-v1').then(cache => {
            cache.put(event.request, response.clone());
          });
          return response;
        });
      })
      .catch(() => new Response('Offline'))
  );
});
```

### Characteristics

- **HTTP responses only:** Stores actual fetch responses with headers, status, body
- **Service worker-tied:** Managed by service worker lifecycle
- **Large capacity:** Tied to storage quota (same 50 MB–2 GB as IndexedDB)
- **Versioning:** Separate caches per version (versioning responsibility falls on developer)

---

## Origin Private File System (OPFS)

Filesystem API allowing file-like access to per-origin storage (not exposed to user filesystem, hence "private").

### Basic Access

```javascript
// Get file handle
const rootDir = await navigator.storage.getDirectory();

// Create or open file
const fileHandle = await rootDir.getFileHandle('data.txt', { create: true });

// Write
const writable = await fileHandle.createWritable();
await writable.write('Hello, world!');
await writable.close();

// Read
const file = await fileHandle.getFile();
const text = await file.text();
console.log(text); // 'Hello, world!'

// Delete
await rootDir.removeEntry('data.txt');
```

### Directory Operations

```javascript
const dir = await navigator.storage.getDirectory();

// Create subdirectory
const subdir = await dir.getDirectoryHandle('logs', { create: true });

// Iterate directory
for await (const entry of dir.values()) {
  console.log(entry.kind, entry.name); // 'directory|file', 'name'
}
```

### Use Cases

- **Large file handling:** Media processing, zipping, exporting datasets
- **Structured storage:** Multiple files organized in directories
- **Binary data:** WebAssembly modules, audio/video processing

### Characteristics

- **File-like interface:** More familiar for file operations than key-value
- **Async API:** Doesn't block main thread
- **Per-origin:** Isolated from other sites and user filesystem
- **Large files:** Can store large objects without serialization overhead
- **Browser support:** Modern browsers only (not universally available; check feature)

---

## Storage Partitioning

### Cookie and Storage Partitioning

Browsers are partitioning site-based storage by top-level site (Origin Private File System and Cache API included).

**Before:** Third-party iframe shares storage globally.
```
site-a.com embeds iframe from tracker.com
site-b.com embeds iframe from tracker.com
→ tracker.com has ONE shared storage (tracking possible)
```

**After:** Each top-level site gets separate partition.
```
site-a.com embeds iframe from tracker.com → tracker.com@site-a.com partition
site-b.com embeds iframe from tracker.com → tracker.com@site-b.com partition
→ partitions isolated (no cross-site tracking via storage)
```

**Implementation:** Safari (ITP), Firefox (Total Cookie Protection), Chrome (gradual rollout under Attribution Reporting & Privacy Sandbox).

---

## Performance Characteristics

| Storage | Capacity | Async | Latency | Use Case |
|---------|----------|-------|---------|----------|
| localStorage | ~10 MB | No | ~1-5 ms | Small settings, cached metadata |
| IndexedDB | 50+ MB | Yes | ~10-50 ms | Structured data, offline DBs |
| Cache API | 50+ MB | Yes | ~20-100 ms | HTTP responses, offline pages |
| OPFS | 50+ MB | Yes | ~10-50 ms | Large files, binary data |

**Guideline:**
- localStorage: <1 MB, simple strings
- IndexedDB: 1-50 MB, structured querying
- Cache API: HTTP responses
- OPFS: Large files (1+MB), binary data

---

## Best Practices

1. **Check storage availability.** Feature-detect before using; graceful degradation if unavailable.

2. **Use async APIs where possible.** localStorage blocking; prefer IndexedDB for large datasets.

3. **Handle quota exceeded errors.** Request persistent storage if data critical; implement eviction strategy.

4. **Version caches.** Use versioned cache names (e.g., cache-v1, cache-v2) for controlled updates.

5. **Monitor quota usage.** Call navigator.storage.estimate() periodically; warn users if approaching limit.

6. **Partition cross-origin data carefully.** Understand storage isolation (partitioning); don't rely on shared storage for embeds.

7. **Combine for redundancy.** Use Cache API for responses; IndexedDB for parse-once data; OPFS for files.

8. **Clear stale storage.** Remove old cache versions, outdated IndexedDB stores; reduces clutter.

---

## See Also
- [web-browser-security.md](web-browser-security.md) — Same-origin security model for storage
- [web-cookie-security.md](web-cookie-security.md) — Cookie storage + partitioning context
- [web-service-workers.md](web-service-workers.md) — Service worker lifecycle & Cache API integration