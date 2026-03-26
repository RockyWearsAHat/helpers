# Frontend State Management Patterns

## Overview

State management determines how components access and modify application data. Solutions range from local component state to global stores, with different philosophies on where state lives, how it updates, and which components can access it. The landscape has shifted dramatically: Redux once dominated, but atomic state (Jotai, Recoil), signals (Solid, Angular, Preact), and lightweight stores (Zustand, Valtio) now coexist for different use cases.

## Local vs Global State

Local state lives in a single component or its descendants via props. Advantages: simple to reason about, no coordination overhead, changes are predictable. Disadvantages: prop drilling (passing through many intermediate components), hard to share state across distant parts of the tree without lifting it all the way up.

Global state lives in a centralized location accessible to many components. Advantages: components can access state from anywhere without props, scales for large apps. Disadvantages: all subscribers re-render when state updates (until optimized), harder to debug, can become monolithic.

The tension: React's default model encourages local state (props), but real apps need shared state. Most applications use a mix: local state for UI details (form inputs, dropdowns), global state for domain data (user, auth, business entities).

## Redux: The Flux Architecture

**Architecture**: Redux implements unidirectional flux: view → action → reducer → store → view. Actions are plain objects describing what happened. Reducers are pure functions receiving (state, action) and returning new state. The store is a single tree holding all app state.

**Middleware system**: Thunks (async actions via redux-thunk) and sagas (side-effect orchestration via redux-saga) handle asynchronous workflows. Middleware intercepts actions, enabling time-travel debugging, logging, analytics injection.

**Selectors** extract computed state (e.g., `selectUnreadCount(state)` selects and filters messages). Selector libraries like Reselect memoize results to prevent unnecessary re-renders.

**Modern Redux** (Redux Toolkit, RTK Query) abstracts boilerplate: immer integration for immutable updates, async thunk helpers, built-in DevTools. However, Redux incurs learning overhead and verbose ceremony compared to newer alternatives.

**When to reach for Redux**: Large-scale apps with complex async workflows, teams highly familiar with the pattern, strong DevTools/debugging needs. Less suitable for small apps or rapidly changing requirements.

## Atomic State: Jotai & Recoil

Atoms are the smallest unit of state — a single value "atom" that components can subscribe to. Jotai (Japanese for "state") and Recoil (Meta/Facebook) both use this model but with different APIs.

**Jotai philosophy**: Simple, minimal, hooks-first. Create an atom, use it in components with `useAtom`. Derived atoms compute from other atoms. No explicit selectors needed — just functions that combine atoms. Better TypeScript support.

**Recoil philosophy**: Atoms, selectors (computed atoms), and atom families (parameterized atoms). Slightly more opinionated, more React-integrated. Atom effects and async selectors for side effects.

**Advantages over Redux**: Atoms only update subscribers to that specific atom (granular), no middleware boilerplate, atoms are composable, minimal initial ceremony. Better code splitting — atoms can be split across files without a monolithic store.

**Disadvantages**: Smaller ecosystem, less established in large teams, fewer debugging tools, sometimes API feels too implicit without formal reducers.

**Trade-off**: Atoms scale well to medium complexity. At very high complexity (lots of derived data, complex workflows), Redux's explicit structure can clarify dependencies. Atomic state excels for "medium chaos" apps.

## Lightweight Stores: Zustand & Valtio

Zustand provides a minimal store architecture similar to Redux but without boilerplate. Create a store with `create()`, return state and actions, use `useStore()` in components. Immutably update state via setState. Optional middleware for devtools, logging, persistence.

Valtio differs conceptually — it embraces mutable-style updates via proxies. `useSnapshot()` signals read dependency; mutations via direct object modification feel more natural than functional updates, though immutability principles still hold under the hood.

**Similarities**: Both are tiny (~3kb), dead-simple to set up, no TypeScript friction, no provider boilerplate. Fast adoption for teams moving away from Redux.

**Zustand trade-off**: Somewhat implicit API (what gets subscribed? depends on what you read in render). Good external state management (non-React state consumers), persistence plugin highly available.

**Valtio trade-off**: Proxy-based reactivity feels React-contradictory to some teams. Excellent for non-React code using state. Slightly different mental model than immutable paradigm.

When to use: Small-to-medium apps, prototypes, replacing prop drilling without Redux complexity.

## Signals: Solid, Angular, Preact

**Signals** are a primitive that React does not have: a mutable reference holding state that automatically schedules component updates when changed. Change a signal's value, and only components reading that signal re-render — no provider, no hooks overhead.

