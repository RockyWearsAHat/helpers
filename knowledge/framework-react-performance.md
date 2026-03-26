# React Performance — Re-renders, Memoization, Compilation & Optimization Strategies

## Overview

React performance optimization is about reducing unnecessary work: fewer re-renders, smaller bundles, faster DOM mutations. The fiber architecture makes incremental improvement possible (you don't have to rewrite everything to optimize), but poor defaults can spiral—props change, components re-render, children re-render, the tree explodes in computational cost.

Performance work follows a sequence: measure (profiler), identify bottlenecks (DevTools), apply targeted fixes (memo, lazy, Suspense), verify improvement (profiler again).

## Re-render Causes & Effects

React re-renders when:

1. **Component state changes** (`setState` called with new value, Object.is comparison)
2. **Parent re-renders** (all children re-render by default, regardless of prop changes)
3. **Context value changes** (all subscribers re-render)
4. **Hook dependency changes** (`useEffect`, `useMemo`, `useCallback` dependency arrays)

### Why This Matters

Re-rendering is not free. It means:
- Component function executes again
- JSX is evaluated
- Fiber tree is traversed and compared
- Hooks are called in order

For a large tree, a root re-render can cascade: root → many children → expensive computations → slow UI response.

**Key insight**: Re-rendering ≠ DOM mutation. React compares the new tree with the old fiber tree (reconciliation). If no actual DOM changes, the browser sees no flicker. But the CPU still paid the render cost.

## Memoization: memo, useMemo, useCallback

Memoization is caching—skip work if inputs haven't changed.

### React.memo

Memoizes a component: re-render only if props change (shallow comparison via Object.is).

```jsx
const Post = React.memo(function Post({ id, title }) {
  console.log('Post rendered');
  return <article>{title}</article>;
});

// Parent re-renders, Post doesn't re-render if id and title are Object.is-equal
```

**Common trap**: Passing a new object/array as prop defeats memo.

```jsx
// ❌ WRONG — new object every render
export function Page() {
  return <Post config={{ theme: 'dark' }} />;
}

// ✅ CORRECT — stable object
const config = { theme: 'dark' };
export function Page() {
  return <Post config={config} />;
}
```

Or with dependency management:

```jsx
const config = useMemo(() => ({ theme: 'dark' }), []);
return <Post config={config} />;
```

**When to use memo**: Components that are expensive (render a large subtree, do heavy computations) and receive stable props most of the time. Avoid memo on every component—it's rarely necessary and adds cognitive overhead.

### useMemo

Memoizes a computed value. Re-compute only if dependency array changes.

```jsx
const expensiveValue = useMemo(() => {
  return complexCalculation(data);
}, [data]);
```

**Valid use cases**:
- Avoiding re-creation of objects/arrays passed as props (triggers child memo)
- Skipping expensive computations (sort, filter, transform large lists)
- Stabilizing context values (to avoid Context Provider re-renders)

**Anti-patterns**:
- `useMemo(() => { return value; }, [])` wrapping primitives or simple expressions (the memo overhead > savings)
- Wrapping render logic instead of moving to a separate component with memo

### useCallback

Memoizes a function. Re-create only if dependencies change.

```jsx
const handleClick = useCallback(() => {
  console.log(count);
}, [count]);
```

**Valid use cases**:
- Passing callbacks to memoized children (new function triggers child re-render)
- Adding to dependency arrays of other hooks
- Registering event listeners or timers (new function recreates listener)

**Anti-pattern**: Wrapping every callback. Most callbacks are cheap; if not passed to memo components, the memo prevents re-render anyway.

### Profiler: Measuring Memoization Impact

React DevTools Profiler shows:
- How long each component took to render
- Which components re-rendered unnecessarily
- Render chains (parent → child cascade)

Use it to identify expensive renders before adding memo. Don't optimize blind.

## Code Splitting & Lazy Loading

Bundle size is a performance killer. React.lazy enables code splitting.

### React.lazy + Suspense

```jsx
const BlogPost = React.lazy(() => import('./BlogPost'));

export default function Page() {
  return (
    <Suspense fallback={<div>Loading post...</div>}>
      <BlogPost />
    </Suspense>
  );
}
```

When `BlogPost` is first rendered, React suspends and downloads the chunk. While suspended, the fallback is shown. Once the chunk loads and component renders, React replaces the fallback.

### Route-Based Splitting (Recommended)

Split at route boundaries (pages), not component boundaries (within pages). Routes naturally partition features.

```jsx
// pages.tsx (or app/router.tsx for Next.js)
const Home = lazy(() => import('./pages/Home'));
const About = lazy(() => import('./pages/About'));
const Blog = lazy(() => import('./pages/Blog'));

export function Router() {
  const [page, setPage] = useState('home');
  return (
    <>
      <nav>
        <button onClick={() => setPage('home')}>Home</button>
        <button onClick={() => setPage('about')}>About</button>
      </nav>
      <Suspense fallback={<div>Loading...</div>}>
        {page === 'home' && <Home />}
        {page === 'about' && <About />}
      </Suspense>
    </>
  );
}
```

### Component-Based Splitting

Split expensive subtrees within a page:

```jsx
const Comments = lazy(() => import('./Comments'));

export default function Post({ postId }) {
  return (
    <>
      <article>...</article>
      <Suspense fallback={<div>Loading comments...</div>}>
        <Comments postId={postId} />
      </Suspense>
    </>
  );
}
```

The post renders immediately; comments load in parallel.

## Suspense Boundaries & Progressive Rendering

Suspense coordinates asynchronous work in render logic. Multiple Suspense boundaries allow partial rendering.

```jsx
export default function Dashboard() {
  return (
    <div>
      <Suspense fallback={<HeaderSkeleton />}>
        <Header /> {/* Suspends for data */}
      </Suspense>
      
      <Suspense fallback={<SidebarSkeleton />}>
        <Sidebar /> {/* Independent suspension */}
      </Suspense>
      
      <Suspense fallback={<MainSkeleton />}>
        <Main /> {/* Renders when ready */}
      </Suspense>
    </div>
  );
}
```

Each boundary renders its fallback independently. The header doesn't block the sidebar. As each section's data arrives, it replaces its fallback.

**Gotcha**: Suspense requires data fetching code that actually throws promises (or uses `use()` with a promise). Not all data libraries integrate; you need library support or wrapper logic.

## Virtualization: Virtual Scrolling

Rendering 10,000 list items kills performance. Virtualization renders only visible items.

### react-window (Lightweight)

```jsx
import { FixedSizeList } from 'react-window';

const items = Array.from({ length: 10000 }, (_, i) => `Item ${i}`);

export default function VirtualList() {
  return (
    <FixedSizeList
      height={600}
      itemCount={items.length}
      itemSize={40}
      width="100%"
    >
      {({ index, style }) => (
        <div style={style}>{items[index]}</div>
      )}
    </FixedSizeList>
  );
}
```

Only ~15 items are in the DOM at any time, regardless of list length. Scrolling reuses DOM nodes.

### react-virtuoso (Feature-rich)

Virtuoso handles variable heights, grouped items, pagination:

```jsx
<Virtuoso
  style={{ height: '600px' }}
  data={items}
  itemContent={(index, item) => <div>{item.name}</div>}
/>
```

**When to use**: Lists > 100 items, fixed or variable heights. Don't virtualize small lists; overhead isn't worth it.

## React Compiler (React 19+)

The React Compiler is an opt-in compiler that automatically memoizes functions and values, eliminating the mental burden of `useMemo`, `useCallback`, and `memo`.

```jsx
// Before: manual memoization
function Component({ count }) {
  const doubled = useMemo(() => count * 2, [count]);
  const handleClick = useCallback(() => console.log(doubled), [doubled]);
  return <button onClick={handleClick}>{doubled}</button>;
}

// After: Compiler handles memoization
function Component({ count }) {
  const doubled = count * 2;
  const handleClick = () => console.log(doubled);
  return <button onClick={handleClick}>{doubled}</button>;
}
```

The compiler analyzes the code and inserts memoization where needed. It assumes you want optimal re-render behavior and only re-renders when actual values change.

**Available environment**: Experimental in React 19 via `babel-plugin-react-compiler`. Stable version TBD. Not yet in Next.js by default.

## Transitions: startTransition & useTransition

Concurrency: Mark low-priority updates so high-priority updates (user input) are handled first.

### startTransition

```jsx
import { startTransition } from 'react';
import { search } from './api';

export function SearchExample() {
  const [results, setResults] = useState([]);
  
  function handleSearch(query) {
    // Mark state update as low-priority
    startTransition(() => {
      const data = search(query);
      setResults(data);
    });
  }
  
  return (
    <>
      <input onChange={(e) => handleSearch(e.target.value)} />
      <Results results={results} />
    </>
  );
}
```

If a fast keystroke follows, React pauses the search render and handles the keystroke first. The search result render resumes when there's time.

### useTransition

Provides a flag (`isPending`) to show loading state:

```jsx
'use client';
import { useTransition } from 'react';

export function SaveButton({ onSave }) {
  const [isPending, startTransition] = useTransition();
  
  function handleClick() {
    startTransition(async () => {
      await onSave();
    });
  }
  
  return (
    <button disabled={isPending} onClick={handleClick}>
      {isPending ? 'Saving...' : 'Save'}
    </button>
  );
}
```

Keeps the UI responsive while async work happens in the background.

**When to use**: Search boxes, autocomplete, form submissions where you want fast keyboard response while slow data fetches don't block.

## Rendering Patterns

### Render As You Fetch

Kick off data fetches as soon as you know you'll need the data, not inside a component render or effect.

```jsx
// ❌ Waterfall: fetch inside component
function UserProfile({ userId }) {
  const [user, setUser] = useState(null);
  useEffect(() => {
    fetch(`/api/user/${userId}`).then(setUser); // Fetch happens after render
  }, [userId]);
}

// ✅ Render as you fetch: start fetch on navigation, not in component
const userPromise = fetch(`/api/user/${userId}`);
const user = use(userPromise); // Suspend, wait for promise
function UserProfile() {
  return <div>{user.name}</div>;
}
```

Second approach eliminates waterfall—the fetch starts as soon as you navigate, not after the component mounts.

### Passing Props vs Context

Props force children to re-render when parent re-renders. Context avoids re-renders at intermediate nodes but all context subscribers re-render when context changes.

**Use props for**: Frequently-changing data (props usually change, so re-rendering is expected).

**Use context for**: Slowly-changing config data (theme, auth) where you want subscribers to optimize independently.

### Separating Server State from UI State

Mixing server state ("what's in the database") with UI state ("which tab is open") creates re-render cascades.

```jsx
// ❌ Mixed: both in component state
const [posts, setPosts] = useState([]);
const [currentTab, setCurrentTab] = useState('all');

// ✅ Separated: use a library (TanStack Query) for server state
const { data: posts } = useQuery(['posts'], fetchPosts);
const [currentTab, setCurrentTab] = useState('all');
```

UI state updates don't invalidate server queries, and vice versa.

## Performance Profiling Checklist

1. **Measure**: Run React DevTools Profiler on a typical workload
2. **Identify**: Find the slowest component (render time or re-render frequency)
3. **Understand**: Is it re-rendering unnecessarily? Is the component itself slow?
4. **Optimize**: memo, lazy, Suspense, or refactor logic
5. **Verify**: Re-profile to confirm improvement and measure new bottleneck

Repeat until satisfied. Premature optimization wastes time; measure first.

## Related

See also: [performance-optimization.md](performance-optimization.md) (general principles), [performance-web-vitals.md](performance-web-vitals.md) (browser metrics), [web-rendering-patterns.md](web-rendering-patterns.md) (SSR, streaming).