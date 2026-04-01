# Accessibility Engineering — WCAG 2.2, ARIA, Screen Readers & Legal Compliance

## WCAG 2.2 and the Four Principles

WCAG (Web Content Accessibility Guidelines) v2.2, maintained by the W3C, provides a framework for creating accessible digital products. It organizes accessibility requirements around **four principles** that form the acronym POUR:

### Perceivable

Information and interface components must be presentable to users in ways they can perceive. This doesn't mean every user perceives content identically — it means at least one accessible pathway exists for each content type.

**Key engineering constraints:**

- **Images**: Every meaningful image requires an `alt` attribute. Make `alt` text descriptive (not "photo", but "dashboard showing quarterly revenue trend"). Decorative images use `alt=""`.
- **Video/audio**: Captions (sync'd to audio) and transcripts are non-negotiable. Live video requires real-time captions. Audio descriptions handle visual-only content.
- **Color**: Never convey information by color alone. Use color + pattern, color + text, or color + icon. WCAG AA requires minimum 4.5:1 contrast for body text; 3:1 for large text (18pt+, 14pt bold+). AAA (strictest) requires 7:1 and 4.5:1 respectively.
- **Motion/animation**: Users can disable animations via `prefers-reduced-motion: reduce` media query. Respect this. Never auto-play animations; offer pause controls.

### Operable

Users must be able to navigate and interact with interfaces using their chosen input method.

**Keyboard navigation:**

- Every interactive element (button, link, form input) must be reachable via keyboard.
- Tab order should follow visual/logical reading order. Use `tabindex` sparingly; prefer natural DOM order.
- Custom components (`<div role="button">`) require keyboard handlers for Enter and Space keys.
- Keyboard traps (entering an element but unable to exit via keyboard) are failures. Modal dialogs must allow Escape to close.
- Focus must be visible (never `outline: none` without replacement). Visible focus indicator should have minimum 3:1 contrast to background.

**Pointer alternatives:**

- Touch targets (buttons, links) must be at least 44×44 CSS pixels (WCAG 2.5.5).
- Drag-and-drop interactions must have keyboard alternatives. Mouse hover shouldn't be the only way to reveal information.

### Understandable

Users must comprehend the interface.

**Text clarity:**

- Language must be at an appropriate reading level. Avoid jargon without explanation.
- Page title (`<title>` tag) should describe the page purpose uniquely.
- Form labels must be explicitly associated (`<label for="email">Email</label>` + `<input id="email">`), not just placeholder text.
- Error messages must describe the problem clearly and suggest remediation.

**Predictability:**

- Links should have descriptive link text ("click here" fails; "Download 2024 Q4 report" succeeds).
- Navigation placement should be consistent across pages.
- Unexpected context changes (form submission auto-navigating, pop-ups without user action) should be minimal and announced.

### Robust

Content must be compatible with assistive technologies.

**Technical standards:**

- Valid HTML. Use semantic tags (`<button>`, `<nav>`, `<main>`, `<article>`) instead of `<div>` everywhere.
- ARIA attributes when semantics don't exist. Custom JavaScript widgets require ARIA roles, states, and properties.
- Mobile native apps use platform accessibility APIs (UIAccessibility on iOS, AccessibilityDelegate on Android).

## ARIA: Roles, States, and Properties

ARIA (Accessible Rich Internet Applications) extends HTML semantics for complex interactive widgets that browser-native elements don't cover.

**ARIA framework:**

- **Roles** define what an element is: `role="button"`, `role="menu"`, `role="dialog"`, `role="tab"`, etc. Change a role only if you're building a custom widget; repurposing existing semantic elements with ARIA is a code smell.
- **States** describe current conditions: `aria-pressed="true"`, `aria-expanded="false"`, `aria-disabled="true"`.
- **Properties** describe relationships or metadata: `aria-label="Close"`, `aria-labelledby="heading-id"`, `aria-describedby="hint-id"`, `aria-live="polite"`, `aria-owns="listbox-id"`.

**Common patterns:**

- **Live regions**: `aria-live="polite"` announces content changes to screen readers without moving focus. Use `aria-live="assertive"` only for urgent announcements (errors, alerts).
- **Modal dialogs**: Require `role="dialog"`, `aria-modal="true"`, a labeling element, and focus trap (focus cycles within modal).
- **Tabs**: Container has `role="tablist"`. Each tab button has `role="tab"` + `aria-selected` + `aria-controls="panel-id"`. Content panel has `role="tabpanel"` + `aria-labelledby="tab-id"`.
- **Combobox/search**: `role="combobox"`, `aria-expanded="true/false"`, `aria-owns` (connects button to listbox), `aria-activedescendant` (keyboard-selected item).
- **Tooltips**: Native `<title>` attribute works but is invisible. ARIA uses `aria-describedby` pointing to a tooltip element.

**Avoid over-use**: ARIA is a bridge for custom components. If HTML provides semantics (button, input, nav), use it. ARIA on `<div role="button">` + `role="menuitem"` layering is a sign of over-engineering.

## Screen Reader Mechanics and Testing

Screen readers convert interface state into speech and braille. Understanding their model changes how you design.

**Screen reader modes:**

- **Browse/Scan mode**: User arrows through content, hearing text and structure. Screen readers announce headings, links, form fields, landmarks (`<nav>`, `<main>`).
- **Focus/Forms mode**: User tabs to interactive elements. Entering a form field or custom widget switches mode; arrow keys now control the widget, not page scrolling.
- **Virtual cursor**: User can arrow through all content including links within paragraphs.

**What screen readers announce:**

- Heading level (`<h1>` through `<h6>`), helping users navigate by structure.
- Explicit labels ("Email" before form field) and instructions ("Required").
- ARIA live announcements for dynamic content.
- Link text (anchor text or `aria-label`). "Click here" links are unusable without context.
- Button state (`aria-pressed`, `aria-expanded`).
- List structure ("list, 5 items").
- Table headers and relationships (`<th scope="col">Name</th>`).

**Testing workflow:**

1. **Manual testing** with a production screen reader (NVDA on Windows, JAWS, VoiceOver on macOS/iOS, TalkBack on Android).
2. **Automated testing** to catch structural issues (missing labels, missing alt text, ARIA misuse).
3. **Keyboard-only testing** (unplug mouse, tab through the entire interface).

## Automated Testing: axe and Lighthouse

Automated tools catch ~30-40% of accessibility issues (structure, contrast, alt text). They miss context-dependent problems (whether alt text is *good* or whether keyboard navigation *makes sense*).

**axe (Deque):**

- Browser extension and integrable library (`npm install @axe-core/react`).
- Runs against WCAG/Section 508/ARIA standard rules.
- Outputs results with severity (critical > serious > moderate > minor) and guidance links.
- Can integrate into CI/CD (axe-core test runners for Jest, Cypress, Playwright).
- Sample integration: Cypress tests with `cy.injectAxe()` + `cy.checkA11y()`.

**Lighthouse (Google/Chrome):**

- Built into Chrome DevTools (Audits tab) or CLI (`npm install -g lighthouse`).
- Includes accessibility, performance, SEO, best practices audits.
- Accessibility section scores based on detected failures; also lists "Passed Audits" (what worked).
- Less granular than axe but sufficient for catching regressions.

**Testing strategy:**

- Run automated checks on every build (fail CI if critical issues detected).
- Manual screen reader testing on critical user journeys (login, checkout, form submission).
- Include keyboard-only testing in QA.
- Annual recertification or audit by an external accessibility firm.

## Legal Landscape: ADA, EAA, and Emerging Standards

Accessibility is increasingly a legal requirement, not just best practice.

**United States: ADA (Americans with Disabilities Act)**

- Enacted 1990; applies to public entities and private businesses serving the public.
- Originally focused on physical accessibility; courts have extended it to digital interfaces.
- Standards reference WCAG 2.1 AA (though 2.2 is now recommended).
- Enforcement: Private right of action (individuals can sue) + DOJ enforcement.
- Penalties: Injunctive relief (fix the site) + damages + attorney fees.

**European Union: EAA (European Accessibility Act)**

- Effective 2025. Applies to products and services in scope: websites, mobile apps, e-commerce, digital media.
- Mandates WCAG 2.1 AA compliance for websites; 2.5 for apps.
- Conformance statement required (public declaration of compliance level and known issues).
- Enforcement: National enforcement bodies + consumer organizations can pursue remedies.

**Emerging trends:**

- **Canada, Australia, UK** developing accessibility legislation aligned with WCAG 2.1+ AA.
- **Section 508** (US government procurement) now references WCAG 2.1; agencies prefer 2.2.
- **Litigation trend**: US legal landscape is aggressive (hundreds of class actions yearly against retailers, financial institutions, tech companies). Inaccessibility is treated as discrimination.

**Best practice defensibility:**

- Conduct accessibility audit and document findings.
- Make good-faith remediation plan and timeline.
- Publish accessibility statement (what you've done, what remains, how users can report issues).
- Maintain evidence of ongoing testing and improvements.
- Include accessibility in hiring/training (not a one-time fix).

## Cross-Cutting Concerns

**Focus management**: When content updates dynamically (search results, filter changes), move focus to the first result or provide a skip link. Forcing users to re-ascend the page breaks keyboard navigation.

**Motion preferences**: Test with `prefers-reduced-motion: reduce`. Transitions, parallax, auto-play video should pause or disable. This isn't just accessibility — it prevents vestibular issues and reduces motion sickness.

**Form accessibility**: Confirmation dialogs are often missed by screen reader users. Instead of "Are you sure?", integrate confirmation into the workflow (show a summary, offer a reconsider button).

**Progressive enhancement**: Server-render critical content (search results, forms). JavaScript failures shouldn't make the site inaccessible. ARIA alone doesn't fix broken navigation.

## See Also

- [Web Accessibility — Principles, Patterns & Inclusive Design](web-accessibility.md)
- [Accessible Design Patterns — WCAG, Component Implementation & User Flows](design-accessibility-patterns.md)
- [Accessibility Testing](testing-accessibility.md)