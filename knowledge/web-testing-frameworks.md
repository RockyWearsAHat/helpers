# Web Testing Frameworks — Jest, Vitest, Playwright, Cypress, Testing Library & MSW

Frontend testing spans three domains: **unit & integration** (testing components and logic), **end-to-end** (simulating user workflows), and **API mocking** (controlling external dependencies). Different frameworks solve different problems.

## Jest vs Vitest: Test Runners for Components & Units

Both are JavaScript test runners: they run test files, assert expectations, and report coverage. They differ in philosophy and integration.

### Jest: The Industry Standard

Jest is configured by convention. Given a test file (`foo.test.js`), Jest:
1. Transpiles via Babel
2. Mocks the entire CommonJS module system (useful for testing Node.js code)
3. Runs the test file
4. Collects coverage, reports results

**Strengths:**
- Globals automatically injected (`test`, `expect`, `describe`, `beforeEach`)
- Excellent snapshot testing for component rendering
- Strong mock utilities (`jest.mock()`, `jest.spyOn()`)
- Works with any bundler or config (Webpack, CRA, etc.)

**Weaknesses:**
- Slow startup and test run (transforms every file via Babel)
- CommonJS-centric design feels dated (though ESM support was added)
- Configuration is often implicit, making it opaque when things break

### Vitest: The ESM-Native Alternative

Vitest is built on Vite and prioritizes **ESM-first, fast test execution**. It reuses Vite's transpilation and module resolution.

**Strengths:**
- Instant startup and hot module reloading for tests
- Same dev/test stack as Vite projects
- ESM imports work transparently
- Simpler async/await in tests (proper Promise integration)
- Can reuse Vite config

**Weaknesses:**
- Newer ecosystem (less mature debugging tooling)
- Snapshot serialization differs slightly from Jest
- Not yet universal in CI/CD pipelines (though adoption is rising)

### Choice
Jest for established projects, large teams with Jest knowledge. Vitest for new Vite-based projects or when speed matters (monorepos with thousands of tests).

## Playwright vs Cypress: End-to-End Testing

E2E tests automate a browser, click buttons, fill forms, assert on rendered output. They exercise the real system — frontend + backend — end-to-end.

### Cypress: Developer-Friendly Local Testing

