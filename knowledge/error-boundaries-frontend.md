# Frontend Error Boundaries — React, Vue, Svelte & Global Error Handlers

## Overview

Frontend error handling operates across multiple layers: component-level boundaries catch rendering errors locally, global handlers catch unhandled promise rejections and runtime errors, and error reporting services aggregate failures for monitoring. Each framework offers different mechanisms, from React's class-component Error Boundaries to Vue's `errorHandler` hook to Svelte's `handleError` and `+error.svelte` components.

---

## React Error Boundaries

Error Boundaries are class components that catch JavaScript errors anywhere in the child component tree, log those errors, and display a fallback UI instead of crashing the component tree.

### componentDidCatch and getDerivedStateFromError

Error Boundaries require **class components** (hooks don't support error boundary logic yet).

```jsx
class ErrorBoundary extends React.Component {
  constructor(props) {
    super(props);
    this.state = { hasError: false, errorMessage: '' };
  }

  static getDerivedStateFromError(error) {
    // Called during render phase; return state update
    return { hasError: true, errorMessage: error.message };
  }

  componentDidCatch(error, errorInfo) {
    // Called during commit phase; safe for side effects
    console.error('Error caught:', error);
    console.error('Error info:', errorInfo.componentStack);
    
    // Send to error reporting service
    reportErrorToServer(error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return <h1>Something went wrong: {this.state.errorMessage}</h1>;
    }
    return this.props.children;
  }
}

// Usage
<ErrorBoundary>
  <UserProfile userId={42} />
</ErrorBoundary>
```

**Key distinction:**
- **getDerivedStateFromError** runs during render (pure, no side effects). Use to update state so fallback UI renders.
- **componentDidCatch** runs after render commits (safe for side effects). Use to log, send telemetry, or retry.

### What Error Boundaries DON'T Catch

Errors during:
- Event handlers (use try/catch in handlers or window.onerror)
- Async callbacks (use try/catch or .catch() chains)
- Server-side rendering
- Errors in the boundary itself

### Error Boundaries in Hook-Based Code

Since `useErrorBoundary` doesn't exist in core React yet, use libraries like **react-error-boundary** (by Dan Abramov):

```jsx
import { ErrorBoundary } from 'react-error-boundary';

function ErrorFallback({error, resetErrorBoundary}) {
  return (
    <div>
      <h1>Oops! Something went wrong.</h1>
      <pre>{error.message}</pre>
      <button onClick={resetErrorBoundary}>Try again</button>
    </div>
  )
}

<ErrorBoundary FallbackComponent={ErrorFallback} onError={logErrorToService}>
  <UserProfile />
</ErrorBoundary>
```

---

## Vue Error Handling

Vue 3 provides `app.config.errorHandler` for global errors and component-scoped error handling.

### app.config.errorHandler

```javascript
const app = createApp(App);

app.config.errorHandler = (err, instance, info) => {
  // Log to error reporting service
  reportErrorToServer(err, { 
    context: info, 
    component: instance?.$options?.name 
  });
  
  // Show toast/notification to user
  showErrorNotification('Something went wrong');
};

app.mount('#app');
```

The handler receives:
- `err`: The error object
- `instance`: The Vue component instance
- `info`: String describing where the error occurred

### Component-Level Error Handling

```vue
<script>
export default {
  methods: {
    async fetchData() {
      try {
        this.data = await api.get('/data');
      } catch (e) {
        this.error = e.message;
        reportError(e);
      }
    }
  }
}
</script>

<template>
  <div v-if="error" class="alert alert-danger">{{ error }}</div>
  <div v-else>{{ data }}</div>
</template>
```

### Global window.onerror and unhandledrejection

Fallback for errors outside Vue's error handling (scripts, promises, external libraries):

```javascript
window.addEventListener('error', (event) => {
  console.error('Global error:', event.error);
  reportErrorToServer(event.error, { type: 'uncaught' });
});

window.addEventListener('unhandledrejection', (event) => {
  console.error('Unhandled promise rejection:', event.reason);
  reportErrorToServer(event.reason, { type: 'unhandled_rejection' });
});
```

---

## Svelte Error Handling

Svelte (especially SvelteKit) offers multiple layers.

### +error.svelte (Route-Level Boundaries)

SvelteKit's error component catches errors from `load` functions and renders instead:

```svelte
<!-- routes/posts/[id]/+page.svelte -->
<script>
  export let data;
</script>

{#if data.posts}
  <h1>{data.posts.title}</h1>
{/if}

<!-- routes/posts/[id]/+error.svelte -->
<script>
  import { page } from '$app/stores';
</script>

<div class="error">
  <h1>Error {$page.status}</h1>
  <p>{$page.error?.message}</p>
</div>
```

When `load` throws:

```javascript
// routes/posts/[id]/+page.js
export async function load({ params }) {
  const post = await fetch(`/api/posts/${params.id}`);
  if (!post.ok) {
    throw error(404, 'Post not found');
  }
  return { post: await post.json() };
}
```

The `+error.svelte` component renders with `$page.error` available.

### handleError Hook

Server-side error interception and transformation:

```javascript
// src/hooks.server.js
export async function handleError({ error, event }) {
  // Log to service
  console.error('Server error:', error);
  
  // Return sanitized error to client (don't leak internals)
  return {
    message: 'Internal server error',
    code: error.code || 'ERROR'
  };
}
```

### Client-side window.onerror / unhandledrejection

For errors outside SvelteKit's framework:

```javascript
// src/routes/+layout.svelte
<script>
  import { onMount } from 'svelte';
  
  onMount(() => {
    window.addEventListener('error', (e) => {
      console.error('Uncaught error:', e.error);
      // send to Sentry/Datadog
    });
    
    window.addEventListener('unhandledrejection', (e) => {
      console.error('Unhandled rejection:', e.reason);
      // send to Sentry/Datadog
    });
  });
</script>
```

---

## Global Error Handlers: window.onerror and unhandledrejection

These are the fallback for errors that escape framework boundaries.

### window.onerror

Fires for uncaught runtime errors:

```javascript
window.onerror = function(message, source, lineno, colno, error) {
  console.error(`Error at ${source}:${lineno}:${colno}: ${message}`);
  
  reportToErrorService({
    message,
    source,
    lineno,
    colno,
    stack: error?.stack,
    userAgent: navigator.userAgent
  });
};
```

**Parameters:**
- `message`: Error message
- `source`: URL where error occurred (minified/obfuscated without source maps)
- `lineno`, `colno`: Line and column
- `error`: The Error object (contains stack trace)

### window.addEventListener('unhandledrejection')

Fires for Promise rejections with no `.catch()` handler:

```javascript
window.addEventListener('unhandledrejection', (event) => {
  console.error('Unhandled rejection:', event.reason);
  
  // Prevent browser's default error dialog
  event.preventDefault();
  
  reportToErrorService({
    type: 'unhandled_rejection',
    reason: event.reason,
    stack: event.reason?.stack
  });
});
```

**Timing:** fires before rejections are treated as unhandled, allowing you to recover if needed (e.g., retry a network call).

### Overlaps and Ordering

- `window.onerror` doesn't catch promise rejections
- `unhandledrejection` doesn't catch synchronous errors
- Both should be installed to cover all cases

```javascript
const errorHandlers = {
  sync: (err) => reportError({ type: 'sync', err }),
  async: (err) => reportError({ type: 'async', err })
};

window.onerror = (msg, src, line, col, err) => 
  errorHandlers.sync(err);

window.addEventListener('unhandledrejection', (e) => 
  errorHandlers.async(e.reason));
```

---

## Error Reporting to Services

### Basic Pattern

Most modern stacks integrate with Sentry, Datadog, or similar:

```javascript
// Initialize early
import * as Sentry from "@sentry/react";

Sentry.init({
  dsn: "https://[key]@sentry.io/[project]",
  environment: process.env.NODE_ENV,
  tracesSampleRate: 0.1
});

// Automatic error capture via integration
const App = Sentry.withProfiler(() => <Routes />);

// Manual capture
try {
  riskyOperation();
} catch (e) {
  Sentry.captureException(e, {
    tags: { section: 'payment' },
    extra: { userId: currentUser.id }
  });
}
```

### Contextual Data

Always attach:
- User ID / Session ID
- Release version
- Environment (dev/staging/prod)
- Feature flags
- Breadcrumbs (sequence of user actions before error)

```javascript
Sentry.setUser({ id: userId, email: userEmail });
Sentry.setTag('feature', 'checkout');
Sentry.addBreadcrumb({
  message: 'User clicked pay button',
  category: 'user-action',
  level: 'info'
});
```

---

## User-Facing Error Messages

### Derivation Strategy

- **Technical errors** (network timeout, database constraint): Don't show raw message
- **Business errors** (invalid input, insufficient funds): Show exact message
- **Unknown errors**: Show generic "something went wrong" with support contact

```javascript
function getUserMessage(error) {
  if (error.isValidationError) {
    return error.message;  // "Email format invalid"
  }
  if (error.type === 'NetworkError') {
    return 'Unable to reach server. Please check your connection.';
  }
  if (error.type === 'TimeoutError') {
    return 'Request took too long. Please try again.';
  }
  return 'An unexpected error occurred. Please contact support.';
}
```

### Retry Patterns in UI

**Transient errors** (network, timeout) should offer retry:

```jsx
function DataFetcher() {
  const [data, setData] = useState(null);
  const [error, setError] = useState(null);
  const [isRetrying, setIsRetrying] = useState(false);

  const fetchData = async () => {
    try {
      setError(null);
      setIsRetrying(true);
      const result = await api.get('/data');
      setData(result);
    } catch (e) {
      if (e.type === 'NetworkError') {
        setError({
          message: 'Network error. Retrying...',
          canRetry: true
        });
      } else {
        setError({
          message: e.message,
          canRetry: false
        });
      }
    } finally {
      setIsRetrying(false);
    }
  };

  useEffect(() => {
    fetchData();
  }, []);

  if (error) {
    return (
      <div>
        <p>{error.message}</p>
        {error.canRetry && (
          <button onClick={fetchData} disabled={isRetrying}>
            {isRetrying ? 'Retrying...' : 'Retry'}
          </button>
        )}
      </div>
    );
  }

  return <div>{data}</div>;
}
```

### Exponential Backoff + Jitter

For automatic retries:

```javascript
async function fetchWithRetry(fn, maxAttempts = 3) {
  let lastError;
  
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      return await fn();
    } catch (e) {
      lastError = e;
      if (attempt < maxAttempts) {
        const delayMs = Math.min(
          1000 * Math.pow(2, attempt - 1),  // exponential
          10000
        ) + Math.random() * 1000;  // jitter
        await new Promise(r => setTimeout(r, delayMs));
      }
    }
  }
  
  throw lastError;
}
```

---

## See Also

- [Error Handling Language Patterns](error-handling-language-patterns.md)
- [Error Reporting Systems](error-reporting-systems.md)
- [Frontend State Management](web-state-management.md)
- [React Performance](framework-react-performance.md)