**Solid.js signals**: Core to Solid's reactivity. `createSignal()` returns a getter and setter; components automatically track reads and re-run only the lines that depend on changes. Enables fine-grained reactivity — not component-level, but expression-level.

**Angular signals** (Signals API since Angular 16): Similar principle. Reduce zone.js overhead, enable precise change detection, coexist with RxJS observables.

**Preact signals** (experimental): A signal implementation for Preact/React bridging projects. Shared state without React context.

**Mental model gap**: React developers accustomed to "render-to-describe" often find signals feel like imperative mutation. Signals are not "functional" in the React sense but are deeply reactive. Projects mixing both paradigms sometimes face confusion.

## Server State vs Client State

**Client state**: Owned by the frontend (UI state, local form data, user preferences). Lives in request, relevant to session only.

**Server state**: Source of truth lives on backend. Frontend is a cache. Examples: database records, remote configuration, real-time chat messages.

Conflating them causes bugs: rerunning queries on every re-render, stale data, sync problems. Libraries like React Query (TanStack Query) and SWR (Vercel) explicitly separate concerns.

**React Query** caches server data, handles refetching, background updates, invalidation. Provides `useQuery()` (fetch) and `useMutation()` (updates). Integrates with any state manager for client state; manages server state independently.

**SWR** (stale-while-revalidate) is lighter, focuses on HTTP caching principles. Automatic refetch on focus/reconnect, deduplication of simultaneous requests.

**Anti-pattern**: Storing server data in a global Redux store and manually synchronizing it. Query libraries eliminate 90% of that code.

## URL as State

URLs are the oldest state management system on the web. A URL encodes which page/resource/filter the user is viewing. Clicking "back" works because history is recorded.

Modern SPAs often ignored this — state lived in memory, URLs were ignored, back button broke. Progressive React patterns embrace URL state: filter state, sort order, pagination, search query go in URL search params.

**Advantages**: Shareable links work (copy/paste URL to a friend), back/forward work, bookmarks work, hard refresh preserves context.

**Tools**: React Router, TanStack Router (formerly React Location) manage this. Libraries like `zustand` with URL sync middleware bridge signals and URLs.

**Pattern**: Route parameters (e.g., `/users/:id`) represent primary navigation. Search parameters (e.g., `?sort=date&filter=active`) represent filters and view options. Form state lives in both (controlled inputs) and URL (submitted state).

## State Machines: XState

For complex stateful flows (e.g., checkout, multi-step forms, video player), state machines model all valid states and transitions explicitly. XState is the de facto library.

**Concept**: Define states, events, and transitions. The machine is the source of truth — no invalid state combinations exist (you can't transition from checkout → payment_complete → checkout). Visualizable, testable, often reduces bugs.

**Example**: A toggle machine has states {idle, loading, success, error} and events {FETCH, SUCCESS, ERROR}. Machine ensures only valid transitions occur.

**In React**: `useMachine()` hook integrates XState. Pairs well with atomic state (store machine state in an atom) or server state queries.

**Trade-off**: Feels heavyweight for simple state (a boolean, a count). Shines for non-trivial flows with many state combinations.

See also: [architecture-state-machines.md](architecture-state-machines.md)

## Synthesizing Choices

**Minimal app (todo, calculator)**: useState + useReducer. Overkill to introduce global state.

**Medium app (dashboard, SPA)**: Zustand or Jotai for client state + React Query for server. Simple, effective, few decisions.

**Complex app (marketplace, collaboration tool)**: Consider Redux or Zustand + React Query + XState for critical flows. Or pure atomic (Jotai) with atoms split across domains.

**Team scale**: Small teams prefer Zustand simplicity. Large teams may benefit from Redux structure (discipline, conventions). Very large apps sometimes justify service-oriented state (GraphQL clients like Apollo, Relay).

**TypeScript teams**: Zustand and React Hook Form have excellent TypeScript support out-of-the-box. Redux and Jotai require slight extra ceremony.

The trend since 2020: Redux → Zustand/Jotai (simplicity); Prop drilling → React Query (separation); Context API → Atoms (fine-grained). The ecosystem has stabilized on "pick one for client state, one for server state, compose them."

## References

- Redux docs: https://redux.js.org/
- Jotai: https://jotai.org/
- Recoil: https://recoiljs.org/
- Zustand: https://github.com/pmndrs/zustand
- Valtio: https://github.com/pmndrs/valtio
- XState: https://xstate.js.org/
- React Query: https://tanstack.com/query
- SWR: https://swr.vercel.app/