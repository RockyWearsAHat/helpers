# Angular

## Core Architecture

Angular is a full-framework platform built on TypeScript with opinionated structure: dependency injection, change detection, a module system (legacy), and a component model.

Since Angular 17+, the recommended approach is **standalone components** — no NgModules required.

```typescript
@Component({
  selector: "app-dashboard",
  standalone: true,
  imports: [CommonModule, RouterOutlet, HeaderComponent],
  template: `<app-header /><router-outlet />`,
})
export class DashboardComponent {}
```

### Bootstrapping (standalone)

```typescript
// main.ts
bootstrapApplication(AppComponent, {
  providers: [
    provideRouter(routes),
    provideHttpClient(withInterceptors([authInterceptor])),
    provideAnimations(),
  ],
});
```

## Signals (Angular 16+)

Signals are Angular's reactive primitive, replacing much of RxJS for synchronous state:

```typescript
import { signal, computed, effect } from "@angular/core";

const count = signal(0);
const doubled = computed(() => count() * 2); // derived, lazy, cached

count.set(5);
count.update((v) => v + 1);

// Side effects
effect(() => {
  console.log(`Count: ${count()}`); // re-runs when count changes
});
```

### Signal-based components

```typescript
@Component({
  /* ... */
})
export class UserProfile {
  // Input signals (Angular 17.1+)
  name = input.required<string>();
  age = input(0); // with default

  // Output
  saved = output<User>();

  // Model (two-way binding)
  selected = model(false);

  // Computed from inputs
  greeting = computed(() => `Hello, ${this.name()}`);
}
```

```html
<user-profile
  [name]="userName"
  [(selected)]="isSelected"
  (saved)="onSave($event)"
/>
```

## Control Flow (Angular 17+)

Built-in template syntax replacing `*ngIf`, `*ngFor`, `*ngSwitch`:

```html
@if (user()) {
<h1>{{ user().name }}</h1>
} @else if (loading()) {
<spinner />
} @else {
<p>No user found</p>
} @for (item of items(); track item.id) {
<div>{{ item.name }}</div>
} @empty {
<p>No items</p>
} @switch (status()) { @case ('active') { <badge color="green" /> } @case
('inactive') { <badge color="gray" /> } @default { <badge color="yellow" /> } }
@defer (on viewport) {
<heavy-chart />
} @placeholder {
<div>Chart loading area</div>
} @loading (minimum 500ms) {
<spinner />
}
```

`@defer` lazily loads component code — the chunk isn't even downloaded until the trigger fires (viewport, idle, hover, timer, interaction).

## Dependency Injection

Angular's DI is hierarchical. Providers can be scoped to the root injector, a component, or a route.

```typescript
// Service
@Injectable({ providedIn: "root" }) // singleton
export class AuthService {
  private http = inject(HttpClient);
  // ...
}

// Component-scoped
@Component({
  providers: [LoggerService], // new instance per component
})
export class Widget {}

// Injection tokens for non-class dependencies
const API_URL = new InjectionToken<string>("API_URL");
// provide: { provide: API_URL, useValue: 'https://api.example.com' }
// inject: inject(API_URL)
```

### inject() vs constructor injection

```typescript
// Modern (standalone-friendly)
export class MyComponent {
  private auth = inject(AuthService);
  private router = inject(Router);
}

// Legacy (still works)
export class MyComponent {
  constructor(
    private auth: AuthService,
    private router: Router,
  ) {}
}
```

## Change Detection

### Zone.js (default)

Zone.js monkey-patches all async APIs (setTimeout, Promise, addEventListener, XHR). After any async operation completes, Angular runs change detection from the root component down.

**OnPush strategy** limits checking to components whose inputs changed (by reference), signals updated, or events emitted from within:

```typescript
@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
})
```

### Zoneless (experimental, Angular 18+)

```typescript
bootstrapApplication(AppComponent, {
  providers: [provideExperimentalZonelessChangeDetection()],
});
```

Zoneless relies entirely on signals and explicit `markForCheck()` — no Zone.js overhead, better performance, simpler mental model.

## RxJS Integration

Angular's `HttpClient`, `Router`, forms, and many APIs return Observables:

```typescript
@Component({
  /* ... */
})
export class SearchComponent {
  private http = inject(HttpClient);
  private destroy$ = new Subject<void>();

  searchTerm = signal("");

  results$ = toObservable(this.searchTerm).pipe(
    debounceTime(300),
    distinctUntilChanged(),
    switchMap((term) => this.http.get<Result[]>(`/api/search?q=${term}`)),
    takeUntilDestroyed(), // Angular 16+ — auto unsubscribe
  );

  // Convert observable back to signal for template
  results = toSignal(this.results$, { initialValue: [] });
}
```

