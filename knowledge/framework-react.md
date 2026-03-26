# React

## Core Architecture

React uses a **fiber architecture** (since React 16) where the work of rendering is split into units called fibers. Each fiber represents a component instance, DOM node, or other React element. The fiber tree is a linked list where each fiber has pointers to its first child, sibling, and parent (return).

**Reconciliation** compares the previous fiber tree with a new element tree. React uses heuristics: elements of different types produce different trees (full teardown), and `key` props identify stable elements across renders. Keys must be stable, unique among siblings — using array index as key breaks state preservation when items reorder.

**Virtual DOM diffing** operates in two phases:

1. **Render phase** (interruptible in concurrent mode): walks the tree, computes changes, builds a work-in-progress fiber tree
2. **Commit phase** (synchronous): applies all DOM mutations, runs layout effects, then passive effects

## Hooks Reference

| Hook                   | Purpose                           | Re-renders on                                    |
| ---------------------- | --------------------------------- | ------------------------------------------------ |
| `useState`             | Local state                       | `setState` with new value (Object.is comparison) |
| `useReducer`           | Complex state logic               | `dispatch` producing new state                   |
| `useEffect`            | Side effects after paint          | Dependency array changes                         |
| `useLayoutEffect`      | Side effects before paint         | Dependency array changes                         |
| `useMemo`              | Memoize computed values           | Dependency array changes                         |
| `useCallback`          | Memoize functions                 | Dependency array changes                         |
| `useRef`               | Mutable ref (no re-render)        | Never (mutation is silent)                       |
| `useContext`           | Subscribe to context              | Context value changes                            |
| `useId`                | Stable unique ID for SSR          | Never                                            |
| `useSyncExternalStore` | External store subscription       | Store snapshot changes                           |
| `useTransition`        | Mark updates as non-urgent        | `isPending` flag                                 |
| `useDeferredValue`     | Defer a value to next render      | Source value changes (deferred)                  |
| `use` (React 19)       | Unwrap promises/context in render | Resolved value                                   |

### useState Gotchas

```jsx
// Stale closure — reads initial count forever
function Counter() {
  const [count, setCount] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setCount(count + 1), 1000); // BUG: stale
    return () => clearInterval(id);
  }, []); // empty deps captures count=0
}

// Fix: updater function
setCount((prev) => prev + 1);
```

State updates are batched in React 18+ — even in setTimeout, promises, and native event handlers (automatic batching). Use `flushSync` to force synchronous updates.

### useEffect Execution Order

1. Component renders (returns JSX)
2. React commits DOM changes
3. Browser paints
4. `useEffect` cleanup from previous render runs
5. `useEffect` callback runs

`useLayoutEffect` runs between steps 2 and 3 — blocks paint. Use for DOM measurements.

### useMemo and useCallback

```jsx
// useMemo: memoize expensive computation
const sorted = useMemo(() => items.sort(compareFn), [items]);

// useCallback: memoize function identity (for child prop stability)
const handleClick = useCallback((id) => {
  setSelected(id);
}, []); // stable reference
```

These are **performance optimizations**, not semantic guarantees. React may discard cached values under memory pressure. The React Compiler (below) automates most of this.

## Server Components (React 19)

Server Components render on the server and send serialized output (RSC payload) to the client. They can directly access databases, filesystems, and private APIs.

**Rules:**

- Server Components cannot use hooks (useState, useEffect, etc.)
- Server Components cannot use browser APIs
- Server Components can `import` Client Components but not vice versa
- Client Components are marked with `"use client"` at the top of the file
- Props passed from Server → Client must be serializable (no functions, classes, Dates)

**Server Actions** (React 19): functions marked with `"use server"` that can be called from client components — they execute on the server and return serialized results.

```jsx
// server action
async function addTodo(formData) {
  "use server";
  await db.todos.insert({ text: formData.get("text") });
  revalidatePath("/todos");
}

// client component using it
<form action={addTodo}>
  <input name="text" />
</form>;
```

## Concurrent Features

**Transitions** mark state updates as non-urgent, allowing React to keep the current UI responsive:

```jsx
const [isPending, startTransition] = useTransition();
startTransition(() => {
  setSearchResults(filterLargeList(query)); // non-urgent
});
```

**Suspense** declares loading boundaries:

```jsx
<Suspense fallback={<Skeleton />}>
  <AsyncComponent /> {/* throws a promise while loading */}
</Suspense>
```

