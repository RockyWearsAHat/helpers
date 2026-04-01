# React Testing Patterns — User Interaction, Mocking, Component Testing & Accessibility

## Overview

React testing covers component behavior, user interaction, API integration, and accessibility. The landscape fragment into tools for different purposes: React Testing Library for component testing (the dominant approach), Storybook for component development and visual regression, Playwright for component testing and E2E, MSW for API mocking.

Philosophy: Test behavior, not implementation. Assert what users see and experience, not internal state or rendering internals.

## React Testing Library: The Standard

React Testing Library emphasizes testing components from the user's perspective—queries find elements by accessible labels, not class names or element types.

### Setup & Rendering

```jsx
import { render, screen } from '@testing-library/react';

test('renders button with label', () => {
  render(<button>Click me</button>);
  
  // Find by accessible text
  const button = screen.getByRole('button', { name: /click me/i });
  expect(button).toBeInTheDocument();
});
```

### Query Hierarchy (Priority Order)

React Testing Library prioritizes accessible queries:

| Query | Find by | Preference |
| --- | --- | --- |
| `getByRole` | ARIA role + accessible name | 1st choice |
| `getByLabelText` | Associated label | For form inputs |
| `getByPlaceholderText` | Placeholder | For inputs without label |
| `getByText` | Element text | For static text |
| `getByDisplayValue` | Input/textarea value | For form state |
| `getByTestId` | `data-testid` attribute | Last resort |

```jsx
// ✅ GOOD: Accessible-first queries
screen.getByRole('button', { name: 'Submit' });
screen.getByLabelText('Email');
screen.getByPlaceholderText('Enter email...');

// ❌ AVOID: Implementation details
screen.getByTestId('submit-btn');
wrapper.find('.submit-button') // enzyme
screen.getByClassName('button');
```

### Async Queries: Handling Suspense & Loading States

Tests must wait for asynchronous updates. Use `findBy*` (waits, throws if not found) or `waitFor`.

```jsx
test('loads and displays user data', async () => {
  render(<UserProfile userId={1} />);
  
  // Suspense fallback visible immediately
  expect(screen.getByText('Loading...')).toBeInTheDocument();
  
  // Wait for the final state
  const userName = await screen.findByText('John Doe');
  expect(userName).toBeInTheDocument();
});

// Or explicit waitFor with custom conditions
test('searches for users', async () => {
  render(<UserSearch />);
  
  const input = screen.getByRole('textbox', { name: /search/i });
  fireEvent.change(input, { target: { value: 'alice' } });
  
  await waitFor(() => {
    expect(screen.getByText('alice')).toBeInTheDocument();
  });
});
```

**Common mistake**: Not awaiting async updates. Test passes locally but flakes in CI.

### User Interaction: userEvent vs fireEvent

`userEvent` simulates real user behavior (typing triggers `onChange`, `onBlur`, etc.). `fireEvent` dispatches events directly, skipping browser normalization.

```jsx
import userEvent from '@testing-library/user-event';

test('submits form on button click', async () => {
  const user = userEvent.setup();
  render(<Form onSubmit={jest.fn()} />);
  
  // Type into input (triggers onChange, onBlur, etc.)
  await user.type(screen.getByPlaceholderText('Name'), 'John');
  
  // Click button
  await user.click(screen.getByRole('button', { name: 'Submit' }));
  
  // Assert result
  expect(screen.getByText('Submitted: John')).toBeInTheDocument();
});

// Compare: fireEvent is lower-level
test('with fireEvent (less realistic)', () => {
  const { getByPlaceholderText } = render(<Form />);
  
  // Just fires the event, doesn't simulate user behavior
  fireEvent.change(getByPlaceholderText('Name'), { target: { value: 'John' } });
  fireEvent.click(screen.getByRole('button'));
});
```

**When to use**: `userEvent` for realistic interaction testing (forms, complex workflows). `fireEvent` for targeted event testing when you don't need full browser simulation.

### Component Props & Callbacks

Test that callbacks fire and props pass correctly:

```jsx
test('calls onSubmit with form data', async () => {
  const mockSubmit = jest.fn();
  render(<Form onSubmit={mockSubmit} />);
  
  const user = userEvent.setup();
  await user.type(screen.getByPlaceholderText('Email'), 'test@example.com');
  await user.click(screen.getByRole('button', { name: 'Submit' }));
  
  expect(mockSubmit).toHaveBeenCalledWith({ email: 'test@example.com' });
});
```

