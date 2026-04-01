# Angular Modern: Signals, Zoneless, and Reactive Architecture

## Overview

Angular 16+ introduced **signals** — a lightweight, reactive primitive that replaces RxJS observables for local state. Combined with zoneless change detection (Angular 18+), standalone components, and new template syntax, modern Angular enables simpler, faster, more declarative applications.

The shift: from imperative subscription management and zone.js patching to composable, type-safe reactivity that feels familiar to developers migrating from React, Svelte, or Vue.

## Signals — The Reactive Primitive

A signal is a wrapper around a value that tracks reads and writes. When a signal changes, components depending on it re-render automatically.

```typescript
import { signal, computed, effect } from '@angular/core';

const count = signal(0);
const doubled = computed(() => count() * 2);

count.set(5);      // Update via set()
count.update(v => v + 1); // Update via function

// Every signal is callable — () reads the current value
console.log(count()); // 5
console.log(doubled()); // 10
```

### Signal Properties

— **Writable signals** (`signal()`) — Hold mutable state
— **Computed signals** (`computed()`) — Derived values (lazy, cached, read-only)
— **Effect signals** (`effect()`) — Side effects triggered by reactive changes

### Computed Signals

Computed signals are lazy — they only recompute when read AND their dependencies changed:

```typescript
const firstName = signal('Alice');
const lastName = signal('Smith');

const fullName = computed(() => {
  console.log('Computing full name...');
  return `${firstName()} ${lastName()}`;
});

// Logs: "Computing full name..."
console.log(fullName());

// No recompute — fullName was not read
firstName.set('Bob');

// Logs: "Computing full name..." (now firstName changed and fullName is read)
console.log(fullName());

// No recompute — fullName was not read since the last computation
firstName.set('Charlie');
```

Computed signals are **memoized** — if dependencies haven't changed, the function isn't re-run even if the signal is read again.

### Effect — Side Effects

Effects run automatically when their reactive dependencies change:

```typescript
const count = signal(0);

effect(() => {
  console.log(`Count is now: ${count()}`);
  document.title = `${count()}`;
});

count.set(1); // Logs: "Count is now: 1", updates document.title
count.set(2); // Logs: "Count is now: 2", updates document.title
```

Effects track dependencies automatically (fine-grained). They run after rendering, so DOM is stable. Avoid setting signals inside effects — use computed instead.

Effects can return a cleanup function that runs before re-execution:

```typescript
effect(() => {
  const subscription = myObservable.subscribe(value => console.log(value));
  return () => subscription.unsubscribe(); // Cleanup
});
```

## Signal Inputs and Outputs

Angular 17.1+ introduced signal-based components via `input()`, `output()`, and `model()` — replacing `@Input()` and `@Output()`:

```typescript
import { Component, input, output, model } from '@angular/core';

@Component({
  selector: 'app-counter',
  template: `
    <p>Count: {{ count() }}</p>
    <button (click)="increment()">+</button>
  `,
})
export class CounterComponent {
  // Required input signal
  initialValue = input.required<number>();

  // Optional input with default
  step = input(1);

  // Output signal
  countChanged = output<number>();

  // Two-way bindable signal
  selected = model(false);

  // Derived from input
  description = computed(() =>
    `Starting at ${this.initialValue()}`
  );

  increment() {
    this.localCount += this.step();
    this.countChanged.emit(this.localCount);
  }
}
```

### Advantages over `@Input()` / `@Output()`

— **Type-safe by default** — `input.required()` enforces required props at compile time
— **Reactive** — Child component automatically re-renders when input changes
— **Composable** — Signals can be passed to computed/effects directly
— **Lighter** — No property decorators needed

### Parent Component Usage

```typescript
export class AppComponent {
  counter = signal(0);

  incrementCounter() {
    this.counter.update(v => v + 1);
  }
}
```

```html
<app-counter
  [initialValue]="counter()"
  [step]="2"
  [(selected)]="isSelected"
  (countChanged)="incrementCounter()"
/>
```

## inject() Function

