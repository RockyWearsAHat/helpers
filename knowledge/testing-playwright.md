# Testing: Playwright

## Overview

Playwright is a modern end-to-end testing framework for web applications that supports Chromium, Firefox, and WebKit. It runs on Windows, Linux, and macOS with native mobile emulation (Chrome/Android and Safari/iOS), both locally and in CI. Playwright bundles the test runner, assertions, isolation, parallelization, and rich debugging tooling into a cohesive framework designed for testing modern web apps at scale.

Key design principles: eliminate flakiness through auto-waiting, provide thorough debugging and introspection, support multiple browsers and contexts in parallel, and offer codegen tooling for rapid test development.

## Locators and Auto-Wait

Playwright's locator model is its architectural anchor. Every locator performs live DOM queries at action time—not at creation time. This means if the DOM changes between actions, the correct element is targeted.

**Recommended locator strategies** prioritize user-facing and explicit contracts:

- `page.getByRole()` — Locate by accessibility role and name (ARIA attributes). Most resilient; reflects how users interact with the app.
- `page.getByText()` — Locate by visible text content.
- `page.getByLabel()` — Locate form controls by associated label text.
- `page.getByPlaceholder()` — Locate inputs by placeholder attribute.
- `page.getByAltText()` — Locate images and elements by alt text.
- `page.getByTitle()` — Locate by title attribute.
- `page.getByTestId()` — Locate by data-testid (or custom attribute).

Each locator encapsulates built-in retry-ability and auto-waiting: Playwright waits up to the configured timeout (default 30s) for the element to appear and be actionable before attempting an action. No explicit `wait()` or `sleep()` calls needed; "flake by timeout" is extremely rare.

## Network Interception

Playwright intercepts and stubs network requests, enabling controlled testing of edge cases without involving the backend.

```javascript
// Mock/stub network responses
await page.route('**/api/users', async route => {
  await route.abort();  // Block the request
  // Or fulfill with custom response:
  // await route.fulfill({ status: 200, body: JSON.stringify({}) });
});

// Inspect and potentially modify requests
await page.route('**/api/**', route => console.log(route.request().url()));
```

Routes are applied to all same-origin and cross-origin requests. Order matters: the first matching route handler is invoked. Unhandled requests pass through normally.

## Browser and Context Isolation

Playwright supports running tests across multiple browsers in parallel. Each test is fully isolated: a new context (like an incognito window) and page are created fresh. This prevents state leakage between tests and enables realistic browser-specific behavior testing.

**Multi-browser configuration** in `playwright.config.ts`:

```typescript
export default defineConfig({
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
    { name: 'Mobile Chrome', use: { ...devices['Pixel 5'] } },
    { name: 'Mobile Safari', use: { ...devices['iPhone 13'] } },
  ],
});
```

Tests run in parallel by default across projects, and within a project across CPU cores.

## Codegen and Test Generation

Codegen generates test scripts by recording user interactions in the browser. Running `npx playwright codegen [URL]` opens a browser and records clicks, types, navigations, and assertions as Playwright code.

Generated code uses `page.getByRole()` and other recommended locators. The output serves as a starting point; manual refinement is often needed for reusability (extracting helpers, parameterizing inputs, adding assertions).

Codegen accelerates initial test creation but doesn't replace thoughtful test design—the generated tests are literal recordings, not optimized abstractions.

## Trace Viewer and Debugging

Playwright records traces during test execution: DOM snapshots, network activity, console messages, and screenshots at each step. Traces are **essential for diagnosing flakes and failures** in CI.

Enable trace recording in config:

```typescript
use: {
  trace: 'on-first-retry',  // or 'on', 'off'
}
```

Open a trace with:

```bash
npx playwright show-trace trace.zip
```

The trace viewer provides time-travel debugging: hover over commands in the step log to see the page state, inspect network requests, view console logs, and download snapshots.

In CI, traces are invaluable for reproducing failures locally that may not repro in development. Upload traces as artifacts for investigation.

## Assertions and Web-First Assertions

Playwright includes a first-class assertions library designed for the async nature of web testing:

```javascript
// Waits up to 5s for the condition to be true
await expect(page.getByText('Welcome')).toBeVisible();
await expect(page).toHaveTitle('My App');
await expect(page.locator('#counter')).toHaveCount(1);
```

Assertions automatically retry. If an element is initially hidden but becomes visible within the timeout, the assertion passes. This eliminates manual waits and makes tests resilient to minor timing variations.

Custom assertions can be built by extending `expect`.

## Visual Comparison and Screenshots

Playwright can perform visual regression testing via `toHaveScreenshot()`:

```javascript
await expect(page).toHaveScreenshot('homepage.png');
```

On first run, a baseline screenshot is created. On subsequent runs, new screenshots are compared pixel-by-pixel. Differences are highlighted. This catches unintended visual changes automatically.

Mobile emulation is fully supported, enabling visual testing across device sizes without hardware.

## Parallel Execution and Sharding

By default, Playwright runs tests in parallel across CPU cores within each browser project. Test files are distributed: if 8 cores available and 4 test files, each file runs on a separate core.

For distributed testing across many machines (CI sharding):

```bash
npx playwright test --shard=1/4   # Run 1st quarter of tests
npx playwright test --shard=2/4   # Run 2nd quarter, etc.
```

Each shard gets a unique subset of tests. CI must coordinate shard count and collection of results. Playwright's HTML reporter aggregates results across shards.

## Configuration and CI Integration

`playwright.config.ts` centralizes all configuration: target browsers, timeouts, retries, projects, reporters, base URL, and more.

Common CI settings:

```typescript
export default defineConfig({
  use: {
    baseURL: 'http://localhost:3000',
    trace: 'on-first-retry',
  },
  webServer: {
    command: 'npm run dev',
    port: 3000,
    reuseExistingServer: !process.env.CI,
  },
});
```

For CI, set `CI=true` to run headless, disable retries if desired, and upload artifacts (traces, HTML reports).

## Page Object Model and Test Structure

While not mandated, the Page Object Model (POM) pattern pairs well with Playwright:

```typescript
export class LoginPage {
  constructor(readonly page: Page) {}
  
  async login(email: string, password: string) {
    await this.page.getByLabel('Email').fill(email);
    await this.page.getByLabel('Password').fill(password);
    await this.page.getByRole('button', { name: 'Sign in' }).click();
  }
  
  async expectWelcome() {
    await expect(this.page.getByText('Welcome')).toBeVisible();
  }
}
```

POMs encapsulate page structure and common interactions, making tests more readable and maintainable. They reduce brittle selectors scattered across test files.

## Tradeoffs and Limitations

**Strengths**: Excellent auto-wait behavior, multi-browser support out of box, fast and consistent (no Selenium), trace viewer, modern testing APIs.

**Weaknesses**: Running in Node.js (not the actual browser) means the test code is slightly decoupled from the app's runtime context—though network interception and DOM access mitigate this. Mobile emulation is not truly native (emulation on desktop browser, not real device). Requires a running application or `webServer` config to start one.

## See Also

- [web-testing-frameworks.md](web-testing-frameworks.md) — Comparison across Jest, Vitest, Cypress, and Testing Library
- [testing-visual-regression.md](testing-visual-regression.md) — Visual testing strategies
- [testing-integration-e2e.md](testing-integration-e2e.md) — E2E testing concepts