## API Mocking: MSW (Mock Service Worker)

MSW intercepts fetch/XMLHttpRequest at the network level, returning mock responses. Tests run without a real backend.

### Setup

```jsx
// mocks/handlers.ts
import { http, HttpResponse } from 'msw';

export const handlers = [
  http.get('/api/users/:id', ({ params }) => {
    return HttpResponse.json({ id: params.id, name: 'John Doe' });
  }),
  
  http.post('/api/users', async ({ request }) => {
    const body = await request.json();
    return HttpResponse.json({ id: 123, ...body }, { status: 201 });
  }),
];

// mocks/server.ts
import { setupServer } from 'msw/node';
import { handlers } from './handlers';

export const server = setupServer(...handlers);

// test setup
beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterAll(() => server.close());
```

### Using in Tests

```jsx
test('fetches user data', async () => {
  render(<UserProfile userId={1} />);
  
  await waitFor(() => {
    expect(screen.getByText('John Doe')).toBeInTheDocument();
  });
});

// Override handler for specific test
test('handles error on failed fetch', async () => {
  server.use(
    http.get('/api/users/:id', () => {
      return HttpResponse.json({ error: 'Not found' }, { status: 404 });
    })
  );
  
  render(<UserProfile userId={999} />);
  
  await waitFor(() => {
    expect(screen.getByText('User not found')).toBeInTheDocument();
  });
});
```

## Storybook: Component Development & Testing

Storybook is an interactive development environment for isolated component testing.

### Writing Stories

```jsx
// Button.stories.tsx
import Button from './Button';

export default {
  component: Button,
};

export const Primary = {
  args: {
    label: 'Click me',
    variant: 'primary',
  },
};

export const Disabled = {
  args: {
    label: 'Disabled',
    disabled: true,
  },
};
```

Each story is a component state. Stories let you see all variations without running the full app.

### Interaction Testing in Storybook

```jsx
import { expect } from '@storybook/test';
import { userEvent, within } from '@storybook/test';

export const Interactive = {
  args: { onSubmit: fn() },
  play: async ({ canvas }) => {
    const button = within(canvas).getByRole('button');
    await userEvent.click(button);
    await expect(button).toHaveAttribute('disabled');
  },
};
```

The `play` function runs user interactions and assertions, combining interaction testing with visual review.

### Visual Regression Testing

Storybook integrates with visual regression tools (Percy, Chromatic) to catch unintended visual changes between commits.

## Playwright Component Testing

Playwright tests React components with the same tools used for E2E testing.

### Setup

```jsx
// playwright.config.ts
import { defineConfig } from '@playwright/experimental-ct-react';

export default defineConfig({
  testDir: './tests',
  use: {
    ctPort: 3100,
  },
});

// Component test
import { test, expect } from '@playwright/experimental-ct-react';
import Button from '../src/Button';

test('renders button', async ({ mount }) => {
  const component = await mount(<Button label="Click" />);
  await expect(component).toContainText('Click');
});
```

Playwright excels at E2E testing within the browser. Component testing is less idiomatic than React Testing Library but powerful for snapshot & interaction testing.

## Testing Strategy: Component Layer Hierarchy

Different layers require different testing approaches:

### Presentational Components (Dumb)

Test rendering with various props, no API calls:

```jsx
test('renders with all prop variations', () => {
  render(<Badge variant="success" label="Active" />);
  expect(screen.getByText('Active')).toHaveClass('badge-success');
});
```

### Container Components (Smart)

Test data fetching, state management, side effects:

```jsx
test('loads and displays data', async () => {
  mockApiResponse({ posts: [{ id: 1, title: 'Hello' }] });
  render(<PostList />);
  
  await expect(screen.findByText('Hello')).resolves.toBeInTheDocument();
});
```

### Integration Tests

Test multiple components interacting:

```jsx
test('form submission flow', async () => {
  const user = userEvent.setup();
  render(<LoginFlow />);
  
  await user.type(screen.getByPlaceholderText('Email'), 'test@example.com');
  await user.type(screen.getByPlaceholderText('Password'), 'pass123');
  await user.click(screen.getByRole('button', { name: 'Login' }));
  
  await waitFor(() => {
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument(); // Modal closed
  });
});
```

## Snapshot Testing: Pitfalls & Trade-offs

