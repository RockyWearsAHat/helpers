# SolidJS

## Overview

SolidJS is a reactive UI library that looks like React (JSX, components) but works fundamentally differently. Components run once — they're setup functions, not re-rendering functions. Reactivity is fine-grained: when a signal changes, only the specific DOM node that reads it updates. No virtual DOM, no diffing, no re-execution.

## Core Reactivity Primitives

### Signals

Signals are reactive values. Reading a signal inside a tracking context (effects, memos, JSX) creates a subscription:

```tsx
import { createSignal } from "solid-js";

function Counter() {
  const [count, setCount] = createSignal(0);
  // count is a getter function, NOT a value

  console.log("This runs ONCE — component setup");

  return (
    <div>
      {/* count() in JSX creates a fine-grained subscription */}
      <p>Count: {count()}</p>
      <button onClick={() => setCount(count() + 1)}>+1</button>
      <button onClick={() => setCount((c) => c + 1)}>+1 (prev)</button>
    </div>
  );
}
```

**Critical difference from React**: `count` is a function. Call `count()` to read. The component function body runs once — only the specific `{count()}` expression re-executes when the signal changes.

### Effects

Run side effects when dependencies change:

```tsx
import { createSignal, createEffect, onCleanup } from "solid-js";

function Timer() {
  const [seconds, setSeconds] = createSignal(0);

  createEffect(() => {
    // Tracks `seconds()` automatically
    console.log("Seconds:", seconds());
  });

  // Effect with cleanup
  createEffect(() => {
    const interval = setInterval(() => setSeconds((s) => s + 1), 1000);
    onCleanup(() => clearInterval(interval));
  });

  return <span>{seconds()}</span>;
}
```

`createEffect` runs synchronously after render, then re-runs when tracked signals change. Dependencies are tracked automatically — no dependency arrays.

### Memos (Derived Values)

```tsx
import { createSignal, createMemo } from "solid-js";

function ExpensiveList() {
  const [items, setItems] = createSignal([1, 2, 3, 4, 5]);
  const [filter, setFilter] = createSignal("");

  // Memoized — only recomputes when items() or filter() change
  const filteredItems = createMemo(() => {
    console.log("Filtering..."); // only runs when dependencies change
    return items().filter((i) => String(i).includes(filter()));
  });

  return (
    <ul>
      {filteredItems().map((i) => (
        <li>{i}</li>
      ))}
    </ul>
  );
}
```

### Resources (Async Data)

```tsx
import { createResource, Suspense } from "solid-js";

const fetchUser = async (id: string) => {
  const resp = await fetch(`/api/users/${id}`);
  return resp.json();
};

function UserProfile(props: { userId: string }) {
  const [user, { mutate, refetch }] = createResource(
    () => props.userId, // source signal (re-fetches when it changes)
    fetchUser, // fetcher function
  );

  return (
    <Suspense fallback={<p>Loading...</p>}>
      <div>
        <h1>{user()?.name}</h1>
        <p>{user()?.email}</p>
        <button onClick={refetch}>Refresh</button>
      </div>
    </Suspense>
  );
}
```

## Control Flow Components

Solid uses special components for control flow — they're compiled for optimal DOM updates:

### Show

```tsx
import { Show } from "solid-js";

<Show when={user()} fallback={<LoginPrompt />}>
  {(u) => <Profile user={u()} />} {/* u is an accessor — call u() */}
</Show>;
```

### Switch / Match

```tsx
import { Switch, Match } from "solid-js";

<Switch fallback={<p>Unknown status</p>}>
  <Match when={status() === "loading"}>
    <Spinner />
  </Match>
  <Match when={status() === "error"}>
    <ErrorMessage />
  </Match>
  <Match when={status() === "success"}>
    <Content />
  </Match>
</Switch>;
```

### For (Keyed Iteration)

```tsx
import { For } from "solid-js";

// For: items are keyed by reference — efficient for objects
<For each={items()}>
  {(item, index) => (
    <div>
      {index()}: {item.name} {/* index is an accessor */}
    </div>
  )}
</For>;
```

### Index (Non-keyed Iteration)

```tsx
import { Index } from "solid-js";

// Index: items keyed by position — efficient for primitives
<Index each={counts()}>
  {(count, index) => (
    <div>
      {index}: {count()} {/* count is reactive, index is static */}
    </div>
  )}
</Index>;
```

**When to use which**: `For` when items change identity (array of objects). `Index` when values change at fixed positions (array of primitives).

### Portal

```tsx
import { Portal } from "solid-js/web";

<Portal mount={document.getElementById("modal-root")!}>
  <div class="modal">Modal content</div>
</Portal>;
```

### Dynamic

```tsx
import { Dynamic } from "solid-js/web";

<Dynamic component={isAdmin() ? AdminPanel : UserPanel} user={user()} />;
```

## Stores

Stores are deeply reactive proxies for complex state:

```tsx
import { createStore, produce } from "solid-js/store";

interface Todo {
  id: number;
  text: string;
  completed: boolean;
}

function TodoApp() {
  const [state, setState] = createStore({
    todos: [] as Todo[],
    filter: "all" as "all" | "active" | "completed",
  });

  // Direct path-based updates
  setState("filter", "active");
  setState("todos", 0, "completed", true); // state.todos[0].completed = true

  // produce for Immer-like syntax
  setState(
    produce((draft) => {
      draft.todos.push({ id: Date.now(), text: "New todo", completed: false });
    }),
  );

  // Functional update
  setState("todos", (todos) => todos.filter((t) => !t.completed));

  // Reconcile — replace array while preserving references where possible
  const newTodos = await fetchTodos();
  setState("todos", reconcile(newTodos));

  return (
    <For each={state.todos}>
      {(todo) => (
        <div>
          <span
            style={{
              "text-decoration": todo.completed ? "line-through" : "none",
            }}
          >
            {todo.text}
          </span>
          <button
            onClick={() =>
              setState(
                "todos",
                (t) => t.id === todo.id,
                "completed",
                (c) => !c,
              )
            }
          >
            Toggle
          </button>
        </div>
      )}
    </For>
  );
}
```

### Store Path Syntax

```tsx
// setState(path..., value)
setState("user", "name", "Alice"); // state.user.name = "Alice"
setState("todos", 0, "completed", true); // state.todos[0].completed = true
setState("todos", { from: 0, to: 4 }, "completed", true); // range
setState("todos", (t) => t.completed, "visible", false); // filtered
```

## Context

```tsx
import { createContext, useContext } from "solid-js";

interface ThemeContextType {
  theme: () => string;
  toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextType>();

function ThemeProvider(props: { children: any }) {
  const [theme, setTheme] = createSignal("light");
  const store: ThemeContextType = {
    theme,
    toggleTheme: () => setTheme((t) => (t === "light" ? "dark" : "light")),
  };

  return (
    <ThemeContext.Provider value={store}>
      {props.children}
    </ThemeContext.Provider>
  );
}

function useTheme() {
  const context = useContext(ThemeContext);
  if (!context) throw new Error("useTheme must be used within ThemeProvider");
  return context;
}
```

## Reactivity Escape Hatches

### untrack

Read a signal without subscribing:

```tsx
import { untrack } from "solid-js";

createEffect(() => {
  console.log(name()); // tracks name
  console.log(untrack(() => age())); // reads age but doesn't track
});
```

### on

Explicit dependency declaration:

```tsx
import { on } from "solid-js";

// Only tracks `a`, ignores `b` even though it's read
createEffect(on(a, (aVal) => {
  console.log(aVal, b());
}));

// Multiple sources
createEffect(on([a, b], ([aVal, bVal]) => { ... }));

// Defer — don't run on initial value
createEffect(on(a, (val) => { ... }, { defer: true }));
```

### batch

Group multiple updates into one:

```tsx
import { batch } from "solid-js";

batch(() => {
  setFirstName("John");
  setLastName("Doe");
  // Effects that depend on both run once, not twice
});
```

## Solid Router

```tsx
import { Router, Route, A, useParams, useNavigate } from "@solidjs/router";

function App() {
  return (
    <Router>
      <Route path="/" component={Home} />
      <Route path="/users" component={UsersLayout}>
        <Route path="/" component={UsersList} />
        <Route path="/:id" component={UserProfile} />
      </Route>
      <Route path="*404" component={NotFound} />
    </Router>
  );
}

function UserProfile() {
  const params = useParams(); // params.id is reactive
  const navigate = useNavigate();

  const [user] = createResource(() => params.id, fetchUser);

  return <div>{user()?.name}</div>;
}

// Link component
<A href="/users/1" activeClass="active">
  User 1
</A>;
```

## SolidStart (Meta-Framework)

SolidStart adds SSR, file-based routing, and server functions to SolidJS:

```tsx
// src/routes/index.tsx
import { createAsync } from "@solidjs/router";
import { getUsers } from "~/server/users";

export default function Home() {
  const users = createAsync(() => getUsers());
  return <For each={users()}>{(user) => <div>{user.name}</div>}</For>;
}

// Server function
("use server");
export async function getUsers() {
  return await db.user.findMany();
}
```

## React vs Solid

| Aspect              | React                         | Solid                                       |
| ------------------- | ----------------------------- | ------------------------------------------- |
| Component execution | Re-runs on every state change | Runs once (setup)                           |
| Reactivity          | Virtual DOM diffing           | Fine-grained signals                        |
| State               | `useState` returns value      | `createSignal` returns getter function      |
| Dependency tracking | Manual (useEffect deps array) | Automatic                                   |
| Re-rendering        | Entire component subtree      | Only affected DOM nodes                     |
| Hooks rules         | Must follow rules of hooks    | No restrictions                             |
| JSX                 | Creates virtual DOM elements  | Compiles to real DOM operations             |
| Bundle size         | ~40KB (react + react-dom)     | ~7KB                                        |
| Refs                | `useRef` / `createRef`        | Variable assignment in JSX                  |
| Children            | `{children}`                  | `{props.children}` (careful — not a getter) |

### Key Gotcha: Props Destructuring

```tsx
// WRONG — breaks reactivity! Values captured at call time
function Bad({ name }: { name: string }) {
  return <div>{name}</div>; // never updates
}

// CORRECT — access through props
function Good(props: { name: string }) {
  return <div>{props.name}</div>; // reactive
}

// Also CORRECT — use splitProps for partial destructure
function Also(props: { name: string; class: string }) {
  const [local, others] = splitProps(props, ["class"]);
  return (
    <div class={local.class} {...others}>
      {props.name}
    </div>
  );
}
```
