# Qwik — JavaScript Framework for Resumability

## Conceptual Foundation: Resumability vs. Hydration

Most JavaScript frameworks (React, Next.js, Vue, Nuxt) use **hydration**: ship HTML from the server, then replay the entire component tree in the browser to attach event listeners and rebuild the component state. This is expensive. Qwik removes that cost through **resumability**.

**Resumability** means the application can resume on the browser exactly where the server left off, without re-executing framework code or rebuilding state. The server generates serialized component information, and the browser picks up execution directly when needed.

```javascript
// Server renders this and serializes the component state
export default component$(() => {
  const count = useSignal(0);
  return <button onClick$={() => count.value++}>{count.value}</button>;
});
// Browser: only the click handler re-downloads and executes when clicked
// No hydration. No replaying React render cycles.
```

Hydration is wasted work for components the user never interacts with. Resumability eliminates it completely.

## Architecture: Lazy Loading + Serialization

### QRL (Qwik Resource Locator)

**QRL** is Qwik's URL-based serialization format for code closures and component references.

```javascript
// Regular closure (doesn't work across serialization boundary)
const handler = () => { /* code */ };

// QRL wraps it
export const myHandler = $(() => { /* code */ });
// Compiles to: myHandler = qrl('./module', 'myHandler', [dependencies]);
// Serializes in HTML as a URL reference
```

When the browser encounters an event or interaction, it fetches only the code needed (lazy-loading) and deserializes the QRL to re-establish the closure. The `$` macro marks code intended to be lazy-loaded.

### Compilation Stages

1. **Build time**: The Qwik compiler breaks code into chunks at function boundaries marked with `$`. Each handler, lifecycle hook, and computed is its own lazy-loadable unit.
2. **Server render**: Components execute once on the server, serializing component state and QRL references into HTML.
3. **Browser idle**: Qwik prefetches likely-needed chunks but doesn't eagerly download everything.
4. **Interaction**: When the user clicks a button, the browser fetches the handler code, deserializes context, and executes.

This is a fundamental architectural shift from render-driven to interaction-driven code loading.

## Core APIs

### Signals

Qwik uses **signals** for reactive state, similar to Solid.js:

```javascript
import { component$, useSignal } from '@builder.io/qwik';

export default component$(() => {
  const count = useSignal(0);
  return (
    <div>
      <p>Count: {count.value}</p>
      <button onClick$={() => count.value++}>Increment</button>
    </div>
  );
});
```

Signals are primitives, not objects. They track dependencies granularly — only rendering code that observes the changed signal. This avoids parent re-renders when child state changes.

### Lifecycle Hooks

```javascript
// Only runs on server
useServerData$(() => { return fetchExpensiveData(); });

// Runs on server, then resumes on browser if needed
useResource$(() => { 
  return /* data fetch or computation */;
});

// Runs when component is visible in viewport
useVisibleTask$(() => { /* browser-only */ });

// Runs on server and browser
useTask$(() => { /* effect */ });
```

The `$` suffix indicates code that's split into its own chunk.

## Qwik City (Meta-Framework)

**Qwik City** adds file-based routing, layouts, and middleware on top of Qwik, similar to Next.js or Nuxt.

```
src/
  routes/
    index.tsx           // / route
    about/
      index.tsx         // /about
      [id]/
        index.tsx       // /about/[id] — dynamic segment
    api/
      data.ts           // API endpoint: /api/data
    layout.tsx          // Root layout
```

### Server$ Functions

**Server** functions run exclusively on the server:

```javascript
import { server$ } from '@builder.io/qwik';

export const fetchUserFromDB = server$((userId: string) => {
  return db.users.findById(userId); // only runs on server
});

export default component$(() => {
  const user = useResource$(async () => {
    return await fetchUserFromDB('123'); // called from browser
  });
  return <div>{user.value?.name}</div>;
});
```

This avoids exposing database queries to the client. The `server$` function becomes an RPC endpoint.

### Hybrid Rendering

Qwik City supports multiple rendering modes:

- **SSR (Server-Side Rendering)** — Default. Render on server, ship resumable HTML.
- **SSG (Static Site Generation)** — Pre-render routes at build time.
- **Streaming** — Start sending HTML before all data fetches complete.

## Comparison to Next.js and Remix

| Aspect              | Qwik                                  | Next.js                              | Remix                         |
| ------------------- | ------------------------------------- | ------------------------------------ | ----------------------------- |
| **Core Mechanism**  | Resumability (no hydration)           | Hydration (replay on browser)        | Progressive enhancement       |
| **State Model**     | Signals (granular reactivity)         | Hooks (functional components)        | Loader/action pattern         |
| **Lazy Loading**    | Automatic by `$` boundary             | Manual with `dynamic()`              | Manual via route splitting    |
| **Bundle Semantics**| QRL-based, interaction-driven         | Tree-shaking + code-splitting        | Loaders split from components |
| **Ideal For**       | Large interactive apps with big JS   | Full-stack apps with rich UX        | Server-driven forms + UX      |

**Hydration cost tradeoff**: Next.js and Remix require hydration for all interactive components. Qwik avoids it but introduces QRL serialization complexity. Very large, heavily interactive apps may see real performance wins. Small apps may not.

**Ecosystem**: Next.js (React ecosystem, large) > Remix (focused) > Qwik (smaller, growing). Qwik uses distinct APIs; jumping from React is non-trivial.

## Progressive Enhancement

Qwik is built on progressive enhancement principles: the initial HTML is fully functional without JavaScript. Event handlers, navigation, and forms work with plain HTML before JavaScript hydrates interactivity.

This is not unique (Remix, Astro embrace it too), but Qwik's resumability makes it natural rather than requiring server-rendered escape hatches.

## Known Tradeoffs

- **Compiler lock-in**: Heavy reliance on Qwik's compiler for `$` boundaries and code-splitting. Framework churn risk.
- **Developer experience**: The `$` convention is unintuitive initially. Debugging serialized state can be harder than traditional React.
- **Third-party integration**: Libraries written for React don't automatically work; Qwik needs wrappers or rewrites.
- **Ecosystem maturity**: Smaller ecosystem than React or Vue. Fewer pre-built integrations.
- **Learning curve**: Fundamentally different architecture from dominant frameworks. Not a React skill transfer.

## Research Direction

Qwik is actively developed by Builder.io. The ecosystem is growing (Qwik City stabilizing, community integrations appearing). The core idea — avoiding hydration — is sound, but adoption depends on real-world performance data and ecosystem growth matching React/Next.js.

**See also**: framework-nextjs, framework-remix, framework-astro, paradigm-progressive-enhancement, web-javascript-loading-strategy