Suspense works with: `React.lazy`, data fetching libraries that integrate with Suspense (Relay, SWR, TanStack Query), and the `use()` hook.

**Streaming SSR**: React 18+ can stream HTML in chunks using `renderToPipeableStream`, sending the shell first and streaming in `<Suspense>` boundaries as they resolve.

## Component Patterns

### Error Boundaries

```jsx
class ErrorBoundary extends Component {
  state = { hasError: false };
  static getDerivedStateFromError(error) {
    return { hasError: true };
  }
  componentDidCatch(error, errorInfo) {
    logErrorToService(error, errorInfo);
  }
  render() {
    if (this.state.hasError) return <Fallback />;
    return this.props.children;
  }
}
```

Error boundaries catch errors in rendering, lifecycle methods, and constructors. They do NOT catch errors in event handlers, async code, SSR, or errors in the boundary itself.

### Portals

```jsx
createPortal(<Modal />, document.getElementById("modal-root"));
```

Portals render children into a different DOM node but events still bubble up through the React tree (not the DOM tree).

### forwardRef and useImperativeHandle

```jsx
const FancyInput = forwardRef((props, ref) => {
  const inputRef = useRef();
  useImperativeHandle(ref, () => ({
    focus: () => inputRef.current.focus(),
    clear: () => {
      inputRef.current.value = "";
    },
  }));
  return <input ref={inputRef} {...props} />;
});
```

React 19 passes `ref` as a regular prop — `forwardRef` is no longer needed.

### Controlled vs Uncontrolled

| Aspect      | Controlled                       | Uncontrolled                     |
| ----------- | -------------------------------- | -------------------------------- |
| State owner | React (via `value` + `onChange`) | DOM (via `defaultValue` + `ref`) |
| Validation  | On every change                  | On submit                        |
| Performance | Re-render per keystroke          | Minimal re-renders               |
| Use case    | Complex forms, dependent fields  | Simple forms, file inputs        |

## Custom Hooks Patterns

```jsx
// Encapsulate fetch logic
function useFetch(url) {
  const [data, setData] = useState(null);
  const [error, setError] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const controller = new AbortController();
    fetch(url, { signal: controller.signal })
      .then((res) => res.json())
      .then(setData)
      .catch(setError)
      .finally(() => setLoading(false));
    return () => controller.abort();
  }, [url]);

  return { data, error, loading };
}

// Debounced value
function useDebouncedValue(value, delay) {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setDebounced(value), delay);
    return () => clearTimeout(id);
  }, [value, delay]);
  return debounced;
}
```

## React Compiler (React Forget)

The React Compiler (shipping with React 19) automatically memoizes components, hooks, and values — making manual `useMemo`, `useCallback`, and `React.memo` unnecessary in most cases.

It works as a Babel plugin that analyzes your code at build time and inserts memoization where beneficial. It respects the **Rules of React**: components must be pure functions, hooks must be called unconditionally at the top level, and props/state must not be mutated.

**What it auto-memoizes:** JSX expressions, hook dependencies, component render results, expensive computations.

**What it can't fix:** side effects in render, mutation of props/state, conditional hook calls.

## Performance Pitfalls

**Unnecessary re-renders:**

- Parent re-render → all children re-render (unless wrapped in `React.memo` or compiler handles it)
- New object/array literals in props: `style={{color: 'red'}}` creates a new object every render
- Inline function props without `useCallback` (matters only when child uses `React.memo`)
- Context value changes re-render ALL consumers — split contexts by update frequency

**Stale closures:** any callback captured in an effect or event handler closes over the state from that render. Updater functions (`setState(prev => ...)`) and refs avoid this.

**Key misuse:** changing `key` forces unmount + remount (useful intentionally to reset state, destructive otherwise).

## Synthetic Events

React wraps native events in `SyntheticEvent` for cross-browser consistency. Events are pooled in React 16 (not in 17+). `e.nativeEvent` accesses the underlying DOM event. `stopPropagation` stops React tree propagation; `e.nativeEvent.stopImmediatePropagation()` stops DOM propagation.

## Strict Mode

`<StrictMode>` in development:

- Double-invokes render, effects, and reducers to surface impure code
- Warns about deprecated APIs
- Does nothing in production

React 18 Strict Mode mounts → unmounts → remounts components to test effect cleanup resilience.