Modern Angular uses `inject()` instead of constructor injection — cleaner, more flexible:

```typescript
import { inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';

@Component({
  selector: 'app-user-profile',
})
export class UserProfileComponent {
  // Instead of constructor injection
  private http = inject(HttpClient);
  private router = inject(Router);
  private activatedRoute = inject(ActivatedRoute);

  // Hydration tokens for SSR
  private transferState = inject(TransferStateService);

  ngOnInit() {
    // Use injected services
    this.http.get('/api/user').subscribe(user => {
      // ...
    });
  }
}
```

`inject()` works in:
— Components
— Directives
— Pipes
— Services
— Route guards
— HTTP interceptors

It's **guard-compatible** — if called outside an injection context, it throws immediately (no silent failures).

## Change Detection: OnPush and Zoneless

### OnPush Strategy

By default, Angular runs change detection whenever the zone detects an event. With OnPush, change detection runs *only* when:
— An input signal/property changes
— An event fires from the component's template
— An output is emitted
— Async pipe emits a new value

```typescript
@Component({
  selector: 'app-profile',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <h1>{{ user().name }}</h1>
    <button (click)="refresh()">Refresh</button>
  `,
})
export class UserProfileComponent {
  user = input.required<User>();

  refresh() {
    // Only this component checks for changes
  }
}
```

OnPush is a performance optimization — it prevents unnecessary checks in large trees.

### Zoneless Mode (Angular 18+)

Zoneless disables zone.js entirely and relies on signals and event tracking:

```typescript
// main.ts
bootstrapApplication(AppComponent, {
  providers: [
    provideZoneChangeDetection({ eventCoalescing: true }),
  ],
});
```

**Zoneless benefits:**
— Smaller bundle (no zone.js wrapping all async)
— Fewer change detection cycles
— Exact tracking of what changed (no "check everything")
— Works seamlessly with signals

**Zoneless constraints:**
— RxJS observables don't trigger change detection automatically (need async pipe or toSignal)
— Third-party libraries that rely on zone patching may break
— Most Angular apps work fine — signals are the future

## Standalone Components

Standalone components declare their own imports — no NgModule wrapper:

```typescript
import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [CommonModule, RouterOutlet, HeaderComponent, SidebarComponent],
  template: `
    <app-header />
    <router-outlet />
    <app-sidebar />
  `,
})
export class AppComponent {}
```

Bootstrap a standalone app:

```typescript
// main.ts
import { bootstrapApplication } from '@angular/platform-browser';

bootstrapApplication(AppComponent, {
  providers: [
    provideRouter(routes),
    provideHttpClient(),
    // More providers...
  ],
});
```

## Control Flow and New Template Syntax

Angular 17+ introduced built-in control flow that replaces `*ngIf`, `*ngFor`, `*ngSwitch`:

```html
@if (user()) {
  <h1>{{ user().name }}</h1>
} @else if (loading()) {
  <p>Loading...</p>
} @else {
  <p>Not found</p>
}

@for (item of items(); track item.id) {
  <div>{{ item.title }}</div>
}
@empty {
  <p>No items</p>
}

@switch (status()) {
  @case ('active') {
    <badge color="green">Active</badge>
  }
  @case ('inactive') {
    <badge color="gray">Inactive</badge>
  }
  @default {
    <badge color="yellow">Unknown</badge>
  }
}
```

**Key differences from structural directives:**
— Simpler syntax (no `*` or brackets)
— Better TypeScript checking
— Smaller compiled output
— No provider registration needed

## @defer Blocks (Lazy Loading)

The `@defer` block defers component loading until a trigger fires:

```html
@defer (on viewport) {
  <app-heavy-chart />
} @placeholder {
  <div class="chart-placeholder">Chart will load when visible</div>
} @loading (minimum 500ms) {
  <spinner />
} @error {
  <p>Failed to load chart</p>
}
```

**Triggers:**
— `on viewport` — Load when element enters viewport
— `on idle` — Load during browser idle time
— `on interaction` — Load on click/focus
— `on hover` — Load on mouse hover
— `on timer(N)` — Load after N milliseconds
— `when condition` — Load when expr becomes true

Deferred components are:
— Loaded in separate chunks
— Lazy-hydrated on server
— Skipped entirely if trigger never fires

## Signal-based Data Fetching (RxJS Interop)

Signals integrate with RxJS via `toSignal()`:

```typescript
import { toSignal, toObservable } from '@angular/core/rxjs-interop';