Key operators for Angular: `switchMap` (cancel previous), `takeUntilDestroyed` (auto cleanup), `shareReplay` (cache), `combineLatest`, `withLatestFrom`.

## Routing

```typescript
const routes: Routes = [
  { path: "", component: HomeComponent },
  {
    path: "admin",
    canActivate: [() => inject(AuthService).isAdmin()],
    loadChildren: () => import("./admin/routes").then((m) => m.ADMIN_ROUTES),
  },
  {
    path: "user/:id",
    component: UserComponent,
    resolve: { user: userResolver },
  },
  { path: "**", component: NotFoundComponent },
];
```

### Functional guards and resolvers (Angular 15+)

```typescript
export const authGuard: CanActivateFn = (route, state) => {
  const auth = inject(AuthService);
  return auth.isLoggedIn() || inject(Router).createUrlTree(["/login"]);
};

export const userResolver: ResolveFn<User> = (route) => {
  return inject(UserService).getById(route.paramMap.get("id")!);
};
```

## Forms

### Reactive Forms

```typescript
@Component({ imports: [ReactiveFormsModule] })
export class ProfileForm {
  private fb = inject(FormBuilder);

  form = this.fb.group({
    name: ["", [Validators.required, Validators.minLength(2)]],
    email: ["", [Validators.required, Validators.email]],
    addresses: this.fb.array([this.createAddress()]),
  });

  createAddress() {
    return this.fb.group({ street: [""], city: [""], zip: [""] });
  }

  onSubmit() {
    if (this.form.valid) {
      console.log(this.form.value); // typed object
    }
  }
}
```

### Template-driven Forms (simpler)

```html
<form #f="ngForm" (ngSubmit)="save(f.value)">
  <input name="email" ngModel required email />
  <div *ngIf="f.controls['email']?.errors?.['email']">Invalid email</div>
</form>
```

## Directives and Pipes

```typescript
// Attribute directive
@Directive({ selector: "[appHighlight]", standalone: true })
export class HighlightDirective {
  color = input("yellow", { alias: "appHighlight" });
  private el = inject(ElementRef);

  constructor() {
    effect(() => {
      this.el.nativeElement.style.backgroundColor = this.color();
    });
  }
}

// Pipe
@Pipe({ name: "fileSize", standalone: true })
export class FileSizePipe implements PipeTransform {
  transform(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1048576).toFixed(1)} MB`;
  }
}
```

## HTTP and Interceptors

```typescript
// Functional interceptor (Angular 15+)
export const authInterceptor: HttpInterceptorFn = (req, next) => {
  const token = inject(AuthService).getToken();
  if (token) {
    req = req.clone({ setHeaders: { Authorization: `Bearer ${token}` } });
  }
  return next(req).pipe(
    catchError((err) => {
      if (err.status === 401) inject(Router).navigate(["/login"]);
      return throwError(() => err);
    }),
  );
};
```

## Testing

```typescript
describe("UserComponent", () => {
  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [UserComponent],
      providers: [
        { provide: UserService, useValue: { getUser: () => of(mockUser) } },
      ],
    });
  });

  it("should display user name", () => {
    const fixture = TestBed.createComponent(UserComponent);
    fixture.detectChanges();
    expect(fixture.nativeElement.textContent).toContain("Alice");
  });
});
```

## SSR with Angular Universal

Angular 17+ uses `@angular/ssr` with hydration:

```typescript
// server.ts
const app = express();
const ssrApp = await CommonEngine.render({
  bootstrap: AppServerModule,
  document: indexHtml,
  url: req.url,
});
```

**Hydration** (default in Angular 17+): the server-rendered HTML is preserved and Angular attaches event listeners to existing DOM nodes without destroying and re-creating them.

## NgRx (State Management)

```typescript
// Store slice
export const counterFeature = createFeature({
  name: "counter",
  reducer: createReducer(
    { count: 0 },
    on(increment, (state) => ({ count: state.count + 1 })),
    on(decrement, (state) => ({ count: state.count - 1 })),
  ),
});

// Component
export class CounterComponent {
  count = this.store.selectSignal(counterFeature.selectCount);
  constructor(private store: Store) {}
  increment() {
    this.store.dispatch(increment());
  }
}
```

## Angular CLI

```bash
ng new my-app --standalone --style=scss --routing
ng generate component features/dashboard --standalone
ng generate service core/auth
ng build --configuration=production  # tree-shaking, AOT, budgets
ng serve --open
ng test --watch
ng update @angular/core @angular/cli  # guided migration
```
