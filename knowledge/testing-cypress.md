# Testing: Cypress

## Overview

Cypress is a front-end testing framework purpose-built for the modern web. Unlike traditional Selenium-based tools, Cypress runs in the same process as the application, giving it direct access to the DOM, window, and network layer. This architecture enables deterministic, fast, and minimal-flake tests with powerful debugging capabilities.

Cypress supports end-to-end testing, component testing (React, Vue, Angular, Svelte), and accessibility testing. It ships with a test runner (Cypress App), a dashboard service (Cypress Cloud) for CI parallelization and result analytics, and optional UI Coverage and Accessibility plugins.

## Command Chaining and Fluent API

Cypress organizes test logic as a chain of commands. Each command queues an action; commands do not execute immediately.

```javascript
cy.visit('https://example.com')
  .get('[data-cy=username]')
  .type('john')
  .get('[data-cy=password]')
  .type('secret')
  .get('[data-cy=submit]')
  .click();
```

This fluent style reads like a narrative of user actions. Cypress internally manages the queue, resolves subjects from one command to the next, and provides implicit waiting. The deferred execution model enables Cypress to retroactively replay steps and provide time-travel debugging.

## Automatic Waiting and Retry-Ability

Cypress does not race against the app; it waits for elements, network responses, and assertions automatically.

```javascript
cy.get('#element')  // Waits up to 4s for element to exist
  .should('be.visible');  // Waits up to 5s for element to be visible
```

The default timeout is 4 seconds for queries (`get`, `intercept`, etc.) and 5 seconds for assertions. Timeouts are configurable globally or per-command. When an element does not appear within the timeout, Cypress takes a screenshot and provides a readable error.

**Retry-ability** is key: if an assertion fails, Cypress re-runs the preceding query and assertion in a loop until timeout. This handles transient timing issues without explicit waits.

```javascript
cy.contains('Welcome')  // Retries until text appears
  .should('be.visible');  // Retries until visible
```

This diverges from Selenium, where you must explicitly wait. Cypress's default behavior eliminates most timing flakes.

## Time Travel and Command Log

As tests run, Cypress records each command and its result. The Command Log in the Cypress App shows all commands in order. Hovering over a command in the log shows a**snapshot of the app at that moment**—DOM, styles, network state—allowing inspection without re-running the test.

This "time travel debugging" drastically reduces debugging time. You can inspect the page state at any command, inspect console errors, and see network requests without leaving the IDE.

Clicking a command highlights the targeted element (if relevant) in the snapshot.

## Network Stubbing and `cy.intercept()`

Cypress intercepts HTTP requests and can stub, spy on, or modify responses.

```javascript
cy.intercept('GET', '/api/users', { fixture: 'users.json' });
// All GET requests to /api/users return fixture data

cy.intercept('POST', '/api/login', (req) => {
  req.reply({ statusCode: 401 });  // Deny login
});

// Spy on requests without modifying
cy.intercept('GET', '/api/config', { middleware: true }, (req) => {
  console.log('Config fetched:', req.response);
});
```

Network stubbing allows testing error conditions, race conditions, slow networks, and offline scenarios without backend cooperation. Patterns like URL matching and middleware hooks provide fine-grained control.

## Component Testing

Cypress Component Testing mounts and tests individual components isolated from the full application.

```javascript
// fixtures/button.cy.jsx
import Button from './Button';

it('renders with custom text', () => {
  cy.mount(<Button>Click me</Button>);
  cy.get('button').should('contain', 'Click me');
});
```

Components can be tested with various frameworks (React, Vue, Angular, Svelte). Testing is fast—no full app startup. State and props are easily manipulated. This bridges the gap between unit testing and e2e testing.

## Spies, Stubs, and Clocks

Cypress integrates Sinon for spying on functions, stubbing methods, and controlling time.

```javascript
cy.window().then(win => {
  cy.spy(win.analytics, 'track');
}).then(() => {
  // Interact with app
  cy.get('button').click();
}).then(() => {
  cy.window().then(win => {
    expect(win.analytics.track).to.be.calledWith('click');
  });
});

// Stub system time
cy.clock();
cy.get('[data-cy=timer]').click();
cy.tick(1000);  // Advance time 1s
cy.contains('1 second ago').should('exist');
```

Spies verify function calls and arguments. Stubs replace implementations. Clock control tests timers and delays deterministically.