Snapshots capture rendered output and flag diffs. Useful for detecting unintended changes, but easily abused.

### When Snapshots Help

Complex, stable component output where visual inspection is valuable:

```jsx
test('renders error boundary UI', () => {
  const { container } = render(
    <ErrorBoundary fallback={<div>Error occurred</div>}>
      <ThrowError />
    </ErrorBoundary>
  );
  
  expect(container).toMatchSnapshot();
});
```

### When Snapshots Fail

Snapshots become brittle when:
- **Frequent intentional changes** (snapshot updates are noisy)
- **Large output** (diffs are hard to review)
- **Dynamic content** (timestamps, IDs change every run)

The risk: developers approve snapshot updates without reviewing changes, defeating the purpose.

### Better Alternative: Specific Assertions

```jsx
// ❌ Avoid: Vague snapshot
expect(component).toMatchSnapshot();

// ✅ Prefer: Explicit assertions
expect(screen.getByRole('button')).toBeDisabled();
expect(screen.getByRole('alert')).toHaveAttribute('aria-live', 'polite');
```

## Accessibility Testing

Test that components are keyboard-navigable and accessible to assistive tech.

### Automated Checks

```jsx
import { axe, toHaveNoViolations } from 'jest-axe';

test('button is accessible', async () => {
  const { container } = render(<Button>Click me</Button>);
  
  const results = await axe(container);
  expect(results).toHaveNoViolations();
});
```

### Manual Accessibility Tests

Automated tools catch ~30% of accessibility issues. Manual testing is necessary:

- **Keyboard navigation**: Tab through interactive elements
- **Screen reader testing**: Use NVDA (Windows), JAWS, or VoiceOver (macOS)
- **Color contrast**: Ensure text passes WCAG AA (4.5:1 for normal text)
- **Focus indicators**: Visible focus ring on interactive elements

```jsx
test('dialog is keyboard accessible', async () => {
  const user = userEvent.setup();
  render(<Modal open={true} />);
  
  // Tab to close button
  await user.tab();
  expect(screen.getByRole('button', { name: /close/i })).toHaveFocus();
  
  // Closing modal traps focus
  await user.keyboard('{Escape}');
  // Focus should return to trigger button or body
});
```

## Common Testing Anti-patterns

### Testing Implementation Instead of Behavior

```jsx
// ❌ Tests internals, breaks on refactor
expect(component.state.isOpen).toBe(true);
expect(component.instance.updateState).toHaveBeenCalled();

// ✅ Tests behavior, independent of implementation
expect(screen.getByText('Modal content')).toBeInTheDocument();
```

### Over-mocking

```jsx
// ❌ Mocks props but doesn't test behavior
jest.mock('./Button', () => () => <div>Mocked</div>);

// ✅ Tests real component or strategically mocks dependencies
render(<Component />); // Real Button
// If needed: mock only network calls, not components
```

### Ignoring Edge Cases

Test error states, loading states, empty states, not just happy path:

```jsx
test('list component states', async () => {
  // Loading
  render(<UserList loading={true} />);
  expect(screen.getByText('Loading...')).toBeInTheDocument();
  
  // Empty
  render(<UserList users={[]} />);
  expect(screen.getByText('No users found')).toBeInTheDocument();
  
  // Error
  render(<UserList error="Failed to load" />);
  expect(screen.getByText('Failed to load')).toBeInTheDocument();
});
```

## Testing Philosophy

- **Test user behavior**, not implementation details
- **Avoid tightly coupled tests**—refactoring shouldn't break tests
- **Prioritize accessibility queries**—forces accessible component design
- **Mock at boundaries** (API, filesystem) not within your code
- **Test before refactoring**—tests catch regressions
- **Measure coverage** but don't chase 100%—focus on critical paths

## Tools Comparison

| Tool | Purpose | Scope |
| --- | --- | --- |
| React Testing Library | Component interaction testing (black-box) | Isolated components |
| MSW | API mocking | Integration testing |
| Storybook | Component development, visual regression | Isolated, interactive |
| jest-axe | Automated accessibility checks | A11y validation |
| Playwright | Browser automation, component + E2E | Broad |

## Related

See also: [testing-advanced-patterns.md](testing-advanced-patterns.md) (general testing patterns), [testing-accessibility.md](testing-accessibility.md) (WCAG, screen readers), [web-accessibility.md](web-accessibility.md) (accessibility principles).