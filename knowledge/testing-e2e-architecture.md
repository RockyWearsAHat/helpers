# E2E Test Architecture — Design Patterns, Frameworks, and Strategies

## Overview

End-to-end testing automates user workflows through the full application stack. Unlike unit tests (single function), E2E tests navigate UIs, trigger business logic, and verify outcomes across database updates, API calls, and rendered states. 

The challenge isn't writing individual tests—any framework can click a button and assert text. The challenge is *scaling* E2E suites: 500+ tests that remain fast, maintainable, and reliable despite UI changes, asynchronous behavior, and data-dependent scenarios.

This guide covers architectural patterns (Page Object Model, Screenplay), infrastructure (Playwright vs. Cypress vs. Selenium), and operational practices (test isolation, flaky test quarantine, visual regression, accessibility checks).

## Design Patterns

### Page Object Model (POM)

Page Object Model encapsulates UI interactions in a class-per-page abstraction. Tests work with the page's logical interface, not raw selectors.

```python
# pages/login_page.py
class LoginPage:
    def __init__(self, page):
        self.page = page
        self.email_input = page.locator('input[name="email"]')
        self.password_input = page.locator('input[name="password"]')
        self.submit_button = page.locator('button[type="submit"]')
    
    def navigate(self):
        self.page.goto('https://app.example.com/login')
    
    def login(self, email, password):
        self.email_input.fill(email)
        self.password_input.fill(password)
        self.submit_button.click()
    
    def get_error_message(self):
        return self.page.locator('.error-message').text_content()

# test_login.py
def test_invalid_login(page):
    login_page = LoginPage(page)
    login_page.navigate()
    login_page.login('invalid@email.com', 'wrong')
    assert 'Invalid credentials' in login_page.get_error_message()
```

**Advantages:**
- Decouples tests from selectors: UI changes don't break tests, only the page object.
- Reusability: multiple tests use the same login flow.
- Readability: tests read like business workflows, not clicks and assertions.

**Disadvantages:**
- Boilerplate: more code upfront.
- Temptation to over-engineer: adding methods for every possible interaction.
- Shallow: doesn't address test data, fixtures, or cross-page workflows.

### Screenplay Pattern

Screenplay expands on POM by modeling tests as *actors* performing *tasks*. An actor has *abilities* (interact with UI, call APIs, manage data) and performs *interactions* toward a *goal*.

```javascript
// Serenity/JS example (JavaScript)
import { Actor, Task, Interaction } from '@serenity/core';
import { Click, Fill, Text } from '@serenity/web';

class LogIn implements Task {
    constructor(private email: string, private password: string) {}
    
    performAs(actor: Actor): Promise<void> {
        return actor.attemptsTo(
            Navigate.to('/login'),
            Fill.in(LoginForm.EmailInput, this.email),
            Fill.in(LoginForm.PasswordInput, this.password),
            Click.on(LoginForm.SubmitButton),
            Wait.until(DashboardPage.Title, isPresent())
        );
    }
}

// Test
describe('User Authentication', () => {
    it('logs in successfully', async () => {
        const alice = new Actor('Alice');
        await alice.attemptsTo(
            new LogIn('alice@example.com', 'password123'),
            Verify.that(DashboardPage.WelcomeMessage, containsText('Welcome, Alice'))
        );
    });
});
```

**Advantages:**
- Business-focused language: non-technical stakeholders can read tests.
- Composable: tasks build from interactions, making complex workflows readable.
- Reusable abilities: the same "login ability" works across different apps.
- Better error reporting: failures describe what the actor *failed to do*, not which button didn't appear.

**Disadvantages:**
- Steeper learning curve: requires understanding actors, tasks, and abilities.
- Overhead: smaller projects may not justify the abstraction.
- Framework dependency: less portable than vanilla Playwright/Cypress code.

### Flow Model Pattern

Flow Model extends POM by organizing interactions as workflows: entry (precondition), flow (steps), and exit (verification). Particularly useful for complex multi-step processes (checkout, form wizards).

```typescript
class CheckoutFlow {
    private cart: CartPage;
    private shipping: ShippingPage;
    private payment: PaymentPage;
    private confirmation: ConfirmationPage;

    async executeCheckout(items: Item[], address: Address) {
        // Setup: items already in cart
        await this.cart.verify();
        
        // Flow: navigate stages
        await this.cart.proceedToShipping();
        await this.shipping.fill(address);
        await this.shipping.proceedToPayment();
        
        await this.payment.enterCard({ number: '4111...', cvv: '123' });
        await this.payment.submit();
        
        // Exit: verify successful completion
        return await this.confirmation.getOrderNumber();
    }
}
```

**Use:** Multi-stage workflows where the entire flow is more meaningful than individual page interactions.