@Component({})
export class UserComponent {
  userId = input.required<string>();
  private http = inject(HttpClient);

  // Convert observable to signal (with loader state)
  userData = toSignal(
    this.http.get(`/api/users/${this.userId()}`),
    { initialValue: null }
  );

  // Convert signal back to observable
  userChanged$ = toObservable(this.userId);
}
```

This allows gradual migration from RxJS-heavy patterns to signals without rewriting everything at once.

## Server-Side Rendering (SSR) with Hydration

Angular 16+ includes hydration — the browser's initial render matches the server's without re-running setup:

```typescript
// main.ts
import { provideClientHydration } from '@angular/platform-browser';

bootstrapApplication(AppComponent, {
  providers: [
    provideClientHydration(),
    // ...
  ],
});
```

**Signals work well with hydration:**
— Initial state flows from server via transfer state
— Signals become reactive immediately on the client
— No re-fetch of data on browser load

## Signal-Driven Architecture Pattern

A complete example combining signals, inputs, effects, and async logic:

```typescript
@Component({
  selector: 'app-user-dashboard',
  standalone: true,
  template: `
    <h1>{{ user()?.name }}</h1>
    <button (click)="loadUser()">Refresh</button>
    {{ userStatus() }}
  `,
})
export class UserDashboardComponent {
  userId = input.required<string>();
  private http = inject(HttpClient);

  // State signals
  user = signal<User | null>(null);
  loading = signal(false);
  error = signal<string | null>(null);

  // Computed status
  userStatus = computed(() => {
    if (this.loading()) return 'Loading...';
    if (this.error()) return `Error: ${this.error()}`;
    return `Loaded: ${this.user()?.name}`;
  });

  // Auto-load when userId changes
  constructor() {
    effect(() => {
      this.loadUser();
    });
  }

  loadUser() {
    this.loading.set(true);
    this.error.set(null);

    this.http
      .get<User>(`/api/users/${this.userId()}`)
      .subscribe({
        next: (u) => {
          this.user.set(u);
          this.loading.set(false);
        },
        error: (e) => {
          this.error.set(e.message);
          this.loading.set(false);
        },
      });
  }
}
```

## Performance Characteristics

— **Signals** are synchronous and cache-aware. Reading a signal multiple times in a template is cheap.
— **Zoneless** eliminates zone.js patching overhead — especially beneficial on slower devices.
— **OnPush + Signals** — Components only check when deps change, no "change all" sweeps.
— **SSR + Hydration** — No refetch penalty; signals initialize from transferred state.
— **Defer blocks** — Unbundle heavy components; chunks load on demand.

## Migration Path from RxJS

Signals don't replace RxJS — they coexist. A typical migration:

1. **Local state** — Replace `BehaviorSubject` + `.subscribe()` with `signal()`
2. **Derived values** — Replace `map()` chains with `computed()`
3. **Side effects** — Replace `.subscribe()` with `effect()`
4. **Async data** — Use `toSignal()` to wrap HTTP responses
5. **Complex async flows** — Keep RxJS; pipe observables to signals at component boundaries

Most apps benefit from **signals for local state** and **RxJS for server communication** — the best of both worlds.

## Key Takeaways

— Signals are lightweight, reactive, and type-safe
— `computed()` and `effect()` compose reactivity naturally
— Signal inputs/outputs replace decorators elegantly
— Zoneless + OnPush maximize performance
— Control flow syntax is simpler and more readable
— `@defer` enables granular code splitting
— Standalone components simplify dependency management
— Signals integrate with RxJS for gradual migration