## Cypress Studio and AI-Assisted Testing

**Cypress Studio** records user interactions and generates code:

1. Open the Cypress App (`npx cypress open`).
2. Click "Add Commands" in the step view.
3. Interact with the app; Studio records actions.
4. Studio generates Cypress commands.

**Studio AI** can recommend assertions automatically based on the recorded state, reducing boilerplate.

This accelerates test writing for developers less familiar with Cypress syntax or for quick regression coverage.

## Cypress Cloud and Parallelization

Cypress Cloud is a dashboard service that records test runs, provides analytics, and enables parallelization.

After recording to Cypress Cloud, tests can be distributed across multiple machines with proper orchestration:

```bash
npx cypress run --record --parallel
# CI agent 1/3
npx cypress run --record --parallel --ci-build-id abc-123
# CI agent 2/3
npx cypress run --record --parallel --ci-build-id abc-123
# CI agent 3/3
npx cypress run --record --parallel --ci-build-id abc-123
```

Cypress Cloud orchestrates spec distribution so that each machine receives a proportion of the test suite.

**Smart Orchestration** features:

- **Spec Prioritization**: Re-run previously-failing specs first.
- **Auto Cancellation**: Stop the test run on first failure for tight feedback.
- **Flake Detection**: Identify unreliable tests across runs.
- **Branch Review**: Compare test results between branches to prevent regressions.

## Plugins and Custom Commands

Extend Cypress with plugins for additional capabilities.

```javascript
// support/e2e.js
Cypress.Commands.add('login', (email, password) => {
  cy.visit('/login');
  cy.get('[data-cy=email]').type(email);
  cy.get('[data-cy=password]').type(password);
  cy.get('[data-cy=submit]').click();
  cy.get('main').should('be.visible');  // Wait for redirect
});

// usage
cy.login('user@example.com', 'password');
```

Custom commands encapsulate common workflows. They receive the subject from the previous command and can return a new subject for chaining.

Plugins (via `cypress.config.js`) can hook into events:

```javascript
on('before:browser:launch', (browser, launchOptions) => {
  // Control how the browser starts
});
on('task', {
  async queryDb(sql) { /* ... */ }
});
```

## Configuration and CI Setup

`cypress.config.js` or `.config.ts` centralizes settings:

```typescript
import { defineConfig } from 'cypress';

export default defineConfig({
  e2e: {
    baseUrl: 'http://localhost:3000',
    viewportWidth: 1280,
    viewportHeight: 720,
    requestTimeout: 5000,
    responseTimeout: 5000,
    defaultCommandTimeout: 4000,
  },
  component: {
    devServer: { framework: 'react', bundler: 'vite' },
  },
});
```

For CI, set `CI=true`, use `--headless` by default, configure video recording, and upload artifacts for debugging.

## Limitations and Tradeoffs

**Strengths**: Excellent debugging (time travel, command log, screenshots), automatic waiting, powerful network stubbing, fast-executing, deterministic—minimal flakiness due to in-process architecture.

**Weaknesses**: Runs only in Chromium-family and Firefox browsers (not Safari, IE). Cannot test multi-tab scenarios well (Cypress controls a single tab). Cross-domain navigation and same-origin policy restrictions can complicate testing. Test code runs in the browser context, so syntax must be browser-compatible (though JavaScript is universal here). Parallelization requires Cypress Cloud (paid).

## Practices

1. **Use data-testid judiciously**: `cy.get('[data-cy=element]')` is explicit but requires test IDs in production code. Prefer user-facing selectors when possible.
2. **Avoid arbitrary waits**: Let Cypress's retry-ability handle timing. Only use `cy.wait()` for spy assertions or explicit delays.
3. **Keep tests focused**: Each test should verify one feature or user flow. Long tests are harder to debug.
4. **Leverage custom commands**: Extract cross-test patterns into reusable commands for DRY tests.
5. **Record to Cloud in CI**: Use Cypress Cloud to diagnose flakes, track trends, parallelize, and integrate with VCS.

## See Also

- [web-testing-frameworks.md](web-testing-frameworks.md) — Comparison with Playwright, Jest, and others
- [testing-integration-e2e.md](testing-integration-e2e.md) — E2E and integration testing strategies
- [testing-accessibility.md](testing-accessibility.md) — Accessibility testing (Cypress has built-in support)