## Framework Comparison: Playwright vs. Cypress vs. Selenium

### Playwright

Modern, multi-browser test framework from Microsoft. Runs in the same process as the browser, not via WebDriver protocol.

| Aspect | Details |
|--------|---------|
| **Browser Support** | Chromium, Firefox, WebKit; mobile (iPhone, Android) |
| **Architecture** | Browser DevTools Protocol (direct, not WebDriver) |
| **Speed** | Fastest: ~2-5ms per interaction; parallel by default |
| **Maintenance** | Auto-waiting: waits for elements to be actionable; fewer flaky tests |
| **API** | Intuitive, modern; locators chain naturally |
| **Test Isolation** | Browser context per test (isolation is natural) |
| **CI Integration** | Excellent: built-in artifacts, screenshots, video capture |
| **Cost Model** | Open-source (free); commercial support available |

```javascript
import { test, expect } from '@playwright/test';

test.describe('User Flow', () => {
    test('complete checkout', async ({ page, context }) => {
        // context isolates: cookies, local storage per test
        await page.goto('/checkout');
        
        // Auto-waiting: waits for element to be visible, enabled
        await page.locator('button:has-text("Add to Cart")').click();
        
        // Responsive assertions
        await expect(page.locator('.cart-count')).toContainText('1');
    });
});
```

**When to use:** New projects, fast-moving teams, Chrome-first applications, need for mobile testing.

**Weaknesses:** Less ecosystem (fewer plugins); smaller community than Selenium.

### Cypress

Specialized E2E testing framework for web applications. Runs in the same browser tab, allowing direct access to application state.

| Aspect | Details |
|--------|---------|
| **Browser Support** | Chromium, Firefox, Edge (not Safari or mobile) |
| **Architecture** | Direct browser process; shared memory with app under test |
| **Speed** | Fast for small suites (~3-5 sec); quadratic slowdown at scale (100+ tests) |
| **Debugging** | Excellent: time-travel debugging, DOM snapshots at each command |
| **API** | Intuitive for beginners; magic waiting (implicit, sometimes unpredictable) |
| **State Access** | Direct: can access `window.localStorage`, application state |
| **CI Integration** | Good; Cypress Cloud (commercial) adds recording, parallelization |
| **Cost** | Open-source; paid parallelization via Cypress Cloud |

```javascript
describe('Checkout', () => {
    it('adds item and completes purchase', () => {
        cy.visit('/products');
        cy.contains('button', 'Add to Cart').click();
        
        // Implicit waiting
        cy.get('.cart-count').should('contain', '1');
        
        // Direct state access
        cy.window().then(win => {
            expect(win.app.state.cartTotal).toBe(29.99);
        });
    });
});
```

**When to use:** Single-page applications with complex client state; teams preferring UI-first debugging; projects limited to Chromium.

**Weaknesses:** Limited to single browser tab (no multi-tab workflows); mobile testing difficult; doesn't support Safari; scaling to 500+ tests is painful (CI parallelization requires paid subscription).

### Selenium

Long-established WebDriver-based framework. Controls browsers via the WebDriver protocol; supports all browsers and platforms.

| Aspect | Details |
|--------|---------|
| **Browser Support** | All: Chrome, Firefox, Safari, Edge; mobile iOS/Android |
| **Architecture** | WebDriver protocol (external process; slower inter-process communication) |
| **Speed** | Slower: ~50-100ms overhead per interaction |
| **Learning Curve** | Higher: more boilerplate, explicit waits required |
| **Stability** | Less auto-waiting; flakier for dynamic applications |
| **Ecosystem** | Largest: many frameworks, integrations, mature libraries |
| **Cost** | Open-source (free); infrastructure for CI parallelization your responsibility |

```java
// Selenium (Java)
WebDriver driver = new ChromeDriver();
driver.get("https://example.com/checkout");

// Explicit waits required (no auto-waiting)
WebDriverWait wait = new WebDriverWait(driver, Duration.ofSeconds(10));
WebElement button = wait.until(
    ExpectedConditions.presenceOfElementLocated(By.xpath("//button[text()='Add to Cart']"))
);
button.click();

WebElement cartCount = driver.findElement(By.className("cart-count"));
Assert.assertTrue(cartCount.getText().contains("1"));

driver.quit();
```

**When to use:** Legacy projects; need for full browser compatibility (Safari, Internet Explorer); large teams with existing Selenium investment.

**Weaknesses:** Verbose; slower execution; flakier due to implicit waits being inadequate.

### Summary: When to Choose?