Cypress is designed for developer ergonomics. Key features:
- Browser-based test runner (you watch tests run in a dedicated Electron browser)
- Time-travel debugging (step through test execution, inspect state at each step)
- Automatic waits — Cypress retries assertions until they pass (doesn't require explicit waits)
- Excellent for local development and fast feedback loops

**Architecture:** Cypress runs inside the browser context, giving it access to the DOM and window object. This means tests can access `localStorage`, mock timers, and manipulate application state directly.

**Limitations:**
- Single-tab only (can't test multi-window interactions)
- Doesn't support cross-browser testing well (Chromium, Firefox, Safari support is uneven)
- Page navigation resets the Cypress context (workarounds exist, but fundamentally limited)
- Cloud parallelization requires Cypress subscription

### Playwright: Cross-Browser, Production-Ready

Playwright (Microsoft) is built for **scale and cross-browser coverage**. It launches actual browser instances (Chromium, Firefox, Safari) and controls them via WebDriver protocol.

**Strengths:**
- Truly cross-browser (fires up real Safari, actual Firefox)
- Context isolation — separate browser contexts mimic independent users
- Parallelization is built-in and free
- No implicit waits — you control timing, reducing flakiness from timing assumptions
- Supports mobile browser emulation

**Weaknesses:**
- Less interactive debugging (no time-travel in the UI)
- More verbose API (explicit waits required)
- Requires more setup for complex scenarios

### Choice
Cypress for rapid local development and debugging. Playwright for production test suites with broad browser coverage and CI parallelization.

### Trade-Off: What E2E Tests Should Cover

E2E tests are **expensive**: slow, flaky if written poorly, slow to execute in CI. Don't E2E-test every scenario. Test critical user flows:
- Signup/login
- Purchase workflow
- Content creation and retrieval
- Error states

Unit and integration tests cover the rest (edge cases, state transitions, logic errors).

## Testing Library: The Testing Philosophy

**Testing Library** isn't a full test framework — it's a library providing utilities to test components the way users interact with them. It's used with Jest or Vitest.

Philosophy: **Test behavior, not implementation.** Bad test (depends on how component is written):
```javascript
const wrapper = render(<MyComponent />);
expect(wrapper.find('.submit-button')).toExist();
```

Good test (depends on user interaction):
```javascript
render(<MyComponent />);
const button = screen.getByRole('button', { name: /submit/i });
expect(button).toBeInTheDocument();
```

### Core Query Types

- `getBy*` — throws if not found; use when something must be present
- `queryBy*` — returns null if not found; use when asserting absence
- `findBy*` — waits for element asynchronously; use for dynamic content

Testing Library encourages **accessibility-first queries**:
- `getByRole('button', { name: 'Submit' })` — accessible to screen readers
- `getByLabelText('Email')` — works for forms
- `getByText('Welcome')` — for other content

**Avoid brittle queries:**
- Don't use `getByTestId` unless necessary (it couples tests to implementation)
- Don't query by CSS class

## Mock Service Worker (MSW): API Mocking

MSW intercepts HTTP requests (fetch and XMLHttpRequest) and returns mock responses. It works at the **network layer**, making it transparent to the application.

### How It Works

1. Define request handlers (route + response):
```javascript
import { http, HttpResponse } from 'msw';

export const handlers = [
  http.get('/api/user/:id', ({ params }) => {
    return HttpResponse.json({ id: params.id, name: 'Alice' });
  })
];
```

2. Set up MSW server in tests:
```javascript
import { setupServer } from 'msw/node';

const server = setupServer(...handlers);
beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterAll(() => server.close());
```

3. Tests use real API calls — no mocking in the component code:
```javascript
render(<UserProfile userId="1" />);
await screen.findByText('Alice');
```

### Advantages

- No need to mock API client code
- Tests exercise real request/response handling
- Shared handler definitions (dev and testing use the same mocks)

### Limitations

- Only works for HTTP requests (not WebSocket)
- Performance overhead from intercepting all requests
- Requires handlers for every endpoint touched in tests

## Component Testing Strategy

Component testing sits between unit tests and E2E. Tests a component in isolation, but exercises the rendered DOM.

Example (React + Testing Library + Vitest):
```javascript
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Button from './Button';

describe('Button', () => {
  it('calls onClick when clicked', async () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Click me</Button>);
    
    await userEvent.click(screen.getByRole('button'));
    expect(onClick).toHaveBeenCalled();
  });
});
```

**What to test:** User interactions, conditional rendering, error states, prop combinations.
**What not to test:** Implementation details (whether state is stored in Redux vs Context), exact CSS classes.

## Visual Testing & Regression Detection

Visual regression testing compares rendered output across test runs. Tools:
- **Percy** — cloud-based visual CI
- **Chromatic** — Storybook-integrated visual testing
- **Playwright Expect** — local snapshot comparison

Useful for: catching unintended CSS changes, design regressions. Trade-off: visual tests are expensive and require manual approval on first runs.

## Test Runner Architecture & Performance

Modern test runners optimize for **parallelization and reusability**:

1. **Worker pools** — Jest and Vitest split test files across multiple processes
2. **Module caching** — repeated imports from the same file reuse in-memory state (can cause test isolation issues)
3. **Dynamic imports** — ESM enables fine-grained caching and performance monitoring

Performance bottlenecks:
- **Startup time** — parsing config, loading plugins
- **Transform time** — transpiling all dependencies (especially TypeScript)
- **Wait time** — async operations in tests (timeouts, API delays)

Optimization tactics:
- Split tests into multiple suites (unit vs integration vs E2E)
- Use `--run` mode in CI (don't watch, execute once and exit)
- Parallelize across CI machines
- Cache node_modules and transpiled code between runs

## Testing Patterns & Conventions

### Arrange-Act-Assert
```javascript
// Arrange: set up state
const user = { name: 'Alice' };

// Act: perform action
const result = formatUser(user);

// Assert: verify outcome
expect(result).toBe('Alice');
```

### Test Independence
Each test must be runnable in any order. Use `beforeEach` and `afterEach` to set up/tear down state.

### Descriptive Names
```javascript
// Poor
test('works', () => { ... });

// Good
test('submits form with valid email without validation error', () => { ... });
```

### Avoid Mock Leakage
Mock handlers set in one test should not affect others. MSW and Jest both provide reset methods.

## Choice Summary

| Framework    | Best For                               | Philosophy                |
|--------------|----------------------------------------|---------------------------|
| Jest         | Unit & component tests, snapshots      | Convention-first          |
| Vitest       | Fast-feedback component testing        | ESM, integrated with Vite |
| Cypress      | Local E2E debugging & rapid iteration  | Developer-friendly UI     |
| Playwright   | Production E2E, cross-browser CI       | Scale & coverage          |
| Testing Lib  | User-behavior-focused assertions       | Test implementation-free  |
| MSW          | Network request mocking                | Network-layer interception|

Typical stack: Vitest + Testing Library for components, Playwright for E2E, MSW for API mocking across both.