| Scenario | Framework |
|----------|-----------|
| New green-field project | Playwright |
| Complex client-side state (SPA) | Cypress |
| Multi-tab, cross-browser (Safari, IE) | Selenium |
| Fast feedback, modern stack | Playwright |
| Debugging deep into application state | Cypress |
| Enterprise with strict browser requirements | Selenium |

## Test Isolation and Parallel Execution

### Test Independence

Each E2E test must be independently executable and produce the same result regardless of execution order or parallelization.

**Violations:**
- Shared database state: Test A deletes a user that Test B depends on.
- Shared browser sessions: cookies, local storage persisted across tests.
- Shared external resources: temporary files, API rate limits.

**Ensuring isolation:**
- Fresh database per test (transaction rollback, fixture per test).
- Clear browser context per test (Playwright contexts, Cypress session clearing).
- Unique test data: generate unique email addresses, usernames per test.

```javascript
// Playwright: automatic context isolation
test.describe.configure({ mode: 'parallel' });

test('Flow A', async ({ page, context }) => {
    // Dedicated browser context for this test
    // Cookies, local storage NOT shared with other tests
    await page.goto('/login');
});

test('Flow B', async ({ page, context }) => {
    // Different context: independent state
    // Can run in parallel with Flow A
});
```

### Data Setup: Fixtures vs. Factories

**Fixtures:** Predefined, static test data created once and reused.

```javascript
const fixtures = {
    user: { id: 1, email: 'test@example.com', name: 'Alice' },
    product: { id: 101, name: 'Widget', price: 29.99 },
};
```

**Factories:** Generate dynamic, unique data per test.

```python
class UserFactory:
    counter = 0
    
    @classmethod
    def create(cls, **overrides):
        cls.counter += 1
        return User(
            email=f"test{cls.counter}@example.com",
            name=overrides.get('name', 'DefaultUser'),
            **overrides
        )

def test_user_registration():
    user = UserFactory.create(name='Alice')
    # Unique email: test1@example.com
```

**Choose fixtures when:** Data is truly immutable (reference data, configuration).
**Choose factories when:** Tests need unique identifiers (email, username) to avoid conflicts.

### Parallel Execution Strategies

**By test file:** Run multiple test files simultaneously on different workers.

```bash
# Playwright: run 4 tests in parallel
npx playwright test --workers=4
```

**By browser/platform:** Run the same test on multiple browsers in parallel (Browserstack, Sauce Labs).

```javascript
test.describe('@firefox', () => {
    test('works on Firefox', async ({ page }) => { ... });
});

test.describe('@safari', () => {
    test('works on Safari', async ({ page }) => { ... });
});
```

**By shard (Playwright):** Distribute test shards across CI machines.

```bash
# Machine 1 of 3
npx playwright test --shard=1/3

# Machine 2 of 3
npx playwright test --shard=2/3

# Machine 3 of 3
npx playwright test --shard=3/3
```

## Flaky Test Detection and Quarantine

A flaky test passes and fails unpredictably on the same code, wasting developer time and eroding trust in the test suite.

### Root Causes

- **Timing:** Waiting for element but timing out inconsistently. Solution: increase timeout, improve wait conditions.
- **Network:** API calls occasionally timeout. Solution: mock network or use testcontainers.
- **DOM race conditions:** Script modifies DOM after assertion. Solution: use auto-waiting, web-wait mechanisms.
- **Database:** Tests step on each other's data. Solution: isolate database per test.
- **Browser resource:** Memory or CPU constraints. Solution: reduce parallelism, investigate resource leaks.

### Detection

**Quarantine pattern:** Run the flaky test multiple times; if it fails *any* iteration, mark as flaky.

```javascript
// Playwright: rerun failed tests 3 times
export default {
    retries: process.env.CI ? 3 : 0,
};
```

**CI-level detection:** Tools like Flake Flagger analyze test history across runs and flag instability.

```yaml
# GitHub Actions workflow
- name: Detect flaky tests
  run: |
    npx flake-flagger --history 50 --threshold 80
    # Marks tests that fail < 80% of runs as flaky
```

### Quarantine Workflow

1. **Identify:** Test fails intermittently in CI.
2. **Tag:** Mark test with `@flaky` or move to `flaky-tests/` directory.
3. **Isolate from CI:** Don't count flaky tests toward pass/fail; don't block merges.
4. **Investigate:** Run in isolation 10+ times, capture logs/screenshots.
5. **Fix:** Address root cause (timing, data isolation, mock setup).
6. **Verify:** Re-run fixed test 50+ times before removing @flaky tag.

```javascript
// Quarantined test (doesn't block merge)
test('@flaky: checkout with payment processing', async ({ page }) => {
    // Isolating to understand intermittent timeout...
});
```

## Visual Regression Testing

Visual regression tests capture screenshots and compare them pixel-by-pixel (or with ML-based diffing) against baseline images.

### Tools

**Percy (by BrowserStack):** Cloud-based visual testing platform. Integrates with Playwright, Cypress, Selenium.

```javascript
// Playwright + Percy
import { percySnapshot } from '@percy/playwright';

test('homepage visual', async ({ page }) => {
    await page.goto('/');
    await percySnapshot(page, 'homepage');
});
```

Percy automatically:
- Captures across multiple browsers and viewport sizes.
- Detects visual changes compared to baseline (previous approved version).
- Allows manual approval of changes before merging.

**Chromatic (by Storybook team):** Specialized for component libraries; integrates with Storybook and Playwright.

```javascript
// Chromatic: Storybook + visual testing
import { expect } from '@playwright/test';

test('Button component', async ({ page }) => {
    await page.goto('http://localhost:6006?path=/stories/button--primary');
    // Chromatic automatically compares against baseline
});
```

**Local/In-Process:** Tools like Pixelmatch or Resemble compare images in-process (faster, but less sophisticated).

```javascript
// Pixelmatch (local)
const img1 = await page.screenshot();
const img2 = fs.readFileSync('./baseline.png');
const diff = pixelmatch(img1, img2);
assert(diff < 100);  // Allow 100 pixels of difference
```

### Best Practices

- **Test at specific breakpoints:** Capture at 1280px (desktop), 768px (tablet), 375px (mobile).
- **Ignore dynamic content:** Timestamps, IDs, user-generated content. Use masks or crop regions.
- **Baseline management:** Store baselines in version control; update deliberately when designs change.
- **Limit snapshot count:** 50-200 snapshots per project; more than that becomes maintenance burden.

## Accessibility Testing in E2E

Accessibility (a11y) ensures the application works for users with disabilities: screen readers, keyboard navigation, color contrast, and semantic HTML.

### Tools and Integration

**axe-core (Deque):** A11y audit library embedded in tests.

```javascript
import { injectAxe, checkA11y } from 'axe-playwright';

test('homepage accessibility', async ({ page }) => {
    await page.goto('/');
    await injectAxe(page);  // Load axe into page
    await checkA11y(page);  // Run audit
    // Fails if violations found
});
```

**Accessibility linting within Playwright:**

```javascript
test('article page is accessible', async ({ page }) => {
    await page.goto('/article/123');
    
    // Check ARIA labels
    const headings = await page.locator('h1, h2, h3').all();
    for (const heading of headings) {
        expect(await heading.textContent()).toBeTruthy();  // No empty headings
    }
    
    // Check focus visibility
    await page.keyboard.press('Tab');
    const focusElement = await page.evaluate(() => document.activeElement?.tagName);
    expect(focusElement).toBeTruthy();  // Something focusable
});
```

### Accessibility Test Types

- **Automated:** axe-core catches ~40% of issues (missing labels, contrast, structural).
- **Manual:** Keyboard navigation, screen reader testing (Narrator, NVDA, JAWS).
- **User testing:** Real users with disabilities (gold standard, expensive).

**Best practice:** Automated tests catch basics; pair with keyboard navigation tests and monthly manual audits by a11y specialist.

## Fixture and Test Data Architecture

### Shared Fixture Setup

```python
# conftest.py (pytest)
import pytest
from testfactory import UserFactory, ProductFactory

@pytest.fixture(scope='session')
def app_server():
    # Start server once per test session
    server = start_app_server()
    yield server
    server.stop()

@pytest.fixture(scope='function')
def authenticated_user(app_server):
    # Fresh user per test
    user = UserFactory.create()
    app_server.register(user)
    yield user
    # Cleanup: delete user after test

@pytest.fixture(scope='function')
def products_in_catalog(app_server, authenticated_user):
    # Setup: products dependent on user
    products = [ProductFactory.create() for _ in range(5)]
    app_server.add_to_catalog(authenticated_user, products)
    yield products
```

### Database Reset Between Tests

```javascript
// Playwright: reset database in beforeEach
import { test as base, expect } from '@playwright/test';

test.beforeEach(async ({ page }) => {
    // Reset or reset-to-clean database
    await page.request.post('/api/test-utils/reset-db');
});

test('user registration', async ({ page }) => {
    // Fresh database state
});
```

## Conclusion

E2E test architecture spans three levels:
1. **Pattern:** How you organize tests (Page Object, Screenplay, Flow Model).
2. **Framework:** Which tool executes tests (Playwright, Cypress, Selenium).
3. **Operations:** How you manage scale (isolation, parallelism, flaky detection, visual regression).

No pattern or framework is universally best. Choose based on application architecture, team expertise, and integration requirements. Prioritize isolation and maintainability over test count—500 maintainable, isolated tests outperform 2000 brittle, interdependent ones.